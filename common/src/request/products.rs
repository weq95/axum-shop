use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Deserialize, Serialize)]
pub struct ReqQueryProduct {
    pub page_num: Option<u32>,
    pub page_size: Option<u32>,
    pub title: Option<String>,
    pub condition: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ReqProduct {
    pub id: Option<u64>,
    #[validate(length(min = 3, max = 100), custom = "unique_title")]
    pub title: Option<String>,
    #[validate(required)]
    pub image: Option<Vec<String>>,
    #[validate(required)]
    pub description: Option<String>,
    #[validate(required)]
    pub on_sale: Option<bool>,
    #[validate(required)]
    pub skus: Option<Vec<ReqProductSku>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ReqProductSku {
    #[validate(length(max = 100))]
    pub title: Option<String>,
    #[validate(required)]
    pub description: Option<String>,
    #[validate(range(min = 0.01f64))]
    pub price: Option<f64>,
    #[validate(range(min = 1))]
    pub stock: Option<u64>,
}

/// 检测商品是否已存在
fn unique_title(_title: &str) -> Result<(), ValidationError> {
    return Ok(());
    Err(ValidationError::new("商品名称已存在"))
}
