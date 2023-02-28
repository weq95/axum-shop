use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use axum::TypedHeader;

use common::error::{ApiError, ApiResult};
use common::jwt::JWT;
use common::ApiResponse;

pub(crate) mod casbin;

/// 登录守卫
pub async fn auth_guard<B>(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut req: Request<B>,
    next: Next<B>,
) -> ApiResult<Response> {
    match JWT::default().verify(auth.token()) {
        Ok(claims) => {
            // 验证redis 用户登录信息
            if claims.token_type() != "access_token" && req.uri().path() != "/refresh_token" {
                return Err(ApiError::Error(
                    ApiResponse::<i32>::fail_msg_code(
                        StatusCode::UNAUTHORIZED.as_u16(),
                        "非法的token".to_string(),
                    )
                    .to_string(),
                ));
            }

            // 携带用户信息到下游去
            req.extensions_mut().insert(claims);
            Ok(next.run(req).await)
        }
        Err(_e) => Err(ApiError::Error(
            ApiResponse::<i32>::fail_msg_code(
                StatusCode::UNAUTHORIZED.as_u16(),
                "您还未登录系统".to_string(),
            )
            .to_string(),
        )),
    }
}
