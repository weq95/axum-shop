use serde::{Deserialize, Serialize};

use common::error::ApiResult;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Categories {
    pub id: i64,
    pub name: String,
    pub parent_id: i64,
    pub is_directory: bool,
    pub level: i16,
    pub path: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

impl Categories {
    // 创建类目处理路径和父级
    pub fn creating(&mut self, category: Option<Self>) {
        if let Some(parent) = category {
            self.level = parent.level + 1;
            self.path = format!("{}{}_", parent.path, parent.parent_id);

            return;
        }

        self.level = 0;
        self.path = "_".to_string();
    }

    // 获取父级
    pub async fn parent(&self) -> ApiResult<Vec<Categories>> {
        let result: Vec<Categories> = sqlx::query_as("select * from categories where parent_id = $1 and deleted_at not null")
            .bind(self.id)
            .fetch_all(&common::postgres().await)
            .await?;

        Ok(result)
    }

    // 获取子集
    pub fn children(&self) -> ApiResult<Vec<Categories>> {
        let result: Vec<Categories> = sqlx::query_as("select * from categories where path::text like '$1' and deleted_at not null")
            .bind(self.id)
            .fetch_all(&comm)
    }

    // 类目关联的商品
    pub fn products(&self) -> ApiResult<bool> {
        // 联合查询
    }
}