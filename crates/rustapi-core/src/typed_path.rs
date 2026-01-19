use serde::{de::DeserializeOwned, Serialize};

/// Trait for defining typed paths
///
/// This trait allows structs to define their own path pattern and URL generation logic.
/// It is usually implemented via `#[derive(TypedPath)]`.
pub trait TypedPath: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// The URL path pattern (e.g., "/users/{id}")
    const PATH: &'static str;

    /// Convert the struct fields to a path string
    fn to_uri(&self) -> String;
}
