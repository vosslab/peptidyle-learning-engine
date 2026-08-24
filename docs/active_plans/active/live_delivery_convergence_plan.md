# Plan: WP-PROF-LD3 live-delivery convergence

## Status and authority

WP-PROF-LD3 is the current professor package. The owner retired the unaccepted WP-PROF-T4
execution sidecar on 2026-08-24 and selected the live demo as the canonical product and acceptance
path. [LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md) leads product behavior,
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) records the durable owner decision, and
[implementation_status.md](../implementation_status.md) owns the current handoff and migration
allocation.

## Outcome

PLE has one assignment-execution model:

- Instructors author, publish, preview current policy, and inspect audited learner work.
- Students exercise the actual assignment delivery path through ordinary enrollments, runs,
  attempts, submissions, immutable receipts, grades, and repeat practice.
- Deterministic graders execute on the server from issuance-owned private material.
- Browser contracts contain the visible question and response shape needed for interaction while
  answer material and grading implementations remain server-owned.
- The disposable HTTPS production stack creates and mutates ordinary PLE data through visible
  workflows and supplies canonical browser acceptance.

Preview remains the accepted WP-PROF-T3 live computation over ordinary course policy. It explains
effective state and provenance. Delivery validation uses ordinary Student work, so evidence from
preview, learner execution, grading, receipt replay, gradebook totals, and Instructor inspection
connects through the same product records.

## Scope

### Retire the unaccepted sidecar

Remove the separate execution aggregate from domain models, stores, server composition, routes,
generated contracts, migrations, tests, and active documentation. Retain its WP-PROF-T4 identity in
the status ledger as retired so package names remain globally unambiguous.

Remove the unaccepted sidecar-only migration allocations `2026081811`, `2026081813`,
`2026081815`, `2026081821`, and `2026081822`. The project is pre-production, so this cleanup
recomposes the current migration epoch before the clean baseline instead of shipping compatibility
tables or destructive forward cleanup.

### Preserve and finish ordinary live authority

WP-PROF-LD3 owns the still-unaccepted ordinary-course capabilities in `2026081812`,
`2026081814`, and `2026081816` through `2026081820`, plus `2026081823`:

- assignment and complete assignment-definition mutation with revision conflict checks;
- course-group, entitlement, accommodation, and schedule-exception source authority;
- immutable issued-question source and private execution snapshots for normal learner work;
- session-derived course creation and co-instructor invitation mutations;
- grade-scheme, export-audit, scoring preparation, and scoring finalization brokers; and
- least-privilege PostgreSQL roles, forced RLS, typed conflicts, and route-bound witnesses.

Each capability has one broker-owned mutation path and one typed application contract. Read
projections use snapshot reads. Mutations lock and verify their complete authority set inside the
owning broker. Server composition routes every operation through the narrow capability that owns it.

### Canonical evidence path

The connected acceptance path uses one disposable HTTPS production stack. Instructor workflows
create the course, assignment, policies, and roster state through visible UI actions. Student
workflows create runs and submissions through the visible learner UI. Instructor pages then show
the resulting grade settings, gradebook, and audited learner-work evidence. Narrow backend setup is
reserved for installation baselines and controlled infrastructure faults.

## Dependency and handoff

WP-PROF-LD3 depends on accepted WP-PROF-T3 and the accepted learner-delivery and issued-receipt
foundations. It is a convergence package, so discovery, collections, curricula, and grading
operations continue to use their existing package identities and dependency order.

After WP-PROF-LD3 acceptance, the professor queue advances to dependency-ready WP-PROF-T5 item
pools. WP-PROF-D1 discovery can proceed as an independent lane under the professor plan.

## Validation

Focused implementation evidence:

1. Rust formatting, default and all-feature checks, strict Clippy, and the affected domain, data
   access, server, and project-tools tests pass.
2. Type generation converges and the TypeScript typecheck, lint, format, and Node tests pass.
3. A fresh PostgreSQL cluster applies the final migration ledger and passes assignment mutation,
   group/accommodation, learner issuance/submission/replay, teaching authority, grade settings,
   and scoring authority tests with exact least-privilege roles and forced RLS.
4. The canonical production HTTPS browser suite exercises authoring, preview, learner delivery,
   deterministic grading, grade settings, and Instructor review on ordinary live state.
5. The WebWork service and replica-restart oracles pass against the same product topology.
6. `source source_me.sh && ./all_test.sh` passes on the final material tree with every required gate
   run and every owned cleanup receipt empty.

One-time implementation inventories may confirm that the retired aggregate has no remaining active
code, migration, route, generated-contract, or current-plan owner. Permanent tests stay focused on
the live behavior and security boundary they protect.

## Acceptance

WP-PROF-LD3 is accepted only when all validation above is green under
[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md), the active status and changelog record the
final evidence, and the current package handoff advances. A partial focused suite remains bounded
evidence rather than package acceptance.
