use std::ops::DerefMut;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query},
    http::Request,
    response::IntoResponse,
    Extension, Json,
};
use serde_json::json;
use validator::Validate;

use common::jwt::Claims;
use common::{
    jwt::{UserSource, UserType, JWT},
    request::user::{ReqCrateUser, ReqGetUser, ReqLogin, ReqQueryUser, ReqUpdateUser},
    utils::redis,
    ApiResponse, SchoolJson,
};

use crate::models::user::{
    create as ModelCreate, delete as ModelDelete, get as ModelGet, list as ModelList,
    update as ModelUpdate,
};
use crate::AppState;

/// 用户注册
pub async fn register(Json(_payload): Json<ReqGetUser>) {}

pub async fn test_redis(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let key = payload.get("key").unwrap().as_str().unwrap();

    let mut conn = redis::get_conn_manager().await;
    let data = redis::json_get::<SchoolJson>(conn.deref_mut(), key, "*").await;
    ApiResponse::response(Some(data)).json()
}

/// 用户登录
pub async fn login(Json(payload): Json<ReqLogin>) -> impl IntoResponse {
    match payload.validate() {
        Ok(_) => {}
        Err(e) => {
            return ApiResponse::fail_msg(e.to_string()).json();
        }
    }

    let email = payload.email.unwrap().clone();
    let user = ModelGet(ReqGetUser {
        id: None,
        name: None,
        age: None,
        nickname: None,
        phone: None,
        email: Some(email.clone()),
    })
    .await;

    let user = match user {
        Ok(userinfo) => userinfo,
        Err(e) => {
            return ApiResponse::fail_msg(e.to_string()).json();
        }
    };

    if user.id == 0 || user.email.clone() != email {
        return ApiResponse::fail_msg("用户名或密码错误".to_string()).json();
    }

    let jwt = JWT::default();
    let mut claims = jwt.new_claims(
        user.id as i64,
        user.email.clone(),
        user.name.clone(),
        payload.password.unwrap().clone(),
        UserSource::PC,
        UserType::User,
    );

    match jwt.token_info(&mut claims) {
        Ok((access_token, refresh_token)) => ApiResponse::response(Some(json!({
            "access_token": access_token,
            "refresh_token":refresh_token,
        })))
        .json(),
        Err(_) => ApiResponse::fail_msg("登录失败, 请稍后重试".to_string()).json(),
    }
}

/// 创建用户
pub async fn create_admin(
    Extension(_state): Extension<Arc<AppState>>,
    Json(user): Json<ReqCrateUser>,
) -> impl IntoResponse {
    ApiResponse::response(Some(ModelCreate(user).await)).json()
}

/// 用户详情
pub async fn get_admin(
    Extension(_state): Extension<Arc<AppState>>,
    Path(userid): Path<u64>,
) -> impl IntoResponse {
    if userid == 0 {
        return ApiResponse::fail_msg("参数错误".to_string()).json();
    }

    let userinfo = ModelGet(ReqGetUser {
        id: Some(userid as i64),
        name: None,
        age: None,
        nickname: None,
        phone: None,
        email: None,
    })
    .await;
    if userinfo.clone().unwrap().id == 0 {
        return ApiResponse::fail_msg("未找到用户信息".to_string()).json();
    }
    ApiResponse::response(Some(userinfo)).json()
}

/// 更新用户信息
pub async fn update_admin(
    Extension(_state): Extension<Arc<AppState>>,
    Json(user): Json<ReqUpdateUser>,
) -> impl IntoResponse {
    ApiResponse::response(Some(ModelUpdate(user).await)).json()
}

/// 删除用户
pub async fn delete_admin(
    Extension(_state): Extension<Arc<AppState>>,
    Path(userid): Path<u64>,
) -> impl IntoResponse {
    if userid == 0 {
        return ApiResponse::fail_msg("参数错误".to_string()).json();
    }

    ApiResponse::response(Some(ModelDelete(userid).await)).json()
}

/// 用户列表
pub async fn user_list(
    Extension(_state): Extension<Arc<AppState>>,
    Query(parma): Query<ReqQueryUser>,
) -> impl IntoResponse {
    ApiResponse::response(Some(ModelList(parma).await)).json()
}

pub async fn refresh_token(mut req: Request<Body>) -> impl IntoResponse {
    match req.extensions_mut().get_mut::<Claims>() {
        Some(claims) => match JWT::default().token_info(claims) {
            Ok((access_token, refresh_token)) => ApiResponse::response(Some(json!({
                "access_token": access_token,
                "refresh_token":refresh_token,
            })))
            .json(),
            Err(_) => ApiResponse::fail_msg("refresh_token 刷新失败[02]".to_string()).json(),
        },
        None => ApiResponse::fail_msg("refresh_token 刷新失败[01]".to_string()).json(),
    }
}
