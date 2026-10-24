//! `layout.pages.insert` clamps an out-of-range `after` to the end instead of overflowing.
use designcraft_engine::Session;
use serde_json::json;

#[test]
fn insert_after_u64_max_appends_at_the_end() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"pages":2})).unwrap();
    let before: Vec<_> = s.doc().unwrap().doc.pages().map(|(_, _, p)| p.id).collect();
    let out = s.execute("layout.pages.insert", &json!({"after": u64::MAX, "count": 2})).unwrap();
    assert_eq!(out["total"], 4);
    let after: Vec<_> = s.doc().unwrap().doc.pages().map(|(_, _, p)| p.id).collect();
    assert_eq!(after[..2], before[..], "the original pages must stay first; new pages go at the end");
    assert!(after[2..].iter().all(|id| !before.contains(id)));
}

#[test]
fn insert_before_zero_inserts_at_the_beginning() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"pages": 2})).unwrap();
    let original: Vec<_> = s.doc().unwrap().doc.pages().map(|(_, _, p)| p.id).collect();
    let out = s.execute("layout.pages.insert", &json!({"before": 0, "count": 1})).unwrap();
    assert_eq!(out["total"], 3);
    let after: Vec<_> = s.doc().unwrap().doc.pages().map(|(_, _, p)| p.id).collect();
    assert_eq!(after.len(), 3);
    assert!(!original.contains(&after[0]), "new page must be at index 0");
    assert_eq!(after[1..], original[..], "original pages shifted to index 1 and 2");
}
