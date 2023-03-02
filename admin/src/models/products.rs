use std::any::Any;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Executor, Row};

use common::error::ApiResult;
use common::products::ReqQueryProduct;
use common::Paginate;

use crate::models::product_skus::ProductSkuModel;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProductModel {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub image: Vec<String>,
    pub on_sale: bool,
    pub rating: f32,
    pub sold_count: u32,
    pub review_count: u32,
    pub price: f64,
    pub skus: Vec<ProductSkuModel>,
}

impl ProductModel {
    /// 创建
    pub async fn create(product: ProductModel) -> ApiResult<u64> {
        let mut transaction = common::pgsql::db().await.begin().await?;

        let product_sku = product
            .skus
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
            .unwrap();

        let id = sqlx::query(
            "insert into products (title, description, image, on_sale, sku_price) values ($1, $2, $3, $4, $5)",
        )
            .bind(&product.title.clone())
            .bind(&product.description.clone())
            .bind(&json!(product.image.clone()))
            .bind(&product.on_sale.clone())
            .bind(product_sku.price)
            .execute(&mut transaction)
            .await?.type_id();

        dbg!(id);
        return Ok(0u64);

        /* ProductSkuModel::delete_product_sku(id, &mut transaction).await?;
        if false == ProductSkuModel::add_product_sku(&product.skus, &mut transaction).await? {
            return Ok(0u64);
        }
        //添加sku
        transaction.commit().await?;
        Ok(id as u64)*/
    }

    pub async fn get(product_id: u64) -> ApiResult<Self> {
        todo!()
    }

    /// 列表
    pub async fn products(payload: ReqQueryProduct) -> ApiResult<Paginate<ProductModel>> {
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
