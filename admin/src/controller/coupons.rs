use std::collections::HashMap;
use axum::Extension;
use axum::extract::Query;
use axum::response::IntoResponse;
use serde_json::json;

use common::{ApiResponse, PagePer, Pagination};
use common::jwt::Claims;
use crate::models::coupons::Coupons;

pub struct CouponController;

impl CouponController {
    // 列表
    pub async fn index(
        Query(page_per): Query<PagePer>,
        Extension(user): Extension<Claims>,
        Query(inner): Query<HashMap<String, serde_json::Value>>
    )  -> impl IntoResponse {
        let mut pagination = Pagination::new(vec![], page_per);
        match Coupons:: { }
        ApiResponse::response(Some(json!({
            "id": 1,
            "name": "",
            "code": "",
            "type": "",
            "value": "",
            "min_amount": "",
            "total":100,
            "used":12,
            "enable": false,
            "created_at": "",
        }))).json()
    }

    // 详情
    pub async fn get() -> impl IntoResponse {
        todo!()
    }

    // 创建
    pub async fn store() -> impl IntoResponse {
        todo!()
    }

    // 更新
    pub async fn update() -> impl IntoResponse {
        todo!()
    }

    // 删除
    pub async fn delete() -> impl IntoResponse {
        todo!()
    }
}