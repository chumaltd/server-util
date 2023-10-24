use deadpool_postgres::{
    Pool,
    tokio_postgres::{
        Error, Statement, ToStatement,
        error::SqlState, types::ToSql
    }
};
use crate::{Row, Type};

pub async fn prepare(pool: &Pool, query: &str) -> Result<Statement, Error> {
    let client = pool.get().await.unwrap();
    client.prepare(query).await
}

pub async fn query<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    let client = pool.get().await.unwrap();
    client.query(statement, params).await
}

pub async fn query_pp(
    pool: &Pool,
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Vec<Row>, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let client = pool.get().await?;
    let stmt = client.prepare_typed_cached(query, types).await?;
    let result = self::query(pool, &stmt, params).await;
    if result.is_ok() { return Ok(result?); }

    let err = result.unwrap_err();
    if err.is_closed() ||
        err.code() == Some(&SqlState::UNDEFINED_PSTATEMENT) {
            let stmt2 = client.prepare_typed_cached(query, types).await?;
            return Ok(self::query(pool, &stmt2, params).await
                      .map_err(|e| Box::new(e))?);
        }
    Err(Box::new(err))
}

pub async fn query_one<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Error>
where
    T: ?Sized + ToStatement,
{
    let client = pool.get().await.unwrap();
    client.query_one(statement, params).await
}

pub async fn query_one_pp(
    pool: &Pool,
    query: &str,
    types: &[Type],
    params: &[&(dyn ToSql + Sync)]
) -> Result<Row, Box<dyn std::error::Error + Send + Sync + 'static>>
{
    let client = pool.get().await?;
    let stmt = client.prepare_typed_cached(query, types).await?;
    let result = query_one(pool, &stmt, params).await;
    if result.is_ok() { return Ok(result?); }

    let err = result.unwrap_err();
    if err.is_closed() ||
        err.code() == Some(&SqlState::UNDEFINED_PSTATEMENT) {
            let stmt2 = client.prepare_typed_cached(query, types).await?;
            return Ok(query_one(pool, &stmt2, params).await
                      .map_err(|e| Box::new(e))?);
        }
    Err(Box::new(err))
}

pub async fn query_opt<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<Option<Row>, Error>
where
    T: ?Sized + ToStatement,
{
    let client = pool.get().await.unwrap();
    client.query_opt(statement, params).await
}

pub async fn execute<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn ToSql + Sync)]
) -> Result<u64, Error>
where
    T: ?Sized + ToStatement,
{
    let client = pool.get().await.unwrap();
    client.execute(statement, params).await
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
