# Flat-question current state audit (post-migration rename)

> **Historical audit.** This dated snapshot is retained as evidence, not current task direction.
> Current authority is the [release completion plan](../active/release_completion_plan.md) and
> [implementation status](../implementation_status.md).

Date: 2026-08-09
Scope: read-only bounded audit of uncommitted flat-question publication state after migration rename.

## Status

`DONE_WITH_CONCERNS`

The flat-question source model and persistence contracts are present, but the publication path is only partially wired. The in-memory catalog implements flat-question promotion validation and grading persistence, while PostgreSQL catalog publication and server publish routes have not been connected to flat-question promotion material yet. This blocks end-to-end publication persistence for static flat questions despite migration and schema support.

## Evidence (file:line)

Plan and guidance:

- `docs/HUMAN_GUIDANCE.md:77-85` define flat-question canonical source as private authorative format; non-signable source boundary plus separate grader/public output requirement.
- `docs/active_plans/implementation_plan.md:611-617` and `657-663` describe flat-question canonical source persistence and immutable published source.
- `docs/active_plans/partial_commit_status.md:342-344` marks "complete the PLE flat-question workspace, immutable-source, public-payload, and grader-only persistence path" as next work.

Current model/data contracts:

- `crates/learning-data-access/src/flat_question.rs:204-234` defines `FlatQuestionPublicationPromotion` with `source` and `grading`.
- `crates/learning-data-access/src/flat_question.rs:268-287` defines `FlatQuestionGradingStore` and `FlatQuestionStore` interfaces used for grading reads and staged sources.
- `crates/learning-data-access/src/publication_validation.rs:437-456` enforces native publication source behavior and requires source artifact only when staged flat promotion is present.
- `crates/learning-data-access/src/publication_validation.rs:460-516` adds flat-specific checks: promotion must exist, source/tenant/workspace/revision checks, family match, and public artifact hash alignment.

In-memory store:

- `crates/learning-data-access/src/in_memory/catalog.rs:14-30` wires `validate_source_artifact_for_publication` and reads `flat_question_promotion`.
- `crates/learning-data-access/src/in_memory/catalog.rs:32-46` and `79-130` show QTI/flat branch behavior with `validate_flat_question_publication`.
- `crates/learning-data-access/src/in_memory/catalog.rs:118-125` and `252-262` show staged flat source lookup and promotion cleanup/published grading storage.

PostgreSQL store (first-party persistence gap):

- `crates/learning-data-access/src/postgres/catalog.rs:50-61` selects only QTI promotion and validates source artifact generically.
- `crates/learning-data-access/src/postgres/catalog.rs:97-117` only handles staged QTI promotion reads/validation.
- `crates/learning-data-access/src/postgres/catalog.rs:298-305` and `302-321` show source artifact + QTI asset/grading promotion writes only.
- `crates/learning-data-access/src/postgres/catalog.rs:299-321` has no `flat_question_promotion` branch and no call to `ple_promote_flat_question_grading`.

Server callsites:

- `crates/server/src/catalog.rs:668-678` build `PublishDraftCommand` with `source_artifact: None`, `qti_promotion: None`, `flat_question_promotion: None` and an inline comment noting source-backed adapters are not wired.
- `crates/server/src/qti_publication.rs:271-281` sets `flat_question_promotion: None` even in QTI flow.
- `crates/server/src/native_backend.rs:401-403`, `crates/server/src/webwork_backend.rs:433-440`, and `crates/server/src/imathas_backend.rs:1256-1258` all set `flat_question_promotion: None`.
- Repository-wide search confirms no active flat promotion construction: `rg -n "flat_question_promotion:\\s*Some" crates/learning-data-access crates/server` returns no matches.

Schema and DB function evidence:

- `schemas/migrations/2026080802_catalog_authoring.sql:317-335` creates `workspace_flat_question_source`.
- `schemas/migrations/2026080802_catalog_authoring.sql:868-875` defines cache-clearing trigger for staged flat source.
- `schemas/migrations/2026080802_catalog_authoring.sql:743-749` adds tenant-scoped policies for staged flat-question source.
- `schemas/migrations/2026080805_operations_analytics.sql:427-438` and `462-468` define `ple_promote_flat_question_grading(...)` plus tenant/hash matching and integrity checks.
- `schemas/migrations/2026080805_operations_analytics.sql:1470-1492` grants DB execution only to `ple_app`.

## Remaining gaps (bounded, next-package scoped)

Compile gaps:

- No compile errors or missing symbols in current check results.
- Runtime path for flat-question publication is not fully connected; this is not a compile-time but an end-to-end functional gap.

Contract gaps:

- No PostgreSQL atomic publication path for flat-question `PublishDraftCommand`.
- Missing invocation of `validate_flat_question_publication` in Postgres catalog publish; native flat-question path is only validated/implemented in memory store.
- Missing call to SQL function `ple_promote_flat_question_grading` during Postgres publication.
- No server endpoint/caller path currently supplies `flat_question_promotion` to `publish_draft`.

Security boundaries:

- Schema/rules exist (`workspace_flat_question_source` and `published`-oriented RBAC/tenant policies), and `ple_promote_flat_question_grading` requires tenant-match and hash checks.
- Because the publication route is not sourcing flat-question promotion objects, the runtime cannot yet exercise those protections for actual static flat-question publishes.

Test gaps:

- Conformance suite currently validates QTI publication behavior (`tests/conformance.rs:...qti`), but no flat-question publication promotion test exists.
- `cargo test -p learning-data-access flat_question -- --list` lists only flat-question model/store unit tests and an in-memory unauthorized visibility test; no end-to-end publish-path test appears.
- No `flat_question_promotion: Some(...)` callsites exist to exercise contract enforcement.

## Narrow validation commands used

- `cargo check -p learning-data-access`
- `cargo check -p server_core`
- `cargo test -p learning-data-access --test '*'`
- `cargo test -p learning-data-access flat_question -- --list`

Observed outputs:

- all targeted checks completed successfully, with existing conformance tests passing.

## Suggested next vertical package

Implement the Postgres-backed flat-question publish package next, centered in:

1. `crates/learning-data-access/src/postgres/catalog.rs`:
   - add `flat_question_promotion` detection and conflict checks,
   - call `validate_flat_question_publication`,
   - persist source object/immutable public projection (if applicable to this stage),
   - invoke `ple_promote_flat_question_grading`.
2. `crates/server/src` publish callsites:
   - route/adapter path to construct and pass `flat_question_promotion` for native flat-question drafts.
3. `crates/learning-data-access/tests/conformance.rs`:
   - add a persistent Postgres conformance test for successful flat-question publication and failure cases.
