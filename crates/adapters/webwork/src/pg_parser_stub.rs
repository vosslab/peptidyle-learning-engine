//! PG source handling and the renderer client boundary (MOD-ADP-WW).
//!
//! Implemented in M4. Two decisions are deferred to that entry point and are
//! recorded as open in the plan: whether PG source is stored locally or
//! referenced remotely (an OPL licensing question), and the render-cache key
//! shape, which must be `(version_id, seed)` so a repeat is a cache hit.
