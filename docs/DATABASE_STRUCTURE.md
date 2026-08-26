# Database structure

This document is the durable map of PLE's PostgreSQL schema: what owns each kind of
fact, which migration state is accepted, and where to find the detailed contracts. It
does not authorize changing an accepted migration. The shared status registry and release
plan remain the authority for unfinished work and deployment acceptance.

## Schema authority

The checked-in SQLx migrations in [schemas/migrations/](../schemas/migrations/) define
the schema. The Rust Store contracts and PostgreSQL implementation must agree with those
migrations, but do not replace them as schema authority. The shared registry in
[implementation_status.md](active_plans/implementation_status.md) owns unfinished migration
allocation and current package handoff; the release plan owns package scope and acceptance evidence:

- [implementation plan](active_plans/implementation_plan.md) defines the platform
  architecture and storage boundaries.
- [release completion plan](active_plans/active/release_completion_plan.md) defines
  the active migration sequence and release gates.
- [database schema evolution plan](active_plans/decisions/database_schema_evolution_plan.md)
  defines the forward-only process.
- [CONTRACTS.md](CONTRACTS.md) registers frozen cross-module contracts.
- [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md) owns the PostgreSQL/object-storage
  commit, repair, and reconciliation contract.

The browser never connects to PostgreSQL. It receives answer-free public envelopes from
the API; server code establishes authenticated transaction context and calls Store
capabilities.

## Migration ledger

The checked-in `schemas/migrations/` directory is the physical inventory authority. The immutable
accepted baseline is the first seven migrations and its historical inventory is 80 top-level
relations: the first six are the accepted pre-data baseline, and
`2026080907_course_appearance.sql` is the first accepted forward migration. That 80-relation number
is not the current schema size. Later files are also present, and the complete directory requires
the current disposable PostgreSQL baseline. Owning product packages that remain open are identified
below; a successful migration gate does not silently accept them. The shared [implementation status
ledger](active_plans/implementation_status.md) owns allocation and disposition. SQLx's ledger and
runtime-created partition children are excluded from the historical inventory. Relation counts are
drift checks, not capacity metrics or a reason to avoid a necessary normalized relation.

| Version    | File                                                                              | State             | Owns                                                                  |
| ---------- | --------------------------------------------------------------------------------- | ----------------- | --------------------------------------------------------------------- |
| 2026080801 | [principals](../schemas/migrations/2026080801_principals.sql)                     | Accepted baseline | Roles, tenant context, session lookup, migration-state projection     |
| 2026080802 | [catalog authoring](../schemas/migrations/2026080802_catalog_authoring.sql)       | Accepted baseline | Private drafts, immutable catalog versions, authoring/import evidence |
| 2026080803 | [courses assignments](../schemas/migrations/2026080803_courses_assignments.sql)   | Accepted baseline | Courses, membership, assignment configuration, enrollment, summaries  |
| 2026080804 | [activity feedback](../schemas/migrations/2026080804_activity_feedback.sql)       | Accepted baseline | Runs, attempts, submissions, feedback, current scores, protected logs |
| 2026080805 | [operations analytics](../schemas/migrations/2026080805_operations_analytics.sql) | Accepted baseline | Worker queue, timing, export, analytics, staging, object delivery     |
| 2026080806 | [retention](../schemas/migrations/2026080806_retention.sql)                       | Accepted baseline | Archive/delete lifecycle and frozen cleanup manifests                 |
| 2026080907 | [course appearance](../schemas/migrations/2026080907_course_appearance.sql)       | Accepted forward  | Course theme and banner candidate/current presentation state          |

Migration `2026081805_assignment_learner_disclosure_policy.sql` is accepted and immutable as
WP-PROF-S4. Migration `2026081806_course_grade_scheme.sql` is accepted and immutable as
WP-PROF-S6. Migration `2026081807_teaching_operations.sql` is accepted and immutable as
WP-PROF-T2. WP-PROF-T1 allocates no migration: it uses the accepted assignment lifecycle,
instructions, and normalized S3 base-policy relations from `2026081804` through one current Store
contract rather than adding a shadow timing or teaching-settings relation.

`2026080908_secure_question_grading_payloads.sql` is checked in as the WP-P2
prerequisite, but is not an accepted migration or a claim about a deployed database. It
adds presentation-binding columns, request-contract versioning, and the
`webwork_grade_replay_state` relation. Do not count it in the accepted 80 relations.

`2026080909_passwordless_identity.sql` is also checked in and passed fresh migration,
no-op replay, ledger verification, role/grant/forced-RLS checks, and the disposable
enrollment oracle. It adds PLE-owned accounts, email/WebAuthn ceremonies, passkeys,
account sessions, tenant learner mappings, roster state/policy/members/invitations,
bounded import staging, and PII-free grade-export audit. WP-RC8 remains acceptance-open.

`2026080916_submission_receipt_presentations.sql` and
`2026080917_issued_presentations_and_successor_receipts.sql` are pre-production,
forward-only receipt-contract migrations. They add the receipt presentation payload/checksum
and then the issued presentation capability/payload/checksum and checksummed successor
descriptor. Migration 2026081805 removes 0916's retired receipt disclosure field. They
intentionally provide no legacy reader, default, or backfill: PLE has no production data, and
missing or mismatched required payloads fail closed.
`2026080919_issued_private_grading_envelopes.sql` completes that issue contract with a
checksummed server-only, answer-free grading envelope. It retains durable response IDs for
first-submit translation and private grading; it is never part of a public receipt or learner DTO.
`2026080920_rebound_flat_question_hotspot_grading.sql` preserves the rebound private flat-question
grading contract for version-scoped HOTSPOT assets.
`2026080921_issued_flat_grading_contracts.sql` and
`2026080922_issued_webwork_grading_contracts.sql` add the explicit checksummed first-grade
contracts for their respective presentation-bearing families. They follow the same fresh-ledger,
no-backfill rule: a required contract that is missing, malformed, or mismatched is unavailable.

### Selected physical migration inventory

An accepted migration is immutable and later accepted work takes the next ordered forward version.
An unaccepted pre-production migration is different: when its design is wrong, correct or replace
that source directly, update the checked ledger expectation, and rebuild a clean disposable
database. Do not add a compatibility migration, nullable fallback, backfill, or legacy reader for
data that does not exist. This selected table inventories representative migration files and their
observed source state; it is intentionally non-exhaustive and is not an allocation or reservation
authority. The checked-in [`schemas/migrations/`](../schemas/migrations/) directory is the complete
physical inventory. The shared registry in
[implementation_status.md](active_plans/implementation_status.md) is the sole authority for
unfinished migration allocation, while the active release plan owns package acceptance.

| Version    | Planned owner     | Intended scope                                                                                                                                                                                                                                                      | Current source state                                                                                       |
| ---------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 2026080908 | WP-P2             | Secure learner grading payload binding and WeBWorK replay state                                                                                                                                                                                                     | File present; acceptance open                                                                              |
| 2026080909 | WP-RC8            | Passwordless account, passkey/email identity, invitation, course roster, import, and grade-export audit                                                                                                                                                             | File present; migration gate passed; package acceptance open                                               |
| 2026080914 | WP-RC4            | Flat-question v2 grading                                                                                                                                                                                                                                            | File present; package acceptance open                                                                      |
| 2026080915 | WP-HG1            | Historical internal catalog display-number range                                                                                                                                                                                                                    | File present; public use superseded by 2026080931                                                          |
| 2026080916 | Receipt closeout  | Submission receipt presentation payload/checksum; retired disclosure field removed by 2026081805                                                                                                                                                                    | File present; live receipt oracle passed                                                                   |
| 2026080917 | Receipt closeout  | Issued presentation capability/payload and successor receipt descriptor                                                                                                                                                                                             | File present; live receipt oracle passed                                                                   |
| 2026080918 | WP-RC5            | Immutable workspace flat-question asset descriptors and delivery bindings                                                                                                                                                                                           | Unaccepted source corrected to canonical `private-content` descriptor bucket; live oracle pending          |
| 2026080919 | Receipt closeout  | Issued private grading envelope for presentation-bearing attempts                                                                                                                                                                                                   | File present; live receipt oracle passed                                                                   |
| 2026080920 | WP-RC5            | Rebound private flat-question HOTSPOT grading for version-scoped assets                                                                                                                                                                                             | File present; acceptance open                                                                              |
| 2026080921 | Receipt closeout  | Issued private flat-question grading contract for first submission                                                                                                                                                                                                  | File present; live receipt oracle passed                                                                   |
| 2026080922 | Receipt closeout  | Issued private WeBWorK definition contract for first submission                                                                                                                                                                                                     | File present; live receipt oracle passed                                                                   |
| 2026080929 | WP-UI1            | Human route references for courses, assignments, runs, and workspaces                                                                                                                                                                                               | File present; browser route contract implemented                                                           |
| 2026080930 | WP-UI1            | Account-backed standard or increased-contrast presentation preference                                                                                                                                                                                               | Unaccepted source uses a session-hash broker; live oracle pending                                          |
| 2026080931 | Question ID       | Crockford Base32 Question ID, current-question projection, and owner-correction propagation                                                                                                                                                                         | File present; Memory/browser gates passed                                                                  |
| 2026080932 | Question ID       | Recreate `catalog_search_view` after 0931 to project `question_id`, preserving `security_invoker`, statistics, and grants                                                                                                                                           | File present; disposable migration baseline passed; acceptance open                                        |
| 2026080933 | Security repair   | Replace the boolean audit switch with a non-action precheck and an audited Sysadmin actor, use built-in SHA-256, grant the broker only `UPDATE (tenant_id)` needed for `FOR KEY SHARE`, and retain the dedicated `ple_roster_support_broker` SECURITY DEFINER owner | File present; focused static/offline checks pending live baseline                                          |
| 2026080934 | Security repair   | Require current-tenant ownership, not catalog visibility, to append catalog tenant grants, immutable version payloads, or source artifacts                                                                                                                          | File present; focused live RLS oracle pending                                                              |
| 2026080935 | Question ID       | Require a live original-instructor session capability for owner corrections; propagate only future assignment definitions through a narrow broker while preserving issued evidence                                                                                  | File present; focused static/offline checks pending live baseline                                          |
| 2026081401 | Catalog discovery | Ranked lexical catalog discovery and stable paging                                                                                                                                                                                                                  | File present; disposable catalog gate passed                                                               |
| 2026081501 | WP-RC8 account    | Atomic verified-email replacement and account/tenant-session revocation                                                                                                                                                                                             | File present; package acceptance open                                                                      |
| 2026081502 | WP-RC8 delivery   | Bounded course-invitation delivery outbox, leases, provider receipts, and cancellation                                                                                                                                                                              | File present; package acceptance open                                                                      |
| 2026081503 | WP-RC8 repair     | Relation-qualified invitation-delivery claim function                                                                                                                                                                                                               | File present; disposable PostgreSQL gate passed                                                            |
| 2026081504 | WP-RC8 repair     | Relation-qualified email-change completion and session-revocation function                                                                                                                                                                                          | File present; disposable PostgreSQL gate passed                                                            |
| 2026081801 | WP-PROF-S2        | Mandatory inclusive teaching-course term dates and exact IANA time-zone text on the course row                                                                                                                                                                      | Accepted and immutable; disposable PostgreSQL 17 baseline passed                                           |
| 2026081802 | WP-PROF-S7        | Positive public scalar for course groups; immutable validated public byline on published versions and catalog projection                                                                                                                                            | Accepted and immutable; fresh PostgreSQL 17 baseline and RLS oracle passed                                 |
| 2026081803 | WP-PROF-S5        | Canonical membership episodes, derived entitlement, typed group purposes/audiences, sealed materialization provenance, and typed assignment summaries                                                                                                               | Accepted and immutable; fresh PostgreSQL 17 baseline and RLS oracle passed                                 |
| 2026081804 | WP-PROF-S3        | Normalized base policy, group offsets/accommodations, individual exceptions, sealed per-attempt effective-policy receipts, field provenance, and current pointer                                                                                                    | Accepted and immutable; fresh PostgreSQL 17 baseline and exact normalized resolver/RLS oracle passed       |
| 2026081805 | WP-PROF-S4        | Assignment-owned five-field learner disclosure policy and removal of retired coarse disclosure columns                                                                                                                                                              | Accepted and immutable; fresh PostgreSQL 17 baseline and RLS oracle passed                                 |
| 2026081806 | WP-PROF-S6        | Course-owned total-points and weighted-category schemes, normalized categories and letter bands, compact-summary totals, and PII-free export audit                                                                                                                  | Accepted and immutable                                                                                     |
| 2026081807 | WP-PROF-T2        | Purpose-aware course-group membership policy, global non-authorizing Instructor approval, target-bound co-instructor invitations, safe public teaching references, final-Instructor protection, and modifier reference guards                                       | Accepted and immutable; fresh PostgreSQL 17 upgrade, group, authority, RLS, and concurrency oracles passed |
| 2026081837 | WP-PROF-B1        | Revisioned Blueprints, public Alpha curricula, immutable publication pins, and shared question reuse                                                                                                                                                                | Accepted and immutable; B1 acceptance passed                                                               |
| 2026081838 | WP-PROF-B2        | Curriculum-adoption lineage, schedules, provenance, receipts, integrity, and forced RLS foundation                                                                                                                                                                  | File present; B2 acceptance open                                                                           |
| 2026081839 | WP-PROF-B2        | Curriculum-adoption broker authority, retention integration, and capability boundary                                                                                                                                                                                | File present; B2 acceptance open                                                                           |
| 2026081840 | WP-PROF-B2        | Relational snapshots, locked preparation, inspection, and reconciliation helpers                                                                                                                                                                                    | File present; B2 acceptance open                                                                           |
| 2026081841 | WP-PROF-B2        | Ordinary-course topology, issued-work fencing, and topology assertions                                                                                                                                                                                              | File present; B2 acceptance open                                                                           |
| 2026081842 | WP-PROF-B2        | Source authorization, closed request validation, and source snapshot facts                                                                                                                                                                                          | File present; carries the forward relative-time-validator repair; B2 acceptance open                       |
| 2026081843 | WP-PROF-B2        | Teaching-course, import, inspection, reconciliation, and controlled schedule facts                                                                                                                                                                                  | File present; B2 acceptance open                                                                           |
| 2026081844 | WP-PROF-B2        | Shared materializer validation, idempotency, receipts, and evidence helpers                                                                                                                                                                                         | File present; B2 acceptance open                                                                           |
| 2026081845 | WP-PROF-B2        | Fork, assignment adoption, fast-forward, and reconciliation materializers                                                                                                                                                                                           | File present; B2 acceptance open                                                                           |
| 2026081846 | WP-PROF-B2        | Whole-course instantiation, rollover, and term-shift materializers                                                                                                                                                                                                  | File present; B2 acceptance open                                                                           |
| 2026081847 | WP-PROF-B2        | Public bridge completion and final broker catalog assertions                                                                                                                                                                                                        | File present; B2 acceptance open                                                                           |

The 2026080931 and 2026080935 entries describe superseded pre-production migration behavior. The
current WP-R2 schema contract uses one immutable Question ID per publication, fresh hidden
`(ProblemId, VersionId)` evidence, optional one-way provenance, and deliberate revision-checked
assignment replacement. The migration ledger remains historical evidence; it does not authorize
same-question correction, version-chain resolution, or automatic propagation.

The upload migration follows the reserved identity/reconciliation/LTI sequence and remains planned while
learner file responses fail closed. See the
[secure learner upload plan](active_plans/active/secure_learner_file_upload_plan.md).
The fresh pre-production `2026080909_passwordless_identity.sql` schema owns the one
canonical course-roster workflow. Migration `2026081803` makes `course_member` the sole current
membership-episode authority and keeps display/contact evidence in the subordinate
`course_roster_profile`; neither relation is an assignment receipt. Local browser entry uses the
same account/session records as the deployed product: passwordless account authentication and
ordinary passkeys are the normal paths, while a deployment-gated seeded persona selector may
enter one of the seeded accounts for connected local evidence. The selector does not introduce a
second identity or membership model. Because this checked-in schema is pre-production-only, a
changed migration baseline requires a clean disposable PostgreSQL volume rather than an in-place
ledger edit.

`2026081801_course_term.sql` keeps the term's three atomic attributes on `public.course` because
they depend on the course key and have no independent lifecycle. Native `date` columns preserve
calendar values, `text` preserves exact IANA spelling, and `NOT NULL`, ordering, range, trim, and
length constraints reject malformed row shapes. Exact known-zone membership remains in the shared
Rust value type so the application can update its reviewed IANA corpus without a competing database
enum or lookup table. The migration has no default, backfill, compatibility reader, or new index.

`2026081804_effective_policy_resolver.sql` keeps mutable assignment policy inputs separate from
immutable attempt evidence. The base row, group schedule offsets, group accommodations, and one
student exception are normalized assignment relations. Each attempt receipt records all resolved
fields and their ordered sources, seals only after complete provenance exists, and is selected by a
current pointer that can reference only a sealed generation. The schema therefore preserves why an
issued policy applied without reconstructing historical timing from changed current inputs.
WP-PROF-T1 makes that normalized base row the only stored schedule/limit/late/deadline authority and
uses the existing assignment lifecycle and instructions columns. A teaching-settings write locks the
assignment, validates the authoritative course term, updates lifecycle, instructions, and base policy
under one revision, then re-resolves active attempts in the same transaction. No legacy
`AssignmentRunTiming`, defaulted compatibility column, or parallel browser-owned clock is retained.

`2026081805_assignment_learner_disclosure_policy.sql` makes assignment disclosure the one current
authority. It adds non-null, checked `score_disclosure`,
`per_item_correctness_disclosure`, `feedback_text_disclosure`,
`solution_disclosure`, and `class_statistics_disclosure` columns. The migration uses temporary
defaults only while adding the closed columns, then drops all defaults, so future application
writes choose every value. It drops `assignment.feedback_disclosure`,
`question_attempt.issued_feedback_disclosure`, and
`submission_receipt_snapshot.feedback_disclosure`; no legacy disclosure column remains as a
reader or writer surface. `feedback_release` remains separate immutable audit evidence under its
retention fence, not a disclosure authority.

`2026081806_course_grade_scheme.sql` adds the course-owned `course_grade_scheme`, normalized
`course_grade_category`, `course_grade_category_assignment`, and `course_grade_letter_band`
relations plus `course_total_export_audit`. The mode check is closed to total points and weighted
categories; completion-based grading is not represented. The scheme revision is positive and
strong-CAS updates replace the whole title-free assignment setting, while scheme reads project
the current server-owned assignment titles. Course totals read one scheme snapshot and maintained
compact assignment summaries; the browser receives no email, learner identity, or raw summary
payload. Export rows are synchronous and ephemeral; only bounded, PII-free audit metadata is
durable. All new relations use tenant keys, forced RLS, and retention reachability.

`2026081807_teaching_operations.sql` adds one course-owned membership policy for each of the five
group purposes. Multiple Section membership warns without blocking; the other defaults allow it.
Global Instructor approval is eligibility evidence only and grants no course or platform authority.
Co-instructor invitations are target-bound, contain no email address, expire after 30 days, and
recheck approval when accepted. Positive public `U-`, `M-`, and `CI-` references are minted and
resolved only through narrow session-, operator-, target-, or exact-course-Instructor capabilities;
the browser never receives account UUIDs or email. Restrictive M2/M3 references, deferred purpose
guards, and the serialized final-active-Instructor guard preserve assignment and roster invariants.
The added relations use forced RLS and least-privilege broker functions; active-attempt S5
entitlement and S3 effective-policy re-resolution remains an atomic Store transaction with the
group or modifier mutation.

The unaccepted `2026080930_account_presentation_preference.sql` derives presentation
preference reads and writes only from a live, opaque 32-byte account-session hash.
`ple_auth` has no direct preference-table privilege; its two callable functions are
owned by the membership-free, forced-RLS `ple_account_presentation_broker`. The
database-baseline oracle creates its own accounts and sessions to prove default/save/get,
isolation, expiry failure, direct-access denial, and broker metadata.

## Data ownership

PLE keeps shared catalog facts, tenant teaching configuration, learner records, and
replaceable projections separate. JSONB holds cohesive, versioned payloads; relational
columns hold identities, ownership, constraints, joins, and lifecycle state. Object bytes
are not stored in these relations.

| Data class                 | Primary relations                                                                                                                                                                       | Ownership and mutability                                                                                                    |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| PLE account authentication | `ple_account`, `email_authentication_challenge`, `authentication_rate_limit`, `account_authentication_session`, `webauthn_ceremony`, `account_passkey`                                  | Global opaque account and private credential state under the authentication role; email is mutable and never a primary key. |
| Catalog and publication    | `problem`, `problem_version`, `problem_version_payload`, `answer_key`, `published_source_artifact`                                                                                      | One human Question ID names one immutable published question; hidden exact evidence preserves grading and provenance.       |
| Private authoring          | `workspace_draft` and `workspace_*` import/source relations                                                                                                                             | Tenant-private mutable work before publication.                                                                             |
| Course activity            | `course`, `course_member`, `tenant_learner_identity`, `course_roster_*`, `course_invitation`, `assignment`, `assignment_item`, `enrollment`                                             | Tenant/course configuration, protected roster PII, membership, and enrollment.                                              |
| Learner evidence           | `assignment_run`, `assignment_run_item`, `question_attempt`, `attempt_effective_policy_receipt`, `attempt_effective_policy_receipt_field_source`, `submission`, `submission_evaluation` | Tenant-owned educational records; effective-policy evidence is append-only and sealed.                                      |
| Current projections        | `attempt_score_current`, `student_assignment_summary`, `course_item_analysis_current`                                                                                                   | Recomputed/published current state; not a substitute for source evidence.                                                   |
| Course grading             | `course_grade_scheme`, `course_grade_category`, `course_grade_category_assignment`, `course_grade_letter_band`, `course_total_export_audit`                                             | Course-owned scheme and normalized membership; export audit stores only actor/course/revision/mode/rounding/count metadata. |
| Protected delivery/audit   | `asset_delivery`, `student_export_*`, `record_access_log`, `audit_event`                                                                                                                | Explicitly authorized and retention-bound record access/evidence.                                                           |

Publication pins an assignment to an exact `(problem_id, version_id)` and an issued run item
to what that learner actually received. A course edit therefore does not reinterpret an
already issued question. The detailed lifecycle is in
[ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md), and the information-class boundary is in
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md). The authoritative database-facing list of
especially radioactive and linkage-radioactive relations is the
[radioactive table map](DATABASE_TENANCY.md#radioactive-table-map). Its label also follows data into
partitions, views, temporary tables, query results, dumps, WAL, replicas, snapshots, and restores.

## Assessment record chain

The relational activity spine is:

```text
course -> assignment -> enrollment -> assignment_run -> assignment_run_item
                                                   -> question_attempt
                                                        -> submission
                                                        -> submission_evaluation
                                                        -> attempt_score_current
                                                    enrollment -> student_assignment_summary
```

All learner-facing transitions derive tenant, learner, assignment position, version, seed,
policy, backend, and timing from the authenticated attempt record. A request body cannot
select those facts.

| Relation                                                 | Durable responsibility                                                                                                                                                                      |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `assignment_run`                                         | One owned pass through an assignment; completion is derived from attempt state.                                                                                                             |
| `assignment_run_item`                                    | Exact version/order and selection evidence delivered in that run.                                                                                                                           |
| `question_attempt`                                       | One issued question instance, including seed, provenance, timing, and controlled state.                                                                                                     |
| `question_prefetch`                                      | One bounded, server-only reservation; it has no started timer, response, or score, but may contain private issue-time grading authority and must never enter a learner DTO or public cache. |
| `submission`                                             | Append-only learner response evidence.                                                                                                                                                      |
| `submission_idempotency` and receipt relations           | Exact retry/replay fence; no second grade on a conflicting retry.                                                                                                                           |
| `submission_evaluation`                                  | Server-produced normalized result and policy-controlled feedback basis.                                                                                                                     |
| `attempt_score_current` and `student_assignment_summary` | Current scoring/gradebook projections, updated under scoring-generation rules.                                                                                                              |

The accepted issuance and receipt design binds a response to `AttemptId`, an idempotency key, and
the issued presentation. The current browser request still carries a tagged `StudentResponse`; the
server validates that tag against issued authority and never trusts it to select a question, key,
or grader. The later compact learner-wire target removes the redundant response kind and sends only
the family-minimal answer plus presentation binding. See
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for that explicit
current-versus-target boundary.

### Presentation and replay state

Migration 0908 introduces descriptor primitives for the secure grading-payload cutover;
0916, 0917, 0919, 0921, and 0922 complete the receipt and private-grading persistence:

| Relation or columns                                                  | Purpose                                                                                                                                                  | Privacy boundary                                                                                                                        |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `question_attempt.presentation_descriptor_*`                         | Versioned descriptor, nonce, and digest from 0908.                                                                                                       | Public descriptor primitives; never a grading key.                                                                                      |
| `question_attempt.presentation_capability`, `presentation_*_payload` | 0917 explicit capability and checksummed answer-free issued presentation snapshot.                                                                       | `EnvelopeV1` is complete; `NotApplicable` contains no inferred fallback data.                                                           |
| `question_attempt.grading_envelope_*`                                | 0919 server-only checksummed answer-free envelope with durable response IDs.                                                                             | First submit translates public rendered IDs without rerendering mutable catalog/backend state; this payload never enters a learner DTO. |
| `question_attempt.flat_grading_*`                                    | 0921 server-only checksummed flat `QuestionDefinition` plus private grading payload.                                                                     | Required for issued flat attempts; first grade reads this immutable contract rather than a current catalog/grader view.                 |
| `question_attempt.webwork_grading_*`                                 | 0922 server-only checksummed WeBWorK `QuestionDefinition`.                                                                                               | Required for issued WeBWorK attempts; first grade does not resolve a current catalog definition or reissue to recover it.               |
| `question_prefetch.presentation_*`                                   | The same binding for one reservation before promotion to an attempt.                                                                                     | Prevents an unbound prefetch from becoming an issued render.                                                                            |
| `submission_idempotency.request_*`                                   | Versioned request fingerprint for safe response retries.                                                                                                 | Server compares it before regrading.                                                                                                    |
| `submission_receipt_snapshot.presentation_*`                         | First-receipt copy of the issued envelope and exact public asset bindings, plus checksum.                                                                | Submitted reads/replays fail closed instead of regenerating mutable presentation data.                                                  |
| `submission_next_attempt.next_payload`                               | Checksummed immutable descriptor of the delivered successor, or a terminal all-null row. The absence of a receipt-link row is recoverable `nextPending`. | A retry cannot infer a successor from later run state.                                                                                  |
| `webwork_grade_replay_state`                                         | Attempt-bound, answer-free mapping needed to reproduce a private WeBWorK grade call.                                                                     | Never enters the browser envelope; contains no source text, credentials, correct answer, or raw renderer result.                        |

The relation is tenant-, course-, attempt-, version-, source-, seed-, renderer-, and
presentation-bound. Its mapping has an explicit size and item-count limit, an SHA-256
fingerprint, forced RLS, retention-broker access, and a foreign key to the precise
attempt. The complete render/response contract, including attempt-specific CRC-16 item IDs,
is in [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md), not this schema map.

## Relational integrity and query paths

Tenant-owned foreign keys include `tenant_id` in both the referencing and referenced key. Shared
published content is the deliberate exception: course/run records reference immutable global
`(problem_id, version_id)` identities, while authorization remains on the tenant-owned assignment
and attempt chain. Delete behavior is explicit rather than inherited from application convention:
assignment composition and course membership use bounded cascades, published versions and receipt
evidence use restrictive references, and learner-record purges go through the retention capability.

JSONB is reserved for cohesive versioned payloads. Every authoritative JSON payload has a relational
identity/owner, a closed discriminator or version, bounded size, and, where replay or private/public
binding matters, a server-computed SHA-256 checksum. Fields used for ownership, joins, state
transitions, ordering, filtering, or lifecycle remain typed relational columns with constraints and
indexes; application decoding is not a substitute for database integrity.

The migration owns indexes alongside the query contract. Representative hot paths are:

| Query path                                       | Owning indexes or constraint                                                              |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| Exact Question ID and text search                | `catalog_search_document_question_id_idx` and `catalog_search_document_search_idx`        |
| Course assignment paging and selected item order | `assignment_course_page_idx` and `assignment_item_assignment_idx`                         |
| Gradebook enrollment paging                      | `enrollment_gradebook_summary_page_idx` over the current summary join key                 |
| One active run and run-attempt cursor paging     | `assignment_run_one_active_idx` and `question_attempt_run_summary_cursor_idx`             |
| Submission replay and immutable successor lookup | primary/unique idempotency keys plus `submission_next_attempt_next_idx`                   |
| Ready worker claims and expired leases           | partial `worker_job_claim_ready_idx` and `worker_job_expired_lease_idx`                   |
| Account-session expiry and user revocation       | `account_authentication_session_expiry_idx` and `account_authentication_session_user_idx` |

This table is a query-ownership map, not an exhaustive index inventory. A new index must name the
production query it serves, retain result-equivalence coverage, and show representative
`EXPLAIN (ANALYZE, BUFFERS)` evidence. The disposable scale oracle currently exercises 260,000
attempts, partition pruning, and a bounded 51-row gradebook page; it is evidence for those shapes,
not a universal production-size claim.

## Tenant isolation and grants

Private and learner-record relations carry `tenant_id`, enable RLS, and use forced RLS.
The application role is not a table owner, superuser, or `BYPASSRLS` role. Each server
transaction sets its tenant context locally; pooled connections must not inherit context.
RLS is the tenant fence, while Store queries additionally bind a learner to the owned
enrollment or require exact instructor course membership.

| Role/capability                     | Narrow purpose                                                                                                                                                                                    |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ple_app`                           | Normal API/server work through RLS and narrowly granted tables/functions.                                                                                                                         |
| `ple_student`                       | Read-only student projections subject to tenant and ownership predicates.                                                                                                                         |
| `ple_grader` / `ple_grading_reader` | Server-only grading material and approved reader functions; never browser access.                                                                                                                 |
| `ple_auth`                          | Hash-based session resolution only.                                                                                                                                                               |
| `ple_retention_broker`              | Retention-manifest and learner-record cleanup work under its RLS policies.                                                                                                                        |
| `ple_statistics_broker`             | Identity-free aggregate/statistics contribution work.                                                                                                                                             |
| `ple_qti_*_broker`                  | Narrow staging/provenance capabilities for QTI import.                                                                                                                                            |
| `ple_roster_support_broker`         | RLS-obeying owner for the non-action roster precheck and narrow audited Sysadmin roster-support actor; it has only the `UPDATE (tenant_id)` privilege needed to take a membership key-share lock. |

Grants do not replace RLS, and RLS does not prove individual learner ownership by itself.
Production acceptance must exercise the deployed roles and transaction context, including
foreign-tenant and foreign-student denial. The detailed model is in
[DATABASE_TENANCY.md](DATABASE_TENANCY.md) and [SECURITY_MODEL.md](SECURITY_MODEL.md).

A `SECURITY DEFINER` function is accepted only as a narrow capability: it has an explicit owner,
pins a safe `search_path`, revokes `PUBLIC` execution, and grants only the role that needs that
operation. New functions must pass the same inventory and deployed-role denial checks; a definer
function is never a convenience escape from RLS or Store authorization.

## Transactions, MVCC, and failure semantics

Each multi-step Store mutation owns one explicit transaction and sets `ple.tenant_id` with
transaction-local scope before tenant data is read or written. Mutations lock the smallest stable
parent row first (for example course, assignment, run, attempt, or draft), then update children in a
documented order. Partial unique indexes and compare-and-swap revisions enforce single-current facts
such as one active run; correctness does not depend on a prior unlocked read.

Commands that opt into automatic retry replay the complete idempotent transaction at most three
times after PostgreSQL serialization failure (`40001`) or deadlock (`40P01`). Code never continues a
failed transaction, retries only the last statement, or retries an ambiguous connection/commit
failure. A retry closure contains no object-store, renderer, email, callback, or other external
effect. Client idempotency keys, immutable receipt checksums, and revision tokens distinguish a safe
replay from a conflicting request. See
[CONCURRENCY_CONTRACTS.md](CONCURRENCY_CONTRACTS.md) for lock order, retry bounds, and command-level
examples.

PostgreSQL and object storage do not form a distributed transaction. Cross-store capabilities write
typed candidate objects, validate exact checksums/ownership, commit the relational pointer, and
compensate only the exact uncommitted candidate on failure. Reconciliation and retention recheck
authoritative references before deleting anything. The full ownership and crash-window rules are in
[STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md).

## Worker, retention, and statistics

`worker_job` is a durable, tenant-scoped queue. Its claim function uses bounded job kinds,
leases, attempt counts, and `FOR UPDATE SKIP LOCKED`; handlers commit under lease and
generation fences. Worker payloads carry identifiers and generations rather than learner
names, raw responses, grades, answer keys, or storage credentials.

Retention is a database-backed lifecycle, not an ad hoc delete:

- `institution_retention_policy`, `course_retention`, notification, stage, dispatch, and
  API-receipt relations persist policy and revision-fenced actions.
- Cleanup manifest relations freeze the exact typed object set before a worker removes
  learner records, so a retry cannot discover a newly written or unrelated object.
- Purge relations record ordered deletion progress for attempts, runs, and exports.
- Shared published content, private drafts, and identity-free question statistics remain
  outside a learner-record purge.

`question_statistics_aggregate` is shared, identity-free statistical state.
`question_statistics_contribution_receipt` makes first-completed-run contribution
idempotent and supports deletion of the learner-owned receipt without deleting the aggregate.
Course item analysis remains a separate tenant-owned current projection. Its
learner-safe class-statistics read uses the latest completed run for each
enrollment and releases only a metric-free `insufficientEvidence` state or an
`available` cohort count plus normalized assignment average. The Store checks
current S5 entitlement; the server exposes that union only when the assignment
field's current S3/time disclosure decision allows it. The default minimum
cohort is five, and incomplete manual grading, recent rescoring, or a missing
or invalid average suppress metric fields. Detailed cross-store failure/recovery
behavior is owned by [STORAGE_CONSISTENCY.md](STORAGE_CONSISTENCY.md); retention
policy is in [RETENTION_POLICY.md](RETENTION_POLICY.md).

## Partitioning and operations

The baseline partitions the high-churn learner evidence tables by month, including
`question_attempt`, `submission`, `record_access_log`, and `audit_event`; a default
partition catches unprepared dates. `problem_version_payload` uses hash partitions by
`problem_id`. Partition children are runtime schema objects and are not included in the
80-relation inventory above.

Keep current grade summaries and compact identity/header relations unpartitioned unless
measured query and retention behavior requires a change. New indexes, partitions, poolers,
read replicas, or external search require representative result-equality checks and
`EXPLAIN (ANALYZE, BUFFERS)` evidence. PostgreSQL remains the source of truth.

Autovacuum, analyze cadence, fillfactor, connection-pool sizing, and per-partition maintenance are
deployment measurements rather than universal constants in a migration. Before production, observe
dead tuples, freeze age, table/index growth, cache hit behavior, lock waits, and slow-query plans
through PostgreSQL statistics, then record any nondefault tuning with its measured workload and
removal condition. Do not disable autovacuum or add an index merely to silence a synthetic test.

Migration startup is read-only: application startup reads the granted
`ple_migration_state` projection and refuses known-incompatible, dirty, pending, or
checksum-mismatched ledgers. Only the explicit migration command applies DDL. A fresh
database, a no-op second apply, and deployed-role RLS checks are separate acceptance
evidence; a passing Rust compile does not prove any of them.

## Change rules

For a durable schema change:

1. Confirm the owning package and the migration's allocation in
   [implementation_status.md](active_plans/implementation_status.md), then confirm accepted or
   unaccepted status in the active release plan.
2. For an accepted version, add the next forward migration allocated in the shared registry and
   never rewrite its filename, version, or checksum. For unaccepted pre-production work, fix the design at its source and
   rebuild the disposable database rather than preserving a superseded shape.
3. Preserve tenant keys, forced RLS, least-privilege grants, retention reachability, and
   answer-key isolation in the same change.
4. Update the Store contract and memory/PostgreSQL conformance only when behavior changes.
5. Use expand, backfill, verify, switch, and retire stages only after durable data actually exists
   and the active plan explicitly owns that rollout; do not pre-build a legacy path for hypothetical
   users.
6. Run the package's fresh/no-op migration, deployed-role, Store-conformance, and security
   gates before calling the schema accepted.

The exact acceptance commands and historical compatibility obligations belong to the
[release completion plan](active_plans/active/release_completion_plan.md), so this document
does not turn a design description into release evidence.
