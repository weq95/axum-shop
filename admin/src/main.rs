#[allow(dead_code)]
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Extension;
use common::rabbitmq::RabbitMQDeadQueue;
use tracing::info;

mod controller;
mod middleware;
mod models;
mod router;

#[derive(Clone, Copy)]
pub struct AppState {}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_writer(std::io::stdout)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    common::application_config().await;
    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));

    let app_state = Arc::new(AppState {});

    let router = router::routers().await.layer(Extension(app_state));

    common::DEAD_QUEUE.get().await.clone().init().await;

    info!("admin-srv run at: {}", addr);
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
