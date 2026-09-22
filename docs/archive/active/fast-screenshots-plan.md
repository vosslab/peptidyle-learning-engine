# Plan: Fast GUI-change to screenshot loop

## Context

Evaluating a GUI change costs a full stack cycle today. `devel/capture_screenshots.sh` starts or
reuses the Live Demo, then `tests/playwright/screenshot_corpus/cli.ts` captures and publishes the
corpus. The user reports that getting services up is the expensive part, not taking pictures, so
this plan keeps the stack warm and rebuilds only what the change requires.

Two repository facts make that possible. `containers/compose.yaml:599-604` bind-mounts host
`../dist` read-only into the Caddy gateway, so a rebuilt browser bundle is served with no image
build, no container replacement, and no database reseed. `pipeline/build_wasm.sh` writes
`dist_wasm/web/`, which `pipeline/build.mjs:132-152` copies into `dist/wasm/`, so WASM is browser
code and a `crates/wasm` change reaches the running stack through the same bind mount.

A second, separable problem surfaced during planning: adding one screenshot currently takes four
hand edits across two files, because capture membership is declared in the manifest, in a
scenario's `checkpoints` list, in the scenario body, and in a coverage ledger. PLE is
pre-production, so this plan fixes that ownership instead of working around it.

## Objectives

- Reach a published, verified screenshot corpus from a TypeScript or WASM edit without any image
  build, container replacement, or database reseed.
- Pick up a server-Rust edit by rebuilding only the application services, leaving the database,
  object store, and renderer running.
- Keep a cold start working unchanged, building exactly once.
- Make adding or removing a screenshot one hand-authored edit in one scenario file.
- Decide the design from measured timings rather than assumption.

## Design philosophy

The trade-off this plan accepts: the driver decides what to rebuild from **file timestamps against
build outputs**, which is coarse and occasionally rebuilds more than strictly necessary, in
exchange for having no change-tracking state, no Git dependency, and no dependency graph to
maintain. Rejected alternative: deriving the rebuild set from `cargo metadata` dependency closures.
That is more precise only for `crates/domain` and `crates/question_model`, which compile into both
the browser bundle and the api binary, and for those the conservative answer is the correct one
anyway. This is **Keep It Simple, Stupid** and **complexity must earn its place**
(`docs/REPO_STYLE.md`, `docs/HUMAN_GUIDANCE.md:50-52`).

The second trade-off: `docs/screenshots/current_capture_manifest.json` becomes generated output
rather than an authored contract. It stays committed and still feeds the receipt digest, so
evidence is unchanged, but its authority moves to the scenario that captures the image. This is
**fix the design, not the symptom** applied with the pre-production freedom
`docs/HUMAN_GUIDANCE.md:85-88` grants.

Evidence strategy for uncertain methods: M0 measures three end-to-end warm loops and every
component before any implementation, and its recorded decision may fold or delete later
milestones. No milestone below survives a measurement that says it saves nothing.

## Scope

- Measure the current loop end to end and per component, and record the decision that follows.
- Replace the body of `devel/capture_screenshots.sh` with a Python driver behind the same command.
- Decide what to build from source timestamps against build outputs and two stack reference times.
- Run only the needed build: client bundle, wasm plus client, or the full `./build.sh`.
- Add `local_stack.py rebuild-application` to move the application service group onto a new image
  without touching stateful services.
- Reuse a running Live Demo, start one when absent, and never build twice in either path.
- Report which tracked screenshots changed during a run.
- Move capture metadata into the scenario declarations and generate the manifest, coverage
  ledgers, and gallery ordering from them.
- Update the screenshot, development, and agent documentation, and the changelog.

## Non-goals

- Skip capture selection and subset publishing; every run captures and publishes the whole corpus.
- Skip parallel scenario execution; mutating scenarios stay serial.
- Skip watch mode and any HMR development server.
- Skip dependency-graph analysis of the Cargo workspace.
- Skip schema hot-apply; a `schemas/` change restarts the stack and reinstalls the database.
- Skip changes to privacy profiles, route policy, or what any scenario navigates.

## Current state summary

| Concern | Where it lives | Problem |
| --- | --- | --- |
| Capture entry point | `devel/capture_screenshots.sh` | Always full build via the launcher; no notion of what changed |
| Stack lifecycle | `launchers/run_live_demo.sh`, `local_stack_control/lifecycle.py:362-461` | Reuse works and is cheap; the only rebuild path is a full start, and `restart` explicitly refuses to build (`lifecycle.py:481`) |
| Browser delivery | `containers/compose.yaml:599-604` | Already a bind mount; nothing consumes that fact |
| Capture membership | manifest, `scenarios_*.ts` checkpoints list, scenario body, coverage ledgers | Four hand edits per screenshot |
| Derived-but-declared data | `routeId` (recomputed at `runtime.ts:61-70`), `role`, `gallery.order` | Declarations that can drift from reality |
| Generated artifacts | receipt, `docs/SCREENSHOT_ATLAS.md` | Already generated; manifest is the outlier |

## Architecture boundaries and ownership

- **Driver boundary** (`devel/capture_screenshots.py`, `devel/change_scope.py`): decides and
  orchestrates. Holds no capture ids, routes, roles, or image paths, and never reads the manifest.
- **Stack boundary** (`local_stack_control/`): owns containers, images, and readiness. The driver
  calls it and never runs `podman compose` itself.
- **Corpus boundary** (`tests/playwright/screenshot_corpus/`): owns what a screenshot is, what it
  asserts, and what gets published. The driver's only contact with the corpus is hashing the PNGs
  in the four role folders before and after a run; the generated manifest, receipt, atlas, and
  coverage exceptions belong to this boundary and the driver does not read them.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M0, M7 / WS-EVIDENCE | `tests/_temp/`, `docs/active_plans/reports/` | Temporary; deleted at M7 and M13 |
| M1, M3, M5 / WS-LOOP | `devel/capture_screenshots.py`, `devel/capture_screenshots.sh` | Driver boundary |
| M2 / WS-LOOP | `devel/change_scope.py` | Driver boundary, pure functions |
| M4 / WS-LOOP | `local_stack_control/cli.py`, `commands.py`, `lifecycle.py` | Stack boundary |
| M6 / WS-LOOP | `tests/e2e/e2e_screenshot_warm_loop.sh` | Permanent E2E |
| M8-M12 / WS-CORPUS | `tests/playwright/screenshot_corpus/*`, `scenarios_*.ts`, `docs/screenshots/` | Corpus boundary |
| M13 / WS-CORPUS | `tests/_temp/` scratch | Temporary acceptance evidence |
| M14 / WS-DOCS | `docs/`, `AGENTS.md` | Documentation |

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M0 | Measure the loop that exists | Time three warm loops and every component | Decide from numbers which milestones survive |
| M1 | Python driver replaces the shell body | Port `capture_screenshots.sh` behavior into Python | Same behavior, one place to extend |
| M2 | Decide what to build | Timestamp comparison to build and containers | One honest decision function |
| M3 | Run the build | Execute exactly the chosen build | No wasted stages |
| M4 | Rebuild the application services | New controller operation for the api image group | Server change without losing data |
| M5 | Wire the driver, warm and cold | Order stack, decide, build, containers, capture | One command for every case |
| M6 | Permanent warm-loop contract | E2E: stale bundle is rebuilt and served | The durable behavior is protected |
| M7 | Prove the other build paths, clean up | WASM and server edits once, then delete scratch | Evidence without permanent cost |
| M8 | Capture metadata into scenarios | Checkpoint declares its own identity | One definition per screenshot |
| M9 | Route id observed, not declared | Record the reached route, keep the policy | Generation has a route to record |
| M10 | Manifest becomes generated | Publish writes it, verify checks drift | Manifest stops being authored |
| M11 | Coverage is computed | Ledgers from routes and Ribbon destinations | Only deliberate reasons stay hand-kept |
| M12 | Gallery order derived | Ordering from grouping and declaration order | No unique integers by hand |
| M13 | Add and remove a screenshot | Acceptance test of the new ownership | One hand edit, proven |
| M14 | Docs and changelog | Screenshot, development, agent docs | Documented for the next person |

### Milestone: M0 measure the loop that exists

- Depends on: none.
- Deliverables: WP-E1.
- Done checks: `bash tests/_temp/measure_screenshot_loop.sh` exits 0; the report carries a number
  and a command for every measured item, and the written decision.
- Entry criteria: a working Podman machine and a buildable tree.
- Exit criteria: the report states whether build work, stack work, or capture dominates each of the
  three warm loops. If capture dominates the warm TypeScript loop, stop and revise before M1. If
  the `crates/wasm` build is indistinguishable from `./build.sh --debug`, fold that row into the
  full build in M2 and say so in the report.
- Parallel-plan ready: no. Every later milestone may be pruned by this result.

### Milestone: M1 Python driver replaces the shell body

- Depends on: M0 (its decision may remove flags before they are ported).
- Deliverables: WP-L1.
- Done checks: `./devel/capture_screenshots.sh --verify` passes; `--help` lists the flags.
- Entry criteria: M0 decision recorded.
- Exit criteria: no behavior difference from the shell version is observed in a publish run.
- Parallel-plan ready: yes, with WS-CORPUS (M8) running at the same time; the two touch disjoint
  files.

### Milestone: M2 decide what to build

- Depends on: none in code; M0 for the wasm-row decision.
- Deliverables: WP-L2.
- Done checks: `source source_me.sh && pytest tests/test_change_scope.py` passes.
- Entry criteria: none.
- Exit criteria: every row of the build table has a passing case, including two multi-stale cases.
- Parallel-plan ready: yes. Pure functions, no dependency on M1.

### Milestone: M3 run the build

- Depends on: M2 (consumes its decision).
- Deliverables: WP-L3.
- Done checks: `source source_me.sh && python3 tests/_temp/check_build_dispatch.py` passes.
- Entry criteria: M2 merged.
- Exit criteria: each decision maps to exactly the commands the table names.
- Parallel-plan ready: no. One lane with M2 and M5.

### Milestone: M4 rebuild the application services

- Depends on: none in code; M5 consumes it.
- Deliverables: WP-L4.
- Done checks: on a running stack, `python3 local_stack.py rebuild-application` reports ready; api,
  worker, and public-asset-publisher have new container ids; PostgreSQL, MinIO, and the renderer
  keep theirs.
- Entry criteria: a running Live Demo.
- Exit criteria: evidence recorded in the M0 report.
- Parallel-plan ready: yes. Stack boundary, disjoint from the driver files.

### Milestone: M5 wire the driver, warm and cold

- Depends on: M1, M2, M3, M4.
- Deliverables: WP-L5.
- Done checks: `bash tests/e2e/e2e_screenshot_warm_loop.sh` passes once M6 exists; before that, a
  manual warm run and a manual cold run both publish successfully.
- Entry criteria: M1-M4 merged.
- Exit criteria: a cold run performs exactly one build; a warm run performs at most one.
- Parallel-plan ready: no. Integration point of WS-LOOP.

### Milestone: M6 permanent warm-loop contract

- Depends on: M5.
- Deliverables: WP-L6.
- Done checks: `bash tests/e2e/e2e_screenshot_warm_loop.sh` exits 0 and leaves the working tree
  unchanged.
- Entry criteria: M5 merged.
- Exit criteria: the runner is listed in `docs/E2E_TESTS.md` at M14.
- Parallel-plan ready: no. Depends on the wired driver.

### Milestone: M7 prove the other build paths, clean up

- Depends on: M5, M6.
- Deliverables: WP-E2.
- Done checks: both driver runs exit 0, each publishing a corpus its own promotion check validates;
  neither temporary file exists.
- Entry criteria: M6 green.
- Exit criteria: the M0 report carries the WASM and server timings.
- Parallel-plan ready: no. Closes WS-LOOP.

### Milestone: M8 capture metadata into scenarios

- Depends on: none.
- Deliverables: WP-C1.
- Done checks: `node --import tsx --test tests/test_screenshot_corpus_definition.mjs` passes;
  `./devel/capture_screenshots.sh --verify` still passes against the hand-written manifest.
- Entry criteria: none.
- Exit criteria: no captured output changed, proving the move was mechanical.
- Parallel-plan ready: yes, with the whole of WS-LOOP.

### Milestone: M9 route id observed, not declared

- Depends on: M8 (the declarations exist but carry no route, so observation must arrive before
  anything generates a manifest from them).
- Deliverables: WP-C3.
- Done checks: the corpus definition test rejects a role-versus-route violation; `--verify` passes
  against the still hand-written manifest, whose declared routes the observed values now confirm.
- Entry criteria: M8 merged.
- Exit criteria: the runtime records the reached route for every capture, and no scenario declares
  one.
- Parallel-plan ready: no. Serial inside WS-CORPUS.

### Milestone: M10 manifest becomes generated

- Depends on: M9 (generation records the observed route).
- Deliverables: WP-C2.
- Done checks: the corpus definition test covers generation and drift; one publish run writes the
  generated manifest and passes its promotion check; a following `--verify` confirms the committed
  file matches regeneration.
- Entry criteria: M9 merged.
- Exit criteria: the committed manifest is byte-identical to the generated one.
- Parallel-plan ready: no.

### Milestone: M11 coverage is computed

- Depends on: M10 (coverage is computed during generation, from the observed route).
- Deliverables: WP-C4.
- Done checks: generation fails for an uncovered, unlisted surface and passes for a listed one; one
  publish run refreshes the generated ledgers; the generated ledgers match the ledgers they replace
  in a reviewed diff.
- Entry criteria: M10 merged.
- Exit criteria: `coverage_exceptions.json` holds only non-inferable entries.
- Parallel-plan ready: no.

### Milestone: M12 gallery order derived

- Depends on: M10 (ordering is a generation concern).
- Deliverables: WP-C5.
- Done checks: ordering is stable and grouped in the test; one publish run succeeds, its own
  promotion check standing as the corpus proof; the atlas diff is reviewed for grouping changes.
- Entry criteria: M10 merged.
- Exit criteria: `gallery.order` appears nowhere.
- Parallel-plan ready: yes, alongside M11; both consume generation and touch different parts of it.
  Maximum two lanes inside WS-CORPUS, because both edit `manifest.ts`.

### Milestone: M13 add and remove a screenshot

- Depends on: M8-M12.
- Deliverables: WP-C6.
- Done checks: both publish runs exit 0, each proving its own corpus through promotion; the
  recorded hand-edited file count is one.
- Entry criteria: WS-CORPUS merged.
- Exit criteria: scratch deleted; count recorded in the M0 report.
- Parallel-plan ready: no. Closes WS-CORPUS.

### Milestone: M14 docs and changelog

- Depends on: M7, M13.
- Deliverables: WP-D1.
- Done checks: `source source_me.sh && pytest tests/test_markdown_links.py tests/test_ascii_compliance.py`
  passes.
- Entry criteria: both tracks closed.
- Exit criteria: `docs/CHANGELOG.md` carries the entry with its gates.
- Parallel-plan ready: no.

## Workstream breakdown

### Workstream: WS-EVIDENCE

- Goal: measure before building, and prove the paths that do not deserve permanent tests.
- Owner: tester.
- Work packages: WP-E1 (M0), WP-E2 (M7).
- Needs: a running Podman machine; the driver from WS-LOOP for WP-E2.
- Provides: the timings report that prunes this plan, and the one-time evidence for M4 and M13.
- Review boundary, when modifying the repository: `tests/_temp/` and one report under
  `docs/active_plans/reports/`; nothing else.

### Workstream: WS-LOOP

- Goal: make a warm stack the normal case and rebuild only what changed.
- Owner: coder, with tester for WP-L6.
- Work packages: WP-L1 through WP-L6 (M1-M6).
- Needs: the M0 decision.
- Provides: `./devel/capture_screenshots.sh` as a fast loop, and
  `local_stack.py rebuild-application`.
- Review boundary, when modifying the repository: driver boundary and stack boundary; no file
  under `tests/playwright/screenshot_corpus/`.

### Workstream: WS-CORPUS

- Goal: one hand-authored definition per screenshot.
- Owner: coder, with tester for WP-C6.
- Work packages: WP-C1 through WP-C6 (M8-M13).
- Needs: nothing from WS-LOOP.
- Provides: a generated manifest, computed coverage, derived ordering.
- Review boundary, when modifying the repository: corpus boundary and `docs/screenshots/`; no file
  under `devel/` or `local_stack_control/`.

### Workstream: WS-DOCS

- Goal: document both results where the next person looks.
- Owner: planner.
- Work packages: WP-D1 (M14).
- Needs: both tracks closed.
- Provides: updated screenshot, development, and agent docs plus the changelog entry.
- Review boundary, when modifying the repository: `docs/` and `AGENTS.md`.

## Work packages

### Work package: WP-E1 measure the current loop

- Owner: tester.
- Touch points: new `tests/_temp/measure_screenshot_loop.sh`, new
  `docs/active_plans/reports/screenshot_loop_timings.md`.
- Depends on: none.
- Acceptance criteria: three end-to-end warm loops measured (a TypeScript edit, a `crates/wasm`
  edit, a `crates/server` edit, each to a finished screenshot) plus the components: cold start,
  warm reuse, `node pipeline/build.mjs --skip-wasm`, `pipeline/build_wasm.sh --debug`,
  `./build.sh --debug`, `compose build api` with application replacement, and one full
  capture-and-publish. Each row carries its command. The report ends with the written decision
  described in M0's exit criteria.
- Evidence or review: the report itself.
- Obvious follow-ons: prune or fold milestones the numbers do not justify, and say so in the report.

### Work package: WP-L1 port the driver

- Owner: coder.
- Touch points: new `devel/capture_screenshots.py`; `devel/capture_screenshots.sh` reduced to a
  delegate that execs it.
- Depends on: WP-E1, for the decision that may drop a flag.
- Acceptance criteria: same order of operations as the shell version - static verify,
  `devel/setup_playwright.sh`, optional stop, `launchers/run_live_demo.sh --headless`, parse
  `Live demo entry:`, resolve the authenticator path through
  `python3 local_stack.py authenticator --env-file ...`, export `NODE_EXTRA_CA_CERTS`, invoke the
  runner. `--verify`, `--headed`, `--fresh`, `--only=IDS`, `--help` keep their meaning. Each step
  prints when it starts, names what it waits on, and reports elapsed seconds
  (`docs/HUMAN_GUIDANCE.md:63-64`).
- Evidence or review: a publish run and a `--verify` run.
- Obvious follow-ons: none.

### Work package: WP-L2 decide what to build

- Owner: coder.
- Touch points: new `devel/change_scope.py`; new `tests/test_change_scope.py`.
- Depends on: WP-E1 for the wasm row.
- Acceptance criteria: `decide(repo_root, api_image_created_at, launch_receipt_at) -> Decision`
  with `build` in `none|client|wasm_client|full` and `containers` in
  `none|replace_application|full_restart`, implementing the table in **Build and container
  decision** below, returning the strongest value per column. `generated/`, `dist/`, `dist_wasm/`,
  and `target/` are skipped as build output. `--build` overrides.
- Evidence or review: `pytest tests/test_change_scope.py` with one case per row plus two
  multi-stale cases.
- Obvious follow-ons: none.

### Work package: WP-L3 execute the build

- Owner: coder.
- Touch points: `devel/capture_screenshots.py`; new `tests/_temp/check_build_dispatch.py`.
- Depends on: WP-L2.
- Acceptance criteria: `run_build(decision, repo_root)` runs exactly one of nothing,
  `node pipeline/build.mjs --skip-wasm`, `pipeline/build_wasm.sh --debug` then
  `node pipeline/build.mjs`, or `./build.sh --debug`; streams output; checks return codes; reports
  elapsed seconds per command.
- Evidence or review: the temporary dispatch check, deleted in WP-E2.
- Obvious follow-ons: none.

### Work package: WP-L4 rebuild the application services

- Owner: coder.
- Touch points: `local_stack_control/cli.py`, `local_stack_control/commands.py`,
  `local_stack_control/lifecycle.py`.
- Depends on: none.
- Acceptance criteria: `local_stack.py rebuild-application` builds the shared application image
  under the existing `image_cleanup.image_build_lease`, then recreates the api, worker, and
  public-asset-publisher group through `lifecycle_profiles.recreate_arguments` and
  `wait_for_complete_ready`. PostgreSQL, MinIO, and the renderer keep running. It keeps what
  `restart` enforces today: the renderer attestation check and refusal of non-default mutation
  targets. The gateway stays out of the group; it serves `dist/` through a bind mount.
- Evidence or review: `local_stack.py status --json` before and after, recorded in the M0 report.
- Obvious follow-ons: none. The operation is named for the group, not for one service, because the
  three share one image and must move together.

### Work package: WP-L5 wire warm and cold paths

- Owner: coder.
- Touch points: `devel/capture_screenshots.py`.
- Depends on: WP-L1, WP-L2, WP-L3, WP-L4.
- Acceptance criteria: the ordering in **Warm and cold ordering** below is implemented exactly;
  after capture, changed images are printed, before/after copies are written under
  `test-results/screenshot-corpus/review/`, and a run with no differences says "no visual change"
  and exits 0. The report hashes **only `.png` files under the four role folders**
  (`docs/screenshots/{public,instructor,student,sysadmin}/`), never the manifest, receipt, atlas,
  or coverage exceptions. Those are generated bookkeeping, and once WS-CORPUS lands they change on
  runs where nothing visual moved; including them would make "visual change" mean something else.
- Evidence or review: a run after a cosmetic edit reports the changed screenshots; a run after a
  WS-CORPUS regeneration with no style change still reports "no visual change".
- Obvious follow-ons: none.

### Work package: WP-L6 permanent warm-loop test

- Owner: tester.
- Touch points: new `tests/e2e/e2e_screenshot_warm_loop.sh`.
- Depends on: WP-L5.
- Acceptance criteria: starts the Live Demo if absent; runs `touch src/style.css` so the source is
  newer than `dist/index.html` without changing content; runs
  `./devel/capture_screenshots.sh --verify`; asserts `dist/index.html` is newer afterwards and the
  replay verification passed; prints elapsed seconds; leaves the working tree as it was.
- Evidence or review: the runner's own output.
- Obvious follow-ons: list it in `docs/E2E_TESTS.md` at WP-D1.

### Work package: WP-E2 prove the other build paths and clean up

- Owner: tester.
- Touch points: `tests/_temp/measure_screenshot_loop.sh`,
  `tests/_temp/check_build_dispatch.py`, `docs/active_plans/reports/screenshot_loop_timings.md`.
- Depends on: WP-L5, WP-L6.
- Acceptance criteria: one driver run for a reversible `crates/wasm/src/` edit shows no container
  work; one for a reversible `crates/server/src/` edit shows the application services replaced and
  the PostgreSQL container unchanged; both edits are restored; both timings are recorded; both
  temporary files are deleted.
- Evidence or review: the report entries.
- Obvious follow-ons: none.

### Work package: WP-C1 move capture metadata into the scenarios

- Owner: coder.
- Touch points: `tests/playwright/screenshot_corpus/scenario_types.ts`,
  `tests/playwright/screenshot_corpus/runtime.ts`, the eight `scenarios_*.ts` files; new
  `tests/test_screenshot_corpus_definition.mjs`.
- Depends on: none.
- Acceptance criteria: `captureCheckpoint` carries `checkpoint`, `area`, `workflow`, `state`,
  `viewport`, `privacyProfile`, `caption`, and optional `featured`; the scenario declares `role`
  once; capture id and image path derive from role and checkpoint (`<role>_<checkpoint>`,
  `<role>/<checkpoint>.png`); the separate `checkpoints` list is gone; runtime reads metadata from
  the call rather than the manifest.
- Evidence or review: the corpus definition test (one definition per capture, unique ids and paths,
  closed vocabularies), then `--verify` passing against the still hand-written manifest, which
  proves the move changed no captured output.
- Obvious follow-ons: WP-C2.

### Work package: WP-C3 observe the route

- Owner: coder.
- Touch points: `tests/playwright/screenshot_corpus/runtime.ts`,
  `tests/playwright/screenshot_corpus/manifest.ts`.
- Depends on: WP-C1.
- Acceptance criteria: the route the page reached is recorded through `routeContractForPathname`
  and carried on the capture result, and a page on no declared route fails; the policy
  `validateCaptureRelationships` enforces today is kept and evaluated against the observed route -
  a capture's role must be inside the route's `requiredProductRoles`, and a public capture must be
  sign-in or expired-session recovery. While the manifest is still hand-written, the observed route
  is also compared against the declared one, which is today's `assertRoute` behavior unchanged.
- Evidence or review: the corpus definition test rejecting a student capture on an instructor-only
  route, then `--verify` against the hand-written manifest.
- Obvious follow-ons: WP-C2.

### Work package: WP-C2 generate the manifest

- Owner: coder.
- Touch points: `tests/playwright/screenshot_corpus/manifest.ts`,
  `tests/playwright/screenshot_corpus/publication.ts`, `tests/playwright/screenshot_corpus/cli.ts`.
- Depends on: WP-C1, WP-C3.
- Acceptance criteria: publishing writes `docs/screenshots/current_capture_manifest.json` from the
  scenario declarations plus the routes observed during that run, and the receipt digests it as
  today. Because a route is known only after a page is reached, manifest generation happens after
  capture: publish writes it, and `--verify` regenerates it in memory from its own live replay and
  fails when the committed file differs. `decodeManifest` remains the reader for the committed
  file. `validateScenarioClosure` is deleted because closure is structural once both sides share a
  source. The declared-versus-observed route comparison from WP-C3 is dropped here, since there is
  no longer a declaration to compare against.
- Evidence or review: a generated manifest accepted by `decodeManifest`; a hand-edited manifest
  failing the drift check; one publish run followed by `--verify`.
- Obvious follow-ons: WP-C4, WP-C5.

### Work package: WP-C4 compute coverage

- Owner: coder.
- Touch points: `tests/playwright/screenshot_corpus/manifest.ts`; new
  `docs/screenshots/coverage_exceptions.json`.
- Depends on: WP-C3.
- Acceptance criteria: route coverage comes from the observed route. Ribbon coverage comes from the
  catalogs: every `TAB_CATALOG` and `RIBBON_TASK_CATALOG` entry whose destination is
  `{ kind: "route", routeId }` (`src/ribbon/ribbon_catalog.ts:49`) is covered when a capture
  observed that route, so no second mapping is written. Controls with a non-route destination, such
  as the `teachingOperations` and `courseSetup` group controls, go in the exceptions file alongside
  `covered_by` targets and `deferred` reasons. Generation fails when a surface is neither captured
  nor listed, preserving today's closure guarantee. The current ledgers are migrated, dropping every
  entry now computed.
- Evidence or review: an uncovered unlisted route failing generation; a listed one passing; a
  route-destination Ribbon control covered by its route's capture; one publish run to refresh the
  ledgers; a reviewed diff showing the generated ledgers match the ones they replace.
- Obvious follow-ons: none.

### Work package: WP-C5 derive gallery order

- Owner: coder.
- Touch points: `tests/playwright/screenshot_corpus/manifest.ts`,
  `tests/playwright/screenshot_corpus/publication.ts`, the eight `scenarios_*.ts` files.
- Depends on: WP-C2.
- Acceptance criteria (ordering only; coverage stays with WP-C4): `gallery.order` is gone; the atlas groups by role, area, and workflow as it
  does today, and within a group orders by declaration order in the scenario file; `caption` and
  `featured` remain declared.
- Evidence or review: stable grouped ordering in the test; one publish run, whose promotion check
  is the corpus proof; the atlas diff reviewed for grouping changes.
- Obvious follow-ons: none.

### Work package: WP-C6 add and remove a screenshot

- Owner: tester.
- Touch points: `tests/_temp/` scratch, deleted at the end; one scenario file, edited and reverted.
- Depends on: WP-C1 through WP-C5.
- Acceptance criteria: adding one `captureCheckpoint` call and rebuilding produces the new PNG, the
  regenerated manifest entry, and the atlas tile; generated files change as expected and exactly one
  hand-authored file was edited; deleting the call and rebuilding prunes the PNG and the manifest
  entry; the hand-edited file count is recorded in the M0 report.
- Evidence or review: the two publish runs, each validated by its own promotion check.
- Obvious follow-ons: none.

### Work package: WP-D1 documentation and changelog

- Owner: planner.
- Touch points: `docs/HOW_TO_SCREENSHOT.md`, `docs/DEVELOPMENT.md`, `AGENTS.md`,
  `docs/E2E_TESTS.md`, `docs/CHANGELOG.md`.
- Depends on: WP-E2, WP-C6.
- Acceptance criteria: the warm loop is documented as the normal path and the build/container table
  as what each kind of edit costs; `local_stack.py rebuild-application` is documented as the way to
  pick up a server change without losing the database; "Add or change a capture" becomes one step -
  add or delete a `captureCheckpoint` call, then rebuild - and names the manifest, receipt, atlas,
  and ledgers as generated, with `coverage_exceptions.json` the one hand-kept list;
  `tests/e2e/e2e_screenshot_warm_loop.sh` is listed in `docs/E2E_TESTS.md`; the changelog records
  the milestones, the M0 measurements, and the gates.
- Evidence or review: `pytest tests/test_markdown_links.py tests/test_ascii_compliance.py`.
- Obvious follow-ons: none.

## Build and container decision

| Newer than its output | Build | Containers |
| --- | --- | --- |
| `src/`, `assets/` newer than `dist/index.html` | `node pipeline/build.mjs --skip-wasm` | none |
| `crates/wasm/` newer than `dist/wasm/ple_bridge_bg.wasm` | `pipeline/build_wasm.sh --debug`, then `node pipeline/build.mjs` | none |
| any other `crates/`, `Cargo.toml`, `Cargo.lock` newer than the api image creation time | `./build.sh --debug` | replace the application containers |
| `schemas/`, `containers/`, `compose*.yaml` newer than the stack's launch receipt | done by the restart itself | full stack restart |
| nothing newer | none | none |

Both sides of every comparison are named files, never directory timestamps, which do not track
their contents. The source side is the newest mtime found by walking the named tree; the output
side is the file that build step actually writes: `dist/index.html` for the client bundle
(`pipeline/build.mjs:178`) and `dist/wasm/ple_bridge_bg.wasm` for the bridge, which
`build_wasm.sh:71` asserts exists and `build.mjs:144-147` copies into `dist/wasm/`. A missing
output file counts as stale, so every ambiguous case fails toward rebuilding.

The two reference times are chosen so the comparison stays true across restarts and reuse. The api
**image** creation time (`podman image inspect` on the image id the running api container reports)
is the moment that Rust source was compiled in; a container restart, a `podman start`, or reusing
yesterday's stack does not move it, while a rebuild does. Container start time would have been
wrong in exactly those cases. The launch receipt
`local_stack_state/live_demo_browser/developer-control.json` marks when this stack was provisioned,
which is when the schema was installed and the Compose topology applied; those facts live in the
database and the running project, not in the image. That lifetime is verified:
`browser_suite_developer.py:649` writes the receipt once, immediately after the launch completes,
and `:686` removes it at teardown. Reusing a running stack never rewrites it, because
`launchers/run_live_demo.sh:77-87` only reads `.origin` and exits, so a schema edit made after
provisioning stays visible as stale.

When several rows are stale at once, `decide()` returns the strongest of each column: `full` beats
`wasm_client` beats `client` beats `none`, and `full_restart` beats `replace_application` beats
`none`. Nothing is built twice, because `./build.sh` already contains the client and wasm stages,
and a full restart performs its own `./build.sh` (`local_stack_control/lifecycle.py:521-533`), so
the driver's build is `none` in that row.

## Warm and cold ordering

1. `--fresh` stops any owned stack first, making this a cold run.
2. Resolve the stack. If none is running, start it through `launchers/run_live_demo.sh --headless`.
   That launch builds everything, so the run skips the decision and the build and goes to capture.
3. With a stack already running: decide, build, then apply the container decision - nothing,
   `local_stack.py rebuild-application`, or a full restart, which performs its own build.
4. Capture and publish the full corpus exactly as today.
5. Compare tracked PNG hashes before and after; report the changed images.

## Two capture modes, used once each

The two invocations are not interchangeable, and neither is a superset in practice:

- `./devel/capture_screenshots.sh` captures every scenario, validates each capture, then promotes:
  `promoteCorpus` re-inspects the published corpus and compares receipts
  (`publication.ts:380-450`). A successful publish leaves a corpus that is consistent by
  construction, so nothing further needs to confirm it.
- `./devel/capture_screenshots.sh --verify` writes nothing. It checks the **committed** artifacts -
  tracked PNG set, receipt bytes, atlas bytes - against the manifest, replays live into
  `test-results/`, and confirms the replay did not modify tracked files (`cli.ts:64, 112-133`).
  That is drift detection on committed state.

Rule for this plan: a milestone that publishes does not also run `--verify`, because publish
already proved the corpus it just wrote. A milestone that changes behavior without changing
generated output uses `--verify`, which proves the change moved nothing it should not have.

## Acceptance criteria and gates

- Per-patch gate: the milestone's own done checks, plus
  `source source_me.sh && ./launchers/run_fast_checks.sh`.
- Integration gate, after M7 and after M13:

```bash
source source_me.sh && ./launchers/run_fast_checks.sh
source source_me.sh && pytest tests/test_change_scope.py
node --import tsx --test tests/test_screenshot_corpus_definition.mjs
bash tests/e2e/e2e_screenshot_warm_loop.sh
source source_me.sh && ./devel/capture_screenshots.sh --verify
```

- Independent review gate: M11's ledger diff and M12's atlas diff are reviewed by an agent other
  than the one that generated them, because both replace hand-written evidence with computed
  evidence.
- Failure semantics: a failing done check blocks its milestone only. A failing integration gate
  blocks M14. A failing `--verify` at any point blocks every later milestone in that workstream,
  because the corpus is the shared artifact.

## Test and verification strategy

Three permanent test files survive this plan. Everything else is one-time evidence under
`tests/_temp/`, deleted in M7 and M13, per `docs/PYTEST_STYLE.md` and
`docs/HUMAN_GUIDANCE.md:65`.

| File | Protects | Lane |
| --- | --- | --- |
| `tests/test_change_scope.py` | Which edit forces which build and which container work | pytest |
| `tests/e2e/e2e_screenshot_warm_loop.sh` | A warm stack rebuilds a stale bundle and serves it | E2E |
| `tests/test_screenshot_corpus_definition.mjs` | One definition per capture, unique ids and paths, closed vocabularies, role-versus-route policy, closed coverage | Node |

Each of M8-M12 extends the corpus definition test with a contract, never with an assertion about
how generation is implemented. Subprocess argument sequences, container ids, and stage choices are
implementation proof and stay temporary.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Timestamps mislead after a `git checkout` restores an older file with a new mtime | An unnecessary full build | Branch switching mid-session | coder | Accepted: the failure direction is extra work, never stale evidence. `--build none` skips it when the operator knows better. |
| The api image id cannot be resolved from a degraded stack | The driver cannot decide | Partially failed stack | coder | Treat an unreadable reference time as "everything is stale" and fall back to a full restart, reporting why. |
| Manifest generation changes bytes the receipt digests | `--verify` fails repo-wide | M10 lands | coder | M8 and M9 land first and prove captured output is unchanged; M10 regenerates and republishes in one milestone, with the diff reviewed. |
| Route observation and manifest generation land out of order, leaving generation with no route to record | M10 cannot generate a valid manifest | M10 dispatched before M9 | manager | M9 is a stated dependency of M10; WP-C3 keeps the declared-versus-observed comparison alive until WP-C2 removes the declaration. |
| Computed coverage silently drops a deliberate exemption | Lost review decision | M11 migration | coder | Generation fails closed on any surface neither captured nor listed; the migration diff is reviewed by a second agent. |
| Losing gallery order changes the atlas reading order | Reviewer confusion | M12 lands | coder | Ordering derives from the grouping the atlas already uses; the atlas diff is reviewed before closing M12. |
| Plan drift: milestones survive that M0 showed to be pointless | Wasted work | M0 report ignored | manager | M0's exit criteria require a written decision, and M1 cannot start until it exists. |
| Scope creep back into capture selection | The rejected complexity returns | A capture run feels slow | manager | Selection is an explicit non-goal; revisit only with a new M0-style measurement. |

## Documentation close-out requirements

- Active plan / progress tracker: `docs/active_plans/active/fast_screenshot_loop.md` carries
  milestone status while the work runs; `git mv` it to `docs/archive/` at closure.
- `docs/active_plans/reports/screenshot_loop_timings.md` is a report, not scratch, and stays after
  M7 deletes the harness that produced it.
- docs/CHANGELOG.md entry: one day block covering both tracks, with the gates named per bullet.
- Archive / closure notes: record in `docs/DESIGN_DECISIONS.md` that the screenshot manifest is
  generated from scenario declarations, with the scenario as the owning contract.

## Open questions and decisions needed

- Manager/subagent decision procedure for the `crates/wasm` row:
  - Decision owner: the WP-E1 tester, recorded in the M0 report.
  - Evidence and decision rule: if `pipeline/build_wasm.sh --debug` plus a client build is not
    meaningfully faster than `./build.sh --debug` on this machine, fold the row into `full` and
    delete the `wasm_client` value from WP-L2.
- Non-blocking follow-up: whether `--only=IDS` still earns its place once the warm loop is fast.
  Decide after M7 from the M0 numbers; removing it is a one-line change in the driver and the
  runner's argument parsing.
- Non-blocking follow-up: whether the review folder under `test-results/screenshot-corpus/review/`
  should also emit a contact sheet. Defer until someone reviews a real change report and asks.
