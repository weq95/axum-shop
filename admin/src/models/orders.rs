use std::collections::HashMap;

use serde_json::json;
use sqlx::Row;

use common::error::{ApiError, ApiResult};
use common::Pagination;

use crate::models::coupons::Coupons;
use crate::models::order_items::{ItemProductSku, OrderItems};
use crate::models::product_skus::ProductSku;

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
    pub ship_data: sqlx::types::Json<Vec<HashMap<String, serde_json::Value>>>,
    pub extra: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub coupon_id: i64,
}

/// 支付方式
#[derive(Debug, PartialEq, sqlx::Type)]
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
#[derive(Debug, PartialEq, sqlx::Type)]
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
#[derive(Debug, PartialEq, sqlx::Type)]
#[repr(i16)]
pub enum ShipStatus {
    // 处理中
    Processing = 0,
    // 待收货
    ToBeReceived = 1,
    // 已收货
    Received = 2,
}

impl ToString for PayMethod {
    fn to_string(&self) -> String {
        match self {
            PayMethod::Unknown => "未支付".to_string(),
            PayMethod::AliPay => "支付宝".to_string(),
            PayMethod::Wechat => "微信支付".to_string(),
            PayMethod::GooglePay => "Google支付".to_string(),
            PayMethod::PayPal => "PayPal支付".to_string(),
        }
    }
}

impl ToString for RefundStatus {
    fn to_string(&self) -> String {
        match self {
            RefundStatus::No => "未退货".to_string(),
            RefundStatus::AlreadyApplied => "已申请退款".to_string(),
            RefundStatus::Waiting => "退款中".to_string(),
            RefundStatus::Yes => "退款成功".to_string(),
            RefundStatus::Fail => "退款失败".to_string(),
        }
    }
}

impl ToString for ShipStatus {
    fn to_string(&self) -> String {
        match self {
            ShipStatus::Processing => "处理中".to_string(),
            ShipStatus::ToBeReceived => "待收货".to_string(),
            ShipStatus::Received => "已收货".to_string(),
        }
    }
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
        coupon_code: Option<String>,
        order_items: HashMap<i64, ItemProductSku>,
    ) -> ApiResult<i64> {
        let mut tx = common::postgres().await.begin().await?;
        if let Some(code) = coupon_code {
            if false == Coupons::use_coupon(code, &mut tx).await? {
                return Err(ApiError::Error("此优惠券不符合使用条件".to_string()));
            }
        }

        let ship_data: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let order_id = sqlx::query(
            "INSERT INTO orders (no,user_id,address,total_amount,remark,ship_status,ship_data) VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id"
        )
            .bind(Self::get_order_no().await?)
            .bind(user_id)
            .bind(json!(address))
            .bind(total_money)
            .bind(remark)
            .bind::<i16>(ShipStatus::Processing.into())
            .bind(json!(ship_data))
            .fetch_one(&mut tx)
            .await?.get::<i64, _>("id");

        let (bool_val, item_ids): (bool, Vec<HashMap<i64, i64>>) =
            OrderItems::create(order_id, order_items, &mut tx).await?;
        if false == bool_val {
            tx.rollback().await?;
            return Err(ApiError::Error("创建商品订单失败".to_string()));
        }

        ProductSku::buckle_inventory(item_ids, -1, &mut tx).await?;

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
    pub async fn index(
        user_id: i64,
        inner: HashMap<String, serde_json::Value>,
        pagination: &mut Pagination<HashMap<String, serde_json::Value>>,
    ) -> ApiResult<()> {
        let mut order_ids: Vec<i64> = Vec::new();
        let mut sql = "select id,total_amount,pay_method,refund_status,created_at from orders where user_id = $1".to_string();
        let mut sql_total = "select count(*) as total from orders where user_id = $1 ".to_string();

        if let Some(start_time) = inner.get("start_time") {
            sql.push_str(format!(" and created_at >= '{}' ", start_time).as_str());
            sql_total.push_str(format!(" and created_at >= '{}' ", start_time).as_str());
        }

        if let Some(end_time) = inner.get("end_time") {
            sql.push_str(format!(" and created_at <= '{}' ", end_time).as_str());
            sql_total.push_str(format!(" and created_at <= '{}' ", end_time).as_str());
        }

        sql.push_str(" order by created_at desc limit $2 offset $3");

        let mut result = sqlx::query(&*sql)
            .bind(user_id)
            .bind(pagination.limit())
            .bind(pagination.offset())
            .fetch_all(common::postgres().await)
            .await?
            .iter()
            .map(|row| {
                let order_id = row.get::<i64, _>("id");
                let pay_method = row.get::<PayMethod, _>("pay_method");
                let refund_status = row.get::<RefundStatus, _>("refund_status");
                order_ids.push(order_id);

                HashMap::from([
                    ("id".to_string(), serde_json::to_value(order_id).unwrap()),
                    (
                        "status".to_string(),
                        serde_json::to_value(Self::status_name(pay_method, refund_status)).unwrap(),
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
            .bind(user_id)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("total");
        let items = OrderItems::items(order_ids).await?;

        for val in result.iter_mut() {
            let order_id = val.get("id").unwrap().as_i64().unwrap();
            if let Some(item) = items.get(&order_id) {
                val.insert("items".to_string(), json!(item));
            }
        }

        pagination.set_total(total as usize);
        pagination.set_data(result);

        Ok(())
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

    // 订单状态
    fn status_name(pay_method: PayMethod, refund_status: RefundStatus) -> String {
        match refund_status {
            RefundStatus::No => match pay_method {
                PayMethod::Unknown => "未支付".to_string(),
                PayMethod::AliPay => "支付宝".to_string(),
                PayMethod::Wechat => "微信支付".to_string(),
                PayMethod::GooglePay => "Google支付".to_string(),
                PayMethod::PayPal => "PayPal".to_string(),
            },
            RefundStatus::Fail => "退款失败".to_string(),
            RefundStatus::Yes => "退款成功".to_string(),
            RefundStatus::Waiting => "退款中".to_string(),
            RefundStatus::AlreadyApplied => "已申请退款，等待审核".to_string(),
        }
    }

    // 发货
    pub async fn ship(userid: i64, id: i64, no: String, company: String) -> ApiResult<bool> {
        let order = Orders::get(id, userid).await?;
        if order.ship_status != ShipStatus::Processing || order.pay_method == PayMethod::Unknown {
            return Ok(false);
        }

        Ok(sqlx::query("update orders set ship_status = $1,ship_data=$2, updated_at = $3 where id = $4 and user_id = $5")
            .bind::<i16>(ShipStatus::ToBeReceived.into())
            .bind(json!(vec![
            HashMap::from([
            ("express_no", no),
            ("company", company),
        ])]))
            .bind(chrono::Local::now())
            .bind(id)
            .bind(userid)
            .execute(common::postgres().await)
            .await?.rows_affected() > 0)
    }

    // 确认收获
    pub async fn received(id: i64, userid: i64) -> ApiResult<bool> {
        let order = Orders::get(id, userid).await?;
        if order.ship_status != ShipStatus::ToBeReceived {
            return Ok(false);
        }

        Ok(sqlx::query(
            "update orders set ship_status = $1, updated_at = $2 where id = $3 and user_id = $4",
        )
        .bind::<i16>(ShipStatus::Received.into())
        .bind(chrono::Local::now())
        .bind(id)
        .bind(userid)
        .execute(common::postgres().await)
        .await?
        .rows_affected()
            > 0)
    }

    // 订单关联的优惠券
    pub async fn coupon(&self) -> ApiResult<Option<i64>> {
        if self.coupon_id <= 0 {
            return Ok(None);
        }

        Ok(Some(1))
    }
}
