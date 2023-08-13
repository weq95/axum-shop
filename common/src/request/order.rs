use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Validate, Deserialize, Serialize, Clone)]
pub struct ReqCreateOrder {
    #[validate(length(
        min = 1,
        max = 50,
        message = "请选择你需要购买的商品, 单次最多允许购买50个商品"
    ))]
    pub products: Option<Vec<OrderProduct>>,
    #[validate(range(min = 1, message = "请选择收获地址"))]
    pub address_id: Option<i64>,
    #[validate(length(min = 0, max = 255, message = "备注信息不能超过255个字符"))]
    pub remark: Option<String>,
    pub coupon_code: Option<String>,
}

#[derive(Validate, Deserialize, Serialize, Clone)]
pub struct OrderProduct {
    #[validate(range(min = 1, message = "非法的商品"))]
    pub product_id: Option<i64>,
    #[validate(range(min = 1, message = "非法的商品sku"))]
    pub product_sku_id: Option<i64>,
    #[validate(range(min = 1, max = 10000, message = "单次购买数量在1-10000之间"))]
    pub amount: Option<i32>,
}

#[derive(Validate, Deserialize, Serialize, Clone)]
pub struct OrderShip {
    #[validate(range(min = 1, message = "订单ID错误"))]
    pub id: Option<i64>,
    #[validate(required)]
    pub express_company: Option<String>,
    #[validate(length(min = 4, max = 100, message = "公司名称必须在4-100字符之间"))]
    pub express_no: Option<String>,
}

#[derive(Validate, Deserialize, Serialize, Clone)]
pub struct OrderEvaluate {
    #[validate(range(min = 1, message = "订单详情ID错误"))]
    pub id: Option<i64>,
    #[validate(range(min = 1, message = "订单ID错误"))]
    pub order_id: Option<i64>,
    #[validate(range(min = 1, message = "分数：1-10分之间哦"))]
    pub score: Option<i8>,
    #[validate(length(min = 4, max = 100, message = "评价内容必须在4-255字符之间"))]
    pub content: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ReqInstallments {
    pub min_amount: f32,
    pub count: u8,
    pub order_id: i64,
}
