//! HATEOAS (Hypermedia As The Engine Of Application State) support
//!
//! This module provides hypermedia link support for REST APIs following
//! the HAL (Hypertext Application Language) specification.
//!
//! # Overview
//!
//! HATEOAS enables REST APIs to provide navigation links in responses,
//! making APIs more discoverable and self-documenting.
//!
//! # Example
//!
//! ```rust,ignore
//! use rustapi_core::hateoas::{Resource, Link};
//!
//! #[derive(Serialize)]
//! struct User {
//!     id: i64,
//!     name: String,
//! }
//!
//! async fn get_user(Path(id): Path<i64>) -> Json<Resource<User>> {
//!     let user = User { id, name: "John".to_string() };
//!     
//!     Json(Resource::new(user)
//!         .self_link(&format!("/users/{}", id))
//!         .link("orders", &format!("/users/{}/orders", id))
//!         .link("profile", &format!("/users/{}/profile", id)))
//! }
//! ```

use rustapi_openapi::Schema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A hypermedia link following HAL specification
///
/// Links provide navigation between related resources.
///
/// # Example
/// ```rust
/// use rustapi_core::hateoas::Link;
///
/// let link = Link::new("/users/123")
///     .title("User details")
///     .set_templated(false);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Schema)]
pub struct Link {
    /// The URI of the linked resource
    pub href: String,

    /// Whether the href is a URI template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templated: Option<bool>,

    /// Human-readable title for the link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Media type hint for the linked resource
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,

    /// URI indicating the link is deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<String>,

    /// Name for differentiating links with the same relation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// URI of a profile document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Content-Language of the linked resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hreflang: Option<String>,
}

impl Link {
    /// Create a new link with the given href
    pub fn new(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            templated: None,
            title: None,
            media_type: None,
            deprecation: None,
            name: None,
            profile: None,
            hreflang: None,
        }
    }

    /// Create a templated link (URI template)
    ///
    /// # Example
    /// ```rust
    /// use rustapi_core::hateoas::Link;
    ///
    /// let link = Link::templated("/users/{id}");
    /// ```
    pub fn templated(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            templated: Some(true),
            ..Self::new("")
        }
    }

    /// Set whether this link is templated
    pub fn set_templated(mut self, templated: bool) -> Self {
        self.templated = Some(templated);
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the media type
    pub fn media_type(mut self, media_type: impl Into<String>) -> Self {
        self.media_type = Some(media_type.into());
        self
    }

    /// Mark as deprecated
    pub fn deprecation(mut self, deprecation_url: impl Into<String>) -> Self {
        self.deprecation = Some(deprecation_url.into());
        self
    }

    /// Set the name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the profile
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Set the hreflang
    pub fn hreflang(mut self, hreflang: impl Into<String>) -> Self {
        self.hreflang = Some(hreflang.into());
        self
    }
}

/// Resource wrapper with HATEOAS links (HAL format)
///
/// Wraps any data type with `_links` and optional `_embedded` sections.
///
/// # Example
/// ```rust
/// use rustapi_core::hateoas::Resource;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "John".to_string() };
/// let resource = Resource::new(user)
///     .self_link("/users/1")
///     .link("orders", "/users/1/orders");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct Resource<T: rustapi_openapi::schema::RustApiSchema> {
    /// The actual resource data (flattened into the JSON)
    #[serde(flatten)]
    pub data: T,

    /// Hypermedia links
    #[serde(rename = "_links")]
    pub links: HashMap<String, LinkOrArray>,

    /// Embedded resources
    #[serde(rename = "_embedded", skip_serializing_if = "Option::is_none")]
    pub embedded: Option<HashMap<String, serde_json::Value>>,
}

/// Either a single link or an array of links
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Schema)]
#[serde(untagged)]
pub enum LinkOrArray {
    /// Single link
    Single(Link),
    /// Array of links (for multiple links with same relation)
    Array(Vec<Link>),
}

impl From<Link> for LinkOrArray {
    fn from(link: Link) -> Self {
        LinkOrArray::Single(link)
    }
}

impl From<Vec<Link>> for LinkOrArray {
    fn from(links: Vec<Link>) -> Self {
        LinkOrArray::Array(links)
    }
}

impl<T: rustapi_openapi::schema::RustApiSchema> Resource<T> {
    /// Create a new resource wrapper
    pub fn new(data: T) -> Self {
        Self {
            data,
            links: HashMap::new(),
            embedded: None,
        }
    }

    /// Add a link with the given relation
    pub fn link(mut self, rel: impl Into<String>, href: impl Into<String>) -> Self {
        self.links
            .insert(rel.into(), LinkOrArray::Single(Link::new(href)));
        self
    }

    /// Add a link object
    pub fn link_object(mut self, rel: impl Into<String>, link: Link) -> Self {
        self.links.insert(rel.into(), LinkOrArray::Single(link));
        self
    }

    /// Add multiple links for the same relation
    pub fn links(mut self, rel: impl Into<String>, links: Vec<Link>) -> Self {
        self.links.insert(rel.into(), LinkOrArray::Array(links));
        self
    }

    /// Add the canonical self link
    pub fn self_link(self, href: impl Into<String>) -> Self {
        self.link("self", href)
    }

    /// Add embedded resources
    pub fn embed<E: Serialize>(
        mut self,
        rel: impl Into<String>,
        resources: E,
    ) -> Result<Self, serde_json::Error> {
        let embedded = self.embedded.get_or_insert_with(HashMap::new);
        embedded.insert(rel.into(), serde_json::to_value(resources)?);
        Ok(self)
    }

    /// Add pre-serialized embedded resources
    pub fn embed_raw(mut self, rel: impl Into<String>, value: serde_json::Value) -> Self {
        let embedded = self.embedded.get_or_insert_with(HashMap::new);
        embedded.insert(rel.into(), value);
        self
    }
}

/// Collection of resources with pagination support
///
/// Provides a standardized way to return paginated collections with
/// navigation links.
///
/// # Example
/// ```rust
/// use rustapi_core::hateoas::{ResourceCollection, PageInfo};
/// use serde::Serialize;
///
/// #[derive(Serialize, Clone)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let users = vec![
///     User { id: 1, name: "John".to_string() },
///     User { id: 2, name: "Jane".to_string() },
/// ];
///
/// let collection = ResourceCollection::new("users", users)
///     .self_link("/users?page=1")
///     .next_link("/users?page=2")
///     .page_info(PageInfo::new(20, 100, 5, 1));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct ResourceCollection<T: rustapi_openapi::schema::RustApiSchema> {
    /// Embedded resources
    #[serde(rename = "_embedded")]
    pub embedded: HashMap<String, Vec<T>>,

    /// Navigation links
    #[serde(rename = "_links")]
    pub links: HashMap<String, LinkOrArray>,

    /// Pagination information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<PageInfo>,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct PageInfo {
    /// Number of items per page
    pub size: usize,
    /// Total number of items
    #[serde(rename = "totalElements")]
    pub total_elements: usize,
    /// Total number of pages
    #[serde(rename = "totalPages")]
    pub total_pages: usize,
    /// Current page number (0-indexed)
    pub number: usize,
}

impl PageInfo {
    /// Create new page info
    pub fn new(size: usize, total_elements: usize, total_pages: usize, number: usize) -> Self {
        Self {
            size,
            total_elements,
            total_pages,
            number,
        }
    }

    /// Calculate page info from total elements and page size
    pub fn calculate(total_elements: usize, page_size: usize, current_page: usize) -> Self {
        let total_pages = total_elements.div_ceil(page_size);
        Self {
            size: page_size,
            total_elements,
            total_pages,
            number: current_page,
        }
    }
}

impl<T: rustapi_openapi::schema::RustApiSchema> ResourceCollection<T> {
    /// Create a new resource collection
    pub fn new(rel: impl Into<String>, items: Vec<T>) -> Self {
        let mut embedded = HashMap::new();
        embedded.insert(rel.into(), items);

        Self {
            embedded,
            links: HashMap::new(),
            page: None,
        }
    }

    /// Add a link
    pub fn link(mut self, rel: impl Into<String>, href: impl Into<String>) -> Self {
        self.links
            .insert(rel.into(), LinkOrArray::Single(Link::new(href)));
        self
    }

    /// Add self link
    pub fn self_link(self, href: impl Into<String>) -> Self {
        self.link("self", href)
    }

    /// Add first page link
    pub fn first_link(self, href: impl Into<String>) -> Self {
        self.link("first", href)
    }

    /// Add last page link
    pub fn last_link(self, href: impl Into<String>) -> Self {
        self.link("last", href)
    }

    /// Add next page link
    pub fn next_link(self, href: impl Into<String>) -> Self {
        self.link("next", href)
    }

    /// Add previous page link
    pub fn prev_link(self, href: impl Into<String>) -> Self {
        self.link("prev", href)
    }

    /// Set page info
    pub fn page_info(mut self, page: PageInfo) -> Self {
        self.page = Some(page);
        self
    }

    /// Build pagination links from page info
    pub fn with_pagination(mut self, base_url: &str) -> Self {
        // Clone page info to avoid borrow issues
        let page_info = self.page.clone();

        if let Some(page) = page_info {
            self = self.self_link(format!(
                "{}?page={}&size={}",
                base_url, page.number, page.size
            ));
            self = self.first_link(format!("{}?page=0&size={}", base_url, page.size));

            if page.total_pages > 0 {
                self = self.last_link(format!(
                    "{}?page={}&size={}",
                    base_url,
                    page.total_pages - 1,
                    page.size
                ));
            }

            if page.number > 0 {
                self = self.prev_link(format!(
                    "{}?page={}&size={}",
                    base_url,
                    page.number - 1,
                    page.size
                ));
            }

            if page.number < page.total_pages.saturating_sub(1) {
                self = self.next_link(format!(
                    "{}?page={}&size={}",
                    base_url,
                    page.number + 1,
                    page.size
                ));
            }
        }
        self
    }
}

/// Helper trait for adding HATEOAS links to any type
pub trait Linkable: Sized + Serialize + rustapi_openapi::schema::RustApiSchema {
    /// Wrap this value in a Resource with HATEOAS links
    fn with_links(self) -> Resource<Self> {
        Resource::new(self)
    }
}

// Implement Linkable for all Serialize + Schema types
impl<T: Serialize + rustapi_openapi::schema::RustApiSchema> Linkable for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct User {
        id: i64,
        name: String,
    }

    #[test]
    fn test_link_creation() {
        let link = Link::new("/users/1")
            .title("Get user")
            .media_type("application/json");

        assert_eq!(link.href, "/users/1");
        assert_eq!(link.title, Some("Get user".to_string()));
        assert_eq!(link.media_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_templated_link() {
        let link = Link::templated("/users/{id}");
        assert!(link.templated.unwrap());
    }

    #[test]
    fn test_resource_with_links() {
        let user = User {
            id: 1,
            name: "John".to_string(),
        };
        let resource = Resource::new(user)
            .self_link("/users/1")
            .link("orders", "/users/1/orders");

        assert!(resource.links.contains_key("self"));
        assert!(resource.links.contains_key("orders"));

        let json = serde_json::to_string_pretty(&resource).unwrap();
        assert!(json.contains("_links"));
        assert!(json.contains("/users/1"));
    }

    #[test]
    fn test_resource_collection() {
        let users = vec![
            User {
                id: 1,
                name: "John".to_string(),
            },
            User {
                id: 2,
                name: "Jane".to_string(),
            },
        ];

        let page = PageInfo::calculate(100, 20, 2);
        let collection = ResourceCollection::new("users", users)
            .page_info(page)
            .with_pagination("/api/users");

        assert!(collection.links.contains_key("self"));
        assert!(collection.links.contains_key("first"));
        assert!(collection.links.contains_key("prev"));
        assert!(collection.links.contains_key("next"));
    }

    #[test]
    fn test_page_info_calculation() {
        let page = PageInfo::calculate(95, 20, 0);
        assert_eq!(page.total_pages, 5);
        assert_eq!(page.size, 20);
    }

    #[test]
    fn test_linkable_trait() {
        let user = User {
            id: 1,
            name: "Test".to_string(),
        };
        let resource = user.with_links().self_link("/users/1");
        assert!(resource.links.contains_key("self"));
    }
}
