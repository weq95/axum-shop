use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use futures::StreamExt;
use lapin::options::{BasicAckOptions, ExchangeDeclareOptions};
use lapin::types::AMQPValue;
use lapin::{
    options::{BasicConsumeOptions, BasicPublishOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
pub struct Order {
    pub id: u32,
    pub created_at: u64,
    pub exp_at: u64,
}

lazy_static! {}

// 正常队列: 队列名称, 交换机, 路由
const ORDER_NORMAL_QUEUE: &str = "order-normal-queue";
const ORDER_NORMAL_EXCHANGE: &str = "order-normal-exchange";
const ORDER_NORMAL_ROUTING_KEY: &str = "order-normal-key";
// 死信队列: 队列名称, 交换机, 路由
const ORDER_DLX_QUEUE: &str = "order_dlx_queue";
const ORDER_DLX_EXCHANGE: &str = "order_dlx_queue";
const ORDER_DLX_ROUTING_KEY: &str = "order_dlx_queue";

pub async fn init_rabbit() {
    // let mut conn = common::rabbit_mq().await;
    let mut conn = Connection::connect(
        ADDR,
        ConnectionProperties::default(),
    )
    .await
    .unwrap();
    println!("链接成功, success!");

    let mut channel = conn.create_channel().await.unwrap();

    println!("1. channel 创建成功");
    // -------------------------------------------------------------------------------------------------
    // # ========== 2.设置正常队列（队列、交换机、绑定） ==========
    // 声明队列
    let mut args = FieldTable::default();
    args.insert(
        "x-dead-letter-exchange".into(),
        AMQPValue::ShortString(ORDER_DLX_EXCHANGE.into()),
    );
    args.insert(
        "x-dead-letter-routing-key".into(),
        AMQPValue::ShortString(ORDER_DLX_ROUTING_KEY.into()),
    );
    args.insert("x-message-ttl".into(), AMQPValue::LongUInt(1800000)); //队列默认30分钟自动过期

    let mut options = QueueDeclareOptions::default();
    options.durable = true;
    channel
        .queue_declare(ORDER_NORMAL_QUEUE, options, args)
        .await
        .unwrap();

    //声明交换机
    channel
        .exchange_declare(
            ORDER_NORMAL_EXCHANGE,
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // 队列绑定（将队列、routing-key、交换机三者绑定到一起）
    channel
        .queue_bind(
            ORDER_NORMAL_QUEUE,
            ORDER_NORMAL_ROUTING_KEY,
            ORDER_NORMAL_EXCHANGE,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();
    println!("3. 普通队列注册成功");
    // -------------------------------------------------------------------------------------------------
    // # ========== 3.设置死信队列（队列、交换机、绑定） ==========
    // 声明死信队列 args 为 default()。切记不要给死信队列设置消息过期时间,否则失效的消息进入死信队列后会再次过期。
    channel
        .queue_declare(
            ORDER_DLX_QUEUE,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // 声明交换机
    channel
        .exchange_declare(
            ORDER_DLX_EXCHANGE,
            ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    // 队列绑定（将队列、routing-key、交换机三者绑定到一起）
    channel
        .queue_bind(
            ORDER_DLX_QUEUE,
            ORDER_DLX_ROUTING_KEY,
            ORDER_DLX_EXCHANGE,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    println!("4. 死信队列注册成功");
    // -------------------------------------------------------------------------------------------------
    // 开始生产订单
    let channel_normal = channel.clone();
    tokio::spawn(async move {
        println!("5. 开始生产订单");
        for i in 100..200 {
            let time_now_int = Utc::now().timestamp() as u64;
            let order = Order {
                id: i,
                created_at: time_now_int,
                exp_at: time_now_int + 3, // 3秒后过期
            };
            let payload = serde_json::to_string(&order).unwrap();
            let properties = BasicProperties::default()
                .with_content_type("application/json".into())
                .with_priority(0)
                .with_delivery_mode(2)
                .with_expiration("3000".into()); // 3秒后过期
                                                 //设置超时后自动推入死信队列
            let _publish_result = channel_normal
                .basic_publish(
                    ORDER_NORMAL_EXCHANGE,
                    ORDER_NORMAL_ROUTING_KEY,
                    BasicPublishOptions::default(),
                    payload.as_bytes(),
                    properties,
                )
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_secs(1));
        }

        println!("5. 订单生产完毕");
    });

    // # ========== 1.消费死信消息 ==========
    let channel_dlx = channel.clone();
    tokio::spawn(async move {
        let mut consumer = channel_dlx
            .basic_consume(
                ORDER_DLX_QUEUE,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();
        println!("2. dlx 队列开始等待消费");

        while let Some(message) = consumer.next().await {
            match message {
                Ok(delivery) => {
                    delivery.ack(BasicAckOptions::default()).await.unwrap();
                    let payload: String = String::from_utf8(delivery.data).unwrap();
                    let order: Order = serde_json::from_str(&payload).unwrap();

                    let timestamp = order.exp_at - (Utc::now().timestamp() as u64);
                    println!(
                        "Received dead[死信] [ {} ]letter message: {:?}",
                        timestamp, order
                    );
                }
                Err(e) => println!("死信队列消费信息错误: {}", e),
            }
        }
    });
}

const ADDR: &str =
    "amqps://iscyijgb:T_DYdACmNMeRIfhOeQdrvDAiw06J1WWg@rattlesnake.rmq.cloudamqp.com/iscyijgb";
