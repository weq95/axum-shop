use std::collections::HashSet;

use chrono::NaiveDateTime;
use sqlx::Row;

use common::error::{ApiError, ApiResult};

pub struct FavoriteProducts {
    pub id: i64,
    pub user_id: i64,
    pub product_id: i64,
    pub created_at: NaiveDateTime,
}

impl FavoriteProducts {
    /// 商品收藏信息
    pub async fn favorite_products(user_id: i64, product_ids: Vec<i64>) -> ApiResult<HashSet<i64>> {
        Ok(sqlx::query(
            "select id from favorite_products where user_id = $1 and product_id = any($2)",
        )
        .bind(user_id)
        .bind(product_ids)
        .fetch_all(common::pgsql::db().await)
        .await?
        .iter()
        .map(|row| row.get::<i64, _>("id"))
        .collect::<HashSet<i64>>())
    }

    /// 收藏商品
    pub async fn favorite(user_id: i64, product_id: i64) -> ApiResult<u64> {
        let exists = sqlx::query(
            "select exists (select id from products where id = $1 and on_sale = true)
UNION ALL
select exists (select id from favorite_products where user_id = $2 and product_id = $3)",
        )
        .bind(product_id)
        .bind(user_id)
        .bind(product_id)
        .fetch_all(common::pgsql::db().await)
        .await?
        .iter()
        .map(|row| row.get::<bool, _>("exists"))
        .collect::<Vec<bool>>();

        if let Some(&product_exists) = exists.get(0) {
            if false == product_exists {
                return Err(ApiError::Error("收藏失败,商品不存在".to_string()));
            }
        }
        if let Some(&favorite_exists) = exists.get(1) {
            if true == favorite_exists {
                return Err(ApiError::Error("该商品已收藏,不能重复收藏".to_string()));
            }
        }

        Ok(sqlx::query(
            "insert into favorite_products (user_id, product_id, created_at) values ($1, $2, $3) RETURNING id",
        )
            .bind(user_id)
            .bind(product_id)
            .bind(chrono::Local::now().naive_local())
            .fetch_one(common::pgsql::db().await)
            .await?
            .get::<i64, _>("id") as u64)
    }

    /// 取消收藏
    pub async fn un_favorite(user_id: i64, product_id: i64) -> ApiResult<u64> {
        Ok(
            sqlx::query("delete from favorite_products where user_id = $1 and product_id = $2")
                .bind(user_id)
                .bind(product_id)
                .execute(common::pgsql::db().await)
                .await?
                .rows_affected(),
        )
    }

    /// 商品下架或商品删除收藏
    pub async fn un_favorite_product(product_id: i64) -> ApiResult<u64> {
        Ok(
            sqlx::query("delete from favorite_products where product_id = $1")
                .bind(product_id)
                .execute(common::pgsql::db().await)
                .await?
                .rows_affected(),
        )
    }

    /// 删除用户的收藏
    pub async fn un_favorite_user(user_id: i64) -> ApiResult<u64> {
        Ok(
            sqlx::query("delete from favorite_products where user_id = $1")
                .bind(user_id)
                .execute(common::pgsql::db().await)
                .await?
                .rows_affected(),
        )
    }
}
