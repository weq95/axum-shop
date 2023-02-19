use axum::{
    extract::Path,
    Json,
    response::IntoResponse,
};

use common::{
    ApiResponse,
    role_permission::{
        ReqPermission,
        ReqRole,
    },
};

/// 获取角色
pub async fn get_role(Path(role_id): Path<i64>) -> impl IntoResponse { todo!() }

/// 添加角色
pub async fn add_role(Json(role): Json<ReqRole>) -> impl IntoResponse { todo!() }

/// 更新角色信息
pub async fn update_role(Json(role): Json<ReqRole>) -> impl IntoResponse { todo!() }

/// 删除角色
pub async fn delete_role(Json(role_id): Json<i64>) -> impl IntoResponse { todo!() }

/// 删除多个角色
pub async fn delete_roles(Json(role_ids): Json<Vec<i64>>) -> impl IntoResponse { todo!() }

/// 角色列表
pub async fn roles() -> impl IntoResponse { todo!() }

/// 获取权限
pub async fn get_permission(Path(permission_id): Path<i64>) -> impl IntoResponse { todo!() }

/// 添加权限
pub async fn add_permission(Json(permission): Json<ReqPermission>) -> impl IntoResponse { todo!() }

/// 更新权限信息
pub async fn update_permission(Json(permission): Json<ReqPermission>) -> impl IntoResponse { todo!() }

/// 删除权限
pub async fn delete_permission(Json(permission_id): Json<i64>) -> impl IntoResponse { todo!() }

/// 删除多个权限
pub async fn delete_permissions(Json(permission_ids): Json<Vec<i64>>) -> impl IntoResponse { todo!() }

/// 权限列表
pub async fn permissions() -> impl IntoResponse { todo!() }