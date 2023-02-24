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
        let dns = dotenv::var("REDIS_URL").unwrap();
        let size: u32 = dotenv::var("REDIS_POOL_SIZE").unwrap().parse().unwrap();

       let manager = RedisConnectionManager::new(dns).unwrap();
       r2d2::Pool::builder().max_size(size).build(manager).unwrap()
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
