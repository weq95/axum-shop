use std::collections::HashMap;

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use validator::Validate;

use common::categories::ReqCategories;
use common::error::format_errors;
use common::ApiResponse;

use crate::models::categories::Categories;

pub struct CategoriesController;

impl CategoriesController {
    pub async fn index(Query(inner): Query<HashMap<String, String>>) -> impl IntoResponse {
        let mut category_id = 0;
        if let Some(i) = inner.get("category_id") {
            category_id = i.parse::<i64>().unwrap_or(0);
        }

        match Categories::index(category_id).await {
            Ok(data) => ApiResponse::response(Some(data)).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }

    pub async fn create(Json(inner): Json<ReqCategories>) -> impl IntoResponse {
        if let Err(e) = inner.validate() {
            return ApiResponse::success_code_data(common::FAIL, Some(json!(format_errors(e))))
                .json();
        }

        let category = Categories {
            parent_id: inner.parent_id.unwrap(),
            name: inner.name.unwrap().clone(),
            ..Categories::default()
        };
        match category.store().await {
            Ok(id) => ApiResponse::response(Some(json!({ "id": id }))).json(),
            Err(e) => ApiResponse::fail_msg(e.to_string()).json(),
        }
    }
}
