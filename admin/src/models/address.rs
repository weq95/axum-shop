use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::json;
use sqlx::postgres::PgArguments;
use sqlx::types::{Json, JsonValue};
use sqlx::{Arguments, Row};
use validator::Validate;

use common::error::{ApiError, ApiResult};
use common::request::address::ReqAddressInfo;

#[derive(Debug, Default, Clone)]
pub struct UserAddress {
    pub id: i64,
    pub user_id: i64,
    pub province: i32,
    pub city: i32,
    pub district: i32,
    pub street: i32,
    pub address: String,
    pub zip: i32,
    pub contact_name: String,
    pub contact_phone: String,
    pub last_used_at: NaiveDateTime,
    pub crated_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl UserAddress {
    /// 用户收获地址详情
    pub async fn get(id: i64, userid: i64) -> ApiResult<UserAddress> {
        Ok(sqlx::query(
            r#"SELECT id,user_id,province,city,street,district,address,zip,contact_name,contact_phone,
    last_used_at,created_at,updated_at FROM user_address WHERE id = $1 and user_id = $2"#,
        )
            .bind(id)
            .bind(userid)
            .fetch_optional(common::pgsql::db().await)
            .await?
            .map(|row| UserAddress {
                id: row.get::<i64, &str>("id"),
                user_id: row.get::<i64, &str>("user_id"),
                province: row.get::<i32, &str>("province"),
                city: row.get::<i32, &str>("city"),
                district: row.get::<i32, &str>("district"),
                street: row.get::<i32, &str>("street"),
                address: row.get("address"),
                zip: row.get::<i32, &str>("zip"),
                contact_name: row.get("contact_name"),
                contact_phone: row.get("contact_phone"),
                last_used_at: row.get::<DateTime<Utc>, &str>("last_used_at").naive_local(),
                crated_at: row.get::<DateTime<Utc>, &str>("created_at").naive_local(),
                updated_at: row.get::<DateTime<Utc>, &str>("updated_at").naive_local(),
            })
            .unwrap_or(UserAddress::default()))
    }

    /// 用户收获地址列表
    pub async fn list(userid: i64) -> ApiResult<Vec<UserAddress>> {
        Ok(sqlx::query(r#"SELECT id,user_id,province,city,street,district,address,zip,contact_name,contact_phone,
    last_used_at,created_at,updated_at FROM user_address WHERE user_id = $1 order by last_used_at desc"#)
            .bind(userid)
            .fetch_all(common::pgsql::db().await).await?.into_iter().map(|row| {
            UserAddress {
                id: row.get::<i64, &str>("id"),
                user_id: row.get::<i64, &str>("user_id"),
                province: row.get::<i32, &str>("province"),
                city: row.get::<i32, &str>("city"),
                district: row.get::<i32, &str>("district"),
                street: row.get::<i32, &str>("street"),
                address: row.get("address"),
                zip: row.get::<i32, &str>("zip"),
                contact_name: row.get("contact_name"),
                contact_phone: row.get("contact_phone"),
                last_used_at: row.get::<DateTime<Utc>, &str>("last_used_at").naive_local(),
                crated_at: row.get::<DateTime<Utc>, &str>("created_at").naive_local(),
                updated_at: row.get::<DateTime<Utc>, &str>("updated_at").naive_local(),
            }
        }).collect::<Vec<UserAddress>>())
    }

    /// 用户创建收获地址
    pub async fn create(userid: i64, info: ReqAddressInfo) -> ApiResult<i64> {
        info.validate()?;

        let count =
            sqlx::query(r#"SELECT COUNT("id") AS count  FROM "user_address" WHERE user_id = $1"#)
                .bind(userid)
                .fetch_one(common::pgsql::db().await)
                .await?
                .get::<i64, &str>("count");
        if count >= 5 {
            return Err(ApiError::Error(
                "收获地址太多啦, 请尝试修改其他收获地址使用".to_string(),
            ));
        }

        let phone = &info.contact_phone.unwrap()[3..].to_string();
        let id: i64 = sqlx::query("insert into user_address (user_id,province,city,district,address,street,zip,\
    contact_name,contact_phone,created_at,last_used_at,updated_at) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id")
            .bind(userid)
            .bind(info.province)
            .bind(info.city)
            .bind(info.district)
            .bind(&info.address)
            .bind(info.street)
            .bind(info.zip)
            .bind(&info.contact_name)
            .bind(phone)
            .bind(Utc::now())
            .bind(Utc::now())
            .bind(Utc::now())
            .fetch_one(common::pgsql::db().await)
            .await?.get::<i64, &str>("id");

        Ok(id)
    }

    /// 用户更新收获地址信息
    pub async fn update(id: i64, userid: i64, info: ReqAddressInfo) -> ApiResult<bool> {
        info.validate()?;
        let rows_num = sqlx::query("update user_address set province=$1,city=$2,district=$3,street=$4,\
    address=$5,zip=$6,contact_name=$7,contact_phone=$8,updated_at=$9 where id = $10 and user_id = $11")
            .bind(info.province)
            .bind(info.city)
            .bind(info.district)
            .bind(info.street)
            .bind(&info.address)
            .bind(info.zip)
            .bind(&info.contact_name)
            .bind(&info.contact_phone)
            .bind(Utc::now())
            .bind(id).bind(userid)
            .execute(common::pgsql::db().await)
            .await?.rows_affected();

        Ok(rows_num > 0)
    }

    /// 用户删除收获地址
    pub async fn delete(id: i64, user_id: i64) -> ApiResult<bool> {
        let rows_num = sqlx::query("delete from user_address where id = $1 and user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(common::pgsql::db().await)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }

    /// 更新收获地址最后一次使用信息
    pub async fn update_last_used_at(&self) -> ApiResult<bool> {
        let rows_num = sqlx::query(
            "update user_address set last_used_at = $1, \
        updated_at = $2 where id = $3 and user_id = $4",
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(self.id)
        .bind(self.user_id)
        .execute(common::pgsql::db().await)
        .await?
        .rows_affected();

        Ok(rows_num > 0)
    }

    // 订单收获地址
    pub async fn harvest_addr(id: i64) -> ApiResult<HashMap<String, serde_json::Value>> {
        let info = sqlx::query(
            "select province,city,district,address,zip,contact_name,contact_phone where id = #1",
        )
        .bind(id)
        .fetch_one(common::pgsql::db().await)
        .await
        .map(|row| UserAddress {
            province: row.get::<i32, _>("province"),
            city: row.get::<i32, _>("city"),
            district: row.get::<i32, _>("province"),
            address: row.get("address"),
            zip: row.get::<i32, _>("zip"),
            contact_name: row.get("contact_name"),
            contact_phone: row.get("contact_phone"),
            ..UserAddress::default()
        })
        .map_err(|err| ApiError::Error(err.to_string()))?;
        let addr_map = get_addr_name(HashSet::from([
            info.province,
            info.city,
            info.district,
            info.street,
        ]))
        .await?;

        let p_name = addr_map.get(&info.province).unwrap().name.clone();
        let c_name = addr_map.get(&info.city).unwrap().name.clone();
        let d_name = addr_map.get(&info.district).unwrap().name.clone();

        Ok(HashMap::from([
            (
                "name".to_string(),
                serde_json::to_value(&info.contact_name).unwrap(),
            ),
            (
                "phone".to_string(),
                serde_json::to_value(&info.contact_phone).unwrap(),
            ),
            ("zip".to_string(), serde_json::to_value(&info.zip).unwrap()),
            (
                "province".to_string(),
                serde_json::to_value(p_name).unwrap(),
            ),
            ("city".to_string(), serde_json::to_value(c_name).unwrap()),
            (
                "district".to_string(),
                serde_json::to_value(d_name).unwrap(),
            ),
            (
                "address".to_string(),
                serde_json::to_value(&info.address).unwrap(),
            ),
        ]))
    }
}

#[derive(Debug, Clone, Default)]
pub struct AddrData {
    pub id: i32,
    pub pid: i32,
    pub name: String,
}

/// 获取收获地址
pub async fn addr_result(pid: i32) -> ApiResult<Vec<AddrData>> {
    Ok(
        sqlx::query("select id,name,pid from address where pid = $1 order by id asc")
            .bind(pid)
            .fetch_all(common::pgsql::db().await)
            .await?
            .into_iter()
            .map(|row| AddrData {
                id: row.get::<i64, &str>("id") as i32,
                name: row.get("name"),
                pid: row.get::<i64, &str>("pid") as i32,
            })
            .collect::<Vec<AddrData>>(),
    )
}

pub async fn get_addr_name(ids: HashSet<i32>) -> ApiResult<HashMap<i32, AddrData>> {
    let mut arg = PgArguments::default();
    let mut placeholder = String::with_capacity(ids.len());
    let mut idx = 0;
    for id in ids {
        arg.add(id);
        idx += 1;
        placeholder.push_str(&*("$".to_owned() + idx.to_string().as_str() + ","));
    }

    let placeholder = placeholder.trim_matches(',');

    let address: Vec<AddrData> = sqlx::query_with(
        &*("select id,name,pid from address where id in (".to_owned() + placeholder + ")"),
        arg,
    )
    .fetch_all(common::pgsql::db().await)
    .await?
    .into_iter()
    .map(|row| AddrData {
        id: row.get::<i64, &str>("id") as i32,
        name: row.get::<String, &str>("name"),
        pid: row.get::<i64, &str>("pid") as i32,
    })
    .collect::<Vec<AddrData>>();

    let mut result: HashMap<i32, AddrData> = HashMap::with_capacity(address.len());
    for item in address {
        result.insert(item.id, item);
    }

    Ok(result)
}
