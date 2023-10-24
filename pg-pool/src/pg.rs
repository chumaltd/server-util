pub use deadpool_postgres::{
    Client, PoolError
};
use deadpool_postgres::tokio_postgres::{
    Error, Statement, ToStatement,
    types::ToSql
};
use crate::{PG_POOL, Row, Type, driver};

pub async fn prepare(query: &str) -> Result<Statement, Error> {
    driver::prepare(&PG_POOL, query).await
}

pub async fn query<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query(&PG_POOL, statement, params).await
}

pub async fn query_pp(
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    driver::query_pp(&PG_POOL, query, types, params).await
}

pub async fn query_one<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query_one(&PG_POOL, statement, params).await
}

pub async fn query_one_pp(
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    driver::query_one_pp(&PG_POOL, query, types, params).await
}

pub async fn query_opt<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Option<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    driver::query_opt(&PG_POOL, statement, params).await
}

pub async fn execute<T>(
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<u64, Error>
where
    T: ?Sized + ToStatement,
{
    driver::execute(&PG_POOL, statement, params).await
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
    PG_POOL.get().await
}

pub fn close() {
    driver::close(&PG_POOL);
}

pub fn vec2string<T: std::fmt::Display>(v: &Box<[T]>) -> String {
    v.iter().map(|s| format!("'{}'", s)).collect::<Vec<String>>().join(",")
}
