#[allow(dead_code)]
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Extension, Router};
use axum::middleware as AxumMiddleware;
use axum::routing::{get, post};
use tower::ServiceBuilder;
use tracing_subscriber::util::SubscriberInitExt;

use controller::{
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
use middleware::casbin::CasbinAuthLayer;

// Path  GET    格式: /user/132
// Query GET    格式: /user/test?id=123&name=456
mod models;
mod controller;
mod middleware;

async fn routers() -> Router {
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

#[derive(Clone, Copy)]
pub struct AppState {}


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish()
        .set_default();

    common::init_read_config();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));

    let app_state = Arc::new(AppState {});

    let router = Router::new().nest("/api", routers().await
        .layer(Extension(app_state)));

    println!("admin-srv run at: {}", addr);
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await.unwrap();
}


