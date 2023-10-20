pub use deadpool_postgres::{
    Client, PoolError
};
use deadpool_postgres::tokio_postgres::{
    Error, Statement, ToStatement,
    row, types
};
use crate::{PG_POOL, driver};

pub async fn prepare(query: &str) -> Result<Statement, Error> {
    driver::prepare(&PG_POOL, query).await
}

pub async fn query<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<Vec<row::Row>, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query(&PG_POOL, statement, params).await
}

pub async fn query_one<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<row::Row, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query_one(&PG_POOL, statement, params).await
}

pub async fn query_opt<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<Option<row::Row>, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query_opt(&PG_POOL, statement, params).await
}

pub async fn execute<T>(
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<u64, Error>
where
    T: ?Sized + ToStatement,
{
    driver::execute(&PG_POOL, statement, params).await
}

pub async fn prepare_typed_cached(
    query: &str,
    types: &[types::Type],
) -> Result<Statement, Box<dyn std::error::Error>> {
    get().await?
        .prepare_typed_cached(query, types).await
        .map_err(|e| e.into() )
}

pub async fn get() -> Result<Client, PoolError> {
    PG_POOL.get().await
}

pub fn close() {
    driver::close(&PG_POOL);
}

pub fn vec2string<T: std::fmt::Display>(v: &Box<[T]>) -> String {
    v.iter().map(|s| format!("'{}'", s)).collect::<Vec<String>>().join(",")
}
