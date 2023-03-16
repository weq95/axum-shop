use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::{Arguments, Postgres, QueryBuilder, Row, Transaction};

use common::error::ApiResult;

#[derive(Debug, Deserialize, Serialize, Default, sqlx::FromRow)]
pub struct ProductSku {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub price: f64,
    pub stock: i32,
    pub product_id: i64,
}

/// 验证订单信息
pub struct CustomProductSku {
    pub id: i64,
    pub product_id: i64,
    pub title: String,
    pub descr: String,
    pub stock: i64,
    pub on_sale: bool,
    pub price: i64,
}

impl ProductSku {
    pub async fn get(id: i64) -> ApiResult<Self> {
        let result: Self = sqlx::query_as("select * from product_skus where id = $1")
            .bind(id)
            .fetch_one(common::postgres().await)
            .await?;

        Ok(result)
    }

    /// 商品sku
    pub async fn skus(product_ids: Vec<i64>) -> ApiResult<HashMap<i64, Vec<Self>>> {
        let result: Vec<Self> =
            sqlx::query_as("select * from product_skus where product_id = any($1)")
                .bind(product_ids)
                .fetch_all(common::postgres().await)
                .await?;

        let mut data_map: HashMap<i64, Vec<Self>> = HashMap::new();
        for sku in result {
            let data = data_map.entry(sku.product_id).or_insert(Vec::new());
            data.push(sku);
        }

        Ok(data_map)
    }

    /// 添加商品的sku
    pub async fn add_product_sku(
        product_id: i64,
        skus: &Vec<ProductSku>,
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

    // 获取商品信息
    pub async fn products(ids: HashMap<i64, i64>) -> ApiResult<HashMap<i64, CustomProductSku>> {
        let mut rows = String::new();
        let mut idx: i32 = 1;
        let mut arg = sqlx::postgres::PgArguments::default();
        for (product_id, product_sku_id) in ids {
            arg.add(product_sku_id);
            arg.add(product_id);
            rows.push_str(format!(" (id = ${} and product_id = ${}) OR", idx, idx + 1).as_str());
            idx += 2;
        }

        let query_builder = format!("select sku.*,p.on_sale from ( SELECT id,product_id,stock FROM product_skus WHERE {} ) as sku \
        left join  products as p ON sku.product_id = p.id", &rows[..(rows.len() - 3)]);

        Ok(sqlx::query_with(&query_builder, arg)
            .fetch_all(common::postgres().await)
            .await?
            .iter()
            .map(|row| CustomProductSku {
                id: row.get::<i64, _>("id"),
                product_id: row.get::<i64, _>("product_id"),
                title: "".to_string(),
                descr: "".to_string(),
                stock: row.get::<i64, _>("stock"),
                on_sale: row.get::<bool, _>("id"),
                price: row.get::<i64, _>("stock"),
            })
            .map(|sku| (sku.product_id, sku))
            .collect::<HashMap<i64, CustomProductSku>>())
    }
}
