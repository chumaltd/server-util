pub use deadpool_postgres::{
    Client, PoolError
};
use deadpool_postgres::tokio_postgres::{
    Error, Statement, ToStatement,
    types::ToSql
};
use std::sync::LazyLock;
use crate::{PGR_POOL, Row, Type, driver::{self, PgPool}};

pub async fn prepare(query: &str) -> Result<Statement, Error> {
    driver::prepare(PgPool::Reader, query).await
}

pub async fn query<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query(PgPool::Reader, statement, params).await
}

pub async fn query_pp(
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    driver::query_pp(PgPool::Reader, query, types, params).await
}

pub async fn query_one<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query_one(PgPool::Reader, statement, params).await
}

pub async fn query_one_pp(
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    driver::query_one_pp(PgPool::Reader, query, types, params).await
}

pub async fn query_opt<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Option<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query_opt(PgPool::Reader, statement, params).await
}

pub async fn execute<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<u64, Error>
where
    T: ?Sized + ToStatement,
{
    driver::execute(PgPool::Reader, statement, params).await
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
    driver::get(PgPool::Reader).await
}

pub fn close() {
    if let Some(pool) = LazyLock::force(&PGR_POOL) {
        driver::close(pool);
    }
}

pub fn vec2string<T: std::fmt::Display>(v: &Box<[T]>) -> String {
    v.iter().map(|s| format!("'{}'", s)).collect::<Vec<String>>().join(",")
}
