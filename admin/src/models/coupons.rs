use std::collections::HashMap;

use rand::distributions::Alphanumeric;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use common::error::ApiResult;
use common::Pagination;

#[derive(Debug, Serialize, Deserialize)]
pub struct Coupons {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub r#type: CouponType,
    pub value: f64,
    pub total: i64,
    pub used: i64,
    pub min_amount: f32,
    pub not_before: Option<chrono::NaiveDateTime>,
    pub not_after: Option<chrono::NaiveDateTime>,
    pub enabled: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum CouponType {
    Fixed,
    Percent,
}

impl ToString for CouponType {
    fn to_string(&self) -> String {
        match self {
            CouponType::Fixed => "固定金额".to_string(),
            CouponType::Percent => "比列".to_string(),
        }
    }
}

impl Coupons {
    // 检测优惠码是否存在
    pub async fn exits(code: &str) -> bool {
        sqlx::query("SELECT EXISTS (SELECT 1 FROM coupons WHERE name = $1) AS exist")
            .bind(code)
            .fetch_one(common::postgres().await)
            .await?.get::<bool, _>("exist")
    }

    // 优惠券code
    pub async fn find_available_code(length: Option<u8>) -> ApiResult<String> {
        let length = length.unwrap_or(16);
        let rng = rand::thread_rng();
        Ok(loop {
            let code_str: String = rng.clone().sample_iter(Alphanumeric)
                .take(length as usize)
                .map(char::from)
                .collect::<String>();

            if false == Self::exits(&code_str) {
                break code_str;
            }
        })
    }

    // 描述
    pub async fn descr_attr(&self) -> String {
        let mut descr_val = String::new();
        let re: Regex = Regex::new(r"\.?0+$").unwrap();

        if self.min_amount > 0f32 {
            descr_val = format!("满{}", re.replace(&self.min_amount.to_string(), ""));
        }
        if self.r#type == CouponType::Percent {
            return format!("{}优惠{}%", descr_val, re.replace(&self.value.to_string(), ""));
        }

        format!("{}减{}", descr_val, re.replace(&self.value.to_string(), ""))
    }

    pub async fn index(
        inner: HashMap<String, serde_json::Value>,
        pagination: &mut Pagination<HashMap<String, serde_json::Value>>,
    ) -> ApiResult<()> {
        let mut sql = ""
    }
}