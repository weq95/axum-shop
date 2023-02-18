use std::sync::Arc;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use sqlx::postgres::PgPoolOptions;

pub type ConnPool = sqlx::PgPool;

lazy_static! {
    static ref PG_SQL: AsyncOnce<Arc<ConnPool>> = AsyncOnce::new(async{
         let dns: String = dotenv::var("DBTABASE_URL").unwrap();
         let size: u32 = dotenv::var("DBTABASE_POOL_SIZE").unwrap().parse().unwrap();
         let conn_pool = PgPoolOptions::new().max_connections(size).connect(&dns).await.unwrap();
         Arc::new(conn_pool)
    });
}

pub async fn db<'left_time>() -> &'left_time ConnPool {
    &(**PG_SQL.get().await)
}