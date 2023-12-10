use axum::{
  async_trait,
  extract::{FromRef, FromRequestParts, State},
  http::{request::Parts, StatusCode},
  response::{IntoResponse, Response},
  routing::{get, post},
  Router,
};
use axum_extra::{
  headers::{authorization::Bearer, Authorization},
  TypedHeader,
};
use firebase_auth::FirebaseAuth;
use serde::Serialize;

use crate::{
  auth::{user::UserService, MyFirebaseUser},
  db::{self, games::PlayStream},
};

pub mod games;
pub mod players;
pub mod presents;

#[derive(Clone)]
pub struct AppState {
  pub pool: sqlx::PgPool,
  pub firebase_auth: FirebaseAuth<MyFirebaseUser>,
  pub claims_service: UserService,
  pub play_stream: PlayStream,
}

impl FromRef<AppState> for sqlx::PgPool {
  fn from_ref(state: &AppState) -> Self {
    state.pool.clone()
  }
}

pub struct Server {
  pub router: Router,
}

impl Server {
  pub fn new(
    pool: sqlx::PgPool,
    firebase_auth: FirebaseAuth<MyFirebaseUser>,
    claims_service: UserService,
    play_stream: PlayStream,
  ) -> Self {
    let app_state = AppState {
      pool,
      firebase_auth,
      claims_service,
      play_stream,
    };

    let router = axum::Router::new()
      .route("/", get(home))
      .route("/health", get(health))
      .route("/games", get(games::list).post(games::create))
      .route("/accept/:game_id", get(games::accept_invitation))
      .route("/play/:game_id", post(games::play))
      .route(
        "/games/:game_id",
        get(games::get)
          .patch(games::update)
          .put(games::replace)
          .delete(games::delete),
      )
      .route("/games/:game_id/events", get(games::list_events))
      .route("/games/:game_id/stream", get(games::events))
      .route(
        "/games/:game_id/players",
        get(players::list).post(players::create),
      )
      .route(
        "/games/:game_id/players/:player_id",
        get(players::get)
          .patch(players::update)
          .put(players::replace)
          .delete(players::delete),
      )
      .route(
        "/games/:game_id/presents",
        get(presents::list).post(presents::create),
      )
      .route(
        "/games/:game_id/presents/:present_id",
        get(presents::get)
          .patch(presents::update)
          .put(presents::replace)
          .delete(presents::delete),
      )
      .with_state(app_state);

    Self { router }
  }
}

// home
pub async fn home() -> &'static str {
  "Hello, World!"
}

// check health
pub async fn health(State(db): State<sqlx::PgPool>) -> (StatusCode, &'static str) {
  match db::health(&db).await {
    Ok(()) => (StatusCode::OK, "👍 Healthy!"),
    _ => (StatusCode::INTERNAL_SERVER_ERROR, "😭 Degraded!"),
  }
}

pub fn handle_db_error(err: db::Error) -> Response {
  match err {
    db::Error::Empty | db::Error::InvalidOrder => {
      (StatusCode::BAD_REQUEST, err.to_string()).into_response()
    }
    db::Error::NotFound => StatusCode::NOT_FOUND.into_response(),
    _ => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
  }
}

pub fn make_json_response<T: Serialize>(res: Result<T, db::Error>) -> Response {
  match res {
    Ok(data) => serde_json::to_string(&data).unwrap().into_response(),
    Err(err) => handle_db_error(err),
  }
}

#[async_trait]
impl<S> FromRequestParts<S> for MyFirebaseUser
where
  S: Send + Sync,
  AppState: FromRef<S>,
{
  type Rejection = (StatusCode, String);

  async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    let TypedHeader(Authorization(bearer)) =
      TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
        .await
        .map_err(http_error_handler(StatusCode::BAD_REQUEST))?;

    let app_state = AppState::from_ref(state);
    match app_state.firebase_auth.verify(bearer.token()) {
      Some(current_user) => Ok(current_user),
      None => Err(http_error(StatusCode::UNAUTHORIZED)),
    }
  }
}

#[derive(Clone)]
pub struct MaybeUser(pub Option<MyFirebaseUser>);

#[async_trait]
impl<S> FromRequestParts<S> for MaybeUser
where
  S: Send + Sync,
  AppState: FromRef<S>,
{
  type Rejection = (StatusCode, String);

  async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    let TypedHeader(Authorization(bearer)) =
      match TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await {
        Ok(b) => b,
        Err(_) => return Ok(Self(None)),
      };

    let app_state = AppState::from_ref(state);
    Ok(Self(app_state.firebase_auth.verify(bearer.token())))
  }
}

fn http_error_handler<E>(status: StatusCode) -> impl Fn(E) -> (StatusCode, String)
where
  E: std::error::Error,
{
  move |err: E| -> (StatusCode, String) { (status, err.to_string()) }
}
fn http_error(status: StatusCode) -> (StatusCode, String) {
  (
    status,
    String::from(status.canonical_reason().unwrap_or(&status.to_string())),
  )
}

impl FromRef<AppState> for FirebaseAuth<MyFirebaseUser> {
  fn from_ref(state: &AppState) -> Self {
    state.firebase_auth.clone()
  }
}

pub struct UnauthorizedResponse {
  msg: String,
}

impl IntoResponse for UnauthorizedResponse {
  fn into_response(self) -> Response {
    (StatusCode::UNAUTHORIZED, self.msg).into_response()
  }
}

impl FromRef<AppState> for UserService {
  fn from_ref(state: &AppState) -> Self {
    state.claims_service.clone()
  }
}
