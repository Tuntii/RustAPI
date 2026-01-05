# cargo-rustapi

CLI tool for the RustAPI framework - Project scaffolding and development utilities.

## Installation

```bash
cargo install cargo-rustapi
```

## Usage

### Create a New Project

```bash
# Interactive mode
cargo rustapi new my-project

# With template
cargo rustapi new my-project --template api

# With features
cargo rustapi new my-project --features jwt,cors
```

### Available Templates

- `minimal` - Bare minimum RustAPI app (default)
- `api` - REST API with CRUD example
- `web` - Web app with templates
- `full` - Full-featured with JWT, CORS, database

### Run Development Server

```bash
# Run with auto-reload
cargo rustapi run

# Run on specific port
cargo rustapi run --port 8080

# Run with specific features
cargo rustapi run --features jwt
```

### Generate Code

```bash
# Generate a new handler
cargo rustapi generate handler users

# Generate a model
cargo rustapi generate model User

# Generate CRUD endpoints
cargo rustapi generate crud users
```

## Commands

| Command | Description |
|---------|-------------|
| `new <name>` | Create a new RustAPI project |
| `run` | Run development server with auto-reload |
| `generate <type> <name>` | Generate code from templates |
| `docs` | Open API documentation |

## Project Templates

### Minimal Template
```
my-project/
├── Cargo.toml
├── src/
│   └── main.rs
└── .gitignore
```

### API Template
```
my-project/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── handlers/
│   │   └── mod.rs
│   ├── models/
│   │   └── mod.rs
│   └── error.rs
├── .env.example
└── .gitignore
```

### Web Template
```
my-project/
├── Cargo.toml
├── src/
│   ├── main.rs
│   └── handlers/
├── templates/
│   ├── base.html
│   └── index.html
├── static/
│   └── style.css
└── .gitignore
```

## License

MIT OR Apache-2.0
