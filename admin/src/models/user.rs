use std::collections::HashMap;

use sqlx::Row;

use common::error::ApiError;
use common::request::user::ReqGetUser;
use common::{
    error::ApiResult,
    parse_field,
    request::user::{ReqCrateUser, ReqUpdateUser},
    response::user::GetUser,
    Pagination,
};

use crate::models::favorite_products::FavoriteProducts;
use crate::models::product_skus::ProductSku;

pub struct Admin {
    pub id: i64,
}

impl Admin {
    /// get 用户详情信息
    pub async fn get(info: ReqGetUser) -> ApiResult<GetUser> {
        let mut sql_str = sqlx::QueryBuilder::new(
            "select id,name,age,nickname,phone,email from users where 1=1 ",
        );
        if let Some(userid) = info.id {
            sql_str.push(" and id = ").push_bind(userid);
        }
        if let Some(username) = info.name {
            sql_str
                .push(" and name like ")
                .push_bind(format!("{}%", username));
        }
        if let Some(age) = info.age {
            sql_str.push(" and age = ").push(age as i16);
        }
        if let Some(phone) = info.phone {
            sql_str.push(" and phone = ").push_bind(phone);
        }
        if let Some(email) = info.email {
            sql_str.push(" and email = ").push_bind(email);
        }

        // 注意这里没数据不会报错
        sql_str
            .build()
            .fetch_optional(common::postgres().await)
            .await?
            .map(|row| {
                Ok(GetUser {
                    id: row.get::<i64, &str>("id") as u64,
                    age: row.get::<i16, &str>("age") as u8,
                    name: row.get("name"),
                    nickname: row.get("nickname"),
                    phone: row.get("phone"),
                    email: row.get("email"),
                })
            })
            .ok_or(ApiError::Error("NotFound".to_string()))?
    }

    /// list 用户列表
    pub async fn lists(
        pagination: &mut Pagination<GetUser>,
        params: &serde_json::Value,
    ) -> ApiResult<()> {
        let mut sql_str = sqlx::QueryBuilder::new(
            "select id,name,age,nickname,phone,email from users where 1=1 ",
        );
        let mut count_str =
            sqlx::QueryBuilder::new("select count(*) as total from users where 1=1 ");
        if let Some(email) = parse_field::<String>(&params, "email") {
            sql_str
                .push(" and email like ")
                .push_bind(format!("%{}%", &email));
            count_str
                .push(" and email like ")
                .push_bind(format!("%{}%", &email));
        }
        if let Some(name) = parse_field::<String>(&params, "name") {
            sql_str
                .push(" and name like ")
                .push_bind(format!("%{}%", &name));
            count_str
                .push(" and name like ")
                .push_bind(format!("%{}%", &name));
        }
        if let Some(phone) = parse_field::<String>(&params, "phone") {
            sql_str
                .push(" and phone like ")
                .push_bind(format!("{}%", &phone));
            count_str
                .push(" and phone like ")
                .push_bind(format!("{}%", &phone));
        }
        if let Some(nickname) = parse_field::<String>(&params, "nickname") {
            sql_str
                .push(" and nickname like ")
                .push_bind(format!("%{}%", &nickname));
            count_str
                .push(" and nickname like ")
                .push_bind(format!("%{}%", &nickname));
        }

        let count = count_str
            .build()
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, &str>("total") as usize;
        pagination.set_total(count);
        sql_str.push(format!(
            " order by id desc limit {} offset {}",
            pagination.limit(),
            pagination.offset()
        ));

        let data = sql_str
            .build()
            .fetch_all(common::postgres().await)
            .await?
            .into_iter()
            .map(|row| GetUser {
                id: row.get::<i64, &str>("id") as u64,
                age: row.get::<i16, &str>("age") as u8,
                name: row.get("name"),
                nickname: row.get("nickname"),
                phone: row.get("phone"),
                email: row.get("email"),
            })
            .collect::<Vec<GetUser>>();

        pagination.set_data(data);
        Ok(())
    }

    /// create 创建用户
    pub async fn create(info: ReqCrateUser) -> ApiResult<u64> {
        let phone = &info.phone.unwrap()[3..].to_string();
        let id: i64 = sqlx::query("insert into users (name, age, nickname, phone, email) values($1, $2, $3, $4, $5) RETURNING id")
            .bind(&info.name).bind(&info.age).bind(&info.nickname)
            .bind(phone).bind(&info.email)
            .fetch_one(common::postgres().await)
            .await?.get::<i64, &str>("id");

        Ok(id as u64)
    }

    /// update 更新用户信息
    pub async fn update(info: ReqUpdateUser) -> ApiResult<bool> {
        let rows_num = sqlx::query("update users set name = $1, age = $2 where id = $3")
            .bind(&info.name.unwrap())
            .bind(&info.age.unwrap())
            .bind(&info.id.unwrap())
            .execute(common::postgres().await)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }

    /// delete 删除用户
    pub async fn delete(userid: u64) -> ApiResult<bool> {
        FavoriteProducts::un_favorite_user(userid as i64).await?;
        let rows_num = sqlx::query("delete from users where id = $1")
            .bind(userid as i64)
            .execute(common::postgres().await)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }

    // 购物车
    pub async fn cart_items(
        user_id: i64,
        pagination: &mut Pagination<HashMap<String, serde_json::Value>>,
    ) -> ApiResult<()> {
        let count = sqlx::query("select count(*) as count from cart_items where user_id = $1")
            .bind(user_id)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("count") as usize;
        pagination.set_total(count).total_pages();

        let mut result: Vec<HashMap<String, serde_json::Value>> = Vec::new();
        let product_ids: Vec<i64> =
            sqlx::query("SELECT ci.id,ci.product_id,ci.product_sku_id,ci.amount,p.title FROM
            (select id,product_id,product_sku_id,amount from cart_items where user_id = $1 order by created_at desc offset $2 limit $3)
            as ci LEFT JOIN products as p ON ci.product_id = p.id")
                .bind(user_id)
                .bind(pagination.offset() as i64)
                .bind(pagination.limit() as i64)
                .fetch_all(common::postgres().await)
                .await?.iter().map(|row| {
                result.push(HashMap::from([
                    ("id".to_string(), serde_json::to_value(row.get::<i64, _>("id")).unwrap()),
                    ("product_id".to_string(), serde_json::to_value(row.get::<i64, _>("product_id")).unwrap()),
                    ("product_sku_id".to_string(), serde_json::to_value(row.get::<i64, _>("product_sku_id")).unwrap()),
                    ("amount".to_string(), serde_json::to_value(row.get::<i16, _>("amount")).unwrap()),
                    ("title".to_string(), serde_json::to_value(row.get::<&str, _>("title")).unwrap()),
                ]));

                row.get::<i64, _>("product_id")
            }).collect::<Vec<i64>>();

        let product_skus = ProductSku::skus(product_ids).await?;
        for (_, item) in result.iter_mut().enumerate() {
            let key = item.get("product_id").unwrap().as_i64().unwrap();

            item.insert(
                "product_skus".to_string(),
                serde_json::to_value(product_skus.get(&key).unwrap_or(&Vec::new())).unwrap(),
            );
        }

        pagination.set_data(result);
        Ok(())
    }
}
