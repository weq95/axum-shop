use std::collections::HashSet;

use chrono::NaiveDateTime;
use sqlx::Row;

use common::error::ApiResult;

use crate::models::products::ProductModel;

pub struct FavoriteProductsModel {
    pub id: i64,
    pub user_id: i64,
    pub product_id: i64,
    pub created_at: NaiveDateTime,
}

impl FavoriteProductsModel {
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
        Ok(sqlx::query(
            "insert into favorite_products (user_id, product_id, created_at) values ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(product_id)
        .bind(chrono::Local::now().naive_local())
        .execute(common::pgsql::db().await)
        .await?
        .rows_affected())
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
