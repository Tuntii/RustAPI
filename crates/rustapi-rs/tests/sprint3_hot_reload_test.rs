//! Sprint 3 Integration Tests — Hot Reload
//!
//! Tests for .hot_reload() builder API.

use rustapi_rs::prelude::*;

#[test]
fn hot_reload_builder_chain() {
    // Verify .hot_reload(true) chains correctly with other builder methods
    let _app = RustApi::new()
        .hot_reload(true)
        .on_start(|| async {})
        .on_shutdown(|| async {});
}

#[test]
fn hot_reload_disabled_by_default() {
    // Verify the builder compiles with hot_reload(false) (the default behavior)
    let _app = RustApi::new().hot_reload(false);
}

#[test]
fn hot_reload_with_full_config() {
    // Verify hot_reload works with all other builder methods
    use rustapi_rs::prelude::*;

    async fn hello() -> &'static str {
        "hello"
    }

    let _app = RustApi::new().hot_reload(true).route("/", get(hello));
}
