use deadpool_postgres::{
    Client,
    Pool, PoolError,
    tokio_postgres::{
        Error, Statement, ToStatement,
        error::SqlState, types::ToSql
    }
};
use deadpool_postgres::TimeoutType;
use crate::{PG_POOL, PGR_POOL, Row, Type};
use log::{debug, warn};
use server_conf::SV_CONF;
use std::sync::LazyLock;
use std::time::Duration;

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
    let (client, origin) = get_with_origin(pool).await.unwrap();
    let stmt = client.prepare_typed_cached(query, types).await?;
    let result = client.query(&stmt, params).await;
    if result.is_ok() { return Ok(result?); }

    let err = result.unwrap_err();
    if !(err.is_closed() || err.code() == Some(&SqlState::UNDEFINED_PSTATEMENT)) {
        return Err(Box::new(err));
    }

    client.statement_cache.clear();
    drop(client);
    let (fresh, _) = get_fresh(pool, origin).await?;
    let stmt2 = fresh.prepare_typed_cached(query, types).await?;
    Ok(fresh.query(&stmt2, params).await.map_err(|e| Box::new(e))?)
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
    let (client, origin) = get_with_origin(pool).await.unwrap();
    let stmt = client.prepare_typed_cached(query, types).await?;
    let result = client.query_one(&stmt, params).await;
    if result.is_ok() { return Ok(result?); }

    let err = result.unwrap_err();
    if !(err.is_closed() || err.code() == Some(&SqlState::UNDEFINED_PSTATEMENT)) {
        return Err(Box::new(err));
    }

    client.statement_cache.clear();
    drop(client);
    let (fresh, _) = get_fresh(pool, origin).await?;
    let stmt2 = fresh.prepare_typed_cached(query, types).await?;
    Ok(fresh.query_one(&stmt2, params).await.map_err(|e| Box::new(e))?)
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
    get_with_origin(pool).await.map(|(c, _)| c)
}

pub(crate) async fn get_with_origin(pool: PgPool)
    -> Result<(Client, &'static Pool), PoolError>
{
    if pool == PgPool::Writer || LazyLock::force(&PGR_POOL).is_none() {
        let client = get_with_retry(LazyLock::force(&PG_POOL)).await?;
        return Ok((client, LazyLock::force(&PG_POOL)));
    }

    let pgr: &'static Pool = LazyLock::force(&PGR_POOL).as_ref().unwrap();
    let result = get_with_retry(pgr).await;
    match result {
        Ok(client) => Ok((client, pgr)),
        Err(e) => {
            if SV_CONF.dbr.as_ref().unwrap().fallback {
                debug!("Fallback to writer DB: {e}");
                let client = get_with_retry(LazyLock::force(&PG_POOL)).await?;
                Ok((client, LazyLock::force(&PG_POOL)))
            } else {
                Err(e)
            }
        }
    }
}

async fn get_fresh(
    pool: PgPool,
    prev_origin: &'static Pool,
) -> Result<(Client, &'static Pool), PoolError> {
    prev_origin.manager().statement_caches.clear();
    get_with_origin(pool).await
}

const MAX_RETRIES: usize = 2;
const BACKOFF_MS: [u64; 2] = [200, 500];

async fn get_with_retry(pool: &Pool) -> Result<Client, PoolError> {
    let mut last_err = None;
    for attempt in 0..=MAX_RETRIES {
        match pool.get().await {
            Ok(client) if client.is_closed() => {
                // RST received while idle: drop the dead connection and retry immediately.
                // Dropping triggers deadpool recycle which discards the connection and
                // replenishes the pool slot, so the next iteration gets a fresh one.
                if attempt == MAX_RETRIES {
                    return Err(last_err.unwrap_or(PoolError::Timeout(TimeoutType::Recycle)));
                }
                warn!("Pool returned closed connection (attempt {}/{}), retrying",
                    attempt + 1, MAX_RETRIES + 1);
            }
            Ok(client) => return Ok(client),
            Err(e) => {
                let retryable = matches!(&e,
                    PoolError::Timeout(TimeoutType::Create) |
                    PoolError::Timeout(TimeoutType::Recycle) |
                    PoolError::Backend(_)
                );
                if !retryable || attempt == MAX_RETRIES {
                    return Err(e);
                }
                warn!("Pool get failed (attempt {}/{}): {}", attempt + 1, MAX_RETRIES + 1, e);
                tokio::time::sleep(Duration::from_millis(BACKOFF_MS[attempt])).await;
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or(PoolError::Timeout(TimeoutType::Recycle)))
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
