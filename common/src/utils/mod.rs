use std::ops::Deref;
use std::sync::Arc;

use async_once::AsyncOnce;
use lazy_static::lazy_static;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use url::form_urlencoded::{byte_serialize, parse};

use crate::casbin::PgSqlAdapter;
use crate::error::ApiResult;

pub mod casbin;
pub mod jwt;
pub mod pwd;
pub mod rabbitmq;
pub mod redis;
pub(crate) mod snowflake;
pub mod tree;

/// 图片存储跟路径
pub const IMAGES_PATH: &str = "./files/images/";

lazy_static! {
    // pgsql 连接池
    pub static ref PG_SQL: AsyncOnce<Arc<ConnPool>> = AsyncOnce::new(async {
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
    pub static ref SNOW_ID_MANAGER: AsyncOnce<crate::snowflake::SnowflakeIdWorker> =  AsyncOnce::new(async {
       crate::snowflake::SnowflakeIdWorker::new(1, 1).unwrap()
    });

    // rabbitmq 链接器
    pub static ref RABBITMQ: AsyncOnce<Arc<lapin::Connection>> = AsyncOnce::new(async{
        let cfg = &crate::application_config().await.rabbitmq;
        let addr = format!("{}://{}:{}@{}:{}/{}",cfg.scheme,cfg.username,cfg.password,cfg.host,cfg.port,cfg.vhost);
        Arc::new(lapin::Connection::connect(addr.as_str(), lapin::ConnectionProperties::default()).await.unwrap())
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

pub async fn rabbit_mq() -> Arc<lapin::Connection> {
    RABBITMQ.get().await.clone()
}

// 格式化年月日,时分秒
pub fn time_ymd_his(date_time: chrono::NaiveDateTime) -> String {
    date_time.format("%Y-%m-%d %H:%M:%S").to_string()
}

// 字符串除去双引号
pub fn string_trim_yh(value: &serde_json::Value) -> String {
    let value = serde_json::to_string(value).unwrap();
    let binding = value.clone();

    binding.trim_matches('"').to_string()
}

fn bubble_sort<T: Ord>(arr: &mut [T]) {
    for i in 0..arr.len() {
        for j in 0..arr.len() - 1 - i {
            if arr[i] > arr[j + 1] {
                arr.swap(j, j + 1);
            }
        }
    }
}

fn fibonacci(n: u32) -> u32 {
    if n == 0 || n == 1 {
        return n;
    }

    fibonacci(n - 1) + fibonacci(n - 2)
}
