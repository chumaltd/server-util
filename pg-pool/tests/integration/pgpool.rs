use pg_pool::{pg, pgr};

#[tokio::test]
async fn pg_query() {
  let rows = pg::query("SELECT 1 + $1", &[&1i32]).await.unwrap();
  let result: i32 = rows[0].get(0);
  assert_eq!(result, 2i32);
}

#[tokio::test]
async fn pgr_query() {
  let rows = pgr::query("SELECT 1 + $1", &[&1i32]).await.unwrap();
  let result: i32 = rows[0].get(0);
  assert_eq!(result, 2i32);
}

#[tokio::test]
async fn pg_query_one() {
  let row = pg::query_one("SELECT 1 + $1", &[&1i32]).await.unwrap();
  let result: i32 = row.get(0);
  assert_eq!(result, 2i32);
}

#[tokio::test]
async fn pgr_query_one() {
  let row = pgr::query_one("SELECT 1 + $1", &[&1i32]).await.unwrap();
  let result: i32 = row.get(0);
  assert_eq!(result, 2i32);
}
