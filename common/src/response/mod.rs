use std::collections::HashMap;
use std::fmt::{Debug, Display};

use crate::error::ApiError;
use axum::response::IntoResponse;
use axum::{body::Body, response::Response};
use http::response::Builder;
use http::{HeaderName, HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub mod address;
pub mod user;

/// 全局通用成功编码
pub const SUCCESS: u16 = 0;
/// 全局通用错误编码
pub const FAIL: u16 = 10000;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: u16,
    pub message: Option<ApiError>,
    pub data: Option<T>,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let message: serde_json::Value = match self.message {
            Some(ApiError::Error(e)) => json!(e),
            Some(ApiError::Array(e)) => json!(e),
            Some(ApiError::Object(e)) => json!(e),
            Some(ApiError::ArrayMap(e)) => json!(e),
            None => json!(String::from("success")),
        };

        let mut response = (
            StatusCode::OK,
            json!({
                "code": self.code,
                "message": message,
                "data": self.data,
            })
            .to_string(),
        )
            .into_response();
        response.headers_mut().insert(
            "Content-Type".parse::<HeaderName>().unwrap(),
            "text/json; charset=UTF-8".parse::<HeaderValue>().unwrap(),
        );

        response
    }
}

impl<T: Serialize> Display for ApiResponse<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = serde_json::to_string(self)
            .map_err(|err| {
                println!("系统解析错误!!!,err: {}", err);
            })
            .unwrap_or("".to_string());
        write!(f, "{}", str)
    }
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(result: Option<T>) -> Self {
        Self {
            code: SUCCESS,
            message: None,
            data: result,
        }
    }
    pub fn response(result: Option<T>) -> Self {
        Self {
            code: SUCCESS,
            message: None,
            data: result,
        }
    }

    pub fn success() -> Self {
        Self {
            code: SUCCESS,
            message: None,
            data: None,
        }
    }

    pub fn success_code(code: u16) -> Self {
        Self {
            code,
            message: None,
            data: None,
        }
    }

    pub fn success_code_data(code: u16, data: Option<T>) -> Self {
        Self {
            code,
            message: None,
            data,
        }
    }

    pub fn fail_msg(message: String) -> Self {
        Self {
            code: FAIL,
            message: Some(ApiError::Error(message)),
            data: None,
        }
    }

    pub fn fail_msg_code(code: u16, message: String) -> Self {
        Self {
            code,
            message: Some(ApiError::Error(message)),
            data: None,
        }
    }

    /// 这里必须返回一个 [`IntoResponse`] 才能符合第三方接口的需求
    pub fn json(&self) -> impl IntoResponse {
        self.response_body().into_response()
    }

    /// 设置返回数据类型
    pub fn set_content_type(content_type: Option<&str>) -> Builder {
        Response::builder()
            .extension(|| {})
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-cache")
            .header(
                "Content-Type",
                content_type.unwrap_or("text/json; charset=UTF-8"),
            )
    }

    pub fn response_body(&self) -> Response<Body> {
        let message: serde_json::Value = match &self.message {
            Some(ApiError::Error(e)) => json!(e),
            Some(ApiError::Array(e)) => json!(e),
            Some(ApiError::Object(e)) => json!(e),
            Some(ApiError::ArrayMap(e)) => json!(e),
            None => json!(String::from("success")),
        };

        Self::set_content_type(None)
            .body(Body::from(
                json!({
                    "code": self.code,
                    "message": message,
                    "data": self.data,
                })
                .to_string(),
            ))
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

/// 通用分页结构
#[derive(Serialize, Clone)]
pub struct Pagination<T> {
    #[serde(default)]
    data: Option<Box<Vec<T>>>,
    // 总条数
    total: usize,
    // 当前页
    page: usize,
    // 页大小
    per_page: usize,
}

#[derive(Deserialize)]
pub struct PagePer {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_per_page")]
    per_page: usize,
}

// 默认页
fn default_page() -> usize {
    1
}

// 默认页大小
fn default_per_page() -> usize {
    30
}

impl<T> Pagination<T> {
    pub fn new(result: Vec<T>, page_per: PagePer) -> Self {
        Pagination {
            data: Some(Box::new(result)),
            page: page_per.page,
            per_page: page_per.per_page,
            total: 0,
        }
    }

    /// 设置总页数
    pub fn set_total(&mut self, total: usize) -> &mut Pagination<T> {
        self.total = total;

        self
    }

    /// 计算总页数
    pub fn total_pages(&mut self) -> usize {
        if self.total % self.per_page == 0 {
            return self.total / self.per_page;
        }

        (self.total / self.per_page) + 1
    }

    /// 是否存在上一页
    pub fn has_previous_page(&self) -> bool {
        self.page > 1
    }

    /// 是否存在下一页
    pub fn has_next_page(&mut self) -> bool {
        self.page < self.total_pages()
    }

    /// 上一页页码
    pub fn previous_page_number(&mut self) -> Option<usize> {
        if self.has_previous_page() {
            return Some(self.page - 1);
        }

        None
    }

    /// 下一页页码
    pub fn next_page_number(&mut self) -> Option<usize> {
        if self.has_next_page() {
            return Some(self.page + 1);
        }

        None
    }

    pub fn offset(&self) -> i32 {
        ((self.page - 1) * self.per_page) as i32
    }

    pub fn limit(&self) -> i32 {
        self.per_page as i32
    }

    /// 添加数据
    pub fn set_data(&mut self, data: Vec<T>) {
        self.data = Some(Box::new(data));
    }

    /// 获取数据
    pub fn get_data(&self) -> &[T] {
        if let Some(data) = &self.data {
            return &data[..];
        }
        &[]
    }
}
