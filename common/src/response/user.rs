use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GetUser {
    pub id: u64,
    pub age: u8,
    pub name: String,
    pub nickname: String,
    pub phone: String,
    pub email: String,
}
