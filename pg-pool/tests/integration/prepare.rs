use pg_pool::{pg, pgr, Type};

#[tokio::test]
async fn invalid_prepare_statement() {
    let stmt = pg::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();
    let row = pg::query_one(&stmt, &[&8i64]).await.unwrap();

    pg::execute("DEALLOCATE ALL", &[]).await.unwrap();
    let result = pg::query_one(&stmt, &[&9i64]).await;
    assert!(result.is_err_and(|err| err.is_closed() ));
}
