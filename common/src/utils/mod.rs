use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::Value;
use url::form_urlencoded::{byte_serialize, parse};

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
