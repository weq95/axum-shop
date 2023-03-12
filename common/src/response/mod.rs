use std::collections::HashMap;
use std::fmt::Debug;

use axum::response::IntoResponse;
use axum::{body::Body, response::Response};
use http::response::Builder;
use serde::{Deserialize, Serialize};

use crate::parse_field;
use crate::response::user::GetUser;

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
        Self::set_content_type(None)
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

/// 通用分页结构
#[derive(Serialize, Deserialize, Clone)]
pub struct Pagination<T> {
    #[serde(default)]
    data: Option<Box<Vec<T>>>,
    // 总条数
    total: usize,
    // 页大小
    per_page: usize,
    // 当前页
    current_page: usize,
}

impl<T> Default for Pagination<T> {
    fn default() -> Self {
        Self {
            data: None,
            total: 0,
            per_page: 15,
            current_page: 1,
        }
    }
}

impl<T> Pagination<T> {
    pub fn new(result: Vec<T>) -> Self {
        Pagination {
            data: Some(Box::new(result)),
            ..Self::default()
        }
    }

    /// 获取请求页数参数
    pub fn init(paging: &serde_json::Value) -> Self {
        let mut pagination = Pagination::default();
        if let Some(per_page) = parse_field::<String>(&paging, "per_page") {
            match per_page.parse::<usize>() {
                Ok(per_page) => {
                    pagination.set_per_page(per_page);
                }
                Err(err) => {
                    println!("{}", err);
                }
            }
        }
        if let Some(current_page) = parse_field::<String>(&paging, "current_page") {
            match current_page.parse::<usize>() {
                Ok(current_page) => {
                    pagination.set_current_page(current_page);
                }
                Err(err) => {
                    println!("{}", err);
                }
            }
        }

        pagination
    }

    /// 设置总页数
    pub fn set_total(&mut self, total: usize) -> &mut Pagination<T> {
        self.total = total;

        self
    }

    /// 设置分页大小
    pub fn set_per_page(&mut self, per_page: usize) -> &mut Pagination<T> {
        self.per_page = per_page;

        self
    }

    /// 设置当前页
    pub fn set_current_page(&mut self, current_page: usize) -> &mut Pagination<T> {
        self.current_page = current_page;

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
        self.current_page > 1
    }

    /// 是否存在下一页
    pub fn has_next_page(&mut self) -> bool {
        self.current_page < self.total_pages()
    }

    /// 上一页页码
    pub fn previous_page_number(&mut self) -> Option<usize> {
        if self.has_previous_page() {
            return Some(self.current_page - 1);
        }

        None
    }

    /// 下一页页码
    pub fn next_page_number(&mut self) -> Option<usize> {
        if self.has_next_page() {
            return Some(self.current_page + 1);
        }

        None
    }

    pub fn offset(&self) -> usize {
        (self.current_page - 1) * self.per_page
    }

    pub fn limit(&self) -> usize {
        self.per_page
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
