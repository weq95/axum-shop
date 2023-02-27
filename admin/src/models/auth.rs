use std::collections::HashSet;

use casbin::prelude::*;
use serde::Deserialize;
use sqlx::Row;

use common::casbin::CasbinRule;
use common::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Deserialize)]
pub struct RoleUser {
    pub user_id: i64,
    pub domain: String,
    pub role_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RolePermissions {
    pub object: String,
    pub action: String,
    pub domain: String,
    pub role_name: String,
}

/// 给角色添加权限
pub async fn role_permissions(permissions: Vec<RolePermissions>) -> ApiResult<bool> {
    if permissions.is_empty() {
        return Err(ApiError::Error("没有需要添加的权限".to_string()));
    }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;

    for permission in &permissions {
        let result = enforcer
            .add_policy(vec![
                permission.role_name.clone(), //角色名称
                permission.domain.clone(),    //域名
                permission.object.clone(),    //请求路径
                permission.action.clone(),    //请求方式
            ])
            .await;
        match result {
            Ok(bool_val) => {
                if bool_val {
                    counter += 1;
                }
            }
            Err(e) => {
                return Err(ApiError::Error(e.to_string()));
            }
        }
    }

    println!("total: {} 条, success: {} 条", permissions.len(), counter);
    Ok(true)
}

/// 获取角色的所有权限
pub async fn permissions_for_role(role_name: String, domain: String) -> ApiResult<Vec<CasbinRule>> {
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let enforcer = enforcer.write().await;

    let casbin_rule = match get_casbin_rule(role_name).await {
        Ok(value) => value,
        Err(e) => {
            return Err(ApiError::Error(e.to_string()));
        }
    };

    Ok(enforcer
        .get_permissions_for_user(casbin_rule.v0.as_str(), Some(domain.as_str()))
        .iter()
        .map(|row| CasbinRule {
            id: 0,
            ptype: "p".to_string(),
            v0: row[0].clone(),
            v1: row[1].clone(),
            v2: row[2].clone(),
            v3: row[3].clone(),
            v4: "".to_string(),
            v5: "".to_string(),
        })
        .collect::<Vec<CasbinRule>>())
}

/// 用户拥有的角色
pub async fn roles_for_user(user_id: u64, domain: String) -> ApiResult<Vec<String>> {
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;

    Ok(enforcer.get_roles_for_user(&*format!("user:{}", user_id), Some(domain.as_str())))
}

/// 用户拥有的角色
pub async fn permissions_for_user(user_id: u64, domain: String) -> ApiResult<Vec<CasbinRule>> {
    let mut permissions = Vec::new();
    for role_name in roles_for_user(user_id, domain.clone()).await? {
        let role_permissions = permissions_for_role(role_name, domain.clone()).await?;

        permissions.extend(role_permissions);
    }

    Ok(permissions)
}

/// 给用户分配角色
pub async fn user_roles(
    userid: u64,
    domain: String,
    casbin_rule: Vec<CasbinRule>,
) -> ApiResult<bool> {
    if casbin_rule.is_empty() {
        return Err(ApiError::Error("没有需要分配的角色".to_string()));
    }

    let mut counter: u32 = 0;
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;
    for rule in &casbin_rule {
        let user_id = format!("user:{}", userid);
        let result = enforcer
            .add_grouping_policy(vec![user_id, rule.v0.clone(), domain.clone()])
            .await;
        match result {
            Ok(boo_val) => {
                if boo_val {
                    counter += 1;
                }
            }
            Err(e) => {
                return Err(ApiError::Error(e.to_string()));
            }
        }
    }

    println!("total: {} 条, success: {} 条", casbin_rule.len(), counter);
    Ok(true)
}

/// 获取角色
pub async fn get_casbin_rule(role_name: String) -> ApiResult<CasbinRule> {
    sqlx::query("SELECT id,ptype,v0,v1,v2,v3,v4,v5 FROM casbin_rule WHERE v0 = $1")
        .bind(role_name)
        .fetch_one(common::pgsql::db().await)
        .await
        .map(|row| {
            Ok(CasbinRule {
                id: row.get::<i32, &str>("id"),
                ptype: row.get("ptype"),
                v0: row.get("v0"),
                v1: row.get("v1"),
                v2: row.get("v2"),
                v3: row.get("v3"),
                v4: row.get("v4"),
                v5: row.get("v5"),
            })
        })?
}

/// 使用id获取相应规则
pub async fn get_casbin_rules(mut role_names: HashSet<String>) -> ApiResult<Vec<CasbinRule>> {
    role_names.retain(|id| !id.is_empty());
    if role_names.is_empty() {
        return Err(ApiError::Error("没有需要分配的权限".to_string()));
    }

    let role_names = role_names
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",");

    Ok(sqlx::query(
        "SELECT id,ptype,v0,v1,v2,v3,v4,v5 FROM casbin_rule WHERE v0 IN ($1) ORDER BY id ASC",
    )
    .bind(role_names)
    .fetch_all(common::pgsql::db().await)
    .await?
    .into_iter()
    .map(|row| CasbinRule {
        id: row.get::<i32, &str>("id"),
        ptype: row.get("ptype"),
        v0: row.get("v0"),
        v1: row.get("v1"),
        v2: row.get("v2"),
        v3: row.get("v3"),
        v4: row.get("v4"),
        v5: row.get("v5"),
    })
    .collect::<Vec<CasbinRule>>())
}

/// 删除角色的权限
pub async fn delete_role_permissions(role_permissions: Vec<RolePermissions>) -> ApiResult<bool> {
    let mut result: Vec<Vec<String>> = Vec::with_capacity(role_permissions.len());
    for res in role_permissions {
        result.push(vec![
            res.role_name.clone(),
            res.domain.clone(),
            res.object.clone(),
            res.action.clone(),
        ])
    }

    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;

    match enforcer.remove_policies(result).await {
        Ok(bool_val) => Ok(bool_val),
        Err(e) => Err(ApiError::Error(e.to_string())),
    }
}

/// 删除用户的角色
pub async fn delete_user_permissions(user_permissions: Vec<RoleUser>) -> ApiResult<bool> {
    let enforcer = common::casbin::casbin_layer().await.get_enforcer().clone();
    let mut enforcer = enforcer.write().await;

    for perm in user_permissions {
        let user_id = format!("user:{}", perm.user_id);
        let _ = enforcer
            .remove_policy(vec![
                "g".to_string(),
                user_id,
                perm.role_name.clone(),
                perm.domain.clone(),
            ])
            .await;
    }

    Ok(true)
}
