use std::collections::HashMap;
use std::fmt::Debug;

use axum::response::IntoResponse;
use axum::{body::Body, response::Response};
use serde::{Deserialize, Serialize};

pub mod address;
pub mod user;

/// 全局通用成功编码
const SUCCESS: u16 = 0;
/// 全局通用错误编码
const FAIL: u16 = 10000;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ToString for ApiResponse<T> {
    fn to_string(&self) -> String {
        serde_json::to_string(self)
            .map_err(|err| {
                println!("系统解析错误!!!,err: {}", err);
            })
            .unwrap_or("".to_string())
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn response(result: Option<T>) -> Self {
        Self {
            code: SUCCESS,
            message: "success".to_string(),
            data: result,
        }
    }

    pub fn success_code(code: u16) -> Self {
        Self {
            code,
            message: "success".to_string(),
            data: None,
        }
    }

    pub fn success_code_data(code: u16, data: Option<T>) -> Self {
        Self {
            code,
            message: "success".to_string(),
            data,
        }
    }

    pub fn fail_msg(message: String) -> Self {
        Self {
            code: FAIL,
            message,
            data: None,
        }
    }

    pub fn fail_msg_code(code: u16, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    /// 这里必须返回一个 [`IntoResponse`] 才能符合第三方接口的需求
    pub fn json(&self) -> impl IntoResponse {
        self.response_body().into_response()
    }

    pub fn response_body(&self) -> Response<Body> {
        Response::builder()
            .extension(|| {})
            .header("Access-Control-Allow-Origin", "*")
            .header("Content-Type", "text/json; charset=UTF-8")
            .header("Cache-Control", "no-cache")
            .body(Body::from(self.to_string()))
            .unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
