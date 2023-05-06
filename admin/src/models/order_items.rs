use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Arguments, Postgres, Row, Transaction};


use common::error::ApiResult;
use common::Pagination;

use crate::models::products::Product;
use crate::models::user::Admin;

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct OrderItems {
    pub id: i64,
    pub order_id: i64,
    pub product_id: i64,
    pub product_sku: sqlx::types::Json<HashMap<String, serde_json::Value>>,
    pub rating: i16,
    pub review: String,
    pub reviewed_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct ItemProductSku {
    pub sku_id: i64,
    pub title: String,
    pub descr: String,
    pub amount: i16,
    pub price: i64,
    pub picture: String,
}

impl OrderItems {
    pub async fn create(
        order_id: i64,
        items: HashMap<i64, ItemProductSku>,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<(bool, Vec<HashMap<i64, i64>>)> {
        let item_len = items.len() as u64;
        let mut sql_builder = String::new();
        let mut idx = 1i32;
        let mut arg_builder = sqlx::postgres::PgArguments::default();
        let mut item_ids: Vec<HashMap<i64, i64>> = Vec::new();
        for (product_id, item) in items.iter() {
            sql_builder.push_str(format!(" (${}, ${}, ${}),", idx, idx + 1, idx + 2).as_str());
            idx += 2;

            arg_builder.add(order_id);
            arg_builder.add(product_id);
            arg_builder.add(json!(item));
            item_ids.push(HashMap::from([(product_id.clone(), item.sku_id)]))
        }

        Ok((
            sqlx::query_with(
                &*format!(
                    "insert into order_items (order_id,product_id,product_sku) values {}",
                    sql_builder[..sql_builder.len() - 1].to_string()
                ),
                arg_builder,
            )
            .execute(tx)
            .await?
            .rows_affected()
                == item_len,
            item_ids,
        ))
    }

    // 获取子订单
    pub async fn get(order_id: i64) -> ApiResult<Vec<OrderItems>> {
        let result: Vec<OrderItems> =
            sqlx::query_as("select * from order_items where order_id = $1")
                .bind(order_id)
                .fetch_all(common::postgres().await)
                .await?;

        Ok(result)
    }

    // 订单列表详情
    pub async fn items(
        order_ids: Vec<i64>,
    ) -> ApiResult<HashMap<i64, HashMap<String, serde_json::Value>>> {
        let item_result: Vec<OrderItems> =
            sqlx::query_as("select * from order_items where order_id = any($1)")
                .bind(order_ids)
                .fetch_all(common::postgres().await)
                .await?;

        let item_result = item_result
            .iter()
            .map(|row| {
                HashMap::from([
                    ("id".to_string(), serde_json::to_value(row.id).unwrap()),
                    (
                        "order_id".to_string(),
                        serde_json::to_value(row.order_id).unwrap(),
                    ),
                    (
                        "product_sku".to_string(),
                        serde_json::to_value(row.product_sku.clone()).unwrap(),
                    ),
                ])
            })
            .map(|row| {
                let order_id = row.get("order_id").unwrap().as_i64().unwrap();
                (order_id, row)
            })
            .collect::<HashMap<i64, HashMap<String, serde_json::Value>>>();

        Ok(item_result)
    }

    // 删除字订单详情
    pub async fn delete(
        order_id: i64,
        product_id: i64,
        tx: &mut Transaction<'_, Postgres>,
    ) -> ApiResult<bool> {
        Ok(
            sqlx::query("delete from order_items where order_id = $1 and product_id = $2")
                .bind(order_id)
                .bind(product_id)
                .execute(tx)
                .await?
                .rows_affected()
                > 0,
        )
    }

    // 订单详情
    pub async fn detail(id: i64, order_id: i64) -> ApiResult<OrderItems> {
        let result: OrderItems =
            sqlx::query_as("select * from order_items where id = $1 and order_id = $2")
                .bind(id)
                .bind(order_id)
                .fetch_one(common::postgres().await)
                .await?;

        Ok(result)
    }

    // 商品评价
    pub async fn evaluate(&self, userid: i64, score: u8, content: String) -> ApiResult<()> {
        let mut tx = common::postgres().await.begin().await?;

        sqlx::query(
            "update order_items set rating = $1, review = $2, reviewed_at=$3 where id = $4",
        )
        .bind(score as i16)
        .bind(content)
        .bind(chrono::Local::now().naive_local())
        .bind(self.id)
        .execute(&mut tx)
        .await?;

        let total = sqlx::query("select count(*) as total from order_items where order_id = $1 and product_id = $2 and rating=0")
            .bind(self.order_id)
            .bind(self.product_id)
            .fetch_one(&mut tx)
            .await?.get::<i64, _>("total");
        if total == 0 {
            let _ = sqlx::query("update orders set reviewed = $1 where id = $2 and user_id = $3")
                .bind(true)
                .bind(self.order_id)
                .bind(userid)
                .execute(&mut tx)
                .await?;
        }

        // 平均评分：
        let mut sql = " update products set review_count = review_count::INT + $1 ".to_string();
        if score > 6 {
            // 好评， 用来计算好评率
            sql.push_str(", rating = rating::INT8 + 1 ")
        }
        sql.push_str(" where id = $3 ");
        sqlx::query(&*sql)
            .bind(1)
            .bind(score as i8)
            .bind(self.product_id)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    // 商品评价列表
    pub async fn evaluate_list(
        product_id: i64,
        pagination: &mut Pagination<HashMap<String, serde_json::Value>>,
    ) -> ApiResult<()> {
        let sql = format!("(select id,order_id,product_id,rating,review,reviewed_at from order_items where product_id={}\
         and rating>0 order by id asc limit {} offset {}) as items", product_id, pagination.limit(), pagination.offset());
        let total_sql = format!(
            "select count(*) as total from order_items where product_id={} and rating>0",
            product_id
        );

        let mut user_ids: Vec<i64> = Vec::new();
        let mut product_ids: Vec<i64> = Vec::new();

        let mut result: Vec<HashMap<String, serde_json::Value>> = sqlx::query(&*format!(
            "select items.*,orders.user_id from {} left join orders on items.order_id = orders.id",
            sql
        ))
        .fetch_all(common::postgres().await)
        .await?
        .iter()
        .map(|row| {
            //id | order_id | product_id | rating | review |reviewed_at | user_id
            let product_id = row.get::<i64, _>("product_id");
            let user_id = row.get::<i64, _>("user_id");
            let reviewed_at = row
                .get::<chrono::NaiveDateTime, _>("reviewed_at")
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();

            user_ids.push(user_id);
            product_ids.push(product_id);

            HashMap::from([
                (
                    "id".to_string(),
                    serde_json::to_value(row.get::<i64, _>("id")).unwrap(),
                ),
                (
                    "user_id".to_string(),
                    serde_json::to_value(product_id).unwrap(),
                ),
                (
                    "product_id".to_string(),
                    serde_json::to_value(product_id).unwrap(),
                ),
                (
                    "rating".to_string(),
                    serde_json::to_value(row.get::<i16, _>("rating")).unwrap(),
                ),
                (
                    "reviewed_at".to_string(),
                    serde_json::to_value(reviewed_at).unwrap(),
                ),
                ("nickname".to_string(), serde_json::to_value("").unwrap()),
                ("title".to_string(), serde_json::to_value("").unwrap()),
            ])
        })
        .collect::<Vec<HashMap<String, serde_json::Value>>>();

        let users = Admin::user_maps(user_ids).await?;
        let products = Product::product_maps(product_ids).await?;
        for row in result.iter_mut() {
            let user_id = row.get("user_id").unwrap().as_i64().unwrap();
            let product_id = row.get("product_id").unwrap().as_i64().unwrap();

            if let Some(user) = users.get(&user_id) {
                row.insert(
                    "nickname".to_string(),
                    serde_json::to_value(&user.nickname).unwrap(),
                );
            }
            if let Some(product) = products.get(&product_id) {
                row.insert("title".to_string(), product.get("title").unwrap().clone());
            }
        }

        let total = sqlx::query(&*total_sql)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("total");

        pagination.set_data(result);
        pagination.set_total(total as usize);

        Ok(())
    }
}
