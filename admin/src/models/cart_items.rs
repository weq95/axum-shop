use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sqlx::Row;

use common::error::{ApiError, ApiResult};

#[derive(Debug, sqlx::FromRow, Deserialize, Serialize)]
pub struct CartItems {
    pub id: i64,
    pub user_id: i64,
    pub product_id: i64,
    pub product_sku_id: i64,
    pub amount: i16,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

pub enum IncrType {
    Add,
    Reduce,
}

impl CartItems {
    // 加入购物车
    pub async fn add(userid: i64, product_id: i64, sku_id: i64, amount: u16) -> ApiResult<u64> {
        let pg_row =
            sqlx::query("select id from cart_items where user_id = $1 and product_id = $2")
                .bind(userid)
                .bind(product_id)
                .fetch_one(common::postgres().await)
                .await
                .ok();

        if let Some(item) = pg_row {
            let item_id = item.get::<i64, _>("id");
            if CartItems::update_amount(item_id, IncrType::Add, amount).await? {
                return Ok(item_id as u64);
            }
            return Err(ApiError::Error("添加失败,请稍后重试".to_string()));
        }

        let detection_value = sqlx::query(
            "SELECT EXISTS (SELECT id FROM products WHERE id = $1 AND on_sale = TRUE)
UNION ALL
SELECT EXISTS (SELECT id FROM product_skus WHERE id = $2)",
        )
        .bind(product_id)
        .bind(sku_id)
        .fetch_all(common::postgres().await)
        .await?
        .iter()
        .map(|row| row.get::<bool, _>("exists"))
        .collect::<Vec<bool>>();

        if let Some(&product_bool) = detection_value.get(0) {
            if false == product_bool {
                return Err(ApiError::Error("商品已下架或商品不存在".to_string()));
            }
        }
        if let Some(&product_bool) = detection_value.get(1) {
            if false == product_bool {
                return Err(ApiError::Error("商品sku不存在".to_string()));
            }
        }

        Ok(sqlx::query("insert into cart_items (user_id,product_id,product_sku_id,amount) values ($1,$2,$3,$4) RETURNING id")
            .bind(userid)
            .bind(product_id)
            .bind(sku_id)
            .bind(amount as i16)
            .fetch_one(common::postgres().await)
            .await?.get::<i64, _>("id") as u64)
    }

    // 购物车数量增减
    pub async fn update_amount(id: i64, up_type: IncrType, val: u16) -> ApiResult<bool> {
        let mut sql_str = "update cart_items set amount = amount ".to_string();
        match up_type {
            IncrType::Add => {
                sql_str.push_str(" + $1 ");
            }
            IncrType::Reduce => {
                sql_str.push_str(" - $1 ");
            }
        }

        Ok(
            sqlx::query(&*(sql_str.to_owned() + " where amount > 0 and id = $2"))
                .bind(val as i16)
                .bind(id)
                .execute(common::postgres().await)
                .await?
                .rows_affected()
                > 0,
        )
    }

    // 删除
    pub async fn delete(id: Vec<i64>) -> ApiResult<u64> {
        Ok(sqlx::query("delete from cart_items where id = any($1)")
            .bind(id)
            .execute(common::postgres().await)
            .await?
            .rows_affected())
    }

    // 清空用户购物车
    pub async fn remove_user(user_id: i64) -> ApiResult<u64> {
        Ok(sqlx::query("delete from cart_items where user_id = $1")
            .bind(user_id)
            .execute(common::postgres().await)
            .await?
            .rows_affected())
    }

    // 删除购物车中商品
    pub async fn remove_product(product_id: i64) -> ApiResult<u64> {
        Ok(sqlx::query("delete from cart_items where product_id = $1")
            .bind(product_id)
            .execute(common::postgres().await)
            .await?
            .rows_affected())
    }

    // 删除购物车中sku
    pub async fn remove_product_sku(product_sku_id: i64) -> ApiResult<u64> {
        Ok(
            sqlx::query("delete from cart_items where product_sku_id = $1")
                .bind(product_sku_id)
                .execute(common::postgres().await)
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
                .fetch_all(common::postgres().await)
                .await?;

        let mut hash_data: HashMap<i64, CartItems> = HashMap::new();
        for item in result {
            hash_data.insert(item.product_id, item);
        }

        Ok(hash_data)
    }

    // 购物车关联的商品
    pub async fn product_exists(user_id: i64, product_ids: Vec<i64>) -> ApiResult<HashSet<i64>> {
        Ok(sqlx::query(
            "SELECT product_id FROM cart_items WHERE user_id =$1 and product_id = ANY($2)",
        )
        .bind(user_id)
        .bind(product_ids)
        .fetch_all(common::postgres().await)
        .await?
        .iter()
        .map(|row| row.get::<i64, _>("product_id"))
        .collect::<HashSet<i64>>())
    }
}
