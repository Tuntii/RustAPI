//! Integration tests for cargo-rustapi CLI

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

/// Helper to get the cargo-rustapi binary
fn cargo_rustapi() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("cargo-rustapi")
}

mod new_command {
    use super::*;

    #[test]
    fn test_new_help() {
        cargo_rustapi()
            .arg("new")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Create a new RustAPI project"));
    }

    #[test]
    fn test_new_minimal_template() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-minimal-project";
        let project_path = dir.path().join(project_name);

        // Change to temp directory and create project
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Verify project structure
        assert!(project_path.exists(), "Project directory should exist");
        assert!(
            project_path.join("Cargo.toml").exists(),
            "Cargo.toml should exist"
        );
        assert!(
            project_path.join("src/main.rs").exists(),
            "src/main.rs should exist"
        );

        // Verify Cargo.toml content
        let cargo_content =
            fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
        assert!(
            cargo_content.contains("rustapi-rs"),
            "Cargo.toml should depend on rustapi-rs"
        );
    }

    #[test]
    fn test_new_api_template() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-api-project";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "api", "--yes"])
            .assert()
            .success();

        // Verify API project structure
        assert!(project_path.join("src/handlers").is_dir());
        assert!(project_path.join("src/models").is_dir());
        assert!(project_path.join("src/handlers/mod.rs").exists());
        assert!(project_path.join("src/handlers/items.rs").exists());
        assert!(project_path.join("src/models/mod.rs").exists());
    }

    #[test]
    fn test_new_with_features() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "test-features-project";
        let project_path = dir.path().join(project_name);

        cargo_rustapi()
            .current_dir(dir.path())
            .args([
                "new",
                project_name,
                "--template",
                "minimal",
                "--features",
                "extras-jwt,extras-cors",
                "--yes",
            ])
            .assert()
            .success();

        let cargo_content =
            fs::read_to_string(project_path.join("Cargo.toml")).expect("Failed to read Cargo.toml");
        assert!(
            cargo_content.contains("extras-jwt") && cargo_content.contains("extras-cors"),
            "Cargo.toml should include extras-jwt and extras-cors features"
        );
    }

    #[test]
    fn test_new_existing_directory_fails() {
        let dir = tempdir().expect("Failed to create temp dir");
        let project_name = "existing-dir";

        // Create the directory first
        fs::create_dir(dir.path().join(project_name)).expect("Failed to create dir");

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", project_name, "--template", "minimal", "--yes"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));
    }

    #[test]
    fn test_new_invalid_name_fails() {
        let dir = tempdir().expect("Failed to create temp dir");

        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "invalid/name", "--template", "minimal", "--yes"])
            .assert()
            .failure();
    }
}

mod doctor_command {
    use super::*;

    #[test]
    fn test_doctor_help() {
        cargo_rustapi()
            .arg("doctor")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("environment health"));
    }

    #[test]
    fn test_doctor_runs() {
        // Doctor should run and check for tools
        // It will succeed even if some tools are missing (just warns)
        cargo_rustapi().arg("doctor").assert().success();
    }

    #[test]
    fn test_doctor_checks_rust() {
        let output = cargo_rustapi()
            .arg("doctor")
            .output()
            .expect("Failed to run doctor");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Rust compiler") || stdout.contains("rustc"),
            "Doctor should check for Rust compiler"
        );
    }
}

mod generate_command {
    use super::*;

    #[test]
    fn test_generate_help() {
        cargo_rustapi()
            .args(["generate", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Generate code from templates"));
    }

    #[test]
    fn test_generate_handler() {
        let dir = tempdir().expect("Failed to create temp dir");

        // First create a minimal project
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "test-gen", "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Generate a handler
        cargo_rustapi()
            .current_dir(dir.path().join("test-gen"))
            .args(["generate", "handler", "users"])
            .assert()
            .success();

        // Verify handler was created
        let handler_path = dir.path().join("test-gen/src/handlers/users.rs");
        assert!(handler_path.exists(), "Handler file should be created");

        let content = fs::read_to_string(&handler_path).expect("Failed to read handler");
        assert!(content.contains("pub async fn list"));
        assert!(content.contains("pub async fn get"));
        assert!(content.contains("pub async fn create"));
    }

    #[test]
    fn test_generate_model() {
        let dir = tempdir().expect("Failed to create temp dir");

        // First create a minimal project
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "test-model", "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Generate a model (model name is used as-is, should be PascalCase)
        cargo_rustapi()
            .current_dir(dir.path().join("test-model"))
            .args(["generate", "model", "User"])
            .assert()
            .success();

        // Model file is lowercase
        let model_path = dir.path().join("test-model/src/models/user.rs");
        assert!(model_path.exists(), "Model file should be created");

        let content = fs::read_to_string(&model_path).expect("Failed to read model");
        // The generate command uses the name as-is for struct name
        assert!(content.contains("struct User"));
        assert!(content.contains("impl User"));
    }
}

mod watch_command {
    use super::*;

    #[test]
    fn test_watch_help() {
        cargo_rustapi()
            .arg("watch")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Watch for changes"));
    }

    #[test]
    fn test_watch_accepts_command_flag() {
        cargo_rustapi()
            .args(["watch", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--command"))
            .stdout(predicate::str::contains("--clear"));
    }

    #[test]
    fn test_watch_accepts_extension_filter() {
        cargo_rustapi()
            .args(["watch", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--extensions"));
    }

    #[test]
    fn test_watch_accepts_path_filter() {
        cargo_rustapi()
            .args(["watch", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--watch-path"));
    }
}

mod migrate_command {
    use super::*;

    #[test]
    fn test_migrate_help() {
        cargo_rustapi()
            .args(["migrate", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Database migration"));
    }

    #[test]
    fn test_migrate_run_help() {
        cargo_rustapi()
            .args(["migrate", "run", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("pending migrations"));
    }

    #[test]
    fn test_migrate_status_help() {
        cargo_rustapi()
            .args(["migrate", "status", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("migration status"));
    }

    #[test]
    fn test_migrate_create_help() {
        cargo_rustapi()
            .args(["migrate", "create", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("new migration"));
    }

    #[test]
    fn test_migrate_create_generates_files() {
        let dir = tempdir().expect("Failed to create temp dir");

        // Create a project first
        cargo_rustapi()
            .current_dir(dir.path())
            .args(["new", "test-migrate", "--template", "minimal", "--yes"])
            .assert()
            .success();

        // Create a migration
        cargo_rustapi()
            .current_dir(dir.path().join("test-migrate"))
            .args(["migrate", "create", "create_users_table"])
            .assert()
            .success();

        // Check migrations directory exists
        let migrations_dir = dir.path().join("test-migrate/migrations");
        assert!(migrations_dir.exists(), "migrations directory should exist");

        // Check that migration files were created
        let entries: Vec<_> = fs::read_dir(&migrations_dir)
            .expect("Failed to read migrations dir")
            .collect();
        assert!(!entries.is_empty(), "Migration files should be created");
    }

    #[test]
    fn test_migrate_revert_help() {
        cargo_rustapi()
            .args(["migrate", "revert", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Revert"));
    }
}
