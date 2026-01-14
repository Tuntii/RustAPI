# Recipe: Adding a New Feature

This guide walks you through the standard process of adding a new feature to RustAPI using the Action pattern.

## Prerequisites
- [x] You have the repo cloned.
- [x] You understand the [Action Pattern](../architecture/action_pattern.md).

## Step 1: Define the Action Struct
Create a new file in the appropriate crate (or `crates/rustapi-core/src/actions/` if it's core).

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateWidget {
    pub name: String,
    pub quantity: i32,
}
```

## Step 2: Implement the Logic
Implement the logic trait.

```rust
impl Action for CreateWidget {
    type Output = Widget;
    type Error = ApiError;

    async fn run(&self, ctx: &Context) -> Result<Self::Output, Self::Error> {
        // Validation
        if self.quantity < 0 {
            return Err(ApiError::BadRequest("Quantity must be positive"));
        }

        // Database
        let widget = sqlx::query_as!(
            Widget,
            "INSERT INTO widgets (name, quantity) VALUES ($1, $2) RETURNING *",
            self.name,
            self.quantity
        )
        .fetch_one(&ctx.db)
        .await?;

        Ok(widget)
    }
}
```

## Step 3: Register the Route
Add the action to your router configuration.

```rust
// in router.rs or module definition
.route("/widgets", post(handle_action::<CreateWidget>))
```

## Step 4: Add Tests
Create a dedicated test file or use the `tests` module in the same file.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        let action = CreateWidget { name: "Test".into(), quantity: -1 };
        // Assert it returns error...
    }
}
```
