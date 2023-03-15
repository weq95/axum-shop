use axum::middleware as AxumMiddleware;
use axum::routing::{delete, get, post};
use axum::Router;
use tower::ServiceBuilder;

use middleware::casbin::CasbinAuthLayer;

use crate::controller::products::ProductController;
use crate::controller::{
    address::AddressController, auth::RolePermissionController, order::OrderController,
    AdminController, CommController,
};
use crate::middleware;

// Path  GET    格式: /user/132
// Query GET    格式: /user/test?id=123&name=456

pub async fn admin() -> Router {
    let login = Router::new()
        .route("/register", post(AdminController::register))
        .route("/login", post(AdminController::login));
    let users = Router::new().nest(
        "/users",
        Router::new()
            .route(
                "/",
                get(AdminController::lists).post(AdminController::create),
            )
            .route(
                "/:id",
                get(AdminController::get)
                    .put(AdminController::update)
                    .delete(AdminController::delete),
            )
            .route(
                "/carts",
                get(AdminController::carts)
                    .post(AdminController::add_cart)
                    .delete(AdminController::delete_carts),
            ),
    );
    let address = Router::new().nest(
        "/address",
        Router::new()
            .route(
                "/",
                get(AddressController::list_address).post(AddressController::create_address),
            )
            .route("/result/:pid", get(AddressController::addr_result))
            .route(
                "/:id",
                get(AddressController::get_address)
                    .put(AddressController::update_address)
                    .delete(AddressController::delete_address),
            ),
    );
    let auth = Router::new().nest(
        "/auth",
        Router::new()
            .route(
                "/perm_for_role",
                post(RolePermissionController::get_permissions_for_role),
            )
            .route(
                "/perm_for_user",
                post(RolePermissionController::get_permissions_for_user),
            )
            .route(
                "/roles_for_user",
                post(RolePermissionController::get_roles_for_user),
            )
            .route(
                "/user_roles",
                post(RolePermissionController::add_user_roles),
            )
            .route(
                "/role_permissions",
                post(RolePermissionController::add_role_permissions),
            )
            .route(
                "/delete_role_permission",
                delete(RolePermissionController::delete_role_permission),
            )
            .route(
                "/delete_user_permission",
                delete(RolePermissionController::delete_user_permission),
            ),
    );
    let products = Router::new().nest(
        "/products",
        Router::new()
            .route(
                "/",
                get(ProductController::products).post(ProductController::create),
            )
            .route(
                "/:id/user/:id",
                get(ProductController::get)
                    .post(ProductController::favorite_product)
                    .delete(ProductController::un_favorite_product),
            )
            .route(
                "/:id",
                post(ProductController::update).delete(ProductController::delete),
            ),
    );

    let orders = Router::new().nest(
        "/orders",
        Router::new()
            .route(
                "/",
                get(OrderController::index).post(OrderController::store),
            )
            .route(
                "/:id",
                get(OrderController::get).post(OrderController::update),
            ),
    );

    Router::new().nest(
        "/admin",
        Router::new()
            .route("/refresh_token", post(CommController::refresh_token))
            .merge(users)
            .merge(address)
            .merge(auth)
            .merge(products)
            .merge(orders)
            .layer(
                ServiceBuilder::new()
                    .layer(AxumMiddleware::from_fn(middleware::auth_guard))
                    .layer(CasbinAuthLayer)
                    .layer(common::casbin::casbin_layer().await),
            )
            .merge(login),
    )
}
