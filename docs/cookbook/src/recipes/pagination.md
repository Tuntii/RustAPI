# Pagination & HATEOAS

Implementing pagination correctly is crucial for API performance and usability. RustAPI provides built-in support for HATEOAS (Hypermedia As The Engine Of Application State) compliant pagination, which includes navigation links in the response.

## Problem

You need to return a list of resources, but there are too many to return in a single request. You want to provide a standard way for clients to navigate through pages of data.

## Solution

Use `ResourceCollection` and `PageInfo` from `rustapi_core::hateoas`. These types automatically generate HAL (Hypertext Application Language) compliant responses with `_links` (self, first, last, next, prev) and `_embedded` resources.

## Example Code

```rust
use rustapi_rs::prelude::*;
use rustapi_rs::{PageInfo, ResourceCollection};
use serde::{Deserialize, Serialize};

// 1. Define your resource
// Note: It must derive Schema for OpenAPI generation
#[derive(Serialize, Clone, Schema)]
struct User {
    id: i64,
    name: String,
}

// 2. Define query parameters
#[derive(Deserialize, Schema)]
struct Pagination {
    page: Option<usize>,
    size: Option<usize>,
}

// 3. Create the handler
#[rustapi_rs::get("/users")]
async fn list_users(Query(params): Query<Pagination>) -> Json<ResourceCollection<User>> {
    let page = params.page.unwrap_or(0);
    let size = params.size.unwrap_or(20);

    // In a real app, you would fetch this from a database
    // let (users, total_elements) = db.fetch_users(page, size).await?;
    let users = vec![
        User { id: 1, name: "Alice".to_string() },
        User { id: 2, name: "Bob".to_string() },
    ];
    let total_elements = 100;

    // 4. Calculate pagination info
    let page_info = PageInfo::calculate(total_elements, size, page);

    // 5. Build the collection response
    // "users" is the key in the _embedded map
    // "/users" is the base URL for generating links
    let collection = ResourceCollection::new("users", users)
        .page_info(page_info)
        .with_pagination("/users");

    Json(collection)
}
```

## Explanation

The response will look like this (HAL format):

```json
{
  "_embedded": {
    "users": [
      { "id": 1, "name": "Alice" },
      { "id": 2, "name": "Bob" }
    ]
  },
  "_links": {
    "self": { "href": "/users?page=0&size=20" },
    "first": { "href": "/users?page=0&size=20" },
    "last": { "href": "/users?page=4&size=20" },
    "next": { "href": "/users?page=1&size=20" }
  },
  "page": {
    "size": 20,
    "totalElements": 100,
    "totalPages": 5,
    "number": 0
  }
}
```

### Key Components

1.  **`ResourceCollection<T>`**: Wraps a list of items. It places them under `_embedded` and adds `_links`.
2.  **`PageInfo`**: Holds metadata about the current page (size, total elements, total pages, current number).
3.  **`with_pagination(base_url)`**: Automatically generates standard navigation links based on the `PageInfo` and the provided base URL.

## Variations

### Cursor-based Pagination

If you are using cursor-based pagination (e.g., `before_id`, `after_id`), you can manually construct links instead of using `with_pagination`:

```rust
let collection = ResourceCollection::new("users", users)
    .self_link("/users?after=10")
    .next_link("/users?after=20");
```

### HATEOAS for Single Resources

You can also add links to individual resources using `Resource<T>`:

```rust
use rustapi_rs::hateoas::Linkable; // Trait for .with_links()

#[rustapi_rs::get("/users/{id}")]
async fn get_user(Path(id): Path<i64>) -> Json<Resource<User>> {
    let user = User { id, name: "Alice".to_string() };

    let resource = user.with_links()
        .self_link(format!("/users/{}", id))
        .link("orders", format!("/users/{}/orders", id));

    Json(resource)
}
```

## Gotchas

*   **Schema Derive**: The type `T` inside `ResourceCollection<T>` or `Resource<T>` MUST implement `RustApiSchema` (via `#[derive(Schema)]`) for OpenAPI generation to work.
*   **Base URL**: The `base_url` passed to `with_pagination` should generally match the route path. If your API is behind a proxy or prefix, ensure this URL is correct from the client's perspective.
