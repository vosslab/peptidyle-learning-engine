# System-wide semantic naming alignment

> **Planning authority.** [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) is the
> product authority. This plan makes its identity, Tuple, Edit Number, and
> Revision Number model coherent across PLE's current implementation.

## Summary

Make this statement true and evidence-backed:

> All fields, identifiers, domain concepts, and terminology in the PostgreSQL
> database structure, Rust code, TypeScript code, JSON/API contracts,
> Terminology Contract, and Human Guidance are in alignment.

PLE is pre-production. Replace obsolete shapes directly; do not add aliases,
compatibility readers, or legacy transport variants.

## Canonical decisions

- A scalar PLE identity uses an exact `...Id` name and nominal type. A scalar
  clock uses its full domain `...EditNumber` or `...RevisionNumber` name and
  type. This applies to fields, parameters, locals, JSON members, SQL API
  parameters/results, and persisted object-address members.
- An exact immutable Question or Blueprint Revision is always a named
  `QuestionRevisionTuple` or `BlueprintRevisionTuple` outside relational
  storage. No public or application-domain DTO carries sibling ID-plus-Revision
  fields.
- Replace generic clocks such as `editNumber`, `revisionNumber`,
  `currentRevisionNumber`, and `expectedSourceRevisionNumber` with exact names
  such as `assessmentEditNumber`, `blueprintEditNumber`,
  `draftQuestionEditNumber`, `expectedSourceBlueprintRevisionTuple`, and
  `expectedAssessmentEditNumber`.
- Add `CourseRosterId` and `CourseRosterTuple { courseInstanceId, rosterId }`.
  Recovery returns typed Course, Assessment, Attempt, Question Revision, and
  Course Roster identities while never exposing `StudentRecordId`.
- Correct `QuestionAssetTuple` to contain `questionAssetId`; every field
  holding it is `questionAssetTuple`. Apply the same rule to Object Address:
  `objectId`, `questionAssetId`, `workspaceId`, `workspaceImportId`,
  `courseBannerId`, and `draftQuestionId`.
- PostgreSQL tables and composite foreign keys retain role-qualified relational
  pairs such as `(source_blueprint_course_id,
  source_blueprint_revision_number)`. That is the intentional physical
  representation documented by `DATABASE_STYLE.md`; Rust constructs a named
  Tuple immediately at the persistence boundary.
- `ETag` and `If-Match` remain HTTP header spellings only. Domain APIs expose
  qualified domain Numbers. Direct HTTP handlers and immutable-asset cache
  headers remain valid; domain fields, errors, comments, and living
  documentation do not use ETag terminology.
- Valid exceptions are resource-root `id`, complete relationship objects named
  `source`/`target`/`left`/`right`, ordinary text such as code source,
  registered HTTP/DOM/vendor names, route syntax before immediate parsing,
  physical SQL keys, and archival historical records. Every other audit finding
  is in scope.

## Implementation

### 1. Authority and contract owner: document the model first

Update the active plan and living authorities: naming, identity, terminology,
concurrency, database-style/structure, API, contracts, design-decision, and
QTI documents.

- State the SQL physical-pair exception, Course Roster decision, root-`id`
  rule, and exact HTTP header encoding.
- Replace stale "metadata ETag" wording with the qualified Edit Number model.
- Preserve archives only as historical records, explicitly outside
  current-contract scans.

### 2. Rust and PostgreSQL owner: repair contract sources

Update `question_model` and `browser-api-contract` before regenerating
TypeScript declarations.

- Complete Blueprint Tuple flows for Course Instance creation/provenance,
  Blueprint history, fork creation, known forks, and Assessment Blueprint
  Update review/apply. Replace `source_revision_number` and
  `expected_source_revision_number` with source Blueprint Revision Tuples.
- Make known-fork results carry current and source Blueprint Revision Tuples
  rather than an ID plus generic revision numbers. Preserve `left`/`right` only
  for complete comparison-side objects; rename scalar IDs precisely.
- Repair Student Work Recovery from SQL through server serialization, including
  typed Course Roster identity and the duplicate Course Instance recovery result
  column.
- Sweep domain, data-access, server, object, and adapter code for bare Course,
  Assessment, Attempt, Entry, Banner, Draft Question, Workspace, Object,
  Question Asset, source/target, and response-item identities.
- Rename role-ambiguous SQL function columns and stored JSON keys; build Tuples
  at SQL-to-Rust boundaries and destructure only for SQL bindings.

### 3. Browser owner: migrate each transport boundary coherently

Update API contracts, decoders, HTTP clients, generated imports, routes, pages,
components, and features.

- Replace raw `string`/`number` product identities with generated types.
- Keep `BlueprintRevisionTuple` in application state. A DOM option may be
  opaque, but it resolves once into a Tuple and never becomes split application
  state.
- Restrict conditional-request utilities to HTTP parsing/formatting. Each
  caller owns typed, domain-named precondition and response helpers.
- Carry Question Asset, Draft Question, Object Address, response-item, Course
  Roster, and Student Work Recovery renames through JSON source, decoders, UI,
  and fixtures.

### 4. Fixtures and living-documentation owner: change examples with contracts

Update Rust/TypeScript fixtures, JSON examples, decoder fixtures, SQL fixture
documents, and current documentation in the same changeset. Canonical shapes
are accepted; split or ambiguous shapes have no fallback parser.

Add one changelog entry only after all gates and the adversarial review pass.

## Permanent-test policy

- Treat tests as liabilities as well as assets. Retain a permanent test only
  when it protects intentionally stable, important, externally meaningful
  behavior that could plausibly regress. Do not retain tests that merely freeze
  an implementation spelling or prior migration path.
- Use `/tests/_temp/` freely for broad `rg`/AST/database migration audits. Keep
  it outside the permanent suite and Git tracking; remove it after it has
  produced implementation evidence.
- Remove the current broad regex-oriented ETag naming pytest. It has incomplete
  exceptions and tests source spelling rather than product behavior.
- Add a small registry-backed pytest, `tests/test_semantic_contract_registry.py`,
  with a tracked `tests/semantic_contracts/registry.json`. Each entry contains:
  - an exact Human Guidance or Terminology authority citation;
  - the canonical JSON fixture;
  - the Rust serialized contract type;
  - the TypeScript decoder/client contract test;
  - the applicable PostgreSQL API function/view contract; and
  - a short statement of the intentional behavior protected.
- The registry contains only stable high-value surfaces: Course Instance
  Blueprint adoption/provenance, Blueprint fork/history/update Tuples, Student
  Work Recovery identities and Student Record non-disclosure, Question Asset
  Tuple structure, and qualified concurrency Numbers.
- The pytest verifies registry completeness and that every registered fixture
  has native Rust, TypeScript, and PostgreSQL conformance coverage. Native tests
  verify behavior:
  - Rust Serde serializes/deserializes the canonical shape.
  - TypeScript strict decoders accept canonical fixtures and reject malformed or
    split exact identities.
  - PostgreSQL integration tests verify registered API results and
    preconditions map to the same semantic contract.
- The pytest does not parse or police arbitrary local variables. The contract
  registry deliberately constrains only documented, public, stable semantics;
  changing a protected behavior requires a conscious Human Guidance and
  registry update.
- Keep a compact decoder suite for canonical shape and generic malformed-shape
  rejection. Remove one-off tests that mention retired field spellings solely
  to preserve history.

## Verification and release condition

- Regenerate browser declarations with `source ./source_me.sh && cargo tools
  tsgen`.
- Run schema documentation/style validation, focused native contract tests,
  then:

  ```sh
  source ./source_me.sh && ./launchers/run_fast_checks.sh
  source ./source_me.sh && ./launchers/all_test.sh
  ```

- Finish with an adversarial source-and-contract review. Classify every
  Id/Tuple/Number/ETag candidate under the exception rules above and fix every
  non-exception.

Completion requires zero remaining ambiguous domain identities, split
exact-revision DTOs, generic domain clocks, raw product identity types, or
non-HTTP ETag terminology.
