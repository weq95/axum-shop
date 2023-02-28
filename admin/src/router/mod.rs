use crate::controller::upload_file;
use axum::routing::post;
use axum::Router;

mod admin;
mod home;

pub async fn routers() -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/upload/images", post(upload_file))
            .merge(admin::admin().await)
            .merge(home::home().await),
    )
}
