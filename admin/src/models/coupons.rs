use std::collections::HashMap;

use rand::distributions::Alphanumeric;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgMoney;
use sqlx::Row;

use common::error::ApiResult;
use common::Pagination;

#[derive(Debug, sqlx::FromRow)]
pub struct Coupons {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub r#type: CouponType,
    pub value: f32,
    pub total: i64,
    pub used: i64,
    pub min_amount: PgMoney,
    pub not_before: Option<chrono::NaiveDateTime>,
    pub not_after: Option<chrono::NaiveDateTime>,
    pub enabled: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[repr(i16)]
pub enum CouponType {
    // 未知
    Unknown = 0,
    // 固定金额
    Fixed = 1,
    // 比例
    Percent = 2,
}

impl ToString for CouponType {
    fn to_string(&self) -> String {
        match self {
            Self::Unknown => "未知".to_string(),
            Self::Fixed => "固定金额".to_string(),
            Self::Percent => "比列".to_string(),
        }
    }
}

impl Coupons {
    // 检测优惠码是否存在
    pub async fn exits(code: &str) -> ApiResult<bool> {
        Ok(
            sqlx::query("SELECT EXISTS (SELECT 1 FROM coupons WHERE name = $1) AS exist")
                .bind(code)
                .fetch_one(common::postgres().await)
                .await?
                .get::<bool, _>("exist"),
        )
    }

    // 优惠券code
    pub async fn find_available_code(length: Option<u8>) -> ApiResult<String> {
        let length = length.unwrap_or(16);
        let rng = rand::thread_rng();
        Ok(loop {
            let code_str: String = rng
                .clone()
                .sample_iter(Alphanumeric)
                .take(length as usize)
                .map(char::from)
                .collect::<String>();

            if false == Self::exits(&code_str).await? {
                break code_str;
            }
        })
    }

    // 描述
    pub fn descr_attr(r#type: CouponType, value: f32, min_amount: PgMoney) -> String {
        let mut descr_val = String::new();

        let min_amount = min_amount.0 / 100;
        if min_amount > 0i64 {
            descr_val = format!("满{}", min_amount.to_string().as_str());
        }
        if r#type == CouponType::Percent {
            return format!("{}优惠{}%", descr_val, value.trunc().to_string().as_str());
        }

        format!("{}减{}", descr_val, value.trunc().to_string().as_str())
    }

    pub async fn index(
        inner: HashMap<String, serde_json::Value>,
        pagination: &mut Pagination<HashMap<String, serde_json::Value>>,
    ) -> ApiResult<()> {
        let mut sql = "SELECT id,name,code,type,value,total,used,min_amount,enabled,created_at FROM coupons where deleted_at is null ".to_string();
        let mut sql_total =
            "select count(*) as total from coupons where deleted_at is null ".to_string();

        if let Some(name) = inner.get("name") {
            let name = common::string_trim_yh(name);
            sql.push_str(format!(" and name::text like '{}%' ", name).as_str());
            sql_total.push_str(format!("and name::text like '{}%' ", name).as_str());
        }

        if let Some(code) = inner.get("code") {
            let code = common::string_trim_yh(code);
            sql.push_str(format!(" and code like '{}%' ", code).as_str());
            sql_total.push_str(format!(r#" and code like '{}%' "#, code).as_str());
        }

        if let Some(created_at) = inner.get("created_at") {
            sql.push_str(format!(" and created_at >= '{}'", created_at).as_str());
            sql_total.push_str(format!(" and created_at >= '{}'", created_at).as_str());
        }

        sql.push_str(" order by created_at desc limit $1 offset $2");

        let mut result = sqlx::query(&*sql)
            .bind(pagination.limit())
            .bind(pagination.offset())
            .fetch_all(common::postgres().await)
            .await?
            .into_iter()
            .map(|row| {
                let coupon_type = row.get::<CouponType, _>("type");

                HashMap::from([
                    (
                        "id".to_string(),
                        serde_json::to_value(row.get::<i64, _>("id")).unwrap(),
                    ),
                    (
                        "name".to_string(),
                        serde_json::to_value(row.get::<String, _>("name")).unwrap(),
                    ),
                    (
                        "code".to_string(),
                        serde_json::to_value(row.get::<String, _>("code")).unwrap(),
                    ),
                    (
                        "t_name".to_string(),
                        serde_json::to_value(coupon_type.to_string()).unwrap(),
                    ),
                    (
                        "descr".to_string(),
                        serde_json::to_value(Self::descr_attr(
                            coupon_type,
                            row.get::<f32, _>("value"),
                            row.get::<PgMoney, _>("min_amount"),
                        ))
                        .unwrap(),
                    ),
                    (
                        "dosage".to_string(),
                        serde_json::to_value(format!(
                            "{}/{}",
                            row.get::<i64, _>("used"),
                            row.get::<i64, _>("total")
                        ))
                        .unwrap(),
                    ),
                    (
                        "enabled".to_string(),
                        serde_json::to_value(row.get::<bool, _>("enabled")).unwrap(),
                    ),
                    (
                        "created_at".to_string(),
                        serde_json::to_value(common::time_ymd_his(
                            row.get::<chrono::NaiveDateTime, _>("created_at"),
                        ))
                        .unwrap(),
                    ),
                ])
            })
            .collect::<Vec<HashMap<String, serde_json::Value>>>();

        let total = sqlx::query(&*sql_total)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("total");

        pagination.set_total(total as usize);
        pagination.set_data(result);

        Ok(())
    }
}
