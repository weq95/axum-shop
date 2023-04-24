use chrono::Utc;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Validate, Deserialize, Serialize, Clone)]
pub struct ReqCoupon {
    #[validate(length(min = 3, max = 30, message = "名称不能超过30个字符"))]
    pub name: Option<String>,
    pub code: Option<String>,
    #[validate(required)]
    pub r#type: Option<i16>,
    #[validate(required)]
    pub value: Option<f64>,
    #[validate(range(min = 1, message = "可发行数不能 < 1"))]
    pub total: Option<i64>,
    #[validate(range(min = 0.01, message = "使用门槛最低为0.01元"))]
    pub min_amount: Option<f64>,
    pub not_before: Option<chrono::DateTime<Utc>>,
    pub not_after: Option<chrono::DateTime<Utc>>,
    #[validate(required)]
    pub enable: Option<bool>,
}
