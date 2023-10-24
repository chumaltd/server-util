use pg_pool::{pg, pgr, Type};
use tokio_postgres::error::SqlState;

#[tokio::test]
async fn invalid_prepare_statement_pg() {
    let stmt = pg::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();
    let row = pg::query_one(&stmt, &[&8i64]).await.unwrap();

    invalidate_pg().await;
    let result = pg::query_one(&stmt, &[&9i64]).await;
    assert!(result.is_err());
    assert_eq!(Some(&SqlState::UNDEFINED_PSTATEMENT), result.unwrap_err().code());

    let result2 = pg::query(&stmt, &[&9i64]).await;
    assert!(result2.is_err());
    assert_eq!(Some(&SqlState::UNDEFINED_PSTATEMENT), result2.unwrap_err().code());
}

#[tokio::test]
async fn query_pp_recover_invalid_statement_pg() {
    let _stmt = pg::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();

    invalidate_pg().await;
    let result = pg::query_pp("SELECT 1 + $1", &[Type::INT8], &[&9i64]).await;
    assert!(result.is_ok());
    let data: i64 = result.unwrap()[0].get(0);
    assert_eq!(data, 10i64);
}

#[tokio::test]
async fn query_one_pp_recover_invalid_statement_pg() {
    let _stmt = pg::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();

    invalidate_pg().await;
    let result = pg::query_one_pp("SELECT 1 + $1", &[Type::INT8], &[&9i64]).await;
    assert!(result.is_ok());
    let data: i64 = result.unwrap().get(0);
    assert_eq!(data, 10i64);
}

#[tokio::test]
async fn invalid_prepare_statement_pgr() {
    let stmt = pgr::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();
    let row = pgr::query_one(&stmt, &[&8i64]).await.unwrap();

    invalidate_pgr().await;
    let result = pgr::query_one(&stmt, &[&9i64]).await;
    assert!(result.is_err());
    assert_eq!(Some(&SqlState::UNDEFINED_PSTATEMENT), result.unwrap_err().code());

    let result2 = pgr::query(&stmt, &[&9i64]).await;
    assert!(result2.is_err());
    assert_eq!(Some(&SqlState::UNDEFINED_PSTATEMENT), result2.unwrap_err().code());
}

#[tokio::test]
async fn query_pp_recover_invalid_statement_pgr() {
    let _stmt = pgr::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();

    invalidate_pgr().await;
    let result = pgr::query_pp("SELECT 1 + $1", &[Type::INT8], &[&9i64]).await;
    assert!(result.is_ok());
    let data: i64 = result.unwrap()[0].get(0);
    assert_eq!(data, 10i64);
}

#[tokio::test]
async fn query_one_pp_recover_invalid_statement_pgr() {
    let _stmt = pgr::prepare_typed_cached("SELECT 1 + $1", &[Type::INT8]).await.unwrap();

    invalidate_pgr().await;
    let result = pgr::query_one_pp("SELECT 1 + $1", &[Type::INT8], &[&9i64]).await;
    assert!(result.is_ok());
    let data: i64 = result.unwrap().get(0);
    assert_eq!(data, 10i64);
}

async fn invalidate_pg() {
    pg::execute("DEALLOCATE ALL", &[]).await.unwrap();
}

async fn invalidate_pgr() {
    pgr::execute("DEALLOCATE ALL", &[]).await.unwrap();
}
