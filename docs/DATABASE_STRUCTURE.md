# Database structure and growth

This document maps the implemented PostgreSQL 17 schema, the authentication tables needed before
the fall pilot, and the growth path from four courses to a catalog of ten million questions. It is
a design and operations reference, not authorization to edit an applied migration. The accepted
baseline and every later schema change remain governed by
[active_plans/decisions/database_schema_evolution_plan.md](active_plans/decisions/database_schema_evolution_plan.md).

The current database declares 80 application tables across seven SQLx migrations. That table count
is not a scaling problem by itself. Row counts, query predicates, write churn, index size,
connection count, and recovery time are the measurements that matter.

## Migration ledger

SQLx owns the immutable migration ledger. The implemented epoch consists of the six reviewed
pre-data baseline files (`2026080801` through `2026080806`) followed by the applied forward
migration `2026080907_course_appearance.sql`. The next reserved, not-yet-applied forward files are
`2026080908_secure_question_grading_payloads.sql`, `2026080909_object_reconciliation.sql`,
`2026080910_oidc_identity.sql`, and `2026080911_lti_advantage.sql`. Applied filenames, versions,
and checksums are immutable; a later change is a new forward migration.

The migration connection has the privileged administrative path: it may inspect
`public._sqlx_migrations`, and only the explicit migration command applies DDL. In contrast,
application startup uses `ple_app` in a read-only transaction and reads only the
`public.ple_migration_state` projection. Startup refuses a reachable database when the ledger or
projection is missing, the application role cannot use the projection, an applied version is
unknown, a migration is dirty or pending, or a checksum differs. It never creates the SQLx ledger
or applies DDL. An unreachable database remains a separately reported degraded-start condition.

## Design summary

Use four kinds of durable state:

- Immutable facts: published question versions, delivered run items, and submitted responses.
- Mutable current state: course membership, assignments, policies, and current evaluation.
- Replaceable projections: catalog search, grade summaries, and item analysis.
- Minimal audit evidence: security-sensitive actions and protected-record access.

Relational columns own identity, authorization, lifecycle, dates, revision numbers, joins, and
retention. Versioned JSONB owns cohesive question definitions and bounded adapter metadata. Media
bytes remain in typed object storage. PostgreSQL documents that constraints enforce row and
relationship invariants, while JSONB and GIN support indexed searches inside structured documents;
PLE uses both for their appropriate jobs
([PostgreSQL constraints](https://www.postgresql.org/docs/17/ddl-constraints.html),
[PostgreSQL JSONB indexing](https://www.postgresql.org/docs/17/datatype-json.html#JSON-INDEXING)).

The implemented relationship spine is:

```text
shared catalog
problem -> problem_version -> problem_version_payload
                    |       `-> answer_key (grader only)
                    |
tenant course       v
course -> assignment -> assignment_item
   |                       |
   +-> course_member       v
   `-> enrollment -> assignment_run -> assignment_run_item
                             |
                             v
                       question_attempt
                             |
                             v
                         submission
                             |
                             v
                    submission_evaluation
                             |
                   +---------+---------+
                   v                   v
          attempt_score_current  student_assignment_summary
```

Every arrow that crosses into an educational record also carries tenant identity. The browser never
connects to PostgreSQL directly.

## Pilot workload

The fall pilot consists of two Genetics sections, one Biochemistry section, and one Biostatistics
section, with two instructors and about 50-100 unique students. This estimate assumes each student
takes one of the four courses. Cross-enrollment increases enrollment rows, not account rows.

| Relation or event              | Pilot estimate | Calculation                        |
| ------------------------------ | -------------: | ---------------------------------- |
| `course`                       |              4 | Fixed pilot scope                  |
| Instructor memberships         |     At least 4 | One or both instructors per course |
| Unique students                |         50-100 | Owner estimate                     |
| `assignment`                   |          40-60 | 4 courses x 10-15 assignments      |
| `assignment_item`              |        200-900 | 40-60 x 5-15 questions             |
| Student-assignment enrollments |      500-1,500 | 50-100 x 10-15 assignments         |
| `assignment_run`               |   1,000-45,000 | Enrollments x 2-30 instances       |
| Question attempts/submissions  |  5,000-675,000 | Runs x 5-15 questions              |

If 50-100 means students per course rather than total unique students, multiply the last three
rows by roughly four. Even that upper pilot shape does not justify sharding or hundreds of custom
partitions. It does justify realistic seed data, a simultaneous class-start load test, and measured
query plans before the first course opens.

## Question revision tracking

Question identity and question content have different lifetimes.

| Table                                | Implemented responsibility                                                          |
| ------------------------------------ | ----------------------------------------------------------------------------------- |
| `problem`                            | Stable UUID, human decimal ID, owner, visibility, license, and lifecycle            |
| `problem_version`                    | Immutable numbered revision, schema version, checksum, title, lineage, and metadata |
| `problem_version_payload`            | Immutable normalized question JSONB, separated from hot browse columns              |
| `answer_key`                         | Answer-bearing payload visible only through the grader boundary                     |
| `catalog_search_document`            | Rebuildable full-text and facet projection                                          |
| `workspace_draft` and related tables | Private mutable authoring before publication                                        |
| `assignment_item`                    | Exact pinned `(problem_id, version_id)` plus current assignment points              |
| `assignment_run_item`                | Exact version actually delivered to one student run                                 |

The lifecycle is:

```text
private draft
    |
    +-- publish new work --> problem P-123 + problem_version v1
    |
    `-- revise owned work ----------------> problem_version v2
                                               |
                         assignment A pins ----+
                         assignment B may remain on v1
```

The database enforces a unique `(problem_id, version_number)`, a linear
`previous_version_id` chain, content checksums, and foreign keys from assignments and delivered
runs. Triggers reject updates or deletes of published payloads and answer keys. A correction creates
a new `problem_version`; it never silently mutates an assignment already pinned to an older version.

Assignments intentionally do not have an `assignment_version` table. Ordinary assignment edits use
a strong positive `revision`, and issued runs preserve only the execution evidence needed to explain
what a student saw. This avoids historical-grade cruft while retaining fair, reproducible attempts.

At ten million stable questions, the row formula is:

```text
problem rows = 10,000,000
problem_version rows = problem rows x average published versions per problem
payload rows = problem_version rows
search rows = visible searchable problem versions
```

The implemented payload table already uses 16 hash partitions by `problem_id`; compact version
metadata and the search projection remain separate. Do not increase the partition count merely
because the catalog reaches a round number. Measure representative exact-version loads and catalog
search with `EXPLAIN (ANALYZE, BUFFERS)` first. PostgreSQL's plan output distinguishes estimated
work, actual rows, and buffer activity
([PostgreSQL EXPLAIN](https://www.postgresql.org/docs/17/using-explain.html)).

## Authentication structure

### What exists now

`auth_session` is implemented. It stores only the SHA-256 hash of a random 256-bit session token,
the tenant and user identity projection, coarse roles, creation time, expiration, and revocation.
The raw token lives only in one HttpOnly browser-session cookie. The narrowly privileged `ple_auth`
role can resolve only the row matching the presented token hash.

The server already defines an `IdentityProvider` boundary, but the only composed provider is the
explicit local-development identity file. WP-RC8 adds institutional OIDC plus the exact principal
mapping below. The fall pilot must not mistake local-file auth for a production account system.

### Version 1 authentication decision

| Choice                          | Recommendation        | Why                                                                         |
| ------------------------------- | --------------------- | --------------------------------------------------------------------------- |
| Roosevelt institutional sign-in | In scope through OIDC | The university owns identity proofing, account disablement, and recovery    |
| Passkeys                        | Out of version 1      | Cleanly fits `IdentityProvider`, but adds ceremony/recovery/browser scope   |
| Self-hosted passwords           | Out of version 1      | Adds password policy, breach response, reset, throttling, and hash upgrades |
| Email magic link/code           | Out of version 1      | Access to email is one factor; it is not email-based 2FA                    |

OpenID Connect is the selected production identity layer over OAuth 2.0, and current OAuth security guidance requires
modern authorization-code protections such as PKCE and transaction-bound state or nonce
([OpenID Connect Core](https://openid.net/specs/openid-connect-core-1_0.html),
[OAuth 2.0 Security BCP](https://www.rfc-editor.org/rfc/rfc9700.html)). PLE stores the stable issuer
and subject pair from its configured institutional provider and treats email as
contact data, never as the immutable authorization key.

NIST explicitly says email must not be used as an out-of-band authenticator. An emailed magic link
can be a passwordless login product decision, and email can support address verification or account
recovery, but calling it two-factor authentication would be inaccurate
([NIST authenticator requirements](https://pages.nist.gov/800-63-4/sp800-63b/authenticators/)).

### Planned version 1 OIDC identity tables

WP-RC8 reserves `schemas/migrations/2026080910_oidc_identity.sql` for these three tenant-owned
relations. They are planned forward schema, not part of the implemented seven-migration epoch:

| Table                | Important columns                                                      | Responsibility                           |
| -------------------- | ---------------------------------------------------------------------- | ---------------------------------------- |
| `principal`          | `tenant_id`, `user_id`, display name, status, timestamps               | Durable instructor or student identity   |
| `federated_identity` | issuer, subject, tenant/user IDs, disabled time, last successful login | Stable institutional OIDC binding        |
| `principal_role`     | tenant/user IDs, `student`/`instructor`/`administrator`, timestamps    | Coarse session and tenant-wide authority |

`auth_session` remains the existing hashed opaque-session relation. `(issuer, subject)` is unique
across active mappings; email never selects tenant or user. Forced RLS and narrow administrator
capabilities own mapping creation/disablement. Login reads one exact mapping, then copies the current
safe display name and roles into the bounded session projection.

The following relations are explicitly post-v1 and are not created by WP-RC8:

| Post-v1 table           | Purpose when a separately approved provider requires it            |
| ----------------------- | ------------------------------------------------------------------ |
| `webauthn_credential`   | Passkey/security-key public credential and signature-counter state |
| `webauthn_ceremony`     | Short-lived single-use WebAuthn registration/login state           |
| `password_credential`   | Locally verified password hash                                     |
| `account_recovery_code` | Single-use local-account recovery                                  |
| `auth_session`          | Existing token hash, subject projection, expiry, revocation        | Shared stateless-server session continuity |

Use `(issuer, subject)` rather than email for a federated identity. Give a user several
`webauthn_credential` rows so a phone, laptop, and hardware key can coexist. A WebAuthn user handle
must be random and non-PII; the W3C specification specifically warns against putting email or a
username in it. The server must generate and temporarily bind a fresh challenge, validate the RP
ID and exact expected origin, reject duplicate credential IDs, verify signatures and user
verification policy, and consume the ceremony exactly once
([WebAuthn Level 3](https://www.w3.org/TR/webauthn-3/)).

If local passwords are unavoidable, store only an Argon2id PHC string with per-password salt and
versioned parameters, keep any pepper outside PostgreSQL, block common or compromised passwords,
rate-limit attempts, and support parameter upgrades after successful login. Do not impose arbitrary
symbol rules or periodic changes without evidence. Argon2id is the RFC 9106 primary variant for
password hashing, while NIST requires salted password hashing and favors long passwords plus a
blocklist over composition rules
([RFC 9106](https://www.rfc-editor.org/rfc/rfc9106.html),
[NIST password requirements](https://pages.nist.gov/800-63-4/sp800-63b/authenticators/#passwords)).

### Passkey effort

Passkeys are medium difficulty with a maintained Rust WebAuthn library and high difficulty if the
cryptographic verification is written locally. PLE should use a maintained implementation and own
only its policy, persistence, routes, and user experience.

A realistic engineering estimate is:

- 5-10 focused engineer-days for one secure-origin registration/login path, database tables,
  session handoff, revocation, focused tests, and one recovery route.
- 2-4 weeks for production-quality multiple-device management, institutional account linking,
  lost-device recovery, administrator support, accessibility, browser coverage, audit evidence,
  and an independent security review.

Those are planning estimates, not measured repository results. The FIDO Alliance's build-versus-buy
guidance identifies server cryptography, client WebAuthn, device compatibility, and especially user
journeys as separate implementation responsibilities
([FIDO passkey deployment guidance](https://fidoalliance.org/wp-content/uploads/2023/07/Build-vs.-Buy-A-Guide-To-Deploying-Passkey-Based-Authentication.pdf)).

For the fall pilot, institutional OIDC is the selected production path. Passkeys fit cleanly behind
the same `IdentityProvider` contract in a separately approved post-v1 package without changing
course, assignment, or scoring tables.

## Assignment creation

Assignment authoring is current state with optimistic concurrency:

| Table                            | Role in creation and delivery                                                      |
| -------------------------------- | ---------------------------------------------------------------------------------- |
| `course`                         | Tenant-owned course container                                                      |
| `course_member`                  | Course-local instructor/student authority                                          |
| `assignment`                     | Title, schedule, attempt policy, presentation policy, revision, scoring generation |
| `assignment_item`                | Ordered exact problem version and current points                                   |
| `assignment_selection_group`     | Random draw count, order policy, and algorithm version                             |
| `assignment_selection_candidate` | Exact eligible problem versions for a selection group                              |
| `assignment_policy_exception`    | Per-student or per-group timing and attempt accommodation                          |
| `enrollment`                     | One student's identity and current state for one assignment                        |
| `assignment_run`                 | One delivered assignment instance for an enrollment                                |
| `assignment_run_item`            | Exact question versions, order, and random-selection evidence delivered            |

The create transaction derives tenant and actor from the authenticated session, verifies instructor
authority in the course, verifies every exact catalog version is visible and assignable, and writes
the assignment plus ordered items atomically. Update requests compare the strong assignment revision
before incrementing it. Request bodies cannot select a tenant, author, or arbitrary scoring
generation.

Once a run is issued, the run-item rows preserve question version and order even if the instructor
later reorders the assignment. Point changes and Delete and Regrade update current assignment state,
increment `scoring_generation`, and publish replacement current scores atomically through staging.
Old computed grades do not accumulate as a misleading history.

## Student scoring isolation

FERPA requires institutions to use reasonable methods to identify and authenticate people who
receive education-record PII; it does not define a product certification or require one particular
database feature. PLE's controls support that institutional obligation, but deployment policy,
training, contracts, and incident handling remain part of compliance
([U.S. Department of Education FERPA guidance](https://studentprivacy.ed.gov/ferpa)).

### Record chain

One student's scoring path is:

```text
authenticated tenant/user
    -> course_member and enrollment ownership
    -> assignment_run
    -> assignment_run_item
    -> question_attempt
    -> submission
    -> submission_evaluation
    -> attempt_score_current
    -> student_assignment_summary
```

The protection is layered:

- Every private row carries `tenant_id`; private keys, foreign keys, and indexes lead with it where
  their relationship allows.
- Every tenant-owned table enables and forces RLS. The application login is not a table owner,
  superuser, or `BYPASSRLS` role.
- Each transaction assumes a narrow role and sets tenant context with transaction-local scope, so a
  pooled connection cannot retain another request's tenant.
- Student Store queries also bind the authenticated `user_id` to `enrollment.user_id`. Database RLS
  supplies tenant isolation; the data-access query supplies per-student isolation.
- Instructor reads require a `course_member` instructor row for that exact course. Tenant-wide
  coarse roles do not silently become course membership.
- `answer_key`, QTI grading material, and other private grader records are unavailable to the normal
  application and browser roles.
- Raw response evidence remains protected and retention-bound. Browser gradebook paths read compact
  current summaries rather than scanning or exposing attempt history.
- Worker payloads contain bounded IDs and generations, not student names, raw responses, grades, or
  answer keys.
- Protected record delivery appends a course-scoped `record_access_log`; sensitive mutations append
  bounded `audit_event` evidence without copying obsolete grades.

PostgreSQL applies a default-deny result when RLS is enabled and no policy permits a row. Table
owners and `BYPASSRLS` roles are important exceptions, and referential-integrity checks can bypass
RLS, so the live acceptance gate must test actual deployment roles rather than merely inspect SQL
text
([PostgreSQL row security](https://www.postgresql.org/docs/17/ddl-rowsecurity.html)).

### Scoring tables

| Table                          | Classification            | Update rule                                                  |
| ------------------------------ | ------------------------- | ------------------------------------------------------------ |
| `question_attempt`             | FERPA education record    | Immutable identity/evidence plus controlled state transition |
| `submission`                   | FERPA education record    | Append-only response evidence with idempotency binding       |
| `submission_evaluation`        | FERPA education record    | One current normalized evaluation per attempt                |
| `manual_grade_receipt`         | Sensitive audit evidence  | Append one revision-fenced instructor action                 |
| `attempt_score_current`        | FERPA current projection  | Replace only through current scoring generation              |
| `student_assignment_summary`   | FERPA current projection  | One current gradebook row per enrollment                     |
| `assignment_*_staging`         | Private transient work    | Publish atomically or discard if superseded                  |
| `course_item_analysis_current` | Instructor-only aggregate | Rebuild without learner identity or raw responses            |

## Other table families

The remaining tables preserve clear expansion seams rather than putting unrelated state into one
generic JSON document.

| Family            | Representative tables                                                   | Expansion boundary                                                        |
| ----------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Catalog discovery | `problem_collection`, `catalog_search_document`, `catalog_tenant_grant` | Rebuild search or add an external projection without moving source truth  |
| Authoring/import  | `workspace_draft`, flat-import and QTI import/evidence tables           | Add format adapters behind staging and explicit publication               |
| Feedback          | `attempt_feedback`, `feedback_release`                                  | Add disclosure policy without exposing grader records                     |
| Timing/support    | `attempt_timing_current`, policy exceptions, receipts                   | Add accommodations without cloning assignments                            |
| Assets/exports    | `asset_delivery`, `student_export_request`, `student_export_artifact`   | Keep object bytes outside PostgreSQL and authorize logical IDs            |
| Analytics         | `course_item_analysis_*`, `question_statistics_*`                       | Rebuild projections; disclose cross-course data only above thresholds     |
| Operations        | `worker_job` and staging tables                                         | Scale worker processes by closed job family                               |
| Retention         | `course_retention*`, `institution_retention_policy`                     | Archive/delete tenant records without following shared catalog references |
| Course appearance | `course_appearance`, `course_banner_candidate`                          | Add revisioned presentation without coupling it to scoring                |

## Growth plan

### Ten thousand students

For 10,000 students taking 10-15 assignments with 2-30 assignment instances and 5-15 questions per
instance, the planning range is:

| Record                            |       Estimated rows |
| --------------------------------- | -------------------: |
| Student-assignment enrollments    |      100,000-150,000 |
| Assignment runs                   |    200,000-4,500,000 |
| Question attempts and submissions | 1,000,000-67,500,000 |

That range is intentionally wide because usage policy matters more than account count. Retakes drive
the high end. Peak concurrency is also independent of total rows; a scheduled exam start may be the
largest operational event.

The current schema already range-partitions `question_attempt`, `submission`, `record_access_log`,
and `audit_event` by month, with pre-created partitions and a default partition. PostgreSQL advises
choosing a partition key from actual query and lifecycle behavior; too many partitions can increase
planning overhead
([PostgreSQL partitioning](https://www.postgresql.org/docs/17/ddl-partitioning.html)). Keep current
grade summaries and identity/header tables unpartitioned so a gradebook query does not aggregate
historical partitions.

### Ten million questions

Keep the catalog central and shared:

- Keep `problem` and compact `problem_version` metadata relational and directly indexed.
- Keep immutable version payloads in the existing hash-partitioned table.
- Keep images and other binary media in object storage, referenced by typed immutable asset rows.
- Use `catalog_search_document` as a replaceable search projection with GIN indexes. PostgreSQL GIN
  is designed for composite values such as documents and their component keys
  ([PostgreSQL GIN](https://www.postgresql.org/docs/17/gin.html)).
- Benchmark public-ID lookup, title/facet search, exact-version load, and newest-visible-version load
  on a production-shaped dataset.
- Add a read replica or external search service only when measured latency, index size, refresh lag,
  or write amplification establishes the need. PostgreSQL remains authoritative.

### Connections and sharding

The current SQLx pool is bounded at eight application connections per API process and four for the
separate grader pool. Record API replica count x pool limits as part of capacity planning. Add
PgBouncer only when measured connection demand or database memory requires it.

The data-access layer uses transaction-local role and tenant settings, which is compatible in shape
with transaction pooling. PgBouncer documents that session features do not survive transaction
pooling, so any future pooler acceptance gate must prove every transaction establishes its own role
and tenant state
([PgBouncer pooling modes](https://www.pgbouncer.org/features.html),
[PgBouncer configuration](https://www.pgbouncer.org/config)).

Start with one primary PostgreSQL cluster. Preserve tenant-leading private keys and avoid cross-tenant
business transactions so a later `tenant_placement` router can move a whole tenant to another
cluster. Do not create a database per instructor, course, or student.

### Maintenance and recovery

High-write attempt and submission tables need observed autovacuum settings. PostgreSQL vacuuming
recovers reusable space, refreshes planner statistics and visibility information, and prevents
transaction-ID wraparound; disabling it is not a scale strategy
([PostgreSQL routine vacuuming](https://www.postgresql.org/docs/17/routine-vacuuming.html)).

Before the pilot stores real student work:

- Pin the deployed PostgreSQL release rather than relying on a mutable container tag.
- Select numerical recovery-point and recovery-time objectives with the institution.
- Enable encrypted base backups plus WAL archiving for point-in-time recovery.
- Back up role definitions and grants without login password hashes, as required by
  [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).
- Restore into an isolated clean cluster and prove migration checksums, owners, grants, forced RLS,
  tenant isolation, application writes, and broker functions.
- Reconcile PostgreSQL recovery with object-storage versioning and retention; restoring only one
  side can produce broken object references.

PostgreSQL distinguishes SQL dumps, file-system backups, and continuous WAL archiving. A backup is
accepted only after an isolated restore proves it
([PostgreSQL backup overview](https://www.postgresql.org/docs/17/backup.html),
[PostgreSQL PITR](https://www.postgresql.org/docs/17/continuous-archiving.html)).

## Activation thresholds

Do not pre-install complexity merely because future numbers are large.

| Change                   | Evidence required before activation                                                          |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| New index                | Same representative query before/after with result equality and `EXPLAIN (ANALYZE, BUFFERS)` |
| More catalog partitions  | Measured partition/index size or maintenance bottleneck                                      |
| More activity partitions | Retention/query windows no longer prune effectively                                          |
| PgBouncer                | Aggregate pool demand approaches the safe server connection budget                           |
| Read replica             | Measured read load competes with grading writes; acceptable replica lag is defined           |
| External search          | PostgreSQL search misses an explicit latency, ranking, or operational requirement            |
| Tenant sharding          | One cluster misses measured capacity or isolation objectives after normal tuning             |

## Validation gates

Before the fall pilot:

1. Apply every migration to a fresh PostgreSQL 17 database and verify the SQLx ledger twice.
2. Load the upper pilot shape, including 45,000 runs and 675,000 attempts/submissions.
3. Capture plans and timings for login/session resolution, course list, assignment list, run resume,
   submission, gradebook summary, instructor item analysis, and retention selection.
4. Exercise concurrent class start, exact submission retry, scoring-generation supersession, and
   worker backpressure.
5. Run a role matrix proving cross-tenant and cross-student reads/writes fail while exact instructor
   course access succeeds.
6. Verify the identity provider, account disablement, recovery, session revocation, throttling,
   non-enumerating errors, and audit path.
7. If passkeys are enabled, test registration, authentication, multiple credentials, revoked
   credentials, lost-device recovery, RP/origin mismatch, challenge replay, and keyboard-only use on
   the supported browser matrix.
8. Restore PostgreSQL and object storage into an isolated environment and record observed recovery
   time and data-loss window.
9. Record row counts, relation/index sizes, default-partition writes, dead tuples, autovacuum state,
   pool saturation, p95 latency, and failed authorization counts during the pilot.

These gates turn the scale design into measured evidence. They also leave room to evolve the schema
when the first real semester reveals better requirements.
