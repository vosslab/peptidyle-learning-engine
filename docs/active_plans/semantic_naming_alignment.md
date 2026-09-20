# PLE semantic naming alignment

> **Planning authority.** This plan turns the current semantic-naming audit into
> dispatchable pre-production work. [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md)
> remains the product authority if a later document conflicts with it.

## Summary

Make the PLE naming model mechanically coherent across PostgreSQL, Rust,
generated contracts, JSON, TypeScript, routes, tests, fixtures, and living
documentation. A scalar identity ends in `Id`, an exact multi-value identity is
a `Tuple`, and mutable or immutable version clocks use `EditNumber` or
`RevisionNumber`. PLE is pre-production, so remove retired shapes outright:
no aliases, dual DTOs, compatibility adapters, or legacy routes.

The existing dirty Question JSON/ETag worktree changes are an entry condition:
preserve them, rebase this work onto them, and make their final contract conform
to this plan.

**Canonical ownership rule.** Every domain value has one canonical semantic
name and one owning abstraction. Transport, serialization, persistence, and UI
layers adapt to that abstraction rather than defining parallel names for the
same concept.

## Canonical decisions

| Value actually represents | Canonical shape |
| --- | --- |
| One object identity | `...Id`; Rust/SQL `..._id`; JSON/TS `...Id` |
| Exact immutable Question or Blueprint version | `QuestionRevisionTuple` or `BlueprintRevisionTuple`, with identity plus `revisionNumber` |
| Current mutable state clock | Domain-specific `...EditNumber` |
| Immutable version sequence | Domain-specific `...RevisionNumber` |
| Cardinality | `...Count`, not `...Number`; a real count is not a version number |
| Ordinal placement | `...Position`; it is neither an identity nor a version number |
| Full resource at its own JSON root | `id` is permitted only as the immediate resource identity, typed as its specific `...Id` |
| Identity nested in a relationship, selection, provenance, command, route, or DTO | Use the precise `...Id`, `...Tuple`, or `...Number`, never shorthand such as `source`, `target`, `course`, `assessment`, `entry`, or `attempt` |
| HTTP conditional transport | `ETag` and `If-Match` remain standard header spellings only; all domain/browser values use the exact Edit or Revision Number |

## Work packages

### 1. Establish enforceable contract ownership

**Owner:** Contract owner (Rust/API), with verification owner.

- Update `docs/NAMING_CONVENTIONS.md` and `docs/TERMINOLOGY_CONTRACT.md` with
  the canonical decisions, including the distinction between a root resource
  `id` and a nested identity. State that an HTTP ETag is only a quoted encoding
  of an explicitly named domain number.
- During this migration, run the untracked one-time scanner
  `tests/_temp/test_semantic_boundary_names.py` aggressively to find retired
  names and verify cleanup. Do not promote that file into `tests/test_*.py`.
  After the migration, promote only narrow durable mechanically decidable
  invariants, such as forbidding domain-facing `BlueprintRevisionEtag` or
  `QuestionAvailabilityEtag`, if they still earn a permanent gate.
- Add mechanical enforcement for syntactically decidable rules: forbidden
  domain-facing `*Etag` names, missing `Id` / `Tuple` / `Number` suffixes where
  the owning type is known, and retired serialized field names. Scan PLE-owned
  production contract roots: Rust browser/domain/server/LDA contracts, SQL
  base-schema routines, TypeScript API/client/route contracts, generated
  declarations, and living API documentation.
- Use focused owning-boundary contract tests for cases that require semantic
  interpretation, such as whether a `source` or `target` field is a complete
  comparison-side view versus a nested identity.
- Allow only direct HTTP protocol constants/header access (`ETAG`, `IF_MATCH`,
  `headers.get("etag")`), deliberate retired-input rejection fixtures,
  historical changelogs/archived plans, ordinary document/policy sources,
  comparison-side roles, predicates/event literals, real counts/positions, and
  immediate resource-root `id`.
- **Success condition:** newly introduced drift is caught by the cheapest
  reliable layer. A syntactically decidable production-boundary name fails the
  mechanical scan; a semantically ambiguous case fails its owning-boundary
  contract test.

### 2. Replace scalar relationship identities with precise names

**Owner:** Domain and browser-contract owner.

Change each name through its Rust struct/serde contract, generated type, strict
TypeScript decoder, HTTP client, UI caller, fixture, and test:

| Family | Current meaning | Canonical replacement |
| --- | --- | --- |
| Course Instance adoption | `blueprintCourse: BlueprintCourseId` | `blueprintCourseId` |
| Course Instance creation | `assignedInstructor?: AccountId` | `assignedInstructorAccountId` |
| Course Instance provenance | `blueprintOrigin.id: BlueprintCourseId` | `blueprintOrigin.blueprintCourseId` |
| Course Instance nested resource | `course` holding a Course Instance projection | `courseInstance` |
| Course creation instructor projection | `id: AccountId` | `accountId` |
| Course invitation domain records | `course`, `target`, and non-root `id` | `courseInstanceId`, `targetAccountId`, and `courseInvitationId` |
| Blueprint Change Proposal commands | scalar `target: BlueprintCourseId` | `targetBlueprintCourseId` |
| Blueprint proposal/fork UI models | full-object `source` or `target` | `sourceBlueprintCourse` or `targetBlueprintCourse` |
| Assessment commands/route helpers | scalar `course`, `assessment`, `entry`, `attempt`, or `banner` | `courseInstanceId`, `assessmentId`, `assessmentEntryId`, `assessmentAttemptId`, or `courseBannerId` |
| Assessment response projections | `assessmentAttempt`, `assessmentEntry`, or `activeAssessmentAttempt` holding IDs | `assessmentAttemptId`, `assessmentEntryId`, or `activeAssessmentAttemptId` |

- Apply the Course Instance changes in
  `crates/learning-data-access/src/course_instance.rs`, `src/api/course_instance.ts`,
  and `src/api/decoders/course_instance.ts`; update
  `tests/test_course_instance_summary.mjs` so canonical JSON is accepted and
  retired fields are rejected.
- Apply invitation changes through `crates/domain/src/teaching_authority.rs`,
  `crates/question_model/src/teaching_authority.rs`, LDA/store implementations,
  teaching-operation routes, invitation UI/client code, SQL routines, and
  invitation tests. Wire-level root `id` remains only for an actual Course
  Invitation resource; internal fields remain `course_invitation_id`.
- Apply Assessment/Student Work changes through `src/api/assessment_attempt_issuance.ts`,
  `assessment_attempt_history.ts`, `assessment_attempt_navigation.ts`,
  `assessment_release.ts`, `assessment_pool_fork.ts`, `assessment_template.ts`,
  `contracts.ts`, their `src/api/http_client/` implementations, affected
  pages/components, and route tests.

### 3. Normalize exact revision identities and version-number types

**Owner:** Rust/API contract owner.

- Preserve the already-correct `QuestionRevisionTuple`, `BlueprintRevisionTuple`,
  `sourceRevisionTuple`, `targetRevisionTuple`, and
  `expectedTargetRevisionTuple`. A comparison-side object may retain `source`
  or `target` because it is a complete role view with explicit tuple and number
  members.
- Replace every production nested pair jointly selecting an immutable Question
  or Blueprint revision with its named tuple field. SQL relational columns
  remain `question_id` plus `revision_number` or `blueprint_course_id` plus
  `revision_number`; normalized storage does not require a tuple column.
- In `src/api/blueprint_course.ts` and its decoder/client, replace raw-string
  revision arguments and `BlueprintRevisionEtag` with `BlueprintRevisionNumber`.
  Use `expectedCurrentBlueprintRevisionNumber` for save preconditions. Remove
  duplicate header-derived `revisionEtag` fields; the returned
  `blueprintRevisionTuple` is authoritative.
- In `src/api/question_availability.ts`, replace raw `editNumber: string` and
  `QuestionAvailabilityEtag` with `QuestionAvailabilityEditNumber` and
  `expectedQuestionAvailabilityEditNumber`.
- In Assessment Pool, Template, and Release contracts, replace raw version
  values with `QuestionPoolEditNumber`, `AssessmentTemplateEditNumber`, and
  `AssessmentEditNumber`.
- Rename `question_revision_tuple(question: String, revision: i32)` in
  `crates/learning-data-access/src/postgres/assessment_release_decode.rs` to
  parameters explicitly naming `question_id` and `revision_number`.
- Rename `revision` in
  `src/features/question_picker/question_picker_model.ts` to
  `blueprintRevision`, because it holds a complete view, not a scalar number.

### 4. Make ETag strictly an HTTP adapter concern

**Owner:** Browser-client and server-route owner.

- Create one narrow conditional-request adapter. Its public functions use exact
  domain terms such as `ifMatchHeaderForAssessmentEditNumber` and
  `assertResponseMatchesBlueprintRevisionNumber`; only its implementation
  accesses `"etag"` or writes `If-Match`.
- Server handlers retain standards-compliant `ETag`/`If-Match`, parse one
  positive strong numeric value, compare it to the exact expected domain number,
  and return `412` when stale. Quoted validators never appear in response
  models. This preserves strict boundary validation and server-side
  authorization. [ASVS 1.5.2, 2.2.1, 2.2.2, 4.2.5, 8.2.2, 8.3.1]
- Convert Blueprint content to `BlueprintRevisionNumber`; Blueprint metadata,
  lifecycle, and promotion to `BlueprintEditNumber`; Course metadata to
  `CourseEditNumber`; availability to `QuestionAvailabilityEditNumber`; Draft
  Question source/JSON/general feedback to `DraftQuestionEditNumber`; live
  Assessment operations to `AssessmentEditNumber`; and Template operations to
  `AssessmentTemplateEditNumber`.
- Update the dirty Question JSON files--`question_json_client`, repository,
  editor model/types/workspace/page, general-feedback client, draft editor page,
  and their tests--without overwriting user work. Replace local signals, maps,
  results, and parameters named `etag` with the exact Draft Question Edit
  Number.
- Update `src/api/http_client/blueprint_course.ts`, `question_availability.ts`,
  `assessment_release.ts`, `assessment_pool_fork.ts`, `assessment_template.ts`,
  `blueprint_stewardship.ts`, and `course_instance.ts` so raw quoted headers
  never escape their adapter.
- Rename database-only validator terminology in
  `schemas/base_schema/50_functions/blueprint_lineage.sql`,
  `blueprint_operations.sql`, and `course_operations.sql` from ETag to the
  corresponding Edit/Revision Number. Rename LDA local `accepted_etag` in
  `postgres/blueprint_change_proposal.rs` to
  `accepted_target_blueprint_edit_number`.
- Update SQL routine callers, database tests, grants/catalog snapshots, and
  error assertions together. Preserve atomic stale-state rejection and existing
  authorization/state-transition behavior. [ASVS 2.1.1, 2.1.2, 2.3.1, 2.3.3]

### 5. Retire the stale generic Assessment client

**Owner:** Browser-client owner.

- Inventory every capability currently exposed by the generic Assessment
  client in `src/api/http_client/request.ts`, `response.ts`, and
  `http_client.ts`. Determine the canonical owner of each capability.
- Move each still-valid capability to that owner, then update callers. Route
  Assessment work through Course Instance, Assessment Release, Assessment
  Attempt Issuance, Navigation, History, Pool Fork, or Student View as the
  owning surface.
- Treat any surviving capability without a clear owner as a design gap that
  this work resolves: assign an owner or delete the capability. Do not retain
  a compatibility facade.
- After callers have moved, remove the generic client and obsolete
  `/api/courses/...` assessment and generic attempt routes completely.
- Rename Axum placeholders and client route templates to
  `{course_instance_id}`, `{assessment_id}`, `{assessment_entry_id}`,
  `{assessment_attempt_id}`, and `{course_banner_id}`. Values and resource URLs
  do not change; only labels/local bindings become accurate.
- Update `src/ribbon/capability_registry.ts` and `docs/API_CONTRACTS.md` in the
  same change. Add tests proving current paths work and obsolete generic methods
  and `/api/courses` assessment routes are absent.
- **Success condition:** there is one obvious API ownership path per Assessment
  capability and no compatibility facade remains.

### 6. Regenerate, document, and verify as one contract change

**Owner:** Integration and verification owner.

- Change Rust owning contracts first, then run the repository TypeScript
  generator. Never hand-edit `generated/api`; commit only generator output.
- Update living documentation: `docs/API_CONTRACTS.md`, `docs/CONTRACTS.md`,
  `docs/DATABASE_STRUCTURE.md`, `docs/DESIGN_DECISIONS.md`,
  `docs/NAMING_CONVENTIONS.md`, and `docs/TERMINOLOGY_CONTRACT.md`. Describe
  ETag only as the HTTP header carrying the stated number.
- Do not rewrite dated changelogs, archived plans, vendored text, or deliberate
  rejection fixtures. They are historical/test evidence, not current contracts.
  Add one new `docs/CHANGELOG.md` entry describing the direct pre-production
  replacement.
- Add focused tests for canonical JSON acceptance and retired-name rejection,
  generated-type parity, exact branded TypeScript values, route construction,
  header adapter isolation, stale preconditions, unchanged authorization, and
  SQL routine parameter/transaction behavior.
- Run generator checks, focused Rust/Node/SQL suites, `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, TypeScript checking,
  `source source_me.sh && ./launchers/run_fast_checks.sh`, schema documentation
  and style generation, then `source source_me.sh && ./launchers/all_test.sh`.
- Run targeted browser acceptance for Draft Question save, Blueprint revision
  save, Course adoption, invitation acceptance, Assessment edit/release/unrelease,
  and Question Pool fork. Each path must keep the canonical number in client
  state while using correct standard HTTP headers.

## Explicit out-of-scope classifications

- Retain correct Question/Blueprint revision tuple contracts and Blueprint
  Change Proposal tuple fields.
- Retain `source`/`target` for complete comparison-side roles, document sources,
  and policy-source values; they are not scalar identities.
- Retain `revision` as an event/discriminant literal, predicate name, or prose.
- Retain true counts and positions; `Count` and `Position` are more accurate than
  `Number` for those concepts.
- Retain SQL syntax, PostgreSQL catalog names, standard HTTP header spellings,
  vendor terminology, historical changelogs, and negative rejection inputs.
  Remove only their production-facing accidental aliases.

## Completion criteria

Work packages organize the work; they do not define completion. After all
packages pass, perform a fresh adversarial review of the *current* tree against
the canonical decisions and the canonical ownership rule. Any newly discovered
mismatch becomes in-scope required work.

Completion means the repository satisfies the semantic model:

- Every in-scope value has its canonical name, type, and owning abstraction
  through its full persistence-to-UI path.
- No public model, generated type, JSON contract, route binding, client method,
  UI state, fixture, or living document carries an ETag-shaped domain value.
- No legacy client or compatibility shape remains, and strict decoders reject
  retired spellings.
- Database/server concurrency checks remain atomic and authorized.
- Mechanical scans, owning-boundary contract tests, and repository gates pass
  against the current tree.

**Execution order:** complete work packages 1-4 first, package 5 after callers
have moved, then package 6, then the adversarial current-tree review. This is
intentionally sequential because each later package consumes the preceding
canonical contract.
