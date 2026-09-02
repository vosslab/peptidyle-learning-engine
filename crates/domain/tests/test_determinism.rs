//! Native half of the WP-C5 cross-target determinism gate.

mod determinism_support;

#[test]
fn deterministic_seed_vector_fixture_set_matches_native_generation() {
    determinism_support::assert_committed_deterministic_seed_vector_fixture_set();
}
