use axum::{
  extract::{Path, Query, State},
  http::StatusCode,
  response::{IntoResponse, Response}, Json,
};
use uuid::Uuid;

use crate::{
  auth::MyFirebaseUser,
  db::{
    players::{self, CreateParams, ReplaceParams, UpdateParams},
    ListParams,
  },
};

use super::{handle_db_error, make_json_response};

// list players
pub async fn list(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Query(p): Query<ListParams>,
  Path(game_id): Path<Uuid>,
) -> Response {
  if user.can_view(game_id) {
    let res = players::list(&db, game_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// get a player
pub async fn get(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, player_id)): Path<(Uuid, i64)>,
) -> Response {
  if user.can_view(game_id) {
    let res = players::get(&db, player_id);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// create a player
pub async fn create(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
  Json(p): Json<CreateParams>,
) -> Response {
  if user.can_edit(game_id) {
    let res = players::create(&db, game_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// update a player
pub async fn update(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, player_id)): Path<(Uuid, i64)>,
  Json(p): Json<UpdateParams>,
) -> Response {
  if user.can_edit(game_id) {
    let res = players::update(&db, player_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// replace a player
pub async fn replace(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, player_id)): Path<(Uuid, i64)>,
  Json(p): Json<ReplaceParams>,
) -> Response {
  if user.can_edit(game_id) {
    let res = players::replace(&db, player_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// delete a player
pub async fn delete(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, player_id)): Path<(Uuid, i64)>,
) -> Result<StatusCode, Response> {
  if user.can_edit(game_id) {
    players::delete(&db, player_id)
      .await
      .map_err(handle_db_error)?;
    Ok(StatusCode::ACCEPTED)
  } else {
    Err(StatusCode::FORBIDDEN.into_response())
  }
}
