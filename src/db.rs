use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder};

pub mod games;
pub mod players;
pub mod presents;

#[derive(thiserror::Error, Debug)]
pub enum Error {
  #[error("Not found")]
  NotFound,
  #[error("Empty update set")]
  Empty,
  #[error("Invalid order param")]
  InvalidOrder,
  #[error("Unknown error")]
  Unknown,
  #[error("Unknown sqlx error {0}")]
  Sqlx(#[from] sqlx::Error),
}

#[derive(Deserialize, Debug)]
pub struct ListParams {
  pub order: Option<String>,
  pub offset: Option<i64>,
  pub limit: Option<i64>,
}

pub fn apply_list_filters<'a>(
  mut query: QueryBuilder<'a, Postgres>,
  p: &'a ListParams,
  cols: Vec<&'a str>,
) -> Result<QueryBuilder<'a, Postgres>, Error> {
  if let Some(order) = &p.order {
    let order = get_order_by_sql(order, cols)?;
    query.push(" ORDER BY ");
    query.push(order);
  }
  if let Some(offset) = p.offset {
    query.push(" OFFSET ");
    query.push(offset);
  }
  if let Some(limit) = p.limit {
    query.push(" LIMIT ");
    query.push(limit);
  }
  Ok(query)
}

fn get_order_by_sql(order: &str, cols: Vec<&str>) -> Result<String, Error> {
  let s: String;
  let sort = if order.starts_with('-') {
    s = order.chars().skip(1).collect();
    "desc"
  } else {
    s = order.to_string();
    "asc"
  };
  for c in cols {
    if c == s {
      return Ok(format!("{} {}", c, sort));
    }
  }
  Err(Error::InvalidOrder)
}

pub fn handle_pg_error(err: sqlx::Error) -> Error {
  match err {
    sqlx::Error::RowNotFound => Error::NotFound,
    _ => Error::Sqlx(err),
  }
}

#[derive(sqlx::FromRow, Serialize, Debug)]
pub struct CreateResult<T: Serialize> {
  pub id: T,
  pub created_at: NaiveDateTime,
}

#[derive(sqlx::FromRow, Serialize, Debug)]
pub struct UpdateResult {
  pub updated_at: NaiveDateTime,
}

// check health
pub async fn health(db: &sqlx::PgPool) -> Result<(), Error> {
  let res: Result<(i32,), sqlx::Error> = sqlx::query_as("SELECT 1").fetch_one(db).await;
  match res {
    Ok(row) if row.0 == 1 => Ok(()),
    Err(err) => Err(Error::Sqlx(err)),
    _ => Err(Error::Unknown),
  }
}
