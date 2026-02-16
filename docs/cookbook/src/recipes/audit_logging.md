# Audit Logging & Compliance

In many enterprise applications, maintaining a detailed audit trail is crucial for security, compliance (GDPR, SOC2), and troubleshooting. RustAPI provides a comprehensive audit logging system in `rustapi-extras`.

This recipe covers how to create, log, and query audit events.

## Prerequisites

Add `rustapi-extras` with the `audit` feature to your `Cargo.toml`.

```toml
[dependencies]
rustapi-extras = { version = "0.1.335", features = ["audit"] }
```

## Core Concepts

The audit system is built around three main components:
- **AuditEvent**: Represents a single action performed by a user or system.
- **AuditStore**: Interface for persisting events (e.g., `InMemoryAuditStore`, `FileAuditStore`).
- **ComplianceInfo**: Additional metadata for regulatory requirements.

## Basic Usage

Log a simple event when a user is created.

```rust
use rustapi_extras::audit::{AuditEvent, AuditAction, InMemoryAuditStore, AuditStore};

#[tokio::main]
async fn main() {
    // Initialize the store (could be FileAuditStore for persistence)
    let store = InMemoryAuditStore::new();

    // Create an event
    let event = AuditEvent::new(AuditAction::Create)
        .resource("users", "user-123")       // Resource type & ID
        .actor("admin@example.com")          // Who performed the action
        .ip_address("192.168.1.1".parse().unwrap())
        .success(true);                      // Outcome

    // Log it asynchronously
    store.log(event);

    // ... later, query events
    let recent_logs = store.query().limit(10).execute().await;
    println!("Recent logs: {:?}", recent_logs);
}
```

## Compliance Features (GDPR & SOC2)

RustAPI's audit system includes dedicated fields for compliance tracking.

### GDPR Relevance
Events involving personal data can be flagged with legal basis and retention policies.

```rust
use rustapi_extras::audit::{ComplianceInfo, AuditEvent, AuditAction};

let compliance = ComplianceInfo::new()
    .personal_data(true)                 // Involves PII
    .data_subject("user-123")            // The person the data belongs to
    .legal_basis("consent")              // Article 6 basis
    .retention("30_days");               // Retention policy

let event = AuditEvent::new(AuditAction::Update)
    .compliance(compliance)
    .resource("profile", "user-123");
```

### SOC2 Controls
Link events to specific security controls.

```rust
let compliance = ComplianceInfo::new()
    .soc2_control("CC6.1"); // Access Control

let event = AuditEvent::new(AuditAction::Login)
    .compliance(compliance)
    .actor("employee@company.com");
```

## Tracking Changes

For updates, it's often useful to record what changed.

```rust
use rustapi_extras::audit::AuditChanges;

let changes = AuditChanges::new()
    .field("email", "old@example.com", "new@example.com")
    .field("role", "user", "admin");

let event = AuditEvent::new(AuditAction::Update)
    .changes(changes)
    .resource("users", "user-123");
```

## Best Practices

1.  **Log All Security Events**: Logins (success/failure), permission changes, and API key management should always be audited.
2.  **Include Context**: Add `request_id` or `session_id` to correlate logs with tracing data.
3.  **Use Asynchronous Logging**: The `AuditStore` is designed to be non-blocking. Use it in a background task or `tokio::spawn` if needed for heavy writes.
4.  **Secure the Logs**: Ensure that the storage backend (file, database) is protected from tampering.
