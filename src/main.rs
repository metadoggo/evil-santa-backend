use std::{env, fs::File, path::Path, str::FromStr};

use firebase_auth::FirebaseAuth;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgListener;
use tower_http::{
  cors::{Any, CorsLayer},
  trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{
  prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

use crate::{
  auth::{user::UserService, MyFirebaseUser, ServiceAccount},
  db::games::{start_listening, PlayEvent},
};
use tokio::sync::broadcast::channel;

mod api;
mod auth;
mod db;

static MIGRATOR: Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() {
  println!("{}", option_env!("RELEASE_VERSION").unwrap_or("v0.0.0-dev"));

  run().await;
}

async fn run<'a>() {
  let log_level = LevelFilter::from_str(&env::var("LOG_LEVEL").unwrap_or(String::from("info")))
    .unwrap_or(LevelFilter::INFO);
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::fmt::layer()
        .compact()
        .without_time()
        .with_file(false)
        .with_line_number(false)
        .with_target(false)
        .with_filter(log_level),
    )
    .init();
  tracing::info!("Log level: {}", log_level);

  tracing::info!("Initialising Firebase client...");
  let sa_path = env::var("FIREBASE_SERVICE_ACCOUNT_PATH")
    .expect("FIREBASE_SERVICE_ACCOUNT_PATH is missing from env");
  let sa_reader = File::open(Path::new(&sa_path)).expect(&format!("Error opening {}", sa_path));
  let firebase_sa: ServiceAccount =
    serde_json::from_reader(sa_reader).expect(&format!("Error reading {}", sa_path));
  let firebase_auth = FirebaseAuth::<MyFirebaseUser>::new(&firebase_sa.project_id).await;
  let claims_service = UserService::new(
    &env::var("FIREBASE_API_KEY").expect("FIREBASE_API_KEY is missing from env"),
    firebase_sa,
  );

  tracing::info!("Preparing DB connection...");
  let db_url = &env::var("DATABASE_URL").expect("DATABASE_URL is missing from env");
  let sqlx_pool = sqlx::PgPool::connect(db_url).await.unwrap();
  MIGRATOR.run(&sqlx_pool).await.unwrap();
  let listener = PgListener::connect_with(&sqlx_pool).await.unwrap();
  let (tx, _rx) = channel::<PlayEvent>(10);

  tracing::info!("Crating service...");
  let server = api::Server::new(sqlx_pool, firebase_auth, claims_service, tx.clone());

  tracing::info!("Spawning PG => SSE worker...");
  tokio::spawn(async move {
    match start_listening(listener, &tx).await {
      Ok(()) => {
        tracing::info!("PG Listener ok")
      }
      Err(err) => {
        tracing::error!("Error listening to PG: {}", err.to_string())
      }
    };
  });

  tracing::info!("Starting service...");
  let cors = CorsLayer::new()
    .allow_methods(Any)
    .allow_origin(Any)
    .allow_headers(Any);
  let trace = TraceLayer::new_for_http()
    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
    .on_request(DefaultOnRequest::new().level(Level::INFO))
    .on_response(DefaultOnResponse::new().level(Level::INFO));
  let layers = tower::ServiceBuilder::new().layer(trace).layer(cors);
  let addr = format!(
    "{}:{}",
    env::var("HOST").unwrap_or(String::from("localhost")),
    env::var("PORT").unwrap_or(String::from("3000"))
  );
  tracing::info!("🚀 Listening on http://{}", &addr);
  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  axum::serve(listener, server.router.layer(layers).into_make_service())
    .await
    .unwrap();
}
