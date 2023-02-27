use axum::extract::FromRequest;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{async_trait, Form, Json, RequestExt};
use http::header::CONTENT_TYPE;

use crate::jwt::Claims;
use crate::ApiResponse;

pub mod address;
pub mod auth;
pub mod user;

/// 存储HTTPBody数据, 当前登录用户信息
#[derive(Debug)]
pub struct AppExtractor<T> {
    pub inner: T,
    pub claims: Claims,
}

#[async_trait]
impl<S, B, T> FromRequest<S, B> for AppExtractor<T>
where
    B: Send + 'static,
    S: Send + Sync,
    T: 'static,
    Json<T>: FromRequest<(), B>,
    Form<T>: FromRequest<(), B>,
{
    type Rejection = Response;

    async fn from_request(req: Request<B>, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = match req.extensions().get::<Claims>() {
            Some(value) => value.clone(),
            None => {
                return Err(ApiResponse::<i32>::fail_msg_code(
                    StatusCode::UNAUTHORIZED.as_u16(),
                    "您还未登录系统".to_string(),
                )
                .response_body()
                .into_response());
            }
        };

        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;

        if content_type.starts_with("application/json") {
            let Json(inner) = req.extract::<Json<T>, _>().await.map_err(|_err| {
                ApiResponse::<i32>::fail_msg_code(
                    StatusCode::PRECONDITION_FAILED.as_u16(),
                    "json 参数解析错误".to_string(),
                )
                .response_body()
                .into_response()
            })?;

            return Ok(Self { inner, claims });
        }

        if content_type.starts_with("application/x-www-form-urlencoded") {
            let Form(inner) = req.extract::<Form<T>, _>().await.map_err(|_err| {
                ApiResponse::<i32>::fail_msg_code(
                    StatusCode::PRECONDITION_FAILED.as_u16(),
                    "x-www-form-urlencoded 参数解析错误".to_string(),
                )
                .response_body()
                .into_response()
            })?;

            return Ok(Self { inner, claims });
        }

        Err(StatusCode::BAD_REQUEST.into_response())
    }
}
