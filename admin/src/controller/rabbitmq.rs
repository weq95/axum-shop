use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use futures::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::{AMQPValue, FieldTable},
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Order {
    pub id: u32,
    pub created_at: u64,
    pub exp_at: u64,
}

// 正常队列: 队列名称, 交换机, 路由
const NORMAL_QUEUE: &str = "order-normal-queue";
const NORMAL_EXCHANGE: &str = "order-normal-exchange";
const NORMAL_ROUTING_KEY: &str = "order-normal-key";
// 死信队列: 队列名称, 交换机, 路由
const DLX_QUEUE: &str = "order_dlx_queue";
const DLX_EXCHANGE: &str = "order_dlx_queue";
const ROUTING_KEY: &str = "order_dlx_queue";

pub async fn init_rabbit() {
    let mut conn = common::rabbit_mq().await;
    let channel_normal = conn.create_channel().await.unwrap();
    let channel_dlx = conn.create_channel().await.unwrap();

    // -------------------------------------------------------------------------------------------------
    // # ========== 2.设置正常队列（队列、交换机、绑定） ==========
    //声明交换机
    channel_normal
        .exchange_declare(
            NORMAL_EXCHANGE,
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // 声明队列
    let mut args = FieldTable::default();
    args.insert("x-message-ttl".into(), AMQPValue::LongUInt(1800000)); //队列默认30分钟自动过期
    args.insert(
        "x-dead-letter-exchange".into(),
        AMQPValue::LongString(DLX_EXCHANGE.into()),
    );
    args.insert(
        "x-dead-letter-routing-key".into(),
        AMQPValue::LongString(ROUTING_KEY.into()),
    );
    let normal_queue = channel_normal
        .queue_declare(NORMAL_QUEUE, QueueDeclareOptions::default(), args)
        .await
        .unwrap();
    // 队列绑定（将队列、routing-key、交换机三者绑定到一起）
    channel_normal
        .queue_bind(
            normal_queue.name().as_str(),
            NORMAL_EXCHANGE,
            NORMAL_ROUTING_KEY,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // -------------------------------------------------------------------------------------------------
    // # ========== 3.设置死信队列（队列、交换机、绑定） ==========
    // 声明交换机
    channel_dlx
        .exchange_declare(
            DLX_EXCHANGE,
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // 声明死信队列 args 为 default()。切记不要给死信队列设置消息过期时间,否则失效的消息进入死信队列后会再次过期。
    let dlx_queue = channel_dlx
        .queue_declare(
            DLX_QUEUE,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // 队列绑定（将队列、routing-key、交换机三者绑定到一起）
    channel_dlx
        .queue_bind(
            dlx_queue.name().as_str(),
            DLX_EXCHANGE,
            ROUTING_KEY,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // -------------------------------------------------------------------------------------------------
    // 开始生产订单
    tokio::spawn(async move {
        println!("5. 开始生产订单");
        for i in 100..200 {
            let time_now_int = Utc::now().timestamp() as u64;
            let order = Order {
                id: i,
                created_at: time_now_int,
                exp_at: time_now_int + 5, // 3秒后过期
            };
            let payload = serde_json::to_string(&order).unwrap();
            let properties = BasicProperties::default()
                .with_content_type("application/json".into())
                .with_priority(0)
                .with_delivery_mode(2)
                .with_expiration("5000".into()); // 3秒后过期
                                                 //设置超时后自动推入死信队列
            let _publish_result = channel_normal
                .basic_publish(
                    NORMAL_EXCHANGE,
                    NORMAL_ROUTING_KEY,
                    BasicPublishOptions::default(),
                    payload.as_bytes(),
                    properties,
                )
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // # ========== 1.消费死信消息 ==========
    tokio::spawn(async move {
        let mut consumer = channel_dlx
            .basic_consume(
                DLX_QUEUE,
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
                    let payload: String = String::from_utf8(delivery.data).unwrap();
                    let order: Order = serde_json::from_str(&payload).unwrap();

                    let time_now = Utc::now().timestamp() as u64;
                    let timestamp = time_now - order.created_at;
                    println!(
                        "Received [dead]( {} )letter message: {:?} ---> time_now: {}",
                        timestamp, order, time_now,
                    );
                }
                Err(e) => println!("死信队列消费信息错误: {}", e),
            }
        }
    });
}
