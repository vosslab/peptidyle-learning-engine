# Development guide

This guide gives contributors the repository's supported edit, build, and verification paths. The
active implementation order, architecture, and acceptance gates remain in
[active_plans/implementation_plan.md](active_plans/implementation_plan.md).

## Start an edit

- Read the relevant contract in [CONTRACTS.md](CONTRACTS.md) before changing a frozen module
  boundary.
- Make the smallest change in the module that owns the behavior. The component map in
  [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) and path map in
  [FILE_STRUCTURE.md](FILE_STRUCTURE.md) identify those owners.
- Keep PLE-owned source files at 999 physical lines or fewer. Move a complete capability to a
  focused module before a parent file becomes an implementation warehouse. The line-limit gate
  permits only exact manager-approved immutable migration or documentation/history exceptions;
  see [REPO_STYLE.md](REPO_STYLE.md#source-file-size).
- Follow the language and test rules in [TYPESCRIPT_STYLE.md](TYPESCRIPT_STYLE.md),
  [RUST_STYLE.md](RUST_STYLE.md), [PYTEST_STYLE.md](PYTEST_STYLE.md),
  [REPO_STYLE.md](REPO_STYLE.md), and [MARKDOWN_STYLE.md](MARKDOWN_STYLE.md). This guide does
  not duplicate their detailed conventions.

## Set up and build

Install the checked-in JavaScript dependencies once, then use the root scripts as the normal
interface:

```bash
npm run setup
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
```

`./build.sh` builds the Rust workspace, WebAssembly bridge, Rust-owned TypeScript definitions,
fixture projection, and Solid browser bundle in dependency order. Use `./build.sh --release` for
optimized host artifacts. `npm run build` and `npm run check` are aliases for the build and check
scripts.

PLE's measured development profile controls disk growth: ordinary dev and test builds disable
incremental compilation and retain line-table debug information for useful filename/line
backtraces without full variable/type debugger data. A deliberate debugging session may temporarily
override those defaults with `CARGO_INCREMENTAL=1`, `CARGO_PROFILE_DEV_DEBUG=full`, or
`CARGO_PROFILE_TEST_DEBUG=full`; do not make those high-storage settings the shared default.

Cargo does not impose a byte ceiling on a workspace `target/` directory. Its retained dependency
artifacts still accelerate repeat builds, so establish any repository ceiling from the measured
size of a clean broad gate rather than treating zero cache as the goal. After two full local-stack
browser builds, a workspace all-target check, and strict workspace Clippy on 2026-08-12, the reduced
profile retained 6.0 GB total (`debug/deps` 4.2 GB and `debug/incremental` 0). Treat 20 GB as the local
investigation threshold: stop adding build matrices, identify unexpected profiles/fingerprints, and
decide explicitly whether the rebuildable target cache should be cleaned.

Cargo artifacts are entirely rebuildable. Before reclaiming space, confirm no Cargo or `rustc`
process is active, inspect the exact target with `cargo clean --dry-run`, and use `cargo clean` only
when discarding the complete workspace build cache is intended. The command does not remove source,
Git state, `node_modules`, or Podman data.

The check gates are deliberately not product builds. The vendored `./check_codebase.sh` owns the
TypeScript typechecks, ESLint, Prettier, and Node tests. The repository-owned `./check_rust.sh`
owns Rust-generated browser contracts and fixtures, Rust formatting, default and all-feature
compilation, strict Clippy, workspace tests and doctests, and the browser WebAssembly target check.
Keeping them separate prevents a vendored codebase-gate refresh from silently removing Rust
verification.

Cargo features are capability boundaries, not a convention to enable globally. The server selects
the production PostgreSQL, S3, and adapter capabilities in its manifest, while memory-oriented
crates keep those dependencies optional. `./check_rust.sh` checks both the default production graph
and the all-feature, all-target graph. For a change to an optional capability, also read the owning
`Cargo.toml` and run the focused package command required by the active work package.

## Generated outputs

Treat these paths as build products, not hand-maintained source:

- `generated/` contains ignored TypeScript definitions and fixture projections generated from Rust
  contracts and checked fixture sources.
- `dist_wasm/` contains the generated WebAssembly bridge and JavaScript glue.
- `dist/` contains the browser bundle and receives the WebAssembly assets under `dist/wasm/`.
- `dist_browser_test/` contains the ignored browser-test artifact and test-double transport assets,
  served only by the Playwright helper; it is separate from the installed Base Course lifecycle.
- `target/` contains Cargo build products.

Change the Rust contract, fixture source, browser source, or build pipeline that owns an output;
then rerun the appropriate front-door script. Do not edit a derived file to make a build appear
current.

## Disposable fixture identities

The Chapter 1 disposable seed mints fresh opaque workspace, problem, version,
and source-object IDs. Its answer-free protected manifest is the replay marker:
replay resolves the assigned Question IDs and requires exact immutable records
and reviewed source content before reuse. The manifest must never appear in
instructor-visible UI, URLs, copyable links, or public fixtures.

## Choose the right gate

Run the narrowest gate that proves the changed behavior, then the broader gate required by the
active work package.

| Change or concern                          | Command                                                     | What it proves                                                                            |
| ------------------------------------------ | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Rust code, features, lints, tests, or Wasm | `./check_rust.sh`                                           | The complete offline Cargo and Rust gate.                                                 |
| TypeScript, browser lint, format, or tests | `./check_codebase.sh`                                       | The vendored TypeScript and Node gate.                                                    |
| Repository documentation and hygiene       | `source source_me.sh && python3 -m pytest tests/`           | Fast Python hygiene and repository-rule checks.                                           |
| Built-browser behavior                     | `./run_playwright_tests.sh --build`                         | The ordinary demo-environment browser suite, with no skips.                               |
| Complete Playwright validation             | `source source_me.sh && python3 local_stack.py acceptance`  | Ordinary browser coverage plus required visual, walkthrough, and live-browser acceptance. |
| One browser scenario                       | `./run_playwright_tests.sh tests/playwright/<file>.spec.ts` | The selected built-browser scenario.                                                      |
| Container-backed behavior                  | `bash tests/e2e/e2e_<name>.sh`                              | The named disposable whole-system oracle.                                                 |
| Local stack diagnosis and lifecycle        | `source source_me.sh && python3 local_stack.py <command>`   | The scoped controller contract.                                                           |

`tests/playwright/` is browser-driven testing and `tests/e2e/` is non-browser whole-system
orchestration. Both are intentionally excluded from `pytest tests/`; see
[E2E_TESTS.md](E2E_TESTS.md) for the test-tier boundary. Install the browser binaries once with
`npm run setup:playwright` before running Playwright.

### Playwright execution lanes

`./run_playwright_tests.sh --build` is the ordinary fast browser gate. It builds
`dist_browser_test/`, starts the browser-test helper configured in `playwright.config.ts`, and
serves the bundle with browser-test/test-double transport handlers. It must finish with no skipped
tests. It neither requires nor reuses a Podman PLE stack. Real-stack, walkthrough, and visual
evidence cases are deliberately outside its collection; they are not ordinary tests that happened
to skip.

Run the complete Playwright Validation test suite explicitly when the active plan requires all
browser claims:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

Start from no existing default or retained walkthrough stack. The command refuses inherited live-target,
credential, and Compose overrides; it owns the temporary visual output and invokes each live or
walkthrough owner with its documented private inputs. A failed or skipped required lane is red, so
the suite is not green until every lane passes. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md)
for the evidence boundary and [USAGE.md](USAGE.md#build-and-validation-commands) for operator
preconditions.

In-memory and other offline contract tests belong in the normal Rust and Node gates. PostgreSQL,
MinIO, role/RLS, migration, restart, and private-renderer claims require their named E2E runner and
real disposable services. Do not treat a memory-backend pass as evidence for a live storage or
authorization boundary.

The roster schema is pre-production-only. After changing its checked-in baseline, discard and
recreate the disposable PostgreSQL volume before rerunning SQLx; a ledger checksum mismatch is a
clean-volume reset signal, never an instruction to edit the database ledger in place.

Keep permanent tests small, deterministic, and behavior-focused. A one-time migration probe,
manual inspection, or live diagnostic is useful implementation evidence, but it belongs in the
work-package record rather than the permanent fast suite unless it satisfies the checklist in
[PYTEST_STYLE.md](PYTEST_STYLE.md#is-this-a-good-pytest). Record both the evidence run and any
unrun live boundary in the handoff.

For Python tools, make routine operator choices visible in the small `argparse`
surface or in an explicitly selected config file. Do not add undocumented
environment switches for test modes, paths, ports, or child-process values.
Fixed child processes receive a single versioned private input file by an
explicit argument, with each reader validating the file's schema and private
filesystem boundary. Test that durable contract offline; record the real
Podman/browser execution separately as one-time evidence.

For the first teaching corpus, run `cargo tools pilot-content` for the tracked source/compiler
contract and `source source_me.sh && python3 tests/e2e/e2e_chapter_one_pilot.py` for the disposable
PostgreSQL/MinIO publication, exact two-by-four assignment matrix, human display identity, and
idempotent-rerun contract. Run
`source source_me.sh && python3 tests/e2e/e2e_chapter_one_browser.py` for the complete eight-question
keyboard-driven learner path through the built PLE browser and private renderer. The remaining
pilot shell file is only a compatibility `exec` facade; it does not own orchestration.

Chapter 1 replay is manifest-resume only. The answer-free host-only manifest records the assigned
Question IDs and exact immutable internal references from the first publication; a replay resolves
those Question IDs and verifies the same reviewed content before reuse. Keep the protected local
manifest with a retained corpus: if it is missing, the local controller refuses to mint duplicates.

## Run local services

Use the controller when a work package needs the supported local PostgreSQL,
MinIO, API, worker, gateway, and external stateless WeBWorK PG renderer:

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py validate
source source_me.sh && python3 local_stack.py start --no-open
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs gateway api worker
source source_me.sh && python3 local_stack.py restart api
source source_me.sh && python3 local_stack.py stop
```

`doctor`, `projects`, `status`, `logs`, and `validate` are read-only. `start`
delegates the application bootstrap to the launcher; a default first start can
create ignored local credentials and starts the supported stack. Do not copy
those local secret files into source control. `stop` retains named volumes.
`restart` is restricted to the stateless API, worker, gateway, and renderer
services, and delegates back to the launcher for readiness verification.

For deliberately disposable default data, run `reset --dry-run` first, inspect
its exact labelled project/resource and database-bound
`containers/local-chapter-one-pilot.json` preview, and then use
`reset --confirm-project containers`. Once the labelled Compose resources and
volumes are gone, reset removes that private Chapter 1 replay record. Reset
retains local host credentials. An ordinary start first removes every container
in its exact labelled project, including Compose orphans, while retaining the
three named simulated-data volumes. It then recreates the complete designed
service suite. After readiness, it prunes only dangling images carrying the
reviewed PLE or local-renderer source label. Global Podman pruning and all other
image cleanup retain their dedicated operator workflow. Raw Compose is a diagnosis or recovery
interface only; normal changes use the controller's project and environment
handling.

The controller uses an already-built `webwork-pg-renderer` image rather than
building a second WeBWorK platform or database. SMTP is an operator-selected
external service enabled only with `--with-smtp`; status safely infers a
persisted SMTP overlay from its labelled resources. [LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md)
documents the stack, configuration, recovery, and service commands.

### Validation classes

Keep controller parsing, ownership, confirmation, and topology behavior in
fast deterministic permanent tests. Run Podman, PostgreSQL, MinIO, renderer,
restart, visual, walkthrough, and browser evidence only through their named
opt-in disposable/live commands. A focused probe while rebuilding a workflow
is useful evidence, but does not become a permanent test unless it satisfies
the repository checklist in [PYTEST_STYLE.md](PYTEST_STYLE.md#is-this-a-good-pytest).

Every goal must finish the active plan's full Validation test suite on the
final material tree. The suite includes all required permanent gates, named
service/browser acceptance, and independent reviews; an unrun or required
skipped live gate is not green. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md#validation-test-suite)
for the evidence model and completion rule.

## Prepare a handoff

- Review the diff and run the gates appropriate to the changed owner and active plan.
- Update frozen contracts and their consumers together; contract-only changes are incomplete.
- Preserve an existing mixed staged/unstaged worktree; do not stage unrelated changes. Only humans
  run `git commit` in this repository.
- Report the commands run, their results, and any unrun live gate so a reviewer can reproduce the
  evidence.
