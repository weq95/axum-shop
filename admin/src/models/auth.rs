use std::collections::HashSet;
use std::ops::{Deref, DerefMut};

use casbin::Adapter;
use casbin::prelude::*;

use common::error::{ApiError, ApiResult};

#[derive(Debug, Clone)]
pub struct User {
    id: i64,
    name: String,
    domain: String,
}

#[derive(Debug, Clone)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub domain: String,
}

#[derive(Debug, Clone)]
pub struct Permission {
    pub id: i64,
    pub name: String,
    pub object: String,
    pub action: String,
    pub domain: String,
}

#[derive(Debug, Clone)]
struct RoleUser {
    id: i64,
    role_id: i64,
    user_id: i64,
}

#[derive(Debug, Clone)]
struct RolePermission {
    id: i64,
    role_id: i64,
    permission_id: i64,
}

/// 添加角色
pub async fn add_roles(mut roles: Vec<Role>, domain: Option<String>) -> ApiResult<bool> {
    roles.retain(|s| !s.name.is_empty());
    if roles.len() == 0 { return Err(ApiError::Error("没有需要添加的角色".to_string())); }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;
    let domain = if let Some(val) = domain { Some(val) } else { Some("localhost".to_string()) };

    for role in &roles {
        let sub = format!("role:{}", role.name.clone()); //角色中文名称
        let result = enforcer.add_role_for_user(&sub, &role.name, domain.as_deref()).await;
        match result {
            Ok(bool_val) => if bool_val { counter += 1; }
            Err(e) => {
                return Err(ApiError::Error(e.to_string()));
            }
        }
    }

    println!("total: {} 条, success: {} 条", roles.len(), counter);
    Ok(true)
}

/// 添加权限
pub async fn add_permission(mut permissions: Vec<Permission>, domain: Option<String>) -> ApiResult<bool> {
    permissions.retain(|s| !s.name.is_empty());
    if permissions.len() == 0 { return Err(ApiError::Error("没有需要添加的权限".to_string())); }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;

    let domain = if let Some(val) = domain { val } else { "localhost".to_string() };
    for permission in &permissions {
        let sub = format!("permission:{}", permission.name.clone()); //权限中文名称
        let result = enforcer.add_permission_for_user(
            &sub,
            vec![
                domain.clone(), //域名
                permission.object.clone(),//请求路径
                permission.action.clone(), //请求方式
            ]).await;
        match result {
            Ok(bool_val) => if bool_val { counter += 1; }
            Err(e) => {
                return Err(ApiError::Error(e.to_string()));
            }
        }
    }

    println!("total: {} 条, success: {} 条", permissions.len(), counter);
    Ok(true)
}


/// 给用户分配角色
pub async fn roles_user(userid: u64, mut role_ids: Vec<u32>, domain: String) -> ApiResult<bool> {
    role_ids.retain(|&id| id > 0);
    if role_ids.len() == 0 { return Err(ApiError::Error("没有需要分配的角色".to_string())); }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;
    for role_id in &role_ids {
        let result = enforcer.add_grouping_policy(
            vec![
                userid.to_string(),
                role_id.to_string(),
                domain.clone(),
            ]).await;
        match result {
            Ok(boo_val) => if boo_val { counter += 1; }
            Err(e) => {
                return Err(ApiError::Error(e.to_string()));
            }
        }
    }

    println!("total: {} 条, success: {} 条", role_ids.len(), counter);
    Ok(true)
}

/// 给角色分配权限
pub async fn role_permissions(role_id: i32, mut permission_ids: Vec<u32>, domain: String) -> ApiResult<bool> {
    permission_ids.retain(|&id| id > 0);
    if permission_ids.len() == 0 { return Err(ApiError::Error("没有需要分配的权限".to_string())); }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;
    for permission_id in &permission_ids {
        let result = enforcer.add_policy(vec![
            role_id.to_string(),
            permission_id.to_string(), //权限名称
            // permission_id.to_string(),
            domain.clone(),
        ]).await;
        match result {
            Ok(bool_val) => if bool_val { counter += 1; }
            Err(e) => {
                return Err(ApiError::Error(e.to_string()));
            }
        }
    }

    println!("total: {} 条, success: {} 条", permission_ids.len(), counter);
    Ok(true)
}