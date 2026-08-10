# Plan: Capability-sized source module decomposition

## Status

Accepted on 2026-08-10. The dated 26-file maintained-source baseline now has zero maintained-code
violations, stable facades preserve the public boundaries, and the permanent size gate is green.
Independent PostgreSQL, server-security, provider-security, TypeScript/HCI, test, size-policy, and
final architecture reviews found no unresolved P0/P1 issue.

## Context

`docs/HUMAN_GUIDANCE.md` requires every maintained source file to stay below 1,000 lines by moving
complete capabilities into focused modules. The 2026-08-10 baseline snapshot records 26 maintained
Rust, TypeScript, TSX, and test sources above that boundary. The largest are persistence and run-route
facades, but the same ownership problem also appears in adapters, project tooling, browser decoding,
mocks, and response widgets.

The system behavior is working and broadly tested. This is therefore a compatibility-preserving
architecture extraction, not a rewrite. It runs after the passed WP-RC3 live gate, before final RC3
acceptance and WP-RC4, so the payload and flat-family packages add behavior to focused owners rather
than expanding the existing warehouses. It must be accepted before WP-RC12.

## Objectives

- Bring every maintained source file in the declared gate below 1,000 physical lines.
- Give each extracted module one durable capability, explicit dependencies, and focused tests.
- Preserve public Rust paths, HTTP and browser contracts, SQL behavior, generated artifacts, and
  learner/instructor behavior byte-for-byte unless formatting is inherently regenerated.
- Add a permanent repository gate so future growth fails at the first oversized file.

## Design philosophy

This plan applies **Fix the design, not the symptom**, **Long-term over short-term**, and **Atomic
task decomposition**. It rejects arbitrary line-range files, include-file fragments, and test-only
rearrangement. Each move follows a real capability boundary and leaves a small facade or explicit
re-export where consumers need stable ownership.

The evidence strategy is compiler- and behavior-led: record each owner's current public symbols and
focused tests, extract one capability, then rerun that focused gate before the next move. A change
that requires a public wire, schema, or semantic alteration is removed from this package and assigned
to its owning feature plan; it is not smuggled into a structural patch.

## Scope

- Extract the sources that violate the current inventory at each WP-ARCH1 package start; the dated
  26-file baseline snapshot below establishes the initial decomposition scope and expected owners.
- Preserve the existing facade/module import paths when Rust or TypeScript consumers rely on them.
- Split oversized tests by behavior and fixture ownership, not by line number.
- Adopt `tests/test_source_file_line_limit.py` with an exact maintained-source and approved-artifact
  exclusion contract.
- Update architecture, file-map, status, and changelog documentation with the final module owners.
- Re-run focused, package, browser, repository, and independent architecture gates.

## Non-goals

- Change route payloads, database schema, SQL semantics, grading rules, security policy, or UI
  behavior; feature packages own those changes.
- Rename public crates, Cargo packages, HTTP routes, generated type names, or browser entry points;
  stable facades make the extraction compatible.
- Split accepted SQL migrations, generated output, vendored repositories, lockfiles, or Markdown;
  they are immutable ledgers, generated artifacts, external sources, or documentation rather than
  maintained capability source modules.
- Exempt a maintained code or test module. The exact approved override list is limited to immutable
  applied migration ledgers and documentation/history artifacts governed by their own structure or
  rotation rules; the package succeeds only when no oversized maintained code remains.
- Replace PostgreSQL, Solid, Axum, adapter, or Store architecture; current boundaries remain valid.

## Dated baseline snapshot and inventory discipline

This 2026-08-10 baseline snapshot is evidence, not a timeless assertion. At the start of every
WP-ARCH1 package and again at that package's acceptance, regenerate the inventory with this exact
read-only command. Record the resulting violation list in the package handoff and reconcile any
added, removed, renamed, or re-sized owner with the work package before acceptance.

The command scans the repository root, including root launch/configuration sources and `containers/`,
while omitting only exact non-maintained prefixes and reporting only files at or above the 1,000-line
boundary:

```bash
rg --files --hidden \
  -g '*.rs' -g '*.ts' -g '*.tsx' -g '*.mts' -g '*.cts' \
  -g '*.js' -g '*.mjs' -g '*.cjs' -g '*.py' -g '*.sh' \
  -g '!.git/**' -g '!.pytest_cache/**' -g '!.venv/**' \
  -g '!OTHER_REPOS/**' -g '!coverage/**' -g '!dist/**' -g '!dist_wasm/**' \
  -g '!generated/**' -g '!node_modules/**' -g '!playwright-report/**' \
  -g '!target/**' -g '!test-results/**' -g '!docs/archive/**' \
  -g '!tests/fixtures/**' -g '!tests/artifacts/**' \
  -g '!tests/e2e/fixtures/**' -g '!tests/playwright/fixtures/**' \
  -0 | xargs -0 wc -l \
  | awk '$2 != "total" && $1 >= 1000' \
  | sort -nr
```

The 2026-08-10 baseline snapshot contains these 26 violations:

| Lines | Current owner                                                               |
| ----: | --------------------------------------------------------------------------- |
| 6,649 | `crates/learning-data-access/src/postgres.rs`                               |
| 5,412 | `crates/server/src/run.rs`                                                  |
| 5,051 | `crates/learning-data-access/tests/conformance.rs`                          |
| 4,198 | `crates/learning-data-access/src/in_memory.rs`                              |
| 2,917 | `crates/project-tools/src/e2e_seed.rs`                                      |
| 2,856 | `src/api/decoders.ts`                                                       |
| 2,336 | `crates/learning-data-access/src/lib.rs`                                    |
| 1,994 | `crates/server/src/workspace.rs`                                            |
| 1,970 | `crates/adapters/webwork/src/http_renderer.rs`                              |
| 1,911 | `crates/server/src/composition.rs`                                          |
| 1,875 | `crates/server/src/catalog.rs`                                              |
| 1,849 | `crates/server/src/retention.rs`                                            |
| 1,758 | `crates/server/src/course.rs`                                               |
| 1,715 | `crates/adapters/native/src/lib.rs`                                         |
| 1,677 | `crates/server/src/imathas_backend.rs`                                      |
| 1,625 | `crates/adapters/imathas/src/lib.rs`                                        |
| 1,438 | `crates/adapters/imathas/src/broker_provider.rs`                            |
| 1,329 | `devel/bump_version.py`                                                     |
| 1,203 | `src/api/mock/handlers.ts`                                                  |
| 1,154 | `crates/server/src/flat_question_publication/tests.rs`                      |
| 1,097 | `crates/adapters/webwork/src/lib.rs`                                        |
| 1,073 | `src/api/http_client.ts`                                                    |
| 1,059 | `crates/domain/src/statistics.rs`                                           |
| 1,035 | `crates/export/src/pdf.rs`                                                  |
| 1,030 | `src/components/response_widget.tsx`                                        |
| 1,022 | `crates/learning-data-access/tests/postgres_flat_import_provenance_live.rs` |

Several owners already have same-name subdirectories. Those are the intended homes for complete
capabilities. A file may coexist with a same-name subdirectory when it remains a small facade;
where Rust module resolution requires it, the owner uses `git mv` from `<name>.rs` to
`<name>/mod.rs` and preserves public re-exports.

### Persistence extraction matrix

WP-SIZE1 uses this exact mapping. The named capability partition and child module names are fixed;
only a mechanical spelling correction accepted in the plan before implementation may change them.

| Current source                                                              | Stable facade                                       | Capability modules                                                                                                                                                                                                                                                                                                                                       | Public surface that remains available                                                                                                                       | Pre/post command                                                                                           | Reviewer                         |
| --------------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | -------------------------------- |
| `crates/learning-data-access/src/postgres.rs`                               | `postgres/mod.rs`                                   | Existing `assets`, `assignment_timing`, `catalog`, `connection`, `course_appearance`, `exports`, `external_tool`, `flat_import_provenance`, `flat_question`, `item_analysis`, `jobs`, `manual_grading`, `migrations`, `qti`, `qti_ingress`, `retention`, and `sessions`, plus focused `activity`, `courses`, `feedback`, `runs`, and `statistics` owners | `PostgresStore`, `PostgresGraderStore`, `Pool`, `lazy_pool`, `ping`, migration exports, and every existing Store/worker trait implementation                | `cargo test -p learning-data-access --all-targets`                                                         | `postgresql-expert`              |
| `crates/learning-data-access/src/in_memory.rs`                              | `in_memory/mod.rs`                                  | Existing `assets`, `catalog`, `course_appearance`, `exports`, `external_tool`, `flat_import_provenance`, `flat_question`, `item_analysis`, `manual_grading`, `qti`, `qti_ingress`, `queue`, `retention`, and `sessions`, plus focused `activity`, `courses`, `feedback`, `runs`, and `statistics` owners matching PostgreSQL                             | `MemoryStore`, `MemoryQtiGraderStore`, `MemoryFlatQuestionGraderStore`, constructors, and every existing Store/worker trait implementation                  | `cargo test -p learning-data-access --all-targets`                                                         | Store-conformance reviewer       |
| `crates/learning-data-access/src/lib.rs`                                    | `lib.rs`                                            | `contracts/{catalog,courses,runs,feedback,jobs,retention,objects,auth,authoring,analytics}.rs`; existing domain modules remain owners                                                                                                                                                                                                                    | All existing `pub use` paths; Store, CatalogStore, CatalogSourceStore, worker and retention traits; StoreError; commands, records, identifiers, and aliases | `cargo check -p learning-data-access --all-targets` and `cargo test -p learning-data-access --all-targets` | public-API architecture reviewer |
| `crates/learning-data-access/tests/conformance.rs`                          | `tests/conformance/mod.rs`                          | Existing capability suites plus focused `activity`, `courses`, `feedback`, `retention`, `runs`, and `statistics` suites; shared fixtures live in `support.rs`                                                                                                                                                                                            | Existing test names and the Memory/PostgreSQL suite invocation contract                                                                                     | `cargo test -p learning-data-access --test conformance`                                                    | Store-conformance reviewer       |
| `crates/learning-data-access/tests/postgres_flat_import_provenance_live.rs` | `tests/postgres_flat_import_provenance_live/mod.rs` | `setup.rs`, `success.rs`, and `refusal.rs`                                                                                                                                                                                                                                                                                                               | Existing ignored/live test names, environment contract, and cleanup behavior                                                                                | `cargo test -p learning-data-access --test postgres_flat_import_provenance_live`                           | `postgresql-expert`              |

## Architecture boundaries and ownership

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                                              | Review boundary                                                                     |
| ---------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| M1 / WS-PERSIST        | Store contracts, Memory/PostgreSQL, conformance        | Public trait/type paths, SQL strings, transactions, RLS context, Store parity       |
| M1 / WS-SERVER         | Axum route owners and composition                      | Router shape, authorization order, response bytes, backend calls, route tests       |
| M1 / WS-ADAPTER        | Adapters, project tools, domain statistics, PDF export | Provider protocols, cache/grading secrecy, CLI output, generated artifacts          |
| M1 / WS-BROWSER        | Browser API, mocks, response widgets                   | Exported TS types, strict decode behavior, same-origin transport, keyboard behavior |
| M2 / WS-CLOSE          | Permanent gate and documentation                       | Whole-tree size policy, full repository behavior, contributor ownership map         |

Each lane owns its files exclusively until its focused gate passes. The integration owner resolves
only facade/re-export conflicts after lane handoff. Implementers do not edit generated API output
directly; they rerun its owner when a move changes generation inputs.

## Milestone plan

| M   | Title                     | Summary                                                                  | Goal                                                                          |
| --- | ------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| M1  | Extract capability owners | Four independent lanes move coherent behavior behind stable facades.     | Every inventoried owner is below 1,000 lines with focused behavior unchanged. |
| M2  | Enforce and close         | Add the permanent line gate, run the integrated suite, and refresh maps. | No oversized maintained source or undocumented ownership remains.             |

### Milestone M1: Extract capability owners

- Depends on: the passed WP-RC3 live gate, because its adapter/server/container changes establish
  the source baseline; no RC4 or payload implementation patch may overlap these owners during
  extraction.
- Deliverables: WP-SIZE1 through WP-SIZE4 source moves, focused tests, and per-lane ownership notes.
- Workstreams: WS-PERSIST, WS-SERVER, WS-ADAPTER, WS-BROWSER.
- Entry criteria: clean focused baseline tests for every affected package; the exact inventory command
  above has been rerun and its current result recorded in the package handoff; no unresolved index
  operation.
- Exit criteria:
  - Every file owned by each lane is at most 999 physical lines.
  - Existing public imports, generated contracts, HTTP fixtures, and behavior tests pass.
  - Each lane receives an independent review with no P0/P1 finding.
- Parallel-plan ready: yes -- max parallel doers: 4. Lanes have disjoint source ownership; only the
  integration owner edits cross-lane maps after all handoffs.

### Milestone M2: Enforce and close

- Depends on: WP-SIZE1, WP-SIZE2, WP-SIZE3, and WP-SIZE4, because the permanent gate has no
  grandfathered exceptions.
- Deliverables: WP-SIZE5 test, full validation evidence, documentation/status/changelog close-out.
- Workstreams: WS-CLOSE.
- Entry criteria: all M1 focused and independent-review gates pass.
- Exit criteria:
  - The permanent size test discovers the declared roots/extensions and reports zero violations.
  - `./check_codebase.sh`, repository Python tests, relevant Playwright, and both diff checks pass.
  - Architecture and file maps name every new module owner and no current doc recommends an
    oversized parent as an implementation warehouse.
- Parallel-plan ready: no -- final gate and documentation consume all four accepted lane handoffs and
  require one integration owner.

## Workstream breakdown

### Workstream WS-PERSIST: Persistence contracts and implementations

- Goal: make Store contracts, in-memory behavior, PostgreSQL behavior, and conformance tests
  capability-owned while preserving exact parity.
- Owner: `expert_coder` with `postgresql-expert` review.
- Work packages: WP-SIZE1.
- Interfaces:
  - Needs: current Store trait/public export inventory and PostgreSQL focused baselines.
  - Provides: stable facade paths and focused persistence modules consumed by server lanes.
- Review boundary, when modifying the repository: traits, SQL transactions, RLS context, error
  mapping, Memory/PostgreSQL conformance, and test fixture ownership.

### Workstream WS-SERVER: Server routes and composition

- Goal: separate route families, projections, tests, and composition capabilities without changing
  authorization or wire behavior.
- Owner: `expert_coder` with independent server-security review.
- Work packages: WP-SIZE2.
- Interfaces:
  - Needs: unchanged learning-data-access public facade.
  - Provides: focused route/composition owners consumed by browser and adapters.
- Review boundary, when modifying the repository: router construction, middleware/auth ordering,
  backend dispatch, status/headers/body, and focused route fixtures.

### Workstream WS-ADAPTER: Adapters and offline tooling

- Goal: separate provider protocol, projection, cache, CLI seed, statistics, and PDF capabilities.
- Owner: one `expert_coder` per crate, coordinated by an integrator.
- Work packages: WP-SIZE3.
- Interfaces:
  - Needs: unchanged question-model, Store, and server adapter traits.
  - Provides: stable adapter/tool public APIs and focused contract tests.
- Review boundary, when modifying the repository: upstream wire exactness, answer secrecy, CLI
  idempotency, generated artifact ownership, statistics semantics, and PDF output.

### Workstream WS-BROWSER: Browser contracts and widgets

- Goal: separate strict decoders, transport, mocks, and response-family UI controllers while keeping
  one public API surface.
- Owner: `coder` with TypeScript and HCI review.
- Work packages: WP-SIZE4.
- Interfaces:
  - Needs: unchanged generated API contract and route fixtures.
  - Provides: stable exports consumed by pages/features and focused browser tests.
- Review boundary, when modifying the repository: strict unknown-field refusal, auth/cookie transport,
  mock/real parity, keyboard/focus behavior, and TypeScript exports.

### Workstream WS-CLOSE: Policy enforcement and integration

- Goal: prevent regression and publish the final contributor map.
- Owner: `integrator`, independently reviewed by an `architect`.
- Work packages: WP-SIZE5.
- Interfaces:
  - Needs: all accepted extraction handoffs.
  - Provides: permanent size gate and release evidence.
- Review boundary, when modifying the repository: discovery scope, exclusions, full-suite evidence,
  architecture/status/changelog accuracy.

## Work packages

### Work package WP-SIZE1: Extract persistence capabilities

- Owner: `expert_coder`; `postgresql-expert` reviews SQL and transaction boundaries.
- Touch points: move `postgres.rs` to `postgres/mod.rs` and `in_memory.rs` to `in_memory/mod.rs`;
  extract remaining catalog, runs, feedback, sessions, manual grading, appearance, retention, jobs,
  exports, imports, and statistics owners into their existing subdirectories; split Store contract
  types/traits from `lib.rs` into `contracts/`; move `tests/conformance.rs` to
  `tests/conformance/mod.rs` and split capability suites; split the flat-import live provenance test
  by setup, success, and refusal behavior.
- Depends on: none within M1.
- Acceptance criteria:
  - Public learning-data-access types and Store method signatures remain source-compatible through
    re-exports.
  - SQL text, bind order, transaction boundaries, RLS session setup, and error mappings are
    behavior-identical.
  - Memory/PostgreSQL conformance, live provenance, migration, RLS, and strict Clippy gates pass.
  - Every touched maintained source is at most 999 lines.
- Evidence or review, when useful: package tests, focused live Store gates, `cargo check`, strict
  Clippy, rustdoc links, and independent PostgreSQL diff review.
- Obvious follow-ons: update `CODE_ARCHITECTURE.md` and `FILE_STRUCTURE.md` persistence maps in the
  WP-SIZE5 handoff; do not stop with stale path documentation.

### Work package WP-SIZE2: Extract server capabilities

- Owner: `expert_coder`; independent security reviewer.
- Touch points: move `run.rs`, `workspace.rs`, `composition.rs`, `catalog.rs`, `retention.rs`,
  `course.rs`, and `imathas_backend.rs` into same-name `mod.rs` facades where needed; extract route,
  command, projection, backend, policy, and test modules; split flat-question publication tests by
  authoring, publication, provenance, and refusal behavior.
- Depends on: none within M1; consumes only the stable persistence facade.
- Acceptance criteria:
  - Route paths, methods, middleware/auth order, status, cache headers, and JSON fixtures are
    unchanged.
  - Composition still fails closed on missing production capabilities and secrets.
  - Server package tests, route/security fixtures, strict Clippy, and generated-contract freshness
    pass.
  - Every touched maintained source is at most 999 lines.
- Evidence or review, when useful: focused route/backend tests before and after each extraction,
  server full suite, and independent authorization/wire diff review.
- Obvious follow-ons: route later payload/family implementation into the extracted owners rather than
  growing the facade.

### Work package WP-SIZE3: Extract adapter and tooling capabilities

- Owner: one `expert_coder` per crate; integration owner controls public re-exports.
- Touch points: WebWork `http_renderer/{client,protocol,response_shape,html_projection,grade,tests}`
  and `lib/{artifact,cache,issue,grade}` owners; native adapter source-family modules; iMathAS
  protocol/launch/result/broker modules; project-tools `e2e_seed/{cli,native,webwork,records,tests}`;
  domain statistics aggregation/disclosure/tests; PDF layout/render/tests; and version-tool
  `devel/bump_version/{cli,discovery,parsing,formatting,rewrite}.py` modules behind the stable
  `devel/bump_version.py` command facade.
- Depends on: none within M1; public question-model and Store contracts remain stable.
- Acceptance criteria:
  - Recorded upstream request/response fixtures, answer-free projections, cache identity, grading,
    S3 publication, statistics, and PDF behavior remain unchanged.
  - CLI arguments, stdout/stderr secrecy, deterministic IDs, and rerun semantics remain unchanged.
  - `python3 devel/bump_version.py --help`, `--help-advanced`, and dry-run discovery remain
    unchanged and are verified with one-time command probes before extraction is accepted. Pure
    version parsing, Cargo normalization, and atomic rewrite behavior remain importable for future
    behavior tests when a concrete regression warrants one; CLI wiring and the current discovered
    path inventory do not receive a permanent pytest solely to prove this refactor.
  - Each crate's tests, strict Clippy, fixture hashes, and generated output checks pass.
  - Every touched maintained source is at most 999 lines.
- Evidence or review, when useful: per-crate focused suites and separate WebWork/iMathAS security
  review after extraction.
- Obvious follow-ons: WP-RC4 and payload owners extend the new family/protocol modules, never the
  facade.

### Work package WP-SIZE4: Extract browser capabilities

- Owner: `coder`; TypeScript and HCI reviewers remain separate.
- Touch points: `src/api/decoders/{auth,catalog,courses,runs,authoring,assets}.ts`,
  `src/api/http_client/{request,response,auth,error}.ts`,
  `src/api/mock/handlers/{auth,catalog,courses,runs,authoring,assets}.ts`, and
  `src/components/responses/` family controllers extracted from `response_widget.tsx`.
- Depends on: none within M1; generated and HTTP contracts remain stable.
- Acceptance criteria:
  - Existing exported imports remain available through facade re-exports.
  - Strict decoders retain bounds and unknown-field refusal; mocks decode the same wire as real
    transport; credentials remain out of storage/logs.
  - All response-family keyboard, focus, validation, and Playwright behavior remains unchanged.
  - TypeScript checks, ESLint at zero warnings, Node tests, focused Playwright, and Prettier pass.
  - Every touched maintained source is at most 999 lines.
- Evidence or review, when useful: decoder fixture parity, network trace, keyboard tests, and fresh
  TypeScript/HCI review.
- Obvious follow-ons: payload and flat-family work adds one decoder/widget owner per new family.

### Work package WP-SIZE5: Enforce source-size ownership

- Owner: `integrator`; independent `architect` review.
- Touch points: `tests/test_source_file_line_limit.py`,
  `tests/source_file_line_limit_overrides.txt`; `docs/CODE_ARCHITECTURE.md`,
  `docs/FILE_STRUCTURE.md`, release/status docs, and `docs/CHANGELOG.md`.
- Depends on: WP-SIZE1, WP-SIZE2, WP-SIZE3, and WP-SIZE4.
- Acceptance criteria:
  - Regenerate the exact inventory at package start and acceptance; reconcile its current violation
    list against this dated baseline before accepting the package.
  - Discovery begins at the repository root and uses the test's closed source-extension and
    conventional-filename sets. Exact override entries may cover only manager-approved immutable
    migration ledgers or documentation/history artifacts; a Rust, TypeScript, JavaScript, Python,
    shell, or maintained test module may not be overridden.
  - The exact excluded prefixes are `.git/`, `.pytest_cache/`, `.venv/`, `OTHER_REPOS/`,
    `coverage/`, `dist/`, `dist_wasm/`, `generated/`, `node_modules/`, `playwright-report/`,
    `target/`, `test-results/`, `docs/archive/`, `tests/fixtures/`, `tests/artifacts/`,
    `tests/e2e/fixtures/`, and `tests/playwright/fixtures/`. The only filename exceptions are the
    reviewed exact paths in `tests/source_file_line_limit_overrides.txt`.
  - Discovery does not follow directory symlinks and rejects a discovered maintained-source file
    that is itself a symlink. It reads bytes, rejects NUL-containing or invalid UTF-8 input, and
    counts physical lines as zero for an empty file or `LF count + 1` when the final line has no LF;
    CRLF is one line terminator. These are policy failures with the relative path in the message.
  - It fails with every relative path and line count at 1,000 or more, and a mutation test proves
    both 999-pass and 1,000-fail boundaries.
  - The full repository gate passes and an independent reviewer reports no P0/P1 architecture or
    behavior regression.
- Evidence or review, when useful: focused pytest, synthetic boundary fixture, full Rust/TypeScript/
  browser/Python gates, line inventory, docs checks, and both diff checks.
- Obvious follow-ons: make this gate part of every later package and route new growth into the mapped
  capability owner immediately.

## Acceptance criteria and gates

- Per-patch gate: one capability extraction at a time; focused pre/post behavior passes; touched
  files are below 1,000 lines; formatting, lint, compile, and diff checks are clean.
- Integration gate: zero permanent size-test violations; `./check_codebase.sh`, repository pytest,
  generated contracts, required live Store tests, and focused built-browser suites pass.
- Independent review gate: separate persistence, security, TypeScript/HCI, and final architecture
  reviewers report no P0/P1; the implementer does not self-accept.

The copy-paste validation contract is:

```bash
# WP-SIZE1: persistence contracts, both implementations, and conformance.
cargo check -p learning-data-access --all-targets
cargo test -p learning-data-access --all-targets
cargo clippy -p learning-data-access --all-targets -- -D warnings
bash tests/e2e/e2e_database_baseline.sh

# WP-SIZE2: server routes, backends, and composition.
cargo check -p server_core --all-targets
cargo test -p server_core --all-targets
cargo clippy -p server_core --all-targets -- -D warnings

# WP-SIZE3: adapters, tooling, domain statistics, PDF export, and version CLI.
cargo test -p adapter_webwork -p adapter_native -p adapter_imathas -p project-tools -p domain -p export_crate --all-targets
cargo clippy -p adapter_webwork -p adapter_native -p adapter_imathas -p project-tools -p domain -p export_crate --all-targets -- -D warnings
python3 -m py_compile devel/bump_version.py devel/bump_version/*.py
python3 devel/bump_version.py --help
python3 devel/bump_version.py --help-advanced
python3 devel/bump_version.py --source VERSION --bump patch --dry-run

# WP-SIZE4: browser contracts and learner behavior.
npx tsc --noEmit -p tsconfig.json
npx tsc --noEmit -p tsconfig.lint.json
npx eslint --max-warnings 0 src tests
node --import tsx --test tests/test_*.mjs
npx playwright test tests/playwright/frontend_contract.spec.ts tests/playwright/student_keyboard_accessibility.spec.ts --workers=1

# WP-SIZE5: permanent policy and integrated close-out.
source source_me.sh && python3 -m pytest -q tests/test_source_file_line_limit.py
./check_codebase.sh
source source_me.sh && python3 -m pytest -q tests
npx playwright test --workers=1
git diff --check
git diff --cached --check
```

`tests/test_source_file_line_limit.py` is the permanent architecture boundary. It checks maintained
tracked sources through the shared hygiene discovery contract and pins the meaningful 999-pass /
1,000-fail behavior. Each WP-ARCH1 package regenerates the exact inventory at its start and
acceptance, records its pre-extraction focused command output, and reruns the same commands after
every capability move; a green final gate cannot replace that before/after evidence.

Exact symbol inventories, module-name lists, pre/post file counts, compiler-error probes, migration
spot-checks, and other assertions whose only purpose is to prove this one-time extraction are
implementation evidence, not permanent tests. Keep those checks as copy-paste commands or disposable
scratch probes and remove them before package acceptance. A new permanent pytest must satisfy every
item in `docs/PYTEST_STYLE.md`; when its lasting behavioral value is uncertain, omit or delete it.

## Test and verification strategy

Each lane regenerates the exact inventory at package start and acceptance, records its
pre-extraction focused command, and reruns it after each capability move. Rust lanes run package
tests, `cargo check --all-targets`, strict Clippy, rustfmt, and relevant live Store or protocol gates.
The browser lane runs strict TypeScript, ESLint, unit tests, generated-contract checks, and
behavior-focused Playwright. WP-SIZE5 runs the permanent line policy, the complete repository gate,
the full Python suite, Markdown/ASCII/whitespace checks, and both diff checks.

The permanent suite retains existing behavior, security, tenancy, serialization, and Store
conformance tests plus the source-size boundary itself. It does not gain tests for exact extracted
file names, source line counts below the boundary, facade method lists, or today's module layout;
those would freeze implementation details rather than protect behavior.

A move fails when public imports break, fixtures change without an owning generator, route bytes or
status change, a security review finds an authorization-order difference, SQL/transaction evidence
changes, or behavior tests regress. The owner restores the prior capability boundary before trying a
different extraction; it does not add a compatibility shim solely to make the split compile.

## Risk register

| Risk                                  | Impact | Trigger                                                            | Owner      | Mitigation                                                                                     |
| ------------------------------------- | ------ | ------------------------------------------------------------------ | ---------- | ---------------------------------------------------------------------------------------------- |
| Hidden behavior change                | High   | Fixture, SQL, route, or UI output changes during a move            | Lane owner | One-capability patches with focused before/after evidence and independent review               |
| Circular module dependencies          | High   | Facade must import a child that imports the facade                 | Lane owner | Move shared types downward into one contract module; prohibit peer back-edges                  |
| Merge conflicts with feature work     | High   | RC4/payload patch edits an active extraction owner                 | Integrator | Accept after RC3 and before RC4; exclusive lane ownership until handoff                        |
| Cosmetic split without ownership gain | Medium | New file has no capability name or independent tests               | Architect  | Reject line-range/include fragments; require mapped responsibility and review boundary         |
| Size test skips real source           | Medium | Oversized maintained file outside policy or an overbroad exception | WP-SIZE5   | Closed discovery contract, artifact-only exact overrides, and independent inventory comparison |
| Documentation drift                   | Medium | Contributor map points to removed parent implementation            | WP-SIZE5   | Architecture/file-map/link gates are acceptance requirements                                   |

## Rollout and release checklist

- [x] Regenerate the exact inventory at implementation start and close-out; reconcile the zero-code
      result with the dated 2026-08-10 baseline snapshot.
- [x] Capture focused baselines and preserve overlapping feature behavior.
- [x] Complete and validate WP-SIZE1 persistence extraction.
- [x] Obtain the independent PostgreSQL review for WP-SIZE1.
- [x] Complete and validate WP-SIZE2 server extraction.
- [x] Obtain the independent security review for WP-SIZE2.
- [x] Complete and validate WP-SIZE3 adapter/tooling extraction.
- [x] Obtain the independent provider-security reviews for WP-SIZE3.
- [x] Complete and validate WP-SIZE4 browser extraction.
- [x] Obtain the independent TypeScript/HCI review for WP-SIZE4.
- [x] Land the source-size gate, prove the 999/1,000 boundary, and verify that every exact override
      is a frozen migration or documentation/history artifact rather than maintained code.
- [x] Run the disposable PostgreSQL baseline through the decomposed Store and live integration-test
      owners, including the atomic flat-import provenance test.
- [x] Run the integrated repository and browser gates.
- [x] Refresh architecture, file map, status, and changelog; record final inventory.
- [x] Obtain final independent architecture review before WP-RC4 begins.

## Documentation close-out requirements

- Active plan and status: update this plan, the release-completion plan, implementation status, and
  the dated status report with exact accepted evidence.
- `docs/CHANGELOG.md`: record the capability owners, stable-facade outcome, final file-count result,
  validation commands, and review status.
- Architecture/file map: replace parent-warehouse descriptions with exact module ownership and point
  future WP-RC4/payload work at the focused owners.
- Closure evidence: write `docs/active_plans/reports/source_module_decomposition_evidence.md` with
  before/after inventory, commands, generated-artifact checks, and independent findings.

## Patch plan and reporting format

- Patch 1: persistence contracts/implementations/conformance extraction (WP-SIZE1).
- Patch 2: server route/composition/test extraction (WP-SIZE2).
- Patch 3: adapter, project-tool, statistics, and PDF extraction (WP-SIZE3).
- Patch 4: browser decoder/transport/mock/widget extraction (WP-SIZE4).
- Patch 5: permanent size policy, integrated evidence, documentation, and acceptance (WP-SIZE5).

Each handoff reports owned files, capability moves, stable facade, focused commands/results, line
inventory, generated artifacts, index state, residual risk, and independent-review status. No
handoff may claim completion while its permanent sources remain at 1,000 lines or more.
