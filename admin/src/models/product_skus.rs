use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder, Transaction};

use common::error::ApiResult;

#[derive(Debug, Deserialize, Serialize, Default, sqlx::FromRow)]
pub struct ProductSkuModel {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub price: f64,
    pub stock: i32,
    pub product_id: i64,
}


impl ProductSkuModel {
    pub async fn get(id: i64) -> ApiResult<Self> {
        let result: Self = sqlx::query_as("select * from product_skus where id = $1")
            .bind(id)
            .fetch_one(common::pgsql::db().await)
            .await?;

        Ok(result)
    }

    /// 获取某商品的全部sku
    pub async fn skus(product_id: i64) -> ApiResult<Vec<Self>> {
        let result: Vec<Self> = sqlx::query_as("select * from product_skus where product_id = $1")
            .bind(product_id)
            .fetch_all(common::pgsql::db().await)
            .await?;

        Ok(result)
    }

    /// 添加商品的sku
    pub async fn add_product_sku(
        product_id: i64,
        skus: &Vec<ProductSkuModel>,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        let mut query_build: QueryBuilder<Postgres> = sqlx::QueryBuilder::new(
            "insert into product_skus (title, description, price, stock, product_id) ",
        );

        query_build.push_values(skus.as_slice().iter().take(skus.len()), |mut b, sku| {
            b.push_bind(sku.title.clone())
                .push_bind(sku.description.clone())
                .push_bind(sku.price)
                .push_bind(sku.stock)
                .push_bind(product_id);
        });

        let rows_num = query_build.build().execute(tx).await?.rows_affected();
        Ok(rows_num as usize == skus.len())
    }

    /// 删除关联商品的全部sku
    pub async fn delete_product_sku(
        product_id: i64,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        let rows_num = sqlx::query("delete from product_skus where product_id = $1")
            .bind(product_id)
            .execute(tx)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }
}
