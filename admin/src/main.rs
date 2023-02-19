#[allow(dead_code)]
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Extension, Router};
use tracing_subscriber::util::SubscriberInitExt;

mod models;
mod controller;
mod middleware;
mod router;


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

    let router = Router::new().nest("/api", router::routers().await
        .layer(Extension(app_state)));

    println!("admin-srv run at: {}", addr);
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await.unwrap();
}


