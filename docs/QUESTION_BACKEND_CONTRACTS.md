# Question backend contracts

This document records the durable execution contract at PLE's question-backend boundary. It is a
reader's map of the implemented system, not a replacement for the active implementation plan.
The plan and its active release plan remain authoritative for dependency order and acceptance.

PLE is question agnostic at the learning-engine boundary. An Issued Question preserves one exact
published Question Revision, and a Question Attempt binds one exact Question Backend adapter. Each
adapter safely issues, reproduces, and grades through its exact Question Backend-specific Question Grading Input; PLE owns Account and exact
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

## Current adapter contract

The mounted `server_core` surface currently has no Question delivery route. The
PLE, WeBWorK, and iMathAS Question Backends each expose their explicit issue,
reproduce, and grade operations; a later server delivery boundary will compose
them without creating a second product vocabulary.

| Concern                               | Common PLE rule                                                                                                                                                                                                                                                              |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source authority                      | A published `QuestionRevision` and immutable `QuestionRevisionReference` select the backend. A browser does not select a backend, source path, source bytes, seed, renderer, or backend configuration.                                                                       |
| Issuance                              | A Question Backend adapter receives trusted server-derived Account and exact course/Student relationship, published Question Revision, and server-owned seed. It returns a key-free Question Presentation and Question Attempt Reproduction Details.                         |
| Reproduction                          | A Question Backend adapter limits reproduction to issue-time work and explicit active Question Backends without a public Question Presentation. Presentation-bearing first submit and submitted delivery validate the owned snapshot/private Question Grading Input instead. |
| Response                              | The browser submits `StudentResponse` to a PLE same-origin attempt route with an idempotency key. It never submits a score, iMathAS Session Authentication state, source identity, renderer field, or answer key.                                                            |
| Grade                                 | A Question Backend adapter returns a server-side outcome. The later delivery route owns policy-aware persistence; the iMathAS Question Backend may atomically commit its verified iMathAS Result Exchange.                                                                   |
| Question Attempt Reproduction Details | `QuestionAttemptReproductionDetails` records a Question Backend Version, optional Question Renderer Version, Source Object Reference, bound assets, Question Grader Version, and Rendered Question SHA-256.                                                                  |
| Failure                               | A backend reports `Unsupported`, `Invalid`, or `Unavailable`. An unavailable renderer or iMathAS Question Backend is not converted into a student incorrect response.                                                                                                        |
| Capabilities                          | `QuestionBackendCapabilities` is a closed declaration. Publication validation refuses an assignment requiring a capability the selected backend did not declare.                                                                                                             |

The browser-safe `QuestionPresentation` contains a public response shape and student presentation, never
an answer key. Its render `kind` selects the browser Question Response Control. The planned compact response wire drops
the redundant response `kind`, because the authoritative attempt already selects the Question Response Format.
See [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) for current and target payloads.

## Backend comparison

| Backend                          | Current authority                                                                                                                                                | Browser response                  | Server grading authority                                    | Current scope                                                                                                                                                                   |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PLE Question JSON                | Immutable PLE Question Source and private Question Grading Input                                                                                                 | Typed PLE Question JSON response  | PLE Question Backend plus isolated PLE Question JSON grader | All eight PLE Question JSON schema version 2 Question Types; supported Authoring Workspace fields; imported/trusted Question Asset bindings and all-type acceptance remain open |
| QTI profile                      | Immutable staged/published archive plus profile conversion evidence                                                                                              | Typed PLE response                | `QtiBackend` plus least-privilege `QtiGradingStore`         | Canvas 1.2 and Blackboard 2.1 static single-choice profiles                                                                                                                     |
| WeBWorK                          | Immutable PGML Question Source governed by its owning Question License and private renderer                                                                      | Opaque PLE choice or match IDs    | Private `/render-api` Question Backend                      | Four reviewed Chapter 1 PGML sources: MC plus MATCH per chapter; exact-source matching partial credit                                                                           |
| iMathAS                          | Immutable Question Source resolution plus strict versioned iMathAS Launch State bytes                                                                            | Same-origin `{ launchUrl }` only  | iMathAS Launch/Result HMAC and protocol verification        | Browser shell has no Challenge/Session/backend secrets; LDA-backed Rust route and backend composition remain downstream                                                         |
| iMathAS Question Backend Session | Exact Account/course/Student attempt, Question Revision, `ImathasQuestionBackendBinding`, seed, Challenge, authentication, and verified Result Exchange checksum | No Session/Challenge/token output | LDA Store with one-use forward transition                   | Browser launch shell is mounted; LDA-backed Rust route, cookie/env backend composition, and live backend remain absent                                                          |

## PLE Question JSON Questions

### Source and render

**Current.** The PLE Question Backend compiles versioned PLE Question JSON into three separate products: an answer-free
draft for Question Revision/Question Presentation, private Answer Key, Question
Feedback, Question Answer Explanation, and format-specific Question Grading
Input, plus an optional private Question Hint. The Hint is bounded authored
pre-response teaching content, not Question Feedback or grading content. Its
Draft Question Revision/Question Revision persistence and issued-Question
delivery binding remain open. The trusted server bridge
resolves immutable published-Question Asset References before issue, replay, or grade. The browser receives prompt
blocks, public Question Response Format, asset references, version, and seed. It returns only the PLE
response shape; it does not return source bytes, a private key, Question Hint, asset-object binding, implementation
version, or a scoring decision.

The current closed source contract supports multiple choice, multiple answer, fill-in-the-blank,
multiple choice, multiple answer, fill-in-the-blank, multi-blank, numerical, matching, ordering, and hotspot questions. The PLE Question Backend dispatches by
registered PLE Question Implementation for the explicit Question Format and Question Type rather than making the Assignment Attempt model type-specific. The browser authoring
surface exposes supported Authoring Workspace fields for seven v2 Question Types; HOTSPOT source data enters through
registered imported or trusted Question Asset References. Its instructor route is a convenience surface only: the server
re-resolves source and Question Asset References at save and publication, and the student contract remains
answer-free. Integrated author-to-publication-to-student acceptance for every Question Type, including imported/trusted
hotspot asset bindings, remains open.

### Grade, replay, and cache

The server validates immutable reference, Question Seed, Rendered Question SHA-256, and asset
References before generic grading or isolated PLE Question JSON grading. First PLE Question JSON grade reads only the issued
checksummed PLE Question JSON Private Grading; ordinary published-Question and browser paths cannot read that private grading data or
replace it with a current Question Grader view. Question Attempt Reproduction Details name the PLE Question Backend and
Question Grader Versions, optional
generator, bound objects, and rendered output hash.

PLE Question JSON generation is deterministic for a published version and Question Seed. A shared cache may contain only
answer-free generated output keyed by that identity. Course/Student state, keys, submissions, and feedback
never enter it. Static questions still use the uniform seed and parameter-hash record so swapped
Question Attempt Reproduction Details mismatch is detectable.

### Capabilities and extension

PLE Question JSON capabilities are the intersection declared by selected registered PLE Question Implementations. A new PLE Question Implementation must
add a closed source/parser/compiler contract, browser-safe Question Response Format, server-only key or
rubric, deterministic issue/reproduction, capability declaration, strict response validation, and
conformance coverage. It must not add a parallel run loop or browser grader.

## QTI profile questions

### Source and render

**Current.** QTI is an import and private-grading path, not a browser QTI runtime. An authorized
author uploads a bounded archive to private workspace object storage. The worker parses a narrow,
hostile-input profile and records safe reports, checksums, normalized item facts, Question Asset References, and
server-only grading handoff. Publication atomically pins archive and item identity.

At issue or replay, `QtiBackend` rereads the exact published archive, verifies object identity and
SHA-256, reparses it, checks the selected item against the durable answer-free Question Content, resolves
immutable assets, and returns a normal answer-free PLE Question Presentation. The student submits the same typed
PLE response as for PLE Questions. Student JSON has no QTI XML, archive Object Address, import ID, or
answer binding.

### Grade, Question Attempt Reproduction Details, and scope

`QtiBackend` obtains `QtiGradingHandoff` only through separately injected, least-privilege
`QtiGradingStore`. The normal published-Question/object store resolves public archive and asset evidence but
cannot recover correct responses. Issue, replay, and grade fail closed if archive, checksum, item,
asset mapping, or private binding no longer reproduces. The Question Attempt Reproduction Details records private-profile adapter,
Source Object Reference, bound assets, QTI private grader, and Rendered Question SHA-256.

When explicitly configured, QTI declares `serverGrading`. Current accepted import profiles are static
single-choice Canvas QTI 1.2 and Blackboard Original QTI 2.1 pools. Other XML, interaction types,
embedded execution, and broad QTI interchange are refused rather than partially interpreted.

**Planned.** Broader QTI Question Types and external QTI-JSONL interchange require new profile decisions,
conversion semantics, private key handling, and independent live acceptance. PLE Question JSON schema version 2 alone does
not enable them.

## WeBWorK private renderer

### Source and render

**Accepted bounded path.** PLE is the only WebWork client. A Published Question resolves to an immutable,
user-authored PGML Question Source governed by its owning Question License and a fixed seed. The API sends server-owned form data to a
private standalone `/render-api` Question Backend service. The browser receives only a typed PLE Question Presentation
and opaque presentation-scoped Question Choice References. It never receives PG source, file path, renderer
URL, credentials, upstream hidden fields, cookies, session key, radio name, or radio value.

The accepted Question Presentation covers the exact reviewed Chapter 1 `RadioButtons` and matching shapes. PLE
rejects unsupported upstream controls and emits opaque IDs per projected label or matching
side. A student submits PLE IDs to PLE, not upstream form fields.

### Grade, replay, cache, and failure

For a newly issued attempt, PLE resolves immutable source, captures and validates the private
field/value mapping, converts durable Question Choice References to presentation-scoped Response Item References, and
persists that mapping with the exact public snapshot, private Question Grading Input, and frozen WeBWorK
Question Source. Normal grade reloads those validated artifacts, maps the Student's Presentation Response Item Reference through
the private Response Item Binding and Question Grading Input, and makes one private grade request. It does not reconstruct an issuance
render or resolve a current published Question Revision. The mapping never
appears in a Question Presentation, safe cache, receipt, log event, or browser response.

The shared immutable cache is keyed by version and seed. It holds only its schema version, an answer-free
typed `QuestionVariationPresentation`, Source Object Reference, Source Object Checksum, and
Question Renderer Version. A cache hit
for a new issuance still performs the bounded private render needed to create that attempt's replay
mapping; reproduction and normal grade do not. Telemetry uses only `renderer_call` and `cache_hit`
event names.

The renderer accepts only expected form/JSON shapes, bounded bodies, fixed origins, known protected
echo fields, and typed Question Content Blocks. Redirects, malformed or duplicate JSON, unknown fields,
protected-field mismatches, unsafe HTML, unsupported controls, and timeouts refuse the operation; they
do not produce a grade or leak secrets.

### Capabilities and scope

The configured backend declares `algorithmicGeneration` and `serverGrading`. The reviewed matching
sources additionally declare `partialCredit` only when both their source path and immutable Source Object Checksum
match the evidence profile. Other PG sources remain all-or-nothing.

**Planned.** Generic PG controls, unreviewed matching sources, broad OPL compatibility, browser
access to WebWork, and upstream gradebook/LTI passback remain outside the accepted scope. Any added
control needs its own Question Presentation, private replay mapping, response contract, browser interaction,
and acceptance evidence.
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

Migration `2026090102` persists the exact Context, separate required issue-time `QuestionGradingRule`,
`ImathasQuestionBackendBinding`, source, `imathas_remote_grading_v1` profile, seed, response checksum, Challenge, authentication, iMathAS Launch Binding Checksum,
expiry/revocation/consumption, lease, and encrypted backend state. Its Result Exchange owns one immutable
normalized-score-only iMathAS Result and LDA-derived checksum, alongside the Result
Token checksum. After iMathAS verification outside PostgreSQL, authenticated staging atomically
consumes the exact Session into Ready-to-Commit, creates the marker `StudentResponse::ImathasQuestionBackend {}`
Question Submission, pending Question Submission Grading, and ready typed grading Job. Only a worker
holding that Job's lease may idempotently commit the derived PLE Grading Result and Automated Grading
Receipt. A lease-expiry recovery claim is permitted; final worker failure belongs to the Job and
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
3. Issue an answer-free Question Presentation; keep keys, rubrics, mappings, credentials, iMathAS Session Authentication state, and raw results server-only.
4. At issue, capture the exact version/seed render, compare the complete Question Attempt Reproduction Details, and persist its
   answer-free public snapshot plus server-only Question Grading Input. Retry, submitted delivery, and
   grade validate those artifacts rather than rerendering.
5. Choose one grading authority: PLE Question JSON Private Grading, private renderer, or verified iMathAS Result.
6. Cache only immutable answer-free render output. Bind private replay state to the exact course/Student attempt, never shared cache.
7. Declare only implemented capabilities, and make assignment validation refuse unsupported policy before issue.
8. Add deterministic conformance tests. Label recorded iMathAS fixtures separately from live service acceptance.

## Contract locations

| Contract                                                        | Primary locations                                                                                                                                                       |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shared Question model and Question Attempt Reproduction Details | `crates/question_model/src/{question_library.rs,student_work.rs,presentation/,capability.rs}`                                                                           |
| Adapter operations                                              | `crates/adapters/{ple,webwork,imathas,qti}`                                                                                                                             |
| Server composition                                              | `crates/server/src/{application.rs,composition.rs}`; Question delivery composition remains unmounted                                                                    |
| WeBWorK renderer                                                | `crates/adapters/webwork` and [WEBWORK_PG_RENDERER_API_USAGE.md](WEBWORK_PG_RENDERER_API_USAGE.md)                                                                      |
| iMathAS Question Backend                                        | `crates/adapters/imathas`                                                                                                                                               |
| Student payload design                                          | [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md)                                                                                                            |
| Security and storage                                            | [SECURITY_MODEL.md](SECURITY_MODEL.md), [OBJECT_STORAGE.md](OBJECT_STORAGE.md), and [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md#typed-operations-and-objects) |
