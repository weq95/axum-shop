use std::collections::HashMap;
use std::ops::Add;

use axum::response::IntoResponse;
use serde_json::json;
use sqlx::postgres::types::PgMoney;
use validator::{Validate, ValidationErrors};

use common::order::ReqCreateOrder;
use common::{ApiResponse, AppExtractor, Pagination};

use crate::models::address::UserAddress;
use crate::models::order_items::{OrderItems, Sku};
use crate::models::orders::Orders;
use crate::models::product_skus::ProductSku;
use crate::models::products::Product;

pub struct OrderController;

impl OrderController {
    pub async fn index(
        params: AppExtractor<HashMap<String, serde_json::Value>>,
    ) -> impl IntoResponse {
    }

    pub async fn get() -> impl IntoResponse {}

    pub async fn store(params: AppExtractor<ReqCreateOrder>) -> impl IntoResponse {
        let store_len = params.inner.products.unwrap().len();

        let mut ids = HashMap::with_capacity(store_len);
        let total_money = PgMoney::from(0);
        match params.inner.validate() {
            Ok(_) => {
                while let Some(req) = params.inner.products.unwrap().iter().next() {
                    match req.validate() {
                        Ok(_) => {
                            ids.insert(req.product_id.unwrap(), req.product_sku_id.unwrap());
                            total_money.add(PgMoney::from(req.amount.unwrap()));
                        }
                        Err(err) => {
                            return ApiResponse::fail_msg(err.to_string()).json();
                        }
                    }
                }
            }
            Err(err) => return ApiResponse::fail_msg(err.to_string()).json(),
        }

        let values = match ProductSku::products(ids).await {
            Ok(product_val) => product_val,
            Err(err) => return ApiResponse::fail_msg(err.to_string()).json(),
        };

        let address = match UserAddress::harvest_addr(params.inner.address_id.unwrap()).await {
            Ok(addr) => addr,
            Err(_err) => return ApiResponse::fail_msg("收获地址未找到".to_string()).json(),
        };

        let mut order_items: HashMap<i64, _> = HashMap::new();
        for (idx, item) in params.inner.products.unwrap().iter().enumerate() {
            match values.get(&item.product_id.unwrap()) {
                Some(sku) => {
                    if false == sku.sale() {
                        return ApiResponse::fail_msg(format!("第{}项商品未上线", idx + 1)).json();
                    }

                    if sku.stock() <= 0 {
                        return ApiResponse::fail_msg(format!("第{}项商品已售完", idx + 1)).json();
                    }

                    if sku.stock() < item.amount.unwrap() {
                        return ApiResponse::fail_msg(format!("第{}项商品库存不足", idx + 1))
                            .json();
                    }

                    order_items.insert(
                        sku.p_id(),
                        OrderItems::generate_sku(
                            sku.id(),
                            sku.price(),
                            item.amount.unwrap() as i16,
                            sku.title(),
                            sku.descr(),
                        )
                        .await,
                    );
                }
                None => return ApiResponse::fail_msg(format!("第{}项商品不存在", idx + 1)).json(),
            }
        }

        let result = Orders::create(
            params.claims.id,
            sqlx::types::Json(address),
            total_money,
            params.inner.remark.unwrap(),
            order_items,
        )
        .await;
        match result {
            Ok(order_id) => ApiResponse::response(Some(json!({ "id": order_id }))).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    pub async fn update() -> impl IntoResponse {}

    pub async fn delete() -> impl IntoResponse {}
}
