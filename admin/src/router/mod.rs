use axum::routing::{get, post};
use axum::Router;

use crate::controller::CommController;

mod admin;
mod home;

pub async fn routers() -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/test_redis", post(CommController::test_redis))
            .route("/get_config", post(CommController::get_application))
            .route("/upload/files", post(CommController::upload_file))
            .route("/show/image", get(CommController::show_image))
            .merge(admin::admin().await)
            .merge(home::home().await),
    )
}
