use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_once::AsyncOnce;
use axum::{Extension, Json};
use axum::response::IntoResponse;
use futures::StreamExt;
use lapin::{BasicProperties, ExchangeKind};
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
    QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::{AMQPValue, FieldTable};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use common::jwt::Claims;
use common::rabbitmq::MQManager;
use common::utils::rabbitmq::{RabbitMQDlxQueue, RabbitMQQueue};

lazy_static! {
    pub static ref MQMANAGER: AsyncOnce<Arc<MQManager>> = AsyncOnce::new(async {
        let mut mq_mamnger = MQManager::new();

        mq_mamnger
            .add_dlx_queue(Arc::new(Box::new(DlxCommQueue::default())))
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

/*impl RabbitMQQueue for CommQueue {
    async fn callback(&self, data: Vec<u8>) {
        info!("CommQueue callback: {:?}", data);
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    async fn init_queue(&self) -> lapin::Result<()> {
        let channel = self.channel().await?;

        let _queue = channel.queue_declare(
            self.queue_name(),
            QueueDeclareOptions::default(),
            FieldTable::default(),
        ).await;

        Ok(())
    }

    fn queue_name(&self) -> &'static str {
        "comm-queue"
    }

    fn exchange_name(&self) -> &'static str {
        "comm-exchange"
    }

    fn router_key(&self) -> &'static str {
        "comm-router-key"
    }
}*/


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
impl RabbitMQDlxQueue for DlxCommQueue {}

#[axum::async_trait]
impl RabbitMQQueue for DlxCommQueue {
    async fn callback(&self, data: Vec<u8>) {
        info!("DlxCommQueue callback: {:?}", &data);
        match serde_json::from_slice::<Self>(data.as_slice()) {
            Ok(result) => {
                info!("callback success, result: {:?}", result);
            }
            Err(e) => {
                error!("DlxCommQueue callback 数据解析错误: {}", e);
            }
        }
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    async fn init_queue(&self) -> lapin::Result<()> {
        let channel = self.channel().await?;

        channel
            .exchange_declare(
                self.exchange_name(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let queue = channel
            .queue_declare(
                self.queue_name(),
                QueueDeclareOptions::default(),
                FieldTable::from(BTreeMap::from([
                    // 队列默认超时时间： 30分钟
                    ("x-message-ttl".into(), AMQPValue::LongUInt(30 * 60 * 1000)),
                    (
                        "x-dead-letter-exchange".into(),
                        AMQPValue::LongString(self.dlx_exchange_name().into()),
                    ),
                    (
                        "x-dead-letter-routing-key".into(),
                        AMQPValue::LongString(self.router_key().into()),
                    ),
                ])),
            )
            .await?;

        channel
            .queue_bind(
                queue.name().as_str(),
                self.exchange_name(),
                self.router_key(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(())
    }
}