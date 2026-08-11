# WP-RC8 production account composition

## Scope and result

`production_router_from_env` now enters the PLE-owned direct passwordless
account/session route graph. It no longer reads `PLE_AUTH_PROVIDER`,
`PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH`, or `PLE_LOCAL_AUTH_FILE`, and it does not
mount the file-backed provider's legacy `/api/auth/login` route.

Production uses an eight-hour `FirstPartyHttps` session policy. Therefore the
account, email-binding, and tenant-session cookies are Secure, HttpOnly, and
first-party `SameSite=Lax`. The default `ReviewNotRequired` gate is explicit:
the institutional review integration remains optional and off by default.

The separately callable `local_development_router_from_env` preserves the
explicit local-file provider, plain-HTTP development cookie policy, legacy
login route, and fail-closed public-publication gate. The binary selects that
launcher only when `PLE_ENABLE_LOCAL_DEVELOPMENT_AUTH=1`; unset or `0` selects
production, and every other explicit value fails before startup.

## Deliberate boundaries

- This slice does not claim a successful send through an external SMTP
  provider, passkey bootstrap acceptance, multi-replica acceptance, or browser
  walkthrough completion.
- No Wasm or TypeScript/browser identity behavior changed.
- The shared account route graph is factored once; local composition layers its
  provider-backed legacy routes over it, rather than using a dummy provider.

## Focused evidence

Run from the repository root:

```text
cargo fmt --check
cargo check -p server_core
cargo clippy -p server_core --all-targets --all-features -- -D warnings
cargo test -p server_core composition::tests -- --nocapture
cargo test -p server_core --bin server_core \
  tests::local_development_router_selection_is_exact_and_fail_closed -- --exact
cargo test -p server_core auth::tests::cookie_attributes_match_the_selected_transport -- --exact
```

All six commands passed on 2026-08-10. The composition tests verify the
production-style graph mounts passwordless, invitation, course-session, and
passkey routes while `/api/auth/login` is 404; the local graph retains that
legacy route; and the production policy is exactly 28,800 seconds over
first-party HTTPS. The binary-selector and cookie-attribute tests also pass.
