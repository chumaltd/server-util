pub use deadpool_postgres::{
    Client, PoolError
};
use deadpool_postgres::tokio_postgres::{
    Error, Statement, ToStatement,
    row, types
};
use once_cell::sync::Lazy;
use crate::{PGR_POOL, driver, pg};

pub async fn prepare(query: &str) -> Result<Statement, Error> {
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::prepare(pool, query).await,
        None => pg::prepare(query).await
    }
}

pub async fn query<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<Vec<row::Row>, Error>
where
    T: ?Sized + ToStatement,
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::query(pool, statement, params).await,
        None => pg::query(statement, params).await
    }
}

pub async fn query_one<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<row::Row, Error>
where
    T: ?Sized + ToStatement,
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::query_one(pool, statement, params).await,
        None => pg::query_one(statement, params).await
    }
}

pub async fn query_opt<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<Option<row::Row>, Error>
where
    T: ?Sized + ToStatement,
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::query_opt(pool, statement, params).await,
        None => pg::query_opt(statement, params).await
    }
}

pub async fn execute<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<u64, Error>
where
    T: ?Sized + ToStatement,
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::execute(pool, statement, params).await,
        None => pg::execute(statement, params).await
    }
}

pub async fn get() -> Result<Client, PoolError> {
    match Lazy::force(&PGR_POOL) {
        Some(pool) => pool.get().await,
        None => pg::get().await
    }
}

pub fn close() {
    if let Some(pool) = Lazy::force(&PGR_POOL) {
        driver::close(pool);
    }
}

pub fn vec2string<T: std::fmt::Display>(v: &Box<[T]>) -> String {
    v.iter().map(|s| format!("'{}'", s)).collect::<Vec<String>>().join(",")
}
