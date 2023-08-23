use std::collections::HashMap;
use std::ops::Add;

use serde::Serialize;
use serde_json::json;
use sqlx::postgres::types::PgMoney;
use sqlx::Row;

use common::error::{ApiError, ApiResult};

use crate::models::installments::{Installments, Status};
use crate::models::{PayMethod, RefundStatus};

#[derive(Debug, sqlx::FromRow)]
pub struct InstallmentItems {
    pub id: u64,
    pub installment_id: u64,
    pub sequence: u16,
    pub base: PgMoney,
    pub fee: PgMoney,
    pub fine: PgMoney,
    pub due_date: chrono::NaiveDateTime,
    pub paid_at: Option<chrono::NaiveDateTime>,
    pub pay_method: PayMethod,
    pub refund_status: RefundStatus,
    pub crated_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl InstallmentItems {
    pub async fn installments(&self) -> ApiResult<Installments> {
        sqlx::query("select * from installments where id = $1")
            .bind(self.installment_id as i64)
            .fetch_optional(&*common::postgres().await)
            .await?
            .map(|row| Installments {
                id: row.get::<i64, _>("id") as u64,
                no: row.get::<String, _>("no"),
                user_id: row.get::<i64, _>("user_id") as u64,
                order_id: row.get::<i64, _>("order_id") as u64,
                total_amount: row.get::<PgMoney, _>("total_amount"),
                count: row.get::<i8, _>("count") as u8,
                fee_rate: row.get::<f32, _>("fee_rate"),
                fine_rate: row.get::<f32, _>("fine_rate"),
                status: row.get::<Status, _>("status"),
                crated_at: row.get::<chrono::NaiveDateTime, _>("crated_at"),
                updated_at: row.get::<chrono::NaiveDateTime, _>("updated_at"),
            })
            .ok_or(ApiError::Error("Not Found".to_string()))
    }

    pub fn get_total_attribute(&self) -> String {
        format!("{:.2}", self.base.add(self.fee).add(self.fine).0 / 100)
    }

    pub fn get_is_overdue_attribute(&self) -> bool {
        chrono::Local::now().naive_local().gt(&self.due_date)
    }

    pub async fn detail(
        installment_id: &i64,
    ) -> ApiResult<(
        Option<HashMap<&str, serde_json::Value>>,
        Vec<HashMap<&str, serde_json::Value>>,
    )> {
        let mut current: Option<HashMap<&str, serde_json::Value>> = None;
        let result = sqlx::query("select id,sequence,base,fee,fine,due_date,paid_at from installment_items where installment_id = $1")
            .bind(*installment_id)
            .fetch_all(common::postgres().await)
            .await?.iter().map(|row| {
            let base = row.get::<PgMoney, _>("base");
            let fee = row.get::<PgMoney, _>("fee");
            let fine = row.get::<PgMoney, _>("fine");
            let paid_at = row.get::<Option<chrono::NaiveDateTime>, _>("paid_at");

            if current.is_none() && paid_at.is_none() {
                current = Some(HashMap::from([
                    ("paid_at", json!(paid_at)),
                    ("base", json!(format!("{:.2}", base.0 as f64/ 1000.0))),
                    ("due_date", json!(row.get::<chrono::NaiveDateTime, _>("due_date"))),
                ]));
            }
            HashMap::from([
                ("id", json!(row.get::<i64, _>("id"))),
                ("installment_id", json!(*installment_id)),
                ("sequence", json!(row.get::<i8, _>("sequence") as u8)),
                ("base", json!(format!("{:.2}", base.0 as f64/ 1000.0))),
                ("fee", json!(format!("{:.2}", fee.0 as f64/ 1000.0))),
                ("fine", json!(format!("{:.2}", fine.0 as f64/ 1000.0))),
                ("due_date", json!(row.get::<chrono::NaiveDateTime, _>("due_date"))),
                ("paid_at", json!(paid_at)),
            ])
        }).collect::<Vec<HashMap<&str, serde_json::Value>>>();

        Ok((current, result))
    }
}
