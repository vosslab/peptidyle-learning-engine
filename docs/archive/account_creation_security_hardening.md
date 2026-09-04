# Account creation security hardening with an isolated passkey capability

## Status

- M0 - Publish the aligned plan: complete.
- M1 - Record the robustness authority: complete.
- M2 - Close Create Instructor Account audit evidence: complete.
- Required account-creation hardening: complete.
- M3 - Attempt the isolated passkey vertical slice: deferred.
- M4 - Live Demo integration: complete and accepted for the deferred-passkey outcome.
- M5 - Security and documentation closeout: complete; archived 2026-09-04.
- Isolated passkey capability: deferred; it does not affect seeded Live Demo, health, ordinary
  session handling, or logout.
- Student activation and Mail.app delivery: separate future work.

## Context

The shared Account architecture remains authoritative, while several prerequisites are
already complete:

- `ProductRole` is immutable and fully replaces `AccountRole`.
- Create Instructor Account generates its Account ID server-side and requires an Active
  Sysadmin session.
- Authenticated Session creation derives and stores Product Role from the Account.
- Account State uses Active, Deactivated, and Closed; deactivation revokes sessions.
- Generic Retry Token and Account-subtype proposals are retired.

The remaining required hardening is durable creator evidence, current security documentation,
and robust failure handling. Initial Sysadmin setup and WebAuthn passkeys remain a desirable but
isolated capability that may be deferred without disabling the seeded Live Demo.

This plan is subordinate to:

1. Direct current human decisions and [Human Guidance](../HUMAN_GUIDANCE.md).
2. [Terminology Contract](../TERMINOLOGY_CONTRACT.md).
3. [Product Roles and Course Membership](../USER_ROLES.md).
4. The remaining durable contracts and design documents.
5. This implementation plan.

The plan agrees with the first three authorities before implementation begins. It does not
schedule changes to `TERMINOLOGY_CONTRACT.md` or `USER_ROLES.md`.

## Objectives

- Record immutable audit evidence identifying the acting Sysadmin whenever Create Instructor
  Account succeeds.
- Correct [Security Model](../SECURITY_MODEL.md) to describe the current Account, Product Role,
  Account State, Authenticated Session, and creation controls accurately.
- Record the user-approved robustness principle in Human Guidance and express its technical
  outcomes in [Failure Recovery](../FAILURE_RECOVERY.md).
- Attempt a complete, shared WebAuthn capability: initial Sysadmin setup, ordinary passkey
  authentication, multiple-passkey management, and real-browser evidence.
- Keep seeded Live Demo entry and unrelated application behavior usable when passkey
  configuration, ceremonies, or individual demo records are unavailable or imperfect.
- Preserve the later Student activation and Mail.app plan as separate work requiring its own
  terminology reconciliation before implementation.

## Design philosophy

- Preserve one shared Account, authentication, passkey, and Authenticated Session architecture
  for all Product Roles.
- Use the existing qualified authority paths:
  - installation administration may establish the first Sysadmin Account;
  - an Active Sysadmin performs Create Instructor Account after Instructor Vetting; and
  - Course Roster Import later resolves or creates Student Accounts under Instructor authority.
- Treat the Terminology Contract and Product Roles and Course Membership as acceptance oracles.
  Private SQL or code identifiers remain implementation details and do not silently establish new
  product vocabulary.
- Capture creator evidence inside the transaction that already verifies the actor.
- Keep optional capability failure local. Passkey failure closes passkey operations while seeded
  demo entry and unrelated routes continue.
- Apply the robustness sequence to imperfect data:
  1. Salvage and normalize the exact item when its meaning and authority remain unambiguous.
  2. Retry the same logical operation from a clean run when ephemeral state or a transient
     dependency could produce a better result.
  3. Quarantine or skip only the irrecoverable item and continue unaffected work.
  4. Fail closed at the affected security or integrity boundary instead of substituting guessed
     data.
- Distinguish runtime resilience from acceptance. The demo can remain available while a passkey
  lane is deferred; deferred behavior cannot be documented as implemented or accepted.

## Scope

### Required hardening

- Consolidate the existing security-plan drafts into this tracked active plan.
- Add qualified immutable audit evidence to Create Instructor Account.
- Preserve server-generated Account IDs, server-derived Product Role, Account State, and shared
  sessions.
- Add permanent behavior tests and proportional mechanism evidence.
- Record the approved robustness rule in Human Guidance and Failure Recovery.
- Update Security Model and lower-authority affected documentation.

### Deferable passkey capability

- A one-shot installation CLI for creating the first Sysadmin and initiating passkey enrollment.
- PostgreSQL-backed WebAuthn ceremonies and credential storage.
- Discoverable passkey sign-in.
- Multiple-passkey listing, addition, reauthentication, and revocation.
- Setup, sign-in, and account-passkey Browser Surfaces.
- Isolated real-stack and real-browser evidence.

## Non-goals

- No edits to `TERMINOLOGY_CONTRACT.md` or `USER_ROLES.md` under this plan.
- No per-role Account, credential, or session implementation and no Account subtype relations.
- No web operation that creates a Sysadmin.
- No second-Sysadmin workflow or lost-all-passkeys recovery workflow.
- No password, TOTP/Google Authenticator code, email-code delivery, or SMTP/provider
  implementation.
- No Student Account creation, Course Roster Import, invitation-link production, or Mail.app
  delivery.
- No Instructor-account creation HTTP route or UI.
- No readiness dependency that makes the entire Live Demo unavailable merely because passkeys are
  unavailable.
- No claim that completing authentication alone completes every teaching workflow in the Live
  Demo.

## Authority-alignment checklist

- [x] Read and follow guidelines in `TERMINOLOGY_CONTRACT.md` and `USER_ROLES.md`; treat these files
      as read only authority documents.
- [x] Use Account, Sysadmin Account, Active Account, Product Role, Account State, Create Instructor
      Account, passkey, and Authenticated Session with their existing meanings.
- [x] Preserve qualified Account-creation paths: installation administration for the first
      Sysadmin, an Active Sysadmin for Create Instructor Account after Instructor Vetting, and future
      Instructor-owned Course Roster Import for Student Accounts.
- [x] Treat installation setup credential and SQL event names as private implementation
      descriptions, not new canonical product terms.
- [x] Preserve server-derived Product Role and the separation between Product Role and Course
      relationships.
- [x] Require a separate authority-level review if implementation exposes a genuine
      contradiction.

The ignored `plan-student-activation-plan.md` is stale planning evidence. Its generic
account-creation and signup language has no authority over current implementation. It must be
updated separately after this plan and before its own implementation.

## Decision reconciliation

| Original decision                                                    | Revised disposition                                                         |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Shared authentication/session implementation                         | Preserved                                                                   |
| No Account subtype relations                                         | Preserved                                                                   |
| Account State handles restriction and revocation                     | Implemented; retain and test                                                |
| Server-generated Account IDs                                         | Implemented                                                                 |
| Session derives Product Role and retains its foreign-key-pinned copy | Implemented; retain unchanged                                               |
| Creation records its actor transactionally                           | Preserved through qualified audit evidence                                  |
| Generic creator column on every Account                              | Replaced by qualified evidence because creation paths have different actors |
| Generic Retry Token                                                  | Rejected                                                                    |
| No route or onboarding UI                                            | Superseded only if the isolated passkey capability is accepted              |
| Permanent rules plus one-time mechanism evidence                     | Preserved                                                                   |
| Product Role and Deactivated terminology                             | Implemented and mandatory                                                   |

## Required account-creation changes

- Allocate `2026090401_account_creation_audit.sql`.
- Add `ple_audit.instructor_account_creation_event` with:
  - server-generated event identity;
  - the created Instructor Account;
  - the acting Sysadmin Account;
  - database-authoritative occurrence time; and
  - role-pinning foreign keys for Instructor subject and Sysadmin actor.
- Replace the private Create Instructor Account function through the new migration while keeping
  `ple_api.create_instructor_account(text, text)` and its result shape unchanged.
- Capture `current_session_account_id()` once, validate that it is an Active Sysadmin Account, and
  use it for both authorization and audit evidence.
- Commit the Account, Instructor Authentication Email, initial Account State, and audit event in
  one transaction.
- Protect the audit relation with forced RLS, revoked runtime table access, no update/delete path,
  and a narrow audit writer.
- Store no email, passkey label, raw credential, or browser data in the creation event.

## Robustness and recovery contract

Add this approved principle to Human Guidance:

> Robust behavior keeps unaffected work available when data is imperfect: salvage an exact
> trustworthy item when possible, retry the same logical work from a clean run when that can
> improve the outcome, and skip or quarantine only the irrecoverable item while continuing.
> Security and integrity violations still fail closed at their affected boundary.

Extend Failure Recovery without changing its committed, rejected, retryable, and indeterminate
model:

- **Salvageable:** normalize or reconstruct only from authoritative facts; preserve the original
  evidence when needed; never guess identity, Product Role, authority, credential state, or a
  committed outcome.
- **Clean retry:** discard only ephemeral attempt state and repeat the same logical operation.
  Preserve durable operation identity where an earlier commit may exist.
- **Irrecoverable item:** quarantine, revoke, or omit that exact item and continue the batch, page,
  demo, or unrelated capability.
- **Security-sensitive loss:** fail the affected credential or operation closed while keeping
  unrelated Accounts, personas, routes, and services available.
- **Disposable Live Demo data:** the owner may perform one clean, owner-scoped regeneration when
  corrupt disposable state could explain the result. A repeated deterministic failure becomes a
  reported defect or deferred capability rather than an endless reset loop.
- **Persistent or production-like data:** never delete or recreate it merely to make a gate pass.

## Deferable passkey design

### Isolation boundary

- Passkey configuration and routes are optional capabilities assembled beside the existing session
  and seeded-demo routers.
- Missing, malformed, or unavailable passkey configuration produces a bounded
  passkey-unavailable result and a safe diagnostic.
- The server, health route, current session route, logout, and seeded persona entry remain usable.
- Passkey availability is not part of whole-demo readiness.
- The Browser Surface hides or disables passkey controls with a clear status when the capability is
  unavailable; seeded entry remains visible.

### Installation setup

If the capability reaches implementation:

```text
cargo tools database bootstrap-first-sysadmin --confirm-empty-installation
cargo tools database rotate-initial-sysadmin-setup-credential --confirm-uncredentialed-sysadmin
```

- The commands use only `PLE_MIGRATION_DATABASE_URL`, require the migration principal, and verify
  schema compatibility.
- Bootstrap requires an empty Account table and no earlier installation-bootstrap evidence.
- It creates one Active Sysadmin Account and a 256-bit, ten-minute setup credential.
- The raw credential is printed once; PostgreSQL stores only its SHA-256 hash.
- The credential is supplied only in an HTTPS request body, never a URL, image, tracked file,
  process argument, or log.
- Successful first-passkey registration consumes it atomically.
- Rotation is available only while the sole Sysadmin has no registered passkey.
- If passkeys are deferred, these commands and credential issuance remain unavailable. No dormant
  privileged credential is created.

### Passkey behavior

If accepted:

- Use the existing private passkey and ceremony foundation.
- Require discoverable credentials and WebAuthn user verification.
- Use a random WebAuthn user handle rather than exposing an Account UUID or email.
- Support multiple active passkeys with bounded, case-insensitively unique Account-local labels.
- Persist validated credential state and signature-counter changes.
- Require fresh passkey reauthentication before adding or revoking a passkey.
- Refuse revocation of the last active passkey.
- Require Active Account State for setup, registration, management, authentication, and session
  issuance.
- Use separate short-lived, browser-bound setup, ceremony, and management cookies; retain the
  existing ordinary session cookie unchanged.

### Imperfect passkey data

- A committed passkey row after a lost browser response is salvageable: reload the Account's
  authoritative passkey list and continue.
- An uncommitted or expired ceremony is retryable through a fresh ceremony with the same intended
  operation.
- A malformed or corrupt credential record is skipped for authentication and never coerced into a
  valid credential.
- Other valid passkeys for that Account remain usable.
- A corrupt last passkey produces a bounded recovery-required result; it does not crash the server
  or permit guessed recovery.
- Duplicate labels are correctable input; duplicate credential material is rejected.
- Replay, wrong-browser binding, wrong Account, invalid signature, and inactive Account results
  fail only the affected operation.

## Live Demo resilience

- Preserve the closed allowed persona set while allowing the runtime to retain valid personas if
  one configured persona is missing or invalid.
- An individual bad persona mapping is omitted or returns a bounded unavailable result; it cannot
  be coerced into another Product Role.
- If at least one valid seeded persona remains, the seeded entry page continues to work and
  identifies that some demo Accounts are unavailable.
- If no persona is valid, the seeded-entry capability is unavailable while the server and non-demo
  routes remain healthy.
- The disposable-stack owner may try one clean regeneration when seed state appears transiently
  damaged.
- Browser tests and reports distinguish:
  - baseline seeded-entry availability;
  - degraded but usable seeded entry;
  - passkey capability accepted; and
  - passkey capability deferred.
- A passkey test failure does not erase successful seeded-entry evidence or prevent someone from
  using the rest of the Live Demo.

## Milestones

### M0 - Publish the aligned plan

- Consolidate the two ignored security-plan drafts into this active plan.
- Complete the authority-alignment checklist against Human Guidance, Terminology Contract, and
  Product Roles and Course Membership.
- Remove the superseded root copies after content reconciliation.
- Run changelog rotation and record plan adoption.
- Exit gate: the plan requires no authority-document changes.

### M1 - Record the robustness authority

- Add the approved concise rule to Human Guidance.
- Add its technical classification and recovery sequence to Failure Recovery.
- Verify consistency with committed, rejected, retryable, and indeterminate outcomes.
- Run Markdown/link checks and update the changelog.
- Exit gate: salvage, clean retry, exact-item skip, and affected-boundary refusal have
  non-conflicting definitions.

### M2 - Close Create Instructor Account audit evidence

**Complete.** Migration `2026090401` records the qualified Active Sysadmin actor in
immutable audit evidence within the Create Instructor Account transaction. The focused connected
PostgreSQL acceptance and independent security review passed.

- Implement the allocated audit migration, narrow audit writing, and connected acceptance.
- Add behavior-focused tests for actor binding, atomicity, denial, immutability, and Account State.
- Produce one-time privilege and mechanism evidence separately.
- Update the changelog after the narrow gate passes.
- Exit gate: every successful Create Instructor Account operation records its Active Sysadmin
  actor, and failed operations leave no partial state.

### M3 - Attempt the isolated passkey vertical slice

**Deferred.** Two clean disposable `webauthn-rs` start/finish attempts reached the same persistent
Store-contract blocker: PLE cannot durably create, retrieve, or atomically consume the opaque
discoverable-ceremony state and validated credential state required for cryptographic completion.
No passkey route, Browser Surface, setup credential, installation command, session issuance, or
completion claim was retained. The deferred capability is independent of seeded demo entry,
health, ordinary session handling, and logout.

- Start with an optional, disabled-by-default capability boundary.
- Implement installation bootstrap, PostgreSQL ceremonies, WebAuthn validation, ordinary session
  issuance, and multiple-passkey management as one vertical slice.
- Keep setup-credential issuance disabled until its consuming registration path passes focused
  tests.
- Apply salvage and fresh-run recovery during implementation.
- If the same non-transient failure remains after a controlled clean retry, classify the passkey
  capability as deferred, leave it disabled, and retain no setup credential or passkey completion
  claim.
- Exit outcomes:
  - **Accepted:** complete passkey behavior and focused security gates pass.
  - **Deferred:** baseline hardening remains valid, passkey routes stay absent or disabled, and the
    reason is recorded.

### M4 - Live Demo integration

Status: complete and accepted for the deferred-passkey outcome. Seeded persona configuration now
salvages each unambiguous valid mapping, reports omitted records without exposing their details,
and isolates a zero-valid-persona configuration to seeded entry while health, ordinary session,
and logout routes remain available. The Browser Surface retains usable choices and reports the
bounded unavailable count. Its ordinary session path was exercised for a retained Student persona.
The browser-scenario registry now selects the baseline seeded-entry/session/logout/course-boundary
journey and no longer requires absent passkey scenarios; this is registry reconciliation, not a
claim of full real-stack browser acceptance, which remains M5 evidence.

- Always verify that seeded persona entry, session resolution, logout, and unrelated demo routes
  remain usable.
- If M3 is accepted, add passkey setup, sign-in, and management surfaces and restore the existing
  named Morgan and Elena passkey journeys.
- If a scenario contains salvageable committed state, reload it; if ephemeral state may be
  responsible, rerun once in a fresh context; if the exact item is irrecoverable, skip it, continue
  the remaining demo checks, and report the passkey scenario as deferred or failed.
- Exit gate: the Live Demo remains usable in both accepted-passkey and deferred-passkey outcomes.

### M5 - Security and documentation closeout

Implementation and verification are complete. Focused checks, connected PostgreSQL acceptance,
baseline Live Demo acceptance, and the complete validation suite passed. Lower-authority security,
database, contract, live-demo, and test-evidence documentation records the accepted required
hardening and the deferred passkey capability without a passkey-completion claim. The required
history-preserving `git mv` archival completed on 2026-09-04; this archived plan records those
separate outcomes.
- Run focused tests, connected PostgreSQL acceptance, baseline Live Demo acceptance, and the
  complete Validation suite.
- If passkeys are accepted, add real-browser automation and a one-time human exercise using two
  actual passkeys.
- If passkeys are deferred, record that status without blocking baseline Live Demo availability or
  claiming passkey acceptance.
- Update lower-authority documentation from accepted evidence.
- Archive the plan with separate statuses for required account hardening and deferable passkeys.

## Test cases and gates

### Required permanent coverage

- Create Instructor Account records exactly one event naming its Active Sysadmin actor.
- Invalid, inactive, non-Sysadmin, duplicate-email, and transactional-failure cases create no
  Account, Authentication Email, or event.
- Product Role remains immutable.
- Authenticated Session Product Role remains derived from the Account.
- Deactivation prevents new sessions and revokes existing sessions.
- Valid seeded personas remain usable when another persona mapping is absent or invalid.
- An invalid optional capability does not crash the server or remove unrelated routes.
- Salvage, clean retry, and exact-item skip behavior is deterministic and does not guess authority
  or committed state.

### Passkey coverage when accepted

- Setup credential expiry, rotation, replay, wrong-browser use, and concurrent completion.
- Registration, discovery authentication, counter updates, and session issuance.
- Multiple passkeys, fresh reauthentication, safe revocation, and last-passkey refusal.
- Corrupt single-passkey isolation while other valid credentials remain usable.
- Strict Origin/Host enforcement, `no-store` responses, rate limiting, and secret redaction.
- Morgan and Elena enrollment, sign-out, and passkey sign-in against the real HTTPS stack.
- One-time manual acceptance with two real passkeys.

Run narrow gates first, then:

```text
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
source source_me.sh && python3 local_stack.py acceptance
source source_me.sh && ./all_test.sh
```

A passkey-specific lane joins `all_test.sh` only after it is stable and accepted. Before that, it
runs as an explicit isolated lane whose failure cannot make the baseline Live Demo unavailable.

## Documentation closeout

- Update Security Model unconditionally for the current Account, Product Role, Account State,
  Authenticated Session, and Create Instructor Account controls.
- Correct stale `auth_session`, `course_member`, and archived-plan authority references.
- Add passkey details to the security model only if the passkey capability is accepted. Otherwise
  retain clear foundation/future wording.
- Treat [Live Demo Specification](../LIVE_DEMO_SPEC.md) as verification-first:
  - make no change merely because passkey code was attempted;
  - update it only for browser behavior that is accepted and remains available on a clean run;
  - preserve seeded-entry availability and all existing limitations; and
  - describe degraded behavior without converting optional passkeys into whole-demo readiness.
- Read and follow guidelines in `TERMINOLOGY_CONTRACT.md` and `USER_ROLES.md`; treat these files as
  read only authority documents.
- Synchronize lower-authority affected documents, migration counts, installation guidance,
  test-evidence descriptions, and the changelog according to the accepted or deferred outcome.
- Before adopting `plan-student-activation-plan.md`, run a separate terminology-plan update against
  the then-current Human Guidance, Terminology Contract, and Product Roles and Course Membership.
  No implementation proceeds from its current stale wording.

## Assumptions and locked defaults

- Required account-creation hardening may close even if the isolated passkey capability is
  explicitly deferred.
- Passkey deferral leaves the present seeded Live Demo entry available and leaves first-Sysadmin
  installation setup acceptance open.
- No installation setup credential exists unless a tested consumer can complete and consume it.
- Neil remains the intended sole trial Sysadmin.
- Passwords, TOTP, email codes, and email delivery remain absent.
- Student Authentication Email remains immutable.
- Course Roster Import remains the future Instructor-owned Student Account resolution/creation
  boundary.
- Security and integrity failures close their exact operation; imperfect unrelated data does not
  crash the application.
