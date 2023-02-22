use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};
use std::path::PathBuf;

use axum::{
    async_trait,
    body::{self, Body, BoxBody},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use casbin::{
    Adapter, CachedEnforcer, CoreApi,
    DefaultModel, error::AdapterError,
    Filter, function_map::{key_match2, regex_match}, Model,
    TryIntoAdapter, TryIntoModel,
};
use dotenv::dotenv;
use futures::future::BoxFuture;
use sqlx::{
    Arguments, error::Error as SqlError,
    FromRow, postgres::PgQueryResult, Row,
};
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
            let response_fn = |code: StatusCode, msg: String| -> Response<BoxBody> {
                ApiResponse::<i32>::fail_msg_code(u16::from(code), msg)
                    .response_body().into_response()
            };
            let option_vals = req.extensions().get::<CasbinVals>()
                .map(|x| x.to_owned());
            let vals = match option_vals {
                Some(val) => val,
                None => return Ok(response_fn(StatusCode::UNAUTHORIZED, "验证用户不存在".to_string())),
            };

            let path = req.uri().path().to_string();
            let method = req.method().clone().to_string();
            let mut lock = cloned_enforcer.write().await;
            if vals.subject.is_empty() {
                return Ok(response_fn(StatusCode::FORBIDDEN, "验证对象不能为空".to_string()));
            }

            let subject = vals.clone().subject;
            if let Some(domain) = vals.domain {
                println!("{} - {} - {} - {}", subject.clone(), domain.clone(), path.clone(), method.clone());
                return match lock.enforce_mut(vec![subject, domain, path, method]) {
                    Ok(bool_val) => {
                        drop(lock);
                        if false == bool_val {
                            return Ok(response_fn(StatusCode::FORBIDDEN, "您没有操作权限".to_string()));
                        }

                        let response: Response<BoxBody> = ready_inner.call(req).await?.map(body::boxed);
                        Ok(response)
                    }
                    Err(_e) => {
                        drop(lock);
                        Ok(response_fn(StatusCode::BAD_GATEWAY, _e.to_string()))
                    }
                };
            }

            match lock.enforce_mut(vec![subject, path, method]) {
                Ok(bool_val) => {
                    drop(lock);
                    if false == bool_val {
                        return Ok(response_fn(StatusCode::FORBIDDEN, "您没有操作权限".to_string()));
                    }

                    let response: Response<BoxBody> = ready_inner.call(req).await?.map(body::boxed);
                    Ok(response)
                }
                Err(_e) => {
                    drop(lock);
                    Ok(response_fn(StatusCode::BAD_GATEWAY, _e.to_string()))
                }
            }
        })
    }
}

pub async fn casbin_layer() -> CasbinLayer {
    let model = DefaultModel::from_file(PathBuf::from("./config/rbac_domains.conf"))
        .await.unwrap();

    let adapter = crate::pgsql::get_pg_adapter().await;
    let casbin_val = CasbinLayer::new(model, adapter).await;
    {
        casbin_val.write().await.get_role_manager().write()
            .matching_fn(Some(key_match2), None);
    }
    {
        casbin_val.write().await.get_role_manager().write()
            .matching_fn(Some(regex_match), None);
    }

    casbin_val
}

// ===================================== ↑ CasbinMiddleware ↑ ===================================== //
// ===================================== ↓   CasbinAdapter  ↓ ===================================== //

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

#[derive(Clone)]
pub struct PgSqlAdapter {
    is_filtered: Arc<AtomicBool>,
}

impl PgSqlAdapter {
    pub(crate) async fn new(pool: &ConnPool) -> Self {
        new(&pool).await.map(|_num| Self {
            is_filtered: Arc::new(AtomicBool::new(false)),
        }).unwrap()
    }

    pub(crate) fn save_policy_line<'a>(&self, ptype: &'a str, rule: &'a [String]) -> Option<NewCasbinRule<'a>> {
        if ptype.trim().is_empty() || rule.is_empty() { return None; }
        let mut new_rule = NewCasbinRule {
            ptype,
            v0: "",
            v1: "",
            v2: "",
            v3: "",
            v4: "",
            v5: "",
        };

        new_rule.v0 = &rule[0];
        if rule.len() > 1 { new_rule.v1 = &rule[1]; }
        if rule.len() > 2 { new_rule.v2 = &rule[2]; }
        if rule.len() > 3 { new_rule.v3 = &rule[3]; }
        if rule.len() > 4 { new_rule.v4 = &rule[4]; }
        if rule.len() > 5 { new_rule.v5 = &rule[5]; }

        Some(new_rule)
    }

    pub(crate) fn load_policy_line(&self, casbin_rule: &CasbinRule) -> Option<Vec<String>> {
        if casbin_rule.ptype.chars().next().is_some() {
            return self.normalize_policy(casbin_rule);
        }

        None
    }

    fn normalize_policy(&self, casbin_rule: &CasbinRule) -> Option<Vec<String>> {
        let mut result = vec![
            &casbin_rule.v0, &casbin_rule.v1,
            &casbin_rule.v2, &casbin_rule.v3,
            &casbin_rule.v4, &casbin_rule.v5,
        ];

        while let Some(last) = result.last() {
            if last.is_empty() { result.pop(); } else { break; }
        }

        if result.is_empty() { return None; }

        Some(result.iter().map(|&x| x.to_owned()).collect())
    }
}

#[async_trait]
impl Adapter for PgSqlAdapter {
    async fn load_policy(&self, m: &mut dyn Model) -> casbin::Result<()> {
        let rules = sqlx::query("SELECT * FROM casbin_rule")
            .fetch_all(crate::pgsql::db().await).await
            .map_err(|err| AdapterError(Box::new(err)))?
            .into_iter().map(|row| {
            CasbinRule {
                id: row.get::<i32, &str>("id"),
                ptype: row.get("ptype"),
                v0: row.get("v0"),
                v1: row.get("v1"),
                v2: row.get("v2"),
                v3: row.get("v3"),
                v4: row.get("v4"),
                v5: row.get("v5"),
            }
        }).collect::<Vec<CasbinRule>>();

        for casbin_rule in &rules {
            let rule = self.load_policy_line(casbin_rule);
            if let Some(ref sec) = casbin_rule.ptype.chars().next().map(|x| x.to_string()) {
                if let Some(t1) = m.get_mut_model().get_mut(sec) {
                    if let Some(t2) = t1.get_mut(&casbin_rule.ptype) {
                        if let Some(rule) = rule {
                            t2.get_mut_policy().insert(rule);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn load_filtered_policy<'a>(&mut self, m: &mut dyn Model, f: Filter<'a>) -> casbin::Result<()> {
        let (g_filter, p_filter) = filtered_where_values(&f);

        let rules = sqlx::query(r#"SELECT * from  casbin_rule WHERE
        (ptype LIKE 'g%' AND v0 LIKE $1 AND v1 LIKE $2 AND v2 LIKE $3 AND v3 LIKE $4 AND v4 LIKE $5 AND v5 LIKE $6 )
        OR
        (ptype LIKE 'p%' AND v0 LIKE $7 AND v1 LIKE $8 AND v2 LIKE $9 AND v3 LIKE $10 AND v4 LIKE $11 AND v5 LIKE $12 )"#)
            .bind(g_filter[0]).bind(g_filter[1]).bind(g_filter[2])
            .bind(g_filter[3]).bind(g_filter[4]).bind(g_filter[5])
            .bind(p_filter[0]).bind(p_filter[1]).bind(p_filter[2])
            .bind(p_filter[3]).bind(p_filter[4]).bind(p_filter[5])
            .fetch_all(crate::pgsql::db().await).await
            .map_err(|err| AdapterError(Box::new(err)))?
            .into_iter().map(|row| {
            CasbinRule {
                id: row.get::<i32, &str>("id"),
                ptype: row.get("ptype"),
                v0: row.get("v0"),
                v1: row.get("v1"),
                v2: row.get("v2"),
                v3: row.get("v3"),
                v4: row.get("v4"),
                v5: row.get("v5"),
            }
        }).collect::<Vec<CasbinRule>>();

        self.is_filtered.store(true, Ordering::SeqCst);

        for casbin_rule in &rules {
            if let Some(policy) = self.normalize_policy(casbin_rule) {
                if let Some(ref sec) = casbin_rule.ptype.chars().next().map(|x| x.to_string()) {
                    if let Some(t1) = m.get_mut_model().get_mut(sec) {
                        if let Some(t2) = t1.get_mut(&casbin_rule.ptype) {
                            t2.get_mut_policy().insert(policy);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn save_policy(&mut self, m: &mut dyn Model) -> casbin::Result<()> {
        let mut rules = Vec::new();
        if let Some(ast_map) = m.get_model().get("p") {
            for (ptype, ast) in ast_map {
                let new_rules = ast.get_policy().into_iter()
                    .filter_map(|x| self.save_policy_line(ptype, x));

                rules.extend(new_rules);
            }
        }

        if let Some(ast_map) = m.get_model().get("g") {
            for (ptype, ast) in ast_map {
                let new_rules = ast.get_policy().into_iter()
                    .filter_map(|x| self.save_policy_line(ptype, x));

                rules.extend(new_rules);
            }
        }

        let mut transaction = crate::pgsql::db().await.begin().await
            .map_err(|err| AdapterError(Box::new(err)))?;
        sqlx::query("DELETE FROM casbin_rule").execute(&mut transaction).await
            .map_err(|err| AdapterError(Box::new(err)))?;

        for rule in rules {
            sqlx::query("INSERT INTO casbin_rule ( ptype, v0, v1, v2, v3, v4, v5 )
                 VALUES ( $1, $2, $3, $4, $5, $6, $7 )")
                .bind(rule.ptype).bind(rule.v0).bind(rule.v1).bind(rule.v2)
                .bind(rule.v3).bind(rule.v4).bind(rule.v5)
                .execute(&mut transaction).await
                .and_then(|n| {
                    if PgQueryResult::rows_affected(&n) == 1 {
                        Ok(true)
                    } else { Err(SqlError::RowNotFound) }
                }).map_err(|err| AdapterError(Box::new(err)))?;
        }

        transaction.commit().await.map_err(|err| AdapterError(Box::new(err)))?;

        Ok(())
    }

    async fn clear_policy(&mut self) -> casbin::Result<()> {
        let mut transaction = crate::pgsql::db().await.begin().await
            .map_err(|err| AdapterError(Box::new(err)))?;
        sqlx::query("DELETE FROM casbin_rule").execute(&mut transaction).await
            .map_err(|err| AdapterError(Box::new(err)))?;
        transaction.commit().await.map_err(|err| AdapterError(Box::new(err)))?;

        Ok(())
    }

    fn is_filtered(&self) -> bool {
        self.is_filtered.load(Ordering::SeqCst)
    }

    async fn add_policy(&mut self, sec: &str, ptype: &str, rule: Vec<String>) -> casbin::Result<bool> {
        self.add_policies(sec, ptype, Vec::from([rule; 1])).await
    }

    async fn add_policies(&mut self, _sec: &str, ptype: &str, rules: Vec<Vec<String>>) -> casbin::Result<bool> {
        let new_rules: Vec<NewCasbinRule> = rules.iter()
            .filter_map(|x: &Vec<String>| self.save_policy_line(ptype, x))
            .collect::<Vec<NewCasbinRule>>();

        let mut transaction = crate::pgsql::db().await.begin().await
            .map_err(|err| AdapterError(Box::new(err)))?;

        for rule in new_rules {
            sqlx::query("INSERT INTO casbin_rule ( ptype, v0, v1, v2, v3, v4, v5 )
                 VALUES ( $1, $2, $3, $4, $5, $6, $7 )")
                .bind(rule.ptype).bind(rule.v0).bind(rule.v1).bind(rule.v2)
                .bind(rule.v3).bind(rule.v4).bind(rule.v5)
                .execute(&mut transaction).await
                .and_then(|n| {
                    if PgQueryResult::rows_affected(&n) == 1 {
                        Ok(true)
                    } else { Err(SqlError::RowNotFound) }
                }).map_err(|err| AdapterError(Box::new(err)))?;
        }

        transaction.commit().await.map_err(|err| AdapterError(Box::new(err)))?;
        Ok(true)
    }

    async fn remove_policy(&mut self, sec: &str, ptype: &str, rule: Vec<String>) -> casbin::Result<bool> {
        self.remove_policies(sec, ptype, Vec::from([rule; 1])).await
    }

    async fn remove_policies(&mut self, _sec: &str, ptype: &str, rules: Vec<Vec<String>>) -> casbin::Result<bool> {
        let mut transaction = crate::pgsql::db().await.begin().await
            .map_err(|err| AdapterError(Box::new(err)))?;

        for rule in rules {
            let rule = normalize_casbin_rule(rule);
            sqlx::query(r#" DELETE FROM casbin_rule WHERE
                    ptype = $1 AND v0 = $2 AND v1 = $3 AND
                    v2 = $4 AND v3 = $5 AND v4 = $6 AND v5 = $7"#)
                .bind(ptype).bind(&rule[0]).bind(&rule[1])
                .bind(&rule[2]).bind(&rule[3]).bind(&rule[4])
                .bind(&rule[5]).execute(&mut transaction).await
                .and_then(|n| {
                    if PgQueryResult::rows_affected(&n) == 1 {
                        Ok(true)
                    } else { Err(SqlError::RowNotFound) }
                }).map_err(|err| AdapterError(Box::new(err)))?;
        }

        transaction.commit().await.map_err(|err| AdapterError(Box::new(err)))?;

        Ok(true)
    }

    async fn remove_filtered_policy(&mut self, _sec: &str, ptype: &str,
                                    field_index: usize, field_values: Vec<String>) -> casbin::Result<bool> {
        if field_index > 5 || field_values.is_empty() || field_values.len() <= field_index {
            return Ok(false);
        }

        let field_vals = normalize_casbin_rule_option(field_values);
        let mut counter = 6 - field_index;
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

        let sql_str = "DELETE FROM casbin_rule WHERE ptype = ".to_owned() + ptype + " and ";
        sqlx::query_with(&*(sql_str.as_str().to_owned() + placeholder), arg)
            .execute(crate::pgsql::db().await)
            .await.map(|n| PgQueryResult::rows_affected(&n) >= 1)
            .map_err(|err| AdapterError(Box::new(err)))?;

        Ok(true)
    }
}


