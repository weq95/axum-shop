use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;
use validator::Validate;

use common::error::{format_errors, ApiError};
use common::jwt::Claims;
use common::request::address::ReqAddressInfo;
use common::response::address::{ResAddrResult, ResAddress};
use common::ApiResponse;

use crate::models::address::{addr_result as AddrResult, get_addr_name as AddrName, UserAddress};
use crate::AppState;

pub struct AddressController;

impl AddressController {
    /// 获取用户收获地址详情
    pub async fn get_address(
        Extension(_state): Extension<Arc<AppState>>,
        Path(id): Path<i64>,
    ) -> impl IntoResponse {
        let userid = 1i64;
        let info = UserAddress::get(id, userid).await.unwrap();
        if info.id == 0 {
            return ApiResponse::fail_msg("为获取到用户收获地址信息".to_string()).json();
        }
        let address = AddrName(HashSet::from([
            info.province,
            info.city,
            info.district,
            info.street,
        ]))
        .await
        .unwrap();

        ApiResponse::response(Some(ResAddress {
            id: info.id,
            user_id: info.user_id,
            province: address
                .get(&info.province)
                .map(|val| val.name.clone())
                .take(),
            city: address.get(&info.city).map(|val| val.name.clone()).take(),
            district: address
                .get(&info.district)
                .map(|val| val.name.clone())
                .take(),
            street: address.get(&info.street).map(|val| val.name.clone()).take(),
            address: info.address.clone(),
            zip: info.zip,
            contact_name: info.contact_phone.clone(),
            contact_phone: info.contact_phone.clone(),
            last_used_at: info.last_used_at.to_string(),
        }))
        .json()
    }

    /// 用户收获地址列表
    pub async fn list_address(Extension(user): Extension<Claims>) -> impl IntoResponse {
        let data = UserAddress::list(user.id).await.unwrap();
        let mut result: Vec<ResAddress> = Vec::with_capacity(data.len());

        let mut ids = HashSet::new();
        for i in &data {
            ids.insert(i.province);
            ids.insert(i.city);
            ids.insert(i.district);
            ids.insert(i.street);
        }

        let address = AddrName(ids)
            .await
            .map_err(|e| {
                return e.into_response();
            })
            .unwrap();

        for i in data {
            result.push(ResAddress {
                id: i.id,
                user_id: i.user_id,
                province: address.get(&i.province).map(|val| val.name.clone()).take(),
                city: address.get(&i.city).map(|val| val.name.clone()).take(),
                district: address.get(&i.district).map(|val| val.name.clone()).take(),
                street: address.get(&i.street).map(|val| val.name.clone()).take(),
                address: i.address,
                zip: i.zip,
                contact_name: i.contact_name,
                contact_phone: i.contact_phone,
                last_used_at: i.last_used_at.to_string(),
            })
        }

        ApiResponse::response(Some(result)).json()
    }

    /// 用户创建收获地址
    pub async fn create_address(
        Extension(user): Extension<Claims>,
        Json(inner): Json<ReqAddressInfo>,
    ) -> impl IntoResponse {
        inner
            .validate()
            .map_err(|e| {
                return ApiError::ArrayMap(format_errors(e)).into_response();
            })
            .unwrap();

        UserAddress::create(user.id, inner)
            .await
            .map_err(|e| {
                return e.into_response();
            })
            .map(|id| {
                return ApiResponse::new(Some(json!({ "id": id })));
            })
    }

    /// 用户更新收获地址
    pub async fn update_address(
        Path(id): Path<i64>,
        Extension(user): Extension<Claims>,
        Json(inner): Json<ReqAddressInfo>,
    ) -> impl IntoResponse {
        _ = inner
            .validate()
            .map_err(|e| {
                return ApiError::ArrayMap(format_errors(e)).into_response();
            })
            .unwrap();

        UserAddress::update(id, user.id, inner)
            .await
            .map_err(|e| {
                return e.into_response();
            })
            .map(|bool_val| {
                return ApiResponse::new(Some(json!({ "status": bool_val })));
            })
    }

    /// 用户删除收获地址
    pub async fn delete_address(
        Extension(_state): Extension<Arc<AppState>>,
        Path(id): Path<i64>,
    ) -> impl IntoResponse {
        let userid = 1i64;
        ApiResponse::response(Some(UserAddress::delete(id, userid).await)).json()
    }

    /// 获取收获地址资源
    pub async fn addr_result(
        Extension(_state): Extension<Arc<AppState>>,
        Path(pid): Path<i32>,
    ) -> impl IntoResponse {
        let result = AddrResult(pid).await.unwrap();
        let mut data = Vec::with_capacity(result.len());
        for item in result {
            data.push(ResAddrResult {
                id: item.id,
                name: item.name,
            })
        }
        ApiResponse::response(Some(data)).json()
    }
}
