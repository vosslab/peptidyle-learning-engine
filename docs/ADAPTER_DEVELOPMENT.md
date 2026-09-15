# Adapter development

This guide explains how a Question Backend joins Peptidyle without changing
the shared attempt loop, gradebook, or browser trust boundary. It is for contributors adding an
adapter, not for defining a new student Question Type. The shared public contract is
[QUESTION_MODEL.md](QUESTION_MODEL.md); durable release direction and acceptance rules are in
[ROADMAP.md](ROADMAP.md) and [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md).

## Non-negotiable boundaries

- Map engine-specific input into `crates/question_model`. Downstream code reads the shared
  `QuestionRevision`, `QuestionPresentation`, and `StudentResponse` contracts rather than adapter
  types.
- Put answer keys, correct-choice bindings, and correctness logic in `crates/grading` or a
  server-only injected grading capability. The browser and WebAssembly dependency closure must not
  reach them. See [SECURITY_MODEL.md](SECURITY_MODEL.md).
- Deliver an answer-free Question Presentation for a PLE-native Question. A backend-owned Question
  instead delivers its exact attempt-bound document through the generic document route. Neither
  path puts source bytes, credentials, correct answers, or private feedback in a browser request.
- Accept an immutable, verified Source Object Reference at issue time and retain only the protected
  backend state and exact Revision evidence needed to interpret the response. A browser request
  never chooses an endpoint, source path, source bytes, Question Seed, iMathAS Profile, or renderer
  identity.
- Current records may still retain `QuestionAttemptReproductionDetails`, renderer versions, hashes,
  snapshots, or Question Grading Input. Those fields are legacy implementation evidence, not a
  universal adapter contract. A new adapter retains only the exact source/Revision relationship,
  opaque backend state, saved response, and outcome evidence its demonstrated behavior requires.
- Keep iMathAS Question Backend configuration, credentials, network policy, iMathAS Question Backend
  Session authentication state, and backend verification inside the server composition and adapter
  boundary. The browser speaks only to the same-origin PLE API.

## Declare capabilities first

Every adapter returns `QuestionBackendCapabilities` from the closed `Capability` set in
`crates/question_model/src/capability.rs`. The empty declaration is safe: an undeclared capability
is unavailable. Assessment validation compares requested behavior with that declaration before
publication, so an instructor sees every missing capability before a student starts work.

Declare only capabilities the adapter enforces for every source it accepts:

- `algorithmicGeneration` requires deterministic variants from the recorded Question Seed.
- `clientRendering` means the browser can render the safe Question Presentation; it never grants browser grading.
- `serverGrading` requires a server-held key or verified server-to-server correctness result.
- `partialCredit`, `hints`, `questionAttemptTimeLimit`, `printExport`, and `offlinePreview` require their
  own complete behavior, not a plausible future implementation.

Adding a capability expands the enum and its exhaustive consumers, then requires contract, adapter,
assignment-validation, generated-client, and browser tests. Do not add an adapter-specific boolean
or a second capability vocabulary. Retain the declaration with backend evidence only when it is
needed to interpret Student Work, and iterate it deterministically.

## Build the adapter seam

Use the following sequence for a question-agnostic adapter.

1. Store one immutable Question Source and bind it through its Source Object Reference to the
   owning Draft Question or Question Revision. Record the Question Backend separately,
   with only its exact backend-specific reference when one is required, such as a WeBWorK PG Path
   or `ImathasQuestionBackendBinding`. Keep credentials and mutable locations outside the stored
   relationship. QTI package and item references belong to Workspace Import evidence; an accepted
   QTI item becomes PLE Question JSON before this Question Source boundary.
2. At import or publication, preserve the exact source in typed object storage with its SHA-256,
   media type, Question License, Source Object Reference and Source Object Checksum, immutable
   Question Revision binding, and any required assets.
   Source archives are private and non-signable. Do not reconstruct source identity from a title or
   display label.
3. Ask the registered Question Backend to issue its answer-free presentation. A PLE-native backend
   supplies the typed Question Presentation; a backend-owned adapter persists its exact returned
   document for the selected Assessment Attempt position. The generic model never interprets a
   backend document or imposes universal Answer Key or Question Grading Input records.
4. Implement `issue` with the trusted Question ID, Question Revision Number, Source Object
   Reference, and a Question Seed only for a backend that uses one. It returns an answer-free
   `QuestionPresentation` or opaque backend document plus only the protected backend state needed
   to resume and interpret the response.
5. Implement `grade` at the server boundary. Validate the persisted issued state and use the
   minimum retained opaque backend evidence the private grader needs. A PLE-native
   backend translates its typed response through protected bindings and Question Grading Input. A
   backend-owned adapter receives only the bounded canonical ordered `[name, value]` pairs captured
   from its document, preserving duplicate names and order without interpreting controls. If a
   demonstrated backend interaction needs additional private issue-time state to interpret the
   response, retain only that opaque state rather than creating a universal snapshot or replay
   contract. Never trust a browser-provided score, session authentication state, source, Question
   Seed, or backend response fields; do not substitute current content for an existing Attempt.
6. Register the backend through the server Assessment Attempt boundary, where Course authorization,
   response saving, Assessment Attempt submission, timer policy, and persistence remain PLE
   responsibilities.

The PLE Question Backend is the small reference: it interprets the complete current PLE Question JSON
and produces the answer-free Question Presentation and server-owned evaluation behavior required by
the shared pipeline. Its supported Question Types are MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and
HOTSPOT. See
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) and
[QUESTION_MODEL.md](QUESTION_MODEL.md).

## Determinism and caching

For an algorithmic adapter, `(immutable Question Revision, Question Seed)` is the render identity. Read variable values only
from the published generation specification and the `QuestionSeed`; use ordered collections whenever
iteration affects output. A source, implementation, or behavior change creates a new published
Question Revision rather than changing historical output. The cross-target rules and Question Seed vector evidence are
in [DETERMINISM_CONTRACT.md](DETERMINISM_CONTRACT.md).

Where an adapter uses a render cache, cache only a validated browser-safe render plus the Source
Object Reference, Source Object Checksum, and Question Renderer Version that identify its source,
implementation, and Question Presentation identity. Cache keys and bytes exclude installation
identity, Answer Key data, credentials, raw backend responses, browser submissions, and upstream
session state. The current WeBWorK adapter intentionally has no render cache: it issues one exact
backend-owned document per selected Assessment position and grades it through the stateless E1 renderer path.

Native PLE Question JSON is static and receives no Question Seed. The server
uses the exact Question Revision and any authored answer-choice-order setting to
reject swapped source or presentation state.

## Browser and grading boundary

The browser renders `QuestionPresentation` blocks and validates response format
locally. It sends a typed response to PLE for saving; the server owns the
authoritative Assessment Attempt, Question Seed, source resolution, grading,
feedback disclosure, and retained evidence. `crates/grading` is intentionally
outside the Wasm closure, and `crates/server/src/*_backend.rs` are the server
bridges that repeat immutable Question Source and retained backend-evidence
validation before invoking an adapter.

Question Backends need an additional boundary. An adapter accepts deployment-selected
configuration only; it uses bounded timeouts and payloads and authenticates server-to-server. A
backend-owned document remains opaque to PLE: PLE hosts it in the generic browser component and
captures its form entries without parsing, rewriting, or classifying controls. The embed document
cannot carry a renderer credential, launch URL, callback, or score. When an upstream result is
needed, correlate and verify it with server-held attempt state before it becomes a PLE grade.

## Current adapter posture

| Adapter           | Implemented behavior                                                                                                                                                         | Current boundary and status                                                                                                                                              |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| PLE Question JSON | PLE Question JSON compilation, client rendering, and server grading for all eight runtime Question Types                                                                     | The reviewed Chapter 1 MC/MATCH publication path is live; complete visual authoring and all-type integrated acceptance remain open.                                      |
| QTI Import        | Hostile archive parsing, Canvas 1.2 and Blackboard 2.1 static single-choice profile import, private QTI Import Package Checksum evidence, and PLE Question JSON mapping      | The accepted static-import boundary deliberately supports only those profiles. Accepted items use the PLE Question Backend after conversion.                             |
| iMathAS           | Exact immutable Question Source/Revision evidence, `imathas_remote_grading_v1`-pinned iMathAS Render Cache, server-managed iMathAS Question Backend Launch, and iMathAS Result verification | The direct iMathAS Question Backend boundary is implemented. Browser-trusted launch or score flows are refused; live iMathAS Question Backend acceptance is not claimed. |
| WeBWorK           | Private standalone `/render-api` Question Backend client, exact attempt-bound backend document, generic ordered-pair capture, server-only grading, and private stateless container | WeBWorK owns PG controls and grading semantics. PLE retains the document, response, lifecycle, and outcome without projecting PGML or inferring a Question Type. |

For the exact current WeBWorK protocol, configuration ownership, and required evidence, use
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md). The author declares the
educational Question Type on the immutable Published Question Revision; PLE never derives it from
PG controls or the returned document.

## Conformance and acceptance gates

An adapter change is complete only when each applicable layer passes.

- Unit and contract tests cover source validation, capability declarations, answer-free Question
  Variation Presentations and Question Presentations, immutable issued state, refusal behavior,
  and grading outcome semantics.
- Store conformance tests cover both in-memory and PostgreSQL implementations when the adapter
  persists source, private mappings, iMathAS Question Backend Session state, assets, or attempt data.
- Tests for a backend-owned adapter prove the small boundary: exact document delivery, bounded
  ordered-pair response capture including duplicate names, server-only grading, and outcome
  recording. A one-time connected lane may establish representative renderer behavior without
  becoming a permanent content corpus.
- Live tests run against the declared disposable or private service, prove authenticated semantic
  render and grading, timeouts and outages, course isolation where relevant, and an answer-free
  PLE-only browser network trace.
- Repository gates include formatting, strict Rust checks, focused browser tests where a student
  path changes, and [E2E_TESTS.md](E2E_TESTS.md) expectations. Run the narrowest adapter command
  first, then the task gate named by the active plan.

Use the active plan's evidence wording exactly. Implemented source code, recorded fixture coverage,
and a live accepted integration are different states; do not promote one to another in code comments,
documentation, or release notes.

H5P is not a current adapter. Its future isolated runtime is blocked on the exact product decision
in [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md#future-h5p-delivery-is-a-blocked-isolated-lumi-runtime);
do not add source, import, lifecycle, or delivery seams before that decision.

## Contributor checklist

- [ ] Capability declaration is exact and Assessment publication validation refuses unsupported use.
- [ ] Published source and all assets are immutable, checksummed, private where required, and bound
      to the exact selected Question Revision.
- [ ] Issued PLE-native presentation or backend-owned document is answer-free, browser-safe, and
      bound to its exact Question Revision plus backend randomization state when applicable; a
      backend-owned document remains opaque to PLE.
- [ ] Grading runs server-side from trusted state and revalidates the Question Source plus required opaque backend state.
- [ ] iMathAS Question Backend integration has no browser endpoint, credential, launch secret, upstream state, or
      browser-trusted score.
- [ ] Conformance, recorded, and live tests are labeled by the evidence they actually supply.
