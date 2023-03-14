use std::collections::HashMap;

use common::error::{ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::models::order_items::{OrderItems, Sku};

#[derive(Debug, sqlx::FromRow)]
pub struct Orders {
    pub id: i64,
    pub no: String,
    pub user_id: i64,
    pub address: sqlx::types::Json<HashMap<String, serde_json::Value>>,
    pub total_amount: sqlx::postgres::types::PgMoney,
    pub remark: String,
    pub paid_at: Option<chrono::NaiveDateTime>,
    pub pay_method: Option<PayMethod>,
    pub pay_no: Option<String>,
    pub refund_status: RefundStatus,
    pub refund_no: Option<String>,
    pub closed: bool,
    pub reviewed: bool,
    pub ship_status: ShipStatus,
    pub ship: sqlx::types::Json<Vec<String>>,
    pub extra: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 支付方式
#[derive(Debug)]
pub enum PayMethod {
    // 未支付
    Unknown = 0,
    // 支付宝
    AliPay = 1,
    // 微信支付
    Wechat = 2,
    // Google支付
    GooglePay = 3,
    // PayPal
    PayPal = 4,
}

/// 退款状态
#[derive(Debug)]
pub enum RefundStatus {
    // 否(未退款)
    No = 0,
    // 已申请
    AlreadyApplied = 1,
    // 等待中
    Waiting = 2,
    // 是(退款成功)
    Yes = 3,
    // 退款失败
    Fail = 4,
}

/// 物理状态
#[derive(Debug)]
pub enum ShipStatus {
    // 处理中
    Processing = 0,
    // 待收货
    ToBeReceived = 1,
    // 已收货
    Received = 2,
}

/// 评价状态
#[derive(Debug, Serialize, Deserialize)]
pub enum Reviewed {
    // 未评价
    No = 0,
    // 已评价
    Yes = 1,
}

impl Default for PayMethod {
    fn default() -> Self {
        PayMethod::AliPay
    }
}

impl Default for RefundStatus {
    fn default() -> Self {
        RefundStatus::No
    }
}

impl Default for ShipStatus {
    fn default() -> Self {
        ShipStatus::Processing
    }
}

impl Default for Reviewed {
    fn default() -> Self {
        Reviewed::No
    }
}

impl Orders {
    // 创建订单
    pub async fn create(
        user_id: i64,
        total_money: sqlx::postgres::types::PgMoney,
        address: sqlx::types::Json<HashMap<String, serde_json::Value>>,
        remark: String,
        order_items: HashMap<i64, Sku>,
    ) -> ApiResult<i64> {
        let order_no = Self::get_order_no().await?;
        let mut tx = common::pgsql::db().await.begin().await?;
        let order_id = sqlx::query(
            "insert into orders (no,user_id,address,total_amount,remark) values ($1,$2,$3,$4,$5) RETURNING id"
        )
            .bind(order_no)
            .bind(user_id)
            .bind(address)
            .bind(total_money)
            .bind(remark)
            .fetch_one(&mut tx)
            .await?.get::<i64, _>("id");

        if false == OrderItems::create(order_id, order_items, &mut tx).await? {
            tx.rollback().await?;
            return Err(ApiError::Error("创建商品订单失败".to_string()));
        }

        tx.commit().await?;

        Ok(order_id)
    }

    // 获取订单号
    pub async fn get_order_no() -> ApiResult<String> {
        todo!()
    }
}