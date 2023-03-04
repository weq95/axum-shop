use std::ops::Deref;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use validator::Validate;

use common::products::{ReqProduct, ReqQueryProduct};
use common::{ApiResponse, Pagination};

use crate::models::product_skus::ProductSkuModel;
use crate::models::products::ProductModel;

pub struct ProductController;

impl ProductController {
    /// 商品列表
    pub async fn products(Query(payload): Query<ReqQueryProduct>) -> impl IntoResponse {
        match ProductModel::products(payload).await {
            Ok((count, result)) => {
                let mut result = Pagination::new(result);

                result
                    .set_total(count as usize)
                    .set_per_page(15)
                    .set_current_page(1);

                ApiResponse::response(Some(result)).json()
            }
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 商品详情
    pub async fn get(Path(product_id): Path<i64>) -> impl IntoResponse {
        if product_id == 0 {
            return ApiResponse::fail_msg("商品不存在".to_string()).json();
        }

        match ProductModel::get(product_id).await {
            Ok(result) => ApiResponse::response(Some(result)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 创建商品
    pub async fn create(Json(payload): Json<ReqProduct>) -> impl IntoResponse {
        match payload.validate() {
            Ok(_) => (),
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
        }

        let mut skus: Vec<ProductSkuModel> = Vec::new();
        for sku in &payload.skus.unwrap() {
            match sku.validate() {
                Ok(_) => (),
                Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            }

            skus.push(ProductSkuModel {
                title: sku.title.clone().unwrap(),
                description: sku.description.clone().unwrap(),
                price: sku.price.unwrap(),
                stock: sku.stock.unwrap(),
                ..ProductSkuModel::default()
            })
        }

        let result = ProductModel::create(ProductModel {
            title: payload.title.clone().unwrap(),
            description: payload.description.clone().unwrap(),
            image: payload.image.clone().unwrap(),
            on_sale: payload.on_sale.unwrap(),
            skus,
            ..ProductModel::default()
        })
        .await;
        match result {
            Ok(product_id) => ApiResponse::response(Some(json!({ "id": product_id }))).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 更新商品
    pub async fn update(Json(payload): Json<ReqProduct>) -> impl IntoResponse {
        match payload.validate() {
            Ok(_) => (),
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
        }

        let mut skus: Vec<ProductSkuModel> = Vec::new();
        for sku in &payload.skus.unwrap() {
            match sku.validate() {
                Ok(_) => (),
                Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            }

            skus.push(ProductSkuModel {
                title: sku.title.clone().unwrap(),
                description: sku.description.clone().unwrap(),
                price: sku.price.unwrap(),
                stock: sku.stock.unwrap(),
                ..ProductSkuModel::default()
            })
        }

        let result = ProductModel::update(ProductModel {
            id: payload.id.unwrap() as i64,
            title: payload.title.clone().unwrap(),
            description: payload.description.clone().unwrap(),
            image: payload.image.clone().unwrap(),
            on_sale: payload.on_sale.unwrap(),
            skus,
            ..ProductModel::default()
        })
        .await;
        match result {
            Ok(bool_val) => {
                if bool_val {
                    return ApiResponse::response(Some(json!({ "status": bool_val }))).json();
                }

                ApiResponse::fail_msg("更新失败".to_string()).json()
            }
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 删除商品
    pub async fn delete(Json(product_id): Json<u64>) -> impl IntoResponse {
        if product_id == 0 {
            return ApiResponse::fail_msg("没有需要删除的商品".to_string()).json();
        }

        match ProductModel::delete(product_id).await {
            Ok(bool_val) => {
                if bool_val {
                    return ApiResponse::response(Some(json!({ "status": bool_val }))).json();
                }

                ApiResponse::fail_msg("删除失败,请稍后重试".to_string()).json()
            }
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }
}
