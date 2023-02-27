use axum::Router;

mod admin;
mod home;

pub async fn routers() -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .merge(admin::admin().await)
            .merge(home::home().await),
    )
}
