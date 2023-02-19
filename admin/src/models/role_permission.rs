/*use std::collections::HashSet;

use casbin::Adapter;
use casbin::prelude::*;

use common::error::ApiResult;

#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    domain: String,
}

#[derive(Debug, Clone)]
struct Role {
    id: i64,
    name: String,
    domain: String,
}

#[derive(Debug, Clone)]
struct Permission {
    id: i64,
    name: String,
    object: String,
    domain: String,
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


async fn add_role_for_user(e: &mut Enforcer, user: &User, role: &Role) -> bool {
    let role_sub = format!("role:{}/{}", role.domain, role.name);
    let user_sub = format!("user:{}/{}", user.domain, user.name);
    let mut adapter = common::pgsql::get_pg_adapter().await;

    adapter.add_grouping_policy(vec![user_sub], Some(&role_sub), None)
        .await
        .unwrap()
}

/// 用户所有的角色
async fn get_roles_for_user(e: &Enforcer, user: &User) -> HashSet<Role> {
    let user_sub = format!("user:{}/{}", user.domain, user.name);
    let mut roles = HashSet::new();
    let mut adapter = common::pgsql::get_pg_adapter().await;
    let role_subs = adapter.get_roles_for_user(&user_sub, None);
    for role_sub in role_subs {
        let role_parts: Vec<&str> = role_sub.split(':').collect();
        if role_parts.len() == 3 && role_parts[0] == "role" {
            let role = Role {
                id: 0,
                domain: role_parts[1].to_string(),
                name: role_parts[2].to_string(),
            };
            roles.insert(role);
        }
    }
    roles
}

/// 角色拥有的权限
async fn get_permissions_for_role(e: &Enforcer, role: &Role) -> Vec<String> {
    let role_sub = format!("role:{}/{}", role.domain, role.name);
    let mut adapter = common::pgsql::get_pg_adapter().await;
    // adapter.get_permissions_for_user(&role_sub, None)
    //     .iter()
    //     .map(|p| p.to_owned())
    //     .collect()
}


/// 用户拥有的权限
async fn get_permissions_for_user(e: &Enforcer, user: &User) -> Vec<String> {
    let mut permissions = vec![];
    let roles = get_roles_for_user(&e, &user);
    for role in roles {
        let role_permissions = get_permissions_for_role(&e, &role).await;
        permissions.extend(role_permissions);
    }
    permissions
}


pub async fn add_user() -> ApiResult<bool> {
    // 加载用户
    let users: Vec<User> = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            domain: "example.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            domain: "example.com".to_string(),
        },
        User {
            id: 3,
            name: "Charlie".to_string(),
            domain: "example.com".to_string(),
        },
    ];
    let mut adapter = common::pgsql::get_pg_adapter().await;
    for user in users {
        let userid = format!("user:{}", user.id);
        // adapter.add_role_for_user(&userid, &user.name).unwrap();
    }

    Ok(true)
}

/// 添加角色
pub async fn add_roles() -> ApiResult<bool> {
    // 加载角色
    let roles = vec![
        Role {
            id: 1,
            name: "admin".to_string(),
            domain: "example.com".to_string(),
        },
        Role {
            id: 2,
            name: "member".to_string(),
            domain: "example.com".to_string(),
        },
    ];
    let mut adapter = common::pgsql::get_pg_adapter().await;
    for role in roles {
        let sub = format!("role:{}", role.id);
        // adapter.add_role_for_user(&sub, &role.name).unwrap();
    }

    Ok(true)
}

pub async fn add_permissions() -> ApiResult<bool> {
    // 加载权限
    let permissions = vec![
        Permission {
            id: 1,
            name: "GET".to_string(),
            object: "/api/user".to_string(),
            domain: "example.com".to_string(),
        },
        Permission {
            id: 2,
            name: "POST".to_string(),
            object: "/api/user/:id".to_string(),
            domain: "example.com".to_string(),
        },
    ];
    let mut adapter = common::pgsql::get_pg_adapter().await;
    for permission in permissions {
        let obj = format!("{}:{}", permission.object, permission.id);
        // adapter.add_permission_for_user(&permission.name, vec![obj]).unwrap();
    }
    Ok(true)
}

pub async fn role_user() -> ApiResult<bool> {
    // 加载角色用户关系
    let role_users = vec![
        RoleUser {
            id: 1,
            role_id: 1,
            user_id: 1,
        },
        RoleUser {
            id: 2,
            role_id: 2,
            user_id: 2,
        },
        RoleUser {
            id: 3,
            role_id: 2,
            user_id: 3,
        },
    ];
    let mut adapter = common::pgsql::get_pg_adapter().await;
    for role_user in role_users {
        let role_sub = format!("role:{}", role_user.role_id);
        let user_sub = format!("user:{}", role_user.user_id);

        // adapter.add_grouping_policy(vec![user_sub, role_sub]).unwrap();
    }

    Ok(true)
}

pub async fn role_permission() -> ApiResult<bool> {
    // 加载角色权限关系
    let role_permissions = vec![
        RolePermission {
            id: 1,
            role_id: 1,
            permission_id: 1,
        },
        RolePermission {
            id: 2,
            role_id: 1,
            permission_id: 2,
        },
        RolePermission {
            id: 3,
            role_id: 2,
            permission_id: 1,
        },
    ];

    let mut adapter = common::pgsql::get_pg_adapter().await;
    for role_permission in role_permissions {
        let role_sub = format!("role:{}", role_permission.role_id);
        let permission_obj = format!("path:{}", role_permission.permission_id);
        adapter.add_policy(&role_sub, &permission_obj, vec![role_permission.permission_id.to_string()])
            .await.unwrap();
    }

    Ok(true)
}

pub async fn test() {
    // 执行测试
    let alice = users.get(0).unwrap();
    let bob = users.get(1).unwrap();
    let charlie = users.get(2).unwrap();

// Alice 是管理员，可以读写文档
    assert!(e.enforce(&format!("user:{}", alice.id), "document:1", "read").unwrap());
    assert!(e.enforce(&format!("user:{}", alice.id), "document:2", "write").unwrap());

// Bob 是普通用户，只能读取文档
    assert!(e.enforce(&format!("user:{}", bob.id), "document:1", "read").unwrap());
    assert!(!e.enforce(&format!("user:{}", bob.id), "document:2", "write").unwrap());

// Charlie 是普通用户，只能读取文档
    assert!(e.enforce(&format!("user:{}", charlie.id), "document:1", "read").unwrap());
    assert!(!e.enforce(&format!("user:{}", charlie.id), "document:2", "write").unwrap());
}*/