use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct ReqCategories {
    #[validate(required)]
    pub parent_id: Option<i64>,
    #[validate(length(min = 2, max = 30, message = "内部名称必须在2-30个字符之间"))]
    pub name: Option<String>,
}
