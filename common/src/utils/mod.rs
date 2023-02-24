use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::Value;

pub mod casbin;
pub mod jwt;
pub mod pgsql;
pub mod pwd;
pub mod redis;

/// 读取系统配置文件
pub fn init_read_config() {
    dotenv::from_path(PathBuf::from("./config/.env")).unwrap();
    dotenv::dotenv().ok();
}

/// 解析任意数据数据
pub fn parse_field<T: DeserializeOwned>(json: &Value, field: &str) -> Option<T> {
    json.get(field)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}
