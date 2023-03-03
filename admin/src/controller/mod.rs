use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Multipart, Path, Query};
use axum::response::IntoResponse;
use http::StatusCode;
use serde_json::json;
use tokio::io::AsyncWriteExt;

use common::error::{ApiError, ApiResult};
use common::ApiResponse;
pub use user::*;

pub mod address;
pub mod auth;
pub mod product_skus;
pub mod products;
pub mod user;

/// 获取系统配置
pub async fn get_application() -> impl IntoResponse {
    let result = common::application_config().await;

    ApiResponse::response(Some(json!({ "application": result }))).json()
}

/// 文件上传
pub async fn upload_file(multipart: Multipart) -> impl IntoResponse {
    let date = chrono::Local::now().format("%Y-%m").to_string();
    let filepath = "./files/images/".to_owned() + date.as_str() + "/";
    match upload_images(filepath, multipart).await {
        Ok(result) => {
            if let Some(path) = result {
                return ApiResponse::response(Some(json!({ "path": path }))).json();
            }

            ApiResponse::fail_msg("文件上传失败".to_string()).json()
        }
        Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
    }
}

/// 读取图片内容
pub async fn show_image(Query(payload): Query<HashMap<String, String>>) -> impl IntoResponse {
    let filepath = match payload.get("path") {
        Some(path) => path,
        None => {
            return ApiResponse::<Vec<u8>>::set_content_type(None)
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("文件不存在"))
                .unwrap()
                .into_response();
        }
    };

    let payload_arr = &filepath.split(".").collect::<Vec<&str>>();
    let ext_name = payload_arr[payload_arr.len() - 1];
    let content_type = format!("image/{}", ext_name);

    match tokio::fs::read(filepath).await {
        Ok(content) => ApiResponse::<Vec<u8>>::set_content_type(Some(&content_type))
            .body(Body::from(content))
            .unwrap()
            .into_response(),
        Err(_e) => ApiResponse::<Vec<u8>>::set_content_type(None)
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("读取文件失败"))
            .unwrap()
            .into_response(),
    }
}

/// 上传图片
async fn upload_images(filepath: String, mut multipart: Multipart) -> ApiResult<Option<String>> {
    let mut path: Option<String> = None;
    while let Some(mut field) = multipart.next_field().await? {
        let mut content_type = field.content_type().unwrap().to_string();
        tokio::fs::create_dir_all(filepath.clone()).await?;
        if !&content_type.contains("image/") {
            return Err(ApiError::Error("不允许上传此类型文件".to_string()));
        }

        let dst = filepath.to_owned() + field.file_name().unwrap();
        path = Some(dst.clone());
        let mut new_file = tokio::fs::File::create(dst).await?;
        while let Some(chunk) = field.chunk().await? {
            new_file.write_all(&chunk).await?;
        }

        new_file.sync_all().await?;
    }

    Ok(path)
}
