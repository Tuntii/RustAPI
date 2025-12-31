//! Category handlers

use chrono::Utc;
use rustapi_rs::prelude::*;
use std::sync::Arc;

use crate::models::{
    Category, CategoryResponse, Claims, CreateCategoryRequest, UpdateCategoryRequest,
};
use crate::stores::AppState;

/// List categories
async fn list_categories(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
) -> Json<crate::models::CategoryListResponse> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);
    let categories = state.categories.find_by_user(user_id).await;

    let responses: Vec<crate::models::CategoryResponse> = categories
        .iter()
        .map(crate::models::CategoryResponse::from)
        .collect();

    Json(crate::models::CategoryListResponse {
        categories: responses,
    })
}

/// Create a new category
async fn create_category(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    ValidatedJson(body): ValidatedJson<CreateCategoryRequest>,
) -> Result<Created<CategoryResponse>, ApiError> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);

    // Check if category name already exists for this user
    if state
        .categories
        .find_by_name(user_id, &body.name)
        .await
        .is_some()
    {
        return Err(ApiError::bad_request("Category name already exists"));
    }

    let category = Category {
        id: 0, // Will be set by store
        user_id,
        name: body.name,
        color: body.color,
        created_at: Utc::now(),
    };

    let created = state.categories.create(category).await;

    Ok(Created(CategoryResponse::from(&created)))
}

/// Update a category
async fn update_category(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    Path(id): Path<u64>,
    ValidatedJson(body): ValidatedJson<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, ApiError> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);

    let mut category = state
        .categories
        .find_by_id(id)
        .await
        .ok_or_else(|| ApiError::not_found("Category not found"))?;

    // Check ownership
    if category.user_id != user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    // Apply updates
    if let Some(name) = body.name {
        // Check if new name already exists
        if let Some(existing) = state.categories.find_by_name(user_id, &name).await {
            if existing.id != id {
                return Err(ApiError::bad_request("Category name already exists"));
            }
        }
        category.name = name;
    }
    if body.color.is_some() {
        category.color = body.color;
    }

    let updated = state
        .categories
        .update(id, category)
        .await
        .ok_or_else(|| ApiError::not_found("Category not found"))?;

    Ok(Json(CategoryResponse::from(&updated)))
}

/// Delete a category
async fn delete_category(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    Path(id): Path<u64>,
) -> Result<NoContent, ApiError> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);

    let category = state
        .categories
        .find_by_id(id)
        .await
        .ok_or_else(|| ApiError::not_found("Category not found"))?;

    // Check ownership
    if category.user_id != user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    // Clear category from all bookmarks
    state.bookmarks.clear_category(id).await;

    // Delete the category
    state.categories.delete(id).await;

    Ok(NoContent)
}

// Route functions
pub fn list_categories_route() -> Route {
    get_route("/categories", list_categories)
}

pub fn create_category_route() -> Route {
    post_route("/categories", create_category)
}

pub fn update_category_route() -> Route {
    put_route("/categories/{id}", update_category)
}

pub fn delete_category_route() -> Route {
    delete_route("/categories/{id}", delete_category)
}
