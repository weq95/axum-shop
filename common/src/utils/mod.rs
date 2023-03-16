use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use url::form_urlencoded::{byte_serialize, parse};

use crate::casbin::PgSqlAdapter;
use crate::error::{ApiError, ApiResult};

pub mod casbin;
pub mod jwt;
pub mod pwd;
pub mod redis;

/// 图片存储跟路径
pub const IMAGES_PATH: &str = "./files/images/";

lazy_static! {
    // pgsql 连接池
    static ref PG_SQL: AsyncOnce<Arc<ConnPool>> = AsyncOnce::new(async {
        let cfg = &crate::application_config().await.postgres;
        let dns: String = format!(
            "postgres://{}:{}@{}:{}/{}",
            cfg.username.clone(),
            cfg.password.clone(),
            cfg.host.clone(),
            cfg.port,
            cfg.db_name.clone()
        );

        let conn_pool = PgPoolOptions::new()
            .max_connections(cfg.pool_size)
            .connect(&dns)
            .await
            .unwrap();
        Arc::new(conn_pool)
    });

    // casbin 管理器
    pub static ref PG_ADAPTER: AsyncOnce<Arc<PgSqlAdapter>> =
        AsyncOnce::new(async { Arc::new(PgSqlAdapter::new(crate::postgres().await).await) });

    // 雪花id管理器
    pub static ref SNOW_ID_MANAGER: AsyncOnce<crate::SnowflakeIdWorker> =  AsyncOnce::new(async {
       crate::SnowflakeIdWorker::new(1, 1).unwrap()
    });

}

/// 解析任意数据数据
pub fn parse_field<T: DeserializeOwned>(json: &Value, field: &str) -> Option<T> {
    json.get(field)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// url_encode 预览地址
pub async fn image_preview_url(path: String) -> (String, String) {
    if &true == &path.starts_with("http://") || &true == &path.starts_with("https://") {
        return (path.clone(), path);
    }

    let url_encode = byte_serialize(&path.as_bytes()).collect::<String>();

    (
        path,
        format!("{}/api/public/{}", server_host().await, url_encode),
    )
}

/// url_decode
pub fn url_decode(path: String) -> String {
    parse(path.as_bytes())
        .map(|(k, v)| [k, v].concat())
        .collect::<String>()
}

/// 服务器hosts
pub async fn server_host() -> String {
    let cfg = crate::application_config().await;
    format!("http://{}:{}", cfg.host.clone(), cfg.port)
}

/// 正则提取字符串数据
pub fn regex_patch(regex_str: &str, text: &str) -> ApiResult<(String, String)> {
    let mut result = ("".to_string(), "".to_string());
    if let Some(captures) = Regex::new(regex_str)?.captures(text) {
        if let Some(field1) = &captures.get(1) {
            result.0 = field1.as_str().to_string();
        }
        if let Some(field2) = &captures.get(2) {
            result.1 = field2.as_str().to_string();
        }
    }

    Ok(result)
}

pub type ConnPool = sqlx::PgPool;

pub async fn postgres<'left_time>() -> &'left_time ConnPool {
    &(**PG_SQL.get().await)
}

pub async fn get_pg_adapter() -> PgSqlAdapter {
    PG_ADAPTER.get().await.clone().deref().clone()
}

pub async fn snow_id() -> u128 {
    SNOW_ID_MANAGER.get().await.clone().next_id().unwrap()
}

#[derive(Clone)]
pub struct SnowflakeIdWorker(Arc<Mutex<SnowflakeIdWorkerInner>>);

impl SnowflakeIdWorker {
    pub fn new(worker_id: u128, data_center_id: u128) -> ApiResult<SnowflakeIdWorker> {
        Ok(Self(Arc::new(Mutex::new(SnowflakeIdWorkerInner::new(
            worker_id,
            data_center_id,
        )?))))
    }

    pub fn next_id(&self) -> ApiResult<u128> {
        let mut inner = match self.0.lock() {
            Ok(result) => result,
            Err(_e) => return Err(ApiError::Error(_e.to_string())),
        };

        inner.next_id()
    }
}

struct SnowflakeIdWorkerInner {
    // 工作节点id
    worker_id: u128,
    // 数据id
    data_center_id: u128,
    // 序列号
    sequence: u128,
    // 上一次时间戳
    last_timestamp: u128,
}

impl SnowflakeIdWorkerInner {
    // 开始时间戳（2023-03-16）
    const TWEPOCH: u128 = 1678955490000;
    // 机器id所占的位数
    const WORKER_ID_BITS: u128 = 5;
    // 数据节点所占的位数
    const DATA_CENTER_ID_BITS: u128 = 5;
    // 支持最大的机器ID，最大是31
    const MAX_WORKER_ID: u128 = (-1 ^ (-1 << Self::WORKER_ID_BITS)) as u128;
    // 支持的最大数据节点ID，结果是31
    const MAX_DATA_CENTER_ID: u128 = (-1 ^ (-1 << Self::DATA_CENTER_ID_BITS)) as u128;
    // 序列号所占的位数
    const SEQUENCE_BITS: u128 = 12;
    // 工作节点标识ID向左移12位
    const WORKER_ID_SHIFT: u128 = Self::SEQUENCE_BITS;
    // 数据节点标识ID向左移动17位（12位序列号+5位工作节点）
    const DATA_CENTER_ID_SHIFT: u128 = Self::SEQUENCE_BITS + Self::WORKER_ID_BITS;
    // 时间戳向左移动22位（12位序列号+5位工作节点+5位数据节点）
    const TIMESTAMP_LEFT_SHIFT: u128 =
        Self::SEQUENCE_BITS + Self::WORKER_ID_BITS + Self::DATA_CENTER_ID_BITS;
    // 生成的序列掩码，这里是4095
    const SEQUENCE_MASK: u128 = (-1 ^ (-1 << Self::SEQUENCE_BITS)) as u128;

    fn new(worker_id: u128, data_center_id: u128) -> ApiResult<Self> {
        // 校验worker_id合法性
        if worker_id > Self::MAX_WORKER_ID {
            return Err(ApiError::Error(format!(
                "workerId:{} must be less than {}",
                worker_id,
                Self::MAX_WORKER_ID
            )));
        }

        // 校验data_center_id合法性
        if data_center_id > Self::MAX_DATA_CENTER_ID {
            return Err(ApiError::Error(format!(
                "datacenterId:{} must be less than {}",
                data_center_id,
                Self::MAX_DATA_CENTER_ID
            )));
        }

        Ok(Self {
            worker_id,
            data_center_id,
            sequence: 0,
            last_timestamp: 0,
        })
    }

    fn next_id(&mut self) -> ApiResult<u128> {
        let mut timestamp = Self::get_time()?;
        if timestamp < self.last_timestamp {
            return Err(ApiError::Error(format!(
                "Clock moved backwards.  Refusing to generate id for {} milliseconds",
                self.last_timestamp - timestamp
            )));
        }

        // 如果当前时间戳等于上一次的时间戳，那么计算出序列号目前是第几位
        if timestamp == self.last_timestamp {
            self.sequence = (self.sequence + 1) & Self::SEQUENCE_MASK;
            if self.sequence == 0 {
                timestamp = Self::til_next_mills(self.last_timestamp)?;
            }
        } else {
            // 如果当前时间戳大于上一次的时间戳，序列号置为0。因为又开始了新的毫秒，所以序列号要从0开始。
            self.sequence = 0;
        }

        // 把当前时间戳赋值给last_timestamp，以便下一次计算next_id
        self.last_timestamp = timestamp;

        Ok(((timestamp - Self::TWEPOCH) << Self::TIMESTAMP_LEFT_SHIFT)
            | (self.data_center_id << Self::DATA_CENTER_ID_SHIFT)
            | (self.worker_id << Self::WORKER_ID_SHIFT)
            | self.sequence)
    }

    // 计算一个大于上一次时间戳的时间戳
    fn til_next_mills(last_timestamp: u128) -> ApiResult<u128> {
        Ok(loop {
            let timestamp = Self::get_time()?;

            if timestamp > last_timestamp {
                break timestamp;
            }
        })
    }

    // 获取当前时间戳
    fn get_time() -> ApiResult<u128> {
        match SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => Ok(duration.as_millis()),
            Err(e) => Err(ApiError::Error(e.to_string())),
        }
    }
}

#[cfg(test)]
mod test {
    use tokio::spawn;

    use crate::*;

    const WORKER_ID: u128 = 1;
    const DATA_CENTER_ID: u128 = 1;

    #[tokio::test]
    async fn create_uuid() {
        let worker = SnowflakeIdWorker::new(WORKER_ID, DATA_CENTER_ID).unwrap();
        let mut handlers = vec![];
        for _ in 0..100 {
            let worker = worker.clone();
            handlers.push(spawn(async move {
                println!("{}", worker.next_id().unwrap());
            }));
        }

        for i in handlers {
            i.await.unwrap()
        }
    }
}
