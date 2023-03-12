use sqlx::Row;
use validator::Validate;

use common::error::ApiError;
use common::request::user::ReqGetUser;
use common::{
    error::ApiResult,
    parse_field,
    request::user::{ReqCrateUser, ReqUpdateUser},
    response::user::GetUser,
    Pagination,
};

use crate::models::cart_items::CartItems;
use crate::models::favorite_products::FavoriteProductsModel;

pub struct AdminModel {
    pub id: i64,
}

impl AdminModel {
    /// get 用户详情信息
    pub async fn get(info: ReqGetUser) -> ApiResult<GetUser> {
        info.validate()?;

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
            .fetch_optional(common::pgsql::db().await)
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
            .fetch_one(common::pgsql::db().await)
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
            .fetch_all(common::pgsql::db().await)
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
        info.validate()?;

        let phone = &info.phone.unwrap()[3..].to_string();
        let id: i64 = sqlx::query("insert into users (name, age, nickname, phone, email) values($1, $2, $3, $4, $5) RETURNING id")
            .bind(&info.name).bind(&info.age).bind(&info.nickname)
            .bind(phone).bind(&info.email)
            .fetch_one(common::pgsql::db().await)
            .await?.get::<i64, &str>("id");

        Ok(id as u64)
    }

    /// update 更新用户信息
    pub async fn update(info: ReqUpdateUser) -> ApiResult<bool> {
        let rows_num = sqlx::query("update users set name = $1, age = $2 where id = $3")
            .bind(&info.name.unwrap())
            .bind(&info.age.unwrap())
            .bind(&info.id.unwrap())
            .execute(common::pgsql::db().await)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }

    /// delete 删除用户
    pub async fn delete(userid: u64) -> ApiResult<bool> {
        FavoriteProductsModel::un_favorite_user(userid as i64).await?;
        let rows_num = sqlx::query("delete from users where id = $1")
            .bind(userid as i64)
            .execute(common::pgsql::db().await)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }

    // 购物车
    pub async fn cart_items(&self, pagination: &mut Pagination<CartItems>) -> ApiResult<()> {
        let count = sqlx::query("select count(*) as count from cart_items where user_id = $1")
            .bind(self.id)
            .fetch_one(common::pgsql::db().await)
            .await?
            .get::<i64, _>("count") as usize;
        pagination.set_total(count).total_pages();

        let cart_items: Vec<CartItems> =
            sqlx::query_as("select * from cart_items where user_id = $1 order by created_at desc offset $2 limit $3")
                .bind(self.id)
                .bind(pagination.offset() as i64)
                .bind(pagination.limit() as i64)
                .fetch_all(common::pgsql::db().await)
                .await?;

        pagination.set_data(cart_items);
        Ok(())
    }
}
