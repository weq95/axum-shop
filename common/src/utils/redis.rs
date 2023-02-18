use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use rustis::bb8::{Pool, PooledConnection as RedisConn};
use rustis::client::PooledClientManager as RedisManager;
use rustis::commands::{JsonCommands, JsonGetOptions};
use serde::{Deserialize, Serialize};

use crate::error::ApiResult;

lazy_static! {
    static ref REDIS_CLIENT: AsyncOnce<Arc<Pool<RedisManager>>> = AsyncOnce::new(async {
        // redis|rediss://[[<username>]:<password>@]<host>[:<port>][/<database>]
        let dns = dotenv::var("REDIS_URL").unwrap();
        let size: u32 = dotenv::var("REDIS_POOL_SIZE").unwrap().parse().unwrap();
        let manager = RedisManager::new(dns).unwrap();
        let pool = rustis::bb8::Pool::builder().max_size(size).build(manager).await.unwrap();

        Arc::new( pool)
    });
}

pub async fn get(key: String) -> ApiResult<String> {
    let mut client: RedisConn<RedisManager> = REDIS_CLIENT.get().await.get().await?;
    let result: String = client.json_get(key, JsonGetOptions::default()).await?;

    Ok(result)
}

/*async fn comm_t<'a, T: Deserialize<'a>>(key: &'a str, val: T) -> ApiResult<T> {
    let mut client: RedisConn<RedisManager> = REDIS_CLIENT.get().await.get().await?;
    let result: String = client.json_get(key, JsonGetOptions::default()).await?;

    // result 到后边就被销掉了, 所以这里不知道怎么解决
    Ok(serde_json::from_str::<val>(result.as_str())?)
}*/

/// 公共转类型方法
pub async fn comm_to<'a, T: Deserialize<'a>>(result: &'a str) -> ApiResult<T> {
    Ok(serde_json::from_str::<T>(result)?)
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SchoolJson {
    pub name: String,
    pub description: String,
    pub class: String,
    #[serde(rename = "type")]
    pub type_data: Vec<String>,
    pub address: HashMap<String, String>,
    pub students: i64,
    pub location: String,
    pub status_log: Vec<String>,
}