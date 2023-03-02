use axum::middleware as AxumMiddleware;
use axum::routing::{delete, get, post};
use axum::Router;
use tower::ServiceBuilder;

use middleware::casbin::CasbinAuthLayer;

use crate::controller::products::ProductController;
use crate::controller::{
    address::{
        addr_result, create_address, delete_address, get_address, list_address, update_address,
    },
    auth::{
        add_role_permissions, add_user_roles, delete_role_permission, delete_user_permission,
        get_permissions_for_role, get_permissions_for_user, get_roles_for_user,
    },
    refresh_token,
    user::{
        create_admin, delete_admin, get_admin, login, register, test_redis, update_admin, user_list,
    },
};
use crate::middleware;

// Path  GET    格式: /user/132
// Query GET    格式: /user/test?id=123&name=456

pub async fn admin() -> Router {
    let login = Router::new()
        .route("/test/redis", post(test_redis))
        .route("/register", post(register))
        .route("/login", post(login));
    let users = Router::new().nest(
        "/users",
        Router::new()
            .route("/", get(user_list).post(create_admin))
            .route(
                "/:id",
                get(get_admin).put(update_admin).delete(delete_admin),
            ),
    );
    let address = Router::new().nest(
        "/address",
        Router::new()
            .route("/", get(list_address).post(create_address))
            .route("/result/:pid", get(addr_result))
            .route(
                "/:id",
                get(get_address).put(update_address).delete(delete_address),
            ),
    );
    let auth = Router::new().nest(
        "/auth",
        Router::new()
            .route("/perm_for_role", post(get_permissions_for_role))
            .route("/perm_for_user", post(get_permissions_for_user))
            .route("/roles_for_user", post(get_roles_for_user))
            .route("/user_roles", post(add_user_roles))
            .route("/role_permissions", post(add_role_permissions))
            .route("/delete_role_permission", delete(delete_role_permission))
            .route("/delete_user_permission", delete(delete_user_permission)),
    );
    let products = Router::new().nest(
        "/products",
        Router::new()
            .route("/", post(ProductController::create))
            .route("/:id", get(ProductController::get)),
    );

    Router::new().nest(
        "/admin",
        Router::new()
            .route("/refresh_token", post(refresh_token))
            .merge(users)
            .merge(address)
            .merge(auth)
            .merge(products)
            .layer(
                ServiceBuilder::new()
                    .layer(AxumMiddleware::from_fn(middleware::auth_guard))
                    .layer(CasbinAuthLayer)
                    .layer(common::casbin::casbin_layer().await),
            )
            .merge(login),
    )
}
