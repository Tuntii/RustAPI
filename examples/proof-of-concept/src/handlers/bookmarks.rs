//! Bookmark CRUD handlers

use chrono::Utc;
use rustapi_rs::prelude::*;
use std::sync::Arc;

use crate::models::{
    Bookmark, BookmarkEvent, BookmarkListParams, BookmarkResponse, Claims,
    CreateBookmarkRequest, PaginatedResponse, UpdateBookmarkRequest,
};
use crate::stores::AppState;

/// List bookmarks with pagination and filtering
async fn list_bookmarks(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    Query(params): Query<BookmarkListParams>,
) -> Json<PaginatedResponse<BookmarkResponse>> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);
    let mut bookmarks = state.bookmarks.find_by_user(user_id).await;

    // Apply filters
    if let Some(category_id) = params.category_id {
        bookmarks.retain(|b| b.category_id == Some(category_id));
    }

    if let Some(is_favorite) = params.is_favorite {
        bookmarks.retain(|b| b.is_favorite == is_favorite);
    }

    if let Some(ref search) = params.search {
        let search_lower = search.to_lowercase();
        bookmarks.retain(|b| {
            b.title.to_lowercase().contains(&search_lower)
                || b.description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
        });
    }

    // Sort by created_at descending
    bookmarks.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Pagination
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let total = bookmarks.len();
    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

    let start = ((page - 1) * limit) as usize;
    let items: Vec<BookmarkResponse> = bookmarks
        .into_iter()
        .skip(start)
        .take(limit as usize)
        .map(|b| BookmarkResponse::from(&b))
        .collect();

    Json(PaginatedResponse {
        items,
        total,
        page,
        limit,
        total_pages,
    })
}

/// Create a new bookmark
async fn create_bookmark(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    ValidatedJson(body): ValidatedJson<CreateBookmarkRequest>,
) -> Created<BookmarkResponse> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);
    let now = Utc::now();

    let bookmark = Bookmark {
        id: 0, // Will be set by store
        user_id,
        url: body.url,
        title: body.title,
        description: body.description,
        category_id: body.category_id,
        is_favorite: body.is_favorite.unwrap_or(false),
        created_at: now,
        updated_at: now,
    };

    let created = state.bookmarks.create(bookmark).await;
    let response = BookmarkResponse::from(&created);

    // Broadcast SSE event
    state.sse_broadcaster.broadcast(BookmarkEvent::Created {
        bookmark: response.clone(),
    });

    Created(response)
}


/// Get a single bookmark by ID
async fn get_bookmark(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    Path(id): Path<u64>,
) -> Result<Json<BookmarkResponse>, ApiError> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);

    let bookmark = state
        .bookmarks
        .find_by_id(id)
        .await
        .ok_or_else(|| ApiError::not_found("Bookmark not found"))?;

    // Check ownership
    if bookmark.user_id != user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    Ok(Json(BookmarkResponse::from(&bookmark)))
}

/// Update a bookmark
async fn update_bookmark(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    Path(id): Path<u64>,
    ValidatedJson(body): ValidatedJson<UpdateBookmarkRequest>,
) -> Result<Json<BookmarkResponse>, ApiError> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);

    let mut bookmark = state
        .bookmarks
        .find_by_id(id)
        .await
        .ok_or_else(|| ApiError::not_found("Bookmark not found"))?;

    // Check ownership
    if bookmark.user_id != user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    // Apply updates
    if let Some(url) = body.url {
        bookmark.url = url;
    }
    if let Some(title) = body.title {
        bookmark.title = title;
    }
    if body.description.is_some() {
        bookmark.description = body.description;
    }
    if body.category_id.is_some() {
        bookmark.category_id = body.category_id;
    }
    if let Some(is_favorite) = body.is_favorite {
        bookmark.is_favorite = is_favorite;
    }
    bookmark.updated_at = Utc::now();

    let updated = state
        .bookmarks
        .update(id, bookmark)
        .await
        .ok_or_else(|| ApiError::not_found("Bookmark not found"))?;

    let response = BookmarkResponse::from(&updated);

    // Broadcast SSE event
    state.sse_broadcaster.broadcast(BookmarkEvent::Updated {
        bookmark: response.clone(),
    });

    Ok(Json(response))
}

/// Delete a bookmark
async fn delete_bookmark(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    Path(id): Path<u64>,
) -> Result<NoContent, ApiError> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);

    let bookmark = state
        .bookmarks
        .find_by_id(id)
        .await
        .ok_or_else(|| ApiError::not_found("Bookmark not found"))?;

    // Check ownership
    if bookmark.user_id != user_id {
        return Err(ApiError::forbidden("Access denied"));
    }

    state.bookmarks.delete(id).await;

    // Broadcast SSE event
    state.sse_broadcaster.broadcast(BookmarkEvent::Deleted { id });

    Ok(NoContent)
}

/// Export bookmarks as JSON
async fn export_bookmarks(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
) -> Json<crate::models::ExportResponse> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);
    let bookmarks = state.bookmarks.find_by_user(user_id).await;
    let categories = state.categories.find_by_user(user_id).await;

    let exports: Vec<crate::models::BookmarkExport> = bookmarks
        .into_iter()
        .map(|b| {
            let category_name = b.category_id.and_then(|cid| {
                categories.iter().find(|c| c.id == cid).map(|c| c.name.clone())
            });

            crate::models::BookmarkExport {
                url: b.url,
                title: b.title,
                description: b.description,
                is_favorite: b.is_favorite,
                category_name,
            }
        })
        .collect();

    Json(crate::models::ExportResponse { bookmarks: exports })
}

/// Import bookmarks from JSON
async fn import_bookmarks(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser<Claims>,
    Json(body): Json<crate::models::ImportBookmarksRequest>,
) -> Json<crate::models::ImportResponse> {
    let user_id: u64 = claims.sub.parse().unwrap_or(0);
    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();

    for (idx, export) in body.bookmarks.into_iter().enumerate() {
        // Validate URL
        if export.url.is_empty() {
            errors.push(format!("Item {}: URL is required", idx));
            skipped += 1;
            continue;
        }

        // Validate title
        if export.title.is_empty() || export.title.len() > 200 {
            errors.push(format!("Item {}: Title must be 1-200 characters", idx));
            skipped += 1;
            continue;
        }

        // Find or create category
        let category_id = if let Some(ref name) = export.category_name {
            state.categories.find_by_name(user_id, name).await.map(|c| c.id)
        } else {
            None
        };

        let now = Utc::now();
        let bookmark = Bookmark {
            id: 0,
            user_id,
            url: export.url,
            title: export.title,
            description: export.description,
            category_id,
            is_favorite: export.is_favorite,
            created_at: now,
            updated_at: now,
        };

        state.bookmarks.create(bookmark).await;
        imported += 1;
    }

    Json(crate::models::ImportResponse {
        imported,
        skipped,
        errors,
    })
}


// Route functions
pub fn list_bookmarks_route() -> Route {
    get_route("/bookmarks", list_bookmarks)
}

pub fn create_bookmark_route() -> Route {
    post_route("/bookmarks", create_bookmark)
}

pub fn get_bookmark_route() -> Route {
    get_route("/bookmarks/{id}", get_bookmark)
}

pub fn update_bookmark_route() -> Route {
    put_route("/bookmarks/{id}", update_bookmark)
}

pub fn delete_bookmark_route() -> Route {
    delete_route("/bookmarks/{id}", delete_bookmark)
}

pub fn export_bookmarks_route() -> Route {
    get_route("/bookmarks/export", export_bookmarks)
}

pub fn import_bookmarks_route() -> Route {
    post_route("/bookmarks/import", import_bookmarks)
}
