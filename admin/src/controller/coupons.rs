use std::collections::HashMap;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

use common::coupon::ReqCoupon;
use common::jwt::Claims;
use common::{ApiResponse, PagePer, Pagination};

use crate::models::coupons::Coupons;

pub struct CouponController;

impl CouponController {
    // 列表
    pub async fn index(
        Query(page_per): Query<PagePer>,
        Query(inner): Query<HashMap<String, serde_json::Value>>,
    ) -> impl IntoResponse {
        let mut pagination = Pagination::new(vec![], page_per);
        match Coupons::index(inner, &mut pagination).await {
            Ok(()) => ApiResponse::response(Some(pagination)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 详情
    pub async fn get() -> impl IntoResponse {
        todo!()
    }

    // 创建
    pub async fn store(Json(inner): Json<ReqCoupon>) -> impl IntoResponse {
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
