use std::ops::DerefMut;

use axum::{
    body::Body,
    extract::{Multipart, Path},
    http::Request,
    Json,
    response::IntoResponse,
};
use http::StatusCode;
use serde_json::json;
use tokio::io::AsyncWriteExt;

use common::{ApiResponse, IMAGES_PATH, redis, SchoolJson};
use common::error::{ApiError, ApiResult};
use common::jwt::{Claims, JWT};
pub use user::*;

pub mod address;
pub mod auth;
pub mod order;
pub mod products;
pub mod user;

pub struct CommController;

impl CommController {
    /*// HTTP请求参数提取示例
    // GET 请求: https://example.com/orders/user/:id?page=1&page_per=15
    pub fn example_get(
        axum::extract::Path(user_id): axum::extract::Path<i32>,
        axum::extract::Query(pagination): axum::extract::Query<
            std::collections::HashMap<String, usize>,
        >,
        http_header: axum::http::HeaderMap,
        axum::extract::State(state): axum::extract::State<crate::AppState>,
        request: axum::http::Requestaxum::body::Body, // || Json || From
    ) -> impl IntoResponse {
        // 希望这个示例可以帮助到像我这样的小白
        // 具体参数提取看文档描述: https://docs.rs/axum/0.6.11/axum/extract/index.html
        let user = request.extensions().get::<User>();
        let host = http_header.get("host");
    }

    pub fn example_get(
        axum::extract::Path(user_id): axum::extract::Path<i32>,
        http_header: axum::http::HeaderMap,
        axum::Extension(user): axum::Extension<Claims>,
        Json(inner): Json<SchoolJson>,
    ) -> impl IntoResponse{
        // 以上代码使用了 Json 提取body体, 所以不能在使用 Request<Body>,
        // 提取用户信息跟GET请求就不一样了

    }*/
    
    /// 测试 redis.json 数据接口
    pub async fn test_redis(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
        let key = payload.get("key").unwrap().as_str().unwrap();

        let mut conn = redis::get_conn_manager().await;
        match redis::json_get::<SchoolJson>(conn.deref_mut(), key, "*").await {
            Ok(data) => ApiResponse::response(Some(data)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 刷新token
    pub async fn refresh_token(mut req: Request<Body>) -> impl IntoResponse {
        match req.extensions_mut().get_mut::<Claims>() {
            Some(claims) => match JWT::default().token_info(claims) {
                Ok((access_token, refresh_token)) => ApiResponse::response(Some(json!({
                    "access_token": access_token,
                    "refresh_token":refresh_token,
                })))
                    .json(),
                Err(_) => ApiResponse::fail_msg("refresh_token 刷新失败[02]".to_string()).json(),
            },
            None => ApiResponse::fail_msg("refresh_token 刷新失败[01]".to_string()).json(),
        }
    }

    /// 获取系统配置
    pub async fn get_application() -> impl IntoResponse {
        let result = common::application_config().await;

        ApiResponse::response(Some(json!({ "application": result }))).json()
    }

    /// 文件上传
    pub async fn upload_file(multipart: Multipart) -> impl IntoResponse {
        match Self::upload_images(multipart).await {
            Ok(result) => {
                if let Some((path, preview_url)) = result {
                    return ApiResponse::response(Some(
                        json!({ "path": path, "preview_url":preview_url }),
                    ))
                        .json();
                }

                ApiResponse::fail_msg("文件上传失败".to_string()).json()
            }
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    /// 读取图片内容
    pub async fn show_image(Path(path): Path<String>) -> impl IntoResponse {
        let filepath = common::utils::url_decode(path);
        let payload_arr = &filepath.split(".").collect::<Vec<&str>>();
        let ext_name = payload_arr[payload_arr.len() - 1];
        let content_type = format!("image/{}", ext_name);

        match tokio::fs::read(format!("{}{}", IMAGES_PATH, filepath)).await {
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
    async fn upload_images(mut multipart: Multipart) -> ApiResult<Option<(String, String)>> {
        let date = chrono::Local::now().format("%Y-%m/").to_string();
        let mut path: Option<(String, String)> = None;
        let root_path = common::utils::IMAGES_PATH;
        while let Some(mut field) = multipart.next_field().await? {
            let content_type = field.content_type().unwrap().to_string();
            tokio::fs::create_dir_all(format!("{}{}", root_path, date)).await?;
            if !&content_type.contains("image/") {
                return Err(ApiError::Error("不允许上传此类型文件".to_string()));
            }

            let dst = date.to_owned() + field.file_name().unwrap();
            path = Some(common::utils::image_preview_url(dst.clone()).await);

            let mut new_file = tokio::fs::File::create(format!("{}{}", root_path, dst)).await?;
            while let Some(chunk) = field.chunk().await? {
                new_file.write_all(&chunk).await?;
            }

            new_file.sync_all().await?;
        }

        Ok(path)
    }
}
