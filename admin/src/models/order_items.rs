use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Arguments, Postgres, Transaction};

use common::error::ApiResult;

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct OrderItems {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub product_sku: sqlx::types::Json<HashMap<String, serde_json::Value>>,
    pub rating: i16,
    pub review: String,
    pub reviewed_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct ItemProductSku {
    pub sku_id: i64,
    pub title: String,
    pub descr: String,
    pub amount: i16,
    pub price: i64,
    pub picture: String,
}

impl OrderItems {
    pub async fn create(
        order_id: i64,
        items: HashMap<i64, ItemProductSku>,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<(bool, Vec<HashMap<i64, i64>>)> {
        let item_len = items.len() as u64;
        let mut sql_builder = String::new();
        let mut idx = 1i32;
        let mut arg_builder = sqlx::postgres::PgArguments::default();
        let mut item_ids: Vec<HashMap<i64, i64>> = Vec::new();
        for (product_id, item) in items.iter() {
            sql_builder.push_str(format!(" (${}, ${}, ${}),", idx, idx + 1, idx + 2).as_str());
            idx += 2;

            arg_builder.add(order_id);
            arg_builder.add(product_id);
            arg_builder.add(json!(item));
            item_ids.push(HashMap::from([(product_id.clone(), item.sku_id)]))
        }

        Ok((
            sqlx::query_with(
                &*format!(
                    "insert into order_items (order_id,product_id,product_sku) values {}",
                    sql_builder[..sql_builder.len() - 1].to_string()
                ),
                arg_builder,
            )
            .execute(tx)
            .await?
            .rows_affected()
                == item_len,
            item_ids,
        ))
    }

    // 获取子订单
    pub async fn get(order_id: i64) -> ApiResult<Vec<OrderItems>> {
        let result: Vec<OrderItems> =
            sqlx::query_as("select * from order_items where order_id = $1")
                .bind(order_id)
                .fetch_all(common::postgres().await)
                .await?;

        Ok(result)
    }

    // 订单列表详情
    pub async fn items(
        order_ids: Vec<i64>,
    ) -> ApiResult<HashMap<i64, HashMap<String, serde_json::Value>>> {
        let item_result: Vec<OrderItems> =
            sqlx::query_as("select * from order_items where order_id = any($1)")
                .bind(order_ids)
                .fetch_all(common::postgres().await)
                .await?;

        let item_result = item_result
            .iter()
            .map(|row| {
                HashMap::from([
                    ("id".to_string(), serde_json::to_value(row.id).unwrap()),
                    (
                        "order_id".to_string(),
                        serde_json::to_value(row.order_id).unwrap(),
                    ),
                    (
                        "product_sku".to_string(),
                        serde_json::to_value(row.product_sku.clone()).unwrap(),
                    ),
                ])
            })
            .map(|row| {
                let order_id = row.get("order_id").unwrap().as_i64().unwrap();
                (order_id, row)
            })
            .collect::<HashMap<i64, HashMap<String, serde_json::Value>>>();

        Ok(item_result)
    }

    // 删除字订单详情
    pub async fn delete(
        order_id: i64,
        product_id: i64,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        Ok(
            sqlx::query("delete from order_items where order_id = $1 and product_id = $2")
                .bind(order_id)
                .bind(product_id)
                .execute(tx)
                .await?
                .rows_affected()
                > 0,
        )
    }
}
