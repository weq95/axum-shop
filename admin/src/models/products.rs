use serde::{Deserialize, Serialize};

use common::error::ApiResult;

use crate::models::product_skus::ProductSku;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Product {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub image: Vec<String>,
    pub on_sale: bool,
    pub rating: f32,
    pub sold_count: u32,
    pub review_count: u32,
    pub price: f64,
    pub skus: Vec<ProductSku>,
}

impl Product {
    /// 创建
    pub async fn create(product: Self) -> ApiResult<i64> {
        Ok(1)
    }

    pub async fn get(product_id: u64) -> ApiResult<Self> {
        todo!()
    }

    /// 列表
    pub async fn products() -> ApiResult<Self> {
        todo!()
    }

    /// 更新
    pub async fn update(product: Self) -> ApiResult<bool> {
        todo!()
    }

    /// 删除
    pub async fn delete(product_id: u64) -> ApiResult<bool> {
        todo!()
    }

    /// 商品sku
    pub async fn skus(&mut self) {
        todo!()
    }
}
