use std::collections::HashMap;

use sqlx::postgres::types::PgMoney;
use sqlx::{Postgres, Transaction};

use common::error::ApiResult;

#[derive(Debug, sqlx::FromRow)]
pub struct OrderItems {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub product_sku: sqlx::types::Json<Sku>,
    pub rating: i8,
    pub review: String,
    pub reviewed_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 订单详情中product_sku
#[derive(Debug)]
pub struct Sku {
    id: i64,
    price: PgMoney,
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
        todo!()
    }

    // 创建生成sku
    pub async fn generate_sku(
        id: i64,
        price: PgMoney,
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
}
