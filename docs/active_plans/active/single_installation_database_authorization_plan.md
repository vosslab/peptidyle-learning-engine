# Plan: Single-installation database authorization

## Context

This plan implements [single_installation_authorization_plan.md](single_installation_authorization_plan.md).
[implementation_status.md](../implementation_status.md) is the sole current package and allocation
registry. This plan does not supersede [release_completion_plan.md](release_completion_plan.md).

## Objectives

- Deliver the staged PostgreSQL authorization and Store boundary for `WP-SD1-C`.
- Enforce actor-derived, exact durable relationships under forced RLS.
- Prove connected PostgreSQL and protected-service behavior before SD1-E begins.
- Promote one complete reviewed epoch only after required acceptance gates pass.

## Design philosophy

- The main plan owns product relationships, privacy outcomes, consent, disclosure, and observers.
- The status registry owns allocation numbers and the `WP-SD1-C` package identity.
- This plan owns capability ACLs, transaction-scoped actor installation, RLS, Store parity, and gates.
- [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) classifies durable tests and one-time evidence.

## Scope

- Non-runtime fresh migration staging and one reviewed promotion.
- PostgreSQL principals, default ACLs, broker functions, forced RLS, and exact predicates.
- Actor-aware Memory/PostgreSQL Store parity and direct protected-service support.
- Connected PostgreSQL, restricted-login, worker-scope, and protected-service acceptance.

## Non-goals

- Define relationship kinds, observer projections, consent, disclosure policy, or privacy outcomes.
- Define browser DTOs, routes, visible workflows, or release acceptance.
- Duplicate the allocation ledger or create packages named after C/D execution tasks.
- Activate a partial epoch, Alpha bridge, tenant compatibility SQL, or caller-supplied actor scope.

## Architecture boundaries and ownership

### Product-contract handoff

The main plan owns equal creator/co-Instructor results and all future-relationship privacy semantics.
This plan implements exact database and Store enforcement without redefining those outcomes.

`2026082911` owns minimal-Blueprint construction; the canonical course-creation capability invokes
it while atomically creating the bound CourseInstance and initial Instructor membership.
`2026082913` owns immutable CourseInstance adoption records and their idempotency key;
`2026082929` owns the only executable curriculum-adoption apply/reconciliation capability over those
records; `2026082930` owns forced RLS for CourseInstance roots and dependent private state.
`2026082906` owns the shared Rust
actor-transaction installer. Curriculum adoption has exactly seven operations and never creates a
blank CourseInstance. An apply receives scope only from session-derived `ActorContext`; neither
adapter nor broker accepts a tenant.

### Principal and ACL capability

The status-allocated principal baseline owns capability roles, schemas, default privileges, and closed
grants. The bootstrap oracle checks LOGIN, NOINHERIT, NOBYPASSRLS, non-superuser status, memberships,
SET-option posture, and no ambient `PUBLIC` authority. Security-definer functions use a fixed search
path, explicit owner/revocation, named grants, and catalog inspection.

### Session and broker capability

The status-allocated session/broker capability exposes only necessary session fields under forced RLS.
The application role has no direct session-table grant or policy, and the broker cannot mutate auth rows.

### Actor-installer capability

The status-allocated actor installer is security-definer with a fixed search path. It refuses malformed
state and resolves missing, expired, or revoked sessions to absent actors using statement-time expiry.

### Trusted transaction boundary

Private `begin_actor(context)` in the trusted Rust transaction adapter sets the application role,
installs/verifies the session actor, and exposes only actor-aware Store operations. RLS uses a sealed
resolver plus an exact relationship predicate, never a caller-provided GUC. Worker scope derives from a
locked durable lease or manifest, never request input.

## Milestone plan

`SD1-C1`--`SD1-C8` and `SD1-D1`--`SD1-D6` are execution-task labels under `WP-SD1-C`, never package
identities or migration allocations. The status registry remains their sole allocation authority.

| Task | Outcome | Depends on | Narrow gate |
| --- | --- | --- | --- |
| `SD1-C1` | Principal/default-ACL baseline | B contracts/status allocation | Bootstrap role/ACL oracle |
| `SD1-C2` | Identity/session/challenge/passkey relations | `SD1-C1` | Authentication/session oracle |
| `SD1-C3` | Catalog/private-authoring schema | `SD1-C1`, needed `SD1-C2` keys | Catalog/workspace oracle |
| `SD1-C4` | BlueprintCourse/CourseInstance schema | `SD1-C3` | Parent/revision/propagation oracle |
| `SD1-C5` | Student delivery, grading, evidence, correction schema | `SD1-C4` | Delivery/grading/correction oracle |
| `SD1-C6` | Jobs, objects, external tools, exports, retention | `SD1-C1` and roots | Lease/object/tool oracle |
| `SD1-C7` | Brokers, forced RLS, ACL closure | `SD1-C1`--`SD1-C6` | Restricted-login/RLS oracle |
| `SD1-C8` | Staging convergence/promotion readiness | `SD1-C1`--`SD1-C7` | Fresh/no-op/checksum witness |
| `SD1-D1` | Actor transaction/raw-path boundary | `SD1-C7`, `WP-SD1-B1-P1` | Actor-install/refusal conformance |
| `SD1-D2` | Catalog/authoring Store parity | `SD1-C3`, `SD1-D1` | Parity/restricted-PostgreSQL oracle |
| `SD1-D3` | Course/Student Store parity | `SD1-C4`, `SD1-D1` | Creator/co-Instructor/cross-course oracle |
| `SD1-D4` | Grading/evidence Store parity | `SD1-C5`, `SD1-D1` | Generation/revision/receipt oracle |
| `SD1-D5` | Operations Store parity | `SD1-C6`, `SD1-D1` | Lease-target oracle |
| `SD1-D6` | Direct protected-service support/acceptance | `SD1-D2`--`SD1-D5` | Connected protected-service acceptance |

### C-to-D handoff

SD1-C accepts only when complete non-runtime staging enforces its durable capability surface. SD1-D
then exposes no actor-unaware protected Store path. SD1-D accepts only when connected protected-service
acceptance proves the same exact-scope outcomes through PostgreSQL and Store paths; SD1-E then begins.

## Acceptance criteria and gates

- Fresh, no-op, and checksum behavior for complete non-runtime staging.
- Restricted roles prove closed ACLs, no `PUBLIC` execution, forced RLS, and no app session-row read.
- Actor installation accepts only valid state and refuses raw, malformed, forged, missing, expired, and
  revoked contexts.
- Exact relationship gates prove Student-self, cross-course/cross-user refusal, revocation, equal
  creator/co-Instructor results, and no observer escalation beyond the main-plan contract.
- Worker/object gates reject foreign targets, stale generations, wrong handler families, and supplied
  scope before read, write, dispatch, or finalization.
- Store parity proves concealment, immutable evidence, revision checks, leases, and generation fences.
- Connected protected-service acceptance proves the SD1-E boundary, not a unit/mock/browser substitute.

## Test and verification strategy

Apply [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md). Permanent tests cover stable Store
behavior, authorization, refusal, and contract invariants. Disposable PostgreSQL oracles cover
migration, principal, RLS, broker, worker, object, and connected-service claims. Browser and visual
acceptance remain main-plan/SD1-F work. Graphify, mappings, ACL inventories, and promotion manifests
are one-time evidence. Required unrun or skipped gates keep their task incomplete.

## Migration and compatibility policy

Only [implementation_status.md](../implementation_status.md) assigns allocations. Staging is non-runtime:
the sole compile-time `MIGRATOR` stays active until one reviewed promotion. Accepted migrations are
immutable. The promotion archives the old active directory byte-for-byte with names, versions, SHA-256,
and source commit; promotes the complete staged directory in one reviewed change; resets only a confirmed
disposable cluster; and proves fresh/no-op/checksum, restricted roles, forced RLS, broker-only session
reads, actor refusal, and connected-service acceptance. No legacy ledger bridge, partial active epoch,
tenant compatibility SQL, or Alpha selector survives.

## Risk register

| Risk | Control |
| --- | --- |
| Ambient role or `PUBLIC` authority bypasses policy | Closed grants/default ACLs plus catalog and restricted-login oracle. |
| Caller selects actor or worker scope | Private actor transaction boundary and locked durable scope derivation. |
| Broker leaks or mutates session state | Minimal forced-RLS broker view; application direct access and mutation denied. |
| Staging affects runtime behavior | Non-runtime staging and one reviewed promotion after connected acceptance. |
| Store parity masks PostgreSQL defect | Restricted PostgreSQL and direct protected-service oracles remain required. |
| Product privacy semantics drift into enforcement | Main plan remains their sole authority. |

## Status

This is execution planning only. It does not claim SD1-C/D implementation, connected acceptance,
browser acceptance, or release completion.
