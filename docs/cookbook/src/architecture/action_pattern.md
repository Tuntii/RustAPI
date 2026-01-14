# The Action Pattern

The **Action Pattern** is the central design abstraction of RustAPI. It redefines how we structure business logic, moving away from monolithic controllers/services into distinct, atomic units of work.

## What is an Action?
An "Action" corresponds to exactly one business intent.
- **Good**: `CreateUser`, `DebitAccount`, `SendWelcomeEmail`
- **Bad**: `UserService` (too broad), `ManageOrders` (vague)

## Why?
1.  **Isolation**: Since every action is its own struct, it has its own file, its own tests, and its own unique set of dependencies. Modifying `CreateUser` cannot accidentally break `DeleteUser`.
2.  **Testability**: You can inject mocks for just the dependencies this specific action needs.
3.  **Readability**: The codebase becomes a catalog of capabilities.

## Implementation Standard

Every action implements a trait (usually `Runnable` or similar) that defines its contract.

```rust
// Example Structure
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

impl Action for CreateUser {
    type Output = User;
    type Error = ApiError;

    async fn run(&self, ctx: &Context) -> Result<Self::Output, Self::Error> {
        // 1. Validate
        // 2. Persist
        // 3. Notify
    }
}
```

> [!TIP]
> Use the `rustapi-macros` crate to auto-implement boilerplate for standard Actions.
