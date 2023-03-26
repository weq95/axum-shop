use std::collections::BTreeMap;
use std::fmt::Debug;

use futures::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::{AMQPValue, FieldTable},
    BasicProperties, Channel, ExchangeKind,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::log::error;

use crate::error::ApiResult;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct DlxOrder {
    pub order_id: i64,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub ext_at: Option<chrono::NaiveDateTime>,
}

impl MQCallBack for DlxOrder {
    fn callback(&mut self) {
        println!("订单未支付：{:#?}", self);
    }

    fn queue(&self) -> String {
        "order-queue".to_string()
    }

    fn exchange(&self) -> String {
        "order-exchange".to_string()
    }

    fn router_key(&self) -> String {
        "order-router".to_string()
    }

    fn expiration(&self) -> usize {
        30000
    }
}

impl RabbitMQDeadQueue for DlxOrder {
    type Output = Self;

    fn new() -> Self::Output {
        DlxOrder {
            order_id: 0,
            created_at: None,
            ext_at: None,
        }
    }
}

#[axum::async_trait]
pub trait MQCallBack {
    // 超时业务逻辑
    fn callback(&mut self);

    // 队列名称
    fn queue(&self) -> String {
        "normal-queue".to_string()
    }

    // 交换机名称
    fn exchange(&self) -> String {
        "normal-exchange".to_string()
    }

    // 路由名称
    fn router_key(&self) -> String {
        "normal-router".to_string()
    }

    // 死信队列
    fn dlx_queue(&self) -> String {
        format!("dlx-{}", self.queue().clone())
    }

    // 死信交换机
    fn dlx_exchange(&self) -> String {
        format!("dlx-{}", self.exchange().clone())
    }

    // 死信路由
    fn dlx_router_key(&self) -> String {
        format!("dlx-{}", self.router_key().clone())
    }

    // 过期时间： 默认30分钟 30 * 60 * 1000
    fn expiration(&self) -> usize {
        1800000
    }
}

#[axum::async_trait]
pub trait RabbitMQDeadQueue:
    MQCallBack + DeserializeOwned + Serialize + Send + Sync + 'static
{
    type Output;

    fn new() -> Self::Output;

    async fn init(mut self) {
        self.init_normal_queue()
            .await
            .expect("init_normal_queue: panic message");
        self.init_dead_queue()
            .await
            .expect("init_dead_queue: panic message");

        tokio::spawn(async move {
            self.consume().await.expect("consume: panic message");
        });
    }

    async fn self_data(&self) -> String {
        serde_json::to_string(&self.clone()).unwrap()
    }

    async fn channel(&self) -> Channel {
        crate::rabbit_mq().await.create_channel().await.unwrap()
    }

    // 普通队列
    async fn init_normal_queue(&self) -> ApiResult<()> {
        let channel = self.channel().await;
        channel
            .exchange_declare(
                self.exchange().as_str(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let queue = channel
            .queue_declare(
                self.queue().as_str(),
                QueueDeclareOptions::default(),
                FieldTable::from(BTreeMap::from([
                    // 队列默认超时时间： 30分钟
                    ("x-message-ttl".into(), AMQPValue::LongUInt(30 * 60 * 1000)),
                    (
                        "x-dead-letter-exchange".into(),
                        AMQPValue::LongString(self.dlx_exchange().into()),
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
                self.exchange().as_str(),
                self.router_key().as_str(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(())
    }

    // 死信队列
    async fn init_dead_queue(&self) -> ApiResult<()> {
        let channel = self.channel().await;

        channel
            .exchange_declare(
                self.dlx_exchange().as_str(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let dlx_queue = channel
            .queue_declare(
                self.dlx_queue().as_str(),
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        channel
            .queue_bind(
                dlx_queue.name().as_str(),
                self.dlx_exchange().as_str(),
                self.router_key().as_str(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(())
    }

    // 生产者
    async fn produce(&self) -> ApiResult<()> {
        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_priority(0)
            .with_delivery_mode(2)
            .with_expiration(self.expiration().to_string().into()); // 设置过期时间

        self.channel()
            .await
            .basic_publish(
                self.exchange().as_str(),
                self.router_key().as_str(),
                BasicPublishOptions::default(),
                self.self_data().await.as_bytes(),
                properties,
            )
            .await
            .unwrap();

        Ok(())
    }

    //消费者
    async fn consume(&mut self) -> ApiResult<()> {
        let mut consumer = self
            .channel()
            .await
            .basic_consume(
                self.dlx_queue().as_str(),
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        while let Some(message) = consumer.next().await {
            match message {
                Ok(delivery) => {
                    delivery.ack(BasicAckOptions::default()).await.unwrap();
                    let mut payload: Self =
                        serde_json::from_str(&String::from_utf8(delivery.data).unwrap()).unwrap();
                    payload.callback();
                }
                Err(e) => println!("死信队列消费信息错误: {}", e),
            }
        }

        let time_str = chrono::Utc::now()
            .naive_utc()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        error!("消费信息失败：「{}」", time_str);

        Ok(())
    }
}
