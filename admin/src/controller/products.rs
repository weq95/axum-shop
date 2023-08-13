use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::types::PgMoney;
use sqlx::types::Json as SqlxJson;
use validator::Validate;
use validator::ValidationError;

use common::{ApiResponse, PagePer, Pagination};

use crate::models::cart_items::CartItems;
use crate::models::categories::Categories;
use crate::models::favorite_products::FavoriteProducts;
use crate::models::product_skus::ProductSku;
use crate::models::products::Product;

pub struct ProductController;

impl ProductController {
    /// 商品列表
    pub async fn products(
        Query(page_per): Query<PagePer>,
        Query(payload): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let mut pagination: Pagination<Product> = Pagination::new(vec![], page_per);

        match Product::products(payload, &mut pagination).await {
            Ok(()) => ApiResponse::response(Some(pagination)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 商品详情
    pub async fn get(Path((product_id, user_id)): Path<(i64, i64)>) -> impl IntoResponse {
        if product_id == 0 {
            return ApiResponse::fail_msg("商品不存在".to_string()).json();
        }

        let favorite_product =
            match FavoriteProducts::favorite_products(user_id, vec![product_id]).await {
                Ok(values) => values,
                Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            };

        let cart_item = match CartItems::product_exists(user_id, vec![product_id]).await {
            Ok(cart_item) => cart_item,
            Err(err) => return ApiResponse::fail_msg(err.to_string()).json(),
        };
        match Product::get(product_id).await {
            Ok(result) => ApiResponse::response(Some(json!({
                "id": result.id,
                "name": result.title,
                "description": result.description,
                "image":result.image,
                "on_sale": result.on_sale,
                "rating": result.rating,
                "sold_count": result.sold_count,
                "review_count":result.review_count,
                "price": result.price,
                "skus": result.skus,
                "cart_status": cart_item.contains(&product_id),
                "favorite_status":favorite_product.contains(&product_id),
            })))
            .json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 创建商品
    pub async fn create(Json(payload): Json<ReqProduct>) -> impl IntoResponse {
        match payload.validate() {
            Ok(_) => (),
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
        }

        let mut skus: Vec<ProductSku> = Vec::new();
        for sku in &payload.skus.unwrap() {
            match sku.validate() {
                Ok(_) => (),
                Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            }

            skus.push(ProductSku {
                title: sku.title.clone().unwrap(),
                description: sku.description.clone().unwrap(),
                price: sku.price.unwrap(),
                stock: sku.stock.unwrap(),
                ..ProductSku::default()
            })
        }

        match Categories::exits(payload.category_id.unwrap()).await {
            Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            Ok(bool_val) => {
                if !bool_val {
                    return ApiResponse::fail_msg("商品类目不存在".to_string()).json();
                }
            }
        }

        match Product::unique_title(&payload.title.clone().unwrap()).await {
            Ok(bool_val) => {
                if bool_val {
                    return ApiResponse::fail_msg("商品已存在".to_string()).json();
                }
            }
            Err(e) => {
                return ApiResponse::fail_msg(e.to_string()).json();
            }
        }

        let result = Product::create(
            Product {
                title: payload.title.clone().unwrap(),
                description: payload.description.clone().unwrap(),
                image: payload.image.clone().unwrap(),
                on_sale: payload.on_sale.unwrap(),
                skus,
                category_id: payload.category_id.unwrap(),
                ..Product::default()
            },
            PgMoney::from(payload.target_amount.unwrap()),
            payload.end_at.unwrap(),
        )
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

        let mut skus: Vec<ProductSku> = Vec::new();
        for sku in &payload.skus.unwrap() {
            match sku.validate() {
                Ok(_) => (),
                Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            }

            skus.push(ProductSku {
                title: sku.title.clone().unwrap(),
                description: sku.description.clone().unwrap(),
                price: sku.price.unwrap(),
                stock: sku.stock.unwrap(),
                ..ProductSku::default()
            })
        }

        let result = Product::update(Product {
            id: payload.id.unwrap() as i64,
            title: payload.title.clone().unwrap(),
            description: payload.description.clone().unwrap(),
            image: payload.image.clone().unwrap(),
            on_sale: payload.on_sale.unwrap(),
            skus,
            ..Product::default()
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

        match Product::delete(product_id).await {
            Ok(bool_val) => {
                if bool_val {
                    return ApiResponse::response(Some(json!({ "status": bool_val }))).json();
                }

                ApiResponse::fail_msg("删除失败,请稍后重试".to_string()).json()
            }
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 收藏商品
    pub async fn favorite_product(
        Path((product_id, user_id)): Path<(u64, u64)>,
    ) -> impl IntoResponse {
        match FavoriteProducts::favorite(user_id as i64, product_id as i64).await {
            Ok(favorite_id) => {
                ApiResponse::response(Some(json!({ "favorite_id": favorite_id }))).json()
            }
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 取消收藏
    pub async fn un_favorite_product(
        Path((product_id, user_id)): Path<(u64, u64)>,
    ) -> impl IntoResponse {
        match FavoriteProducts::un_favorite(user_id as i64, product_id as i64).await {
            Ok(un_rows) => ApiResponse::response(Some(json!({ "un_rows": un_rows }))).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ReqProduct {
    pub id: Option<u64>,
    #[validate(length(min = 3, max = 100), custom = "unique_title")]
    pub title: Option<String>,
    #[validate(required)]
    pub image: Option<SqlxJson<Vec<String>>>,
    #[validate(required)]
    pub description: Option<String>,
    #[validate(required)]
    pub on_sale: Option<bool>,
    #[validate(required)]
    pub skus: Option<Vec<ReqProductSku>>,
    #[validate(range(min = 1, message = "请选择类目"))]
    pub category_id: Option<i64>,
    #[validate(required(message = "请选择类型"))]
    pub r#type: Option<u8>,
    pub target_amount: Option<i64>,
    pub end_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ReqProductSku {
    #[validate(length(max = 100))]
    pub title: Option<String>,
    #[validate(required)]
    pub description: Option<String>,
    #[validate(range(min = 0.01f64))]
    pub price: Option<f64>,
    #[validate(range(min = 1))]
    pub stock: Option<i32>,
}

/// 检测商品是否已存在
fn unique_title(_title: &str) -> Result<(), ValidationError> {
    // 由于不能直接执行async函数，下边的代码使用方式也不正确，所有这里返回true
    return Ok(());
    /* let mut rt = tokio::runtime::Runtime::new().unwrap();
     let bool_val = rt.block_on(async {
         // 调用异步函数
         ProductModel::unique_title(_title).await
     });

     if let Ok(bool_val) = bool_val {
         if false == bool_val {
             return Ok(());
         }
     }

    Err(ValidationError::new("商品名称已存在"))*/
}
