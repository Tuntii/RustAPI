# System Architecture

RustAPI follows a **Facade Architecture** — a stable public API that shields you from internal complexity and breaking changes.

## System Overview

```mermaid
graph TB
    subgraph Client["🌐 Client Layer"]
        HTTP[HTTP Request]
        LLM[LLM/AI Agent]
        MCP[MCP Client]
    end

    subgraph Public["📦 rustapi-rs (Public Facade)"]
        direction TB
        Prelude[prelude::*]
        Macros["#[rustapi_rs::get/post]<br>#[rustapi_rs::main]"]
        Types[Json, Query, Path, Form]
    end

    subgraph Core["⚙️ rustapi-core (Engine)"]
        direction TB
        Router[Radix Router<br>matchit]
        Extract[Extractors<br>FromRequest trait]
        MW[Middleware Stack<br>Tower-like layers]
        Resp[Response Builder<br>IntoResponse trait]
    end

    subgraph Extensions["🔌 Extension Crates"]
        direction LR
        OpenAPI["rustapi-openapi<br>OpenAPI 3.1 + Docs"]
        Validate["rustapi-validate<br>Validation (v2 native)"]
        Toon["rustapi-toon<br>LLM Optimization"]
        Extras["rustapi-extras<br>JWT/CORS/RateLimit"]
        WsCrate["rustapi-ws<br>WebSocket Support"]
        ViewCrate["rustapi-view<br>Template Engine"]
    end

    subgraph Foundation["🏗️ Foundation Layer"]
        direction LR
        Tokio[tokio<br>Async Runtime]
        Hyper[hyper 1.0<br>HTTP Protocol]
        Serde[serde<br>Serialization]
    end

    HTTP --> Public
    LLM --> Public
    MCP --> Public
    Public --> Core
    Core --> Extensions
    Extensions --> Foundation
    Core --> Foundation
```

## Request Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant R as Router
    participant M as Middleware
    participant E as Extractors
    participant H as Handler
    participant S as Serializer

    C->>R: HTTP Request
    R->>R: Match route (radix tree)
    R->>M: Pass to middleware stack
    
    loop Each Middleware
        M->>M: Process (JWT, CORS, RateLimit)
    end
    
    M->>E: Extract parameters
    E->>E: Json<T>, Path<T>, Query<T>
    E->>E: Validate (v2 native / optional legacy)
    
    alt Validation Failed
        E-->>C: 422 Unprocessable Entity
    else Validation OK
        E->>H: Call async handler
        H->>S: Return response type
        
        alt TOON Enabled
            S->>S: Check Accept header
            S->>S: Serialize as TOON/JSON
            S->>S: Add token count headers
        else Standard
            S->>S: Serialize as JSON
        end
        
        S-->>C: HTTP Response
    end
```

## Crate Dependency Graph

```mermaid
graph BT
    subgraph User["Your Application"]
        App[main.rs]
    end

    subgraph Facade["Single Import"]
        RS[rustapi-rs]
    end

    subgraph Internal["Internal Crates"]
        Core[rustapi-core]
        Macros[rustapi-macros]
        OpenAPI[rustapi-openapi]
        Validate[rustapi-validate]
        Toon[rustapi-toon]
        Extras[rustapi-extras]
        WS[rustapi-ws]
        View[rustapi-view]
    end

    subgraph External["External Dependencies"]
        Tokio[tokio]
        Hyper[hyper]
        Serde[serde]
        Validator[validator]
        Tungstenite[tungstenite]
        Tera[tera]
    end

    App --> RS
    RS --> Core
    RS --> Macros
    RS --> OpenAPI
    RS --> Validate
    RS -.->|optional| Toon
    RS -.->|optional| Extras
    RS -.->|optional| WS
    RS -.->|optional| View
    
    Core --> Tokio
    Core --> Hyper
    Core --> Serde
    OpenAPI --> Serde
    Validate -.->|legacy optional| Validator
    Toon --> Serde
    WS --> Tungstenite
    View --> Tera

    style RS fill:#e1f5fe
    style App fill:#c8e6c9
```

## Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Single Entry Point** | `use rustapi_rs::prelude::*` imports everything you need |
| **Zero Boilerplate** | Macros generate routing, OpenAPI specs, and validation |
| **Compile-Time Safety** | Generic extractors catch type errors at compile time |
| **Opt-in Complexity** | Features like JWT, TOON are behind feature flags |
| **Engine Abstraction** | Internal hyper/tokio upgrades don't break your code |

## Crate Responsibilities

| Crate | Role |
|-------|------|
| `rustapi-rs` | Public facade — single `use` for everything |
| `rustapi-core` | HTTP engine, routing, extractors, response handling |
| `rustapi-macros` | Procedural macros: `#[rustapi_rs::get]`, `#[rustapi_rs::main]` |
| `rustapi-openapi` | Native OpenAPI 3.1 model, schema registry, and docs endpoints |
| `rustapi-validate` | Validation runtime (v2 native default, legacy validator optional) |
| `rustapi-toon` | TOON format serializer, content negotiation, LLM headers |
| `rustapi-extras` | JWT auth, CORS, rate limiting, audit logging |
| `rustapi-ws` | WebSocket support with broadcast channels |
| `rustapi-view` | Template engine (Tera) for server-side rendering |
| `rustapi-jobs` | Background job processing (Redis/Postgres) |
| `rustapi-testing` | Test utilities, matchers, expectations |
