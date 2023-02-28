use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};

use common::error::ApiResult;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProductSku {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub price: f64,
    pub stock: u64,
    pub product_id: u64,
}

impl ProductSku {
    /// 添加商品的sku
    pub async fn add_product_sku(
        skus: Vec<ProductSku>,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        let mut sql = sqlx::QueryBuilder::new(
            "insert into product_skus (title, description, price, stock, product_id) values ",
        );

        let mut idx = 1;
        // ($1, $1, $3, $4)
        let sku_len = skus.len();
        for (i, sku) in skus.into_iter().enumerate() {
            let field1 = (" ($".to_owned() + idx.to_string().as_str()).to_string();
            sql.push(field1).push_bind(sku.title.clone());
            idx += 1;
            let field2 = (",$".to_owned() + idx.to_string().as_str()).to_string();
            sql.push(field2).push_bind(sku.description.clone());
            idx += 1;
            let field3 = (",$".to_owned() + idx.to_string().as_str()).to_string();
            sql.push(field3).push_bind(sku.price);
            idx += 1;
            let field4 = (",$".to_owned() + idx.to_string().as_str()).to_string();
            sql.push(field4).push_bind(sku.stock as i64);
            idx += 1;

            let field5 = if i == sku_len - 1 {
                (",%".to_owned() + idx.to_string().as_str() + ")").to_string()
            } else {
                (",%".to_owned() + idx.to_string().as_str() + "), ").to_string()
            };
            sql.push(field5).push_bind(sku.product_id as i64);
            idx += 1;
        }

        let rows_num = sql.build().execute(transaction).await?.rows_affected();
        Ok(rows_num as usize == sku_len)
    }

    /// 删除关联商品的全部sku
    pub async fn delete_product_sku(
        product_id: u64,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        let rows_num = sqlx::query("delete from products_skus where product_id = $1")
            .bind(product_id as i64)
            .execute(transaction)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }
}
