# Plan: Single-installation terminology and contract alignment

## Purpose

PLE is one installation with global Accounts. Its durable product vocabulary is
owned by [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md): Published and Draft
Questions, the global Question Library, Blueprint Courses, Course Instances,
Sysadmins, Instructors, Students, Question Stars, Question Watches, Question
Folders, and strictly automated grading.

This pre-production plan removes terminology and ownership drift directly. It
uses the current schema, source, generated contracts, tests, and durable docs
as the evidence base; it preserves no compatibility vocabulary or dual model.

## Authority and records

- `docs/HUMAN_GUIDANCE.md` owns product decisions and the glossary.
- `docs/TERMINOLOGY_CONTRACT.md` defines canonical meanings and distinctions.
- `docs/NAMING_CONVENTIONS.md` owns spelling and identifier conventions.
- `docs/VOCABULARY_REPLACEMENTS.md` is the retained correction checklist. A row
  stays visible and becomes checked only with current source and gate evidence.
- `docs/active_plans/implementation_status.md` is the sole current-package and
  migration-allocation registry.
- `docs/active_plans/implementation_plan.md` and
  `docs/active_plans/active/release_completion_plan.md` own implementation
  dependency order and release acceptance.
- `docs/DESIGN_DECISIONS.md` records settled technical interpretations.

## Completed foundation

- The schema is a fresh, domain-ordered baseline under `schemas/migrations/`.
  A clean volume applies it in order; legacy migration history is removed.
- Global `AccountId` and exact Course, Student, Authoring Workspace, Question,
  Question Revision, membership, grant, and lease relationships own
  authorization. Each decision names its exact owning relationship.
- The browser contract and Question Model use Question Library and Question
  Search vocabulary. The direct Question Summary, Question Search Result,
  Question Details, Question Statistics, and Question Search boundaries replace
  the parallel discovery model.
- The current implemented server is `server_core`: Account session handling,
  composition, health, request lifecycle, and HTTP security. Course, Question
  Library, delivery, and worker routes remain explicit downstream construction
  work rather than falsely documented available surfaces.
- Exact whole-word source searches confirm that the retired installation-scope
  and generic account-principal labels are absent from active PLE source,
  schemas, tests, and durable documentation. Remaining ordinary-language or
  platform terms are reviewed in their sentence-level context.

## Remaining work

### 1. Correct one owned boundary at a time

For each unchecked vocabulary row, identify the owning model, relation, API,
schema object, route, documentation section, or visible workflow. Replace its
meaning with the exact glossary term, updating consumers in the same change.

Prioritize boundaries that affect authorization, data ownership, stored
records, generated contracts, public routes, or visible Instructor and Student
language. Leave platform-defined and ordinary-language uses in place once
their context is verified.

### 2. Keep stored structure and contracts synchronized

When an owned stored record changes, update its schema, Rust model, generated
types, strict decoder, typed client, fixtures, and behavior tests together.
Question Revisions remain immutable pins; assignments, issued work, grading
evidence, and Question Folders bind the exact Question Revision they use.

Use Question Library for the global set of Published Questions. Use Question
Search for a request, filter, facet set, search result, or page. Use Question
Folder for an Account-owned organization of published Question references. Use
Question Star for the visible endorsement relationship and Question Watch for
the private notification subscription.

### 3. Keep documentation current

Durable docs describe only current source and implemented behavior. Plans and
historical evidence state their temporal scope explicitly. Documentation does
not promise nonexistent routes, deleted modules, unsupported roster partitions,
or retired product objects.

Update `docs/CHANGELOG.md` after a defined correction has its narrow gate and
consumer gate. Keep `docs/HUMAN_GUIDANCE.md` as owner input rather than an
implementation diary.

### 4. Verify each correction proportionately

Use Graphify for targeted impact tracing before broad exploration, then verify
the current source and tests. Run the owning unit or behavior gate, required
consumer gate, TypeScript or Rust compilation where applicable, Markdown links
for documentation changes, and `git diff --check`.

Classify real-stack, browser, database, and one-time evidence separately from
permanent deterministic tests according to `docs/TEST_EVIDENCE_MODEL.md` and
`docs/PYTEST_STYLE.md`.

### 5. Close only with current evidence

The final audit checks every retained vocabulary row in context. It confirms
that source, schemas, generated contracts, tests, durable docs, and active
plans use the canonical term at every PLE-owned boundary. It also runs the
required aggregate gates from the active release plan and records unrun
human-only acceptance separately.

## Non-goals

- Preserve a retired product term as a compatibility alias, bridge, or
  alternate persisted shape.
- Add product capabilities beyond the owner guidance.
- Treat a raw string count as proof without inspecting each remaining context.
- Add permanent tests that merely count words rather than protect user-visible
  behavior, authorization, data ownership, or a durable contract.
