use axum::{
    body::Body,
    extract::Path,
    http::Request,
    Json,
    response::IntoResponse,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use validator::Validate;

use common::{ApiResponse, parse_field, auth::{
    ReqPermission,
    ReqRole,
}};
use common::jwt::Claims;

use crate::models::auth::{
    add_permission,
    add_roles,
    Permission,
    Role,
    role_permissions,
    roles_user,
};

/// 获取角色
pub async fn get_role(req: Request<Body>) -> impl IntoResponse {
    let user = req.extensions().get::<Claims>().unwrap();
    ApiResponse::response(Some(user.id)).json()
}

/// 添加角色
pub async fn create_role(Json(role): Json<ReqRole>) -> impl IntoResponse {
    match role.validate() {
        Ok(_) => {}
        Err(e) => {
            return ApiResponse::fail_msg(e.to_string()).json();
        }
    }

    ApiResponse::response(Some(add_roles(vec![
        Role {
            id: 0,
            name: role.name.unwrap(),
            domain: "".to_string(),
        }
    ], Some("localhost".to_string())).await)).json()
}

/// 更新角色信息
pub async fn update_role(Json(role): Json<ReqRole>) -> impl IntoResponse { todo!() }

/// 删除多个角色
pub async fn delete_roles(Json(role_ids): Json<Vec<i64>>) -> impl IntoResponse { todo!() }

/// 角色列表
pub async fn roles() -> impl IntoResponse {
    todo!()
}

/// 获取权限
pub async fn get_permission(Path(permission_id): Path<i64>) -> impl IntoResponse { todo!() }

/// 添加权限
pub async fn create_permission(Json(permission): Json<ReqPermission>) -> impl IntoResponse {
    match permission.validate() {
        Ok(_) => {}
        Err(e) => {
            return ApiResponse::fail_msg(e.to_string()).json();
        }
    }

    ApiResponse::response(Some(add_permission(vec![
        Permission {
            id: 0,
            name: permission.name.unwrap(),
            object: permission.object.unwrap(),
            action: permission.action.unwrap(),
            domain: "".to_string(),
        }
    ], Some("localhost".to_string())).await)).json()
}

/// 更新权限信息
pub async fn update_permission(Json(permission): Json<ReqPermission>) -> impl IntoResponse { todo!() }

/// 删除多个权限
pub async fn delete_permissions(Json(permission_ids): Json<Vec<i64>>) -> impl IntoResponse { todo!() }

/// 权限列表
pub async fn permissions() -> impl IntoResponse {
    todo!()
}

/// 给用户分配角色
pub async fn add_role_user(Json(payload): Json<Value>) -> impl IntoResponse {
    let mut role_ids: Vec<u32> = match parse_field(&payload, "role_ids") {
        Some(val) => val,
        None => {
            return ApiResponse::fail_msg("角色参数错误".to_string()).json();
        }
    };
    let user_id = match parse_field::<u64>(&payload, "user_id") {
        Some(val) => val,
        None => {
            return ApiResponse::fail_msg("用户参数错误".to_string()).json();
        }
    };

    let domain = "localhost".to_string();
    ApiResponse::response(Some(roles_user(user_id, role_ids, domain).await)).json()
}

/// 给角色分配权限
pub async fn add_role_permission(Json(payload): Json<Value>) -> impl IntoResponse {
    let mut permission_ids: Vec<u32> = match parse_field(&payload, "permission_ids") {
        Some(val) => val,
        None => {
            return ApiResponse::fail_msg("权限参数错误".to_string()).json();
        }
    };
    let role_id = match parse_field(&payload, "role_id") {
        Some(val) => val,
        None => {
            return ApiResponse::fail_msg("角色参数错误".to_string()).json();
        }
    };

    let domain = "localhost".to_string();
    ApiResponse::response(Some(role_permissions(role_id, permission_ids, domain).await)).json()
}