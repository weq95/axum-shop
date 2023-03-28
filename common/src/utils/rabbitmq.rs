use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::sync::Arc;

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
use tracing::log::{error, info};
use tracing_subscriber::util::SubscriberInitExt;

use crate::error::ApiResult;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct DlxOrder {
    pub order_id: i64,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub ext_at: Option<chrono::NaiveDateTime>,
}

impl RabbitMQQueue for DlxOrder {
    fn default() -> Self
    where
        Self: Sized,
    {
        todo!()
    }
    fn callback(&self, _data: Vec<u8>) {
        // let message = String::from_utf8_lossy(&delivery.body);
        // let callback: Box<dyn MQCallBack> = Box::new(MyCallback::from_message(message.as_ref()).unwrap());
        // 调用 callback 的方法进行处理
        println!("订单未支付：{:#?}", self);
    }

    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
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

/// MQ 队列管理器
pub struct MQPluginManager {
    plugins: HashMap<&'static str, &'static Arc<Box<dyn RabbitMQQueue>>>,
}

impl MQPluginManager {
    pub(crate) fn new() -> Self {
        MQPluginManager {
            plugins: HashMap::new(),
        }
    }

    // 获取主驱动
    pub fn get_mq_core(&mut self, queue_name: &str) -> Option<Arc<Box<dyn RabbitMQQueue>>> {
        if let Some(&rabbit) = self.plugins.get(queue_name) {
            return Some(rabbit.clone());
        }

        info!(
            "mq驱动未注册: time: {:?}, name: {}",
            std::time::SystemTime::now(),
            queue_name
        );
        None
    }

    // 添加队列驱动
    pub fn add_plugin(
        &mut self,
        queue_name: &'static str,
        plugin: &'static Arc<Box<dyn RabbitMQQueue>>,
    ) {
        self.plugins.insert(queue_name, plugin);

        tokio::spawn(async move{
            plugin.init();
        });
    }
}

#[axum::async_trait]
pub trait RabbitMQQueue: Send + Sync {
    fn default() -> Self
    where
        Self: Sized;

    // consume 业务消费业务逻辑
    fn callback(&self, data: Vec<u8>);

    fn to_string(&self) -> String;

    fn plugin(&self) -> (&'static str, &'static Box<dyn RabbitMQQueue>) {
        todo!()
    }

    // 初始化队列, 启动消费者
    async fn init(&self) {
        self.init_queue().await.expect("init_queue: panic message");

        self.init_dlx_queue()
            .await
            .expect("start_dlx_queue: panic message");

        self.consume().await;
    }

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

    // 单条消息过期时间： 默认30分钟 30 * 60 * 1000
    fn expiration(&self) -> usize {
        1800000
    }

    // 获取 mq channel
    async fn channel(&self) -> Channel {
        let rabbit = crate::rabbit_mq().await.clone();

        rabbit.create_channel().await.unwrap()
    }

    // 普通队列
    async fn init_queue(&self) -> ApiResult<()> {
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

    // 死信队列, 不需要时请实现空接口,系统初始化会调用此函数
    async fn init_dlx_queue(&self) -> ApiResult<()> {
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
    async fn produce(&self, result: Box<dyn RabbitMQQueue>) -> ApiResult<()> {
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
                result.to_string().as_bytes(),
                properties,
            )
            .await
            .unwrap();

        Ok(())
    }

    //消费者
    async fn consume(&self) {
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

                    self.callback(delivery.data);
                }
                Err(e) => println!("死信队列消费信息错误: {}", e),
            }
        }

        let time_str = chrono::Utc::now()
            .naive_utc()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        error!("消费信息失败：「{}」", time_str);
    }
}
