use std::sync::Arc;

use axum::{Router, ServiceExt};
use axum::middleware as AxumMiddleware;
use axum::routing::{get, post};
use tower::ServiceBuilder;

use middleware::casbin::CasbinAuthLayer;

use crate::AppState;
use crate::controller::{
    address::{
        addr_result,
        create_address,
        delete_address,
        get_address,
        list_address,
        update_address,
    },
    auth::{
        add_role_permission,
        add_role_user,
        create_permission,
        create_role,
        delete_permissions,
        delete_roles,
        get_permission,
        get_role,
        permissions,
        roles,
        update_permission,
        update_role,
    },
    user::{
        create_admin,
        delete_admin,
        get_admin,
        login,
        register,
        test_redis,
        update_admin,
        user_list,
    },
};
use crate::middleware;

// Path  GET    格式: /user/132
// Query GET    格式: /user/test?id=123&name=456

pub async fn routers() -> Router {
    let login = Router::new()
        .route("/test/redis", post(test_redis))
        .route("/register", post(register))
        .route("/login", post(login));
    let users = Router::new().nest("/users", Router::new()
        .route("/", get(user_list).post(create_admin))
        .route("/:id", get(get_admin).put(update_admin).delete(delete_admin)));
    let address = Router::new().nest("/address", Router::new()
        .route("/", get(list_address).post(create_address))
        .route("/result/:pid", get(addr_result))
        .route("/:id", get(get_address).put(update_address).delete(delete_address)));
    let role_perm = Router::new().nest("/auth", Router::new()
        .nest("/roles", Router::new()
            .route("/", get(roles).post(create_role))
            .route("/role_user", post(add_role_user))
            .route("/role_permission", post(add_role_permission))
            .route("/:id", get(get_role).post(update_role).delete(delete_roles)))
        .nest("/permissions", Router::new()
            .route("/", get(permissions).post(create_permission))
            .route("/:id", get(get_permission).post(update_permission).delete(delete_permissions))),
    );

    Router::new()
        .nest("/api/",
              Router::new()
                  .merge(users)
                  .merge(address)
                  .layer(
                      ServiceBuilder::new()
                          .layer(AxumMiddleware::from_fn(middleware::guard))
                          .layer(CasbinAuthLayer)
                          .layer(common::casbin::casbin_layer().await)
                  )

                  .merge(role_perm)
                  .merge(login),
        )
}