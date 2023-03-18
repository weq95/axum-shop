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
    pub product_sku: sqlx::types::Json<Sku>,
    pub rating: i16,
    pub review: String,
    pub reviewed_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 订单详情中product_sku
#[derive(Debug, Serialize, Deserialize)]
pub struct Sku {
    id: i64,
    price: i64,
    amount: i16,
    title: String,
    descr: String,
}

impl OrderItems {
    pub async fn create(
        order_id: i64,
        items: HashMap<i64, Sku>,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        let item_len = &items.len();
        let mut sql_builder = String::new();
        let mut idx = 1i32;
        let mut arg_builder = sqlx::postgres::PgArguments::default();
        for (product_id, sku) in items.iter() {
            sql_builder.push_str(format!(" (${}, ${}, ${}),", idx, idx + 1, idx + 2).as_str());
            idx += 2;

            arg_builder.add(order_id);
            arg_builder.add(product_id);
            arg_builder.add(json!(sku));
        }

        Ok(sqlx::query_with(
            &*format!(
                "insert into order_items (order_id,product_id,product_sku) values {}",
                sql_builder[..sql_builder.len() - 1].to_string()
            ),
            arg_builder,
        )
        .execute(tx)
        .await?
        .rows_affected()
            == (*item_len as u64))
    }

    // 创建生成sku
    pub async fn generate_sku(
        id: i64,
        price: i64,
        amount: i16,
        title: String,
        descr: String,
    ) -> Sku {
        Sku {
            id,
            price,
            amount,
            title,
            descr,
        }
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
