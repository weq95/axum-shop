use serde::Serialize;

pub mod address;
pub mod auth;
pub mod cart_items;
pub mod categories;
pub mod coupons;
pub mod crowdfunding;
pub mod favorite_products;
pub mod installment_items;
pub mod installments;
pub mod order_items;
pub mod orders;
pub mod product_skus;
pub mod products;
pub mod user;

// 支付方式
#[derive(Debug, PartialEq, sqlx::Type, Serialize)]
#[repr(i8)]
pub enum PayMethod {
    Unknown,
    AliPay,
    WeChat,
    GooglePay,
    PayPal,
    Installment,
}

impl Default for PayMethod {
    fn default() -> Self {
        PayMethod::AliPay
    }
}

impl AsRef<str> for PayMethod {
    fn as_ref(&self) -> &str {
        match self {
            PayMethod::AliPay => "支付宝",
            PayMethod::WeChat => "微信支付",
            PayMethod::GooglePay => "Google",
            PayMethod::PayPal => "Paypal",
            PayMethod::Installment => "分期付款",
            PayMethod::Unknown => "其他方式",
        }
    }
}

impl From<PayMethod> for i8 {
    fn from(value: PayMethod) -> Self {
        match value {
            PayMethod::Unknown => 0,
            PayMethod::AliPay => 1,
            PayMethod::WeChat => 2,
            PayMethod::GooglePay => 3,
            PayMethod::PayPal => 4,
            PayMethod::Installment => 5,
        }
    }
}

// 退款状态
#[derive(Debug, PartialEq, sqlx::Type)]
#[repr(i8)]
pub enum RefundStatus {
    PENDING,
    PROCESSING,
    Waiting,
    SUCCESS,
    FAILED,
}

impl Default for RefundStatus {
    fn default() -> Self {
        RefundStatus::PENDING
    }
}

impl AsRef<str> for RefundStatus {
    fn as_ref(&self) -> &str {
        match self {
            RefundStatus::PENDING => "未退款",
            RefundStatus::PROCESSING => "已申请",
            RefundStatus::Waiting => "等待中",
            RefundStatus::SUCCESS => "退款成功",
            RefundStatus::FAILED => "退款失败",
        }
    }
}

impl From<RefundStatus> for i8 {
    fn from(status: RefundStatus) -> i8 {
        match status {
            RefundStatus::PENDING => 0,
            RefundStatus::PROCESSING => 1,
            RefundStatus::Waiting => 2,
            RefundStatus::SUCCESS => 3,
            RefundStatus::FAILED => 4,
        }
    }
}

// 物流状态
#[derive(Debug, PartialEq, sqlx::Type)]
#[repr(i8)]
pub enum LogisticStatus {
    Processing,
    ToBeReceived,
    Received,
}

impl Default for LogisticStatus {
    fn default() -> Self {
        LogisticStatus::Processing
    }
}

impl AsRef<str> for LogisticStatus {
    fn as_ref(&self) -> &str {
        match self {
            LogisticStatus::Processing => "处理中",
            LogisticStatus::ToBeReceived => "待收货",
            LogisticStatus::Received => "已收货",
        }
    }
}

impl From<LogisticStatus> for i8 {
    fn from(value: LogisticStatus) -> Self {
        match value {
            LogisticStatus::Processing => 0,
            LogisticStatus::ToBeReceived => 1,
            LogisticStatus::Received => 2,
        }
    }
}
