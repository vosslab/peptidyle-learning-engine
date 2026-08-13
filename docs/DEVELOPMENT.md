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
./check_codebase.sh
pytest tests/
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

The check gate is deliberately not a product build. It refreshes the ignored generated TypeScript
projections, then runs TypeScript typechecks, ESLint, Prettier, Node tests, the focused WASM
dependency-boundary check, Rust formatting, Clippy, and workspace tests. A missing Rust toolchain
is reported as `SKIP`; it is not a successful Rust verification.

Cargo features are capability boundaries, not a convention to enable globally. The server selects
the production PostgreSQL, S3, and adapter capabilities in its manifest, while memory-oriented
crates keep those dependencies optional. `./check_codebase.sh` checks the workspace configuration
it owns with `--all-targets`; for a change to an optional capability, read the owning `Cargo.toml`
and run the focused package command required by the active work package.

## Generated outputs

Treat these paths as build products, not hand-maintained source:

- `generated/` contains ignored TypeScript definitions and fixture projections generated from Rust
  contracts and checked fixture sources.
- `dist_wasm/` contains the generated WebAssembly bridge and JavaScript glue.
- `dist/` contains the browser bundle and receives the WebAssembly assets under `dist/wasm/`.
- `target/` contains Cargo build products.

Change the Rust contract, fixture source, browser source, or build pipeline that owns an output;
then rerun the appropriate front-door script. Do not edit a derived file to make a build appear
current.

## Choose the right gate

Run the narrowest gate that proves the changed behavior, then the broader gate required by the
active work package.

| Change or concern                                          | Command                                                     | What it proves                                     |
| ---------------------------------------------------------- | ----------------------------------------------------------- | -------------------------------------------------- |
| Rust, generated contracts, TypeScript, lint, or formatting | `./check_codebase.sh`                                       | The repository's fast cross-language gate.         |
| Repository documentation and hygiene                       | `pytest tests/`                                             | Fast Python hygiene and repository-rule checks.    |
| Built-browser behavior                                     | `./run_playwright_tests.sh --build`                         | A fresh browser artifact and the Playwright suite. |
| One browser scenario                                       | `./run_playwright_tests.sh tests/playwright/<file>.spec.ts` | The selected built-browser scenario.               |
| Container-backed behavior                                  | `bash tests/e2e/e2e_<name>.sh`                              | The named disposable whole-system oracle.          |

`tests/playwright/` is browser-driven testing and `tests/e2e/` is non-browser whole-system
orchestration. Both are intentionally excluded from `pytest tests/`; see
[E2E_TESTS.md](E2E_TESTS.md) for the test-tier boundary. Install the browser binaries once with
`npm run setup:playwright` before running Playwright.

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
contract and `bash tests/e2e/e2e_chapter_one_pilot.sh` for the disposable PostgreSQL/MinIO
publication, exact two-by-four assignment matrix, human display identity, and idempotent-rerun
contract. Run `bash tests/e2e/e2e_chapter_one_browser.sh` for the complete eight-question
keyboard-driven learner path through the built PLE browser and private renderer.

## Run local services

Use the launcher when a work package needs the supported local PostgreSQL, MinIO, API, worker,
gateway, and external stateless WeBWorK PG renderer:

```bash
./launch_local_stack.sh --check
./launch_local_stack.sh --no-open
```

`--check` validates configuration without changing state. A normal launcher run can create ignored
local credentials and starts the supported stack; do not copy those local secret files into source
control. It uses an already-built `webwork-pg-renderer` image rather than building a second WeBWorK
platform or database. SMTP is an operator-selected external service enabled only with `--with-smtp`.
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) documents the stack, configuration,
recovery, and service commands.

## Prepare a handoff

- Review the diff and run the gates appropriate to the changed owner and active plan.
- Update frozen contracts and their consumers together; contract-only changes are incomplete.
- Preserve an existing mixed staged/unstaged worktree; do not stage unrelated changes. Only humans
  run `git commit` in this repository.
- Report the commands run, their results, and any unrun live gate so a reviewer can reproduce the
  evidence.
