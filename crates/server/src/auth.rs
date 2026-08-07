//! Authentication, sessions, and the middleware stack (MOD-API-AUTH).
//!
//! Implemented in M2. Sessions must survive a replica change, so no session
//! state may live in process memory.
