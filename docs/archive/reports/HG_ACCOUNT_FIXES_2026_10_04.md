# HG Instructor Account fixes - 2026-10-04

## Implemented boundary

- Removed the in-PLE Instructor vetting receipt, endpoint, database table, audit foreign key, and the `Verified Instructor` language.
- A Sysadmin now creates an Instructor Account in one request with normalized email, first name, last name, and affiliation. PostgreSQL validates all fields, derives the active Sysadmin from the installed session, and writes the Account, authentication email, Instructor Profile, and creation audit event atomically.
- `ple_private.instructor_profile` is the single source for Instructor names and affiliation. `ple_private.instructor_display_name` and `ple_private.instructor_affiliation` provide narrow projections for existing content and search queries.
- Added a configured SMTP delivery adapter for the fixed setup notice. The recipient is read only through a Sysadmin-authorized server-side Store method and never reaches browser DTOs or logs. `PLE_INSTRUCTOR_SETUP_SMTP_URL`, `PLE_INSTRUCTOR_SETUP_FROM`, and HTTPS `PLE_BROWSER_ORIGIN` enable it; absent configuration returns `setupEmailSent: false` instead of a false success. The notice includes the configured `/sign-in` URL. Production SMTP requires `smtps` or `smtp` with `tls=required`.
- The create response includes the new Account summary and `setupEmailSent`; retry delivery is available at `POST /api/instructor-accounts/{account_id}/send-setup-email`.

## Security evidence

- ASVS 2.2.1/2.2.2: client and trusted PostgreSQL boundaries validate exact field shapes and bounds.
- ASVS 2.3.3: Account, email, Profile, and audit write share the database transaction.
- ASVS 8.2.1/8.3.1: the database derives the active Sysadmin; a request cannot choose an actor, role, or delivery destination.
- ASVS 14.2.6: Authentication Email remains server-only and setup delivery does not log it.

## First-time sign-in

The existing browser-bound email-code ceremony now uses the same configured SMTP delivery adapter. The setup email only points to the configured sign-in page; it is not a bearer credential. The browser requests and completes the ordinary one-time email code, so the same safe path works for first and later sign-in. Email-code routes are absent when delivery is unconfigured, leaving the seeded Live Demo entry available. The database accepts five valid start requests per normalized-address ten-minute window. Eligible and non-eligible addresses use the same provider path and public response; non-eligible delivery carries a non-persisted cover code that cannot create a Session. The cover delivery is necessary in this synchronous, no-outbox flow: returning generic success without delivery would expose an Account when a real provider failure produces an error. It remains bounded by the same per-address rate limit and has no stored challenge, Account lookup result, or session effect. A provider failure returns no success response and leaves any earlier valid code usable; only a provider-accepted replacement invalidates earlier codes.

## Validation status

- `check_rust.sh` passed the complete workspace, strict Clippy, both test feature configurations, doctests, and WebAssembly checks.
- Both TypeScript typechecks and the production browser bundle pass. The email-code client tests pass.
- `cargo check -p server_core`, `cargo check -p learning-data-access --features postgres`, `cargo test -p server_core email_code --lib` (3), and `schema_style/check_schema_style.py` passed. The explicit local in-memory capture transport verifies the sent destination/code and covers Instructor session issuance, missing/different browser binding, malformed/wrong/reused/expired codes, unavailable provider, and a provider failure that preserves an earlier usable code.
- Real provider acceptance and inbox delivery remain pending the external mail-provider configuration. The unconfigured adapter fails closed and reports `setupEmailSent: false`.

The live-start integration check caught a configuration mismatch: the required
browser origin was treated as a partial SMTP configuration. The optional-mail
branch now accepts an independently configured browser origin. The regression
`composition::tests::browser_origin_does_not_require_an_email_provider` passes,
and the full Rust workspace gate passes with that correction.

A one-time disposable PostgreSQL 17 replay installed the full current schema
with the normal migration-principal bootstrap. It verified the `ple_auth`
EXECUTE grants and exercised prepare, commit, and consume as that role for an
active Instructor. A different browser-binding hash returned no Account; the
correct binding returned the Instructor; reuse returned no Account. The
container and temporary fixture were removed. This proves the database path,
separately from the local capture tests and the still-pending real inbox check.

The running Live Demo passed the Instructor-account service and browser lanes.
They cover Sysadmin creation with the four required fields, deactivation and
reactivation, retained authored content and Course/Student work, and denied
access for other roles. Provider-unconfigured creation returns
`setupEmailSent: false`; the browser reports that delivery did not occur.
Service evidence is in `/private/tmp/hg_live_instructor_accounts.log`; its initial
browser launch hit the macOS execution boundary, and the unchanged browser retry
passed in `/private/tmp/hg_live_instructor_accounts_browser.log`. The refreshed
247-image corpus also captures the account fields and delivery result.
