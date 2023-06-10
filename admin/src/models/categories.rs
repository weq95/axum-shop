use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;

use common::error::{ApiError, ApiResult};
use common::tree::{Node, NodeTrait};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Default, Clone)]
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
    pub async fn index(category_id: i64) -> ApiResult<Vec<Value>> {
        let mut sql = "select id,name,parent_id,is_directory,level,path from categories \
        where 1=1 "
            .to_string();
        if category_id > 0 {
            sql.push_str(&format!(" and path like '%{}%'", category_id));
        }
        sql.push_str(" and deleted_at is null order by id asc");

        let mut result: Vec<Categories> = sqlx::query(&*sql)
            .fetch_all(&*common::postgres().await)
            .await?
            .iter()
            .map(|row| {
                let time_only: chrono::NaiveDateTime = chrono::Local::now().naive_local();
                Categories {
                    id: row.get::<i64, _>("id"),
                    name: row.get::<String, _>("name"),
                    parent_id: row.get::<i64, _>("parent_id"),
                    is_directory: row.get::<bool, _>("is_directory"),
                    level: row.get::<i16, _>("level"),
                    path: row.get::<String, _>("path"),
                    created_at: time_only,
                    updated_at: time_only,
                    deleted_at: None,
                }
            })
            .collect::<Vec<Categories>>();

        Ok(Self::build_tree(&mut result, category_id))
    }

    pub async fn get(id: i64) -> ApiResult<Option<Categories>> {
        if id <= 0 {
            return Ok(None);
        }
        let result: Option<Categories> =
            sqlx::query_as("select * from categories where id = $1 and deleted_at is null")
                .bind(id)
                .fetch_optional(&*common::postgres().await)
                .await?;

        Ok(result)
    }

    pub async fn unique_name(name: &str, this_id: Option<i64>) -> ApiResult<bool> {
        let mut sql = "".to_string();
        if let Some(id) = this_id {
            sql.push_str(format!(" and id != {} ", id).as_str())
        }
        Ok(sqlx::query(&*format!(
            "select exists (select id from categories where name = $1 {} and deleted_at is null)",
            sql
        ))
        .bind(name)
        .fetch_one(&*common::postgres().await)
        .await?
        .get::<bool, _>("exists"))
    }

    // 创建
    pub async fn store(mut self) -> ApiResult<i64> {
        if Self::unique_name(&self.name, None).await? {
            return Err(ApiError::Error("类目名称已存在, 请换一个试试!".to_string()));
        }

        self.creating(Self::get(self.parent_id).await?);

        Ok(sqlx::query("insert into categories (name,parent_id,is_directory,level,path) values ($1, $2, $3, $4, $5) RETURNING id")
            .bind(&self.name)
            .bind(self.parent_id)
            .bind(self.is_directory)
            .bind(self.level)
            .bind(&self.path)
            .fetch_one(&*common::postgres().await)
            .await?.get::<i64, _>("id"))
    }

    // 创建类目处理路径和父级
    pub fn creating(&mut self, category: Option<Self>) {
        if let Some(parent) = category {
            self.level = parent.level + 1;
            self.path = format!("{}{}_", parent.path, parent.id);

            return;
        }

        self.level = 0;
    }

    // 是否有子类目
    pub async fn is_children(id: i64) -> ApiResult<bool> {
        Ok(
            sqlx::query("select exists (select id from categories where parent_id = $1)")
                .bind(id)
                .fetch_one(&*common::postgres().await)
                .await?
                .get::<bool, _>("exists"),
        )
    }

    // 类目是否存在
    pub async fn exits(category_id: i64) -> ApiResult<bool> {
        if category_id <= 0 {
            return Ok(false);
        }

        Ok(sqlx::query(
            "select exists (select id from categories where id = $1 and deleted_at is null)",
        )
        .bind(category_id)
        .fetch_one(&*common::postgres().await)
        .await?
        .get::<bool, _>("exists"))
    }

    pub async fn update(self, name: &str) -> ApiResult<bool> {
        if Self::unique_name(name, Some(self.id)).await? {
            return Err(ApiError::Error("类目名称已存在，请换一个试试".to_string()));
        }

        Ok(
            sqlx::query("update categories set name = $1, updated_at = $2 where id = $3")
                .bind(name)
                .bind(chrono::Local::now())
                .bind(self.id)
                .execute(&*common::postgres().await)
                .await?
                .rows_affected()
                > 0,
        )
    }

    // 类目是否商品使用
    pub async fn is_use_product(category_id: i64) -> ApiResult<bool> {
        Ok(
            sqlx::query("select exists (select id from products where category_id = $1 limit 1)")
                .bind(category_id)
                .fetch_one(&*common::postgres().await)
                .await?
                .get::<bool, _>("exists"),
        )
    }

    pub async fn delete(id: i64) -> ApiResult<bool> {
        if Self::is_use_product(id).await? {
            return Err(ApiError::Error("正在使用中...".to_string()));
        }

        if Self::is_children(id).await? {
            return Err(ApiError::Error("请先删除子类目".to_string()));
        }

        Ok(sqlx::query("delete from categories where id = $1")
            .bind(id)
            .execute(&*common::postgres().await)
            .await?
            .rows_affected()
            > 0)
    }
}

impl Node for Categories {
    fn get_pid(&self) -> i64 {
        self.parent_id
    }

    fn get_id(&self) -> i64 {
        self.id
    }

    fn get_data(&self) -> Value {
        json!({
            "id": json!(self.id),
            "name":  json!(self.name.clone()),
            "parent_id": json!( self.parent_id),
            "is_directory":json!( self.is_directory),
            "level":  json!( self.level),
            "path":  json!(self.path.clone()),
        })
    }

    fn is_root(&self, pid: i64) -> bool {
        self.parent_id == pid
    }
}

impl NodeTrait<Categories> for Categories {}
