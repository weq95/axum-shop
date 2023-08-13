use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::types::PgMoney;
use sqlx::{Postgres, Row, Transaction};

use common::error::{ApiError, ApiResult};
use common::Pagination;

use crate::models::products::Product;

#[derive(Debug, sqlx::FromRow)]
pub struct CrowdfundingProduct {
    pub id: i64,
    pub product_id: i64,
    pub target_amount: PgMoney,
    pub total_amount: PgMoney,
    pub user_count: i32,
    pub end_at: chrono::NaiveDateTime,
    pub status: Status,
}

#[repr(i16)]
#[derive(Debug, Serialize, Deserialize, PartialEq, sqlx::Type)]
pub enum Status {
    Funding = 0,
    Success = 1,
    Fail = 2,
}

impl Default for Status {
    fn default() -> Self {
        Self::Funding
    }
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Status::Funding => "众筹中".to_string(),
            Status::Success => "众筹成功".to_string(),
            Status::Fail => "众筹失败".to_string(),
        }
    }
}

impl CrowdfundingProduct {
    pub fn percent(&self) -> String {
        let value = self.total_amount.0 / self.target_amount.0;

        format!("{:.1$}", value, 2)
    }

    pub async fn product(&self) -> ApiResult<Product> {
        Product::get(self.product_id).await
    }

    // 列表
    pub async fn index(
        payload: HashMap<String, String>,
        pagination: &mut Pagination<serde_json::Value>,
    ) -> ApiResult<()> {
        let mut sql_str = "SELECT c.id,c.product_id,c.target_amount,c.total_amount,c.end_at,c.status,p.title,p.sku_price FROM crowdfunding_products as c LEFT JOIN products as p ON p.id = c.product_id WHERE c.deleted_at is NULL and p.on_sale = TRUE ".to_string();
        let mut count_str = "SELECT count(*) as count FROM crowdfunding_products as c LEFT JOIN products as p ON p.id = c.product_id WHERE c.deleted_at is NULL and p.on_sale = TRUE ".to_string();

        if let Some(title) = payload.get("title") {
            let str = format!(" and p.title::text like '{}%'", title);
            sql_str.push_str(str.as_str());
            count_str.push_str(str.as_str());
        }

        if let Some(min_money) = payload.get("min_money") {
            let money = min_money.parse::<i64>()?;
            let str = format!(" and p.sku_price::money >= {}", money);
            sql_str.push_str(str.as_str());
            count_str.push_str(str.as_str());
        }

        if let Some(max_money) = payload.get("max_money") {
            let money = max_money.parse::<i64>()?;
            let str = format!(" and p.sku_price::money <= {}", money);
            sql_str.push_str(str.as_str());
            count_str.push_str(str.as_str());
        }

        sql_str.push_str(&format!(
            " limit {} offset {}",
            pagination.limit(),
            pagination.offset()
        ));

        let result: Vec<serde_json::Value> = sqlx::query(&*sql_str)
            .fetch_all(common::postgres().await)
            .await?
            .iter()
            .map(|row| {
                let target_amount = row.get::<PgMoney, _>("target_amount");
                let total_amount = row.get::<PgMoney, _>("total_amount");
                json!({
                    "id": row.get::<i64, _>("id"),
                    "product_id": row.get::<i64, _>("product_id"),
                    "target_amount": target_amount.0,
                    "total_amount": total_amount.0,
                    "end_at": row.get::<chrono::NaiveDateTime, _>("end_at"),
                    "status": row.get::<Status, _>("status"),
                    "title": row.get::<i64, _>("title"),
                    "sku_price": row.get::<i64, _>("sku_price"),
                })
            })
            .collect::<Vec<serde_json::Value>>();

        let count = sqlx::query(&*count_str)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("count") as usize;

        pagination.set_total(count);
        pagination.set_data(result);

        Ok(())
    }

    //详情
    pub async fn get() -> ApiResult<Self> {
        sqlx::query("select * from crowdfunding_products where deleted_at is null")
            .fetch_optional(&*common::postgres().await)
            .await?
            .map(|row| CrowdfundingProduct {
                id: row.get::<i64, _>("id"),
                product_id: row.get::<i64, _>("product_id"),
                target_amount: row.get::<PgMoney, _>("target_amount"),
                total_amount: row.get::<PgMoney, _>("total_amount"),
                user_count: row.get::<i32, _>("user_count"),
                end_at: row.get::<chrono::NaiveDateTime, _>("end_at"),
                status: row.get::<Status, _>("status"),
            })
            .ok_or(ApiError::Error("NotFound".to_string()))
    }

    // 创建
    pub async fn store(
        product_id: i64,
        target_amount: PgMoney,
        end_at: chrono::NaiveDateTime,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<i64> {
        Ok(sqlx::query(
            "insert into crowdfunding_products (product_id,target_amount,end_at,\
        created_at,updated_at,status) values ($1, $2, $3, $4, $5, $6)",
        )
        .bind(product_id)
        .bind(target_amount)
        .bind(end_at)
        .bind(chrono::Local::now().naive_local())
        .bind(chrono::Local::now().naive_local())
        .bind(Status::Funding)
        .fetch_one(tx)
        .await?
        .get::<i64, _>("id"))
    }

    // 更新
    pub async fn update(
        id: i64,
        target_amount: PgMoney,
        end_at: chrono::NaiveDateTime,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        Ok(sqlx::query("update crowdfunding_products set target_amount=$1, end_at=$2, updated_at=$3 where id = $4 and deleted_at is null")
            .bind(target_amount)
            .bind(end_at)
            .bind(chrono::Local::now().naive_utc())
            .bind(id)
            .execute(tx)
            .await?.rows_affected() > 0)
    }

    // 删除
    pub async fn delete(id: i64) -> ApiResult<bool> {
        Ok(sqlx::query("update crowdfunding_products set deleted_at = $1 where id = $2 and deleted_at is null ")
            .bind(chrono::Local::now().naive_utc())
            .bind(id)
            .execute(&*common::postgres().await)
            .await?.rows_affected() > 0)
    }
}
