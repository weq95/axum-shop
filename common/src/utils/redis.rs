use std::borrow::BorrowMut;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use r2d2_redis::r2d2::PooledConnection;
use r2d2_redis::{r2d2, r2d2::Pool, redis, RedisConnectionManager};
use serde::de::DeserializeOwned;

use crate::error::{ApiError, ApiResult};

lazy_static! {
   pub static ref REDIS_CLIENT: AsyncOnce<Pool<RedisConnectionManager>> = AsyncOnce::new(async {
        // redis|rediss://[[<username>]:<password>@]<host>[:<port>][/<database>]

        let cfg = &crate::application_config().await.redis;
        let dns = format!("redis://{}:{}@{}:{}/{}",
        cfg.username.clone(),
        cfg.password.clone(),
        cfg.host.clone(),
        cfg.port,
        cfg.db);

       let manager = RedisConnectionManager::new(dns).unwrap();
       r2d2::Pool::builder().max_size(cfg.pool_size).build(manager).unwrap()
    });
}

pub async fn get_conn_manager() -> PooledConnection<RedisConnectionManager> {
    REDIS_CLIENT.get().await.clone().get().unwrap()
}

pub async fn json_get<T: DeserializeOwned>(
    conn: &mut redis::Connection,
    key: &str,
    field: &str,
) -> ApiResult<T> {
    let mut binding = redis::cmd("JSON.GET");

    let cmd = binding.arg(key).arg(format!("$.{}", field));
    let result: Option<String> = {
        let cmd = cmd.clone();
        cmd.query(conn.borrow_mut())?
    };

    if let Some(data) = result {
        let value: T = serde_json::from_str(&data)?;
        return Ok(value);
    }

    Err(ApiError::Error("ket not found".to_string()))
}
