use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Clone, Validate)]
pub struct ReqRole {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub domain: Option<String>,
}

#[derive(Deserialize, Clone, Validate)]
pub struct ReqPermission {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub object: Option<String>,
    pub domain: Option<String>,
}