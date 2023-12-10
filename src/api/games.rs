use std::{collections::HashMap, time::Duration};

use axum::{
  extract::{Path, Query, State},
  http::StatusCode,
  response::{sse::Event, IntoResponse, Response, Sse},
  Json,
};
use chrono::NaiveDateTime;
use futures_util::Stream;
use futures_util::StreamExt;
use serde::Deserialize;
use serde::Serialize;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
  auth::{user::UserService, CustomClaims, MyFirebaseUser},
  db::{
    games::{self, PlayStream, ReplaceParams, UpdateData},
    ListParams,
  },
};

use super::{handle_db_error, make_json_response};

pub const OWNER_PERMISSION: i64 = 0xff;
pub const PLAY_PERMISSION: i64 = 0x2;
pub const VIEW_PERMISSION: i64 = 0x1;

// list games
pub async fn list(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Query(p): Query<ListParams>,
) -> Response {
  make_json_response(games::list(&db, &user.sub, p).await)
}

// get a game
pub async fn get(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
) -> Response {
  if !user.can_view(game_id) {
    return StatusCode::FORBIDDEN.into_response();
  }
  make_json_response(games::get(&db, game_id).await)
}

#[derive(Deserialize)]
pub struct CreateParams {
  pub name: String,
  pub images: Option<Vec<String>>,
  pub users: Option<HashMap<String, i64>>,
}

#[derive(Serialize)]
pub struct GameCreated {
  id: Uuid,
  users: HashMap<String, i64>,
  created_at: NaiveDateTime,
}

// create a game
pub async fn create(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  State(mut claims_service): State<UserService>,
  Json(p): Json<CreateParams>,
) -> Response {
  let id = Uuid::new_v4();
  let permission = OWNER_PERMISSION;
  let mut claims = user.custom_claims();
  claims.games.insert(id.to_string(), permission);

  match claims_service
    .set_custom_attributes(&user.sub, claims)
    .await
  {
    Ok(()) => {
      let mut users = p.users.unwrap_or_default();
      users.insert(user.sub, permission);
      let res = games::create(
        &db,
        games::CreateParams {
          id,
          name: &p.name,
          images: p.images.unwrap_or_default(),
          users: &users,
        },
      );
      make_json_response(res.await.map(|res| GameCreated {
        id,
        users,
        created_at: res.created_at,
      }))
    }
    Err(err) => (
      StatusCode::INTERNAL_SERVER_ERROR,
      format!("Error update claims: {}", err),
    )
      .into_response(),
  }
}

// update a game
pub async fn update(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
  data: Option<Json<UpdateData>>,
) -> Response {
  if !user.can_edit(game_id) {
    return StatusCode::FORBIDDEN.into_response();
  }
  let data = data.unwrap_or_default().0;
  if let Some(users) = &data.users {
    if matches!(users.get(&user.sub), Some(p) if p.lt(&OWNER_PERMISSION)) {
      return StatusCode::BAD_REQUEST.into_response();
    }
  }
  make_json_response(games::update(&db, game_id, data).await)
}

#[derive(Deserialize, Default, Debug)]
pub struct PlayParams {
  pub action: String,
}

#[derive(Deserialize, Default)]
pub struct PlayData {
  pub present_id: i64,
}

// update a game
pub async fn play(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
  Query(q): Query<PlayParams>,
  data: Option<Json<PlayData>>,
) -> Response {
  if !user.can_play(game_id) {
    return StatusCode::FORBIDDEN.into_response();
  }
  match q.action.as_str() {
    "start" => games::start(&db, game_id)
      .await
      .map_err(handle_db_error)
      .into_response(),
    "reset" => games::reset(&db, game_id)
      .await
      .map_err(handle_db_error)
      .into_response(),
    "roll" => games::roll(&db, game_id)
      .await
      .map_err(handle_db_error)
      .into_response(),
    "pick" => match data {
      Some(data) => games::pick(&db, game_id, data.present_id)
        .await
        .map_err(handle_db_error)
        .into_response(),
      None => StatusCode::BAD_REQUEST.into_response(),
    },
    "keep" => games::keep(&db, game_id)
      .await
      .map_err(handle_db_error)
      .into_response(),
    "steal" => match data {
      Some(data) => games::steal(&db, game_id, data.present_id)
        .await
        .map_err(handle_db_error)
        .into_response(),
      None => StatusCode::BAD_REQUEST.into_response(),
    },
    _ => StatusCode::BAD_REQUEST.into_response(),
  }
}

// replace a game
pub async fn replace(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
  Json(p): Json<ReplaceParams>,
) -> Response {
  if !user.can_edit(game_id) {
    return StatusCode::FORBIDDEN.into_response();
  }
  make_json_response(games::replace(&db, game_id, p).await)
}

// delete a game
pub async fn delete(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
) -> Result<StatusCode, Response> {
  if !user.can_edit(game_id) {
    return Err(StatusCode::FORBIDDEN.into_response());
  }
  games::delete(&db, game_id).await.map_err(handle_db_error)?;
  Ok(StatusCode::ACCEPTED)
}

// accept view permission for the current user
pub async fn accept_invitation(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  State(mut claims_service): State<UserService>,
  Path(game_id): Path<Uuid>,
) -> Result<StatusCode, Response> {
  let game = crate::db::games::get(&db, game_id)
    .await
    .map_err(handle_db_error)?;

  let game_id_string = game_id.to_string();
  if game.users.get(&user.sub).is_some() && user.games.get(&game_id_string).is_none() {
    let mut new_games = user.games.clone();
    new_games.insert(game_id_string, VIEW_PERMISSION);
    match claims_service
      .set_custom_attributes(&user.sub, CustomClaims { games: new_games })
      .await
    {
      Ok(()) => Ok(StatusCode::OK),
      Err(err) => Err((StatusCode::BAD_GATEWAY, err.to_string()).into_response()),
    }
  } else {
    Ok(StatusCode::OK)
  }
}

// list games
pub async fn list_events(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
  Query(p): Query<ListParams>,
) -> Response {
  if !user.can_view(game_id) {
    return StatusCode::FORBIDDEN.into_response();
  }
  make_json_response(games::list_events(&db, game_id, p).await)
}

pub async fn events(
  State(play_stream): State<PlayStream>,
) -> Sse<impl Stream<Item = Result<Event, anyhow::Error>>> {
  let rx = play_stream.subscribe();

  let receiver = BroadcastStream::new(rx);
  let stream = receiver.map(|message| {
    let message = message?;
    let data = serde_json::to_string(&message)?;
    Ok(Event::default().data(data))
  });

  Sse::new(stream).keep_alive(
    axum::response::sse::KeepAlive::new()
      .interval(Duration::from_secs(1))
      .text("It's good to be alive!"),
  )
}
