use axum::extract::Multipart;
use axum::response::IntoResponse;

use common::ApiResponse;
use common::error::{ApiError, ApiResult};
pub use user::*;

pub mod address;
pub mod auth;
pub mod product_skus;
pub mod products;
pub mod user;

pub async fn upload_file(multipart: Multipart) -> impl IntoResponse {
    upload_images(multipart).await;
    ApiResponse::<i32>::response(None).json()
}

async fn upload_images(mut multipart: Multipart) -> ApiResult<String> {
    while let Some(mut field) = multipart.next_field().await? {
        let content_type = field.content_type().unwrap().to_string();
        let random_number = (random::<f32>()* 1000000000 as f32) as i32;
        if !&content_type.contains("image/") {
            return Err(ApiError::Error("不允许上传次类型文件".to_string()));
        }



        println!("content_type: {}", content_type);
        while let Some(chunk) = field.chunk().await? {
            println!("received {} bytes", chunk.len());
        }
    }


    println!("success !!!");
    Ok("".to_string())
}