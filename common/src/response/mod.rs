use std::fmt::Debug;

use axum::{
    body::Body,
    response::Response,
};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use crate::error::ApiError;

pub mod user;
pub mod address;

/// 全局通用成功编码
const SUCCESS: u16 = 0;
/// 全局通用错误编码
const FAIL: u16 = 10000;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ToString for ApiResponse<T>
    where T: Clone + Serialize + DeserializeOwned + Debug {
    fn to_string(&self) -> String {
        serde_json::to_string(self)
            .map_err(|err| {
                println!("系统解析错误!!!,err: {}", err);
            })
            .unwrap_or("".to_string())
    }
}

impl<T: Serialize + DeserializeOwned + Clone + Debug> ApiResponse<T> {
    pub fn response(result: &Result<T, ApiError>) -> Self {
        if false == result.is_ok() {
            return Self::fail_msg(result.clone().unwrap_err().to_string());
        }


        Self {
            code: SUCCESS,
            message: "success".to_string(),
            data: result.clone().ok(),
        }
    }

    pub fn success_code(code: u16) -> Self {
        Self {
            code: code,
            message: "success".to_string(),
            data: None,
        }
    }

    pub fn success_code_data(code: u16, data: &Result<T, ApiError>) -> Self {
        Self {
            code: code,
            message: "success".to_string(),
            data: data.clone().ok(),
        }
    }

    pub fn fail_msg(message: String) -> Self {
        Self {
            code: FAIL,
            message: message,
            data: None,
        }
    }

    pub fn fail_msg_code(code: u16, message: String) -> Self {
        Self {
            code: code,
            message: message,
            data: None,
        }
    }

    /// 这里必须返回一个 [`IntoResponse`] 才能符合第三方接口的需求
    pub fn json(&self) -> impl IntoResponse {
        self.response_body().into_response()
    }

    pub fn response_body(&self) -> Response<Body> {
        Response::builder().extension(|| {})
            .header("Access-Control-Allow-Origin", "*")
            .header("Content-Type", "text/json; charset=UTF-8")
            .header("Cache-Control", "no-cache")
            .body(Body::from(self.to_string()))
            .unwrap()
    }
}