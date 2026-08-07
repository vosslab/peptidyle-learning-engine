//! Attempt lifecycle and the attempt state machine (MOD-RUN, MOD-STATE).
//!
//! Implemented in M2. The shape to preserve: `apply(state, event)` is a pure
//! transition function, so every legal transition and one rejected illegal one
//! are testable without a database, a clock, or a server.
