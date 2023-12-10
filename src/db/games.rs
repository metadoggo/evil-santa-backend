use std::collections::HashMap;

use axum::{extract::FromRef, response::IntoResponse};
use chrono::{DateTime, NaiveDateTime, Utc};
use is_empty::IsEmpty;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sqlx::{
  postgres::PgListener, prelude::FromRow, query, query_as, types::Json, PgPool, Postgres,
  QueryBuilder,
};
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

use crate::api::AppState;

use super::{apply_list_filters, handle_pg_error, Error, ListParams, UpdateResult};

#[derive(FromRow, Serialize)]
pub struct Game {
  pub id: Uuid,
  pub name: String,
  #[sqlx(json)]
  pub users: HashMap<String, i64>,
  pub images: Vec<String>,
  pub player_id: Option<i64>,
  pub present_id: Option<i64>,
  pub started_at: Option<NaiveDateTime>,
  pub created_at: NaiveDateTime,
  pub updated_at: Option<NaiveDateTime>,
}

// list games
pub async fn list(db: &PgPool, user_id: &str, p: ListParams) -> Result<Vec<Game>, Error> {
  let mut query = QueryBuilder::<Postgres>::new(
    "SELECT id, name, images, users, player_id, present_id, started_at, created_at, updated_at FROM games WHERE users ? ",
  );
  query.push_bind(user_id);
  query = apply_list_filters(query, &p, vec!["id", "name"])?;

  query
    .build_query_as()
    .fetch_all(db)
    .await
    .map_err(Error::Sqlx)
}

// get a game
pub async fn get(db: &PgPool, id: Uuid) -> Result<Game, Error> {
  query_as("SELECT id, name, images, users, player_id, present_id, started_at, created_at, updated_at FROM games WHERE id = $1")
  .bind(id)
  .fetch_one(db)
  .await
  .map_err(handle_pg_error)
}

pub struct CreateParams<'a> {
  pub id: Uuid,
  pub name: &'a str,
  pub images: Vec<String>,
  pub users: &'a HashMap<String, i64>,
}

#[derive(sqlx::FromRow, Serialize, Debug)]
pub struct CreateResult {
  pub created_at: NaiveDateTime,
}

// create a game
pub async fn create<'a>(db: &PgPool, p: CreateParams<'a>) -> Result<CreateResult, Error> {
  query_as(
    "INSERT INTO games (id, name, images, users) VALUES ($1, $2, $3, $4) RETURNING created_at",
  )
  .bind(p.id)
  .bind(p.name)
  .bind(p.images)
  .bind(Json(p.users))
  .fetch_one(db)
  .await
  .map_err(handle_pg_error)
}

#[derive(Deserialize, IsEmpty, Default)]
pub struct UpdateData {
  pub name: Option<String>,
  pub images: Option<Vec<String>>,
  pub users: Option<HashMap<String, i64>>,
}

#[skip_serializing_none]
#[derive(sqlx::FromRow, Serialize, Debug)]
pub struct GameStateUpdateResult {
  pub player_id: Option<i64>,
  pub present_id: Option<i64>,
  pub started_at: Option<NaiveDateTime>,
  pub updated_at: NaiveDateTime,
}

impl IntoResponse for GameStateUpdateResult {
  fn into_response(self) -> axum::response::Response {
    serde_json::to_string(&self).unwrap().into_response()
  }
}

// update a game
pub async fn update(db: &PgPool, game_id: Uuid, data: UpdateData) -> Result<UpdateResult, Error> {
  if data.is_empty() {
    return Err(Error::Empty);
  }

  let mut query = QueryBuilder::<Postgres>::new("UPDATE games SET");
  let mut sep = query.separated(", ");

  if let Some(name) = data.name {
    sep.push(" name = ").push_bind_unseparated(name);
  }
  if let Some(images) = data.images {
    sep.push(" images = ").push_bind_unseparated(images);
  }
  if let Some(users) = data.users {
    sep.push(" users = ").push_bind_unseparated(Json(users));
  }
  sep.push(" updated_at = NOW()");
  query.push(" WHERE id = ").push_bind(game_id);
  query.push(" RETURNING updated_at");
  query
    .build_query_as()
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)
}

#[derive(Deserialize)]
pub struct ReplaceParams {
  pub name: String,
  pub images: Option<Vec<String>>,
  pub users: HashMap<String, i64>,
}

// replace a game
pub async fn replace(db: &PgPool, id: Uuid, p: ReplaceParams) -> Result<UpdateResult, Error> {
  let mut query = QueryBuilder::<Postgres>::new("UPDATE games SET");
  let mut sep = query.separated(", ");
  sep.push(" name = ").push_bind_unseparated(p.name);
  sep
    .push(" images = ")
    .push_bind_unseparated(p.images.unwrap_or_default());
  sep.push(" users = ").push_bind_unseparated(Json(p.users));
  sep.push(" updated_at = NOW()");
  query.push(" WHERE id = ").push_bind(id);
  query.push(" RETURNING updated_at");
  query
    .build_query_as()
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)
}

// delete a game
pub async fn delete(db: &PgPool, game_id: Uuid) -> Result<(), Error> {
  match query!("DELETE FROM games WHERE id = $1", game_id)
    .execute(db)
    .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }
}

// update a game
pub async fn start(db: &PgPool, game_id: Uuid) -> Result<GameStateUpdateResult, Error> {
  let game = query!("UPDATE games SET started_at = NOW() WHERE id = $1 AND started_at IS NULL RETURNING started_at, updated_at", game_id)
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)?;

  Ok(GameStateUpdateResult {
    player_id: None,
    present_id: None,
    started_at: game.started_at,
    updated_at: game.updated_at.unwrap_or_default(),
  })
}

// reset a game
pub async fn reset(db: &PgPool, game_id: Uuid) -> Result<GameStateUpdateResult, Error> {
  let mut tx = db.begin().await.map_err(|err| Error::Sqlx(err))?;

  match query!(
    "UPDATE presents SET player_id = NULL, updated_at = NOW() WHERE game_id = $1",
    game_id,
  )
  .execute(&mut *tx)
  .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }?;

  let game = query!(
    "UPDATE games
     SET started_at = NULL,
       player_id = NULL,
       present_id = NULL,
       updated_at = NOW()
     WHERE id = $1
     RETURNING updated_at",
    game_id
  )
  .fetch_one(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  match query!("DELETE FROM play_events WHERE game_id = $1", game_id)
    .execute(&mut *tx)
    .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }?;

  tx.commit().await.map_err(handle_pg_error)?;

  Ok(GameStateUpdateResult {
    player_id: None,
    present_id: None,
    started_at: None,
    updated_at: game.updated_at.unwrap_or_default(),
  })
}

// roll a dice to pick a player
pub async fn roll(db: &PgPool, game_id: Uuid) -> Result<GameStateUpdateResult, Error> {
  let mut tx = db.begin().await.map_err(|err| Error::Sqlx(err))?;

  let game = query!(
    "UPDATE games SET player_id = (
    SELECT players.id 
    FROM players
    WHERE id NOT IN (
      SELECT player_id
      FROM presents 
      WHERE game_id = $1 
      AND player_id IS NOT NULL)
    AND game_id = $1
    ORDER BY random() 
    LIMIT 1) 
  WHERE player_id IS NULL 
  AND id = $1 RETURNING player_id, updated_at",
    game_id
  )
  .fetch_one(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  match game.player_id {
    Some(player_id) => {
      query!(
        "INSERT INTO play_events (game_id, player_id) VALUES ($1, $2)",
        game_id,
        player_id
      )
      .execute(&mut *tx)
      .await
      .map_err(handle_pg_error)?;

      tx.commit().await.map_err(handle_pg_error)?;

      Ok(GameStateUpdateResult {
        player_id: Some(player_id),
        present_id: None,
        started_at: None,
        updated_at: game.updated_at.unwrap_or_default(),
      })
    }
    None => Err(Error::NotFound),
  }
}

// pick a present
pub async fn pick(
  db: &PgPool,
  game_id: Uuid,
  present_id: i64,
) -> Result<GameStateUpdateResult, Error> {
  let mut tx = db.begin().await.map_err(|err| Error::Sqlx(err))?;

  let game = query!(
    "UPDATE games SET
      present_id = $1,
      updated_at = NOW()
    WHERE present_id IS NULL
      AND id = $2
    RETURNING player_id, updated_at",
    present_id,
    game_id
  )
  .fetch_one(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  query!(
    "INSERT INTO play_events (game_id, player_id, present_id) VALUES ($1, $2, $3)",
    game_id,
    game.player_id,
    present_id
  )
  .execute(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  tx.commit().await.map_err(handle_pg_error)?;

  Ok(GameStateUpdateResult {
    player_id: None,
    present_id: Some(present_id),
    started_at: None,
    updated_at: game.updated_at.unwrap_or_default(),
  })
}

// keep a present
pub async fn keep(db: &PgPool, game_id: Uuid) -> Result<GameStateUpdateResult, Error> {
  let mut tx = db.begin().await.map_err(|err| Error::Sqlx(err))?;

  let game = query!(
    "SELECT player_id, present_id FROM games WHERE id = $1",
    game_id
  )
  .fetch_one(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  match query!(
    "UPDATE presents SET player_id = $1, updated_at = NOW() WHERE id = $2",
    game.player_id,
    game.present_id
  )
  .execute(&mut *tx)
  .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }?;

  let game_after = query!(
    "UPDATE games SET
      player_id = NULL,
      present_id = NULL,
      updated_at = NOW()
    WHERE id = $1
    RETURNING updated_at",
    game_id
  )
  .fetch_one(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  query!(
    "INSERT INTO play_events (game_id, player_id, present_id, from_player_id, from_present_id) VALUES ($1, $2, $3, $4, $5)",
    game_id,
    game.player_id,
    game.present_id,
    game.player_id,
    game.present_id,
  )
  .execute(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  tx.commit().await.map_err(handle_pg_error)?;

  Ok(GameStateUpdateResult {
    player_id: None,
    present_id: None,
    started_at: None,
    updated_at: game_after.updated_at.unwrap_or_default(),
  })
}

// steal a present
pub async fn steal(
  db: &PgPool,
  game_id: Uuid,
  present_id: i64,
) -> Result<GameStateUpdateResult, Error> {
  let mut tx = db.begin().await.map_err(|err| Error::Sqlx(err))?;

  let game = query!(
    "SELECT player_id, present_id FROM games WHERE id = $1",
    game_id
  )
  // .bind(game_id)
  .fetch_one(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  let present = query!("SELECT player_id FROM presents WHERE id = $1", present_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(handle_pg_error)?;

  match query!(
    "UPDATE presents SET player_id = $1, updated_at = NOW() WHERE id = $2",
    game.player_id,
    present_id,
  )
  .execute(&mut *tx)
  .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }?;

  match query!(
    "UPDATE presents SET player_id = $1, updated_at = NOW() WHERE id = $2",
    present.player_id,
    game.present_id
  )
  .execute(&mut *tx)
  .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }?;

  let game_after = query!(
    "UPDATE games SET
      player_id = NULL,
      present_id = NULL,
      updated_at = NOW()
    WHERE id = $1
    RETURNING updated_at",
    game_id
  )
  .fetch_one(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  query!(
    "INSERT INTO play_events (game_id, player_id, present_id, from_player_id, from_present_id) VALUES ($1, $2, $3, $4, $5)",
    game_id,
    game.player_id,
    game.present_id,
    present.player_id,
    present_id,
  )
  .execute(&mut *tx)
  .await
  .map_err(handle_pg_error)?;

  tx.commit().await.map_err(handle_pg_error)?;

  Ok(GameStateUpdateResult {
    started_at: None,
    player_id: None,
    present_id: None,
    updated_at: game_after.updated_at.unwrap_or_default(),
  })
}

#[derive(FromRow, Clone, Serialize, Deserialize, Debug)]
pub struct PlayEvent {
  pub id: i64,
  pub player_id: i64,
  pub present_id: Option<i64>,
  pub from_player_id: Option<i64>,
  pub from_present_id: Option<i64>,
  pub created_at: NaiveDateTime,
}

pub type PlayStream = Sender<PlayEvent>;

impl FromRef<AppState> for PlayStream {
  fn from_ref(state: &AppState) -> Self {
    state.play_stream.clone()
  }
}

pub async fn list_events(
  db: &PgPool,
  game_id: Uuid,
  p: ListParams,
) -> Result<Vec<PlayEvent>, Error> {
  let mut query = QueryBuilder::<Postgres>::new(
    "
    SELECT id,
      game_id,
      player_id,
      present_id,
      from_player_id,
      from_present_id,
      created_at
    FROM play_events
    WHERE game_id = ",
  );
  query.push_bind(game_id);
  query = apply_list_filters(query, &p, Vec::new())?;

  query
    .build_query_as()
    .fetch_all(db)
    .await
    .map_err(Error::Sqlx)
}

#[derive(Deserialize, Debug)]
pub struct PlayLogPayload {
  pub id: i64,
  pub player_id: i64,
  pub present_id: Option<i64>,
  pub from_player_id: Option<i64>,
  pub from_present_id: Option<i64>,
  pub created_at: DateTime<Utc>,
}

pub async fn start_listening(
  mut listener: PgListener,
  tx: &PlayStream,
) -> Result<(), anyhow::Error> {
  listener.listen("play").await?;
  loop {
    if let Some(notif) = listener.try_recv().await? {
      match serde_json::from_str::<PlayEvent>(notif.payload()) {
        Ok(payload) => match tx.send(payload) {
          Ok(n) => {
            tracing::info!("Sent event to {} subscribers", n);
          }
          Err(e) => {
            tracing::error!("Error send message to client: {}", e.to_string());
          }
        },
        Err(e) => {
          tracing::error!("Error deserialize message: {}", e.to_string());
        }
      }
    }
  }
}
