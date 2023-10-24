use pg_pool::{pg, pgr};

#[tokio::test]
async fn invalid_prepare_statement() {
    let stmt = pg::prepare_cached("SELECT 1 + 2").await.unwrap();
    let row = pg::query_one(&stmt, &[]).await.unwrap();

    pg::execute("DEALLOCATE ALL", &[]).await.unwrap();
    let result = pg::query_one(&stmt, &[]).await;
    assert!(result.is_err());
}
