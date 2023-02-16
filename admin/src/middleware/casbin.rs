use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    body::{self, Body, BoxBody, boxed},
    http::{Request, StatusCode},
    response::Response,
};
use axum::response::IntoResponse;
use casbin::{
    CachedEnforcer, CoreApi, DefaultModel,
    FileAdapter, function_map::key_match2, TryIntoAdapter,
    TryIntoModel,
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

use common::casbin::CasbinVals;

#[derive(Clone)]
pub struct CasbinAuthLayer;

#[derive(Clone)]
struct CabinAuthMiddleware<S> {
    inner: S,
}

impl<S> Layer<S> for CasbinAuthLayer {
    type Service = CabinAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CabinAuthMiddleware { inner }
    }
}

impl<S> Service<Request<Body>> for CabinAuthMiddleware<S>
    where S: Service<Request<Body>, Response=Response, Error=Infallible>,
          S: Clone + Send + 'static,
          S::Future: Send + 'static {
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let not_ready_inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, not_ready_inner);

        Box::pin(async move {
            let _username = String::from("username");
            req.extensions_mut().insert(CasbinVals {
                subject: _username,
                domain: None,
            });
            inner.call(req).await
        })
    }
}
