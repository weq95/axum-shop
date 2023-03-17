use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    Extension, Json,
};
use serde_json::json;
use validator::Validate;

use common::error::format_errors;
use common::jwt::Claims;
use common::response::user::GetUser;
use common::{
    jwt::{UserSource, UserType, JWT},
    request::user::{ReqCrateUser, ReqGetUser, ReqLogin, ReqUpdateUser},
    ApiResponse, PagePer, Pagination,
};

use crate::models::cart_items::CartItems;
use crate::models::user::Admin;
use crate::{get_pager, AppState};

pub struct AdminController;

impl AdminController {
    /// 用户注册
    pub async fn register(Json(_payload): Json<ReqGetUser>) {}

    /// 用户登录
    pub async fn login(Json(payload): Json<ReqLogin>) -> impl IntoResponse {
        match payload.validate() {
            Ok(_) => {}
            Err(e) => {
                return ApiResponse::fail_msg(e.to_string()).json();
            }
        }

        let email = payload.email.unwrap().clone();
        let user = Admin::get(ReqGetUser {
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
    pub async fn create(
        Extension(_state): Extension<Arc<AppState>>,
        Json(user): Json<ReqCrateUser>,
    ) -> impl IntoResponse {
        match user.validate() {
            Ok(()) => (),
            Err(e) => {
                return ApiResponse::success_code_data(
                    common::response::FAIL,
                    Some(json!(format_errors(e))),
                )
                .json();
            }
        }

        match Admin::create(user).await {
            Ok(id) => ApiResponse::response(Some(json!({ "id": id }))).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 用户详情
    pub async fn get(
        Extension(user): Extension<Claims>,
        Json(inner): Json<ReqGetUser>,
    ) -> impl IntoResponse {
        ApiResponse::response(Some(json!({
            "id": user.id,
        })))
        .json()
    }

    /// 更新用户信息
    pub async fn update(
        Extension(_state): Extension<Arc<AppState>>,
        Json(user): Json<ReqUpdateUser>,
    ) -> impl IntoResponse {
        ApiResponse::response(Some(Admin::update(user).await)).json()
    }

    /// 删除用户
    pub async fn delete(
        Extension(_state): Extension<Arc<AppState>>,
        Path(userid): Path<u64>,
    ) -> impl IntoResponse {
        if userid == 0 {
            return ApiResponse::fail_msg("参数错误".to_string()).json();
        }

        ApiResponse::response(Some(Admin::delete(userid).await)).json()
    }

    /// 用户列表
    pub async fn lists(
        Query(params): Query<serde_json::Value>,
        page_per: Option<Query<PagePer>>,
    ) -> impl IntoResponse {
        let mut pagination = Pagination::new(vec![], get_pager(page_per));

        match Admin::lists(&mut pagination, &params).await {
            Ok(()) => ApiResponse::response(Some(pagination)).json(),
            Err(err) => ApiResponse::fail_msg(err.to_string()).json(),
        }
    }

    /// 加入购物车
    pub async fn add_cart(
        Extension(user): Extension<Claims>,
        Json(inner): Json<HashMap<String, i64>>,
    ) -> impl IntoResponse {
        let product_id = match &inner.get("product_id") {
            Some(&product_id) => product_id,
            None => return ApiResponse::fail_msg("添加失败,参数错误01".to_string()).json(),
        };
        let sku_id = match inner.get("product_sku_id") {
            Some(&product_sku_id) => product_sku_id,
            None => return ApiResponse::fail_msg("添加失败,参数错误02".to_string()).json(),
        };

        match CartItems::add(user.id, product_id, sku_id, 1).await {
            Ok(cart_id) => ApiResponse::response(Some(json!({ "id": cart_id }))).json(),
            Err(err) => ApiResponse::fail_msg(err.to_string()).json(),
        }
    }

    /// 删除购物车商品
    pub async fn delete_carts(
        Extension(user): Extension<Claims>,
        Json(ids): Json<Vec<i64>>,
    ) -> impl IntoResponse {
        match CartItems::delete(ids, user.id).await {
            Ok(rows) => ApiResponse::response(Some(json!({ "rows": rows }))).json(),
            Err(err) => ApiResponse::fail_msg(err.to_string()).json(),
        }
    }

    /// 购物车列表
    pub async fn carts(
        page_per: Option<Query<PagePer>>,
        Extension(user): Extension<Claims>,
    ) -> impl IntoResponse {
        let mut pagination: Pagination<HashMap<String, serde_json::Value>> =
            Pagination::new(vec![], get_pager(page_per));

        match Admin::cart_items(user.id, &mut pagination).await {
            Ok(()) => ApiResponse::response(Some(pagination)).json(),
            Err(err) => ApiResponse::fail_msg(err.to_string()).json(),
        }
    }
}
