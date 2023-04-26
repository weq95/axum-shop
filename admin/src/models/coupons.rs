use std::collections::HashMap;

use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgMoney;
use sqlx::Row;

use common::error::{ApiError, ApiResult};
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

impl From<CouponType> for i16 {
    fn from(value: CouponType) -> Self {
        match value {
            CouponType::Percent => 2,
            CouponType::Fixed => 1,
            CouponType::Unknown => 0,
        }
    }
}

impl From<i16> for CouponType {
    fn from(value: i16) -> Self {
        match value {
            1 => Self::Fixed,
            2 => Self::Percent,
            _ => Self::Unknown,
        }
    }
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
    pub async fn code_exits(code: &str, id: Option<i64>) -> ApiResult<bool> {
        let mut sql_string = " name = $1 and deleted_at is null ".to_string();
        if let Some(id) = id {
            sql_string.push_str(format!(" and id != {} ", id).as_str());
        }

        Ok(sqlx::query(&*format!(
            "SELECT EXISTS (SELECT 1 FROM coupons WHERE {}) AS exist",
            sql_string
        ))
        .bind(code)
        .fetch_one(common::postgres().await)
        .await?
        .get::<bool, _>("exist"))
    }

    // 优惠券code
    async fn find_available_code(length: Option<u8>) -> ApiResult<String> {
        let length = length.unwrap_or(16);
        let rng = rand::thread_rng();
        let mut i: u8 = 0;
        const MAX_NUM: u8 = 25;

        while i < MAX_NUM {
            let code_str: String = rng
                .clone()
                .sample_iter(Alphanumeric)
                .take(length as usize)
                .map(char::from)
                .collect::<String>();

            if false == Self::code_exits(&code_str, None).await? {
                return Ok(code_str);
            }

            i += 1;
        }

        Err(ApiError::Error("生成CODE码失败".to_string()))
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

    // 列表
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

        let result = sqlx::query(&*sql)
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

    // 创建
    pub async fn store(
        name: String,
        r#type: i16,
        discount: f64,
        total: i64,
        // 单位: 分
        min_amount: i64,
        not_before: Option<chrono::NaiveDateTime>,
        not_after: Option<chrono::NaiveDateTime>,
        enabled: bool,
    ) -> ApiResult<i64> {
        Ok(sqlx::query(
            "INSERT INTO coupons (name,code,type,value,total,min_amount,not_before,not_after,enabled,created_at)\
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
        )
            .bind(name)
            .bind(chrono::Local::now().timestamp().to_string())
            .bind(r#type)
            .bind(discount)
            .bind(total)
            .bind(min_amount)
            .bind::<Option<chrono::NaiveDateTime>>(not_before)
            .bind::<Option<chrono::NaiveDateTime>>(not_after)
            .bind(chrono::Local::now())
            .bind(enabled)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("id"))
    }

    // 详情
    pub async fn get(id: i64) -> ApiResult<Coupons> {
        let result: Coupons =
            sqlx::query_as("SELECT * FROM coupons where id = $1 and deleted_at is null")
                .bind(id)
                .fetch_one(common::postgres().await)
                .await?;

        Ok(result)
    }

    // 更新
    pub async fn update(
        id: i64,
        name: String,
        code: String,
        r#type: i16,
        discount: f64,
        total: i64,
        // 单位: 分
        min_amount: i64,
        not_before: Option<chrono::NaiveDateTime>,
        not_after: Option<chrono::NaiveDateTime>,
        enabled: bool,
    ) -> ApiResult<bool> {
        if Self::code_exits(&code.clone(), Some(id)).await? {
            return Err(ApiError::Error("优惠券码已存在, 请换一个试试".to_string()));
        }

        Ok(sqlx::query(
            "UPDATE coupons SET name=$1,code=$2,type=$3,value=$4,total=$5,min_amount=$6,not_before=$7,not_after=$8,enabled=$9 WHERE id=$10"
        )
            .bind(name)
            .bind(code)
            .bind(r#type)
            .bind(discount)
            .bind(total)
            .bind(min_amount)
            .bind::<Option<chrono::NaiveDateTime>>(not_before)
            .bind::<Option<chrono::NaiveDateTime>>(not_after)
            .bind(enabled)
            .bind(id)
            .execute(common::postgres().await)
            .await?.rows_affected() == 1u64
        )
    }

    // 软删除
    pub async fn delete(id: i64) -> ApiResult<bool> {
        Ok(
            sqlx::query("update coupons set deleted_at = $1 where id = $2")
                .bind(chrono::Local::now().naive_local())
                .bind(id)
                .execute(common::postgres().await)
                .await?
                .rows_affected()
                > 0,
        )
    }

    // 检测优惠券是否有效
    pub async fn is_in_effect(code: String) -> ApiResult<bool> {
        let row = sqlx::query("SELECT code,enabled,total,used,not_after,not_before FROM coupons WHERE code = $1 and deleted_at is NULL")
            .bind(code)
            .fetch_optional(common::postgres().await)
            .await?
            .ok_or(ApiError::Error("Not Found".to_string()))?;

        if false == row.get::<bool, _>("enabled") {
            return Err(ApiError::Error("优惠券不存在".to_string()));
        }

        if (row.get::<i64, _>("total") - row.get::<i64, _>("used")) <= 0 {
            return Err(ApiError::Error("该优惠券已被兑完".to_string()));
        }

        if let Some(not_before) = row.get::<Option<chrono::NaiveDateTime>, _>("not_before") {
            if not_before.gt(&chrono::Local::now().naive_local()) {
                return Err(ApiError::Error("该优惠券现在还不能使用".to_string()));
            }
        }
        if let Some(not_after) = row.get::<Option<chrono::NaiveDateTime>, _>("not_after") {
            if not_after.lt(&chrono::Local::now().naive_local()) {
                return Err(ApiError::Error("该优惠券已过期".to_string()));
            }
        }

        Ok(true)
    }
}
