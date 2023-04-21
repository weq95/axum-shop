use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Validate, Deserialize, Serialize, Clone)]
pub struct ReqCoupon {
    #[validate(length(min = 3, max = 30, message = "名称不能超过30个字符"))]
    pub name: Option<String>,
    #[validate(length(min = 5, max = 20, message = "优惠码必须在5-20字符之间"))]
    pub code: Option<String>,
    #[validate(required)]
    pub r#type: Option<String>,
    #[validate(required)]
    pub amin: Option<f64>,
    #[validate(min = 1, message = "total > 0")]
    pub total: Option<i64>,
    #[validate(min = 0.01, message = "使用门槛最低为0.01元")]
    pub min_amount: Option<f64>,
    #[validate(required)]
    pub start_time: Option<chrono::NaiveDateTime>,
    #[validate(required)]
    pub end_time: Option<chrono::NaiveDateTime>,
    #[validate(required)]
    pub enable: Option<bool>,
}