use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Clone, Validate)]
pub struct ReqRole {
    pub id: Option<i64>,
    #[validate(length(min = 3, max = 100, message = "名称必须在3-100字之间"))]
    pub name: Option<String>,
    pub domain: Option<String>,
}

#[derive(Deserialize, Clone, Validate)]
pub struct ReqPermission {
    pub id: Option<i64>,
    #[validate(length(min = 3, max = 100, message = "名称必须在3-100字之间"))]
    pub name: Option<String>,
    #[validate(length(min = 1, max = 100, message = "对象必须在1-100字之间"))]
    pub object: Option<String>,
    #[validate(required)]
    pub action: Option<String>,
    pub domain: Option<String>,
}