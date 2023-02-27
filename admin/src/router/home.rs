use axum::Router;

pub async fn home() -> Router {
    Router::new().nest("/home", Router::new())
}
