use rustapi_rs::prelude::*;
use std::collections::HashMap;
use std::sync::{atomic::{AtomicU64, Ordering}, Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    next_id: Arc<AtomicU64>,
    todos: Arc<RwLock<HashMap<u64, TodoItem>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
struct TodoItem {
    id: u64,
    title: String,
    completed: bool,
}

#[derive(Debug, Deserialize, Schema)]
struct CreateTodo {
    title: String,
}

#[derive(Debug, Deserialize, Schema)]
struct UpdateTodo {
    title: Option<String>,
    completed: Option<bool>,
}

#[derive(Debug, Serialize, Schema)]
struct TodoEnvelope {
    found: bool,
    item: Option<TodoItem>,
    message: Option<String>,
}

async fn list_todos(State(state): State<AppState>) -> Json<Vec<TodoItem>> {
    let todos = state.todos.read().await;
    let mut items: Vec<_> = todos.values().cloned().collect();
    items.sort_by_key(|todo| todo.id);
    Json(items)
}

async fn create_todo(State(state): State<AppState>, Json(payload): Json<CreateTodo>) -> Created<TodoItem> {
    let id = state.next_id.fetch_add(1, Ordering::SeqCst);
    let item = TodoItem {
        id,
        title: payload.title,
        completed: false,
    };

    state.todos.write().await.insert(id, item.clone());
    Created(item)
}

async fn get_todo(State(state): State<AppState>, Path(id): Path<u64>) -> Json<TodoEnvelope> {
    let item = state.todos.read().await.get(&id).cloned();
    Json(TodoEnvelope {
        found: item.is_some(),
        item,
        message: Some(format!("Looked up todo {}", id)),
    })
}

async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(payload): Json<UpdateTodo>,
) -> Json<TodoEnvelope> {
    let mut todos = state.todos.write().await;
    let item = todos.get_mut(&id).map(|todo| {
        if let Some(title) = payload.title {
            todo.title = title;
        }
        if let Some(completed) = payload.completed {
            todo.completed = completed;
        }
        todo.clone()
    });

    Json(TodoEnvelope {
        found: item.is_some(),
        item,
        message: Some(format!("Updated todo {}", id)),
    })
}

async fn delete_todo(State(state): State<AppState>, Path(id): Path<u64>) -> NoContent {
    state.todos.write().await.remove(&id);
    println!("Deleted todo {} (if it existed)", id);
    NoContent
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting CRUD example...");
    println!(" -> GET    http://127.0.0.1:3000/todos");
    println!(" -> POST   http://127.0.0.1:3000/todos");
    println!(" -> GET    http://127.0.0.1:3000/todos/1");
    println!(" -> PUT    http://127.0.0.1:3000/todos/1");
    println!(" -> DELETE http://127.0.0.1:3000/todos/1");

    RustApi::new()
        .state(AppState {
            next_id: Arc::new(AtomicU64::new(1)),
            todos: Arc::new(RwLock::new(HashMap::new())),
        })
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/{id}", get(get_todo).put(update_todo).delete(delete_todo))
        .run("127.0.0.1:3000")
        .await
}
