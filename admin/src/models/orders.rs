use std::collections::HashMap;

use serde_json::json;
use sqlx::Row;

use common::error::{ApiError, ApiResult};
use common::Pagination;

use crate::models::coupons::Coupons;
use crate::models::order_items::{ItemProductSku, OrderItems};
use crate::models::product_skus::ProductSku;
use crate::models::{LogisticStatus, PayMethod, RefundStatus};

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
    pub ship_status: LogisticStatus,
    pub ship_data: sqlx::types::Json<Vec<HashMap<String, serde_json::Value>>>,
    pub extra: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub coupon_id: i64,
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
            .bind::<i8>(LogisticStatus::Processing.into())
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
                    ("id".to_string(), json!(order_id)),
                    (
                        "status".to_string(),
                        json!(Self::status_name(pay_method, refund_status)),
                    ),
                    (
                        "created_at".to_string(),
                        json!(common::time_ymd_his(
                            row.get::<chrono::NaiveDateTime, _>("created_at"),
                        )),
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
            RefundStatus::PENDING => pay_method.as_ref().to_string(),
            RefundStatus::FAILED => "退款失败".to_string(),
            RefundStatus::SUCCESS => "退款成功".to_string(),
            RefundStatus::Waiting => "退款中".to_string(),
            RefundStatus::PROCESSING => "已申请退款，等待审核".to_string(),
        }
    }

    // 发货
    pub async fn ship(userid: i64, id: i64, no: String, company: String) -> ApiResult<bool> {
        let order = Orders::get(id, userid).await?;
        if order.ship_status != LogisticStatus::Processing || order.pay_method == PayMethod::Unknown
        {
            return Ok(false);
        }

        Ok(sqlx::query("update orders set ship_status = $1,ship_data=$2, updated_at = $3 where id = $4 and user_id = $5")
            .bind::<i8>(LogisticStatus::ToBeReceived.into())
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
        if order.ship_status != LogisticStatus::ToBeReceived {
            return Ok(false);
        }

        Ok(sqlx::query(
            "update orders set ship_status = $1, updated_at = $2 where id = $3 and user_id = $4",
        )
        .bind::<i8>(LogisticStatus::Received.into())
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
