use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use axum::{
    body::{Body, BoxBody},
    http::Request,
    response::Response,
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

use common::casbin::CasbinVals;
use common::jwt::Claims;

#[derive(Clone)]
pub struct CasbinAuthLayer;

#[derive(Clone)]
pub struct CabinAuthMiddleware<S> {
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

        let subject = match req.extensions().get::<Claims>() {
            Some(user) => {
                let uid = user.id.to_string();
                "user:".to_owned() + uid.as_str()
            }
            None => String::from("")
        };

        Box::pin(async move {
            req.extensions_mut().insert(CasbinVals {
                subject: subject,
                domain: Some("localhost".to_string()),
            });
            inner.call(req).await
        })
    }
}
