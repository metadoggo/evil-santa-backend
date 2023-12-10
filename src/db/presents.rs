use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as, PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use super::{apply_list_filters, handle_pg_error, CreateResult, Error, ListParams, UpdateResult};

#[derive(FromRow, Serialize)]
pub struct Present {
  pub id: i64,
  pub game_id: Uuid,
  pub name: String,
  pub player_id: Option<i64>,
  pub wrapped_images: Vec<String>,
  pub unwrapped_images: Vec<String>,
  pub created_at: NaiveDateTime,
  pub updated_at: Option<NaiveDateTime>,
}

// list presents
pub async fn list(db: &PgPool, game_id: Uuid, p: ListParams) -> Result<Vec<Present>, Error> {
  let mut query = QueryBuilder::<Postgres>::new(
        "SELECT id, game_id, name, wrapped_images, unwrapped_images, player_id, created_at, updated_at FROM presents WHERE game_id = $1",
    );
  query = apply_list_filters(query, &p, vec!["id", "name"])?;

  query
    .build_query_as()
    .bind(game_id)
    .fetch_all(db)
    .await
    .map_err(Error::Sqlx)
}

// get a present
pub async fn get(db: &PgPool, id: i64) -> Result<Present, Error> {
  query_as(
        "SELECT id, game_id, name, wrapped_images, unwrapped_images, player_id, created_at, updated_at FROM presents WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)
}

#[derive(Deserialize)]
pub struct CreateParams {
  pub name: String,
  pub wrapped_images: Option<Vec<String>>,
  pub unwrapped_images: Option<Vec<String>>,
}

// create a present
pub async fn create(
  db: &PgPool,
  game_id: Uuid,
  p: CreateParams,
) -> Result<CreateResult<i64>, Error> {
  query_as(
        "INSERT INTO presents (game_id, name, wrapped_images, unwrapped_images) VALUES ($1, $2, $3, $4) RETURNING id, created_at",
    )
    .bind(game_id)
    .bind(p.name)
    .bind(p.wrapped_images.unwrap_or_default())
    .bind(p.unwrapped_images.unwrap_or_default())
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)
}

#[derive(Deserialize)]
pub struct UpdateParams {
  pub name: Option<String>,
  pub wrapped_images: Option<Vec<String>>,
  pub unwrapped_images: Option<Vec<String>>,
  pub player_id: Option<i16>,
}

// update a present
pub async fn update(db: &PgPool, id: i64, p: UpdateParams) -> Result<UpdateResult, Error> {
  let mut query = QueryBuilder::<Postgres>::new("UPDATE presents SET");
  let mut sep = query.separated(", ");
  if let Some(name) = p.name {
    sep.push(" name = ").push_bind_unseparated(name);
  }
  if let Some(wrapped_images) = p.wrapped_images {
    sep
      .push(" wrapped_images = ")
      .push_bind_unseparated(wrapped_images);
  }
  if let Some(unwrapped_images) = p.unwrapped_images {
    sep
      .push(" unwrapped_images = ")
      .push_bind_unseparated(unwrapped_images);
  }
  if let Some(player_id) = p.player_id {
    sep.push(" player_id = ").push_bind_unseparated(player_id);
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
  pub wrapped_images: Option<Vec<String>>,
  pub unwrapped_images: Option<Vec<String>>,
  pub player_id: Option<i16>,
}

// replace a present
pub async fn replace(db: &PgPool, id: i64, p: ReplaceParams) -> Result<UpdateResult, Error> {
  let mut query = QueryBuilder::<Postgres>::new("UPDATE presents SET");
  let mut sep = query.separated(", ");
  sep.push(" name = ").push_bind_unseparated(p.name);
  sep
    .push(" wrapped_images = ")
    .push_bind_unseparated(p.wrapped_images.unwrap_or_default());
  sep
    .push(" unwrapped_images = ")
    .push_bind_unseparated(p.unwrapped_images.unwrap_or_default());
  sep.push(" player_id = ").push_bind_unseparated(p.player_id);
  sep.push(" updated_at = NOW()");
  query.push(" WHERE id = ").push_bind(id);
  query.push(" RETURNING updated_at");
  query
    .build_query_as()
    .fetch_one(db)
    .await
    .map_err(handle_pg_error)
}

// delete a present
pub async fn delete(db: &PgPool, id: i64) -> Result<(), Error> {
  match sqlx::query("DELETE FROM presents WHERE id = $1")
    .bind(id)
    .execute(db)
    .await
  {
    Ok(_) => Ok(()),
    Err(err) => Err(handle_pg_error(err)),
  }
}
