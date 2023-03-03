use std::collections::HashSet;

use axum::{Json, response::IntoResponse};
use serde_json::Value;
use validator::Validate;

use common::{
    ApiResponse,
    auth::{ReqRolePermissions, ReqRoleUser}, parse_field,
};

use crate::models::auth::{AdminAuth, RolePermissions, RoleUser};

pub struct RolePermissionController;

impl RolePermissionController {
    /// 获取角色所有得权限
    pub async fn get_permissions_for_role(Json(payload): Json<Value>) -> impl IntoResponse {
        let role_id = match parse_field::<String>(&payload, "role_name") {
            Some(id_str) => id_str,
            None => {
                return ApiResponse::fail_msg("角色参数错误".to_string()).json();
            }
        };
        let domain = match parse_field::<String>(&payload, "domain") {
            Some(domain) => domain,
            None => {
                return ApiResponse::fail_msg("域名参数错误".to_string()).json();
            }
        };

        match AdminAuth::permissions_for_role(role_id, domain).await {
            Ok(result) => ApiResponse::response(Some(result)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 获取角色所有的角色
    pub async fn get_roles_for_user(Json(payload): Json<Value>) -> impl IntoResponse {
        let user_id: u64 = match parse_field(&payload, "user_id") {
            Some(user_id) => user_id,
            None => return ApiResponse::fail_msg("用户参数错误".to_string()).json(),
        };
        let domain: String = match parse_field(&payload, "domain") {
            Some(domain) => domain,
            None => return ApiResponse::fail_msg("参数错误".to_string()).json(),
        };

        match AdminAuth::roles_for_user(user_id, domain).await {
            Ok(result) => ApiResponse::response(Some(result)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 获取用户拥有的全部权限
    pub async fn get_permissions_for_user(Json(payload): Json<Value>) -> impl IntoResponse {
        let user_id: u64 = match parse_field(&payload, "user_id") {
            Some(user_id) => user_id,
            None => return ApiResponse::fail_msg("用户参数错误".to_string()).json(),
        };
        let domain: String = match parse_field(&payload, "domain") {
            Some(domain) => domain,
            None => return ApiResponse::fail_msg("参数错误".to_string()).json(),
        };
        match AdminAuth::permissions_for_user(user_id, domain).await {
            Ok(result) => ApiResponse::response(Some(result)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 给用户分配角色
    pub async fn add_user_roles(Json(payload): Json<ReqRoleUser>) -> impl IntoResponse {
        match payload.validate() {
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            Ok(_) => {}
        }
        let role_names = HashSet::from([payload.name.unwrap()]);
        let rules = match AdminAuth::get_casbin_rules(role_names).await {
            Ok(result) => result,
            Err(e) => {
                return ApiResponse::fail_msg(e.to_string()).json();
            }
        };

        ApiResponse::response(Some(
            AdminAuth::user_roles(
                payload.user_id.unwrap(),
                payload.domain.unwrap().clone(),
                rules,
            )
                .await,
        ))
            .json()
    }

    /// 给角色分配权限
    pub async fn add_role_permissions(
        Json(payload): Json<Vec<ReqRolePermissions>>,
    ) -> impl IntoResponse {
        for data in &payload {
            match data.validate() {
                Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
                Ok(_) => {}
            }
        }
        let permissions: Vec<RolePermissions> = payload
            .into_iter()
            .map(|row| RolePermissions {
                object: row.subject.clone().unwrap(),
                action: row.action.clone().unwrap(),
                domain: row.domain.clone().unwrap(),
                role_name: row.role_name.clone().unwrap(),
            })
            .collect::<Vec<RolePermissions>>();
        if permissions.clone().is_empty() {
            return ApiResponse::fail_msg("没有需要添加的权限".to_string()).json();
        }

        ApiResponse::response(Some(AdminAuth::role_permissions(permissions).await)).json()
    }

    /// 删除角色
    pub async fn delete_role_permission(Json(payload): Json<ReqRolePermissions>) -> impl IntoResponse {
        match &payload.validate() {
            Err(e) => return ApiResponse::<bool>::fail_msg(e.to_string()).json(),
            Ok(_) => {}
        }

        let result: Vec<RolePermissions> = vec![RolePermissions {
            object: payload.subject.clone().unwrap(),
            action: payload.action.clone().unwrap(),
            domain: payload.domain.clone().unwrap(),
            role_name: payload.role_name.clone().unwrap(),
        }];
        match AdminAuth::delete_role_permissions(result).await {
            Ok(bool_val) => ApiResponse::response(Some(bool_val)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 删除用户的权限
    pub async fn delete_user_permission(Json(payload): Json<ReqRoleUser>) -> impl IntoResponse {
        match &payload.validate() {
            Err(e) => return ApiResponse::<bool>::fail_msg(e.to_string()).json(),
            Ok(_) => {}
        }

        let result: Vec<RoleUser> = vec![RoleUser {
            user_id: payload.user_id.unwrap() as i64,
            domain: payload.domain.unwrap(),
            role_name: payload.name.unwrap(),
        }];

        match AdminAuth::delete_user_permissions(result).await {
            Ok(bool_val) => ApiResponse::response(Some(bool_val)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }
}

