pub use deadpool_postgres::{
    Client, PoolError
};
use deadpool_postgres::tokio_postgres::{
    Error, Statement, ToStatement,
    types::ToSql
};
use log::debug;
use once_cell::sync::Lazy;
use server_conf::SV_CONF;
use crate::{PGR_POOL, Row, Type, driver, pg};

pub async fn prepare(query: &str) -> Result<Statement, Error> {
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::prepare(pool, query).await,
        None => pg::prepare(query).await
    }
}

pub async fn query<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::query(pool, statement, params).await,
        None => pg::query(statement, params).await
    }
}

pub async fn query_pp(
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::query_pp(pool, query, types, params).await,
        None => pg::query_pp(query, types, params).await
    }
}

pub async fn query_one<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Error>
where
    T: ?Sized + ToStatement,
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::query_one(pool, statement, params).await,
        None => pg::query_one(statement, params).await
    }
}

pub async fn query_one_pp(
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::query_one_pp(pool, query, types, params).await,
        None => pg::query_one_pp(query, types, params).await
    }
}

pub async fn query_opt<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Option<Row>, Error>
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
    params: &[&(dyn ToSql + Sync)]
) -> Result<u64, Error>
where
    T: ?Sized + ToStatement,
{
    match Lazy::force(&PGR_POOL) {
        Some(pool) => driver::execute(pool, statement, params).await,
        None => pg::execute(statement, params).await
    }
}

pub async fn prepare_typed_cached(
    query: &str,
    types: &[Type],
) -> Result<Statement, Box<dyn std::error::Error>> {
    get().await?
        .prepare_typed_cached(query, types).await
        .map_err(|e| e.into() )
}

pub async fn get() -> Result<Client, PoolError> {
    match Lazy::force(&PGR_POOL) {
        Some(pool) => {
            let result = pool.get().await;
            if SV_CONF.dbr.as_ref().unwrap().fallback && result.is_err() {
                debug!("Fallback to writer DB: {}", result.unwrap_err());
                return pg::get().await;
            }

            result
        },
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
