use std::collections::HashMap;
use std::ops::Add;

use chrono::Timelike;
use serde::Serialize;
use serde_json::json;
use sqlx::postgres::types::PgMoney;
use sqlx::Row;

use common::{Pagination, utils};
use common::error::{ApiError, ApiResult};

use crate::models::installment_items::{InstallmentItems, PayMethod, RefundStatus};
use crate::models::orders::Orders;
use crate::models::user::Admin;

#[derive(Debug, sqlx::FromRow)]
pub struct Installments {
    pub id: u64,
    pub no: String,
    pub user_id: u64,
    pub order_id: u64,
    pub total_amount: PgMoney,
    pub count: u8,
    pub fee_rate: f32,
    pub fine_rate: f32,
    pub status: Status,
    pub crated_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[repr(i8)]
#[derive(Debug, sqlx::Type, Serialize)]
pub enum Status {
    PENDING,
    REPAYING,
    FINISHED,
}

impl AsRef<str> for Status {
    fn as_ref(&self) -> &str {
        match self {
            Status::FINISHED => "已完成",
            Status::PENDING => "未执行",
            Status::REPAYING => "还款中",
        }
    }
}

impl Installments {
    // 生成订单号
    pub async fn order_no() -> ApiResult<String> {
        let no = format!(
            "{}-{}",
            chrono::Local::now().format("%Y%m%d"),
            utils::get_random_str(8)
        );
        loop {
            let exists = sqlx::query(
                "SELECT exists(SELECT id FROM installments WHERE no = $1 AND deleted_at IS NULL)",
            )
                .bind(&no)
                .fetch_one(&*common::postgres().await)
                .await?
                .get::<bool, _>("exists");
            if !exists {
                break;
            }
        }

        Ok(no)
    }

    pub async fn get(id: u64) -> ApiResult<Self> {
        sqlx::query("select * from installments where id=$1")
            .bind(id as i64)
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

    pub async fn index(
        user_id: i64,
        pagination: &mut Pagination<HashMap<&str, serde_json::Value>>,
    ) -> ApiResult<()> {
        let result = sqlx::query("select id,no,user_id,order_id,total_amount,count,fee_rate,fine_rate,status from installments where user_id = $1 order by id desc limit $2 offset $3")
            .bind(user_id)
            .bind(pagination.limit())
            .bind(pagination.offset())
            .fetch_all(common::postgres().await)
            .await?.iter().map(|row| {
            let total_amount = row.get::<PgMoney, _>("total_amount");
            let fee_rate = row.get::<PgMoney, _>("fee_rate");
            let fine_rate = row.get::<PgMoney, _>("fine_rate");

            HashMap::from([
                ("id", json!(row.get::<i64, _>("id"))),
                ("no", json!(row.get::<String, _>("no"))),
                ("user_id", json!(row.get::<i64, _>("user_id"))),
                ("order_id", json!(row.get::<i64, _>("order_id"))),
                ("total_amount", json!(total_amount.0 / 100)),
                ("count", json!(row.get::<i8, _>("count"))),
                ("fee_rate", json!(fee_rate.0/ 100)),
                ("fine_rate", json!(fine_rate.0/ 100)),
                ("status", json!(row.get::<Status, _>("status"))),
            ])
        }).collect::<Vec<HashMap<&str, serde_json::Value>>>();

        let total = sqlx::query("select count(*) as total from installments where user_id = $1")
            .bind(user_id)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("total");

        pagination.set_total(total as usize);
        pagination.set_data(result);

        Ok(())
    }

    pub async fn create(
        order_id: u64,
        user_id: u64,
        count: u8,
        total_amount: PgMoney,
    ) -> ApiResult<i64> {
        let cfg = common::application_config().await;
        let fee_rate = cfg.installment_fee_rate.get(&count).unwrap().clone();
        // 明天零点
        let mut tomorrow = chrono::Local::now()
            .naive_local()
            .add(chrono::Duration::days(1))
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        println!("{:?}", tomorrow);
        let order_no = Self::order_no().await?;
        // 每期本金
        let principal = PgMoney::from(total_amount.0 / count as i64);
        // 每期手续费
        let fee_per_period = PgMoney::from(principal.0 * ((fee_rate * 100.0) as i64));

        let mut tx = common::postgres().await.begin().await?;
        let installment_id = sqlx::query("insert into installments (no,user_id,order_id,total_amount,count,fee_rate,fine_rate,status) values ($1,$1,$3,$4,$5,$6,$7,$8) RETURNING id")
            .bind(order_no)
            .bind(user_id as i64)
            .bind(order_id as i64)
            .bind(total_amount)
            .bind(count as i8)
            .bind(fee_rate)
            .bind(cfg.installment_fine_rate)
            .bind(Status::PENDING)
            .fetch_one(&mut tx)
            .await?.get::<i64, _>("id");

        for i in 0..count {
            sqlx::query("inset into installment_items (installment_id,sequence,base,fee,fine,due_date) values ($1,$2,$3,$4,$5,$6)")
                .bind(installment_id)
                .bind(i as i8)
                .bind(principal)
                .bind(fee_per_period)
                .bind(PgMoney::from(0))
                .bind(tomorrow.clone())
                .execute(&mut tx)
                .await?;

            tomorrow += chrono::Duration::days(30);
        }

        tx.commit().await?;

        Ok(installment_id)
    }

    // 用户信息
    pub async fn user(&self) -> ApiResult<Admin> {
        sqlx::query("select id,name,age,nickname,phone,email from users where id = $1")
            .bind(self.user_id as i64)
            .fetch_optional(&*common::postgres().await)
            .await?
            .map(|row| Admin {
                id: row.get::<i64, &str>("id"),
                age: row.get::<i16, &str>("age") as u8,
                name: row.get("name"),
                nickname: row.get("nickname"),
                phone: row.get("phone"),
                email: row.get("email"),
            })
            .ok_or(ApiError::Error("Not Found".to_string()))
    }

    // 订单信息
    pub async fn order(&self) -> ApiResult<Orders> {
        Orders::get(self.order_id as i64, self.user_id as i64).await
    }

    // 分期信息
    pub async fn items(&self) -> ApiResult<Vec<InstallmentItems>> {
        Ok(
            sqlx::query("select * from installment_items where installment_id = $1")
                .bind(self.id as i64)
                .fetch_all(&*common::postgres().await)
                .await?
                .iter()
                .map(|row| InstallmentItems {
                    id: row.get::<i64, _>("id") as u64,
                    installment_id: row.get::<i64, _>("installment_id") as u64,
                    sequence: row.get::<i16, _>("sequence") as u16,
                    base: row.get::<PgMoney, _>("base"),
                    fee: row.get::<PgMoney, _>("fee"),
                    fine: row.get::<PgMoney, _>("fine"),
                    due_date: row.get::<chrono::NaiveDateTime, _>("due_date"),
                    paid_at: row.get::<Option<chrono::NaiveDateTime>, _>("paid_at"),
                    pay_method: row.get::<PayMethod, _>("pay_method"),
                    refund_status: row.get::<RefundStatus, _>("refund_status"),
                    crated_at: row.get::<chrono::NaiveDateTime, _>("crated_at"),
                    updated_at: row.get::<chrono::NaiveDateTime, _>("updated_at"),
                })
                .collect::<Vec<InstallmentItems>>(),
        )
    }

    pub async fn delete(order_id: i64, status: Status) -> ApiResult<bool> {
        Ok(
            sqlx::query("delete from installments where order_id = $1 and status = $2")
                .bind(order_id)
                .bind(status)
                .execute(common::postgres().await)
                .await?
                .rows_affected()
                > 0,
        )
    }

    pub async fn detail(
        id: &i64,
        userid: i64,
    ) -> ApiResult<(
        Option<HashMap<&str, serde_json::Value>>,
        Vec<HashMap<&str, serde_json::Value>>,
    )> {
        struct Detail {
            id: i64,
            order_id: i64,
            total_amount: PgMoney,
            count: u8,
            fee_rate: f32,
            fine_rate: f32,
        }
        let installment = sqlx::query("select id,order_id,total_amount,count,fee_rate,fine_rate where id = $1 and user_id = $2")
            .bind(*id)
            .bind(userid)
            .fetch_optional(&*common::postgres().await)
            .await?.map(|row| {
            Detail {
                id: row.get::<i64, _>("id"),
                order_id: row.get::<i64, _>("order_id"),
                total_amount: row.get::<PgMoney, _>("total_amount"),
                count: row.get::<i8, _>("count") as u8,
                fee_rate: row.get::<f32, _>("fee_rate"),
                fine_rate: row.get::<f32, _>("fine_rate"),
            }
        }).ok_or(ApiError::Error("Not Found".to_string()))?;
        let (mut detail, list) = InstallmentItems::detail(id).await?;
        if let Some(mut val) = detail {
            val.insert("id", json!(installment.id));
            val.insert("order_id", json!(installment.order_id));
            val.insert(
                "total_amount",
                json!(format!("{:.2}", installment.total_amount.0 as f64 / 100.0)),
            );
            val.insert("count", json!(installment.count));
            val.insert("fee_rate", json!(format!("{:.2}", installment.fee_rate)));
            val.insert("fine_rate", json!(format!("{:.2}", installment.fine_rate)));

            detail = Some(val)
        }

        Ok((detail, list))
    }

    pub async fn overdue_items(ids: Vec<i64>) -> ApiResult<HashMap<i64, PgMoney>> {
        let mut result = HashMap::with_capacity(ids.len());
        let _ = sqlx::query("select id, fine_rate from installments where status=$1 and any($2)")
            .bind(Status::REPAYING)
            .bind(ids)
            .fetch_all(&*common::postgres().await)
            .await?.iter().map(|row| {
            let key = row.get::<i64, _>("id");
            let val = row.get::<PgMoney, _>("fine_rate");
            result.insert(key, val);
            ()
        }).collect::<Vec<()>>();

        Ok(result)
    }
}
