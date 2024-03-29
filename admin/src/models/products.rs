use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::types::PgMoney;
use sqlx::types::Json;
use sqlx::Row;

use common::error::{ApiError, ApiResult};
use common::Pagination;

use crate::models::crowdfunding::CrowdfundingProduct;
use crate::models::favorite_products::FavoriteProducts;
use crate::models::product_property::ProductProperty;
use crate::models::product_skus::ProductSku;
use crate::models::products::PType::Crowdfunding;

#[derive(Debug, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct Product {
    pub id: i64,
    pub title: String,
    pub long_title: String,
    pub description: String,
    pub image: Json<Vec<String>>,
    pub on_sale: bool,
    pub rating: i64,
    pub sold_count: i64,
    pub review_count: i32,
    pub price: f64,
    pub skus: Vec<ProductSku>,
    pub property: Vec<ProductProperty>,
    pub category_id: i64,
    pub r#type: PType,
}

#[repr(i16)]
#[derive(Debug, Serialize, Deserialize, PartialEq, sqlx::Type)]
pub enum PType {
    Normal = 1,
    Crowdfunding = 2,
}

impl Default for PType {
    fn default() -> Self {
        Self::Normal
    }
}

impl Product {
    /// 创建
    pub async fn create(
        product: Product,
        target_amount: PgMoney,
        end_at: Option<chrono::NaiveDateTime>,
    ) -> ApiResult<u64> {
        let mut tx = common::postgres().await.begin().await?;

        let product_sku = product
            .skus
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
            .unwrap();

        let id = sqlx::query(
            "insert into products (title,description,image,on_sale,sku_price,category_id,long_title) \
            values ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
            .bind(&product.title.clone())
            .bind(&product.description.clone())
            .bind(product.image.clone())
            .bind(&product.on_sale.clone())
            .bind(product_sku.price)
            .bind(product.category_id)
            .bind(product.long_title.clone())
            .fetch_one(&mut tx)
            .await?
            .get::<i64, _>("id");

        ProductSku::delete_product_sku(id, &mut tx).await?;

        if false == ProductSku::add_product_sku(id, &product.skus, &mut tx).await? {
            tx.rollback().await?;
            return Err(ApiError::Error("添加商品sku失败, 请稍后重试".to_string()));
        }

        if product.r#type == Crowdfunding {
            CrowdfundingProduct::store(id, target_amount, end_at.unwrap(), &mut tx).await?;
        }

        let property = product
            .property
            .iter()
            .map(|i| (i.name.as_str(), i.value.as_str()))
            .collect::<Vec<(&str, &str)>>();
        ProductProperty::create(id, property, &mut tx).await?;
        //添加sku
        tx.commit().await?;
        Ok(id as u64)
    }

    /// 商品详情
    pub async fn get(product_id: i64) -> ApiResult<Self> {
        let mut result: Product =
            sqlx::query("select * from products where id = $1 and on_sale = true")
                .bind(product_id)
                .fetch_optional(common::postgres().await)
                .await?
                .map(|row| Product {
                    id: row.get::<i64, _>("id"),
                    title: row.get("title"),
                    long_title: row.get("long_title"),
                    description: row.get("description"),
                    image: row.get::<Json<Vec<String>>, _>("image"),
                    on_sale: row.get::<bool, _>("on_sale"),
                    rating: row.get::<i64, _>("rating"),
                    sold_count: row.get::<i64, _>("sold_count"),
                    review_count: row.get::<i32, _>("review_count"),
                    price: row.get::<f64, _>("sku_price"),
                    category_id: row.get::<i64, _>("category_id"),
                    skus: Vec::default(),
                    property: Vec::default(),
                    r#type: row.get::<PType, _>("type"),
                })
                .ok_or(ApiError::Error("NotFound".to_string()))?;

        result.image_preview_url().await.skus().await?;

        Ok(result)
    }

    /// 列表
    pub async fn products(
        payload: HashMap<String, String>,
        pagination: &mut Pagination<Product>,
    ) -> ApiResult<()> {
        let mut sql_str = "select * from products where on_sale=true ".to_string();
        let mut count_str =
            "select count(*) as count from products where on_sale=true ".to_string();

        if let Some(title) = payload.get("title") {
            let str = format!(r#" and title::text like '{}%' "#, title);
            sql_str.push_str(&str);
            count_str.push_str(&str);
        }

        if let Some(order_by) = payload.get("order_by") {
            let (field, _type) = common::utils::regex_patch(r#"^(.+)_(asc|desc)$"#, &order_by)?;
            if &field != "" && _type != "" && Self::order_by_field(&field) {
                sql_str.push_str(&format!(" order by {} {}", field, _type));
            }
        }

        if let Some(cid) = payload.get("category_id") {
            let category_id = cid.parse::<i64>().unwrap_or(0);
            if category_id > 0 {
                let sql = format!(" and category_id in (SELECT id FROM categories  WHERE id = {} or path like '%{}%' and deleted_at is NULL)",
                                  category_id, category_id);
                sql_str.push_str(&sql);
                count_str.push_str(&sql);
            }
        }

        sql_str.push_str(&format!(
            " limit {} offset {}",
            pagination.limit(),
            pagination.offset()
        ));

        let mut result: Vec<Self> = sqlx::query(&*sql_str)
            .fetch_all(common::postgres().await)
            .await?
            .into_iter()
            .map(|row| Product {
                id: row.get::<i64, _>("id"),
                title: row.get("title"),
                long_title: row.get("long_title"),
                description: row.get("description"),
                image: row.get::<Json<Vec<String>>, _>("image"),
                on_sale: row.get::<bool, _>("on_sale"),
                rating: row.get::<i64, _>("rating"),
                sold_count: row.get::<i64, _>("sold_count"),
                review_count: row.get::<i32, _>("review_count"),
                price: row.get::<f64, _>("sku_price"),
                category_id: row.get::<i64, _>("category_id"),
                skus: Vec::default(),
                property: Vec::default(),
                r#type: row.get::<PType, _>("type"),
            })
            .collect::<Vec<Self>>();

        for product in result.iter_mut() {
            product.image_preview_url().await;
        }
        let count = sqlx::query(&*count_str)
            .fetch_one(common::postgres().await)
            .await?
            .get::<i64, _>("count") as usize;
        pagination.set_total(count);
        pagination.set_data(result);

        Ok(())
    }

    /// 更新
    pub async fn update(product: Self) -> ApiResult<bool> {
        let count =
            sqlx::query("select count(*) as count from products where title = $1 and id != $2")
                .bind(product.title.clone())
                .bind(product.id)
                .fetch_one(common::postgres().await)
                .await?
                .get::<i64, _>("count");
        if count > 0 {
            return Err(ApiError::Error("修改失败，商品名称重复".to_string()));
        }

        let product_sku = product
            .skus
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
            .unwrap();
        let mut tx = common::postgres().await.begin().await?;
        ProductSku::delete_product_sku(product.id, &mut tx).await?;
        let row_bool = sqlx::query("update products set title = $1, description = $2, image = $3, on_sale = $4, sku_price = $5, long_title = $6 where id = $7")
            .bind(product.title.clone())
            .bind(product.description.clone())
            .bind(product.image)
            .bind(product.on_sale)
            .bind(product_sku.price)
            .bind(product.long_title.clone())
            .bind(product.id)
            .execute(&mut tx)
            .await?.rows_affected() == 1;

        if false == row_bool {
            tx.rollback().await?;

            return Err(ApiError::Error("商品信息修改失败, 请稍后重试".to_string()));
        }

        if false == ProductSku::add_product_sku(product.id, &product.skus, &mut tx).await? {
            tx.rollback().await?;

            return Err(ApiError::Error("修改商品sku失败, 请稍后重试".to_string()));
        }

        tx.commit().await?;

        Ok(row_bool)
    }

    /// 删除
    pub async fn delete(product_id: u64) -> ApiResult<bool> {
        FavoriteProducts::un_favorite_product(product_id as i64).await?;
        let mut tx = common::postgres().await.begin().await?;

        ProductSku::delete_product_sku(product_id as i64, &mut tx).await?;

        let rows_num = sqlx::query("delete from products where id = $1")
            .bind(product_id as i64)
            .execute(&mut tx)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }

    /// 商品sku
    pub async fn skus(&mut self) -> ApiResult<()> {
        let skus = ProductSku::skus(vec![self.id]).await?;
        if let Some(values) = skus.get(&self.id) {
            for item in values {
                self.skus.push(ProductSku {
                    id: item.id,
                    title: item.title.clone(),
                    description: item.description.clone(),
                    price: item.price,
                    stock: item.stock,
                    product_id: item.product_id,
                })
            }

            return Ok(());
        }

        self.skus = Vec::new();
        Ok(())
    }

    /// 处理图片URL
    pub async fn image_preview_url(&mut self) -> &mut Product {
        let mut image_arr: Vec<String> = Vec::new();
        for path in self.image.0.clone() {
            image_arr.push(common::image_preview_url(path.clone()).await.1)
        }

        self.image = Json(image_arr);

        self
    }

    /// 检测字段是否存在
    fn order_by_field(field: &str) -> bool {
        let fields: [&str; 3] = ["sku_price", "sold_count", "rating"];
        for i in fields {
            if i == field {
                return true;
            }
        }

        false
    }

    /// 检测商品是否存在
    pub async fn unique_title(title: &str) -> ApiResult<bool> {
        Ok(
            sqlx::query("select exists (select id from products where title = $1)")
                .bind(title)
                .fetch_one(common::postgres().await)
                .await?
                .get::<bool, _>("exists"),
        )
    }

    // 获取商品信息
    pub async fn product_maps(ids: Vec<i64>) -> ApiResult<HashMap<i64, serde_json::Value>> {
        Ok(
            sqlx::query("select id, title,sku_price from products where id = any($1)")
                .bind(ids)
                .fetch_all(common::postgres().await)
                .await?
                .into_iter()
                .map(|row| {
                    json!({
                        "id": row.get::<i64, _>("id"),
                        "title": row.get::<String, _>("title"),
                        "sku_price": row.get::<f64, _>("sku_price"),
                    })
                })
                .map(|row| (row.get("id").unwrap().as_i64().unwrap(), row))
                .collect::<HashMap<i64, serde_json::Value>>(),
        )
    }

    pub async fn crowdfunding(&self) -> ApiResult<Option<CrowdfundingProduct>> {
        Ok(
            sqlx::query("select * from crowdfunding_products where product_id = $1")
                .bind(self.id)
                .fetch_optional(&*common::postgres().await)
                .await?
                .map(|row| CrowdfundingProduct {
                    id: row.get::<i64, _>("id"),
                    product_id: row.get::<i64, _>("product_id"),
                    target_amount: row.get::<sqlx::postgres::types::PgMoney, _>("target_amount"),
                    total_amount: row.get::<sqlx::postgres::types::PgMoney, _>("total_amount"),
                    user_count: row.get::<i32, _>("user_count"),
                    end_at: row.get::<chrono::NaiveDateTime, _>("end_at"),
                    status: row.get::<crate::models::crowdfunding::Status, _>("status"),
                }),
        )
    }
}
