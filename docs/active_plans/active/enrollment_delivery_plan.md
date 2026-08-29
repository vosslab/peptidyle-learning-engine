# Enrollment delivery plan

This active plan owns enrollment delivery status, implementation packages,
acceptance gates, non-goals, and the maintainer checklist. The durable product
contract remains in [ENROLLMENT_DESIGN.md](../../ENROLLMENT_DESIGN.md); this plan
must not become a second identity, authorization, Store, HTTP, privacy, or
user-experience authority.

## Delivery status

**Implemented source and generic-route components:** PLE owns opaque global
accounts, short-lived browser-bound email-authentication and account-session
components, discoverable WebAuthn registration and authentication, multiple
passkeys, verified account-email replacement, and account-to-course-context
selection. The generic router mounts those account routes with the course
roster, invitation, bulk import, atomic enrollment, and calculated roster score
CSV export routes. Memory and PostgreSQL implement the same Store contract,
including canonical course-membership episodes, the Student identity, derived
assignment entitlement, and first-event materialization of one assignment
receipt with its typed empty summary and immutable provenance. Roster and
assignment writes do not eagerly create learner records.

**Current startable composition:** `production_router_from_env` builds the
persistent dependencies and composes the PLE passwordless/account/session graph
with an eight-hour `FirstPartyHttps` policy and explicit `ReviewNotRequired`.
The local browser uses that same graph: email authentication is canonical,
ordinary passkeys are optional shortcuts, and a seeded persona selector is
available only when its deployment configuration is complete. The selector
enters the same persisted account/session state; it is not a separate identity
or membership model.

**Production acceptance still open:** canonical email-authentication evidence
needs a live operator-selected external SMTP provider test account. Optional-
passkey and multi-replica journeys plus deployment acceptance remain. PLE does
not own a mail server, sender reputation, or deliverability stack.

Current route truth remains in [API_CONTRACTS.md](../../API_CONTRACTS.md) and
[routing.rs](../../../crates/server/src/course/routing.rs). Focused Graphify
navigation identified `UserId` at `crates/question_model/src/auth.rs`,
`StudentId` at `crates/question_model/src/activity.rs`, and the roster HTTP
boundary at `crates/server/src/course/roster.rs`; direct current-source review
is authoritative for implementation conclusions.

## Implementation packages

The safe dependency order is:

### ENR1: PLE identity and account

The source and generic route components are implemented. The repository-owned
production composition is implemented and independently reviewed; package
acceptance remains open.

- Add a production PLE-owned account store keyed by opaque global `UserId`.
- Add short-lived, single-use email authentication for registration and sign-in,
  including browser binding where practical, uniform outward responses, rate
  limits, secret hashing, and redacted diagnostics.
- Add discoverable WebAuthn credentials on the PLE account boundary using an
  established implementation; support multiple passkeys and account-managed
  credential revocation.
- Keep email mutable and separate from `UserId`; require verified account
  control for email changes.
- Add the stable `(CourseId, UserId) <-> StudentId` pedagogical mapping without
  letting browser input select course or identity.
- Keep the PLE-owned email-account path in the production composition;
  optional SSO links to an existing account through the same identity boundary
  when enabled.

### ENR2: Atomic roster Store

Implemented.

- Add focused add, revoke, list, invitation, and bulk-reconcile capabilities.
- Persist course-scoped roster email, roster identifier, allowed domains, and
  invitation lifecycle with the learner-record retention boundary.
- Add a roster revision and course-level lock order.
- Derive assignment entitlement from current membership, audience, and groups.
- Materialize enrollment and summary together only at a bounded first event.

### ENR3: Single-member HTTP

Implemented.

- Mount roster list, invitation creation/copy/redemption, optional established
  SMTP delivery, allowed-domain settings, and access-revocation routes.
- Use existing course concealment and direct-Instructor authorization.
- Add strict request/response DTOs, no-store behavior, idempotency, revisions,
  and safe errors.

### ENR4: Instructor and learner UI

Implemented; acceptance remains open.

- Add the Students screen, invitation flow, learner claim page, initial passkey
  enrollment, multiple-passkey management, and email change.
- Keep the common path course-level and hide internal enrollment mechanics.
- Prove the platform keyboard path before optional shortcuts.

### ENR5: Bulk roster

Implemented.

- Add bounded `email` plus `roster_id` CSV template, institutional LMS export
  profiles, parse/preview, staged revision, and atomic invitation commit.
- Add row status and correction guidance without raw-PII diagnostics.
- Add exact normalized allowed-domain validation and reviewed institutional
  roster-ID profiles without arbitrary instructor regexes.

### ENR6: Integration evidence

Acceptance remains open.

- Replace SQL-arranged membership in the primary multi-actor walkthrough with
  instructor roster action and learner claim/login.
- Keep course and assignment creation as the only arranged setup steps until
  their instructor UI exists; every later membership, login, course-entry,
  learner-work, and gradebook action is walked through the supported surface.
- Exercise copy-link invitation handoff without a PLE-owned mail system. Use a
  test account at the operator-selected SMTP provider for the canonical
  email-authentication ceremony. For connected local evidence, the deployment-
  gated seeded persona selector may enter a seeded account, and the ordinary
  passkey path applies only after the learner account exists.
- Prove gradebook, item analysis, learner isolation, assignment creation after
  roster creation, and roster addition after assignment creation.
- Keep LTI Names and Roles roster synchronization in its separately authorized
  integration package; it must call the same Store command.
- Prove a deterministic calculated roster score CSV keyed by the protected
  course-scoped roster identifier; do not claim it changes the institutional
  system of record.

## Acceptance gates

Permanent behavior and contract tests must prove:

- Instructor, Sysadmin-without-membership, Student, nonmember, and
  foreign-course and foreign-Student authorization outcomes;
- email-authentication and invitation secrets are single-use, bounded, hashed,
  rate-limited, and cannot cross account, browser binding, course, or expiry;
- existing and nonexistent email addresses have the same outward invitation
  and authentication response shape;
- one account may hold multiple passkeys, and revoking one credential does not
  revoke the others or expose credential metadata to course instructors;
- a changed or reassigned email cannot inherit another `UserId` or educational
  record;
- allowed-domain matching uses the complete normalized domain and rejects
  substring, suffix-confusion, and malformed-IDNA cases;
- course roster IDs are unique inside the course, absent from account lookup,
  and present in the intended calculated roster score CSV;
- Student membership cannot create Instructor or Sysadmin authority;
- adding a member creates the membership episode and profile without assignment
  activity rows;
- creating an assignment stores its audience without creating learner rows;
- pre-activity summary reads return `no_activity` without a write;
- concurrent first-event materialization creates exactly one enrollment and
  summary receipt;
- Memory and PostgreSQL implement the same idempotent behavior;
- a failed summary insert rolls back first-receipt enrollment creation;
- removal revokes access without deleting educational records;
- re-addition reuses the learner identity and existing activity;
- bulk preview is bounded and commit is revisioned and idempotent;
- raw roster bytes and expired previews are removed at their documented
  boundary;
- error bodies, exports, and logs exclude provider subjects, passkey metadata,
  invitation secrets, raw CSV cells, and unrelated learner data; and
- current gradebook reads remain empty until an educational event creates a
  summary.

Disposable integration evidence must prove:

- a real non-superuser PostgreSQL role and RLS context enforce the boundary;
- one API replica can create or redeem membership and another can serve the
  resulting course and assignment;
- an instructor can create and copy a learner invitation through the browser
  without SQL, `cargo tools e2e-seed`, or configured SMTP;
- the learner authenticates by email through the configured provider, optionally
  enrolls a passkey, enters the course, starts the assignment, submits, and
  appears in the instructor gradebook;
- the instructor downloads a protected calculated roster score CSV whose roster
  IDs match the imported rows and whose contents exclude account email and
  global `UserId`; and
- the deployment-gated seeded persona selector, PLE passwordless composition,
  optional OIDC/SAML connector, and future LTI adapter converge on the same
  `UserId`, session, and Store operation rather than implementing separate
  roster semantics.

One-time implementation probes may inspect lock order, query plans, migration
backfill, and representative CSV timing. Keep a probe as a permanent test only
when it meets the behavior-focused criteria in [PYTEST_STYLE.md](../../PYTEST_STYLE.md)
and [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

## Non-goals

This enrollment slice keeps the following work in its existing owner:

- automatic LMS Names and Roles roster synchronization remains integration
  work after the core roster and calculated roster score CSV export contract;
- optional institutional OIDC/SAML account linking remains an integration
  inside WP-RC8 and is not required for production passwordless login;
- legal-identity verification and proctoring are not implied by email,
  passkeys, roster IDs, or authenticator user verification;
- course sections use separate PLE courses until teaching evidence requires a
  nested section model;
- co-instructor and grader invitation use a separate role-elevation review;
- assignment subsets require an explicit assignment-audience design; and
- roster removal does not replace retention, archival, legal hold, or account
  deletion.

## Maintainer checklist

Before changing enrollment behavior, verify:

1. Is this a course access relationship, a learner identity, or assignment
   activity state?
2. Does the authenticated session establish the account, then derive
   `ActorContext` and exact course relationship rather than browser input?
3. Does an email remain a verified, mutable authentication attribute rather
   than becoming account identity or course authority?
4. Does membership remain sufficient to derive current assignment entitlement
   without creating learner activity rows?
5. Does only a bounded first event atomically create the enrollment, empty
   summary, grant basis, scopes, and provenance?
6. Does removal revoke access while retention remains authoritative for
   records?
7. Are bulk and single paths the same command with the same idempotency and
   authorization?
8. Can the primary instructor-to-learner browser journey run without SQL or a
   seeding CLI?
9. Can the instructor import and export by course roster ID without exposing
   email, global `UserId`, or unrelated course activity?

## Related documents

The durable contract and related authority documents are linked from
[ENROLLMENT_DESIGN.md](../../ENROLLMENT_DESIGN.md). The active package order is
also coordinated by [release_completion_plan.md](release_completion_plan.md) and
the sole current-package registry in
[implementation_status.md](../implementation_status.md).
