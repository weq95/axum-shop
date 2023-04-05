use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use futures::StreamExt;
use lapin::{
    BasicProperties,
    Channel,
    ExchangeKind, options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    }, types::FieldTable,
};
use lapin::types::AMQPValue;
use tracing::{error, info};

/// 队列管理器
pub struct MQManager {
    pub queue: HashMap<&'static str, Arc<Box<dyn RabbitMQQueue>>>,
    pub dlx_queue: HashMap<&'static str, Arc<Box<dyn RabbitMQDlxQueue>>>,
}

impl MQManager {
    pub fn new() -> Self {
        MQManager {
            queue: HashMap::new(),
            dlx_queue: HashMap::new(),
        }
    }

    // 普通队列
    pub async fn add_normal_queue(&mut self, plugin: Arc<Box<dyn RabbitMQQueue>>) {
        let queue_name = plugin.queue_name();
        if self.queue.contains_key(queue_name) {
            return;
        }

        self.queue.insert(queue_name, plugin.clone());
        let channel = plugin.channel().await.unwrap();
        info!("{}: [normal]队列开始启动", queue_name);
        if let Err(e) = plugin.init_queue(channel).await {
            error!(
                "{}",
                format!(" {} [normal]队列初始化失败: {}", queue_name, e)
            );
            return;
        }

        tokio::spawn(async move {
            info!("{}: [normal]开始启动消费者", queue_name);
            if let Err(e) = plugin.consume(queue_name).await {
                error!(
                    "{}",
                    format!(" {} [normal]消费者启动失败: {}", queue_name, e)
                );
                return;
            }
            info!("{}: [normal]消费者已成功启动", queue_name);
        });

        info!("{}: [normal]队列成功启动", queue_name);
    }

    // 死信队列
    pub async fn add_dlx_queue(&mut self, plugin: Arc<Box<dyn RabbitMQDlxQueue>>) {
        let queue_name = plugin.queue_name();
        if self.dlx_queue.contains_key(queue_name) {
            return;
        }

        self.dlx_queue.insert(queue_name, plugin.clone());

        info!("{}: [dlx]死信队列开始启动", queue_name);
        let channel = plugin.channel().await.unwrap();
        if let Err(e) = plugin.init_dlx_queue(channel).await {
            error!(
                "{}",
                format!(" {} [dlx]队列初始化失败: {}", plugin.dlx_queue_name(), e)
            );
            return;
        }

        tokio::spawn(async move {
            info!("{}: [dlx]消费者开始启动", queue_name);
            if let Err(e) = plugin.consume(plugin.dlx_queue_name()).await {
                error!("{}", format!(" {} [dlx]消费者启动失败: {}", queue_name, e));
                return;
            }
            info!("{}: [dlx]消费者启动成功", queue_name);
        });

        info!("{}: [dlx]队列启动成功", queue_name);
    }
}

/// 普通队列
#[axum::async_trait]
pub trait RabbitMQQueue: Send + Sync {
    // 回调函数
    async fn callback(&self, data: Vec<u8>);

    // 转换mq需要的消息格式
    fn to_string(&self) -> String;

    // 初始化队列
    async fn init_queue(&self, channel: Channel) -> lapin::Result<()> {
        channel.exchange_declare(
            self.exchange_name(),
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        ).await?;

        let queue = channel.queue_declare(
            self.queue_name(),
            QueueDeclareOptions::default(),
            FieldTable::default(),
        ).await?;

        channel.queue_bind(
            queue.name().as_str(),
            self.exchange_name(),
            self.router_key(),
            QueueBindOptions::default(),
            FieldTable::default(),
        ).await?;

        Ok(())
    }

    // 生产者
    // expiration: 30分钟 = 30 * 60 * 1000
    async fn produce(&self, expiration: usize) -> lapin::Result<()> {
        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_priority(0)
            .with_delivery_mode(2)
            .with_expiration(expiration.to_string().into()); // 设置过期时间

        self.channel()
            .await?
            .basic_publish(
                self.exchange_name(),
                self.router_key(),
                BasicPublishOptions::default(),
                self.to_string().as_bytes(),
                properties,
            )
            .await?;

        Ok(())
    }

    // 消费者
    async fn consume(&self, queue_name: &'static str) -> lapin::Result<()> {
        let mut consumer = self
            .channel()
            .await?
            .basic_consume(
                queue_name,
                self.consumer_tag(),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        while let Some(message) = consumer.next().await {
            match message {
                Ok(delivery) => {
                    let _ = delivery.ack(BasicAckOptions::default()).await;

                    self.callback(delivery.data).await;
                }
                Err(err) => {
                    error!("{}", format!("消费信息错误: {}", err));
                }
            }
        }

        Ok(())
    }

    // 队列名称
    fn queue_name(&self) -> &'static str;

    // 交换机名称
    fn exchange_name(&self) -> &'static str;

    // 路由
    fn router_key(&self) -> &'static str;

    // 标签
    fn consumer_tag(&self) -> &'static str {
        ""
    }

    // mq channel
    async fn channel(&self) -> lapin::Result<Channel> {
        let rabbit = crate::rabbit_mq().await.clone();

        rabbit.create_channel().await
    }
}

/// 死信队列
#[axum::async_trait]
pub trait RabbitMQDlxQueue: RabbitMQQueue {
    // 初始化死信队列
    async fn init_dlx_queue(&self, channel: Channel) -> lapin::Result<()> {
        // -------------------------- 1. 正常队列 --------------------------
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
                        AMQPValue::LongString(self.dlx_router_key().into()),
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

        // -------------------------- 2. 死信队列 --------------------------
        channel
            .exchange_declare(
                self.dlx_exchange_name(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let dlx_queue = channel
            .queue_declare(
                self.dlx_queue_name(),
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        channel
            .queue_bind(
                dlx_queue.name().as_str(),
                self.dlx_exchange_name(),
                self.dlx_router_key(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(())
    }

    // 死信队列名称
    fn dlx_queue_name(&self) -> &'static str {
        Box::leak(Box::new(format!("dlx-{}", self.queue_name())))
    }

    // 死信交换机名称
    fn dlx_exchange_name(&self) -> &'static str {
        Box::leak(Box::new(format!("dlx-{}", self.exchange_name())))
    }

    // 死信路由器
    fn dlx_router_key(&self) -> &'static str {
        Box::leak(Box::new(format!("dlx-{}", self.router_key())))
    }
}
