use axum::{
  extract::{Path, Query, State},
  http::StatusCode,
  response::{IntoResponse, Response}, Json,
};
use uuid::Uuid;

use crate::{
  auth::MyFirebaseUser,
  db::{
    presents::{self, CreateParams, ReplaceParams, UpdateParams},
    ListParams,
  },
};

use super::{handle_db_error, make_json_response};

// list presents
pub async fn list(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
  Query(p): Query<ListParams>,
) -> Response {
  if user.can_view(game_id) {
    let res = presents::list(&db, game_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// get a present
pub async fn get(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, present_id)): Path<(Uuid, i64)>,
) -> Response {
  if user.can_view(game_id) {
    let res = presents::get(&db, present_id);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// create a present
pub async fn create(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path(game_id): Path<Uuid>,
  Json(p): Json<CreateParams>,
) -> Response {
  if user.can_edit(game_id) {
    let res = presents::create(&db, game_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// update a present
pub async fn update(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, present_id)): Path<(Uuid, i64)>,
  Json(p): Json<UpdateParams>,
) -> Response {
  if user.can_edit(game_id) {
    let res = presents::update(&db, present_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// replace a present
pub async fn replace(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, present_id)): Path<(Uuid, i64)>,
  Json(p): Json<ReplaceParams>,
) -> Response {
  if user.can_edit(game_id) {
    let res = presents::replace(&db, present_id, p);
    make_json_response(res.await)
  } else {
    StatusCode::FORBIDDEN.into_response()
  }
}

// delete a present
pub async fn delete(
  State(db): State<sqlx::PgPool>,
  user: MyFirebaseUser,
  Path((game_id, present_id)): Path<(Uuid, i64)>,
) -> Result<StatusCode, Response> {
  if user.can_edit(game_id) {
    presents::delete(&db, present_id)
      .await
      .map_err(handle_db_error)?;
    Ok(StatusCode::ACCEPTED)
  } else {
    Err(StatusCode::FORBIDDEN.into_response())
  }
}
