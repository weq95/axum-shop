use std::collections::HashSet;
use std::sync::Arc;

use axum::{Extension, Json};
use axum::extract::Path;
use axum::response::IntoResponse;

use common::ApiResponse;
use common::request::address::ReqAddressInfo;
use common::response::address::{ResAddress, ResAddrResult};

use crate::AppState;
use crate::models::address::{addr_result as ModelAddrResult, create as ModelCreate, delete as ModelDelete, get as ModelGet, get_addr_name as ModelAddrName, list as ModelList, update as ModelUpdate};

/// 获取用户收获地址详情
pub async fn get_address(Extension(_state): Extension<Arc<AppState>>, Path(id): Path<i64>) -> impl IntoResponse {
    let userid = 1i64;
    let info = ModelGet(id, userid).await.unwrap();
    if info.id == 0 {
        return ApiResponse::fail_msg("为获取到用户收获地址信息".to_string()).json();
    }
    let address = ModelAddrName(HashSet::from(
        [info.province, info.city, info.district, info.street])).await.unwrap();

    ApiResponse::response(Some(ResAddress {
        id: info.id,
        user_id: info.user_id,
        province: address.get(&info.province).map(|val| val.name.clone()).take(),
        city: address.get(&info.city).map(|val| val.name.clone()).take(),
        district: address.get(&info.district).map(|val| val.name.clone()).take(),
        street: address.get(&info.street).map(|val| val.name.clone()).take(),
        address: info.address.clone(),
        zip: info.zip,
        contact_name: info.contact_phone.clone(),
        contact_phone: info.contact_phone.clone(),
        last_used_at: info.last_used_at.to_string(),
    })).json()
}

/// 用户收获地址列表
pub async fn list_address(Extension(_state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let userid = 1i64;
    let data = ModelList( userid).await.unwrap();
    let mut result: Vec<ResAddress> = Vec::with_capacity(data.len());


    let mut ids = HashSet::new();
    for i in &data {
        ids.insert(i.province);
        ids.insert(i.city);
        ids.insert(i.district);
        ids.insert(i.street);
    }
    let address = ModelAddrName(ids).await.unwrap();

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
pub async fn create_address(Extension(_state): Extension<Arc<AppState>>, Json(info): Json<ReqAddressInfo>) -> impl IntoResponse {
    let userid = 1i64;
    ApiResponse::response(Some(ModelCreate(userid, info).await)).json()
}

/// 用户更新收获地址
pub async fn update_address(Extension(_state): Extension<Arc<AppState>>, Path(id): Path<i64>, Json(info): Json<ReqAddressInfo>) -> impl IntoResponse {
    let userid = 1i64;
    ApiResponse::response(Some(ModelUpdate( id, userid, info).await)).json()
}

/// 用户删除收获地址
pub async fn delete_address(Extension(_state): Extension<Arc<AppState>>, Path(id): Path<i64>) -> impl IntoResponse {
    let userid = 1i64;
    ApiResponse::response(Some(ModelDelete(id, userid).await)).json()
}

/// 获取收获地址资源
pub async fn addr_result(Extension(_state): Extension<Arc<AppState>>, Path(pid): Path<i32>) -> impl IntoResponse {
    let result = ModelAddrResult( pid).await.unwrap();
    let mut data = Vec::with_capacity(result.len());
    for item in result {
        data.push(ResAddrResult {
            id: item.id,
            name: item.name,
        })
    }
    ApiResponse::response(Some(data)).json()
}