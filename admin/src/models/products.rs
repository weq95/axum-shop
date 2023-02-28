use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::Row;

use common::error::ApiResult;
use common::products::ReqQueryProduct;
use common::Paginate;

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
    pub async fn create(product: Self) -> ApiResult<u64> {
        let mut transaction = common::pgsql::db().await.begin().await?;
        let id: u64 = sqlx::query(
            "insert into product (title, description, image, os_sale) values ($1, $2, $3, $4)",
        )
        .bind(&product.title.clone())
        .bind(&product.description.clone())
        .bind(&product.image.clone().join(","))
        .bind(product.on_sale)
        .fetch_one(&mut transaction)
        .await?
        .get::<i64, &str>("id") as u64;

        ProductSku::delete_product_sku(id, &mut transaction).await?;
        if false == ProductSku::add_product_sku(product.skus, &mut transaction).await? {
            return Ok(0u64);
        }
        //添加sku
        transaction.commit().await?;
        Ok(id)
    }

    pub async fn get(product_id: u64) -> ApiResult<Self> {
        todo!()
    }

    /// 列表
    pub async fn products(payload: ReqQueryProduct) -> ApiResult<Paginate<Product>> {
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
