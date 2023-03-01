use axum::routing::{get, post};
use axum::Router;

use crate::controller::{show_image, upload_file};

mod admin;
mod home;

pub async fn routers() -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/upload/files", post(upload_file))
            .route("/show/image", get(show_image))
            .merge(admin::admin().await)
            .merge(home::home().await),
    )
}
