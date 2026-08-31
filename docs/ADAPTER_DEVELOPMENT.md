# Adapter development

This guide explains how an external or first-party question engine joins Peptidyle without changing
the shared attempt loop, gradebook, or browser trust boundary. It is for contributors adding an
adapter, not for defining a new student Question Type. The shared public contract is
[QUESTION_MODEL.md](QUESTION_MODEL.md); authoritative release scope and acceptance state are in
[release_completion_plan.md](active_plans/active/release_completion_plan.md).

## Non-negotiable boundaries

- Map engine-specific input into `crates/question_model`. Downstream code reads the shared
  `QuestionVersion`, `QuestionPresentation`, and `StudentResponse` contracts rather than adapter
  types.
- Put answer keys, correct-choice bindings, and correctness logic in `crates/grading` or a
  server-only injected grading capability. The browser and WebAssembly dependency closure must not
  reach them. See [SECURITY_MODEL.md](SECURITY_MODEL.md).
- Deliver only an answer-free envelope: title, prompt blocks, Question Response Format, immutable
  version, and seed. Do not put source bytes, credentials, upstream session state, correct answers,
  or private feedback in an envelope, browser cache, or browser request.
- Accept an immutable, verified Source Object Reference at issue time and retain the protected attempt
  provenance required for grade. A browser request never
  chooses an endpoint, source path, source bytes, seed, provider profile, or renderer identity.
- Record source object and checksum, Question Backend/Grader/Renderer releases, parameter
  hash, rendered-envelope hash, and bound assets in `QuestionAttemptReproductionDetails`. Presentation-bearing
  attempts also persist a checksummed public snapshot and server-only grading envelope; missing or
  mismatched state makes grade unavailable rather than reissuing.
- Keep provider configuration, credentials, network policy, correlation state, and upstream
  verification inside the server composition and adapter boundary. The browser speaks only to the
  same-origin PLE API.

## Declare capabilities first

Every adapter returns `QuestionBackendCapabilities` from the closed `Capability` set in
`crates/question_model/src/capability.rs`. The empty declaration is safe: an undeclared capability
is unavailable. Assignment validation compares requested behavior with that declaration before
publication, so an instructor sees every missing capability before a student starts work.

Declare only capabilities the adapter enforces for every source it accepts:

- `algorithmicGeneration` requires deterministic variants from the recorded seed.
- `clientRendering` means the browser can render the safe envelope; it never grants browser grading.
- `serverGrading` requires a server-held key or verified server-to-server correctness result.
- `partialCredit`, `hints`, `questionAttemptTimeLimit`, `printExport`, and `offlinePreview` require their
  own complete behavior, not a plausible future implementation.

Adding a capability expands the enum and its exhaustive consumers, then requires contract, adapter,
assignment-validation, generated-client, and browser tests. Do not add an adapter-specific boolean
or a second capability vocabulary. The capability declaration is serialized in reproducibility
records and must iterate deterministically.

## Build the adapter seam

Use the following sequence for a question-agnostic adapter.

1. Define or use a `QuestionSource` / `DraftQuestionSource` variant that identifies the engine
   without embedding credentials or mutable locations. Keep a draft locator private until trusted
   server work snapshots and validates it.
2. At import or publication, preserve the exact source in typed object storage with its SHA-256,
   media type, license, provenance, immutable problem/version binding, and any required assets.
   Source archives are private and non-signable. Do not reconstruct source identity from a title or
   display label.
3. Compile the source to a key-free `QuestionVersion`. Keep an answer-bearing compilation product
   in private grading storage, or retain an immutable source that only server-side grading can read.
4. Implement `issue` with the trusted problem/version/source/seed inputs. It returns an answer-free
   `QuestionPresentation`, a parameter hash, and complete `QuestionAttemptReproductionDetails`.
5. Implement `grade` at the server boundary. Validate the persisted issued snapshot, translate
   public rendered IDs through the protected grading envelope, and use retained immutable source
   provenance where a private grader needs it. A family that needs private first-grade material
   persists a typed, checksummed issue-time contract and consumes that contract rather than a
   current published Question definition, grader, or renderer. Never trust browser-provided score,
   correlation, source, seed, or upstream response fields; do not rerender a receipt-era attempt.
6. Register the backend through the server run boundary, where course authorization, attempt
   identity, idempotency, timer policy, and persistence remain PLE responsibilities.

The native flat adapter is the small reference: it compiles answer-bearing PLE flat-question JSON v2
into a public definition and separate private material. Its closed v2 `singleChoice` Question Type is one of
the eight native response types; new semantics require their own reviewed contract rather than an
ad hoc adapter widening. See
[QTI-JSON_OBJECT_FORMAT.md](QTI-JSON_OBJECT_FORMAT.md) and
[implementation_plan.md](active_plans/implementation_plan.md).

## Determinism and caching

For a seeded adapter, `(immutable version, seed)` is the render identity. Read variable values only
from the published generation specification and the `QuestionSeed`; use ordered collections whenever
iteration affects output. A source, implementation, or behavior change creates a new published
version rather than changing historical output. The cross-target rules and seed-vector evidence are
in [DETERMINISM_CONTRACT.md](DETERMINISM_CONTRACT.md).

Render caches are immutable, shared-content artifacts keyed by version and seed. Cache only a
validated browser-safe render plus enough provenance to prove its source, implementation, and
envelope identity. Cache keys and bytes must exclude installation identity, answer material, credentials,
raw provider responses, browser submissions, and upstream session state. If stateless replicas race
to write the same key, reload and validate the winning immutable record.

Static questions still record a seed and a deterministic parameter hash. This makes the attempt
record uniform and lets reproduction reject swapped version/source/Question Attempt Reproduction Details.

## Browser and grading boundary

The browser renders `QuestionPresentation` blocks and validates response format locally. It submits a
typed response to PLE; the server owns the authoritative attempt, seed, source resolution, grading,
feedback release, and receipt. `crates/grading` is intentionally outside the Wasm closure, and
`crates/server/src/*_backend.rs` are the server bridges that repeat immutable-source and provenance
validation before invoking an adapter.

External engines need an additional boundary. A provider client accepts deployment-selected
configuration only; it must use bounded timeouts and payloads, authenticate server-to-server, and
convert untrusted output into safe prompt blocks before caching or delivery. Browser-visible embeds
cannot carry a launch URL, token, callback, score, iframe markup, or answer. When an upstream result
is needed, correlate and verify it with server-held attempt state before it becomes a PLE grade.

## Current adapter posture

| Adapter     | Implemented behavior                                                                                                                                       | Current boundary and status                                                                                                                                                             |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Native flat | PLE flat JSON v2 compilation, client rendering, and server grading for all eight runtime Question Types                                                    | The reviewed Chapter 1 MC/MATCH publication path is live; complete visual authoring and all-type integrated acceptance remain under WP-RC5.                                             |
| QTI         | Hostile archive parsing, Canvas 1.2 and Blackboard 2.1 static single-choice profile import, private provenance, native conversion, and server-only grading | WP-QTI-1 through WP-QTI-12 are accepted. Profile breadth remains deliberately bounded.                                                                                                  |
| H5P         | Supported static multiple-choice import into an answer-free internal question                                                                              | Native H5P declares only `clientRendering` and is ungraded practice. Server-graded H5P is not supported; WP-RC6 owns protected-native conversion and the complete capability close-out. |
| iMathAS     | Immutable server snapshot, profile-pinned safe render cache, server-brokered verified-result design, and contracted backend                                | Implemented contracted boundary. Generic hosted execution and browser-trusted launch/score flows are refused; live provider acceptance is not claimed.                                  |
| WeBWorK     | External standalone `/render-api` client, bounded PGML projection, server-only grading, sanitized immutable render cache, and private stateless container  | The four reviewed Chapter 1 MC/MATCH sources passed live renderer and browser acceptance. Other PG controls or source revisions require their own evidence.                             |

For the exact current WeBWorK protocol, the supported control shape, configuration ownership, and
required evidence, use [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md). Do
not generalize its reviewed Chapter 1 profile into broad OPL or generic PG support.

## Conformance and acceptance gates

An adapter change is complete only when each applicable layer passes.

- Unit and contract tests cover source validation, capability declarations, key-free envelopes,
  deterministic issue/replay, cache validation, provenance tampering, refusal behavior, and grading
  outcome semantics.
- Store conformance tests cover both in-memory and PostgreSQL implementations when the adapter
  persists source, private mappings, external-tool state, assets, or attempt data.
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
- [ ] Issued envelope and cache are answer-free, browser-safe, deterministic, and version/seed bound.
- [ ] Grading runs server-side from trusted state and revalidates source plus issued provenance.
- [ ] External integration has no browser endpoint, credential, launch secret, upstream state, or
      browser-trusted score.
- [ ] Conformance, recorded, and live tests are labeled by the evidence they actually supply.
