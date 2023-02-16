use std::env;
use std::sync::Arc;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use sqlx::PgPool;

pub mod user;
pub mod address;

lazy_static! {
    static ref PG_SQL: AsyncOnce<Arc<PgPool>> = AsyncOnce::new(async{
         let dns: String = env::var("PG_DNS").unwrap_or("postgres://postgres:123456@localhost:5432".to_string());

         Arc::new(PgPool::connect(&dns).await.unwrap())
    });
}

async fn db<'left_time>() -> &'left_time PgPool {
    &(**PG_SQL.get().await)
}