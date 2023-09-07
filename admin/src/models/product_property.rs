use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Acquire, Executor, Postgres, Row, Transaction};

use common::error::{ApiError, ApiResult};

use crate::models::products::Product;

#[derive(Debug, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct ProductProperty {
    pub id: i64,
    pub product_id: i64,
    pub name: String,
    pub value: String,
}

impl ProductProperty {
    pub async fn product(&self) -> ApiResult<Product> {
        Product::get(self.product_id).await
    }

    pub async fn get(id: i64) -> ApiResult<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM product_properties WHERE id = $1")
            .bind(id)
            .fetch_one(common::postgres().await)
            .await
            .map_err(ApiError::from)
    }

    pub async fn create(
        product_id: i64,
        items: Vec<(&str, &str)>,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<()> {
        for row in items.iter() {
            sqlx::query(
                "INSERT INTO product_properties (product_id, name, value) VALUES ($1, $2, $3)",
            )
                .bind(product_id)
                .bind(&row.0)
                .bind(&row.1)
                .execute(&mut *tx)
                .await?;
        }

        Ok(())
    }

    // 商品的所有属性
    pub async fn propertys(product_id: i64) -> ApiResult<Vec<HashMap<String, serde_json::Value>>> {
        let mut result = HashMap::new();

        let _ = sqlx::query("select * from product_properties where product_id = $1")
            .bind(product_id)
            .fetch_all(common::postgres().await)
            .await?
            .iter()
            .map(|row| ProductProperty {
                id: row.get::<i64, _>("id"),
                product_id: row.get::<i64, _>("product_id"),
                name: row.get::<String, _>("name"),
                value: row.get::<String, _>("value"),
            })
            .map(|row| {
                let values = result.entry(row.name.clone()).or_insert(Vec::new());

                values.push(HashMap::from([
                    ("id".to_string(), json!(row.id)),
                    ("value".to_string(), json!(row.value.clone())),
                ]));
            })
            .collect::<Vec<()>>();

        let result = result.into_iter().map(|(key, val)| {
            HashMap::from([
                ("key".to_string(), json!(key)),
                ("value".to_string(), json!(val)),
            ])
        }).collect::<Vec<HashMap<String, serde_json::Value>>>();

        Ok(result)
    }
}
