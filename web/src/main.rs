use std::net::SocketAddr;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    axum::Server::bind(&SocketAddr::from(([127, 0, 0, 1], 8081)))
        .serve(router().into_make_service())
        .await
        .unwrap();
}

fn router() -> axum::Router {
    axum::Router::new().route("/alipay/web", axum::routing::get(alipay_web))
}

async fn alipay_web() -> impl IntoResponse {
    let alipay = &common::application_config().await.alipay;
    let response = pay::AliPay::new(
        include_str!("../../cert/appPublicCert.crt"),
        alipay.private_key.clone().as_str(),
    )
    .request(alipay.app_id.as_str())
    .add_cert(
        Some(include_str!("../../cert/alipayPublicCert.crt")),
        Some(include_str!("../../cert/alipayRootCert.crt")),
    )
    .sandbox()
    .add_request(vec![
        ("return_url", alipay.return_url.as_str()),
        ("notify_url", alipay.notify_url.as_str()),
    ])
    .post(
        "alipay.trade.page.pay",
        Some(&vec![
            (
                "out_trade_no",
                &chrono::Local::now().timestamp().to_string(),
            ),
            ("total_amount", "15000"),
            ("subject", "iphone15pro"),
            ("product_code", "FAST_INSTANT_TRADE_PAY"),
        ]),
    )
    .await;

    match response {
        Ok(body) => Response::builder()
            .extension(|| {})
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-cache")
            .header("Content-Type", "text/html; charset=UTF-8")
            .body(body.text().await.unwrap())
            .unwrap()
            .into_response(),
        Err(_e) => (StatusCode::BAD_REQUEST, format!("{:?}", _e)).into_response(),
    }
}
