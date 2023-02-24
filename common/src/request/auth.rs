use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Clone, Validate)]
pub struct ReqRoleUser {
    #[validate(required)]
    pub user_id: Option<u64>,
    #[validate(length(min = 3, max = 100, message = "名称必须在3-100字之间"))]
    pub name: Option<String>,
    #[validate(required)]
    pub domain: Option<String>,
}

#[derive(Deserialize, Clone, Validate)]
pub struct ReqRolePermissions {
    #[validate(length(min = 3, max = 100, message = "名称必须在3-100字之间"))]
    pub role_name: Option<String>,
    #[validate(length(min = 1, max = 100, message = "对象必须在1-100字之间"))]
    pub object: Option<String>,
    #[validate(required)]
    pub action: Option<String>,
    #[validate(required)]
    pub domain: Option<String>,
}
