use serde::{Deserialize};
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct ReqAddressInfo {
    #[validate(required)]
    pub province: Option<i32>,
    #[validate(required)]
    pub city: Option<i32>,
    #[validate(required)]
    pub district: Option<i32>,
    #[validate(required)]
    pub street: Option<i32>,
    #[validate(length(min = 3, max = 255, message = "详细地址必须在3-255字之间"))]
    pub address: Option<String>,
    // #[validate(zip)] 使用自己的 zip 验证逻辑
    pub zip: Option<i32>,
    #[validate(length(min = 1, max = 30, message = "联系人必须在1-30字之间"))]
    pub contact_name: Option<String>,
    #[validate(phone)]
    pub contact_phone: Option<String>,
}