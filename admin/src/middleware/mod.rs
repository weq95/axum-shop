pub(crate) mod casbin;

use axum::headers::Authorization;
use axum::headers::authorization::Bearer;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::TypedHeader;

use common::ApiResponse;
use common::error::{ApiError, ApiResult};
use common::jwt::JWT;

/// 登录守卫
pub async fn guard<B>(TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
                      mut req: Request<B>, next: Next<B>) -> ApiResult<Response> {
    match JWT::default().verify(auth.token()) {
        Ok(claims) => {
            // 验证redis 用户登录信息

            // 携带用户信息到下游去
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_e) => {
            Err(ApiError::Error(ApiResponse::<i32>::fail_msg_code(
                StatusCode::UNAUTHORIZED.as_u16(),
                "您还未登录系统".to_string()).to_string()))
        }
    }
}