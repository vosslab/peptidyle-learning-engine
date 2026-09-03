# Adapter development

This guide explains how a Question Backend joins Peptidyle without changing
the shared attempt loop, gradebook, or browser trust boundary. It is for contributors adding an
adapter, not for defining a new student Question Type. The shared public contract is
[QUESTION_MODEL.md](QUESTION_MODEL.md); authoritative release scope and acceptance state are in
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Non-negotiable boundaries

- Map engine-specific input into `crates/question_model`. Downstream code reads the shared
  `QuestionRevision`, `QuestionPresentation`, and `StudentResponse` contracts rather than adapter
  types.
- Put answer keys, correct-choice bindings, and correctness logic in `crates/grading` or a
  server-only injected grading capability. The browser and WebAssembly dependency closure must not
  reach them. See [SECURITY_MODEL.md](SECURITY_MODEL.md).
- Deliver only an answer-free Question Presentation: title, prompt blocks, Question Response Format, immutable
  version, and seed. Do not put source bytes, credentials, upstream session state, correct answers,
  or private feedback in a Question Presentation, browser cache, or browser request.
- Accept an immutable, verified Source Object Reference at issue time and retain the protected attempt
  Question Attempt Reproduction Details required for grade. A browser request never
  chooses an endpoint, source path, source bytes, seed, iMathAS Profile, or renderer identity.
- Record Source Object Reference and Source Object Checksum, Question Backend/Grader/Renderer versions, parameter
  hash, Rendered Question SHA-256, and bound assets in `QuestionAttemptReproductionDetails`. Presentation-bearing
  attempts also persist a checksummed public snapshot and server-only Question Grading Input; missing or
  mismatched state makes grade unavailable rather than reissuing.
- Keep iMathAS Question Backend configuration, credentials, network policy, iMathAS Question Backend Session
  authentication state, and backend verification inside the server composition and adapter boundary. The browser speaks only to the
  same-origin PLE API.

## Declare capabilities first

Every adapter returns `QuestionBackendCapabilities` from the closed `Capability` set in
`crates/question_model/src/capability.rs`. The empty declaration is safe: an undeclared capability
is unavailable. Assignment validation compares requested behavior with that declaration before
publication, so an instructor sees every missing capability before a student starts work.

Declare only capabilities the adapter enforces for every source it accepts:

- `algorithmicGeneration` requires deterministic variants from the recorded seed.
- `clientRendering` means the browser can render the safe Question Presentation; it never grants browser grading.
- `serverGrading` requires a server-held key or verified server-to-server correctness result.
- `partialCredit`, `hints`, `questionAttemptTimeLimit`, `printExport`, and `offlinePreview` require their
  own complete behavior, not a plausible future implementation.

Adding a capability expands the enum and its exhaustive consumers, then requires contract, adapter,
assignment-validation, generated-client, and browser tests. Do not add an adapter-specific boolean
or a second capability vocabulary. The capability declaration is serialized in reproducibility
records and must iterate deterministically.

## Build the adapter seam

Use the following sequence for a question-agnostic adapter.

1. Store one immutable Question Source and bind it through its Source Object Reference to the
   owning Draft Question or Question Revision. Record the Question Backend separately,
   with only its exact backend-specific reference when one is required, such as a WeBWorK PG Path
   or `ImathasQuestionBackendBinding`. Keep credentials and mutable locations outside the stored
   relationship. QTI package and item references belong to Workspace Import evidence; an accepted
   QTI item becomes PLE Question JSON before this Question Source boundary.
2. At import or publication, preserve the exact source in typed object storage with its SHA-256,
   media type, Question License, Source Object Reference and Source Object Checksum, immutable Question Revision binding, and any required assets.
   Source archives are private and non-signable. Do not reconstruct source identity from a title or
   display label.
3. Ask the registered Question Backend to interpret the complete source and produce an answer-free
   Question Presentation. The generic Question model stores and routes the complete source without
   imposing universal Answer Key or Question Grading Input records.
4. Implement `issue` with the trusted Question ID, Question Revision Number, Source Object Reference,
   and Question Seed inputs. It returns an answer-free
   `QuestionPresentation`, a parameter hash, and complete `QuestionAttemptReproductionDetails`.
5. Implement `grade` at the server boundary. Validate the persisted issued snapshot, translate
   public Presentation Response Item References through the protected Response Item Bindings and Question Grading Input, and use retained immutable source
   Question Attempt Reproduction Details where a private grader needs them. A Question Backend that needs a private first-grade contract
   persists a typed, checksummed issue-time contract and consumes that contract rather than a
   current published Question Revision, grader, or renderer. Never trust browser-provided score,
   iMathAS Session Authentication state, source, seed, or backend response fields; do not rerender a receipt-era attempt.
6. Register the backend through the server Assignment Attempt boundary, where course authorization, attempt
   identity, idempotency, timer policy, and persistence remain PLE responsibilities.

The PLE Question Backend is the small reference: it interprets complete PLE Question JSON version 3
and produces the answer-free Question Presentation and server-owned evaluation behavior required by
the shared pipeline. Its supported Question Types are MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and
HOTSPOT. See
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) and
[implementation_plan.md](active_plans/implementation_plan.md).

## Determinism and caching

For a seeded adapter, `(immutable version, seed)` is the render identity. Read variable values only
from the published generation specification and the `QuestionSeed`; use ordered collections whenever
iteration affects output. A source, implementation, or behavior change creates a new published
version rather than changing historical output. The cross-target rules and seed-vector evidence are
in [DETERMINISM_CONTRACT.md](DETERMINISM_CONTRACT.md).

Render caches are immutable, shared-content artifacts keyed by version and seed. Cache only a
validated browser-safe render plus the Source Object Reference, Source Object Checksum, and Question Renderer Version that identify its source, implementation, and
Question Presentation identity. Cache keys and bytes must exclude installation identity, Answer Key data, credentials,
raw backend responses, browser submissions, and upstream session state. If stateless replicas race
to write the same key, reload and validate the winning immutable record.

Static questions still record a seed and a deterministic parameter hash. This makes the attempt
record uniform and lets reproduction reject swapped version/source/Question Attempt Reproduction Details.

## Browser and grading boundary

The browser renders `QuestionPresentation` blocks and validates response format locally. It submits a
typed response to PLE; the server owns the authoritative attempt, seed, source resolution, grading,
feedback release, and receipt. `crates/grading` is intentionally outside the Wasm closure, and
`crates/server/src/*_backend.rs` are the server bridges that repeat immutable Question Source and Question Attempt Reproduction Details
validation before invoking an adapter.

Question Backends need an additional boundary. An adapter accepts deployment-selected
configuration only; it must use bounded timeouts and payloads, authenticate server-to-server, and
convert untrusted output into safe prompt blocks before caching or delivery. Browser-visible embeds
cannot carry a launch URL, token, callback, score, iframe markup, or answer. When an upstream result
is needed, correlate and verify it with server-held attempt state before it becomes a PLE grade.

## Current adapter posture

| Adapter           | Implemented behavior                                                                                                                                                              | Current boundary and status                                                                                                                                                  |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PLE Question JSON | PLE Question JSON compilation, client rendering, and server grading for all eight runtime Question Types                                                                          | The reviewed Chapter 1 MC/MATCH publication path is live; complete visual authoring and all-type integrated acceptance remain open.                                          |
| QTI Import        | Hostile archive parsing, Canvas 1.2 and Blackboard 2.1 static single-choice profile import, private QTI Import Package Checksum evidence, and PLE Question JSON mapping          | The accepted static-import boundary deliberately supports only those profiles. Accepted items use the PLE Question Backend after conversion.                                |
| H5P               | Supported H5P Package parsing and current ungraded-practice behavior                                                                                                               | H5P retains its distinct package source behind the shared Draft Question, publication, Assignment, and delivery operations as integration advances.                         |
| iMathAS           | Immutable Question Source snapshot, `imathas_remote_grading_v1`-pinned iMathAS Render Cache, server-managed iMathAS Question Backend Launch, and iMathAS Result verification      | The direct iMathAS Question Backend boundary is implemented. Browser-trusted launch or score flows are refused; live iMathAS Question Backend acceptance is not claimed.     |
| WeBWorK           | Private standalone `/render-api` Question Backend client, bounded PGML projection, server-only grading, sanitized immutable render cache, and private stateless container         | The four reviewed Chapter 1 MC/MATCH sources passed live renderer and browser acceptance. Other PG controls or source revisions require their own evidence.                  |

For the exact current WeBWorK protocol, the supported control shape, configuration ownership, and
required evidence, use [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md). Do
not generalize its reviewed Chapter 1 profile into broad OPL or generic PG support.

## Conformance and acceptance gates

An adapter change is complete only when each applicable layer passes.

- Unit and contract tests cover source validation, capability declarations, answer-free Question
  Variation Presentations and Question Presentations,
  deterministic issue/replay, Question Attempt Reproduction Details tampering, refusal behavior, and grading
  outcome semantics.
- Store conformance tests cover both in-memory and PostgreSQL implementations when the adapter
  persists source, private mappings, iMathAS Question Backend Session state, assets, or attempt data.
- Recorded tests use redacted, fixed upstream fixtures to verify request/response parsing and
  projection without claiming that an upstream service was exercised. WeBWorK RC3 recorded checks
  do not replace its live gate.
- Live tests run against the declared disposable or private service, prove authenticated semantic
  render and correct/incorrect grading, repeat/cache behavior, timeouts and outages, course
  isolation where relevant, and an answer-free PLE-only browser network trace.
- Repository gates include formatting, strict Rust checks, focused browser tests where a student
  path changes, and [E2E_TESTS.md](E2E_TESTS.md) expectations. Run the narrowest adapter command
  first, then the task gate named by the active plan.

Use the active plan's evidence wording exactly. Implemented source code, recorded fixture coverage,
and a live accepted integration are different states; do not promote one to another in code comments,
documentation, or release notes.

## Contributor checklist

- [ ] Capability declaration is exact and assignment validation refuses unsupported use.
- [ ] Published source and all assets are immutable, checksummed, private where required, and carried
      in Question Attempt Reproduction Details.
- [ ] Issued Question Presentation and cache are answer-free, browser-safe, deterministic, and version/seed bound.
- [ ] Grading runs server-side from trusted state and revalidates the Question Source plus issued Question Attempt Reproduction Details.
- [ ] iMathAS Question Backend integration has no browser endpoint, credential, launch secret, upstream state, or
      browser-trusted score.
- [ ] Conformance, recorded, and live tests are labeled by the evidence they actually supply.
