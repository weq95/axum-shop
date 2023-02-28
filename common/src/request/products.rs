use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

pub struct ReqQueryProduct {}

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
    Err(ValidationError::new("商品名称已存在"))
}
