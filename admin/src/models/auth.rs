use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use std::process::id;

use casbin::Adapter;
use casbin::prelude::*;
use serde::Deserialize;
use sqlx::{Arguments, Row};
use sqlx::postgres::PgArguments;

use common::casbin::{CasbinRule, CasbinVals};
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

#[derive(Debug, Clone, Deserialize)]
pub struct Permission {
    pub id: Option<i64>,
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

/// 给角色添加权限
pub async fn role_permissions(role: String, domain: Option<String>, mut permissions: Vec<Permission>) -> ApiResult<bool> {
    permissions.retain(|s| !s.name.is_empty());
    if role.is_empty() || permissions.len() == 0 { return Err(ApiError::Error("没有需要添加的权限".to_string())); }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;

    let domain = if let Some(val) = domain { val } else { "localhost".to_string() };
    for permission in &permissions {
        let _sub = format!("permission:{}", permission.name.clone()); //权限中文名称
        let result = enforcer.add_policy(
            vec![
                permission.name.clone(), //权限名称
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
pub async fn user_roles(userid: u64, domain: String, casbin_rule: Vec<CasbinRule>) -> ApiResult<bool> {
    if casbin_rule.is_empty() { return Err(ApiError::Error("没有需要分配的角色".to_string())); }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;
    for rule in &casbin_rule {
        let user_id = format!("user:{}", userid);
        let result = enforcer.add_grouping_policy(
            vec![
                user_id,
                rule.v0.clone(),
                domain.clone(),
            ]).await;
        match result {
            Ok(boo_val) => if boo_val { counter += 1; }
            Err(e) => {
                return Err(ApiError::Error(e.to_string()));
            }
        }
    }

    println!("total: {} 条, success: {} 条", casbin_rule.len(), counter);
    Ok(true)
}

/// 使用id获取相应规则
pub async fn get_casbin_rules(mut role_ids: HashSet<i32>) -> ApiResult<Vec<CasbinRule>> {
    role_ids.retain(|&id| id > 0);
    if role_ids.is_empty() { return Err(ApiError::Error("没有需要分配的权限".to_string())); }

    let mut arg = PgArguments::default();
    let mut placeholder = String::with_capacity(role_ids.len());
    let mut idx = 0;
    for role_id in &role_ids {
        arg.add(role_id);
        idx += 1;
        placeholder.push_str(&*("$".to_owned() + idx.to_string().as_str() + ","));
    }

    let placeholder = placeholder.trim_matches(',');
    Ok(sqlx::query_with(&*("SELECT id,ptype,v0,v1,v2,v3,v4,v5 FROM \
    casbin_rule WHERE id IN (".to_owned() + placeholder + ") ORDER BY id ASC"), arg)
        .fetch_all(common::pgsql::db().await).await?.into_iter().map(|row| {
        CasbinRule {
            id: row.get::<i32, &str>("id"),
            ptype: row.get::<String, &str>("ptype"),
            v0: row.get::<String, &str>("v0"),
            v1: row.get::<String, &str>("v1"),
            v2: row.get::<String, &str>("v2"),
            v3: row.get::<String, &str>("v3"),
            v4: row.get::<String, &str>("v4"),
            v5: row.get::<String, &str>("v5"),
        }
    }).collect::<Vec<CasbinRule>>())
}