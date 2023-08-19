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
    /* match response {
        Ok(value) => {
            println!("{value:#?}");
            println!("{:?}", value.bytes().await)
        }
        Err(_e) => {
            println!("{:?}", _e)
        }
    }*/
}

fn router() -> axum::Router {
    axum::Router::new().route("/alipay/web", axum::routing::get(alipay_web))
}

static key: &str = "MIIEpQIBAAKCAQEAuy4cF0w64ixQMeVdmfR4LPpmEk+CPyj1wVgRP2V+jR18+iQUOUno1XVbjKpPw82YQykzRgmaSj7BUEYad7V2eaXzHUGpc/jIqiVyVtdtOggbEGJk9ufDlm5TqPX2UExc0Zj6EzoMZc0KKNoOBQ/3Lm76aF8D2VH+stBl0aYDLTGSgwfvNu11ErVLQceY9QxiVcL9j2XfM02tXrj9gTJ28/S3C5ObaGdlFjQyT3UyRXrnIHLlL1MdHzq0WDtro/DSuKpdiz6eWfi2EtEZHWjdE2Gry34oBbErWKOL15BJjG+QF2zjx9FGObWdsmjAKL77n8o7Uthb5ktMuumY0ejm0wIDAQABAoIBAQCT6KTD+DXTgWbBdtiXDmpkSF2d/HwUgr5n0LqYWRA+XF3kn9vnRTMacgksx2wOcojuEUF2B6KHJr3FPBAwJhF/oRXSOY+4l4+he8O1QbgLElqogMf9nzibx4SOUAYaf60c5wA9bzJaw0JS87P+ZhZR99oh3WsCMFvOWwUKPF/oNhWLdDm339vvil/KEWkxXNLksBI+SQLbuZT7Y47HzW4Ed8+9igSEf8wj+QUbH+B/JoUp/9qejiurxvPEw9icUyGzcQOxVvDZyviLHvB6W/M7zDMmBuOb4uygFSt4N2JSA5se5wLYstIolukMXkEqwcqgsUmzR8DlFjWnjHvVOjFJAoGBAPvpw5ueQg9tMNu7yBweoUBLyraPlsqRfsuOI6STWXz7jTb/dABo47XaqWsW4yFBupfoxMtouwY4L+TDbyuh5DRrdJO26jTnl2J87TOeo122Ubzszo4aXUW+ysY0aP2kRn/wBbYWrWnVA4oNraa88qbgerSFgzxuJxDyxebIqv+FAoGBAL43f7rJrL9drexf9ANNVcWG6uGa3WzoBV0qbPgudfKcN5GUYKZC6kKiQPRGE2kTLeL3Sw2bs6/arQdBHoALAgPKSZxHpP3673wo7VNLfL+kkLeEsUilqy7/2nHwoYcIMs5ESosEt/Gh/hUH+0BKV4nDF7v4PQNyRfaOgSgypaB3AoGAArLIU5xoXL3wrgne5N43H/cv3rC/DsBsOUX2f8bMSJhxNMubtH1rIwGKmwkNucd4djQaF4uxpSlo6exl/nOnfCBCiWqGK7bnWji4WbszSMexLHLk64TAxwR6K5FYJo9h6fDqPr8TcHTFqu7mk0im2L7C0bg0ZatQY9AV6pjvq/kCgYEAs/odH+YcTkDcBFBRuCIfKrNaYDZAlf/+u0UeL8D+Fpyas2L6A7ZCouOUo8v+J6he/WZQnEKbRKOanceOjUZdFKr89SKQyUL5/7dVvj8pfMa/qvShLYSbMPAihzZQD0zBjYruIRVI2hcVKl8P6qespgty1IavbChebEgvipJJRkcCgYEAnKyWNdt0RJj7r4ZJ64nJ8w4ijNVRSlqGN+f0SnVuE0LMwW7+94Z2kZWprTjuR1aRDHfCLWHCebYfvZJrYclFjZqAd503KB5BjxNfL36Z5K85SlbkFHdtVLTWVK8hKTcVA9aEx2nY5DwvSN253jWmssiLIuUIGhAjze5wDqwGhCk=";

async fn alipay_web() -> impl IntoResponse {
    let response = pay::AliPay::new(include_str!("../../cert/appPublicCert.crt"), key)
        .request("9021000125634026")
        .add_cert(
            Some(include_str!("../../cert/alipayPublicCert.crt")),
            Some(include_str!("../../cert/alipayRootCert.crt")),
        )
        .sandbox()
        .add_request(vec![
            ("return_url", "用户支付成功跳转页面"),
            ("notify_url", "通知商户支付成功URL，商户处理发货业务"),
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
