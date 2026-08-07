//! Cursor-based paging (MOD-STO).
//!
//! Implemented in M1 (WP-C4). The `Store` trait exposes no `OFFSET` parameter
//! anywhere, on purpose: offset paging degrades as a table grows and silently
//! skips or repeats rows when the underlying set changes between pages. A
//! cursor is stable under concurrent writes, which a gradebook needs.
