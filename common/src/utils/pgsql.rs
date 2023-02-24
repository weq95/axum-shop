use std::ops::Deref;
use std::sync::Arc;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use sqlx::postgres::PgPoolOptions;

use crate::casbin::PgSqlAdapter;

pub type ConnPool = sqlx::PgPool;

lazy_static! {
    static ref PG_SQL: AsyncOnce<Arc<ConnPool>> = AsyncOnce::new(async {
        let dns: String = dotenv::var("DBTABASE_URL").unwrap();
        let size: u32 = dotenv::var("DBTABASE_POOL_SIZE").unwrap().parse().unwrap();
        let conn_pool = PgPoolOptions::new()
            .max_connections(size)
            .connect(&dns)
            .await
            .unwrap();
        Arc::new(conn_pool)
    });
    pub static ref PG_ADAPTER: AsyncOnce<Arc<PgSqlAdapter>> =
        AsyncOnce::new(async { Arc::new(PgSqlAdapter::new(db().await).await) });
}

pub async fn db<'left_time>() -> &'left_time ConnPool {
    &(**PG_SQL.get().await)
}

pub async fn get_pg_adapter() -> PgSqlAdapter {
    PG_ADAPTER.get().await.clone().deref().clone()
}
