# Question backend contracts

This document records the durable execution contract at PLE's question-backend boundary. It is a
reader's map of the implemented system, not a replacement for the active implementation plan.
The plan and its active release plan remain authoritative for dependency order and acceptance.

PLE uses the same lifecycle for every Question Backend: Draft Question authoring, publication,
Assignment selection, issuance, presentation, response save, finalization, outcome recording,
feedback release, and Gradebook effects. Each operation resolves the exact Published Question
Revision and registered Question Backend, then delegates backend-owned rendering, interaction,
response interpretation, and evaluation to that backend. PLE owns Account and exact
course/Student authorization, Assignment policy, Question Attempt lifecycle and timing, durable
evidence, Gradebook persistence, retention, and the same-origin browser API.

Read this with [ADAPTER_DEVELOPMENT.md](ADAPTER_DEVELOPMENT.md) for contributor workflow,
[SECURITY_MODEL.md](SECURITY_MODEL.md) for answer-bearing boundaries,
[WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md) for the exact private RPC,
and [RELATED_PROJECTS.md](RELATED_PROJECTS.md) for ecosystem scope and comparison sources.

## Status words

- **Current** means code and an explicit contract exist in this checkout.
- **Accepted** means the active release evidence accepts that bounded path.
- **Configured** means composition must deliberately install the backend and protected dependencies.
- **Planned** means it is deliberately outside the current contract, not an implied feature.

## Current adapter contract

The implemented `server_core` surface has the current Assignment Access and Question delivery
routes listed in [API_CONTRACTS.md](API_CONTRACTS.md). The PLE and WeBWorK Question Backends
participate in the implemented lifecycle; iMathAS and H5P retain their bounded planned or
ungraded scopes behind the same shared boundaries. A later backend composes its own adapter with
this lifecycle; it does not create a backend-specific Assignment workflow.

| Concern | Common PLE rule |
| --- | --- |
| Source authority | A Published Question Revision and immutable Question Revision Reference select the backend. A browser does not select a backend, source path, source bytes, Question Seed, renderer, or backend configuration. |
| Question Type | The author declares the educational Question Type on the Draft source binding. Publication copies it to immutable Published Question Revision metadata. PLE uses this metadata for search, filters, labels, and other presentation needs; it does not infer it from backend controls or output. |
| Issuance | A backend adapter receives trusted server-derived identity, the Published Question Revision, immutable source, and Question Seed. It returns the public response descriptor, reproduction facts, and, when applicable, a backend-owned document for the issued position. |
| Presentation | PLE presents native response controls for PLE-native Questions. A `backendOwned` descriptor causes PLE to host the separately authorized backend document without inspecting its elements, controls, or authored styling. |
| Response | The browser saves `StudentResponse` through a PLE same-origin Question Attempt route. PLE structurally validates the declared response format and persists the bounded opaque payload without interpreting backend fields. The browser never submits a score, source identity, renderer configuration, credential, or answer key. |
| Finalization and grade | PLE finalizes an Attempt only after every issued position has a saved response. The selected backend interprets its response and produces the server-side outcome; PLE records that outcome under Assignment policy. |
| Reproduction details | `QuestionAttemptReproductionDetails` records Question Backend Version, optional Question Renderer Version, Source Object Reference, bound assets, Question Grader Version, and rendered-document SHA-256 where applicable. |
| Lifecycle state | The shared contract has one bounded opaque state slot for a backend that requires it. WeBWorK uses decision E1: state is absent at issue and grade. |
| Failure | A backend reports `Unsupported`, `Invalid`, or `Unavailable`. A renderer or backend outage is not converted into a Student incorrect response. |
| Capabilities | `QuestionBackendCapabilities` is a closed declaration. Question Publication Validation refuses an Assignment requiring a capability the selected backend did not declare. |

The browser-safe `QuestionPresentation` contains a public response shape and student presentation, never
an answer key. Its render `kind` selects the browser Question Response Control. The planned compact response wire drops
the redundant response `kind`, because the authoritative attempt already selects the Question Response Format.
See [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for current and target payloads.

## Backend comparison

| Backend                          | Current authority                                                                                                                                                                    | Browser response                  | Server grading authority                             | Current scope                                                                                                                                                            |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| PLE Question JSON                | One complete immutable PLE Question Source                                                                                                                                           | Typed PLE Question JSON response  | PLE Question Backend                                 | All eight PLE Question JSON version 3 Question Types; supported Authoring Workspace fields; M12 accepted issued native-control and Question Asset delivery for all eight types |
| QTI Import                       | Checksum-pinned archive, profile conversion, and Workspace Import evidence                                                                                                           | Becomes PLE Question JSON         | PLE Question Backend after conversion                | Canvas 1.2 and Blackboard 2.1 supported flat-item mappings                                                                                                               |
| WeBWorK                          | Immutable PG source, author-declared Question Type, private standalone renderer, and issued backend document                                                                         | Opaque ordered form-pair payload  | Private `/render-api` through the WeBWorK adapter    | Backend-owned HTML, interaction semantics, response interpretation, and server grading; stateless E1 lifecycle                                                           |
| iMathAS                          | Immutable Question Source resolution plus strict versioned iMathAS Launch State bytes                                                                                                | Same-origin `{ launchUrl }` only  | iMathAS Launch/Result HMAC and protocol verification | Browser shell has no Challenge/Session/backend secrets; live backend composition remains deferred                                                  |
| H5P Package                      | Complete H5P Package Question Source                                                                                                                                                 | H5P practice presentation         | No current server grading capability                 | Current ungraded practice; full shared-lifecycle integration remains open                                                                                                |
| iMathAS Question Backend Session | Exact Account, Course, Student Question Attempt, Question Revision, `ImathasQuestionBackendBinding`, Question Seed, Challenge, authentication, and verified Result Exchange checksum | No Session/Challenge/token output | LDA Store with one-use forward transition            | Browser launch shell is available; LDA-backed Rust Server Route, cookie/env backend composition, and live backend remain absent                                          |

## PLE Question JSON Questions

### Source and render

**Current.** The PLE Question Backend interprets one complete PLE Question JSON
source through the shared Question operations. It derives an answer-free
Question Presentation, evaluates Student Response, and supplies optional
protected Question Hint, Question Feedback, Question Answer, or Question Answer
Explanation when the applicable policy releases them. These roles remain
subordinate to the complete source rather than universal stored sidecars. The
Draft Question/Question Revision persistence and issued-Question delivery
binding remain open. The trusted server bridge
resolves immutable published-Question Asset References before issue, replay, or grade. The browser receives prompt
blocks, public Question Response Format, Question Asset References, Question Revision, and Question Seed. It returns only the PLE
response shape; it does not return source bytes, a private key, Question Hint, asset-object binding, implementation
version, or a scoring decision.

The current closed source contract supports multiple choice, multiple answer, fill-in-the-blank,
multi-blank, numerical, matching, ordering, and hotspot questions. The PLE Question Backend dispatches by
registered PLE Question Implementation for the explicit Question Format and Question Type rather than making the Assignment Attempt model type-specific. The browser authoring
surface exposes supported Authoring Workspace fields for the version 3 Question Types; HOTSPOT source data enters through
registered imported or trusted Question Asset References. Its instructor route is a convenience surface only: the server
re-resolves source and Question Asset References at save and publication, and the student contract remains
answer-free. Integrated author-to-publication-to-student acceptance for every Question Type, including imported/trusted
hotspot asset bindings, remains open.

### Grade, replay, and cache

The server validates immutable reference, Question Seed, Rendered Question SHA-256, and asset
References before asking the PLE Question Backend to evaluate the Student Response from the exact
issued source and reproduction details. Question Attempt Reproduction Details name the PLE Question Backend and
Question Grader Versions, optional
generator, bound objects, and rendered output hash.

PLE Question JSON generation is deterministic for a published Question Revision and Question Seed. A shared cache may contain only
answer-free generated output keyed by that identity. Course/Student state, keys, submissions, and feedback
never enter it. Static Questions still use the uniform Question Seed and parameter-hash record so swapped
Question Attempt Reproduction Details mismatch is detectable.

### Capabilities and extension

PLE Question JSON capabilities are the intersection declared by selected registered PLE Question Implementations. A new PLE Question Implementation supplies
a closed source/parser/compiler contract, browser-safe Question Response Format, server-owned
evaluation behavior, deterministic issue/reproduction, capability declaration, strict response
validation, and conformance coverage through the shared Question operations.

## QTI Import

QTI is an import, export, and archival interchange pathway rather than a Question Backend. An
authorized Instructor supplies a bounded archive to private Workspace Import object storage. The QTI
adapter parses the selected hostile-input profile and records an answer-free report, checksums,
mapping facts, Question Asset References, warnings, and unsupported-item results.

Each accepted item is converted into one complete PLE Question JSON Draft Question. From that point,
the shared Draft Question, publication, Assignment, issuance, presentation, submission, evaluation,
and feedback-release operations resolve the PLE Question Backend exactly as they do for directly
authored PLE Question JSON. The original QTI archive and mapping remain Workspace Import evidence;
they are not reinterpreted as another runtime pipeline.

Current accepted import profiles cover supported static Canvas QTI 1.2 and Blackboard Original QTI
2.1 items. Broader QTI interaction mappings and external QTI-JSONL interchange require explicit mapping
decisions and independent import acceptance.

## WeBWorK private renderer

### Source and render

**Current bounded path.** PLE is the only WeBWorK renderer client. A Published Question resolves
to immutable PG source under its Question License, the author-declared Question Type recorded on
that Revision, and a fixed Question Seed. The trusted adapter sends the source and seed to the
private standalone `/render-api` service. The renderer returns an opaque HTML document and its
renderer identity. PLE stores the exact document with the issued position and exposes only the
`backendOwned` response descriptor in the public Question Presentation.

The browser obtains that immutable document only from its Student-authorized same-origin document
route. PLE does not parse, rewrite, classify, or project PG controls, HTML structure, interaction
behavior, or Question-authored CSS. PLE supplies the frame, ordinary baseline styling, and response
save workflow. The document may contain legitimate PG hidden answer fields; it contains no PLE
credential or JWT hidden input. The private renderer URL, source path and bytes, credentials,
cookies, answer key, and renderer protocol stay outside the browser contract.

### Grade, replay, cache, and failure

The backend document bridge serializes the complete form as a bounded canonical JSON array of
`[name, value]` pairs. It preserves form order, repeated names, and legitimate hidden PG fields.
PLE base64-encodes that JSON as the opaque `StudentResponse::BackendOwned` payload, validates only
its bounded canonical envelope, and saves it at the issued position. It does not interpret field
names, values, controls, or the educational Question Type. Its 64 KiB raw UTF-8 bound applies only
to that decoded canonical Student response payload. It does not constrain PG/PGML source (256 KiB),
the rendered backend document and renderer envelope (1 MiB), or assets. Renderer credentials and
JWT are absent from the embed document, while legitimate hidden PG fields remain part of the
captured response.

The browser preflight and shared response API enforce the same 64 KiB bound. If real supported
content reaches it, revise the shared model, browser, and adapter contract together, then update
the boundary test; ordinary threshold drift is repaired at the layer that diverged.

After PLE finalizes the Attempt, the WeBWorK worker resolves the exact immutable source and seed,
passes the opaque pair array to the adapter once, and records the renderer's normalized outcome.
There is no WeBWorK replay mapping, PLE control-specific conversion, renderer-output cache, or
backend state to reconstruct. Decision E1 is stateless: the shared lifecycle-state slot is absent
for both render and grade. A renderer failure refuses issuance or grading; it never records an
incorrect response.

Decision C2 hosts the document in a same-origin iframe with `allow-scripts`, `allow-forms`, and
`allow-same-origin`, with the document CSP and asset proxy providing the complementary boundary.
The public asset proxy serves only renderer `webwork2_files` and `pg_files` paths through
`/api/webwork-assets/{prefix}/{path}`. It preserves no renderer authority and accepts only bounded,
safe static or generated asset responses.

### Capabilities and scope

The configured backend declares `algorithmicGeneration`, `serverGrading`, and `partialCredit`.
Those capability declarations are independent of the author-declared Question Type and of renderer
control structure.

**Planned.** Stateful continuation, post-submit WeBWorK feedback display, broad OPL compatibility,
browser access to the private renderer, and upstream gradebook/LTI passback remain outside the
current contract. A later PG interaction changes Question content or the backend adapter when its
backend behavior needs it; it does not require a new PLE interaction implementation merely because
the document contains different controls.
The detailed protocol is in [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md).

## iMathAS Question Backend Session

**Current server-only boundary.** Question Model owns `ImathasQuestionBackendBinding` with its
iMathAS Deployment Reference, iMathAS Item Reference, and pinned `imathas_remote_grading_v1`
profile. LDA persists that binding and owns the sole `ImathasQuestionBackendSession`, typed
Reference, preparation/restore/lease/verified-Result-Exchange Store operations, and backend-state
protection. `ImathasLaunchState` is iMathAS-owned, strict versioned opaque backend-handle bytes;
the adapter owns iMathAS Launch/Result HMAC and protocol verification plus iMathAS Render Cache
Entry. It cannot own a parallel Session, lifecycle, or backend-state encryption model.

LDA solely mints the fresh 256-bit iMathAS Session Challenge, keeps it immutable in one
Session, and accepts it once only through verified Exchange. iMathAS carries and verifies the
signed `ple_launch_challenge` protocol claim; it owns no duplicate Challenge type. The Challenge
expires with the Session and has no browser or generated DTO.

LDA solely owns the private, redacted, non-Serde iMathAS Grading Context
`{ QuestionAttemptId, QuestionRevisionReference, QuestionSeed }` across Session, Store, and
adapter validation. It inherits Student/Course/Assignment authority through its owning Session and
Question Attempt and expires with the Session. `authentication_payload_v1` keeps its accepted bytes;
the Context is distinct from iMathAS Launch Binding Checksum, Challenge, Result Token, and
iMathAS Result. The browser launch shell has no Context DTO.

LDA also owns the bounded opaque iMathAS Result Token and its redacted checksum. iMathAS verifies
the exact server-to-server response before deriving the checksum. Raw response bytes have no browser,
generated, durable, log, or Debug representation; the checksum is Exchange-only evidence, distinct
from the iMathAS Result checksum.

Migration `2026090102` persists the exact Context, `ImathasQuestionBackendBinding`, source,
`imathas_remote_grading_v1` profile, Question Seed, response checksum, Challenge, authentication, iMathAS Launch Binding Checksum,
expiry/revocation/consumption, lease, and encrypted backend state. Its Result Exchange owns one immutable
normalized-score-only iMathAS Result and LDA-derived checksum, alongside the Result
Token checksum. After iMathAS verification outside PostgreSQL, authenticated staging atomically
consumes the exact Session into Ready-to-Commit, creates the marker `StudentResponse::ImathasQuestionBackend {}`
Question Submission, pending Question Submission Grading, and ready typed grading Job. Only a worker
holding that Job's lease may lock the selected Issued Question, resolve its point value and scoring
rule, combine them with backend QuestionEvaluation, and idempotently commit the Assignment-owned
Grading Result and Automated Grading Receipt. A lease-expiry recovery claim is permitted; final worker failure belongs to the Job and
Question Submission Grading (`instructor_attention`) while ready evidence remains for an authorized
recovery Job. RLS and least-privilege SECURITY DEFINER functions require the authenticated active
Student and exact restore tuple; validity is half-open, binding immutable, and direct mutation refused.
iMathAS Result is distinct from the Result Token and PLE Grading Result; no browser/generated
Result DTO is created. LTI remains future registered-protocol planning only.

**Composition status.** The SolidJS browser launch shell POSTs the same-origin launch request,
accepts only `{ launchUrl }`, validates the exact safe path, and opens an iframe. It carries no
Challenge, Session, or backend secret. An LDA-backed Rust route, cookie/env production backend
composition, and live-backend acceptance remain absent. The ordinary separately implemented
indeterminate-effect policy continues to govern effectful iMathAS Question Backend requests where it is used.

**Adapter protocol vocabulary.** iMathAS Item Reference names the iMathAS-local logical item;
Source Object Reference and Source Object Checksum name immutable stored Question Source bytes;
the iMathAS Launch Binding Checksum verifies the exact launch-match value. These facts
remain server-only and never become browser-selected endpoints, source bytes, scores, or cookies.
Generic hosted MyOpenMath, arbitrary endpoints, browser-trusted launch URLs/scores, and unverified
iMathAS callbacks remain outside the supported boundary.

## Extension rules

1. Define durable published and private draft source identity without secrets or mutable endpoints.
2. Pin source bytes, Source Object Checksum, Question License, Question Attempt Reproduction Details, implementation/profile facts, and assets at publication.
3. Issue an answer-free Question Presentation or a backend-owned document; keep keys, credentials,
   iMathAS Session Authentication state, raw renderer output, and raw grading results server-only.
4. At issue, persist the exact Question Revision, Question Seed, Question Attempt Reproduction
   Details, and required immutable delivery evidence. A backend-owned document is retained as an
   exact issued document and served through its authorized document route. Each backend defines
   the evidence it needs for grade without making PLE inspect its interaction model.
5. Choose one grading authority: PLE Question JSON Private Grading, private renderer, or verified iMathAS Result.
6. Cache only evidence whose ownership and immutability are explicit. Never use shared cache state
   to carry a Student response, credential, or backend lifecycle state.
7. Declare only implemented capabilities, and make assignment validation refuse unsupported policy before issue.
8. Add deterministic conformance tests. Label recorded iMathAS fixtures separately from live service acceptance.

## Contract locations

| Contract                                                        | Primary locations                                                                                                                                                       |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shared Question model and Question Attempt Reproduction Details | `crates/question_model/src/{question_library.rs,student_work.rs,presentation/,capability.rs}`                                                                           |
| Adapter operations                                              | `crates/adapters/{ple,webwork,imathas,qti}`                                                                                                                             |
| Server composition and delivery                                 | `crates/server/src/{application.rs,composition.rs,assignment_delivery.rs,webwork_document_route.rs,webwork_asset_proxy.rs}`                                               |
| WeBWorK renderer                                                | `crates/adapters/webwork` and [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md)                                                                      |
| iMathAS Question Backend                                        | `crates/adapters/imathas`                                                                                                                                               |
| Student payload design                                          | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                                                                                                            |
| Security and storage                                            | [SECURITY_MODEL.md](SECURITY_MODEL.md), [OBJECT_STORAGE.md](OBJECT_STORAGE.md), and [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#typed-operations-and-objects) |
