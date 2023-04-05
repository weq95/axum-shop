use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;
use validator::Validate;

use common::error::format_errors;
use common::jwt::Claims;
use common::order::ReqCreateOrder;
use common::rabbitmq::{RabbitMQDlxQueue, RabbitMQQueue};
use common::{ApiResponse, PagePer, Pagination};

use crate::models::address::UserAddress;
use crate::models::order_items::{ItemProductSku, OrderItems};
use crate::models::orders::Orders;
use crate::models::product_skus::ProductSku;

pub struct OrderController;

impl OrderController {
    // 订单列表
    pub async fn index(
        Query(page_per): Query<PagePer>,
        Extension(user): Extension<Claims>,
        Query(inner): Query<HashMap<String, serde_json::Value>>,
    ) -> impl IntoResponse {
        let mut pagination = Pagination::new(vec![], page_per);
        match Orders::index(user.id, inner, &mut pagination).await {
            Ok(()) => ApiResponse::response(Some(pagination)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
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
        "no": order.no,
        "user_id": order.user_id,
        "address": order.address,
        "total_amount":order.total_amount.0/100,
        "remark": order.remark,
        "paid_at": order.paid_at,
        "pay_method": Into::<i16>::into(order.pay_method),
        "pay_no": order.pay_no,
        "refund_status":  Into::<i16>::into(order.refund_status),
        "refund_no":order.refund_no,
        "closed": order.closed,
        "reviewed": order.reviewed,
        "ship_status": Into::<i16>::into(order.ship_status),
        "extra": order.extra,
        "created_at": order.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "updated_at": order.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            "items": order_items,
        })))
        .json()
    }

    // 保存订单
    pub async fn store(
        Extension(user): Extension<Claims>,
        Json(inner): Json<ReqCreateOrder>,
    ) -> impl IntoResponse {
        let mut ids = HashMap::new();
        match &inner.validate() {
            Ok(()) => (),
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
        }
        if let Some(result) = &inner.products {
            for req in result {
                match req.validate() {
                    Ok(()) => {
                        ids.insert(req.product_id.unwrap(), req.product_sku_id.unwrap());
                    }
                    Err(e) => {
                        return ApiResponse::success_code_data(
                            common::FAIL,
                            Some(json!(format_errors(e))),
                        )
                        .json();
                    }
                }
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

        let result = Orders::create(
            user.id,
            total_money,
            sqlx::types::Json(address),
            inner.remark.unwrap(),
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

    pub async fn delete() -> impl IntoResponse {}
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
                println!("订单详情： {:?}", order);
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
