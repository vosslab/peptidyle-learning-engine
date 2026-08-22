# WP-RC8 production account composition review

## Verdict

ACCEPTED for the repository-owned production-composition slice, subject to the
documentation correction below. The code sends the production entry point to
the PLE-owned passwordless account graph without a fake `IdentityProvider`.
Local development remains explicit and operationally compatible with the
existing Compose launcher.

## Contract review

- `production_router_from_env` constructs only `PersistentDependencies` and
  calls `production_router` ([composition.rs](../../../crates/server/src/composition.rs):80).
  `ProductionSettings::from_env` did not read the three local-identity
  variables; their reads remained confined to the now-retired
  `local_development_authentication_from_env` in `local_identity.rs`.
- The shared router is factored once. Production uses
  `compose_passwordless_router`, while local development layers
  `crate::auth::router` on it ([backend.rs](../../../crates/server/src/composition/backend.rs):160).
  This preserves account, invitation, course-session, and passkey routes but
  leaves `/api/auth/login` out of the production graph
  ([router.rs](../../../crates/server/src/composition/router.rs):139).
- Production selects `ReviewNotRequired`, whose documented default permits
  publication ([backend.rs](../../../crates/server/src/composition/backend.rs):173;
  [capabilities.rs](../../../crates/server/src/catalog/capabilities.rs):60).
  Local development still receives its denying gate
  ([composition.rs](../../../crates/server/src/composition.rs):91).
- The production session policy is exactly eight hours with
  `FirstPartyHttps` ([backend.rs](../../../crates/server/src/composition/backend.rs):255).
  The existing cookie contract makes that transport Secure, HttpOnly, host-only,
  and `SameSite=Lax` ([auth.rs](../../../crates/server/src/auth.rs):502).
- The binary's exact selector defaults to production for unset or `0`, accepts
  only `1` for local development, and rejects other values
  ([main.rs](../../../crates/server/src/main.rs):30). The existing Compose
  launcher supplies `PLE_AUTH_PROVIDER=local-file`, the required enable flag,
  and local identity file ([compose.yaml](../../../containers/compose.yaml):128),
  so it continues to enter the local path.
- This is native Rust service composition. There is no `wasm32`, wasm-bindgen,
  browser adapter, or TypeScript change in the reviewed diff; no Wasm boundary
  work is required for this slice.

## Evidence

The following independent commands passed from the repository root on
2026-08-10:

```text
cargo fmt --check
cargo check -p server_core
cargo test -p server_core composition::tests -- --nocapture
cargo test -p server_core --bin server_core \
  tests::local_development_router_selection_is_exact_and_fail_closed -- --exact
cargo test -p server_core auth::tests::cookie_attributes_match_the_selected_transport -- --exact
cargo clippy -p server_core --all-targets --all-features -- -D warnings
source source_me.sh && python3 tests/check_ascii_compliance.py -i \
  docs/active_plans/workstreams/wp_rc8_production_account_composition.md
source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q
git diff --check -- crates/server/src/composition.rs \
  crates/server/src/composition/backend.rs crates/server/src/composition/router.rs \
  crates/server/src/composition/tests/mod.rs crates/server/src/main.rs
```

Results: all Cargo checks passed; 20 composition tests passed; both focused
tests passed; the Markdown link test passed 136 cases; and the scoped diff
check and ASCII check were clean.

## Finding

### P2: workstream evidence counts six commands as four

- File: [wp_rc8_production_account_composition.md](../workstreams/wp_rc8_production_account_composition.md):44
- Evidence: the fenced command block at lines 35-41 contains six commands,
  while line 44 says "All four commands passed."
- Fix: change "four" to "six" (or reduce the block to the four intended
  commands). Keep the surrounding result summary aligned with the actual
  executed command set.
- Test: rerun the listed commands and the Markdown ASCII/link checks after the
  wording correction.

## Boundaries retained

The reviewed source and workstream do not claim an external SMTP delivery,
passkey-bootstrap, multi-replica, browser walkthrough, or production rollout
acceptance. The remaining WP-RC8 acceptance work described by the release plan
is still outside this slice.
