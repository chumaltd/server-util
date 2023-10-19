use deadpool_postgres::{
    Pool,
    tokio_postgres::{
        Error, Statement, ToStatement,
        row, types
    }
};

pub async fn prepare(pool: &Pool, query: &str) -> Result<Statement, Error> {
    let client = pool.get().await.unwrap();
    client.prepare(query).await
}

pub async fn query<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<Vec<row::Row>, Error>
where
    T: ?Sized + ToStatement,
{
    let client = pool.get().await.unwrap();
    client.query(statement, params).await
}

pub async fn query_one<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<row::Row, Error>
where
    T: ?Sized + ToStatement,
{
    let client = pool.get().await.unwrap();
    client.query_one(statement, params).await
}

pub async fn query_opt<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
) -> Result<Option<row::Row>, Error>
where
    T: ?Sized + ToStatement,
{
    let client = pool.get().await.unwrap();
    client.query_opt(statement, params).await
}

pub async fn execute<T>(
    pool: &Pool,
    statement: &T,
    params: &[&(dyn types::ToSql + Sync)]
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
