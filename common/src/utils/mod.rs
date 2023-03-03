use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::Value;

pub mod casbin;
pub mod file;
pub mod jwt;
pub mod pgsql;
pub mod pwd;
pub mod redis;

/// 解析任意数据数据
pub fn parse_field<T: DeserializeOwned>(json: &Value, field: &str) -> Option<T> {
    json.get(field)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}
