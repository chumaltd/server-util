use pg_pool::{pg, pgr, Type};
use tokio_postgres::error::SqlState;

#[tokio::test]
async fn invalid_prepare_statement() {
    let stmt = pg::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();
    let row = pg::query_one(&stmt, &[&8i64]).await.unwrap();

    pg::execute("DEALLOCATE ALL", &[]).await.unwrap();
    let result = pg::query_one(&stmt, &[&9i64]).await;
    assert!(result.is_err());
    assert_eq!(SqlState::from_code("0000"), result.unwrap_err().code().unwrap().clone());
}
