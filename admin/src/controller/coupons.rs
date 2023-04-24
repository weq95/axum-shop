use std::collections::HashMap;

use axum::extract::{Json, Path, Query};
use axum::response::IntoResponse;
use serde_json::json;
use validator::Validate;

use common::coupon::ReqCoupon;
use common::{ApiResponse, PagePer, Pagination};

use crate::models::coupons::{CouponType, Coupons};

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
    pub async fn get(Path(id): Path<i64>) -> impl IntoResponse {
        match Coupons::get(id).await {
            Ok(coupon) => ApiResponse::response(Some(json!({
                "id": coupon.id,
                "name": coupon.name,
                "code": coupon.code,
                "type": coupon.r#type,
                "value": coupon.value,
                "total": coupon.total,
                "min_amount": coupon.min_amount.0,
                "not_before": coupon.not_before,
                "not_after": coupon.not_after,
                "enabled": coupon.enabled,
            })))
            .json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 创建
    pub async fn create(Json(inner): Json<ReqCoupon>) -> impl IntoResponse {
        if let Err(e) = inner.validate() {
            return ApiResponse::success_code_data(
                common::FAIL,
                Some(json!(common::error::format_errors(e))),
            )
            .json();
        }

        if let Some(value) = inner.value {
            let r#type = inner.r#type.unwrap();
            if r#type == <CouponType as Into<i16>>::into(CouponType::Fixed) {
                if value < 0.01f64 {
                    return ApiResponse::fail_msg("免费金额必须 >= 0.01 元".to_string()).json();
                }
            } else if r#type == <CouponType as Into<i16>>::into(CouponType::Percent) {
                if value <= 0f64 || value > 100f64 {
                    return ApiResponse::fail_msg("优惠比例必须在 1%-100% 之间".to_string()).json();
                }
            } else {
                return ApiResponse::fail_msg("优惠券类型不正确".to_string()).json();
            }
        }

        let before = if let Some(start) = inner.not_before {
            chrono::NaiveDateTime::from_timestamp_millis(start.timestamp_millis())
        } else {
            None
        };
        let after = if let Some(end) = inner.not_after {
            chrono::NaiveDateTime::from_timestamp_millis(end.timestamp_millis())
        } else {
            None
        };
        match Coupons::store(
            inner.name.clone().unwrap(),
            inner.r#type.unwrap(),
            inner.value.unwrap(),
            inner.total.unwrap(),
            (inner.min_amount.unwrap() * 100.0) as i64,
            before,
            after,
            inner.enable.unwrap(),
        )
        .await
        {
            Ok(id) => ApiResponse::response(Some(json!({ "id": id }))).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 更新
    pub async fn update(Path(id): Path<i64>, Json(inner): Json<ReqCoupon>) -> impl IntoResponse {
        if let Err(e) = inner.validate() {
            return ApiResponse::success_code_data(
                common::FAIL,
                Some(json!(common::error::format_errors(e))),
            )
            .json();
        }

        if inner.code.is_none() {
            return ApiResponse::fail_msg("参数错误, card not found".to_string()).json();
        }

        if let Some(value) = inner.value {
            let r#type = inner.r#type.unwrap();
            if r#type == <CouponType as Into<i16>>::into(CouponType::Fixed) {
                if value < 0.01f64 {
                    return ApiResponse::fail_msg("免费金额必须 >= 0.01 元".to_string()).json();
                }
            } else if r#type == <CouponType as Into<i16>>::into(CouponType::Percent) {
                if value <= 0f64 || value > 100f64 {
                    return ApiResponse::fail_msg("优惠比例必须在 1%-100% 之间".to_string()).json();
                }
            } else {
                return ApiResponse::fail_msg("优惠券类型不正确".to_string()).json();
            }
        }

        let min_amount = (inner.min_amount.unwrap() * 100.0) as i64;

        let before = if let Some(start) = inner.not_before {
            chrono::NaiveDateTime::from_timestamp_millis(start.timestamp_millis())
        } else {
            None
        };
        let after = if let Some(end) = inner.not_after {
            chrono::NaiveDateTime::from_timestamp_millis(end.timestamp_millis())
        } else {
            None
        };
        match Coupons::update(
            id,
            inner.name.clone().unwrap(),
            inner.code.unwrap(),
            inner.r#type.unwrap(),
            inner.value.unwrap(),
            inner.total.unwrap(),
            min_amount,
            before,
            after,
            inner.enable.unwrap(),
        )
        .await
        {
            Ok(bool_value) => ApiResponse::response(Some(json!({
                "status": bool_value,
            })))
            .json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    // 删除
    pub async fn delete(Path(id): Path<i64>) -> impl IntoResponse {
        match Coupons::delete(id).await {
            Ok(bool_val) => ApiResponse::response(Some(json!({
                "bool_val": bool_val,
            })))
            .json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }
}
