use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::Json as SqlxJson;
use validator::Validate;
use validator::ValidationError;

use common::{ApiResponse, Pagination};

use crate::models::favorite_products::FavoriteProductsModel;
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
    pub async fn get(Path((user_id, product_id)): Path<(i64, i64)>) -> impl IntoResponse {
        if product_id == 0 {
            return ApiResponse::fail_msg("商品不存在".to_string()).json();
        }

        let favorite_product =
            match FavoriteProductsModel::favorite_products(user_id, vec![product_id]).await {
                Ok(values) => values,
                Err(e) => return ApiResponse::fail_msg(e.to_string()).json(),
            };

        match ProductModel::get(product_id).await {
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
                "favorite_status":favorite_product.contains(&product_id)
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

        match ProductModel::unique_title(&payload.title.clone().unwrap()).await {
            Ok(bool_val) => {
                if bool_val {
                    return ApiResponse::fail_msg("商品已存在".to_string()).json();
                }
            }
            Err(e) => {
                return ApiResponse::fail_msg(e.to_string()).json();
            }
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

#[derive(Deserialize, Serialize)]
pub struct ReqQueryProduct {
    pub page_num: Option<i32>,
    pub page_size: Option<i32>,
    pub title: Option<String>,
    pub order_by: Option<String>,
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
