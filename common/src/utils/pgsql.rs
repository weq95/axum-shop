use std::env;
use std::sync::Arc;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use sqlx::postgres::PgPoolOptions;

pub type ConnPool = sqlx::PgPool;

lazy_static! {
    static ref PG_SQL: AsyncOnce<Arc<ConnPool>> = AsyncOnce::new(async{
         let dns: String = env::var("PG_DNS").unwrap_or("postgres://postgres:123456@localhost:5432".to_string());
         let conn_pool = PgPoolOptions::new().max_connections(15).connect(&dns).await.unwrap();
         Arc::new(conn_pool)
    });
}

pub async fn db<'left_time>() -> &'left_time ConnPool {
    &(**PG_SQL.get().await)
}