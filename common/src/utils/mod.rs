use regex::{CaptureMatches, Regex};
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::form_urlencoded::{byte_serialize, parse};

use crate::error::{ApiError, ApiResult};

pub mod casbin;
pub mod jwt;
pub mod pgsql;
pub mod pwd;
pub mod redis;

/// 图片存储跟路径
pub const IMAGES_PATH: &str = "./files/images/";

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
