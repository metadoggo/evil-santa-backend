use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::{apply_list_filters, handle_pg_error, CreateResult, Error, ListParams, UpdateResult};

#[derive(FromRow, Serialize)]
pub struct Player {
  pub id: i64,
  pub game_id: Uuid,
  pub name: String,
  pub images: Vec<String>,
}

// list players
pub async fn list(db: &PgPool, game_id: Uuid, p: ListParams) -> Result<Vec<Player>, Error> {
  let mut query = QueryBuilder::<Postgres>::new(
    "SELECT id, game_id, name, images FROM players WHERE game_id = $1",
  );

  query = apply_list_filters(query, &p, vec!["id", "name"])?;
  query
    .build_query_as()
    .bind(game_id)
    .fetch_all(db)
    .await
    .map_err(Error::Sqlx)
}

// get a player
pub async fn get(db: &PgPool, id: i64) -> Result<Player, Error> {
  query_as("SELECT id, game_id, name, images FROM players WHERE id = $1")
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)
}

#[derive(Deserialize)]
pub struct CreateParams {
  pub name: String,
  pub images: Vec<String>,
}

// create a player
pub async fn create(
  db: &PgPool,
  game_id: Uuid,
  p: CreateParams,
) -> Result<CreateResult<i64>, Error> {
  // QueryBuilder::<Postgres>::new("INSERT INTO players(name, images) VALUES (?, ?, ?) RESTURNING id, created_at")
  query_as!(
    CreateResult::<i64>,
    "INSERT INTO players (game_id, name, images) VALUES ($1, $2, $3) RETURNING id, created_at",
    game_id,
    p.name,
    &p.images
  )
  .fetch_one(db)
  .await
  .map_err(handle_pg_error)
}

#[derive(Deserialize)]
pub struct UpdateParams {
  pub name: Option<String>,
  pub images: Option<Vec<String>>,
}

// update a player
pub async fn update(db: &PgPool, id: i64, p: UpdateParams) -> Result<UpdateResult, Error> {
  let mut query = QueryBuilder::<Postgres>::new("UPDATE players SET");
  let mut sep = query.separated(", ");
  if let Some(name) = p.name {
    sep.push(" name = ").push_bind_unseparated(name);
  }
  if let Some(images) = p.images {
    sep.push(" images = ").push_bind_unseparated(images);
  }
  sep.push(" updated_at = NOW()");
  query.push(" WHERE id = ").push_bind(id);
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
}

// replace a player
pub async fn replace(db: &PgPool, id: i64, p: ReplaceParams) -> Result<UpdateResult, Error> {
  let mut query = QueryBuilder::<Postgres>::new("UPDATE players SET");
  let mut sep = query.separated(", ");
  sep.push(" name = ").push_bind_unseparated(p.name);
  sep
    .push(" images = ")
    .push_bind_unseparated(p.images.unwrap_or_default());
  sep.push(" updated_at = NOW()");
  query.push(" WHERE id = ").push_bind(id);
  query.push(" RETURNING updated_at");
  query
    .build_query_as()
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)
}

// delete a player
pub async fn delete(db: &PgPool, id: i64) -> Result<(), Error> {
  match sqlx::query("DELETE FROM players WHERE id = $1")
    .bind(id)
    .execute(db)
    .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }
}
