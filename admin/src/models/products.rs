use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Executor, Row};
use sqlx::types::Json;

use common::error::{ApiError, ApiResult};
use common::Paginate;
use common::products::ReqQueryProduct;

use crate::models::product_skus::ProductSkuModel;

#[derive(Debug, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct ProductModel {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub image: Json<Vec<String>>,
    pub on_sale: bool,
    pub rating: i64,
    pub sold_count: i64,
    pub review_count: i32,
    pub price: f64,
    pub skus: Vec<ProductSkuModel>,
}

impl ProductModel {
    /// 创建
    pub async fn create(product: ProductModel) -> ApiResult<u64> {
        let mut tx = common::pgsql::db().await.begin().await?;

        let product_sku = product
            .skus
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
            .unwrap();

        let id = sqlx::query(
            "insert into products (title, description, image, on_sale, sku_price) values ($1, $2, $3, $4, $5) RETURNING id",
        )
            .bind(&product.title.clone())
            .bind(&product.description.clone())
            .bind(product.image.clone())
            .bind(&product.on_sale.clone())
            .bind(product_sku.price)
            .fetch_one(&mut tx)
            .await?.get::<i64, _>("id");

        ProductSkuModel::delete_product_sku(id, &mut tx).await?;

        if false == ProductSkuModel::add_product_sku(id, &product.skus, &mut tx).await? {
            tx.rollback().await?;
            return Err(ApiError::Error("添加商品sku失败, 请稍后重试".to_string()));
        }

        //添加sku
        tx.commit().await?;
        Ok(id as u64)
    }

    /// 商品详情
    pub async fn get(product_id: i64) -> ApiResult<Self> {
        let mut result: ProductModel = sqlx::query("select * from products where id = $1")
            .bind(product_id)
            .fetch_optional(common::pgsql::db().await)
            .await?.map(|row| {
            ProductModel {
                id: row.get::<i64, _>("id"),
                title: row.get("title"),
                description: row.get("description"),
                image: row.get::<Json<Vec<String>>, _>("image"),
                on_sale: row.get::<bool, _>("on_sale"),
                rating: row.get::<i64, _>("rating"),
                sold_count: row.get::<i64, _>("sold_count"),
                review_count: row.get::<i32, _>("review_count"),
                price: row.get::<f64, _>("sku_price"),
                skus: Vec::default(),
            }
        }).ok_or(ApiError::Error("NotFound".to_string()))?;

        result.skus().await?;

        Ok(result)
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
    pub async fn skus(&mut self) -> ApiResult<()> {
        match ProductSkuModel::skus(self.id).await {
            Ok(values) => {
                self.skus = values;

                Ok(())
            }
            Err(_e) => return Err(ApiError::Error(_e.to_string())),
        }
    }
}
