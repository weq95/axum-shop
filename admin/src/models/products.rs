use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::Row;

use common::error::{ApiError, ApiResult};

use crate::controller::products::ReqQueryProduct;
use crate::models::favorite_products::FavoriteProductsModel;
use crate::models::product_skus::ProductSkuModel;

#[derive(Debug, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct ProductModel {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub image: Json<Vec<String>>,
    pub on_sale: bool,
    pub rating: i64,
    pub sold_count: i64,
    pub review_count: i32,
    pub price: f64,
    pub skus: Vec<ProductSkuModel>,
}

impl ProductModel {
    /// 创建
    pub async fn create(product: ProductModel) -> ApiResult<u64> {
        let mut tx = common::pgsql::db().await.begin().await?;

        let product_sku = product
            .skus
            .iter()
            .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
            .unwrap();

        let id = sqlx::query(
            "insert into products (title, description, image, on_sale, sku_price) values ($1, $2, $3, $4, $5) RETURNING id",
        )
            .bind(&product.title.clone())
            .bind(&product.description.clone())
            .bind(product.image.clone())
            .bind(&product.on_sale.clone())
            .bind(product_sku.price)
            .fetch_one(&mut tx)
            .await?.get::<i64, _>("id");

        ProductSkuModel::delete_product_sku(id, &mut tx).await?;

        if false == ProductSkuModel::add_product_sku(id, &product.skus, &mut tx).await? {
            tx.rollback().await?;
            return Err(ApiError::Error("添加商品sku失败, 请稍后重试".to_string()));
        }

        //添加sku
        tx.commit().await?;
        Ok(id as u64)
    }

    /// 商品详情
    pub async fn get(product_id: i64) -> ApiResult<Self> {
        let mut result: ProductModel =
            sqlx::query("select * from products where id = $1 and on_sale = true")
                .bind(product_id)
                .fetch_optional(common::pgsql::db().await)
                .await?
                .map(|row| ProductModel {
                    id: row.get::<i64, _>("id"),
                    title: row.get("title"),
                    description: row.get("description"),
                    image: row.get::<Json<Vec<String>>, _>("image"),
                    on_sale: row.get::<bool, _>("on_sale"),
                    rating: row.get::<i64, _>("rating"),
                    sold_count: row.get::<i64, _>("sold_count"),
                    review_count: row.get::<i32, _>("review_count"),
                    price: row.get::<f64, _>("sku_price"),
                    skus: Vec::default(),
                })
                .ok_or(ApiError::Error("NotFound".to_string()))?;

        result.image_preview_url().await.skus().await?;

        Ok(result)
    }

    /// 列表
    pub async fn products(payload: ReqQueryProduct) -> ApiResult<(u64, Vec<ProductModel>)> {
        let mut sql_str = "select * from products where on_sale=true ".to_string();
        let mut count_str =
            "select count(*) as count from products where on_sale=true ".to_string();

        if let Some(title) = &payload.title {
            let str = format!(r#" and title::text like '{}%' "#, title);
            sql_str.push_str(&str);
            count_str.push_str(&str);
        }

        if let Some(order_by) = &payload.order_by {
            let (field, _type) = common::utils::regex_patch(r#"^(.+)_(asc|desc)$"#, &order_by)?;
            if &field != "" && _type != "" && Self::order_by_field(&field) {
                sql_str.push_str(&format!(" order by {} {}", field, _type));
            }
        }

        let page_size = payload.page_size.unwrap_or(15);
        let per_page = page_size * (payload.page_num.unwrap_or(1) - 1);

        sql_str.push_str(&format!(" limit {} offset {}", page_size, per_page));

        let mut result: Vec<Self> = sqlx::query(&*sql_str)
            .fetch_all(common::pgsql::db().await)
            .await?
            .into_iter()
            .map(|row| ProductModel {
                id: row.get::<i64, _>("id"),
                title: row.get("title"),
                description: row.get("description"),
                image: row.get::<Json<Vec<String>>, _>("image"),
                on_sale: row.get::<bool, _>("on_sale"),
                rating: row.get::<i64, _>("rating"),
                sold_count: row.get::<i64, _>("sold_count"),
                review_count: row.get::<i32, _>("review_count"),
                price: row.get::<f64, _>("sku_price"),
                skus: Vec::default(),
            })
            .collect::<Vec<Self>>();

        for product in result.iter_mut() {
            product.image_preview_url().await.skus().await?
        }
        let count = sqlx::query(&*count_str)
            .fetch_one(common::pgsql::db().await)
            .await?
            .get::<i64, _>("count");

        Ok((count as u64, result))
    }

    /// 更新
    pub async fn update(product: Self) -> ApiResult<bool> {
        let count =
            sqlx::query("select count(*) as count from products where title = $1 and id != $2")
                .bind(product.title.clone())
                .bind(product.id)
                .fetch_one(common::pgsql::db().await)
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
        let mut tx = common::pgsql::db().await.begin().await?;
        ProductSkuModel::delete_product_sku(product.id, &mut tx).await?;
        let row_bool = sqlx::query("update products set title = $1, description = $2, image = $3, on_sale = $4, sku_price = $5 where id = $6")
            .bind(product.title.clone())
            .bind(product.description.clone())
            .bind(product.image)
            .bind(product.on_sale)
            .bind(product_sku.price)
            .bind(product.id)
            .execute(&mut tx)
            .await?.rows_affected() == 1;

        if false == row_bool {
            tx.rollback().await?;

            return Err(ApiError::Error("商品信息修改失败, 请稍后重试".to_string()));
        }

        if false == ProductSkuModel::add_product_sku(product.id, &product.skus, &mut tx).await? {
            tx.rollback().await?;

            return Err(ApiError::Error("修改商品sku失败, 请稍后重试".to_string()));
        }

        tx.commit().await?;

        Ok(row_bool)
    }

    /// 删除
    pub async fn delete(product_id: u64) -> ApiResult<bool> {
        FavoriteProductsModel::un_favorite_product(product_id as i64).await?;
        let mut tx = common::pgsql::db().await.begin().await?;

        ProductSkuModel::delete_product_sku(product_id as i64, &mut tx).await?;

        let rows_num = sqlx::query("delete from products where id = $1")
            .bind(product_id as i64)
            .execute(&mut tx)
            .await?
            .rows_affected();

        Ok(rows_num > 0)
    }

    /// 商品sku
    pub async fn skus(&mut self) -> ApiResult<()> {
        let skus = ProductSkuModel::skus(vec![self.id]).await?;
        if let Some(values) = skus.get(&self.id) {
            for item in values {
                self.skus.push(ProductSkuModel {
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
    pub async fn image_preview_url(&mut self) -> &mut ProductModel {
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
                .fetch_one(common::pgsql::db().await)
                .await?
                .get::<bool, _>("exists"),
        )
    }
}
