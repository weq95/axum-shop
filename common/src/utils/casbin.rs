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
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower::{Layer, Service};

use crate::ApiResponse;

#[derive(Clone)]
pub struct CasbinVals {
    pub subject: String,
    pub domain: Option<String>,
}

#[derive(Clone)]
pub struct CasbinLayer {
    enforcer: Arc<RwLock<CachedEnforcer>>,
}

impl CasbinLayer {
    pub async fn new<M: TryIntoModel, A: TryIntoAdapter>(model: M, adapter: A) -> Self {
        let mut enforcer: CachedEnforcer = CachedEnforcer::new(model, adapter).await.unwrap();
        CasbinLayer {
            enforcer: Arc::new(RwLock::new(enforcer)),
        }
    }

    pub fn get_enforcer(&mut self) -> Arc<RwLock<CachedEnforcer>> {
        self.enforcer.clone()
    }

    pub fn set_enforcer(ef: Arc<RwLock<CachedEnforcer>>) -> CasbinLayer {
        CasbinLayer { enforcer: ef }
    }
}

impl<S> Layer<S> for CasbinLayer {
    type Service = CasbinMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CasbinMiddleware {
            enforcer: self.enforcer.clone(),
            inner,
        }
    }
}

impl Deref for CasbinLayer {
    type Target = Arc<RwLock<CachedEnforcer>>;

    fn deref(&self) -> &Self::Target {
        &self.enforcer
    }
}

impl DerefMut for CasbinLayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.enforcer
    }
}

#[derive(Clone)]
pub struct CasbinMiddleware<S> {
    inner: S,
    enforcer: Arc<RwLock<CachedEnforcer>>,
}

impl<S> Service<Request<Body>> for CasbinMiddleware<S>
    where S: Service<Request<Body>, Response=Response, Error=Infallible>,
          S: Clone + Send + 'static,
          S::Future: Send + 'static {
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let cloned_enforcer = self.enforcer.clone();
        let not_ready_inner = self.inner.clone();

        let mut ready_inner = std::mem::replace(&mut self.inner, not_ready_inner);
        Box::pin(async move {
            let response: Response<BoxBody> = ApiResponse::<i32>::fail_msg_code(
                u16::from(StatusCode::FORBIDDEN),
                "您没有操作权限!".to_string(),
            ).response_body().into_response();

            let option_vals = req.extensions().get::<CasbinVals>()
                .map(|x| x.to_owned());
            let vals = match option_vals {
                Some(val) => val,
                None => return Ok(response),
            };
            let path = req.uri().clone().to_string();
            let method = req.method().clone().to_string();
            let mut lock = cloned_enforcer.write().await;

            if vals.subject.is_empty() {
                return Ok(response);
            }

            let subject = vals.clone().subject;
            if let Some(domain) = vals.domain {
                match lock.enforce_mut(vec![subject, domain, path, method]) {
                    Ok(bool_val) => {
                        drop(lock);
                        if false == bool_val { return Ok(response); }

                        let response: Response<BoxBody> = ready_inner.call(req).await?.map(body::boxed);
                        return Ok(response);
                    }
                    Err(_) => {
                        drop(lock);
                        return Ok(response);
                    }
                }
            }

            match lock.enforce_mut(vec![subject, path, method]) {
                Ok(bool_val) => {
                    drop(lock);
                    if false == bool_val {
                        return Ok(response);
                    }

                    let response: Response<BoxBody> = ready_inner.call(req).await?.map(body::boxed);
                    Ok(response)
                }
                Err(_) => {
                    drop(lock);
                    Ok(response)
                }
            }
        })
    }
}

/// default 默认处理规则
const API_DEFAULT_MODEL: &str = r#"
[request_definition]
r = sub, obj, act

[policy_definition]
p = sub, obj, act

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub) && g2(r.obj, p.obj) && regexMatch(r.act, p.act) || r.sub == "admin"
"#;


pub async fn casbin_layer() -> CasbinLayer {
    let model = DefaultModel::from_str(API_DEFAULT_MODEL)
        .await.unwrap();

    // 以后需要切换成数据库驱动
    let adapter = FileAdapter::new("config/policy.csv");

    let casbin_val = CasbinLayer::new(model, adapter).await;
    casbin_val.write().await.get_role_manager().write()
        .matching_fn(Some(key_match2), None);

    casbin_val
}


