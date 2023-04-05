use std::collections::BTreeMap;
use std::ops::DerefMut;
use std::sync::Arc;

use async_once::AsyncOnce;
use axum::{
    body::Body,
    extract::{Multipart, Path},
    http::Request,
    response::IntoResponse,
    Json,
};
use http::StatusCode;
use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable},
    Channel, ExchangeKind,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

use common::error::{ApiError, ApiResult};
use common::jwt::{Claims, JWT};
use common::rabbitmq::{MQManager, RabbitMQDlxQueue, RabbitMQQueue};
use common::{redis, ApiResponse, SchoolJson, IMAGES_PATH};

pub mod address;
pub mod auth;
pub mod order;
pub mod products;
pub mod user;

pub struct CommController;

impl CommController {
    /*// HTTP请求参数提取示例
    // 具体参数提取看文档描述: https://docs.rs/axum/0.6.11/axum/extract/index.html
    // GET 请求: https://example.com/orders/user/:id?page=1&page_per=15
    pub fn example_get(
        axum::extract::Path(user_id): axum::extract::Path<i32>,
        axum::extract::Query(pagination): axum::extract::Query<
            std::collections::HashMap<String, usize>,
        >,
        http_header: axum::http::HeaderMap,
        axum::extract::State(state): axum::extract::State<crate::AppState>,
        request: axum::http::Request<axum::body::Body>, // || Json || From
    ) -> impl IntoResponse {
        let user = request.extensions().get::<Claims>();
        let host = http_header.get("host");
        todo!()
    }

    // POST 请求
    pub fn example_post(
        axum::extract::Path(user_id): axum::extract::Path<i32>,
        http_header: axum::http::HeaderMap,
        axum::Extension(user): axum::Extension<Claims>,
        Json(inner): Json<SchoolJson>,
    ) -> impl IntoResponse {
        // 以上代码使用了 Json 提取body体, 所以不能在使用 Request<Body>,
        // 提取用户信息跟GET请求就不一样了
        todo!()
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

    // 测试队列
    pub async fn test_mq(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
        let rabbit_normal = CommQueue {
            r#type: 255,
            data: payload.clone(),
            crated_at: Some(chrono::Local::now().naive_local()),
        };

        if let Err(e) = rabbit_normal.produce(0).await {
            return ApiResponse::fail_msg(e.to_string()).json();
        }

        let rabbit_dlx = DlxCommQueue {
            r#type: 255,
            data: payload,
            crated_at: Some(chrono::Local::now().naive_local()),
        };
        if let Err(e) = rabbit_dlx.produce(30000).await {
            return ApiResponse::fail_msg(e.to_string()).json();
        }

        ApiResponse::response(Some(json!({
            "queue": true,
            "dlx-queue": true,
        })))
        .json()
    }
}

lazy_static! {
    pub static ref MQMANAGER: AsyncOnce<Arc<MQManager>> = AsyncOnce::new(async {
        let mut mq_mamnger = MQManager::new();

        mq_mamnger
            .add_dlx_queue(Arc::new(Box::new(DlxCommQueue::default())))
            .await;

        mq_mamnger
            .add_normal_queue(Arc::new(Box::new(CommQueue::default())))
            .await;

        Arc::new(mq_mamnger)
    });
}

/// 通用普通队列
#[derive(Debug, Serialize, Deserialize)]
pub struct CommQueue {
    pub r#type: u8,
    pub data: serde_json::Value,
    pub crated_at: Option<chrono::NaiveDateTime>,
}

impl Default for CommQueue {
    fn default() -> Self {
        CommQueue {
            r#type: 255,
            data: Default::default(),
            crated_at: Some(chrono::Local::now().naive_local()),
        }
    }
}

#[axum::async_trait]
impl RabbitMQQueue for CommQueue {
    async fn callback(&self, data: Vec<u8>) {
        match serde_json::from_slice::<Self>(data.as_slice()) {
            Ok(result) => {
                info!("CommQueue: {:?}", result);
            }
            Err(e) => {
                error!("DlxCommQueue callback 数据解析错误: {}", e);
            }
        }
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn queue_name(&self) -> &'static str {
        "normal-queue"
    }

    fn router_key(&self) -> &'static str {
       "normal-queue-router-key"
    }

    fn exchange_name(&self) -> &'static str {
       "normal-queue-exchange"
    }

    fn consumer_tag(&self) -> &'static str {
        "my_consumer"
    }
}

/// 通用死信队列
#[derive(Debug, Serialize, Deserialize)]
pub struct DlxCommQueue {
    pub r#type: u8,
    pub data: serde_json::Value,
    pub crated_at: Option<chrono::NaiveDateTime>,
}

impl Default for DlxCommQueue {
    fn default() -> Self {
        DlxCommQueue {
            r#type: 255,
            data: Default::default(),
            crated_at: Some(chrono::Local::now().naive_local()),
        }
    }
}

#[axum::async_trait]
impl RabbitMQQueue for DlxCommQueue {
    async fn callback(&self, data: Vec<u8>) {
        match serde_json::from_slice::<Self>(data.as_slice()) {
            Ok(result) => {
                info!("DlxCommQueue: {:?}", result);
            }
            Err(e) => {
                error!("DlxCommQueue callback 数据解析错误: {}", e);
            }
        }
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn queue_name(&self) -> &'static str {
        "normal-order-queue"
    }

    fn exchange_name(&self) -> &'static str {
        "normal-order-exchange"
    }

    fn router_key(&self) -> &'static str {
        "normal-order-router"
    }
}

#[axum::async_trait]
impl RabbitMQDlxQueue for DlxCommQueue {}
