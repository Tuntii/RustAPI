//! In-memory data stores for the Bookmark Manager POC
//!
//! These stores provide thread-safe access to application data.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

use crate::models::{Bookmark, Category, User};
use crate::sse::SseBroadcaster;

/// Application state containing all stores
pub struct AppState {
    pub users: UserStore,
    pub bookmarks: BookmarkStore,
    pub categories: CategoryStore,
    pub sse_broadcaster: SseBroadcaster,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            users: UserStore::new(),
            bookmarks: BookmarkStore::new(),
            categories: CategoryStore::new(),
            sse_broadcaster: SseBroadcaster::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe user store with email index
pub struct UserStore {
    users: RwLock<HashMap<u64, User>>,
    email_index: RwLock<HashMap<String, u64>>,
    next_id: AtomicU64,
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
            email_index: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a new user, returns None if email already exists
    pub async fn create(&self, user: User) -> Option<User> {
        let mut email_index = self.email_index.write().await;
        
        // Check if email already exists
        if email_index.contains_key(&user.email) {
            return None;
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut new_user = user;
        new_user.id = id;

        email_index.insert(new_user.email.clone(), id);
        
        let mut users = self.users.write().await;
        users.insert(id, new_user.clone());

        Some(new_user)
    }

    /// Find user by email
    pub async fn find_by_email(&self, email: &str) -> Option<User> {
        let email_index = self.email_index.read().await;
        let user_id = email_index.get(email)?;
        
        let users = self.users.read().await;
        users.get(user_id).cloned()
    }

    /// Find user by ID
    pub async fn find_by_id(&self, id: u64) -> Option<User> {
        let users = self.users.read().await;
        users.get(&id).cloned()
    }
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}


/// Thread-safe bookmark store with user index
pub struct BookmarkStore {
    bookmarks: RwLock<HashMap<u64, Bookmark>>,
    user_index: RwLock<HashMap<u64, HashSet<u64>>>,
    next_id: AtomicU64,
}

impl BookmarkStore {
    pub fn new() -> Self {
        Self {
            bookmarks: RwLock::new(HashMap::new()),
            user_index: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a new bookmark
    pub async fn create(&self, bookmark: Bookmark) -> Bookmark {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut new_bookmark = bookmark;
        new_bookmark.id = id;

        let mut bookmarks = self.bookmarks.write().await;
        bookmarks.insert(id, new_bookmark.clone());

        let mut user_index = self.user_index.write().await;
        user_index
            .entry(new_bookmark.user_id)
            .or_default()
            .insert(id);

        new_bookmark
    }

    /// Find bookmark by ID
    pub async fn find_by_id(&self, id: u64) -> Option<Bookmark> {
        let bookmarks = self.bookmarks.read().await;
        bookmarks.get(&id).cloned()
    }

    /// Find all bookmarks for a user
    pub async fn find_by_user(&self, user_id: u64) -> Vec<Bookmark> {
        let user_index = self.user_index.read().await;
        let bookmark_ids = match user_index.get(&user_id) {
            Some(ids) => ids.clone(),
            None => return vec![],
        };

        let bookmarks = self.bookmarks.read().await;
        bookmark_ids
            .iter()
            .filter_map(|id| bookmarks.get(id).cloned())
            .collect()
    }

    /// Update a bookmark
    pub async fn update(&self, id: u64, bookmark: Bookmark) -> Option<Bookmark> {
        let mut bookmarks = self.bookmarks.write().await;
        if bookmarks.contains_key(&id) {
            bookmarks.insert(id, bookmark.clone());
            Some(bookmark)
        } else {
            None
        }
    }

    /// Delete a bookmark
    pub async fn delete(&self, id: u64) -> Option<Bookmark> {
        let mut bookmarks = self.bookmarks.write().await;
        let bookmark = bookmarks.remove(&id)?;

        let mut user_index = self.user_index.write().await;
        if let Some(ids) = user_index.get_mut(&bookmark.user_id) {
            ids.remove(&id);
        }

        Some(bookmark)
    }

    /// Clear category from all bookmarks
    pub async fn clear_category(&self, category_id: u64) {
        let mut bookmarks = self.bookmarks.write().await;
        for bookmark in bookmarks.values_mut() {
            if bookmark.category_id == Some(category_id) {
                bookmark.category_id = None;
            }
        }
    }
}

impl Default for BookmarkStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe category store
pub struct CategoryStore {
    categories: RwLock<HashMap<u64, Category>>,
    user_index: RwLock<HashMap<u64, HashSet<u64>>>,
    next_id: AtomicU64,
}

impl CategoryStore {
    pub fn new() -> Self {
        Self {
            categories: RwLock::new(HashMap::new()),
            user_index: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a new category
    pub async fn create(&self, category: Category) -> Category {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut new_category = category;
        new_category.id = id;

        let mut categories = self.categories.write().await;
        categories.insert(id, new_category.clone());

        let mut user_index = self.user_index.write().await;
        user_index
            .entry(new_category.user_id)
            .or_default()
            .insert(id);

        new_category
    }

    /// Find category by ID
    pub async fn find_by_id(&self, id: u64) -> Option<Category> {
        let categories = self.categories.read().await;
        categories.get(&id).cloned()
    }

    /// Find all categories for a user
    pub async fn find_by_user(&self, user_id: u64) -> Vec<Category> {
        let user_index = self.user_index.read().await;
        let category_ids = match user_index.get(&user_id) {
            Some(ids) => ids.clone(),
            None => return vec![],
        };

        let categories = self.categories.read().await;
        category_ids
            .iter()
            .filter_map(|id| categories.get(id).cloned())
            .collect()
    }

    /// Find category by name for a user
    pub async fn find_by_name(&self, user_id: u64, name: &str) -> Option<Category> {
        let categories = self.find_by_user(user_id).await;
        categories.into_iter().find(|c| c.name == name)
    }

    /// Update a category
    pub async fn update(&self, id: u64, category: Category) -> Option<Category> {
        let mut categories = self.categories.write().await;
        if categories.contains_key(&id) {
            categories.insert(id, category.clone());
            Some(category)
        } else {
            None
        }
    }

    /// Delete a category
    pub async fn delete(&self, id: u64) -> Option<Category> {
        let mut categories = self.categories.write().await;
        let category = categories.remove(&id)?;

        let mut user_index = self.user_index.write().await;
        if let Some(ids) = user_index.get_mut(&category.user_id) {
            ids.remove(&id);
        }

        Some(category)
    }
}

impl Default for CategoryStore {
    fn default() -> Self {
        Self::new()
    }
}
