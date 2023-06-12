use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgMoney;
use sqlx::Postgres;

#[derive(Debug, sqlx::FromRow)]
pub struct CrowdfundingProduct {
    pub id: i64,
    pub product_id: i64,
    pub target_amount: PgMoney,
    pub total_amount: PgMoney,
    pub user_count: i32,
    pub end_at: chrono::NaiveDateTime,
    pub status: Status,
}

#[repr(i16)]
#[derive(Debug, Serialize, Deserialize, PartialEq, sqlx::Type)]
pub enum Status {
    Funding = 0,
    Success = 1,
    Fail = 2,
}

impl Default for Status {
    fn default() -> Self {
        Self::Funding
    }
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Status::Funding => "众筹中".to_string(),
            Status::Success => "众筹成功".to_string(),
            Status::Fail => "众筹失败".to_string(),
        }
    }
}

impl CrowdfundingProduct {
    pub fn percent(&self) -> String {
        let value = self.total_amount.0 / self.target_amount.0;

        format!("{:.1$}", value, 2)
    }
}
