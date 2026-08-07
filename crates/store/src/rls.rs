//! Row-level security policies and tenant context (MOD-SCHEMA).
//!
//! Implemented in M2. This module owns the one path that sets the tenant
//! context for a connection, so there is exactly one place to audit.
//!
//! The context is derived from the authenticated session. It is never read
//! from a request parameter, a header, or a JSON body -- a client that can
//! name its own tenant has no isolation at all. The M2 gate is a foreign
//! tenant context returning zero rows.
