use serde::Deserialize;
use validator::Validate;

#[derive(Validate, Deserialize, Clone)]
pub struct ReqCreateOrder {
    #[validate(range(
        min = 1,
        max = 50,
        message = "请选择你需要购买的商品, 单次最多允许购买50个商品"
    ))]
    pub products: Option<Vec<OrderProduct>>,
    #[validate(range(min = 1), message = "请选择收获地址")]
    pub address_id: Option<i64>,
    #[validate(range(min = 0, max = 255, message = "备注信息不能超过255个字符"))]
    pub remark: Option<String>,
}

#[derive(Validate, Deserialize, Clone)]
pub struct OrderProduct {
    #[validate(range(min = 1), message = "非法的商品")]
    pub product_id: Option<i64>,
    #[validate(range(min = 1), message = "非法的商品sku")]
    pub product_sku_id: Option<i64>,
    #[validate(range(min = 1, max = 10000), message = "单次购买数量在1-10000之间")]
    pub amount: Option<i64>,
}
