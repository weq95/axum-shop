use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::io::Error;
use std::num::ParseIntError;
use std::str::Utf8Error;

use axum::extract::multipart::MultipartError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::ParseError;
use redis::RedisError;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use validator::{ValidationError, ValidationErrors};

/// 返回资源类型
pub type ApiResult<T> = Result<T, ApiError>;

/// 系统定义错误
/// kind 错误类型
/// 详情
#[derive(Debug, Clone, Serialize)]
pub enum ApiError {
    #[serde(rename = "string")]
    Error(String),
    #[serde(rename = "array")]
    Array(Vec<String>),
    #[serde(rename = "map")]
    Object(HashMap<String, String>),
    #[serde(rename = "array_map")]
    ArrayMap(Vec<HashMap<String, String>>),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut err_type = String::from("string");
        let message: serde_json::Value = match self {
            ApiError::Error(e) => {
                err_type = String::from("string");
                json!(e)
            }
            ApiError::Array(e) => {
                err_type = String::from("array");
                json!(e)
            }
            ApiError::Object(e) => {
                err_type = String::from("map");
                json!(e)
            }
            ApiError::ArrayMap(e) => {
                err_type = String::from("array_map");
                json!(e)
            }
        };

        let value = json!({
            "code":  StatusCode::BAD_REQUEST.as_u16(),
            "message": message,
            "err_type": err_type,
            "data": None::<serde_json::Value>,
        })
            .to_string();
        (StatusCode::OK, value).into_response()
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Error(err) => {
                write!(f, "{}", err)
            }
            _ => {
                write!(f, "{}", "other errors")
            }
        }
    }
}

impl From<Error> for ApiError {
    fn from(_e: Error) -> Self {
        ApiError::Error(_e.to_string())
    }
}

pub fn format_error(validate_err: ValidationError) -> Vec<HashMap<String, String>> {
    let mut errors = ValidationErrors::new();
    errors.add("", validate_err.clone());
    format_errors(errors)
}

pub fn format_errors(errors: ValidationErrors) -> Vec<HashMap<String, String>> {
    errors
        .field_errors()
        .into_iter()
        .map(|(field, err)| {
            let message = err
                .into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>()
                .join(",");

            HashMap::from([(field.to_owned(), message)])
        })
        .collect::<Vec<HashMap<String, String>>>()
}

impl From<sqlx::Error> for ApiError {
    fn from(_e: sqlx::Error) -> Self {
        ApiError::Error(_e.to_string())
    }
}

impl From<&str> for ApiError {
    fn from(_e: &str) -> Self {
        ApiError::Error(_e.to_string())
    }
}

impl From<String> for ApiError {
    fn from(_e: String) -> Self {
        ApiError::Error(_e.to_string())
    }
}

impl From<axum::Error> for ApiError {
    fn from(_e: axum::Error) -> Self {
        ApiError::Error(_e.to_string())
    }
}

impl From<ParseError> for ApiError {
    fn from(_e: ParseError) -> Self {
        ApiError::Error(_e.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<ParseIntError> for ApiError {
    fn from(value: ParseIntError) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<Utf8Error> for ApiError {
    fn from(value: Utf8Error) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<RedisError> for ApiError {
    fn from(value: RedisError) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<r2d2_redis::Error> for ApiError {
    fn from(value: r2d2_redis::Error) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<r2d2_redis::redis::RedisError> for ApiError {
    fn from(value: r2d2_redis::redis::RedisError) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<Infallible> for ApiError {
    fn from(value: Infallible) -> Self {
        let err = value.to_string();
        ApiError::Error(("你没有访问权限 ".to_owned() + err.as_str()).to_string())
    }
}

impl From<MultipartError> for ApiError {
    fn from(value: MultipartError) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<serde_yaml::Error> for ApiError {
    fn from(value: serde_yaml::Error) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<regex::Error> for ApiError {
    fn from(value: regex::Error) -> Self {
        ApiError::Error(value.to_string())
    }
}

impl From<lapin::Error> for ApiError {
    fn from(value: lapin::Error) -> Self {
        ApiError::Error(value.to_string())
    }
}

struct ApiVisitor;

impl<'de> Visitor<'de> for ApiVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
    {
        Ok(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
    {
        Ok(v)
    }
}

impl<'de> Deserialize<'de> for ApiError {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        Ok(ApiError::Error(de.deserialize_string(ApiVisitor)?))
    }
}
