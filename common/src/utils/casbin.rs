use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

use axum::{
    body::{self, Body, BoxBody},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use casbin::{
    Adapter, CachedEnforcer, CoreApi,
    DefaultModel, error::AdapterError, FileAdapter,
    Filter, function_map::key_match2, Model,
    TryIntoAdapter, TryIntoModel,
};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sqlx::{Arguments, error::Error as SqlError, FromRow, postgres::PgQueryResult, Row};
use sqlx::postgres::PgArguments;
use tokio::sync::RwLock;
use tower::{Layer, Service};

use crate::{
    ApiResponse,
    error::ApiResult,
    pgsql::ConnPool,
};

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
        let enforcer: CachedEnforcer = CachedEnforcer::new(model, adapter).await.unwrap();
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

#[derive(Debug, FromRow)]
pub(crate) struct CasbinRule {
    pub id: i32,
    pub ptype: String,
    pub v0: String,
    pub v1: String,
    pub v2: String,
    pub v3: String,
    pub v4: String,
    pub v5: String,
}

#[derive(Debug)]
pub(crate) struct NewCasbinRule<'a> {
    pub ptype: &'a str,
    pub v0: &'a str,
    pub v1: &'a str,
    pub v2: &'a str,
    pub v3: &'a str,
    pub v4: &'a str,
    pub v5: &'a str,
}

/// 数组长度重置 length = 6, 不够补 空 字符串
fn normalize_casbin_rule(mut rule: Vec<String>) -> Vec<String> {
    rule.resize(6, String::new());

    rule
}

/// 数组长度重置 length = 6， 不够补 空 占位符
fn normalize_casbin_rule_option(rule: Vec<String>) -> Vec<Option<String>> {
    let mut rule_option = rule.iter().map(|x| Some(x.clone()))
        .collect::<Vec<Option<String>>>();

    rule_option.resize(6, None);
    rule_option
}

pub async fn new(conn: &ConnPool) -> ApiResult<u64> {
    Ok(sqlx::query(r#" CREATE TABLE IF NOT EXISTS casbin_rule (
                    id SERIAL PRIMARY KEY,
                    ptype VARCHAR NOT NULL,
                    v0 VARCHAR NOT NULL,
                    v1 VARCHAR NOT NULL,
                    v2 VARCHAR NOT NULL,
                    v3 VARCHAR NOT NULL,
                    v4 VARCHAR NOT NULL,
                    v5 VARCHAR NOT NULL,
                    CONSTRAINT unique_key_sqlx_adapter UNIQUE(ptype, v0, v1, v2, v3, v4, v5));
    "#).execute(conn).await?.rows_affected())
}

pub async fn remove_policy(conn: &ConnPool, pt: &str, rule: Vec<String>) -> ApiResult<bool> {
    let arr: Vec<Vec<String>> = Vec::from([rule; 1]);

    remove_policies(conn, pt, arr).await
}

pub async fn remove_policies(conn: &ConnPool, pt: &str, rules: Vec<Vec<String>>) -> ApiResult<bool> {
    let mut transaction = conn.begin().await?;

    for rule in rules {
        let rule = normalize_casbin_rule(rule);
        sqlx::query(r#" DELETE FROM casbin_rule WHERE
                    ptype = $1 AND
                    v0 = $2 AND
                    v1 = $3 AND
                    v2 = $4 AND
                    v3 = $5 AND
                    v4 = $6 AND
                    v5 = $7"#).bind(pt).bind(&rule[0]).bind(&rule[1])
            .bind(&rule[2]).bind(&rule[3]).bind(&rule[4])
            .bind(&rule[5]).execute(&mut transaction).await
            .and_then(|n| {
                if PgQueryResult::rows_affected(&n) == 1 {
                    return Ok(true);
                }

                return Err(SqlError::RowNotFound);
            })?;
    }

    transaction.commit().await?;
    Ok(true)
}

pub async fn remove_filtered_policy(conn: &ConnPool, pt: &str, field_idx: usize, field_vals: Vec<String>) -> ApiResult<()> {
    let field_vals = normalize_casbin_rule_option(field_vals);

    let mut counter = 6 - field_idx;
    let mut arg = PgArguments::default();
    let mut placeholder = String::with_capacity(counter);
    let mut idx = 1;

    while counter > 0 {
        let v_field_n = "v".to_owned() + idx.to_string().as_str();//v1
        let v_name = "$".to_owned() + (idx + 1).to_string().as_str();//$2

        // (v1 is NULL OR v1 = COALESCE($2,v1)) AND
        placeholder.push_str(&*(" (".to_owned() + v_field_n.clone().as_str() + " is null or " +
            v_field_n.clone().as_str() + " coalesce(" + v_name.clone().as_str() + "," + v_field_n.clone().as_str()
            + ")) and "));
        arg.add(&field_vals[idx]);

        idx += 1;
        counter -= 1;
    }


    let placeholder = &placeholder[1..placeholder.len() - 5];


    let sql_str = "DELETE FROM casbin_rule WHERE ptype = ".to_owned() + pt + " and ";
    sqlx::query_with(&*(sql_str.as_str().to_owned() + placeholder), arg).execute(conn)
        .await.map(|n| PgQueryResult::rows_affected(&n) >= 1)?;

    Ok(())
}

pub(crate) async fn load_policy(conn: &ConnPool) -> ApiResult<Vec<CasbinRule>> {
    Ok(sqlx::query("SELECT * FROM casbin_rule").fetch_all(conn)
        .await?.into_iter().map(|row| {
        CasbinRule {
            id: row.get::<i64, &str>("id") as i32,
            ptype: row.get("ptype"),
            v0: row.get("v0"),
            v1: row.get("v1"),
            v2: row.get("v2"),
            v3: row.get("v3"),
            v4: row.get("v4"),
            v5: row.get("v5"),
        }
    }).collect::<Vec<CasbinRule>>())
}

fn filtered_where_values<'a>(filter: &Filter<'a>) -> ([&'a str; 6], [&'a str; 6]) {
    let mut g_filter: [&'a str; 6] = ["%", "%", "%", "%", "%", "%"];
    let mut p_filter: [&'a str; 6] = ["%", "%", "%", "%", "%", "%"];

    for (idx, val) in filter.g.iter().enumerate() {
        if val != &"" { g_filter[idx] = val; }
    }

    for (idx, val) in filter.p.iter().enumerate() {
        if val != &"" { p_filter[idx] = val; }
    }

    (g_filter, p_filter)
}

pub(crate) async fn load_filtered_policy<'a>(conn: &ConnPool, filter: &Filter<'_>) -> ApiResult<Vec<CasbinRule>> {
    let (g_filter, p_filter) = filtered_where_values(filter);

    Ok(sqlx::query(r#"SELECT * from  casbin_rule WHERE
        (ptype LIKE 'g%' AND v0 LIKE $1 AND v1 LIKE $2 AND v2 LIKE $3 AND v3 LIKE $4 AND v4 LIKE $5 AND v5 LIKE $6 )
        OR
        (ptype LIKE 'p%' AND v0 LIKE $7 AND v1 LIKE $8 AND v2 LIKE $9 AND v3 LIKE $10 AND v4 LIKE $11 AND v5 LIKE $12 )"#)
        .bind(g_filter[0]).bind(g_filter[1]).bind(g_filter[2])
        .bind(g_filter[3]).bind(g_filter[4]).bind(g_filter[5])
        .bind(p_filter[0]).bind(p_filter[1]).bind(p_filter[2])
        .bind(p_filter[3]).bind(p_filter[4]).bind(p_filter[5])
        .fetch_all(conn).await?.into_iter().map(|row| {
        CasbinRule {
            id: row.get::<i64, &str>("id") as i32,
            ptype: row.get("ptype"),
            v0: row.get("v0"),
            v1: row.get("v1"),
            v2: row.get("v2"),
            v3: row.get("v3"),
            v4: row.get("v4"),
            v5: row.get("v5"),
        }
    }).collect::<Vec<CasbinRule>>())
}

pub(crate) async fn save_policies(conn: &ConnPool, rules: Vec<NewCasbinRule<'_>>) -> ApiResult<()> {
    let mut transaction = conn.begin().await?;
    sqlx::query("DELETE FROM casbin_rule").execute(&mut transaction).await?;

    for rule in rules {
        sqlx::query("INSERT INTO casbin_rule ( ptype, v0, v1, v2, v3, v4, v5 )
                 VALUES ( $1, $2, $3, $4, $5, $6, $7 )")
            .bind(rule.ptype).bind(rule.v0).bind(rule.v1).bind(rule.v2)
            .bind(rule.v3).bind(rule.v4).bind(rule.v5)
            .execute(&mut transaction).await
            .and_then(|n| {
                if PgQueryResult::rows_affected(&n) == 1 {
                    return Ok(true);
                }

                Err(SqlError::RowNotFound)
            })?;
    }

    transaction.commit().await?;

    Ok(())
}

pub(crate) async fn add_policy(conn: &ConnPool, rule: NewCasbinRule<'_>) -> ApiResult<bool> {
    let arr: Vec<NewCasbinRule<'_>> = Vec::from([rule; 1]);

    add_policies(conn, arr).await
}

pub(crate) async fn add_policies(conn: &ConnPool, rules: Vec<NewCasbinRule<'_>>) -> ApiResult<bool> {
    let mut transaction = conn.begin().await?;

    for rule in rules {
        sqlx::query("INSERT INTO casbin_rule ( ptype, v0, v1, v2, v3, v4, v5 )
                 VALUES ( $1, $2, $3, $4, $5, $6, $7 )")
            .bind(rule.ptype).bind(rule.v0).bind(rule.v1).bind(rule.v2)
            .bind(rule.v3).bind(rule.v4).bind(rule.v5)
            .execute(&mut transaction).await
            .and_then(|n| {
                if PgQueryResult::rows_affected(&n) == 1 {
                    return Ok(true);
                }

                Err(SqlError::RowNotFound)
            })?;
    }

    transaction.commit().await?;
    Ok(true)
}

pub(crate) async fn clear_policy(conn: &ConnPool) -> ApiResult<()> {
    let mut transaction = conn.begin().await?;
    sqlx::query("DELETE FROM casbin_rule").execute(&mut transaction).await?;
    transaction.commit().await?;

    Ok(())
}