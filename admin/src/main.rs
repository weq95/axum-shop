#[allow(dead_code)]
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Extension;
use tracing_subscriber::util::SubscriberInitExt;

mod controller;
mod middleware;
mod models;
mod router;

#[derive(Clone, Copy)]
pub struct AppState {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .finish()
        .set_default();

    common::application_config().await;
    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));

    let app_state = Arc::new(AppState {});

    let router = router::routers().await.layer(Extension(app_state));

    println!("admin-srv run at: {}", addr);
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
