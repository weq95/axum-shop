use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;

use common::error::{ApiError, ApiResult};
use common::Pagination;

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
    pub pay_method: PayMethod,
    pub pay_no: Option<String>,
    pub refund_status: RefundStatus,
    pub refund_no: Option<String>,
    pub closed: bool,
    pub reviewed: bool,
    pub ship_status: ShipStatus,
    pub extra: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// 支付方式
#[derive(Debug, sqlx::Type)]
#[repr(i16)]
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
#[derive(Debug, sqlx::Type)]
#[repr(i16)]
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
#[derive(Debug, sqlx::Type)]
#[repr(i16)]
pub enum ShipStatus {
    // 处理中
    Processing = 0,
    // 待收货
    ToBeReceived = 1,
    // 已收货
    Received = 2,
}

impl Default for PayMethod {
    fn default() -> Self {
        PayMethod::AliPay
    }
}

impl Into<i16> for PayMethod {
    fn into(self) -> i16 {
        self as i16
    }
}

impl Default for RefundStatus {
    fn default() -> Self {
        RefundStatus::No
    }
}

impl Into<i16> for RefundStatus {
    fn into(self) -> i16 {
        self as i16
    }
}

impl Default for ShipStatus {
    fn default() -> Self {
        ShipStatus::Processing
    }
}

impl Into<i16> for ShipStatus {
    fn into(self) -> i16 {
        self as i16
    }
}

impl Orders {
    // 创建订单
    pub async fn create(
        user_id: i64,
        total_money: i64,
        address: sqlx::types::Json<HashMap<String, serde_json::Value>>,
        remark: String,
        order_items: HashMap<i64, Sku>,
    ) -> ApiResult<i64> {
        let mut tx = common::postgres().await.begin().await?;
        let order_id = sqlx::query(
            "INSERT INTO orders (no,user_id,address,total_amount,remark,ship_status) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id"
        )
            .bind(Self::get_order_no().await?)
            .bind(user_id)
            .bind(json!(address))
            .bind(total_money)
            .bind(remark)
            .bind(0)
            .fetch_one(&mut tx)
            .await?.get::<i64, _>("id");

        if false == OrderItems::create(order_id, order_items, &mut tx).await? {
            tx.rollback().await?;
            return Err(ApiError::Error("创建商品订单失败".to_string()));
        }

        tx.commit().await?;

        Ok(order_id)
    }

    // 订单信息
    pub async fn get(id: i64, user_id: i64) -> ApiResult<Orders> {
        let result: Orders = sqlx::query_as("select * from orders where id = $1 and user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_one(common::postgres().await)
            .await?;

        Ok(result)
    }

    // 订单列表
    pub async fn index(pagination: Pagination<Orders>) -> ApiResult<()> {
        todo!()
    }

    // 更新订单
    pub async fn update_harvest_addr(
        id: i64,
        user_id: i64,
        addr: serde_json::Value,
    ) -> ApiResult<bool> {
        Ok(sqlx::query(
            "update order_items set updated_at = $1, address = $2 where id = $3 and user_id = $4",
        )
        .bind(chrono::Utc::now().naive_utc())
        .bind(addr)
        .bind(id)
        .bind(user_id)
        .execute(common::postgres().await)
        .await?
        .rows_affected()
            > 0)
    }

    // 获取订单号
    async fn get_order_no() -> ApiResult<String> {
        Ok(common::snow_id().await.to_string())
    }
}
