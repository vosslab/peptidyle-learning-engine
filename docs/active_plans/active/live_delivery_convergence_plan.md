# Plan: WP-INST-LD3 live-delivery convergence

## Status and authority

WP-INST-LD3 was accepted on 2026-08-24. It established the live demo as the canonical product and
acceptance path over ordinary assignments, Assignment Attempts, grading, evidence, and Instructor
inspection. WP-INST-T5 item pools are accepted. [LIVE_DEMO_SPEC.md](../../LIVE_DEMO_SPEC.md) leads
product behavior,
[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) records durable owner decisions, and
[implementation_status.md](../implementation_status.md) owns the current handoff and migration
allocation.

## Outcome

PLE has one assignment-execution model:

- Instructors author, publish, preview current policy, and inspect audited student work.
- Students exercise the actual assignment delivery path through ordinary enrollments, Assignment Attempts,
  attempts, submissions, immutable receipts, grades, and repeat practice.
- Deterministic graders execute on the server from issuance-owned private material.
- Browser contracts contain the visible question and response shape needed for interaction while
  Answer Key data and Question Grader code remain server-owned.
- The disposable HTTPS production stack creates and mutates ordinary PLE data through visible
  workflows and supplies canonical browser acceptance.

Preview remains the accepted WP-INST-T3 live computation over ordinary course policy. It explains
effective state and Assignment Policy Source. Delivery validation uses ordinary Student work, so evidence from
preview, student execution, grading, receipt replay, gradebook totals, and Instructor inspection
connects through the same product records.

## Scope

### Converge public seeded-role entry

The live demo exposes one public role-entry surface for every seeded human role: Student, Instructor, and
Sysadmin. Graphify's identity-path snapshot identifies `seeded_account_selector_router()` and
`select_seeded_account()` in `crates/server/src/auth/seeded_account_selector.rs`, which issue an ordinary
account session. LD3 makes that selector the direct seeded-role entry and keeps passkeys as ordinary
account-security behavior after entry. The selector supplies identity; the server resolves the account,
session, course, membership, role, and authorization state from live PLE records.

Direct Sysadmin entry preserves the full Sysadmin capability set and keeps ordinary passkey enrollment and sign-in
demonstrable after entry. The public path opens the selected ordinary account immediately. PLE generates and
manages disposable internal demo credentials only as installation-scoped process-isolation capabilities. SOPS is
reserved for a later deployment design that needs persistent or externally supplied credentials. Ordinary live
state, visible mutations, and reset-to-seeded-baseline behavior remain unchanged.

### Ordinary live authority

WP-INST-LD3 owns the accepted ordinary-course capabilities in `2026081812`,
`2026081814`, and `2026081816` through `2026081820`, plus `2026081823`:

- Assignment and complete Assignment Content mutation with revision conflict checks;
- course-group, entitlement, accommodation, and schedule-exception source authority;
- immutable issued-question source and private execution snapshots for normal student work;
- session-derived course creation and co-instructor invitation mutations;
- grade-scheme, export-audit, scoring preparation, and scoring finalization operations; and
- least-privilege PostgreSQL roles, forced RLS, typed conflicts, and route-bound witnesses.

Each capability has one protected database operation and one typed application contract. Read
projections use snapshot reads. Mutations lock and verify their complete authority set inside the
owning protected database operation. Server composition routes every operation through the narrow
authority that owns it.

### Canonical evidence path

The connected acceptance path uses one disposable HTTPS production stack. Instructor workflows
create the course, assignment, policies, and roster state through visible UI actions. Student
workflows create Assignment Attempts and submissions through the visible Student UI. Instructor pages then show
the resulting grade settings, gradebook, and audited student-work evidence. Narrow backend setup is
reserved for installation baselines and controlled infrastructure faults.

## Dependency and handoff

WP-INST-LD3 depends on accepted WP-INST-T3 and the accepted student-delivery and issued-receipt
foundations. It is a convergence package, so discovery, collections, curricula, and grading
operations continue to use their existing package identities and dependency order.

WP-INST-T5 accepted its item-pool productization on the canonical live path. The active Instructor
handoff is recorded only in [implementation_status.md](../implementation_status.md).

## Validation

Focused implementation evidence:

1. Rust formatting, default and all-feature checks, strict Clippy, and the affected domain, data
   access, server, and project-tools tests pass.
2. Type generation converges and the TypeScript typecheck, lint, format, and Node tests pass.
3. A fresh PostgreSQL cluster applies the final migration ledger and passes assignment mutation,
   group/accommodation, student issuance/submission/replay, teaching authority, grade settings,
   and scoring authority tests with exact least-privilege roles and forced RLS.
4. The canonical production HTTPS browser suite exercises authoring, preview, student delivery,
   deterministic grading, grade settings, and Instructor review on ordinary live state.
5. The WebWork service and replica-restart oracles pass against the same product topology.
6. The connected browser lane starts as an anonymous visitor, visibly enters each seeded role including Sysadmin,
   and verifies that account, session, course, membership, role, and authorization are resolved by the server from
   ordinary live state rather than accepted as browser role claims.
7. From direct role entry, the browser visibly exercises the ordinary passkey path for both named acceptance
   personas: Elena Instructor and Morgan Sysadmin each enroll a passkey, sign out, and sign back in. Elena retains
   Instructor authorization, and Morgan retains Sysadmin authorization and full capabilities throughout the flow.
   Each journey begins directly from its selected ordinary account.
8. Reset regenerates the seeded baseline, discards installation-scoped process-isolation credentials, and preserves
   the ordinary live-state and full-capability contracts.
9. `source source_me.sh && ./all_test.sh` passes on the final material tree with every required gate
   Assignment Attempt and every owned cleanup receipt empty.

## Acceptance

WP-INST-LD3 is accepted under [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md). The active
status and changelog record its final evidence, and the current package handoff has advanced.
