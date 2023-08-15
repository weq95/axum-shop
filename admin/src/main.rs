#[allow(dead_code)]
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Extension;
use tracing::info;

use crate::controller::MQMANAGER;

mod controller;
mod jobs;
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

    MQMANAGER.get().await;

    info!("admin-srv run at: {}", addr);
    tokio::spawn(async {
        if let Err(e) = jobs::calculate_installment_fine().await {
            println!("calculate_fine: {}", e);
        }
        jobs::start_jobs();
    });
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}
