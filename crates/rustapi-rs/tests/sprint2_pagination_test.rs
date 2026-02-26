//! Sprint 2 Integration Tests — Pagination & HATEOAS Helpers
//!
//! Tests for Paginate, CursorPaginate extractors and Paginated<T>, CursorPaginated<T> responses.

use rustapi_rs::prelude::*;

// ─── Paginate Extractor ─────────────────────────────────────────────────────

#[test]
fn paginate_defaults() {
    let p = Paginate::new(0, 0);
    assert_eq!(p.page, 1, "page should default to minimum 1");
    assert_eq!(p.per_page, 1, "per_page should default to minimum 1");
}

#[test]
fn paginate_clamps_per_page() {
    let p = Paginate::new(1, 9999);
    assert_eq!(p.per_page, 100, "per_page should be capped to MAX_PER_PAGE");
}

#[test]
fn paginate_offset_calculation() {
    let p = Paginate::new(3, 20);
    assert_eq!(p.offset(), 40, "page 3 with per_page 20 => offset 40");
    assert_eq!(p.limit(), 20);
}

#[test]
fn paginate_helper_method() {
    let p = Paginate::new(2, 10);
    let paginated = p.paginate(vec!["a", "b"], 50);
    assert_eq!(paginated.page, 2);
    assert_eq!(paginated.per_page, 10);
    assert_eq!(paginated.total, 50);
    assert_eq!(paginated.items.len(), 2);
}

// ─── CursorPaginate Extractor ───────────────────────────────────────────────

#[test]
fn cursor_paginate_first_page() {
    let cp = CursorPaginate::new(None, 25);
    assert!(cp.is_first_page());
    assert_eq!(cp.after(), None);
    assert_eq!(cp.limit(), 25);
}

#[test]
fn cursor_paginate_with_cursor() {
    let cp = CursorPaginate::new(Some("abc123".to_string()), 50);
    assert!(!cp.is_first_page());
    assert_eq!(cp.after(), Some("abc123"));
    assert_eq!(cp.limit(), 50);
}

#[test]
fn cursor_paginate_clamps_per_page() {
    let cp = CursorPaginate::new(None, 500);
    assert_eq!(cp.limit(), 100, "per_page should be capped to MAX_PER_PAGE");
}

// ─── Paginated<T> Response ──────────────────────────────────────────────────

#[test]
fn paginated_total_pages_calculation() {
    let p: Paginated<String> = Paginated {
        items: vec!["a".into(), "b".into()],
        page: 1,
        per_page: 10,
        total: 25,
    };
    assert_eq!(p.total_pages(), 3, "25 items / 10 per page = 3 pages");
}

#[test]
fn paginated_has_next_prev() {
    let p: Paginated<String> = Paginated {
        items: vec![],
        page: 2,
        per_page: 10,
        total: 30,
    };
    assert!(p.has_next(), "page 2 of 3 should have next");
    assert!(p.has_prev(), "page 2 should have prev");

    let first: Paginated<String> = Paginated {
        items: vec![],
        page: 1,
        per_page: 10,
        total: 30,
    };
    assert!(!first.has_prev(), "page 1 should NOT have prev");

    let last: Paginated<String> = Paginated {
        items: vec![],
        page: 3,
        per_page: 10,
        total: 30,
    };
    assert!(!last.has_next(), "last page should NOT have next");
}

// ─── CursorPaginated<T> Response ────────────────────────────────────────────

#[test]
fn cursor_paginated_basic() {
    let cp: CursorPaginated<String> = CursorPaginated {
        items: vec!["item1".into(), "item2".into()],
        next_cursor: Some("cursor_xyz".to_string()),
        has_more: true,
    };
    assert_eq!(cp.items.len(), 2);
    assert_eq!(cp.next_cursor.as_deref(), Some("cursor_xyz"));
    assert!(cp.has_more);
}

#[test]
fn cursor_paginated_last_page() {
    let cp: CursorPaginated<String> = CursorPaginated {
        items: vec!["last".into()],
        next_cursor: None,
        has_more: false,
    };
    assert!(!cp.has_more);
    assert!(cp.next_cursor.is_none());
}
