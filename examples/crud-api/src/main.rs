//! CRUD API Example for RustAPI
//!
//! This example demonstrates a complete CRUD API with:
//! - All HTTP methods (GET, POST, PUT, PATCH, DELETE)
//! - Request validation
//! - Error handling
//! - Middleware (RequestId, Tracing, Body Limit)
//! - OpenAPI documentation with Swagger UI
//!
//! Run with: cargo run -p crud-api
//! Then visit: http://127.0.0.1:8080/docs

use rustapi_rs::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================
// Data Models
// ============================================

/// A task in our todo list
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
}

/// Request body for creating a task
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct CreateTask {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: String,
    #[validate(length(max = 1000, message = "Description must be at most 1000 characters"))]
    pub description: Option<String>,
}

/// Request body for updating a task
#[derive(Debug, Deserialize, Validate, Schema)]
pub struct UpdateTask {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: String,
    #[validate(length(max = 1000, message = "Description must be at most 1000 characters"))]
    pub description: Option<String>,
    pub completed: bool,
}

/// Request body for partial task update
#[derive(Debug, Deserialize, Schema)]
pub struct PatchTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

/// Query parameters for listing tasks
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListParams {
    /// Filter by completion status
    pub completed: Option<bool>,
    /// Page number (1-indexed)
    #[param(minimum = 1)]
    pub page: Option<u32>,
    /// Items per page
    #[param(minimum = 1, maximum = 100)]
    pub limit: Option<u32>,
}

/// Paginated response wrapper
#[derive(Debug, Serialize, Schema)]
pub struct PaginatedTasks {
    pub tasks: Vec<Task>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
}

// ============================================
// In-Memory Database
// ============================================

/// Simple in-memory task store
#[derive(Clone)]
pub struct TaskStore {
    tasks: Arc<RwLock<HashMap<u64, Task>>>,
    next_id: Arc<RwLock<u64>>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        }
    }

    pub fn create(&self, create: CreateTask) -> Task {
        let mut next_id = self.next_id.write().unwrap();
        let id = *next_id;
        *next_id += 1;

        let task = Task {
            id,
            title: create.title,
            description: create.description,
            completed: false,
        };

        self.tasks.write().unwrap().insert(id, task.clone());
        task
    }

    pub fn get(&self, id: u64) -> Option<Task> {
        self.tasks.read().unwrap().get(&id).cloned()
    }

    pub fn list(&self, completed: Option<bool>) -> Vec<Task> {
        let tasks = self.tasks.read().unwrap();
        let mut result: Vec<Task> = tasks
            .values()
            .filter(|t| completed.map_or(true, |c| t.completed == c))
            .cloned()
            .collect();
        result.sort_by_key(|t| t.id);
        result
    }

    pub fn update(&self, id: u64, update: UpdateTask) -> Option<Task> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&id) {
            task.title = update.title;
            task.description = update.description;
            task.completed = update.completed;
            Some(task.clone())
        } else {
            None
        }
    }

    pub fn patch(&self, id: u64, patch: PatchTask) -> Option<Task> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(&id) {
            if let Some(title) = patch.title {
                task.title = title;
            }
            if let Some(description) = patch.description {
                task.description = Some(description);
            }
            if let Some(completed) = patch.completed {
                task.completed = completed;
            }
            Some(task.clone())
        } else {
            None
        }
    }

    pub fn delete(&self, id: u64) -> bool {
        self.tasks.write().unwrap().remove(&id).is_some()
    }
}

impl Default for TaskStore {
    fn default() -> Self {
        Self::new()
    }
}


// ============================================
// Handlers
// ============================================

/// List all tasks with optional filtering and pagination
#[rustapi_rs::get("/tasks")]
#[rustapi_rs::tag("Tasks")]
#[rustapi_rs::summary("List Tasks")]
#[rustapi_rs::description("Returns a paginated list of tasks. Can filter by completion status.")]
async fn list_tasks(
    State(store): State<TaskStore>,
    Query(params): Query<ListParams>,
) -> Json<PaginatedTasks> {
    let all_tasks = store.list(params.completed);
    let total = all_tasks.len();
    
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10);
    let skip = ((page - 1) * limit) as usize;
    
    let tasks: Vec<Task> = all_tasks
        .into_iter()
        .skip(skip)
        .take(limit as usize)
        .collect();
    
    Json(PaginatedTasks {
        tasks,
        total,
        page,
        limit,
    })
}

/// Get a single task by ID
#[rustapi_rs::get("/tasks/{id}")]
#[rustapi_rs::tag("Tasks")]
#[rustapi_rs::summary("Get Task")]
#[rustapi_rs::description("Returns a single task by its ID. Returns 404 if not found.")]
async fn get_task(
    State(store): State<TaskStore>,
    Path(id): Path<u64>,
) -> Result<Json<Task>, ApiError> {
    store
        .get(id)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("Task {} not found", id)))
}

/// Create a new task
#[rustapi_rs::post("/tasks")]
#[rustapi_rs::tag("Tasks")]
#[rustapi_rs::summary("Create Task")]
#[rustapi_rs::description("Creates a new task. Validates title (1-200 chars) and description (max 1000 chars).")]
async fn create_task(
    State(store): State<TaskStore>,
    ValidatedJson(body): ValidatedJson<CreateTask>,
) -> Created<Task> {
    let task = store.create(body);
    Created(task)
}

/// Update a task completely (PUT)
#[rustapi_rs::put("/tasks/{id}")]
#[rustapi_rs::tag("Tasks")]
#[rustapi_rs::summary("Update Task")]
#[rustapi_rs::description("Replaces a task entirely. All fields are required.")]
async fn update_task(
    State(store): State<TaskStore>,
    Path(id): Path<u64>,
    ValidatedJson(body): ValidatedJson<UpdateTask>,
) -> Result<Json<Task>, ApiError> {
    store
        .update(id, body)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("Task {} not found", id)))
}

/// Partially update a task (PATCH)
#[rustapi_rs::patch("/tasks/{id}")]
#[rustapi_rs::tag("Tasks")]
#[rustapi_rs::summary("Patch Task")]
#[rustapi_rs::description("Partially updates a task. Only provided fields are updated.")]
async fn patch_task(
    State(store): State<TaskStore>,
    Path(id): Path<u64>,
    Json(body): Json<PatchTask>,
) -> Result<Json<Task>, ApiError> {
    store
        .patch(id, body)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("Task {} not found", id)))
}

/// Delete a task
#[rustapi_rs::delete("/tasks/{id}")]
#[rustapi_rs::tag("Tasks")]
#[rustapi_rs::summary("Delete Task")]
#[rustapi_rs::description("Deletes a task by ID. Returns 204 on success, 404 if not found.")]
async fn delete_task(
    State(store): State<TaskStore>,
    Path(id): Path<u64>,
) -> Result<NoContent, ApiError> {
    if store.delete(id) {
        Ok(NoContent)
    } else {
        Err(ApiError::not_found(format!("Task {} not found", id)))
    }
}

/// Health check endpoint
#[rustapi_rs::get("/health")]
#[rustapi_rs::tag("System")]
#[rustapi_rs::summary("Health Check")]
async fn health() -> &'static str {
    "OK"
}

// ============================================
// Main
// ============================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize task store with some sample data
    let store = TaskStore::new();
    store.create(CreateTask {
        title: "Learn RustAPI".to_string(),
        description: Some("Build a web API with RustAPI framework".to_string()),
    });
    store.create(CreateTask {
        title: "Write tests".to_string(),
        description: Some("Add unit and integration tests".to_string()),
    });
    store.create(CreateTask {
        title: "Deploy to production".to_string(),
        description: None,
    });

    println!("🚀 CRUD API Example");
    println!();
    println!("Endpoints:");
    println!("  GET    /tasks       - List all tasks");
    println!("  GET    /tasks/:id   - Get a task");
    println!("  POST   /tasks       - Create a task");
    println!("  PUT    /tasks/:id   - Update a task");
    println!("  PATCH  /tasks/:id   - Partially update a task");
    println!("  DELETE /tasks/:id   - Delete a task");
    println!("  GET    /health      - Health check");
    println!("  GET    /docs        - Swagger UI");
    println!();
    println!("Server running at http://127.0.0.1:8080");

    RustApi::new()
        .state(store)
        .body_limit(1024 * 1024) // 1MB limit
        .layer(RequestIdLayer::new())
        .layer(TracingLayer::new())
        .register_schema::<Task>()
        .register_schema::<CreateTask>()
        .register_schema::<UpdateTask>()
        .register_schema::<PatchTask>()
        .register_schema::<PaginatedTasks>()
        .mount_route(list_tasks_route())
        .mount_route(get_task_route())
        .mount_route(create_task_route())
        .mount_route(update_task_route())
        .mount_route(patch_task_route())
        .mount_route(delete_task_route())
        .mount_route(health_route())
        .docs("/docs")
        .run("127.0.0.1:8080")
        .await
}
