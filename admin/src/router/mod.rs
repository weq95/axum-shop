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


    Router::new()
        .merge(users)
        .merge(address)
        .layer(
            ServiceBuilder::new()
                .layer(AxumMiddleware::from_fn(middleware::guard))
                .layer(CasbinAuthLayer)
                .layer(common::casbin::casbin_layer().await)
        )
        .merge(login)
}