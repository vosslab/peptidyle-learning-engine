//! Generation specifications: the input side of seeded generation (WP-C1).
//!
//! A generation spec plus a seed must fully determine a variant. Nothing here
//! may read the clock, the environment, or a random source, because the same
//! spec runs on the server and in the browser and the two must agree byte for
//! byte (WP-C5 seed parity).
