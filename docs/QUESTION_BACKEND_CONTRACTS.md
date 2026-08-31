# Question backend contracts

This document records the durable execution contract at PLE's question-backend boundary. It is a
reader's map of the implemented system, not a replacement for the active implementation plan.
The plan and its active release plan remain authoritative for dependency order and acceptance.

PLE is question agnostic at the learning-engine boundary. An Issued Question preserves one exact
published Question Version, and a Question Attempt uses the common `RunBackend` contract. A
backend safely issues, reproduces, and grades its own material; PLE owns Account and exact
course/Student authorization, assignment policy, attempt identity, timing, idempotency,
gradebook persistence, retention, and the
browser API.

Read this with [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) for contributor workflow,
[SECURITY_MODEL.md](SECURITY_MODEL.md) for answer-bearing boundaries,
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) for the exact private RPC,
and [RELATED_PROJECTS.md](RELATED_PROJECTS.md) for ecosystem scope and comparison sources.

## Status words

- **Current** means code and an explicit contract exist in this checkout.
- **Accepted** means the active release evidence accepts that bounded path.
- **Configured** means composition must deliberately install the backend and protected dependencies.
- **Planned** means it is deliberately outside the current contract, not an implied feature.

## Shared engine contract

All installed backends enter the server through `crates/server/src/run/contracts.rs`.

| Concern          | Common PLE rule                                                                                                                                                                                                  |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source authority | A published `QuestionDefinition` and immutable `QuestionVersionReference` select the backend. A browser does not select a backend, source path, source bytes, seed, renderer, or provider.                              |
| Issuance         | `RunBackend::issue` receives trusted `AuthenticatedSession`, exact course/Student relationship, published reference, definition, and server-owned seed. It returns a key-free envelope, parameter hash, and provenance. |
| Reproduction     | `RunBackend::reproduce` is limited to issue-time work and explicit envelope-less active families. Presentation-bearing first submit and submitted delivery validate the owned snapshot/private envelope instead. |
| Response         | The browser submits `StudentResponse` to a PLE same-origin attempt route with an idempotency key. It never submits a score, provider correlation, source identity, renderer field, or answer key.                |
| Grade            | `RunBackend::grade` returns a server-side outcome. The common route owns policy-aware persistence; an external-tool backend may atomically commit a verified broker result.                                      |
| Provenance       | `AttemptProvenance` records adapter, optional renderer/generator, source artifact, bound assets, grader, and rendered-question SHA-256.                                                                          |
| Failure          | A backend reports `Unsupported`, `Invalid`, or `Unavailable`. An unavailable renderer or provider is not converted into a student incorrect response.                                                            |
| Capabilities     | `BackendCapabilities` is a closed declaration. Publication validation refuses an assignment requiring a capability the selected backend did not declare.                                                         |

The browser-safe `QuestionEnvelope` contains a public response shape and student presentation, never
an answer key. Its render `kind` selects the browser widget. The planned compact response wire drops
the redundant response `kind`, because the authoritative attempt already selects the response schema.
See [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for current and target payloads.

## Backend comparison

| Backend              | Current authority                                                   | Browser response                                           | Server grading authority                            | Current scope                                                                                                                            |
| -------------------- | ------------------------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Native flat          | Immutable PLE flat source and private flat grading payload          | Typed PLE response                                         | Native adapter plus isolated flat grader            | All eight PLE flat JSON v2 Question Types; protected visual author editor; end-to-end all-type and hotspot lifecycle acceptance remains open |
| QTI profile          | Immutable staged/published archive plus profile conversion evidence | Typed PLE response                                         | `QtiBackend` plus least-privilege `QtiGradingStore` | Canvas 1.2 and Blackboard 2.1 static single-choice profiles                                                                              |
| WeBWorK              | Immutable licensed PGML source and private renderer                 | Opaque PLE choice or match IDs                             | Private external `/render-api`                      | Four reviewed Chapter 1 PGML sources: MC plus MATCH per chapter; exact-source matching partial credit                                    |
| iMathAS              | Immutable server snapshot and deployment-selected provider profile  | `ExternalTool` marker through protected same-origin routes | Server broker and verified provider result          | Explicitly configured contracted scored-embed provider only                                                                              |
| External-tool broker | Exact course/Student attempt, launch session, and protected exchange row | `ExternalTool {}` marker plus HttpOnly launch proof | Backend-owned atomic verified-result commit | Shared mechanism used by the contracted iMathAS path |

## Native flat questions

### Source and render

**Current.** The native adapter compiles versioned PLE flat JSON into two products: an answer-free
`QuestionDefinition`/`QuestionEnvelope` and private grading material. The trusted server bridge
resolves immutable catalog asset bindings before issue, replay, or grade. The browser receives prompt
blocks, public response definition, asset references, version, and seed. It returns only the PLE
response shape; it does not return source bytes, a private key, asset-object binding, implementation
version, or a scoring decision.

The current closed source contract supports multiple choice, multiple answer, fill-in-the-blank,
multiple choice, multiple answer, fill-in-the-blank, multi-blank, numerical, matching, ordering, and hotspot questions. The native adapter dispatches by
registered Native Question Implementation for the explicit Question Format, Question Type, and optional Question Generator rather than making the run model type-specific. The protected visual author
editor exposes all eight v2 Question Types. Its instructor route is a convenience surface only: the server
re-resolves source and asset bindings at save and publication, and the student contract remains
answer-free. Integrated author-to-publication-to-student acceptance for every family, including the
hotspot lifecycle, remains open.

### Grade, replay, and cache

The server validates immutable reference, seed, parameter hash, rendered-question hash, and asset
bindings before generic grading or isolated flat grading. First flat grade reads only the issued
checksummed flat grading contract; ordinary catalog and browser paths cannot read that material or
replace it with a current grader view. Provenance names the native adapter and grader, optional
generator, bound objects, and rendered output hash.

Native generation is deterministic for published version and seed. A shared cache may contain only
answer-free generated output keyed by that identity. Course/Student state, keys, submissions, and feedback
never enter it. Static questions still use the uniform seed and parameter-hash record so swapped
provenance is detectable.

### Capabilities and extension

Native capabilities are the intersection declared by selected registered implementations. A new Question Implementation must
add a closed source/parser/compiler contract, browser-safe response definition, server-only key or
rubric, deterministic issue/reproduction, capability declaration, strict response validation, and
conformance coverage. It must not add a parallel run loop or browser grader.

## QTI profile questions

### Source and render

**Current.** QTI is an import and private-grading path, not a browser QTI runtime. An authorized
author uploads a bounded archive to private workspace object storage. The worker parses a narrow,
hostile-input profile and records safe reports, checksums, normalized item facts, asset bindings, and
server-only grading handoff. Publication atomically pins archive and item identity.

At issue or replay, `QtiBackend` rereads the exact published archive, verifies object identity and
SHA-256, reparses it, checks the selected item against the durable public definition, resolves
immutable assets, and returns a normal answer-free PLE envelope. The student submits the same typed
PLE response as for native questions. Student JSON has no QTI XML, archive object key, import ID, or
answer binding.

### Grade, provenance, and scope

`QtiBackend` obtains answer-bearing material only through separately injected, least-privilege
`QtiGradingStore`. The normal catalog/object store resolves public archive and asset evidence but
cannot recover correct responses. Issue, replay, and grade fail closed if archive, checksum, item,
asset mapping, or private binding no longer reproduces. Provenance records private-profile adapter,
source artifact, bound assets, QTI private grader, and rendered envelope hash.

When explicitly configured, QTI declares `serverGrading`. Current accepted import profiles are static
single-choice Canvas QTI 1.2 and Blackboard Original QTI 2.1 pools. Other XML, interaction types,
embedded execution, and broad QTI interchange are refused rather than partially interpreted.

**Planned.** Broader QTI families and external QTI-JSONL interchange require new profile decisions,
conversion semantics, private key handling, and independent live acceptance. Flat JSON v2 alone does
not enable them.

## WeBWorK private renderer

### Source and render

**Accepted bounded path.** PLE is the only WebWork client. A published problem resolves to immutable,
licensed, user-authored PGML source and a fixed seed. The API sends server-owned form data to a
private external standalone `/render-api` service. The browser receives only a PLE envelope,
sanitized prompt markup, and opaque PLE choice IDs. It never receives PG source, file path, renderer
URL, credentials, upstream hidden fields, cookies, session key, radio name, or radio value.

The accepted projection covers the exact reviewed Chapter 1 `RadioButtons` and matching shapes. PLE
removes upstream controls from prompt markup and emits opaque IDs per projected label or matching
side. A student submits PLE IDs to PLE, not upstream form fields.

### Grade, replay, cache, and failure

For a newly issued attempt, PLE resolves immutable source, captures and validates the private
field/value mapping, converts durable choice identities to presentation-scoped rendered IDs, and
persists that mapping with the exact public snapshot, private grading envelope, and frozen WeBWorK
definition. Normal grade reloads those validated artifacts, maps the student's rendered ID through
the private envelope, and makes one private grade request. It does not reconstruct an issuance
render or resolve a current catalog definition. The mapping never
appears in an envelope, safe cache, receipt, log event, or browser response.

The shared immutable cache is keyed by version and seed. It holds only sanitized answer-free
envelope/markup, source-artifact binding, renderer identity, and rendered-output checksum. A cache hit
for a new issuance still performs the bounded private render needed to create that attempt's replay
mapping; reproduction and normal grade do not. Telemetry uses only `renderer_call` and `cache_hit`
event names.

The renderer accepts only expected form/JSON shapes, bounded bodies, fixed origins, known protected
echo fields, and sanitized markup. Redirects, malformed or duplicate JSON, unknown fields,
protected-field mismatches, unsafe HTML, unsupported controls, and timeouts refuse the operation; they
do not produce a grade or leak secrets.

### Capabilities and scope

The configured backend declares `algorithmicGeneration` and `serverGrading`. The reviewed matching
sources additionally declare `partialCredit` only when both their source path and immutable digest
match the evidence profile. Other PG sources remain all-or-nothing.

**Planned.** Generic PG controls, unreviewed matching sources, broad OPL compatibility, browser
access to WebWork, and upstream gradebook/LTI passback remain outside the accepted scope. Any added
control needs its own projection, private replay mapping, response contract, browser interaction,
and acceptance evidence.
The detailed protocol is in [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

## iMathAS broker

### Source, render, and launch

**Current contracted boundary.** iMathAS begins as a server-authorized draft locator with opaque,
deployment-configured provider key and provider-local item reference. Before publication, the server
pins immutable source snapshot, SHA-256, and integration profile. A published definition stores no
endpoint, credential, launch material, or mutable provider location.

The adapter creates a safe render from that snapshot and caches only answer-free public rendering by
immutable identity. The student response is not provider data: it is PLE's `ExternalTool {}` marker.
The external activity loads only from PLE's protected same-origin launch route.

### Verified result and persistence

PLE creates a short-lived exact course/Student/attempt-bound launch session only after reproducing the
issued attempt, through nested
`POST /api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch`.
The exact typed route binding is broker-verified before provider work and enters neither JSON nor
provider data. The browser receives an HttpOnly, Secure, SameSite=Strict cookie scoped to launch.
The corresponding GET is an inert same-origin shell: it cannot create or renew a session or disclose
a provider URL. The shell uses a sandboxed iframe and constrained message bridge. Provider state is
AEAD-wrapped before protected store persistence and is never a JSON field.

The server creates a broker binding over the authenticated Account, exact course/Student attempt, problem, version, seed, immutable source,
profile, and canonical marker response. Before an effectful provider POST it durably records an
indeterminate-dispatch marker under the active, unexpired lease and exact launch-token hash. A
crash or ambiguous transport result leaves the attempt fenced rather than retrying an action that
may have reached the provider; claim, new launch, grade, finalization, and revocation refuse until
the operator-safe resolution path decides the state. A valid provider result is verified against the
binding and atomically clears the marker while persisting the first verified result under the
idempotency key. Replay returns the committed record, not another provider call. Grade retrieval is
structurally GET-only and side-effect free; it cannot be substituted for an effectful provider POST.

Provider results are intentionally non-serializable. Correlation, provider state, launch proof, and
lease token redact debug output. Timeout, authentication failure, malformed provider response, bad
correlation, or verification mismatch is unavailable/invalid, never an incorrect student result.

### Capability and scope

The configured provider declares `algorithmicGeneration`, `serverGrading`, and `partialCredit`.
Profile, transport, provider identity, and verifier belong to deployment composition, not authors or
students.

**Planned or refused.** Generic hosted MyOpenMath/iMathAS execution, arbitrary endpoints,
browser-trusted launch URLs or scores, and unverified provider callbacks are refused. Live provider
acceptance is not implied by the implemented broker boundary and recorded fixtures.

## External-tool broker path

The external-tool transaction is reusable server-side machinery, currently exercised by contracted
iMathAS. It allows an external activity UI without handing it PLE grading authority.

| Stage     | Current contract                                                                                                                                 |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Authorize | Authenticate, load an RLS-visible attempt, and prove ownership of its run before launch, proxy, or submission work.                              |
| Bind      | `ExternalToolBinding` covers provider, problem/version, seed, immutable source checksum, profile, and SHA-256 of canonical `ExternalTool {}`.    |
| Launch    | POST creates a server-owned session with opaque random token; GET is only an inert shell. A protected cookie is required for activity and submit. |
| Proxy     | Browser calls same-origin PLE activity route. Only the sandboxed activity POST may carry `Origin: null`, and it must also present the launch cookie and AEAD-bound context. PLE alone contacts the provider using encrypted server-held state. |
| Lease     | Broker returns a committed replay, verified-pending result, in-progress state, or one unexpired lease holder. A pre-dispatch indeterminate marker fences an ambiguous provider POST, so concurrent retries cannot duplicate grading. |
| Verify    | Backend accepts only a server-verified result matching the authenticated Account, exact course/Student attempt, problem, version, seed, and correlation. |
| Commit    | Backend atomically commits verified grade and receipt; PLE then applies disclosure and gradebook policy.                                         |

The marker does not mean "trust the external tool." It means the current attempt requires the
server-held launch and verification protocol. A new external backend may use this transaction only
with immutable source identity, protected launch state, authenticated result verification, bounded
proxy policy, replay/idempotency semantics, and a closed capability declaration.

## Extension rules

1. Define durable published and private draft source identity without secrets or mutable endpoints.
2. Pin source bytes, checksum, license, provenance, implementation/profile facts, and assets at publication.
3. Issue an answer-free envelope; keep keys, rubrics, mappings, credentials, correlation, and raw results server-only.
4. At issue, capture the exact version/seed render, compare complete provenance, and persist its
   answer-free public snapshot plus server-only grading envelope. Retry, submitted delivery, and
   grade validate those artifacts rather than rerendering.
5. Choose one grading authority: private PLE material, private renderer, or verified external result.
6. Cache only immutable answer-free render output. Bind private replay state to the exact course/Student attempt, never shared cache.
7. Declare only implemented capabilities, and make assignment validation refuse unsupported policy before issue.
8. Add deterministic conformance tests. Label recorded provider fixtures separately from live service acceptance.

## Contract locations

| Contract                    | Primary locations                                                                                                                           |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Shared model and provenance | `crates/question_model/src/definition.rs`, `activity.rs`, `response.rs`, and `capability.rs`                                                |
| Common run seam             | `crates/server/src/run/contracts.rs` and `crates/server/src/composite_backend.rs`                                                           |
| Native flat                 | `crates/adapters/native`, `crates/server/src/native_backend.rs`, and `crates/learning-data-access/src/flat_question.rs`                     |
| QTI private grading         | `crates/adapters/qti`, `crates/server/src/qti_backend.rs`, and `crates/learning-data-access/src/qti.rs`                                     |
| WeBWorK renderer            | `crates/adapters/webwork`, `crates/server/src/webwork_backend.rs`, and [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) |
| iMathAS broker              | `crates/adapters/imathas`, `crates/server/src/imathas_backend.rs`, and `crates/server/src/run/external_tool.rs`                             |
| Student payload design      | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                                                                                |
| Security and storage        | [SECURITY_MODEL.md](SECURITY_MODEL.md), [OBJECT_STORAGE.md](OBJECT_STORAGE.md), and [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#typed-operations-and-objects) |
