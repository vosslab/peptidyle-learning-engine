//! Native half of the WP-C5 cross-target determinism gate.

mod determinism_support;

#[test]
fn committed_seed_vectors_match_native_generation() {
    determinism_support::assert_committed_seed_vectors();
}
