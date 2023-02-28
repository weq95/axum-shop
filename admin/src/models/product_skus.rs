use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProductSku {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub price: f64,
    pub stock: u64,
    pub product_id: u64,
}
