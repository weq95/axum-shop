use std::sync::Arc;

use axum::{
    Extension, extract::{Path, Query},
    Json,
    response::IntoResponse,
};
use serde_json::json;
use validator::Validate;

use common::{
    ApiResponse,
    jwt::{JWT, UserSource, UserType},
    request::user::{
        ReqCrateUser,
        ReqGetUser,
        ReqLogin,
        ReqQueryUser,
        ReqUpdateUser,
    },
    utils::redis,
};
use common::redis::SchoolJson;

use crate::AppState;
use crate::models::user::{
    create as ModelCreate,
    delete as ModelDelete,
    get as ModelGet,
    list as ModelList,
    update as ModelUpdate,
};

/// 用户注册
pub async fn register(Json(_payload): Json<ReqGetUser>) {}

pub async fn test_redis(Json(_payload): Json<serde_json::Value>) -> impl IntoResponse {
    let key: String = "school_json:1".to_string();
    match redis::get(key).await {
        Err(_e) => ApiResponse::fail_msg(_e.to_string()).json(),
        Ok(data) => {
            ApiResponse::response(&redis::comm_to::<SchoolJson>(&data).await).json()
        }
    }
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
    }).await;

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
    let claims = jwt.new_claims(
        user.id as i64,
        user.email.clone(),
        user.name.clone(),
        UserSource::PC,
        payload.password.unwrap().clone(),
        UserType::User, "".to_string());
    if let Ok(token) = jwt.token(&claims) {
        return ApiResponse::response(&Ok(json!({"token": token}))).json();
    }

    ApiResponse::fail_msg("登录失败,请稍后重试".to_string()).json()
}

/// 创建用户
pub async fn create_admin(Extension(_state): Extension<Arc<AppState>>, Json(user): Json<ReqCrateUser>) -> impl IntoResponse {
    ApiResponse::response(&ModelCreate(user).await).json()
}

/// 用户详情
pub async fn get_admin(Extension(_state): Extension<Arc<AppState>>, Path(userid): Path<u64>) -> impl IntoResponse {
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
    }).await;
    if userinfo.clone().unwrap().id == 0 {
        return ApiResponse::fail_msg("未找到用户信息".to_string()).json();
    }
    ApiResponse::response(&userinfo).json()
}

/// 更新用户信息
pub async fn update_admin(Extension(_state): Extension<Arc<AppState>>, Json(user): Json<ReqUpdateUser>) -> impl IntoResponse {
    ApiResponse::response(&ModelUpdate(user).await).json()
}

/// 删除用户
pub async fn delete_admin(Extension(_state): Extension<Arc<AppState>>, Path(userid): Path<u64>) -> impl IntoResponse {
    if userid == 0 {
        return ApiResponse::fail_msg("参数错误".to_string()).json();
    }

    ApiResponse::response(&ModelDelete(userid).await).json()
}

/// 用户列表
pub async fn user_list(Extension(_state): Extension<Arc<AppState>>, Query(parma): Query<ReqQueryUser>) -> impl IntoResponse {
    ApiResponse::response(&ModelList(parma).await).json()
}