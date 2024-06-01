use deadpool_postgres::{
    Client,
    Pool, PoolError,
    tokio_postgres::{
        Error, Statement, ToStatement,
        error::SqlState, types::ToSql
    }
};
use crate::{PG_POOL, PGR_POOL, Row, Type};
use log::debug;
use once_cell::sync::Lazy;
use server_conf::SV_CONF;

#[derive(Copy, Clone, PartialEq)]
pub enum PgPool {
    Writer = 0,
    Reader = 1,
}

pub async fn prepare(pool: PgPool, query: &str) -> Result<Statement, Error> {
    let client = get(pool).await.unwrap();
    client.prepare(query).await
}

pub async fn query<T>(
    pool: PgPool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    let client = get(pool).await.unwrap();
    client.query(statement, params).await
}

pub async fn query_pp(
    pool: PgPool,
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let client = get(pool).await.unwrap();
    let stmt = client.prepare_typed_cached(query, types).await?;
    let result = client.query(&stmt, params).await;
    if result.is_ok() { return Ok(result?); }

    let err = result.unwrap_err();
    if err.is_closed() ||
        err.code() == Some(&SqlState::UNDEFINED_PSTATEMENT) {
            client.statement_cache.clear();
            let stmt2 = client.prepare_typed_cached(query, types).await?;
            return Ok(client.query(&stmt2, params).await
                      .map_err(|e| Box::new(e))?);
        }
    Err(Box::new(err))
}

pub async fn query_one<T>(
    pool: PgPool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Error>
where
    T: ?Sized + ToStatement,
{
    let client = get(pool).await.unwrap();
    client.query_one(statement, params).await
}

pub async fn query_one_pp(
    pool: PgPool,
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let client = get(pool).await.unwrap();
    let stmt = client.prepare_typed_cached(query, types).await?;
    let result = client.query_one(&stmt, params).await;
    if result.is_ok() { return Ok(result?); }

    let err = result.unwrap_err();
    if err.is_closed() ||
        err.code() == Some(&SqlState::UNDEFINED_PSTATEMENT) {
            client.statement_cache.clear();
            let stmt2 = client.prepare_typed_cached(query, types).await?;
            return Ok(client.query_one(&stmt2, params).await
                      .map_err(|e| Box::new(e))?);
        }
    Err(Box::new(err))
}

pub async fn query_opt<T>(
    pool: PgPool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Option<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    let client = get(pool).await.unwrap();
    client.query_opt(statement, params).await
}

pub async fn execute<T>(
    pool: PgPool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<u64, Error>
where
    T: ?Sized + ToStatement,
{
    let client = get(pool).await.unwrap();
    client.execute(statement, params).await
}

pub async fn get(pool: PgPool) -> Result<Client, PoolError> {
    if pool == PgPool::Writer || Lazy::force(&PGR_POOL).is_none() {
        return PG_POOL.get().await;
    }

    let result = PGR_POOL.as_ref().unwrap().get().await;
    if SV_CONF.dbr.as_ref().unwrap().fallback && result.is_err() {
        debug!("Fallback to writer DB: {}", result.unwrap_err());
        return PG_POOL.get().await;
    }

    result
}

pub fn close(pool: &Pool) {
    pool.close();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
