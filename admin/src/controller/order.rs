use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info};
use validator::Validate;

use common::{
    error::format_errors,
    jwt::Claims,
    order::{OrderEvaluate, OrderShip, ReqCreateOrder, ReqInstallments},
    rabbitmq::{RabbitMQDlxQueue, RabbitMQQueue},
    ApiResponse, PagePer, Pagination,
};

use crate::models::{
    address::UserAddress,
    coupons::Coupons,
    installments::{Installments, Status},
    order_items::{ItemProductSku, OrderItems},
    orders::Orders,
    product_skus::ProductSku,
};

pub struct OrderController;

impl OrderController {
    // 订单列表
    pub async fn index(
        Query(page_per): Query<PagePer>,
        Extension(user): Extension<Claims>,
        Query(inner): Query<HashMap<String, serde_json::Value>>,
    ) -> impl IntoResponse {
        let mut pagination = Pagination::new(vec![], page_per);
        if let Err(e) = Orders::index(user.id, inner, &mut pagination).await {
            return ApiResponse::fail_msg(e.to_string()).json();
        }

        ApiResponse::response(Some(pagination)).json()
    }

    // 订单详情
    pub async fn get(Path(id): Path<i64>, Extension(user): Extension<Claims>) -> impl IntoResponse {
        let order = match Orders::get(id, user.id).await {
            Ok(result) => result,
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
        };

        let order_items = match OrderItems::get(order.id).await {
            Ok(items) => items,
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
        };

        ApiResponse::response(Some(json!({
            "id": order.id,
            "ship_data": order.ship_data,
            "no": order.no,
            "user_id": order.user_id,
            "address": order.address,
            "total_amount":order.total_amount.0/100,
            "remark": order.remark,
            "paid_at": order.paid_at,
            "pay_method": order.pay_method.as_ref(),
            "pay_no": order.pay_no,
            "refund_status":  order.refund_status.as_ref(),
            "refund_no":order.refund_no,
            "closed": order.closed,
            "reviewed": order.reviewed,
            "ship_status": order.ship_status.as_ref(),
            "extra": order.extra,
            "created_at": order.created_at.format("%F %T").to_string(),
            "updated_at": order.updated_at.format("%F %T").to_string(),
            "items": order_items,
        })))
        .json()
    }

    // 保存订单
    pub async fn store(
        Extension(user): Extension<Claims>,
        Json(inner): Json<ReqCreateOrder>,
    ) -> impl IntoResponse {
        if let Err(e) = inner.validate() {
            return ApiResponse::success_code_data(common::FAIL, Some(json!(format_errors(e))))
                .json();
        }
        let mut ids = HashMap::new();

        if let Some(result) = &inner.products {
            for req in result {
                if let Err(e) = req.validate() {
                    return ApiResponse::success_code_data(
                        common::FAIL,
                        Some(json!(format_errors(e))),
                    )
                    .json();
                }

                ids.insert(req.product_id.unwrap(), req.product_sku_id.unwrap());
            }
        }

        let values = match ProductSku::products(ids).await {
            Ok(product_val) => product_val,
            Err(err) => return ApiResponse::fail_msg(err.to_string()).json(),
        };

        let address = match UserAddress::harvest_addr(inner.address_id.unwrap(), user.id).await {
            Ok(addr) => addr,
            Err(_err) => return ApiResponse::fail_msg("收获地址未找到".to_string()).json(),
        };

        let mut order_items: HashMap<i64, ItemProductSku> = HashMap::new();
        let mut total_money = 0i64;
        if let Some(order) = &inner.products {
            for (idx, item) in order.iter().enumerate() {
                match values.get(&item.product_id.unwrap()) {
                    Some(sku) => {
                        if false == sku.on_sale {
                            return ApiResponse::fail_msg(format!("第{}项商品未上线", idx + 1))
                                .json();
                        }

                        if sku.stock <= 0 {
                            return ApiResponse::fail_msg(format!("第{}项商品已售完", idx + 1))
                                .json();
                        }

                        if sku.stock < item.amount.unwrap() as i32 {
                            return ApiResponse::fail_msg(format!("第{}项商品库存不足", idx + 1))
                                .json();
                        }

                        total_money += sku.price;
                        order_items.insert(
                            sku.product_id,
                            //商品sku相关信息
                            ItemProductSku {
                                sku_id: sku.id,
                                title: sku.title.clone(),
                                descr: sku.descr.clone(),
                                amount: item.amount.unwrap() as i16,
                                price: sku.price,
                                picture: sku.picture.clone(),
                            },
                        );
                    }
                    None => {
                        return ApiResponse::fail_msg(format!("第{}项商品不存在", idx + 1)).json();
                    }
                }
            }
        }
        let mut coupon_code: Option<String> = None;
        if let Some(code) = inner.coupon_code {
            coupon_code = Some(code.clone());
            match Coupons::is_in_effect(code, Some(total_money)).await {
                Ok(bool_val) => {
                    if false == bool_val {
                        return ApiResponse::fail_msg("此优惠券不能使用".to_string()).json();
                    }
                }
                Err(e) => {
                    return ApiResponse::fail_msg(e.to_string()).json();
                }
            }
        }
        let result = Orders::create(
            user.id,
            total_money,
            sqlx::types::Json(address),
            inner.remark.unwrap(),
            coupon_code,
            order_items,
        )
        .await;
        match result {
            Ok(order_id) => {
                let delay_order = DelayOrder {
                    order_id: order_id,
                    user_id: user.id,
                    created_at: Some(chrono::Local::now().naive_local()),
                };
                if let Err(e) = delay_order.produce(1 * 60 * 1000).await {
                    error!("订单加入队列失败： {}", e);
                }
                ApiResponse::response(Some(json!({ "id": order_id }))).json()
            }
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 更新订单(收货信息)
    pub async fn update(
        Path(id): Path<i64>,
        Extension(user): Extension<Claims>,
        Json(addr_id): Json<i64>,
    ) -> impl IntoResponse {
        if id <= 0 || addr_id <= 0 {
            return ApiResponse::fail_msg("参数错误".to_string()).json();
        }

        let address = match UserAddress::harvest_addr(addr_id, user.id).await {
            Ok(result) => result,
            Err(_err) => return ApiResponse::fail_msg("收获地址未找到".to_string()).json(),
        };

        match Orders::update_harvest_addr(id, user.id, json!(address)).await {
            Ok(bool_val) => ApiResponse::response(Some(json!({ "status": bool_val }))).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    pub async fn delete() -> impl IntoResponse {
        todo!()
    }

    // 发货
    pub async fn ship(
        Extension(claims): Extension<Claims>,
        Json(payload): Json<OrderShip>,
    ) -> impl IntoResponse {
        if let Err(e) = payload.validate() {
            return ApiResponse::success_code_data(
                common::response::FAIL,
                Some(json!(format_errors(e))),
            )
            .json();
        }

        let company = payload.express_company.unwrap();
        let no = payload.express_no.unwrap();
        let id = payload.id.unwrap();

        match Orders::ship(claims.id, id, no, company).await {
            Ok(bool_val) => ApiResponse::response(Some(json!({
                "status": bool_val,
            })))
            .json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 收货
    pub async fn received(
        Extension(claims): Extension<Claims>,
        Path(id): Path<i64>,
    ) -> impl IntoResponse {
        match Orders::received(id, claims.id).await {
            Ok(bool_val) => ApiResponse::response(Some(json!({
                "status": bool_val,
            })))
            .json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 商品评价
    pub async fn evaluate(
        Extension(claims): Extension<Claims>,
        Json(payload): Json<OrderEvaluate>,
    ) -> impl IntoResponse {
        if let Err(e) = payload.validate() {
            return ApiResponse::success_code_data(
                common::response::FAIL,
                Some(json!(format_errors(e))),
            )
            .json();
        }

        let order = match OrderItems::detail(payload.id.unwrap(), payload.order_id.unwrap()).await {
            Ok(data) => data,
            Err(e) => {
                error!("商品评价查询订单信息错误： {}", e);
                return ApiResponse::fail_msg("订单不存在".to_string()).json();
            }
        };

        match order
            .evaluate(
                claims.id,
                payload.score.unwrap() as u8,
                payload.content.unwrap(),
            )
            .await
        {
            Ok(()) => ApiResponse::success().json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 商品评价列表
    pub async fn evaluate_list(
        Query(page_per): Query<PagePer>,
        Path(product_id): Path<i64>,
    ) -> impl IntoResponse {
        let mut pagination: Pagination<HashMap<String, serde_json::Value>> =
            Pagination::new(vec![], page_per);

        if let Err(e) = OrderItems::evaluate_list(product_id, &mut pagination).await {
            return ApiResponse::fail_msg(e.to_string()).json();
        }

        ApiResponse::response(Some(pagination)).json()
    }

    pub async fn pay_by_installments(
        Extension(user): Extension<Claims>,
        Json(payload): Json<ReqInstallments>,
    ) -> impl IntoResponse {
        let cfg = common::application_config().await;
        if payload.min_amount < cfg.min_installment_amount {
            return ApiResponse::fail_msg(format!(
                "最低可分期金额: {}",
                cfg.min_installment_amount
            ))
            .json();
        }

        if cfg.installment_fee_rate.contains_key(&payload.count) {
            return ApiResponse::fail_msg("分期参数不合法".to_string()).json();
        }

        let order = match Orders::get(payload.order_id, user.id).await {
            Err(_e) => {
                return ApiResponse::fail_msg("分期失败".to_string()).json();
            }
            Ok(value) => value,
        };

        if order.closed {
            return ApiResponse::fail_msg("订单已关闭".to_string()).json();
        }

        let _ = Installments::delete(order.id, Status::PENDING).await;

        let result = Installments::create(
            order.id as u64,
            user.id as u64,
            payload.count,
            order.total_amount,
        )
        .await;
        match result {
            Err(_e) => return ApiResponse::fail_msg("创建订单失败".to_string()).json(),
            Ok(id) => ApiResponse::response(Some(json!({
                "id": id,
            })))
            .json(),
        }
    }

    pub async fn installment_index(
        Query(page_per): Query<PagePer>,
        Extension(user): Extension<Claims>,
    ) -> impl IntoResponse {
        let mut pagination = Pagination::new(vec![], page_per);

        if let Err(e) = Installments::index(user.id, &mut pagination).await {
            return ApiResponse::fail_msg(e.to_string()).json();
        }

        return ApiResponse::response(Some(pagination)).json();
    }

    pub async fn installment_detail(
        Query(installment_id): Query<i64>,
        Extension(user): Extension<Claims>,
    ) -> impl IntoResponse {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DelayOrder {
    order_id: i64,
    user_id: i64,
    created_at: Option<chrono::NaiveDateTime>,
}

impl Default for DelayOrder {
    fn default() -> Self {
        DelayOrder {
            order_id: 0,
            user_id: 0,
            created_at: None,
        }
    }
}

#[axum::async_trait]
impl RabbitMQDlxQueue for DelayOrder {}

#[axum::async_trait]
impl RabbitMQQueue for DelayOrder {
    async fn callback(&self, data: Vec<u8>) {
        match serde_json::from_slice::<DelayOrder>(data.as_slice()) {
            Err(e) => {
                error!("数据解析失败，订单超时未被正确处理: {}", e);
            }
            Ok(order) => {
                let order = match Orders::get(order.order_id, order.user_id).await {
                    Err(err) => {
                        error!("订单不存在： {}", err);
                        return;
                    }
                    Ok(result) => {
                        if result.paid_at.is_some() {
                            // 订单已支付， 不再进行后续处理
                            return;
                        }
                        result
                    }
                };

                let items: Vec<HashMap<i64, i64>> = match OrderItems::get(order.id).await {
                    Err(err) => {
                        error!("订单详情不存在： {}", err);
                        return;
                    }
                    Ok(result) => result
                        .into_iter()
                        .map(|value| {
                            let mut sku_id: i64 = 0;

                            if let Some(value) = &value.product_sku.0.get("sku_id") {
                                if let Some(id) = value.as_i64() {
                                    sku_id = id
                                }
                            }

                            HashMap::from([(value.product_id, sku_id)])
                        })
                        .collect::<Vec<HashMap<i64, i64>>>(),
                };

                if let Ok(mut tx) = common::postgres().await.begin().await {
                    if let Err(err) = ProductSku::buckle_inventory(items, 1, &mut tx).await {
                        error!("订单超时未支付， 增加库存失败： {}", err);
                        tx.rollback().await.unwrap();
                        return;
                    }

                    tx.commit().await.unwrap();
                    info!("-------------------- success --------------------");
                    return;
                }

                error!("订单超时未支付，处理业务失败");
            }
        }
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn queue_name(&self) -> &'static str {
        "orders-queue"
    }

    fn exchange_name(&self) -> &'static str {
        "orders-exchange"
    }

    fn router_key(&self) -> &'static str {
        "orders-router-key"
    }
}
