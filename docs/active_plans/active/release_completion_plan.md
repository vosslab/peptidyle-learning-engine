# Plan: Peptidyle Learning Engine release completion

## Status

Planning state: implementation in progress on 2026-08-10. WP-RC1 course appearance and WP-RC2
production-seam closure are accepted; this plan owns all remaining work through the version 1 release. It supplements the architecture in
[implementation_plan.md](../implementation_plan.md) and replaces every unresolved scope question in
that document and its active companion plans. The dependency-ordered next package is WP-RC3,
shipped upstream WeBWorK integration.

Completed packages remain accepted evidence; they are not reopened by this plan. A package below is
complete only when its named production artifacts work, its behavior and security gates pass, its
documentation is current, and an independent reviewer reports no P0/P1 finding. A type declaration,
empty module, mock-only route, disabled test, source-name assertion, or TODO does not satisfy a
deliverable.

Two release boundaries are explicit:

- **Working-codebase release:** all repository-owned code, migrations, tests, documentation,
  containers, and declarative deployment artifacts are complete and reproducible without
  institutional secrets.
- **Production activation:** an operator supplies institutional credentials, applies the checked
  deployment, runs disposable/live gates, completes legal review, and enrolls the human pilot.
  These are external actions with named evidence; they do not hide unfinished repository work.

## Decisions

### In-scope decision ledger

| Topic                 | Decision for version 1                                                                                                                                                                                                                                 | Owning package |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------- |
| Next flat family      | MC is already accepted; MATCH is next, followed by MA, FIB, NUM, ORDER, MULTI-FIB, media, and HOTSPOT                                                                                                                                                  | WP-RC5         |
| Flat source           | Adopt the owner-authored QTI-JSONL v1 contract from QTI Package Maker; do not invent a competing PLE schema                                                                                                                                            | WP-RC4         |
| Grade default         | `highest`                                                                                                                                                                                                                                              | WP-RC0         |
| New-run variation     | `newSeeds`; resuming an issued attempt preserves its seed                                                                                                                                                                                              | WP-RC0         |
| Retention defaults    | Notify at 30 days, archive at 100 days, delete learner records at 365 days, and publish aggregates only at k >= 5                                                                                                                                      | WP-RC0         |
| Course deletion       | Retain assignment definitions by default; delete learner records and student-record objects                                                                                                                                                            | WP-RC0         |
| Operational payloads  | Keep normalized source/public/private payloads in PostgreSQL only within their existing hard ceilings; refuse an oversized write rather than silently moving a hot-path model                                                                          | WP-RC0         |
| WeBWorK source        | Copy the exact licensed user-authored `content/pilot/webwork/which_hydrophobic-simple.pgml` fixture and provenance sidecar into immutable PLE object storage at publication; attempts never depend on a mutable OPL checkout                           | WP-RC3         |
| WeBWorK protocol      | Use authenticated upstream `/render_rpc` form requests for render and grade, project exactly one radio-choice PG control in RC3, use the fixed render-course secret boundary, and build a verified OCI digest from exact unmodified upstream revisions | WP-RC3         |
| Course banner timing  | Candidate expires after 60 minutes; a protected course-banner delivery grant lasts at most 60 minutes and rechecks the exact current pointer                                                                                                           | WP-RC1         |
| QTI profile v1        | Canvas 1.2 and Blackboard 2.1 remain strict static single-choice profiles; unsupported media, feedback, `sub`, and `sup` refuse without loss                                                                                                           | WP-RC6         |
| QTI profile export    | Canvas and Blackboard export run as background jobs and appear in the author UI only as queued status plus a protected download                                                                                                                        | WP-RC6         |
| H5P                   | Serve native H5P only as ungraded practice and import supported static families into the protected native model for grading                                                                                                                            | WP-RC6         |
| Object lifecycle      | Database records define intended existence; bucket inventory proves bytes; reconciliation quarantines twice-observed orphans and alerts on missing referenced bytes                                                                                    | WP-RC7         |
| Production identity   | Implement standards-based institutional OIDC Authorization Code with PKCE behind `IdentityProvider`; map stable issuer/subject pairs to a PLE tenant and user                                                                                          | WP-RC8         |
| LTI                   | Implement LTI 1.3 launch plus Assignment and Grade Services passback as a separate verified credential path                                                                                                                                            | WP-RC9         |
| Infrastructure        | Use OpenTofu in `deploy/opentofu/`; production is AWS Fargate, RDS PostgreSQL, S3, CloudFront, ALB, WAF, KMS, Secrets Manager, and private networking                                                                                                  | WP-RC10        |
| Anonymous traffic     | Ship a static `www` landing origin, same-origin authenticated app/API, aggregate edge metrics, bounded WAF/rate rules, and no client analytics                                                                                                         | WP-RC11        |
| Migration names       | Continue compact ordered names such as `2026080908_secure_question_grading_payloads.sql`; the date and two-digit sequence are the readable ordering contract                                                                                           | WP-P2          |
| Source ownership      | After WP-RC3 and before WP-RC4, extract every maintained source at 1,000 lines or more into capability modules behind stable facades; add a permanent no-exception size gate                                                                           | WP-ARCH1       |
| Course visual default | `grass`, with the accepted 15-theme catalog and one 1200 by 328 WebP center crop                                                                                                                                                                       | WP-RC1         |
| Release content       | Genetics and biochemistry Chapter 1 each ship four questions: WeBWorK MC, WeBWorK MATCH, flat MC, and flat MATCH                                                                                                                                       | WP-RC5         |

`WP-RC0` is a decision-freeze documentation package completed by this plan. It updates the plan,
status, and durable owner guidance; the behavior defaults already present in source remain gates for
later integrated acceptance.

### Out-of-scope decisions

| Excluded from version 1                                                                         | Why version 1 succeeds without it                                                                                                                                                                                                                     |
| ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Content-addressed byte deduplication                                                            | Stable typed keys, checksums, immutable writes, and reconciliation already provide correctness. Deduplication is a storage-cost optimization that can be added behind `ObjectStore` after measured duplication warrants it.                           |
| A TypeScript API server                                                                         | Native `axum` is implemented, tested, and owns the request path. Reopening the runtime would delay release without adding product behavior.                                                                                                           |
| Scored native H5P                                                                               | H5P exposes evaluation to the browser. Server-graded native and imported flat questions supply the secure graded path.                                                                                                                                |
| Passkeys, local passwords, and email-code login                                                 | Institutional OIDC supplies production authentication without storing passwords. A passkey provider can later implement the same `IdentityProvider`; email codes are authentication, not a second factor by themselves.                               |
| Third-party or client-side analytics                                                            | Edge and server aggregate metrics answer cost and reliability questions without adding student tracking or anonymous API work.                                                                                                                        |
| Kubernetes, Redis, Kafka, sharding, a dedicated search service, and multi-region operation      | The target pilot and 10,000-student scale fit stateless replicas, PostgreSQL, object storage, and a worker queue. Every replacement retains a measured trigger in the architecture plan.                                                              |
| Rich media in the accepted vendor-profile v1 import                                             | The strict importer succeeds by refusing unsupported semantics without data loss. QTI-JSONL media and HOTSPOT provide the version 1 rich-media path.                                                                                                  |
| Vendor feedback, `sub`, or `sup` in profile v1                                                  | No accepted fixture establishes a lossless mapping. Refusal preserves correctness; a new profile version can add one exact fixture-backed mapping later.                                                                                              |
| Broad local-corpus compatibility statistics                                                     | Minimized positive and near-miss fixtures plus live profile acceptance prove the claimed contract. Corpus sampling may guide later profile versions but does not widen v1.                                                                            |
| A Rust port of QTI Package Maker                                                                | QTI Package Maker remains the Python interoperability oracle. PLE ports only the versioned parser/compiler behavior needed at runtime.                                                                                                                |
| Actual institutional credentials, FERPA legal certification, or participation by named students | The repository can prove technical controls with synthetic identities and disposable infrastructure. Credential issuance, legal sign-off, and human participation require the institution and are production-activation evidence, not code artifacts. |
| Ten million real production questions or a live 10,000-student cohort                           | Synthetic partition, query-plan, concurrency, queue, and restore gates prove the design before exposing real records. Real growth is an operational outcome, not a prerequisite for a correct release.                                                |
| Learning trees, discussions, clickers, LMS roster sync, research exports, and generated content | Version 1 succeeds as an assignment, question, attempt, feedback, grade, import/export, and retention platform. These features do not block that learning loop.                                                                                       |

## Objectives

- Finish every repository-owned platform behavior promised by the original implementation plan.
- Provide a useful fall-pilot path with two Chapter 1 assignments and no special-case biology logic.
- Preserve server-only grading, immutable publication, forced-RLS tenancy, replica safety, and
  answer-free browser contracts across every new family and integration.
- Make local first success one command, and make production deployment reproducible from checked
  OpenTofu rather than console memory.
- Close each package with behavior evidence, durable documentation, and independent review.

## Scope

The version 1 scope is WP-RC1 through WP-RC12 in dependency order. It includes course appearance,
production-seam cleanup, shipped WeBWorK integration, the owner QTI-JSONL contract, all eight flat
families, pilot content, honest QTI/H5P boundaries, object reconciliation and M5 integration,
institutional OIDC, LTI Advantage, OpenTofu deployment, bot-cost controls, and final release
acceptance.

No package may defer a required file or behavior to a later implementer. If implementation evidence
invalidates a decision, the owner updates this decision ledger, every consumer package, and the
acceptance gate in one reviewed planning patch before code continues.

## Non-goals

The out-of-scope ledger above is exhaustive for known exclusions. An implementer may not create a
new non-goal to avoid an acceptance criterion. New product ideas enter a separate post-v1 plan only
after WP-RC12 closes.

## Architecture and ownership

| Boundary                           | Authoritative owner                                             | Rule                                                                     |
| ---------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Product decisions and defaults     | `docs/HUMAN_GUIDANCE.md` plus this ledger                       | Code may expose configuration but must ship the decided defaults         |
| Public question and response types | `crates/question_model`                                         | Answer-free and generated to TypeScript                                  |
| Source interpretation              | `crates/adapters/native`, `qti`, `webwork`, `h5p`, `imathas`    | Each format has one strict versioned adapter                             |
| Grading                            | `crates/grading` plus injected server-only adapter capabilities | Never in Wasm, generated TS, or browser JSON                             |
| Persistence and RLS                | `crates/learning-data-access` and `schemas/migrations`          | Memory/PostgreSQL parity; PostgreSQL is production authority             |
| Objects                            | `crates/objects`                                                | Typed keys, checksums, role-based delivery, inventory and reconciliation |
| HTTP and workers                   | `crates/server`                                                 | Same-origin, bounded, stateless request handling and durable jobs        |
| Browser                            | `src/`                                                          | Strict decoders, accessible visible flows, no source archive parsing     |
| Local stack                        | `launch_local_stack.sh` and `containers/`                       | One maintained build/start/migrate/seed/wait/open path                   |
| Production deployment              | `deploy/opentofu/`                                              | Declarative, reviewable, drift-detectable, disposable before activation  |

## Dependency map

```text
WP-RC1 course appearance
    |
    v
WP-RC2 production-seam cleanup ---> WP-RC7 M5 reconciliation/integration
    |
    +--> WP-RC3 shipped WeBWorK ---> WP-RC5 pilot content
    |
    +--> WP-RC4 QTI-JSONL contract ---> WP-RC5 eight families ---> WP-RC6 profiles/H5P
                                                        |
                                                        v
WP-RC8 OIDC ---> WP-RC9 LTI ---> WP-RC10 OpenTofu ---> WP-RC11 bot controls
                                                        |
                                                        v
                                                WP-RC12 release acceptance
```

WP-RC4 begins after the owner supplies the normative external artifacts, but that work is an
assigned in-scope package rather than an unowned wait. WP-RC7 may run after WP-RC2 while content
packages proceed, provided the migration owner preserves the reserved ordering below.

## Milestone plan

| Milestone                          | Packages               | Exit condition                                                                         |
| ---------------------------------- | ---------------------- | -------------------------------------------------------------------------------------- |
| RC-A: complete current experience  | WP-RC1                 | Course appearance works end to end and passes visual/live review                       |
| RC-B: close runtime truth gaps     | WP-RC2, WP-RC3         | Production names describe real code and the shipped WeBWorK service renders and grades |
| RC-C: finish content breadth       | WP-RC4, WP-RC5, WP-RC6 | Eight families, Chapter 1 content, QTI export, and honest H5P behavior pass            |
| RC-D: harden data and integration  | WP-RC7                 | Reconciliation and the combined M2-M5 gate pass                                        |
| RC-E: finish production interfaces | WP-RC8, WP-RC9         | OIDC and LTI use verified identities with no browser-trusted grades                    |
| RC-F: deploy and defend            | WP-RC10, WP-RC11       | Disposable AWS deployment, restore, bot-cost, and legitimate-use gates pass            |
| RC-G: release                      | WP-RC12                | Full working-codebase gate and independent release audit pass                          |

## Work packages

### WP-RC1: Complete course appearance

- **Owner:** `ui-ux-engineer` for WP-CA6, then `integrator`; independent HCI, color, PostgreSQL, and
  security review.
- **Status:** accepted on 2026-08-09. The complete implementation and exact executable/visual review
  evidence are in `docs/active_plans/workstreams/course_appearance_implementation.md`; three
  independent read-only reviews reported no P0/P1/P2.
- **Files:** `src/features/course_appearance/`, `src/routes.ts`, `src/api/client.ts`,
  `src/api/contracts.ts`, `src/api/decoders.ts`, `src/api/http_client.ts`, `src/api/runtime.tsx`,
  `tests/playwright/course_appearance.spec.ts`, `tests/e2e/e2e_course_appearance.sh`,
  `docs/active_plans/workstreams/course_appearance_implementation.md`, and the durable architecture,
  contract, file-map, status, and changelog documents named in the course-appearance plan.
- **Behavior:** provide keyboard-complete theme selection, exact wide/narrow previews, decorative or
  informative alternative text, one save action, conflict reload, replacement/removal, responsive
  layouts, and current-only banner delivery. Candidate rows expire 60 minutes after creation;
  protected delivery grants expire within 60 minutes and do not bypass the current-pointer check.
- **Success:** all 15 themes and the Grass default render across course routes without global bleed;
  one banner appears only at course entry; stale, foreign, student-mutation, hostile-image, missing,
  candidate, and superseded cases refuse; independent review reports no P0/P1.
- **Validation:** focused Node and Playwright tests; Rust Store/server suites; disposable PostgreSQL
  and MinIO oracle; 15-theme contact sheet and contrast metrics; `./check_codebase.sh`; built-browser
  Playwright; `source source_me.sh && python3 -m pytest -q tests/`; both diff checks.

### WP-RC2: Remove placeholder production seams

- **Owner:** `rust-code-expert`; documentation owner for maps; independent code reviewer.
- **Status:** accepted on 2026-08-09; exact evidence is in
  `docs/active_plans/workstreams/production_seam_closure.md`.
- **Files:** `crates/adapters/h5p/src/import.rs`, `crates/adapters/qti/src/parser.rs` and
  `parser/tests.rs`, `crates/adapters/webwork/src/renderer_contract.rs`, adapter/server importers,
  `crates/learning-data-access/src/lib.rs`, Memory/PostgreSQL/test Store implementations,
  `crates/server/src/run.rs`, and the named contract/map/status/changelog documents.
- **Behavior:** module names describe their concrete responsibility; catalog resolve/search are
  required Store capabilities; named test adapters retain explicit limited behavior; current feedback
  reads durable release state through the sole `project_feedback` policy.
- **Success:** no maintained production file is empty or named `stub`; no production path relies on
  `todo!`, `unimplemented!`, placeholder return data, or a missing catalog-capability default.
  Explicit mocks remain under test/local-development owners.
- **Validation:** focused adapter/Store/server tests, format, strict workspace Clippy, workspace
  tests, a human-reviewed closure scan, `./check_codebase.sh` (11/11), 1,733 repository Python
  tests, and both diff checks passed; independent review reported no P0/P1.

### WP-RC3: Integrate upstream WeBWorK as shipped

- **Owner:** `rust-code-expert` for the adapter, `integrator` for containers, independent security
  reviewer.
- **Status:** implementation complete; live acceptance is pending on 2026-08-10. The
  decision-complete execution contract is in
  [webwork_shipped_integration.md](../workstreams/webwork_shipped_integration.md).
- **Files:** `crates/adapters/webwork/src/http_renderer.rs`, `renderer_contract.rs`,
  `shipped_render_rpc.rs`, and `sanitizer.rs`; `crates/server/src/webwork_backend.rs`;
  `containers/compose.yaml`, `containers/compose.webwork.yaml`, `containers/env.example`,
  `containers/webwork/Containerfile`, `containers/webwork/entrypoint.sh`,
  `containers/webwork/probe_render_rpc.sh`, `containers/webwork/webwork2.mojolicious.yml`,
  `containers/webwork/site.conf`, `containers/webwork/course.conf`, and
  `containers/webwork/init_render_course.sh` for exact-source OCI build, private MariaDB, and
  render-course configuration; `launch_local_stack.sh`; the immutable fixture and provenance under
  `content/pilot/webwork/`; `crates/project-tools/src/e2e_seed.rs`;
  `tests/e2e/e2e_webwork_api_secret_mode.sh`, `tests/e2e/e2e_webwork_render_rpc.sh`,
  `tests/playwright/webwork_live_config.ts`, `tests/playwright/webwork_run.spec.ts`, and
  `tests/test_webwork_renderer_container.py`; container, contract, architecture, status, and
  changelog docs.
- **Behavior:** call private upstream `/render_rpc` with server-owned render-course credentials,
  base64 PG source, immutable PG path/version, seed, and bounded form fields. The same upstream route
  renders and grades. RC3 translates exactly one radio-choice PG control to an answer-free PLE
  single-choice envelope, sanitizes browser markup, re-renders to reconstruct the private field/value
  mapping for grade, and translates the server-side score to `GradeOutcome`. Reject answer hashes,
  correct answers, credentials, unexpected protected fields/scripts/resources, redirects, oversized
  bodies, identity/version drift, malformed output, and unsupported controls. One question outage does
  not stop native questions or API health.
- **Success:** `./launch_local_stack.sh --with-webwork` builds/starts a private WeBWorK plus MariaDB
  profile from exact unmodified upstream revisions, verifies the resulting OCI digest, renders the exact
  immutable licensed `content/pilot/webwork/which_hydrophobic-simple.pgml` RadioButtons fixture twice
  with the same seed and stable result, grades correct and incorrect submissions, proves a cache hit,
  and proves no protected material crosses the browser network trace. This bounded fixture proves the
  shipped upstream protocol and PLE translation with owner-controlled license/provenance; broad OPL
  corpus compatibility remains outside RC3.
- **Validation:** adapter contract tests with recorded redacted upstream responses; private-container
  live gate; outage/timeout/size/redirect/identity tests; browser render/grade test; full repository
  and container gates.

### WP-RC4: Freeze and adopt QTI-JSONL v1

- **Owner:** product owner in QTI Package Maker for the source contract; `rust-code-expert` in PLE for
  strict adoption; independent contract reviewer.
- **External files:** `OTHER_REPOS/qti-package-maker/docs/QTI_JSONL_SPEC.md`,
  `qti_package_maker/engines/qti_jsonl/__init__.py`, `engine_class.py`, `write_item.py`, and
  `read_item.py`; `examples/qti_jsonl/all_question_types.jsonl`;
  `tests/unit/test_qti_jsonl_contract.py`; `tests/integration/test_qti_jsonl_roundtrip.py`; QTI
  Package Maker architecture, engine, file-map, usage, and changelog docs.
- **PLE files:** `crates/adapters/native/src/qti_jsonl/mod.rs`, `schema.rs`, `parser.rs`,
  `compiler.rs`, `media.rs`, and `families/{single_choice,multiple_answer,fill_in,multi_fill_in,numeric,matching,ordered,hotspot}.rs`;
  normative valid/invalid `.jsonl` inputs under `crates/adapters/native/tests/qti_jsonl/`; contract,
  object-format, architecture, file-map, and changelog docs.
- **Behavior:** the external spec gives every record a named spec/version and family, lossless answer
  semantics, optional overall feedback, media references, HOTSPOT geometry and accessible fallback,
  strict validation, and deterministic JSONL framing. PLE accepts only the named version, rejects
  duplicate members/unknown versions/invalid family data, preserves exact source, derives stable PLE
  identities when absent, and compiles answer-free public plus grader-only private values.
- **Success:** all eight valid reference records round-trip through QTI Package Maker; every invalid
  boundary refuses; PLE compiles the same records deterministically without redefining fields; v1
  `singleChoice` bytes remain unchanged. This request explicitly authorizes the bounded normative
  QTI-JSONL fixture directories needed for cross-project contract evidence.
- **Validation:** QTI Package Maker unit/integration gates; PLE adapter/compiler tests; historical v1
  regression; secret-free Wasm/generated/browser scans; independent cross-repository contract review.

### WP-RC5: Implement all flat families and Chapter 1 content

- **Owner:** one `rust-code-expert` per family in strict sequence, `solid-js-expert` for author/learner
  widgets, `bptools-writer-expert` for source review, and an integration owner for pilot content.
- **Prerequisite contract:** implement and accept the atomic learner render/response boundary in
  `docs/active_plans/decisions/secure_question_grading_payload_plan.md`
  after WP-RC3 live acceptance and before the first WP-RC5 family. It makes `QuestionAttemptId`,
  authenticated session, and `Idempotency-Key` the only submission authority; it does not delay
  current WP-RC3 live acceptance.
- **Files:** `crates/question_model/src/response.rs` and `envelope.rs`;
  `crates/domain/src/flat_response_validation.rs`; `crates/grading/src/flat_question.rs`;
  the WP-RC4 `crates/adapters/native/src/qti_jsonl/` family modules;
  `crates/learning-data-access/src/flat_question.rs`; `crates/server/src/flat_question_publication.rs`;
  generated contracts; `src/features/flat_question_authoring/`;
  `src/components/responses/{multiple_answer,fill_in,multi_fill_in,numeric,matching,ordered,hotspot}.tsx`;
  `src/components/response_widget.tsx`; behavior tests beside each owner;
  `content/pilot/chapter_1_assignments.yaml` and copied, licensed source
  under `content/pilot/sources/`; `crates/project-tools/src/pilot_content.rs`;
  `tests/e2e/e2e_pilot_content.sh`; `docs/PILOT_CONTENT.md`.
- **WeBWorK MATCH files:** before accepting Chapter 1 content, extend
  `crates/adapters/webwork/src/shipped_render_rpc.rs`, `renderer_contract.rs`, their contract tests,
  server translation tests, and `tests/playwright/webwork_run.spec.ts` with a real matching render and
  grade path.
- **Behavior:** implement MATCH first, then MA, FIB, NUM, ORDER, MULTI-FIB, media, and HOTSPOT. Each
  family supports strict parse, author edit/preview, CAS save, immutable publication, issue, accessible
  keyboard response, server grading, optional feedback, summary, retention, and cleanup. WP-RC5's
  first task extracts and grades a typed WeBWorK MATCH projection through the named adapter, contract,
  server, and browser-live owners before the Chapter 1 matching source is accepted. HOTSPOT has a
  keyboard/list alternative and scale-independent normalized coordinates.
- **Pilot inputs:** Genetics uses `genetic_disorders-which_one.pgml`,
  `genetic_disorders-matching.pgml`, `bbq-WOMC-genetic_disorders-questions.txt`, and
  `bbq-MATCH-genetic_disorders-questions.txt`. Biochemistry uses
  `biochemical_functional_groups-which_one.pgml`, `biochemical_functional_groups-matching.pgml`,
  `bbq-WOMC-biochemical_functional_groups-questions.txt`, and
  `bbq-MATCH-biochemical_functional_groups-questions.txt` from the local biology-problems project.
- **Success:** every family passes one complete Memory/PostgreSQL author-to-learner path; the two
  Chapter 1 assignments each publish exactly four reviewed questions with immutable source,
  license/provenance, correct/incorrect grading, and no answer-bearing learner payload.
- **Validation:** family gates from the flat-family plan; disposable RLS/object acceptance; keyboard,
  screen-reader semantics, 320 px, zoom, and built-browser Playwright; pilot import/publish/grade
  script; full repository gate and independent family plus content reviews.

### WP-RC6: Close QTI export and H5P capability claims

- **Owner:** separate Canvas and Blackboard exporter owners; H5P adapter owner; worker/UI integrator.
- **Files:** `crates/adapters/qti/src/export_profiles/{mod,canvas,blackboard,archive}.rs`;
  `crates/adapters/qti/tests/profile_export.rs`; `crates/server/src/qti_export.rs` and worker tests;
  `src/features/qti_profile_export/`; `crates/adapters/h5p/src/import.rs`; H5P tests;
  `crates/learning-data-access/src/lib.rs`, `in_memory/exports.rs`, and `postgres/exports.rs`;
  `crates/objects/src/lib.rs` and `bucket.rs`; generated contracts; Playwright; contracts,
  architecture, usage, and changelog docs.
- **Behavior:** background profile export snapshots an immutable supported problem version, refuses
  unsupported fields before object creation, emits a protected requester-owned artifact, and exposes
  queued/running/failed/download states without revealing answers. H5P declares ungraded practice
  honestly and imports only the source families it can map losslessly into the protected native
  compiler.
- **Success:** supported Canvas and Blackboard subsets re-import with equal prompt, choice order,
  correct binding, points, and stable PLE identity; every unsupported semantic refuses without an
  artifact. H5P never claims server grading for a browser-evaluated package.
- **Validation:** exporter semantic round trips and refusal matrix; worker replay/concurrency/object
  authorization tests; H5P capability/import tests; visible author export flow; full package gate and
  independent review.

### WP-RC7: Reconcile objects and close M2 through M5

- **Owner:** `postgresql-expert` and object-store `rust-code-expert`, followed by integration and
  security reviewers.
- **Files:** `crates/objects/src/inventory.rs`; `crates/learning-data-access/src/object_reconciliation.rs`
  plus Memory/PostgreSQL owners; `schemas/migrations/2026080909_object_reconciliation.sql`;
  `crates/server/src/object_reconciliation_worker.rs`; `tests/e2e/e2e_object_reconciliation.sh`;
  `tests/e2e/e2e_release_integration.sh`; object, retention, security, architecture, status, and
  changelog docs.
- **Behavior:** page through bounded bucket inventory; register every deterministic render/cache
  object; mark first-observed unreferenced bytes; quarantine and delete only after a later inventory
  still finds them beyond injected policy; cancel deletion when a reference appears; alert and
  quarantine delivery when a database record points to missing or mismatched bytes; never delete a
  database record to hide a broken reference; make every pass idempotent and tenant-safe.
- **Success:** memory, PostgreSQL, and MinIO agree on exact/replayed/orphan/missing/mismatch cases;
  concurrent creation cannot be swept; all M2-M5 exit criteria run together, including hostile
  import, answer-key denial, foreign tenant, partition pruning, renderer outage, below-k statistics,
  archive/delete, replica, and worker recovery.
- **Validation:** Store/object conformance; fresh/no-op forward migration; disposable PostgreSQL and
  MinIO lifecycle oracle; multi-replica/worker soak; combined E2E; full repository gate; independent
  data-security and milestone reviews.

### WP-RC8: Implement institutional OIDC

- **Owner:** authentication `rust-code-expert`, PostgreSQL owner, UI owner, and independent security
  reviewer.
- **Files:** `crates/server/src/auth/oidc.rs`; `crates/learning-data-access/src/external_identity.rs`
  plus Memory/PostgreSQL owners; `schemas/migrations/2026080910_oidc_identity.sql`;
  `src/pages/sign_in_page.tsx`; `src/auth/session_context.tsx`; `tests/e2e/e2e_oidc_login.sh`;
  `tests/playwright/oidc_login.spec.ts`; auth/security/deployment/usage docs and changelog.
- **Behavior:** use Authorization Code with PKCE, discovery, exact issuer allowlist, state, nonce,
  redirect URI, signature, issuer, audience, expiry, issued-at, and authorized-party validation.
  Bind `(issuer, subject)` to one tenant/user through an administrator-managed mapping; never trust
  email alone for tenancy. Mint only the existing hashed opaque database session; rotate/revoke on
  logout or mapping disable; keep the cookie host-only, secure, HTTP-only, and same-site appropriate.
- **Success:** login on one replica works on another; replay, mix-up, stale code, wrong issuer,
  unmapped subject, disabled mapping, CSRF, open redirect, and provider outage fail safely with no
  session; logs and browser state contain no token or secret.
- **Validation:** provider-neutral unit tests with local signed keys; Store conformance and RLS;
  disposable standards-compliant OIDC provider E2E; multi-replica Playwright; security scan; full
  repository gate and independent auth review.

### WP-RC9: Implement LTI Advantage launch and grade passback

- **Owner:** LTI `rust-code-expert`, Store/PostgreSQL owner, LMS integration reviewer.
- **Files:** `crates/server/src/lti.rs`, `lti_launch.rs`, and `lti_ags.rs`;
  `crates/learning-data-access/src/lti.rs` plus backends;
  `schemas/migrations/2026080911_lti_advantage.sql`; `src/pages/lti_launch_page.tsx`;
  `tests/e2e/e2e_lti_advantage.sh`; LTI contract/security/usage/deployment docs and changelog.
- **Behavior:** validate OIDC login initiation and LTI 1.3 launch state/nonce, issuer, deployment,
  client, signature, audience, message type, roles, resource link, and deep-link/assignment binding.
  Store platform credentials server-side; send idempotent AGS scores derived only from
  `student_assignment_summary`; retry with bounded durable jobs; browser messages never become grades.
- **Success:** one LMS sandbox launch maps to the intended tenant/course/assignment and one score
  passback is exact under replay; forged/cross-tenant/stale/mismatched launches and grade URLs refuse;
  provider outage queues bounded retry without blocking native assignment use.
- **Validation:** signed protocol fixtures generated in tests, not captured secrets; Store/RLS tests;
  disposable LMS sandbox or standards harness; replay/cross-tenant/outage tests; full gate and
  independent protocol/security review.

### WP-RC10: Add declarative AWS deployment

- **Owner:** M6 deployment architect and `postgresql-expert`; independent operations/security review.
- **Files:** `deploy/opentofu/{versions,providers,variables,locals,network,database,storage,compute,edge,waf,observability,outputs}.tf`;
  `deploy/opentofu/env.example.tfvars`; `deploy/opentofu/tests/policy.tftest.hcl`;
  `devel/deploy_disposable.sh`; `devel/rehearse_restore.sh`; deployment, container, security,
  backup/restore, operations, and cost docs; status and changelog.
- **Behavior:** create private subnets, encrypted RDS with PITR, three private encrypted buckets and
  lifecycles, ECR/Fargate API/worker/renderer, ALB private origin, CloudFront `www` and `app`, WAF,
  KMS, Secrets Manager references, least IAM, logs/metrics/alarms, autoscaling ceilings, and immutable
  deployment/rollback manifests. Remote state is encrypted/restricted and no secret enters plan,
  output, image, repository, or browser.
- **Success:** a disposable environment plans from an empty account boundary, deploys, migrates,
  passes semantic health and one complete assignment, restores from backup, rolls back the app/static
  manifests, detects drift, and destroys only resources tagged with its unique deployment ID.
- **Validation:** `tofu fmt -check -recursive`, `tofu init -backend=false`, `tofu validate`, `tofu test`;
  secret/config policy scan; disposable plan/apply/health/restore/rollback/drift/destroy rehearsal;
  independent cost, security, database, and operations reviews.

### WP-RC11: Implement bot-cost controls

- **Owner:** the owners assigned in
  [bot_traffic_cost_reduction_plan.md](bot_traffic_cost_reduction_plan.md), with M6 architect
  integration.
- **Files:** `landing/index.html`, `landing/style.css`, `pipeline/build_landing.mjs`,
  `crates/server/src/request_cost.rs`, the existing server auth/router
  owners, `deploy/opentofu/edge.tf`, `waf.tf`, `observability.tf`, and
  `deploy/opentofu/tests/policy.tftest.hcl`; `tests/test_landing_artifact.mjs`;
  `tests/playwright/landing.spec.ts`; `tests/e2e/e2e_bot_cost.sh`;
  `docs/OPERATIONS_BOT_TRAFFIC.md`; and
  `docs/active_plans/workstreams/bot_traffic_cost_acceptance.md`.
- **Behavior:** anonymous landing traffic terminates at static edge storage; missing/malformed session
  input causes zero Store calls; random valid-format unknown tokens cause at most one indexed lookup;
  direct origins remain private; WAF rules start in count mode and preserve class-start/shared-egress,
  VPN/datacenter, international, IPv4/IPv6, keyboard, and screen-reader use; emergency modes expire or
  have one documented recovery action.
- **Success:** cold/warm anonymous load causes no PLE database, object-signing, queue, renderer, or
  grading work; normalized cost and legitimate-failure evidence are recorded; no client analytics or
  educational-record join is introduced; independent reviewers report no P0/P1.
- **Validation:** permanent offline policy/browser tests, disposable crawler and class-start replay,
  origin/cache-poison matrix, alarm injection, emergency rehearsal, before/after cost report, and
  the bot plan's complete gate.

### WP-ARCH1: Enforce capability-sized source ownership

- **Owner:** four independent expert owners for persistence, server, adapters/tooling, and browser;
  one integration owner; separate PostgreSQL, security, TypeScript/HCI, and architecture reviewers.
- **Depends on:** accepted WP-RC3; it completes before WP-RC4 so later payload and family behavior
  lands in focused owners rather than the existing oversized facades.
- **Files:** exact current inventory, module destinations, ownership, dependencies, behavior gates,
  and closure artifacts are in `docs/active_plans/active/source_module_decomposition_plan.md`; the permanent policy is
  `tests/test_source_file_size.py`.
- **Behavior:** move complete capabilities behind stable public facades; preserve routes, wire,
  schema/SQL behavior, generated contracts, grading/security boundaries, CLI output, and browser
  behavior. Split tests by behavior. Scan maintained source roots with no filename exception list.
- **Success:** every maintained source file is at most 999 physical lines; the permanent test proves
  999 passes and 1,000 fails; all focused and integrated behavior gates pass; documentation maps the
  new owners; independent reviewers report no P0/P1.
- **Validation:** per-lane Rust/Store/protocol/TypeScript/browser gates; `./check_codebase.sh`; full
  repository pytest; generated-contract checks; final line inventory; Markdown/ASCII/diff checks;
  independent multi-discipline review.

### WP-RC12: Release acceptance and documentation closure

- **Depends on:** WP-ARCH1 and WP-RC1 through WP-RC11.
- **Owner:** release integrator; separate code, security, database, accessibility, operations, and
  documentation reviewers.
- **Files:** `tests/e2e/e2e_release_candidate.sh`; `docs/RELEASE_EVIDENCE.md`; `README.md`;
  `docs/INSTALL.md`, `USAGE.md`, `CONTAINER.md`, `SECURITY_MODEL.md`, `DATABASE_STRUCTURE.md`,
  `CODE_ARCHITECTURE.md`, `FILE_STRUCTURE.md`, `CONTRACTS.md`, `RETENTION_POLICY.md`;
  implementation status/report, changelog, and release notes.
- **Behavior:** provide one repeatable local path and one disposable production path from clean clone
  through build, migrate, seed/configure identity, author, publish, assign, issue, answer, grade,
  feedback, export, retain/delete, backup, restore, and rollback. Record every environment-derived
  dependency and every out-of-scope production-activation action plainly.
- **Success:** no empty maintained file, production stub, TODO-only section, fake/disabled acceptance
  test, unresolved scope question, answer leak, cross-tenant path, silent format loss, or undocumented
  operator step remains. All claimed capabilities have current evidence and all reviewers report no
  P0/P1.
- **Validation:** `./check_codebase.sh`; strict Rust/TypeScript/Node/Python gates; built Playwright;
  combined local and disposable-cloud E2E; migration/restore/replica/worker/load/security/a11y gates;
  link/ASCII/diff checks; independent multi-discipline audit.

## Acceptance criteria and gates

- **Binary scope gate:** every known uncertainty is present in one of the two decision ledgers. No
  active plan ends with an open-question or deferred-decision section.
- **Artifact gate:** every in-scope package names owners and real production, test, documentation, and
  evidence files; generated outputs are validated through their owning generator.
- **Behavior gate:** mocks may support focused tests but cannot be the only acceptance evidence for a
  production route, Store, object, identity, worker, or deployment path.
- **Security gate:** answers, keys, provider tokens, object credentials, and tenant selection remain
  server-owned; forced RLS, least grants, non-enumeration, bounded input, and malicious-upload gates
  remain green.
- **Accessibility gate:** all student actions are keyboard complete; author flows have visible focus,
  status, recovery, zoom/reflow, and semantic labels; HOTSPOT has a non-pointer alternative.
- **Data gate:** migrations are forward-only after `2026080907_course_appearance.sql`; each migration
  passes fresh, no-op, checksum, RLS/grant, and live behavior evidence.
- **Review gate:** the implementer does not self-accept. Each package requires an independent no-P0/P1
  review after executable gates pass.

## Test and verification strategy

Run the narrowest owner test first, then the package gate, then the release gate. Permanent tests
assert stable behavior and security boundaries. Disposable checks own cloud credentials, upstream
services, large data, restore timing, cost, and visual artifacts. Scratch probes and deliberate
mutations are removed after their conclusions are recorded; no source-string or exact-count test is
kept merely to memorialize implementation details.

The release gate is:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
npx tsc --noEmit -p tsconfig.json
node --import tsx --test tests/test_*.mjs
./check_codebase.sh
bash run_playwright_tests.sh --build
bash tests/e2e/e2e_release_candidate.sh
source source_me.sh && python3 -m pytest -q tests/
git diff --check
git diff --cached --check
```

Environment-backed commands report `SKIP` with the missing named prerequisite only during ordinary
development. They must pass, not skip, in WP-RC12 release evidence.

## Migration and compatibility policy

- Preserve the accepted six-file baseline and `2026080907_course_appearance.sql`.
- Reserve `2026080908_secure_question_grading_payloads.sql`,
  `2026080909_object_reconciliation.sql`, `2026080910_oidc_identity.sql`, and
  `2026080911_lti_advantage.sql` in that order. WP-RC7 schema work begins after WP-P2, while its
  non-schema object inventory work may run in parallel. A package needing another migration takes
  the next two-digit daily sequence; it does not insert or rename an accepted version.
- QTI-JSONL adapter/spec identity lives inside the existing versioned source payload and immutable
  object/checksum binding; no family-shaped table is added.
- Existing flat v1, generic QTI, Canvas/Blackboard profile v1, published problem versions, and
  historical attempts remain readable. Incompatible source/profile changes add a named reader or a
  new version and never reinterpret immutable history.

## Risk register

| Risk                                                     | Owner                | Control and trigger                                                                                  |
| -------------------------------------------------------- | -------------------- | ---------------------------------------------------------------------------------------------------- |
| Plan completeness becomes documentation-only             | Release integrator   | Package acceptance requires working artifacts and behavior evidence; mocks/docs alone fail           |
| External QTI-JSONL work stalls PLE                       | Product owner        | WP-RC4 has exact external artifacts and tests; course, seam, WeBWorK, and M5 work remain independent |
| Shipped WeBWorK output leaks keys or unsafe markup       | WeBWorK owner        | Strict result translator, sanitizer, response scan, private network, and real browser trace          |
| New family leaks an answer through generated types       | Family owner         | Public/private compiler split, Wasm dependency gate, DTO scan, server-only grader                    |
| Reconciliation deletes a concurrent valid object         | Object owner         | Two observations, quarantine, reference recheck, idempotency, concurrent-creation oracle             |
| OIDC email or callback selects a tenant                  | Auth owner           | Exact issuer/subject mapping, state/nonce/PKCE, fixed redirect, no email tenancy                     |
| LTI browser input becomes a grade                        | LTI owner            | AGS derives from summary rows; signed launch and server credentials; no browser grade authority      |
| Cloud plan contains a secret or destructive broad target | Deployment architect | Secret references only, unique deployment tags, reviewed plan, bounded destroy rehearsal             |
| Bot rules block a shared campus or assistive user        | Edge owner           | Count mode, versioned legitimate corpus, accessible recovery, immediate rollback                     |
| Real pilot begins before external activation evidence    | Product owner        | Production activation checklist is separate and must be signed before enrollment                     |

## Rollout and release checklist

- [x] WP-RC1 course appearance accepted.
- [x] WP-RC2 production seams accepted.
- [ ] WP-RC3 shipped WeBWorK accepted.
- [ ] WP-RC4 QTI-JSONL contract accepted in both repositories.
- [ ] WP-RC5 eight families and Chapter 1 content accepted.
- [ ] WP-RC6 QTI export and H5P claims accepted.
- [ ] WP-RC7 M2-M5 reconciliation/integration accepted.
- [ ] WP-RC8 OIDC accepted.
- [ ] WP-RC9 LTI Advantage accepted.
- [ ] WP-RC10 OpenTofu disposable deployment/restore accepted.
- [ ] WP-RC11 bot-cost controls accepted.
- [ ] WP-RC12 release evidence and independent audits accepted.
- [ ] Production operator supplies credentials and records issuer/client/deployment identities.
- [ ] Institutional owner completes FERPA/legal/security sign-off.
- [ ] Human fall-pilot accessibility and teaching walkthrough passes before student enrollment.

## Documentation close-out requirements

Every package updates the changelog only after executable acceptance. A package that changes an
owner boundary also updates contracts, code architecture, and file structure. A package that changes
operator or user behavior updates install, usage, container/deployment, and security documentation.
WP-RC12 replaces status prose with exact evidence, archives completed active companion plans using
`git mv`, and leaves this plan open only for external production-activation checklist results.

## Patch plan and reporting format

Land one work package per reviewable patch in WP-RC order. A package may use separately reviewed
subpatches only where its file ownership table permits; the package is not accepted until the
integrated behavior passes. Each handoff reports:

- owner and work-package ID;
- exact files changed and index state;
- user-visible and security behavior completed;
- focused, package, and release commands with results;
- one-time evidence and artifact paths;
- out-of-scope decisions relied upon; and
- independent findings and their disposition.
