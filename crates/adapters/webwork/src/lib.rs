//! MOD-ADP-WW: the WeBWorK PG adapter.
//!
//! PG questions are rendered by a separate renderer service, never in this
//! process. The renderer has no public endpoint and no database access, and a
//! repeat `(version_id, seed)` is served from the render cache without
//! touching it. With the renderer stopped, only WeBWorK questions degrade --
//! that isolation is the M4 acceptance criterion.

/// PG source handling and the renderer client boundary.
pub mod pg_parser_stub;
