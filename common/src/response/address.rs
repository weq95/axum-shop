use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct ResAddress {
    pub id: i64,
    pub user_id: i64,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub street: Option<String>,
    pub address: String,
    pub zip: i32,
    pub contact_name: String,
    pub contact_phone: String,
    pub last_used_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResAddrResult {
    pub id: i32,
    pub name: String,
}