use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::sync::Arc;

use futures::StreamExt;
use http_body::Body;
use lapin::{
    BasicProperties,
    Channel,
    ExchangeKind, options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    }, types::{AMQPValue, FieldTable},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct DlxOrder {
    pub order_id: i64,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub ext_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrdinaryUser {
    pub id: i64,
    pub name: String,
    pub age: u8,
    pub school: String,
}

/*impl RabbitMQQueue for OrdinaryUser {
    fn default() -> Self where Self: Sized {
        OrdinaryUser{
            id: 0,
            name: "".to_string(),
            age: 0,
            school: "".to_string(),
        }
    }

    fn callback(&self, data: Vec<u8>) {
        let data: Self = serde_json::from_slice(data.as_slice()).unwrap();
        println!("用户信息：{:#?}", data);
    }

    fn to_string(&self) -> String {
        self.to_string()
    }

    fn queue(&self) -> &'static str {
        "user-queue"
    }

    fn exchange(&self) -> &'static str {
        "user-exchange"
    }

    fn router_key(&self) -> &'static str {
        "user-router-key"
    }

    fn expiration(&self) -> usize {
        30000
    }

    async fn init_queue(&self) -> lapin::Result<()> {
        let channel = self.channel().await?;

        channel.queue_declare(
            self.queue(),
            QueueDeclareOptions::default(),
            FieldTable::default(),
        ).await?;

        Ok(())
    }

    async fn init_dlx_queue(&self) -> lapin::Result<()> {
        Ok(())
    }

    async fn produce(&self) -> lapin::Result<()> {
        let channel = self.channel().await?;

        let _queue = channel
            .queue_declare(
                self.queue(),
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        channel.basic_publish(
            self.exchange(),
            self.router_key(),
            BasicPublishOptions::default(),
            self.to_string().as_bytes(),
            BasicProperties::default(),
        ).await?.await?;

        Ok(())
    }

    async fn consume(&self) -> lapin::Result<()> {
        let channel = self.channel().await?;

        let mut consumer = channel.basic_consume(
            self.queue(),
            "my_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        ).await?;


        while let Some(message) = consumer.next().await {
            match message {
                Ok(delivery) => {
                    delivery.ack(BasicAckOptions::default()).await;

                    self.callback(delivery.data)
                }
                Err(e) => {
                    error!(format!( " 消息消费错误: {}", e));
                    break;
                }
            }
        }

        Ok(())
    }
}*/


impl RabbitMQQueue for DlxOrder {
    fn default() -> Self
        where
            Self: Sized,
    {
        DlxOrder{
            order_id: 0,
            created_at: None,
            ext_at: None,
        }
    }
    fn callback(&self, _data: Vec<u8>) {
        let data: Self = serde_json::from_slice(_data.as_slice()).unwrap();
        println!("订单未支付：{:#?}", data);
    }

    fn to_string(&self) -> String {
        self.to_string()
    }

    fn queue(&self) -> &'static str {
        "order-queue"
    }

    fn exchange(&self) -> &'static str {
        "order-exchange"
    }

    fn router_key(&self) -> &'static str {
        "order-router"
    }

    fn expiration(&self) -> usize {
        30000
    }
}

/// MQ 队列管理器
pub struct MQPluginManager {
    plugins: HashMap<&'static str, Arc<Box<dyn RabbitMQQueue>>>,
}

impl MQPluginManager {
    pub(crate) fn new() -> Self {
        MQPluginManager {
            plugins: HashMap::new(),
        }
    }

    // 注册队列
    pub fn register_plugin(&mut self) {
        let dlx_order = Box::new(<DlxOrder as RabbitMQQueue>::default());

        let plugins: [(&'static str, Box<dyn RabbitMQQueue>); 1] = [(dlx_order.queue(), dlx_order)];

        for (key, plugin) in plugins {
            self.add_plugin(key, Arc::new(plugin))
        }
    }

    // 添加队列驱动, auto: 初始化队列和启动消费端
    pub fn add_plugin(&mut self, key: &'static str, plugin: Arc<Box<dyn RabbitMQQueue>>) {
        if self.plugins.contains_key(key) {
            return;
        }
        self.plugins.insert(key, plugin.clone());

        tokio::spawn(async move {
            if plugin.init_queue().await.is_err() {
                error!("init_queue: 队列启动失败");
                return;
            }

            if plugin.init_dlx_queue().await.is_err() {
                error!("init_dlx_queue: 死信队列启动失败");
                return;
            }

            plugin.consume().await;
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

    // 队列名称
    fn queue(&self) -> &'static str {
        "normal-queue"
    }

    // 交换机名称
    fn exchange(&self) -> &'static str {
        "normal-exchange"
    }

    // 路由名称
    fn router_key(&self) -> &'static str {
        "normal-router"
    }

    // 死信队列
    fn dlx_queue(&self) -> &'static str {
        Box::leak(Box::new(format!("dlx-{}", self.queue())))
    }

    // 死信交换机
    fn dlx_exchange(&self) -> &'static str {
        Box::leak(Box::new(format!("dlx-{}", self.exchange())))
    }

    // 死信路由
    fn dlx_router_key(&self) -> &'static str {
        Box::leak(Box::new(format!("dlx-{}", self.router_key())))
    }

    // 单条消息过期时间： 默认30分钟 30 * 60 * 1000
    fn expiration(&self) -> usize {
        1800000
    }

    // 获取 mq channel
    async fn channel(&self) -> lapin::Result<Channel> {
        let rabbit = crate::rabbit_mq().await.clone();

        rabbit.create_channel().await
    }

    // 普通队列
    async fn init_queue(&self) -> lapin::Result<()> {
        let channel = self.channel().await?;

        channel
            .exchange_declare(
                self.exchange(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let queue = channel
            .queue_declare(
                self.queue(),
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
                self.exchange(),
                self.router_key(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(())
    }

    // 死信队列, 不需要时请实现空接口,系统初始化会调用此函数
    async fn init_dlx_queue(&self) -> lapin::Result<()> {
        let channel = self.channel().await?;

        channel
            .exchange_declare(
                self.dlx_exchange(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let dlx_queue = channel
            .queue_declare(
                self.dlx_queue(),
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        channel
            .queue_bind(
                dlx_queue.name().as_str(),
                self.dlx_exchange(),
                self.router_key(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(())
    }

    // 生产者
    async fn produce(&self) -> lapin::Result<()> {
        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_priority(0)
            .with_delivery_mode(2)
            .with_expiration(self.expiration().to_string().into()); // 设置过期时间

        self.channel()
            .await?
            .basic_publish(
                self.exchange(),
                self.router_key(),
                BasicPublishOptions::default(),
                self.to_string().as_bytes(),
                properties,
            )
            .await?;

        Ok(())
    }

    //消费者
    async fn consume(&self) -> lapin::Result<()> {
        let mut consumer = self
            .channel()
            .await
            .map_err(|err| {
                error!("mq 队列 未进行初始化, err: {}", err);
                err
            })?
            .basic_consume(
                self.dlx_queue(),
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        while let Some(message) = consumer.next().await {
            match message {
                Ok(delivery) => {
                    delivery.ack(BasicAckOptions::default()).await;

                    self.callback(delivery.data);
                }
                Err(e) => println!("消费信息错误: {}", e),
            }
        }

        Ok(())
    }
}
