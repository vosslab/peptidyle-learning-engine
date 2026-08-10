# Contract register

This register turns the module catalog in the
[active implementation plan](active_plans/implementation_plan.md) into an
ownership and change-control boundary. It records all 36 catalog modules. It
does not mark later milestone behavior as implemented.

## How to read the register

Contract source entries use three states:

- **Frozen** means a callable type, trait, route, or facade exists and current
  consumers may compile against it.
- **Implemented** means the named production behavior exists and is exercised
  through its listed contract or focused behavior tests.
- **Bounded test double** means a mock or fixture implements only a declared
  test seam; it is not a second public API or a production fallback.
- **Reserved** means this register row is the current frozen contract and names
  an unsupported or future boundary. A reserved entry is not evidence that the
  source file or behavior exists.

Owners are plan roles, not permanent people. One role owns each contract.
Consumers lists direct module consumers. MOD-DEPLOY consumes the whole system
and is implicit in every row unless it is the only consumer.

## QTI profile-to-native contract

The accepted profile-import path has one bounded ownership chain:

1. [crates/server/src/qti_profile_import.rs](../crates/server/src/qti_profile_import.rs)
   authorizes an existing workspace draft before reading opaque ZIP bytes, stores the protected
   archive, queues the profile worker, and projects only the answer-free report.
2. [crates/server/src/qti_import.rs](../crates/server/src/qti_import.rs) invokes the bounded
   Canvas/Blackboard profile parser and commits accepted and rejected outcomes with exact digests.
3. [crates/server/src/qti_profile_conversion.rs](../crates/server/src/qti_profile_conversion.rs)
   re-reads the immutable archive, repeats the report and acknowledgement bindings, and delegates
   one accepted item through
   [crates/server/src/qti_profile_flat_bridge.rs](../crates/server/src/qti_profile_flat_bridge.rs)
   to the native flat compiler. One Store command commits public source, opaque grading material,
   provenance, and the current draft revision atomically.
4. [src/features/qti_profile_import/](../src/features/qti_profile_import/) owns the same-origin author
   workflow for upload, polling, review, acknowledgement, conversion, and editor handoff.

The browser report and draft remain answer-free. Only the separately injected PostgreSQL grader
handle can resolve the opaque private payload. The real disposable acceptance oracle in
[crates/server/src/qti_profile_postgres_live.rs](../crates/server/src/qti_profile_postgres_live.rs)
is invoked by [tests/e2e/e2e_database_baseline.sh](../tests/e2e/e2e_database_baseline.sh); it proves
mixed accepted/rejected import, edit/publish, correct and incorrect grading, role and foreign-tenant
denials, archive/provenance checksum retention, and exact cleanup against PostgreSQL 17.

## Course appearance contract

`crates/question_model/src/course_appearance.rs` owns the frozen browser-safe appearance vocabulary.
It defines the 15 closed theme IDs, with `grass`
as the only default; exact decimal-string appearance revisions; explicit decorative or validated
informative banner text; opaque current and candidate route identities; the safe current projection;
and one strict theme plus keep/remove/replace update body. The generated TypeScript union is derived
from this Rust owner, including `coral-reef`, `salt-marsh`, and `sea-floor`. Unknown IDs refuse.

The projection and candidate receipt contain no object key, bucket, checksum, filename, source bytes,
upload metadata, signed URL, grading material, or answer-bearing type. Course identity is route-owned,
and compare-and-swap authority is the strong `If-Match` header rather than a body field. The working
browser route is `/instructor/courses/:courseId/appearance`. Implemented production HTTP paths are
`GET`/`PUT /api/courses/{courseId}/appearance`, author-only
`POST /api/courses/{courseId}/appearance/banner-candidates`, and same-origin
`GET /api/assets/{id}` for a current banner only after current-pointer authorization. The
tenant/course-bound `CourseBannerCandidate` and `CourseBanner`
object identities without caller-supplied paths: candidates are temporary and non-signable, while
immutable current banners are protected `CourseContent` and signable only at the typed-object layer.
Storage creates the default appearance with its course in Memory and PostgreSQL, enforces
persisted manager authority and revision CAS, tracks bytes-first promotion, authorizes only the exact
current persisted pointer, and rechecks that pointer during bounded cleanup. Its security-definer
actor resolver returns only an authorized actor identity; `ple_app` retains no direct
`auth_session` read grant. A database trigger independently rejects a current pointer unless its
delivery kind, tenant, and course exactly match the appearance row. The production `GET`, `PUT`, and
candidate-upload routes use strong ETags and no-store responses. Successful appearance traffic runs
one bounded best-effort tenant claim/delete/complete sweep: absence is idempotent, another tenant's
key is never accepted, and object-store failure leaves the database claim retryable without blocking
a course read. The server-only image boundary accepts bounded
JPEG, PNG, or WebP, rejects active/animated or malformed input, applies orientation, strips metadata,
and writes one verified candidate before any atomic appearance mutation. Replacement revalidates a
server-private future banner identity and copies exact bytes before revision CAS; the shared asset
route authorizes only the persisted current pointer. No appearance operation is a Wasm export.

The Solid owner under `src/features/course_appearance/` implements the strict client decoder,
state-preserving instructor form, exact preview, and route-scoped 15-theme registry. The learner
course entry consumes the already-authorized `CourseRouteData`, renders the title as text, and emits
one image only for a current banner. Assignment, run, summary, editor, gradebook, appearance, and
global routes never display that entry banner. Browser state carries no object key, checksum,
filename in JSON, signed URL, answer, or grading material.

The browser's `CourseRouteData` pairs one authorized `CourseSummary` with that safe appearance.
Course-ID routes load both through one route query. `RunScreenData` carries the same pair so the
attempt page does not issue a second appearance request, while `RunSummaryResponse` receives its
course identity only from the server-authorized stored assignment before loading the safe
projection. The strict decoder accepts only the 15 generated IDs and positive decimal revisions;
unknown theme IDs are contract failures and never silently fall back.

The image contract is also fixed: after orientation, an accepted raster must supply a 1200 by 328
pixel center crop. The server strips metadata and emits exactly one WebP at those dimensions without
upscaling. Browser surfaces scale that derivative down while preserving its intrinsic aspect; they do
not stretch, recrop, or request device-specific variants.

## Domain contracts

| ID        | Contract source and state                                                                                                                                                                                                    | Owner          | Direct consumers                                                                                                                                                                                   | Reference/test implementation                                                                                               |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| MOD-QM    | Frozen: `crates/question_model/src/lib.rs`, browser-safe `course_appearance.rs`, and generated `generated/api/` types; reserved atomic draft-identity refactor with MOD-ID, MOD-STO, MOD-SCHEMA, MOD-API-CAT, and MOD-CLIENT | `architect`    | MOD-ID, MOD-RUN, MOD-STATE, MOD-TIME, MOD-SCORE, MOD-CAP, MOD-GEN, MOD-GRD, MOD-OBJ, MOD-STO, MOD-ADP-NAT, MOD-ADP-WW, MOD-ADP-QTI, MOD-ADP-H5P, MOD-ADP-IMATHAS, MOD-EXPORT, MOD-WASM, MOD-CLIENT | Current frozen contract remains authoritative until every refactor consumer, generated client, and fixture lands atomically |
| MOD-ID    | Frozen: `crates/question_model/src/identity.rs`, `lifecycle.rs`, and `catalog.rs`; reserved atomic draft-only/published-only identity and publish-mint transition                                                            | `architect`    | MOD-OBJ, MOD-STO, MOD-SCHEMA, MOD-API-CAT, MOD-CLIENT, MOD-ADP-IMATHAS                                                                                                                             | Current frozen contract remains authoritative until the draft-identity refactor lands atomically                            |
| MOD-RUN   | Frozen: `crates/question_model/src/activity.rs`, `run_policy.rs`, and `crates/domain/src/run.rs`; compatibility scoring re-export in `run.rs`                                                                                | `architect`    | MOD-STATE, MOD-SCORE, MOD-STO, MOD-SCHEMA, MOD-API-RUN, MOD-STATS                                                                                                                                  | n/a                                                                                                                         |
| MOD-STATE | Frozen: `crates/domain/src/attempt.rs` and `completion.rs`; compatibility completion re-export in `run.rs`                                                                                                                   | `expert_coder` | MOD-GRD, MOD-WASM, MOD-API-RUN                                                                                                                                                                     | n/a; pure transition contract                                                                                               |
| MOD-TIME  | Frozen: `crates/domain/src/timing.rs`                                                                                                                                                                                        | `expert_coder` | MOD-WASM, MOD-API-RUN                                                                                                                                                                              | n/a; pure verdict contract                                                                                                  |
| MOD-SCORE | Frozen: `crates/domain/src/scoring.rs`                                                                                                                                                                                       | `expert_coder` | MOD-STO, MOD-API-RUN                                                                                                                                                                               | n/a; batch selection and incremental projection                                                                             |
| MOD-CAP   | Frozen: `crates/question_model/src/capability.rs`, `crates/domain/src/policy.rs`, and the committed violation table                                                                                                          | `expert_coder` | MOD-WASM, MOD-API-CAT, MOD-UI-EDITOR                                                                                                                                                               | n/a; complete violation list contract                                                                                       |
| MOD-GEN   | Frozen: `crates/domain/src/generator.rs` and `crates/domain/tests/seed_vectors.json`                                                                                                                                         | `expert_coder` | MOD-ADP-NAT, MOD-WASM                                                                                                                                                                              | n/a; parity evidence owned by `tester`                                                                                      |
| MOD-GRD   | Frozen and implemented server-only `grade(question, response, key)` contract: `crates/grading/src/lib.rs`, `key.rs`, and `checker.rs`. The narrowly typed flat private-integrity surface is the only permitted MOD-STO use.  | `expert_coder` | MOD-ADP-NAT, MOD-STO, MOD-API-RUN                                                                                                                                                                  | n/a; server-only                                                                                                            |

## Cross-cutting assessment contracts

These rows index a contract that crosses more than one catalog module. They
do not create alternate schemas: the named Rust type, migration, route, or
durable design document remains the authority. "Reserved" identifies a
decided next boundary, not a browser route that is already available.

| ID | Contract source and state | Owner | Direct consumers | Reference/test implementation |
| --- | --- | --- | --- | --- |
| CON-ATTEMPT-ISSUANCE | Implemented: `IssueQuestionAttemptCommand`, `PresentationBindingV1`, `QuestionAttempt`, and `PrefetchedQuestion` bind an issued or promoted attempt to its tenant-owned run, immutable version, seed, timing, predecessor receipt, and exact presentation. The paired `in_memory/runs/attempt_issuance.rs` and `postgres/runs/attempt_issuance.rs` owners preserve the same issue-or-resume, prefetch-promotion, timer, conflict, and private replay-persistence behavior. | `expert_coder` | MOD-RUN, MOD-STO, MOD-API-RUN, MOD-ADP-NAT, MOD-ADP-WW | Memory/PostgreSQL conformance and focused run-prefetch tests |
| CON-PRESENTATION-V1 | Implemented foundation: `crates/question_model/src/presentation/` owns the closed public presentation descriptor, SHA-256 digest, nonce, CRC-16/CCITT-FALSE rendered-item IDs, collision retry, and exact private `PresentationBindingV1`. The browser-safe envelope carries render discriminants; durable item identities, keys, and scoring material do not cross the boundary. | `expert_coder` | MOD-QM, MOD-STO, MOD-API-RUN, MOD-WASM, MOD-UI-RENDER | Presentation codec/builder vectors and Wasm descriptor verification |
| CON-LEARNER-PAYLOAD-V1 | Reserved atomic public cutover: one purpose-built learner screen and an attempt-ID route with `Idempotency-Key`, presentation digest, and family-minimal type-free `answer`. The current run route still projects broad attempts and tagged `StudentResponse`; it is not the v1 compact learner wire. The normative replacement, rollout, and acceptance gates are [secure question grading payloads](active_plans/decisions/secure_question_grading_payload_plan.md) and [assessment payload design](ASSESSMENT_PAYLOAD_DESIGN.md). | `expert_coder` | MOD-API-RUN, MOD-CLIENT, MOD-UI-ATTEMPT, MOD-ADP-NAT, MOD-ADP-WW | WP-P3/5/6 family, secrecy, browser-trace, and migration gates |
| CON-WEBWORK-REPLAY-V1 | Implemented foundation: the private `WebworkIssuedAttempt::replay` mapping and `WebworkReplayMappingV1` are captured at issue and persisted tenant-bound with the attempt/prefetch presentation by MOD-STO. Reserved: WP-P4's normal one-private-RPC grade path and its narrowly bounded missing-state self-heal. Browser-visible state never contains the mapping, PG source, field/value pairs, credentials, or raw renderer result. | `expert_coder` | MOD-ADP-WW, MOD-STO, MOD-API-RUN | Adapter recorded fixtures, Store parity, and WP-P4 private trace gate |
| CON-FLAT-V2 | Implemented but not RC4-accepted: `crates/adapters/native/src/flat_question/v2.rs` owns strict PLE flat JSON v2 parsing and compilation for multiple choice, multiple answer, fill-in-the-blank, multi-blank, numerical, matching, ordering, and hotspot families. It preserves the v1 single-choice source contract. Visual authoring, integrated all-family acceptance, hotspot media/pointer work, and pilot content remain assigned to WP-RC4/RC5. | `expert_coder` | MOD-ADP-NAT, MOD-API-CAT, MOD-API-RUN, MOD-UI-RENDER | Native flat-v2 parser/compiler and response-widget tests |
| CON-LEARNER-UPLOAD | Implemented fail-closed current boundary: file-upload responses refuse before submission mutation and no learner upload capability, object key, signed URL, or scanner bypass is exposed. Reserved implementation: the server-issued, tenant/learner/attempt/presentation-bound `LearnerUploadId`, streaming temporary ingress, closed inspection, atomic consumption, protected delivery, retention, and reconciliation contract in [secure learner file-upload plan](active_plans/active/secure_learner_file_upload_plan.md). | `expert_coder` | MOD-QM, MOD-OBJ, MOD-STO, MOD-API-RUN, MOD-WORKER, MOD-UI-ATTEMPT, MOD-RETENTION | Existing refusal test; WP-FU1..WP-FU6 acceptance gates |
| CON-EVIDENCE-CLASS | Frozen verification boundary: permanent tests prove deterministic public behavior, security, and accessibility invariants; one-time probes record rebuild, fixture-size, latency, migration, or query-plan evidence and do not become brittle suite residents. [PYTEST_STYLE.md](PYTEST_STYLE.md) and [assessment payload design](ASSESSMENT_PAYLOAD_DESIGN.md#test-classification) define the classification; work-package plans own their required gates. | `tester` | Every contract owner | Focused behavior tests, disposable live oracles, and recorded implementation evidence |

## Storage and adapter contracts

| ID              | Contract source and state                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Owner          | Direct consumers                                                                                                                 | Reference/test implementation                                  |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| MOD-OBJ         | Frozen contract and implemented backend: `crates/objects/src/lib.rs` and `s3.rs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | `expert_coder` | MOD-ADP-WW, MOD-ADP-QTI, MOD-ADP-IMATHAS, MOD-EXPORT, MOD-API-ASSET, MOD-RETENTION                                               | `MemoryObjectStore`; opt-in MinIO gate                         |
| MOD-STO         | Learning data access (compatibility ID/path): frozen contracts plus in-memory and PostgreSQL backends under `crates/learning-data-access`, including atomic flat-question draft/source persistence and a separately injected `PostgresGraderStore`. MOD-STO may consume MOD-GRD only to validate opaque typed flat private material and its answer-free public binding; it exposes neither bytes nor grading decisions.                                                                                                                                       | `expert_coder` | MOD-SCHEMA, MOD-API-AUTH, MOD-API-CAT, MOD-API-COURSE, MOD-API-RUN, MOD-API-ASSET, MOD-WORKER, MOD-STATS, MOD-RETENTION, MOD-LTI | In-memory data access (`MemoryStore`); opt-in PostgreSQL gate  |
| MOD-SCHEMA      | Implemented: `schemas/migrations/` and `crates/learning-data-access/src/rls.rs`; reserved atomic draft-identity JSON/schema migration                                                                                                                                                                                                                                                                                                                                                                                                                         | `expert_coder` | MOD-STO, MOD-ID, MOD-API-CAT                                                                                                     | n/a; opt-in fresh-database gate                                |
| MOD-ADP-NAT     | Implemented algorithmic issue/reproduction/grading plus strict PLE flat-question JSON v1 compatibility and the v2 MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT parser/compiler: `crates/adapters/native/src/lib.rs`, `generator.rs`, `flat_question.rs`, and `flat_question/v2.rs`. Atomic workspace/publication persistence, instructor flat authoring, QTI-derived draft editing/publication, and the isolated native runtime are implemented; the visual author editor remains v1 single choice. CON-FLAT-V2 records the unaccepted RC4/RC5 work still outside this implemented source contract. | `expert_coder` | MOD-API-CAT, MOD-API-RUN, MOD-WORKER                                                                                             | Native parser/compiler and family response tests                |
| MOD-ADP-WW      | Implemented bounded server-only renderer boundary: `crates/adapters/webwork/src/lib.rs`, `renderer_contract.rs`, `http_renderer.rs`, `shipped_render_rpc.rs`, and `sanitizer.rs`. The contract owns renderer request, response, identity, failure, render, grade, and issue-time private replay-mapping types; WP-RC3 uses the upstream shipped `/webwork2/render_rpc` form/JSON protocol. CON-WEBWORK-REPLAY-V1 records the separately reserved one-call grade optimization.                                                                                                                                                           | `expert_coder` | MOD-API-CAT, MOD-API-RUN, MOD-WORKER                                                                                             | Recorded renderer fixtures and private RC3 acceptance           |
| MOD-ADP-QTI     | Private staging, immutable promotion, reviewed Canvas/Blackboard profile import, profile-to-native conversion, and opt-in published runtime are implemented. The profile routes authorize before reading bytes, expose only safe reports, reparse the checksum-pinned archive, and commit source, opaque grading material, provenance, and draft revision atomically. Published QTI runtime is registered only with `PLE_QTI_RUNTIME_ENABLED=1` plus separate `PLE_GRADER_DATABASE_URL`; answer bindings remain reachable only through `PostgresGraderStore`. | `expert_coder` | MOD-API-CAT, MOD-API-RUN, MOD-WORKER                                                                                             | `MemoryObjectStore`; opt-in PostgreSQL grader gate             |
| MOD-ADP-H5P     | Implemented key-free H5P practice import: `crates/adapters/h5p/src/lib.rs` and `import.rs`. The adapter accepts bounded supported package content into unpublished practice payloads and deliberately has no grading-key dependency.                                                                                                                                                                                                                                                                                                                          | `expert_coder` | MOD-API-CAT, MOD-WORKER                                                                                                          | n/a; ungraded capability declaration                           |
| MOD-ADP-IMATHAS | Implemented only for an explicitly configured contracted/self-hosted scored-embed provider: immutable source, protected same-origin launch, server verification, and optional production composition. Generic hosted MyOpenMath remains refused. Backend value `imathas`; iMathAS is the label and MyOpenMath a provider                                                                                                                                                                                                                                      | `expert_coder` | MOD-API-CAT, MOD-API-RUN, MOD-WORKER, MOD-UI-RENDER, MOD-UI-ATTEMPT                                                              | Recorded redacted fixtures; non-production provider probe only |
| MOD-EXPORT      | Implemented answer-key-free print model and deterministic four-artifact bundle: `crates/export/src/lib.rs`, `docx.rs`, and `pdf.rs`. Standard and accessible DOCX/PDF writers accept only published prompt/response presentation plus checksum-verified printable assets; unsupported response or asset shapes refuse before rendering.                                                                                                                                                                                                                       | `coder`        | MOD-WORKER, MOD-API-ASSET                                                                                                        | published fixture version                                      |
| MOD-WASM        | Frozen: `crates/wasm/src/lib.rs` and browser facade `src/wasm/index.ts`, including key-free format, timer, capability evaluation, and unversioned native-draft preview                                                                                                                                                                                                                                                                                                                                                                                        | `expert_coder` | MOD-UI-WIDGETS, MOD-UI-EDITOR                                                                                                    | n/a; exact export allowlist                                    |

## WeBWorK RPC contract

WP-RC3's renderer is a private upstream WebWork2 service, not a browser API or
the retired PLE `/v1/render` and `/v1/grade` dialect. `http_renderer.rs` makes
the same bounded authenticated `POST` request for render and grade to the
application-base-relative `render_rpc` path. It sends
`application/x-www-form-urlencoded` fields from server-owned configuration
only: a base64 `problemSource`, immutable `fileName`, stored seed, direct
render-course credentials, `outputformat=json`, display flags, and, on grade,
one re-rendered radio field/value plus `WWsubmit=1`.

The response parser accepts the fixed default upstream JSON shape, validates
the complete `hidden_input_field` object against the outgoing request, verifies
the root `real_webwork_SITE_URL` and exact `real_webwork_FORM_ACTION_URL`, and
then discards those values. Redirects, non-JSON responses, duplicate JSON keys,
unexpected fields, protected-field mismatches, malformed markup, unsafe
resources, and all unsupported controls refuse. The only supported projection
is one RadioButtons group with a bounded number of options. Its label-wrapped
radio inputs are removed from the prompt, then emitted as PLE opaque choice
IDs. The submitted ID is mapped back to an upstream field/value only inside the
second server-side request. A valid score is exactly numeric `0` or `100`;
`100` earns all published positive points and `0` earns none.

The browser receives only the typed PLE question envelope and later submits a
PLE response to the same-origin run API. It never receives PG source, file
path, direct password, session key, upstream URL, upstream field/value, raw
RPC body, or an upstream cookie. The adapter cache is immutable by published
version and stored seed. It records only the sanitized envelope/markup,
source-artifact binding, renderer identity, and rendered-output checksum.
`ple.webwork.cache` records `renderer_call` on a miss and `cache_hit` when the
same safe render is replayed; neither event contains protected material.

The shipped local-service contract is owned by
`containers/compose.yaml`, `containers/compose.webwork.yaml`,
`containers/webwork/`, and `launch_local_stack.sh`. Its exact public source
revisions are WebWork2 `c7060fe858cb27b17aad5cf77574ff7d1ae3e1fa` and PG
`726ff42840f968a1d6dfcc270c23c297e1d963f4`. The source stage verifies those
full revisions before the OCI build. The selected Ubuntu, Alpine/git, Node,
and MariaDB inputs are immutable OCI digests with arm64 manifests; the
launcher records the resulting local OCI image ID together with both source
revisions and passes that local identity to the adapter configuration.

`renderer_private` contains only API and WebWork; `webwork_db_private` contains
only WebWork and MariaDB. The renderer and MariaDB have no host-published ports
and never join PLE PostgreSQL, MinIO, gateway, browser, or worker networks.
The launcher atomically creates two different ignored mode-0600 files: the
render-course password and the Mojolicious signing secret. The renderer mounts
both directly; the API receives a one-shot, fixed-UID, read-only runtime copy
of the password through `webwork-api-secret-init`. Every `--with-webwork`
launch refreshes that API copy, so a host-password rotation cannot silently use
an old API secret. Native-only startup does not add the renderer endpoint,
credentials, runtime-secret volume, or initializer dependency.

The direct authenticated `probe_render_rpc.sh` readiness check proves only
that the private upstream service is available. It is not evidence that PLE
can issue, cache, grade, and present a learner attempt. Live container, PLE
integration, and browser-boundary evidence remain pending until their recorded
gates run successfully.

## API and service contracts

The browser signatures in `src/api/client.ts` are the current route-group
contracts. Rust implements authentication, catalog, course, run, asset, and
key-free validation-fallback behavior. `src/api/http_client.ts` is the real
same-origin transport, while mock handlers are bounded test doubles for
server-free browser work. They are not a second public API or a production
fallback.

MOD-API-RUN below distinguishes the implemented compatibility route from the
reserved compact learner cutover. The accepted
[secure grading payload plan](active_plans/decisions/secure_question_grading_payload_plan.md)
defines the atomic replacement: authenticated attempt ID, idempotency key,
presentation consistency values, and the family-minimal answer cross the
learner boundary; response family, scoring, partial credit, and grader state
remain server-derived.

| ID             | Contract source and state                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Owner          | Direct consumers                                                    | Reference/test implementation                       |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------------- | --------------------------------------------------- |
| MOD-API-AUTH   | Implemented provider-neutral route and session contracts: `crates/server/src/auth.rs`, `crates/learning-data-access/src/session.rs`; WP-RC8 owns PLE-managed email/passkey accounts, invite-by-email enrollment, and optional SSO account linking without replacing the opaque session boundary                                                                                                                                                                                                                                                                                                                                                                                                                  | `expert_coder` | MOD-CLIENT, MOD-LTI                                                 | `MemoryStore` and mock auth handler                 |
| MOD-API-CAT    | Authenticated catalog routes include hot-metadata `GET /api/problems/search` and safe exact `GET /api/problems/{problem}/versions/{version}/detail`: `CatalogSearchQuery` is normalized and cursor-bound, page rows and facets share one tenant-visible snapshot, PostgreSQL reads only `problem_version`, and detail excludes source, response, grading, keys, providers, and student statistics. Before MOD-STATS, anonymous statistics are explicitly unavailable/suppressed. Source: `crates/{question_model,learning-data-access,server}/src/catalog.rs`.                                                                                                                                                                    | `expert_coder` | MOD-CLIENT, MOD-UI-BROWSE, MOD-STATS                                | `MemoryStore`, mock handler, opt-in PostgreSQL gate |
| MOD-API-COURSE | Implemented authenticated routes, course-local membership, and instructor-only current item-analysis read: `crates/server/src/course.rs`, `crates/server/src/item_analysis.rs`, `crates/question_model/src/course.rs`, `crates/learning-data-access/src/lib.rs`, `crates/learning-data-access/src/item_analysis.rs`, `src/api/client.ts`, and mock handlers. Item analysis authorizes from the persisted session inside Store and returns only aggregate current-report fields.                                                                                                                                                                                                                                                   | `expert_coder` | MOD-CLIENT, MOD-WORKER                                              | `MemoryStore`, mock handler, opt-in PostgreSQL gate |
| MOD-API-RUN    | Implemented compatibility run, attempt, tagged-response submission, summary, and next-question prefetch routes: `crates/server/src/run.rs`, `crates/learning-data-access/src/lib.rs`, `src/api/client.ts`, and mock handlers. Prefetch reserves a key-free variation without starting a timer; submission atomically promotes it and stores an immutable minimal `nextIssued` receipt link. Replica recovery heals only the sole owned pending predecessor. The internal presentation binding is already issued and persisted. Reserved CON-LEARNER-PAYLOAD-V1 replaces the broad active projection and tagged response wire atomically; it is not silently implied by this route. External-tool projection exposes only a non-secret same-origin session-authenticated attempt route/handle, while provider launch material stays server-held or in an HttpOnly session. | `expert_coder` | MOD-CLIENT, MOD-ADP-IMATHAS                                         | `MemoryStore`, mock handler, opt-in PostgreSQL gate |
| MOD-API-ASSET  | Implemented public-CDN and protected delivery contract: `crates/server/src/asset.rs`, `crates/learning-data-access/src/lib.rs`, `src/api/client.ts`, and the mock asset handler                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | `expert_coder` | MOD-CLIENT, MOD-UI-RENDER                                           | `MemoryStore`, `MemoryObjectStore`, mock handler    |
| MOD-WORKER     | Implemented durable family-filtered queue broker and production drain runtime for current scoring, course item analysis, attempt auto-submit, retention, assignment export, and QTI import: `crates/server/src/worker.rs`, `worker/runtime.rs`, `composition/worker.rs`, the six capability handlers/committers, Store job contracts, and `schemas/migrations/2026080805_operations_analytics.sql`. The registry derives the mandatory broker filter only from complete handler/committer pairs; reserved Render and generic Import variants remain unclaimed. One-job production passes preserve bounded shutdown, while competing processes use `FOR UPDATE SKIP LOCKED`; lease and generation fences make stale work harmless. | `expert_coder` | MOD-API-CAT, MOD-API-COURSE, MOD-API-RUN, MOD-EXPORT, MOD-RETENTION | `MemoryStore`; live PostgreSQL concurrency fixture  |
| MOD-STATS      | Implemented identity-free aggregation and k-anonymous catalog disclosure: `crates/domain/src/statistics.rs`, `crates/learning-data-access/src/statistics.rs`, Memory/PostgreSQL Store projections, and `schemas/migrations/2026080805_operations_analytics.sql`. Exactly-once first-completed-assignment contributions retain shared version aggregates after learner-record purge; safe catalog views disclose only at the deployment-wide floor. Course-local current item analysis is deliberately a separate tenant-owned MOD-WORKER projection, not this non-retractable global aggregate.                                                                                                                                   | `expert_coder` | MOD-API-CAT, MOD-RETENTION                                          | `MemoryStore`; one-time PostgreSQL role gate        |
| MOD-RETENTION  | Implemented configurable course-end notify/archive/delete lifecycle: `crates/server/src/retention.rs`, `retention_worker.rs`, Store retention contracts, and `schemas/migrations/2026080806_retention.sql`. Strong-revision manager actions dispatch only scheduler-bound jobs; archive centrally fences learner access; permanent deletion uses an exact typed-object manifest and lease/generation-fenced relational purge while preserving published content, drafts, and anonymous statistics. Policy: [RETENTION_POLICY.md](RETENTION_POLICY.md).                                                                                                                                                                            | `expert_coder` | MOD-WORKER, MOD-STATS                                               | `MemoryStore`; one-time PostgreSQL/object gates     |

## Browser contracts

| ID               | Contract source and state                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Owner       | Direct consumers                                                                              | Reference/test implementation              |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------- | ------------------------------------------ |
| MOD-CLIENT       | Implemented typed boundary: `src/api/client.ts`, `contracts.ts`, `decoder.ts`, `decoders.ts`, `http_client.ts`, and `crates/server/src/validation.rs`. Prefetch uses a body-free same-origin POST, bounded strict JSON decoding, requested-predecessor binding, and exact submission-successor identity checks; the mock routes and decodes the same wire responses.                                                                                                                                                                                                                                                                                                                                                                                              | `coder`     | MOD-UI-SHELL, MOD-UI-ATTEMPT, MOD-UI-BROWSE, MOD-UI-EDITOR, MOD-UI-GRADEBOOK, MOD-ADP-IMATHAS | `src/api/mock/handlers.ts` and `client.ts` |
| MOD-UI-SHELL     | Frozen route and boundary contract: `src/route_contract.ts`, `routes.ts`, and `app.tsx`, including the honest course-appearance contract surface                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | `architect` | none; browser composition root                                                                | mock handlers and contract pages           |
| MOD-UI-WIDGETS   | Frozen reference signature: `src/components/multiple_choice_response.tsx`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | `coder`     | MOD-UI-RENDER                                                                                 | reference multiple-choice widget           |
| MOD-UI-RENDER    | Implemented browser-safe envelope mapping in `src/components/question_renderer.tsx`, `src/pages/run_page.tsx`, and editor preview surfaces. It renders the closed prompt/response vocabulary and accepts only the component's allowlisted sanitized-markup projection.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | `coder`     | MOD-UI-ATTEMPT, MOD-UI-EDITOR                                                                 | published fixture envelopes                |
| MOD-UI-ATTEMPT   | Implemented attempt loop in `src/pages/run_page.tsx` and `src/features/attempt/`: idempotent submit/retry, server-projected feedback, timer recovery, memory-only exact next-question prefetch, bounded same-origin asset warming, mismatch/outage fallback, and route-teardown abort. A committed receipt is the only authority that activates a prefetched envelope.                                                                                                                                                                                                                                                                                                                                                                                            | `coder`     | MOD-UI-SHELL                                                                                  | mock handlers                              |
| MOD-UI-BROWSE    | Implemented bounded catalog browser: `src/pages/library_route_page.tsx`, `library_page.tsx`, `library_page_model.ts`, and `src/api/catalog_repository.ts`. The route composes the real same-origin catalog repository; mocks remain test doubles.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | `coder`     | MOD-UI-SHELL                                                                                  | mock handlers                              |
| MOD-UI-EDITOR    | Implemented workspace and assignment authoring boundary: `crates/server/src/workspace.rs`, `author_preview.rs`, `course.rs`, `src/pages/editor_*.tsx`, `assignment_editor_*.tsx`, and `src/features/qti_profile_import/`. Authenticated owner/collaborator ACL protects unversioned drafts; strong revision ETags bind save, delete, review, QTI conversion, publication, and explicit author preview. QTI upload/report/review stays same-origin and answer-free, and a committed conversion must refetch the replacement draft before editor handoff. Assignment writes retain their own strong revisions and persisted capability validation. Browser/WASM preview stays key-free, and student routes construct neither workspace nor assignment repositories. | `coder`     | MOD-UI-SHELL                                                                                  | mock handlers                              |
| MOD-UI-GRADEBOOK | Implemented instructor gradebook and bounded run-history projection: `src/pages/gradebook_page.tsx`, `gradebook_page_model.ts`, and `src/api/runtime.tsx`. It loads course-authorized summary rows and cursor-paged history through the same-origin API.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | `coder`     | MOD-UI-SHELL                                                                                  | mock handlers                              |

## Platform contracts

| ID         | Contract source and state                                                                                                                                                                                                             | Owner          | Direct consumers      | Reference/test implementation            |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | --------------------- | ---------------------------------------- |
| MOD-LTI    | Reserved: `crates/server/src/lti.rs`                                                                                                                                                                                                  | `expert_coder` | none; platform edge   | LMS sandbox fixtures                     |
| MOD-DEPLOY | Implemented local deployment contract: `containers/compose.yaml`, `containers/compose.webwork.yaml`, and `launch_local_stack.sh`; production infrastructure under `deploy/` is a reserved boundary and unsupported in this repository | `expert_coder` | none; deployment edge | local stack and focused container checks |

## Shared artifact ownership

These artifacts have one writer. Consumers may read or validate them but must
not create a competing generator or copy.

| Artifact                                              | Owning module |
| ----------------------------------------------------- | ------------- |
| `crates/domain/tests/seed_vectors.json`               | MOD-GEN       |
| `crates/domain/tests/capability_violation_cases.json` | MOD-CAP       |
| `tests/fixtures/published_problem/`                   | MOD-QM        |
| `schemas/migrations/`                                 | MOD-SCHEMA    |
| `tests/test_wasm_export_allowlist.mjs`                | MOD-WASM      |
| `src/api/mock/handlers.ts`                            | MOD-CLIENT    |
| `containers/compose.yaml`                             | MOD-DEPLOY    |

Generated TypeScript under `generated/api/` and `generated/fixtures/` is
derivative. Its Rust model or fixture generator is the contract owner. The
generated output stays ignored and is never edited by a consumer lane.

## Frozen-contract change rule

A frozen contract change must land atomically. The same patch must:

1. update this register;
2. update the owning source contract;
3. update every direct consumer named in that row, including any bounded test double;
4. regenerate derivative types or fixtures through their owning generator;
5. update conformance, secrecy, parity, or browser evidence affected by the
   change; and
6. record the behavior or decision in `docs/CHANGELOG.md`.

A contract change without every consumer is blocking. Do not merge a producer
first and repair consumers in later lane patches. Additive wire changes still
follow this rule because exhaustive TypeScript unions and Rust matches can make
an apparently additive variant a consumer-breaking change.

## Boundary invariants

- Rust modules, functions, and fields use snake case; Rust types and variants
  use upper camel case. Serde converts browser wire fields and discriminants to
  lower camel case. Raw wasm-bindgen snake-case exports stop at
  `src/wasm/index.ts`.
- MOD-GRD is server-only. It may never enter the MOD-WASM dependency closure,
  generated browser types, mock payloads, or client source.
- MOD-STO's typed flat private-integrity check is a server-only exception to
  the usual storage/domain direction; it may not make MOD-GRD, grader pools,
  private bytes, or grading decisions reachable from MOD-WASM. The browser
  closure remains `wasm_bridge`, `domain`, and `question_model` only.
- Published problem versions are shared and immutable. Educational records
  carry direct tenant ownership and cross only a server-authorized boundary.
- `OnRelease` feedback stores only an immutable tenant-owned release decision;
  it never rewrites the first submission receipt or copies private feedback.
  The Store validates direct course-instructor membership. Tenant-administrator
  release requires a later authenticated server-composition boundary.
- List contracts use bounded cursors. No public contract introduces an offset
  or an unbounded list operation.
- A newly issued parameterized attempt receives a fresh server-owned seed.
  Resume, re-render, audit, and debugging of that same attempt reuse its stored
  seed and provenance. API-minted seeds stay within JavaScript's exact integer
  range even though the internal generator accepts the full Rust `u64` domain.
- Successful HTTP bodies enter TypeScript as `unknown` and pass exhaustive
  field decoders before becoming generated browser types. The client uses no
  `any`, unchecked assertion, tenant selector, or cross-origin API base.
