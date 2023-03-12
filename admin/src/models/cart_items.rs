use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::Row;

use common::error::ApiResult;

#[derive(Debug, sqlx::FromRow, Deserialize, Serialize)]
pub struct CartItems {
    pub id: i64,
    pub user_id: i64,
    pub product_id: i64,
    pub product_sku_id: i64,
    pub amount: i32,
    pub created_at: chrono::NaiveDateTime,
}

pub enum IncrType {
    Add,
    Reduce,
}

impl CartItems {
    // 加入购物车
    pub async fn add(item: Self) -> ApiResult<u64> {
        let item_id = sqlx::query("select id from cart_items where user_id = $1,product_id = $2")
            .bind(item.user_id)
            .bind(item.product_id)
            .fetch_one(common::pgsql::db().await)
            .await?
            .get::<i64, _>("id");
        if item_id > 0 {
            if CartItems::update_amount(item_id, IncrType::Add, 1).await? {
                return Ok(1);
            }
            return Ok(0);
        }

        Ok(sqlx::query("insert into cart_items (user_id,product_id,product_sku_id,amount,created_at) values ($1,$2,$3,$4,$5) RETURNING id")
            .bind(item.user_id)
            .bind(item.product_id)
            .bind(item.product_sku_id)
            .bind(item.amount)
            .bind(chrono::Utc::now())
            .fetch_one(common::pgsql::db().await)
            .await?.get::<i64, _>("id") as u64)
    }

    // 购物车数量增减
    pub async fn update_amount(id: i64, up_type: IncrType, val: u32) -> ApiResult<bool> {
        let amount_str = match up_type {
            IncrType::Add => format!(" amount - {} ", val),
            IncrType::Reduce => format!(" amount + {} ", val),
        };

        Ok(
            sqlx::query("update cart_items set amount = $1  where amount > 0 and id = $2")
                .bind(amount_str)
                .bind(id)
                .execute(common::pgsql::db().await)
                .await?
                .rows_affected()
                > 0,
        )
    }

    // 删除
    pub async fn delete(id: i64) -> ApiResult<u64> {
        Ok(sqlx::query("delete from cart_items where id = $1")
            .bind(id)
            .execute(common::pgsql::db().await)
            .await?
            .rows_affected())
    }

    // 清空用户购物车
    pub async fn remove_user(user_id: i64) -> ApiResult<u64> {
        Ok(sqlx::query("delete from cart_items where user_id = $1")
            .bind(user_id)
            .execute(common::pgsql::db().await)
            .await?
            .rows_affected())
    }

    // 删除购物车中商品
    pub async fn remove_product(product_id: i64) -> ApiResult<u64> {
        Ok(sqlx::query("delete from cart_items where product_id = $1")
            .bind(product_id)
            .execute(common::pgsql::db().await)
            .await?
            .rows_affected())
    }

    // 删除购物车中sku
    pub async fn remove_product_sku(product_sku_id: i64) -> ApiResult<u64> {
        Ok(
            sqlx::query("delete from cart_items where product_sku_id = $1")
                .bind(product_sku_id)
                .execute(common::pgsql::db().await)
                .await?
                .rows_affected(),
        )
    }

    // sku关联的商品
    pub async fn product_sku(
        user_id: i64,
        product_ids: Vec<i64>,
    ) -> ApiResult<HashMap<i64, CartItems>> {
        let result: Vec<CartItems> =
            sqlx::query_as("select * from cart_items where user_id = $1 and product_id = any($2)")
                .bind(user_id)
                .bind(product_ids)
                .fetch_all(common::pgsql::db().await)
                .await?;

        let mut hash_data: HashMap<i64, CartItems> = HashMap::new();
        for item in result {
            hash_data.insert(item.product_id, item);
        }

        Ok(hash_data)
    }
}
