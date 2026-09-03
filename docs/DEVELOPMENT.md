# Development guide

This guide gives contributors the repository's supported edit, build, and verification paths. The
active implementation order, architecture, and acceptance gates remain in
[active_plans/implementation_plan.md](active_plans/implementation_plan.md).

## Start an edit

- Read the relevant contract in [CONTRACTS.md](CONTRACTS.md) before changing a frozen module
  boundary.
- For database, identity, authorization, or generated-contract work, read
  [TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md) with the
  [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md#glossary) glossary before selecting a model.
- Make the smallest change in the module that owns the behavior. The component map in
  [CODE_ARCHITECTURE.md](CODE_ARCHITECTURE.md) and path map in
  [FILE_STRUCTURE.md](FILE_STRUCTURE.md) identify those owners.
- Keep PLE-owned source files at 999 physical lines or fewer. Move a complete capability to a
  focused module before a parent file becomes an implementation warehouse. The line-limit gate
  permits only exact manager-approved immutable migration or documentation/history exceptions;
  see [REPO_STYLE.md](REPO_STYLE.md#source-file-size).
- Follow the language and test rules in [TYPESCRIPT_STYLE.md](TYPESCRIPT_STYLE.md),
  [RUST_STYLE.md](RUST_STYLE.md), [PYTEST_STYLE.md](PYTEST_STYLE.md),
  [NAMING_CONVENTIONS.md](NAMING_CONVENTIONS.md), [REPO_STYLE.md](REPO_STYLE.md), and
  [MARKDOWN_STYLE.md](MARKDOWN_STYLE.md). This guide does not duplicate their detailed conventions.

## Set up and build

Install the checked-in JavaScript dependencies once, then use the root scripts as the normal
build and validation interface. The TypeScript and Playwright installers are propagated developer
helpers; the root build and validation scripts are repository-owned front doors:

```bash
source source_me.sh && python3 -m pip install --requirement pip_requirements.txt --requirement pip_requirements-dev.txt
./devel/setup_typescript.sh
./devel/setup_playwright.sh       # once, before browser tests
./devel/setup_wasm_tests.sh       # only when running wasm-bindgen tests
./build.sh
./check_rust.sh
./check_codebase.sh
source source_me.sh && python3 -m pytest tests/
```

`devel/DEVEL_README.md` indexes maintainer-only helpers. Use `devel/clean_build.sh` for a light
rebuildable-output cleanup (`npm run clean`), and `devel/dist_clean.sh` for a distribution-clean
reset that also removes `node_modules`, generated outputs, and Cargo's `target/`. Use
`devel/reset_podman.sh --dry-run` to preview the fixed disposable Podman resources; its unqualified
form is destructive and follows the explicit fixed-project confirmation owned by `local_stack.py`.

`./build.sh` builds the Rust workspace, WebAssembly bridge, Rust-owned TypeScript definitions,
fixture projection, and Solid browser bundle in dependency order. Use `./build.sh --release` for
optimized host artifacts. `npm run build` and `npm run check` are aliases for the build and check
scripts.

### Inspect command surfaces

Use the read-only help surfaces to confirm available local-stack and browser options before
starting a service or acceptance lane:

```bash
source source_me.sh && python3 local_stack.py --help
./run_live_demo.sh --help
```

The help commands do not start containers, open a browser, or mutate generated artifacts. Choose
the narrowest command that proves the behavior under review before running a broader gate.

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

| Change or concern                          | Command                                                    | What it proves                                                       |
| ------------------------------------------ | ---------------------------------------------------------- | -------------------------------------------------------------------- |
| Rust code, features, lints, tests, or Wasm | `./check_rust.sh`                                          | The complete offline Cargo and Rust gate.                            |
| TypeScript, browser lint, format, or tests | `./check_codebase.sh`                                      | The vendored TypeScript and Node gate.                               |
| Repository documentation and hygiene       | `source source_me.sh && python3 -m pytest tests/`          | Fast Python hygiene and repository-rule checks.                      |
| Connected current acceptance               | `source source_me.sh && python3 local_stack.py acceptance` | Current database/object service receipts under the typed controller. |
| Container-backed behavior                  | `bash tests/e2e/e2e_<name>.sh`                             | The named disposable whole-system oracle.                            |
| Local stack diagnosis and lifecycle        | `source source_me.sh && python3 local_stack.py <command>`  | The scoped controller contract.                                      |

`tests/playwright/` is browser-driven testing and `tests/e2e/` is non-browser whole-system
orchestration. Both are intentionally excluded from `pytest tests/`; see
[E2E_TESTS.md](E2E_TESTS.md) for the test-tier boundary. Install the browser binaries once with
`./devel/setup_playwright.sh` (or `npm run setup:playwright`) before running Playwright.
The fresh Store-backed browser owner and visual-publication path return after the mounted
course-delivery surface is rebuilt. The current aggregate validates the active code and service
contracts without claiming browser or screenshot acceptance.

### Permanent and connected evidence

Permanent offline gates are the deterministic behavior, contract, security, hygiene, Rust, and
Node checks owned by `./check_rust.sh`, `./check_codebase.sh`, and
`source source_me.sh && python3 -m pytest tests/`. They run without PostgreSQL, MinIO, the
renderer, or a browser and do not prove those external boundaries.

Connected and one-time evidence is opt-in and remains separate from the permanent fast lane:

- `source source_me.sh && python3 local_stack.py acceptance` runs the complete connected acceptance
  lane and its current browser-free service oracles.
- Named `tests/e2e/` runners, migration probes, rendered captures, manual inspections, and load or
  query-plan observations prove only their stated disposable or one-time claim.

Do not promote a probe, inventory, screenshot, count, or live diagnostic to a permanent test unless
it satisfies the admission rules in [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) and
[PYTEST_STYLE.md](PYTEST_STYLE.md#is-this-a-good-pytest).

### Future browser execution

The fresh Store-backed browser owner will build production `dist/`, serve it through the HTTPS PLE
gateway, and create product state through visible PLE controls. Until that owner and the mounted
course-delivery routes exist together, browser evidence remains unclaimed.

Run the complete Playwright Validation test suite explicitly when the active plan requires all
browser claims:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

The command invokes the canonical browser lane once with its documented private
inputs and retains only browser-free service receipts. A failed or skipped
required lane is red, so the suite is not green until every lane passes. See
[TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md) for the evidence boundary and
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for live-stack operator preconditions.

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

For the first teaching set, run `cargo tools pilot-content` for the tracked
source/compiler contract. Fixed seed/manifest and Rust behavior tests own its
publication semantics; the canonical live-demo lifecycle installs that baseline.
Browser scenarios remain staged source until the fresh browser owner is present.

Chapter 1 replay is manifest-resume only. The answer-free host-only manifest records the assigned
Question IDs and exact immutable internal references from the first publication; a replay resolves
those Question IDs and verifies the same reviewed content before reuse. Keep the protected local
manifest with a retained teaching set: if it is missing, the local controller refuses to mint duplicates.

## Run local services

Use the fixed owner when a work package needs the supported PostgreSQL, MinIO,
API, worker, gateway, and external stateless WeBWorK PG renderer:

```bash
source source_me.sh && python3 local_stack.py start --headless
source source_me.sh && python3 local_stack.py stop
```

`start` first authenticates to and cleans the previous developer owner, then builds production
`dist/`, regenerates the fixed `ple-live-demo-browser` disposable stack, and opens (or prints with
`--headless`) its HTTPS origin. `stop` performs the same exact owner cleanup without launching a
replacement. Developer and browser tests serialize through the same owner lease; do not add project,
environment, identity, SMTP, or build selectors.

The fixed owner uses the reviewed standalone `webwork-pg-renderer` image rather
than building a second WeBWorK platform or database. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for the owner and cleanup
contract.

The convenience wrapper `./run_live_demo.sh` starts or stops that same owner. It sources the
repository shell environment through its fixed `source_me.sh` path, runs `python3 local_stack.py`,
and installs TypeScript dependencies when `node_modules` is absent. Use it for a human demo; use
`source source_me.sh && python3 local_stack.py <command>` directly when selecting a controller
command or collecting diagnostics.

`source_me.sh` is a shell precondition: it requires Bash and loads the repository's shell setup.
`run_live_demo.sh` and direct controller diagnostics use that shell environment and `python3`.
Install the live-demo runtime dependency with
`source source_me.sh && python3 -m pip install --requirement pip_requirements.txt`.
For pytest and aggregate-validation commands, install both declared requirement files with
`source source_me.sh && python3 -m pip install --requirement pip_requirements.txt --requirement pip_requirements-dev.txt`.
All commands use the same selected `python3` interpreter.

### Validation classes

Keep controller parsing, ownership, confirmation, and topology behavior in
fast deterministic permanent tests. Run Podman, PostgreSQL, MinIO, renderer,
restart, and browser evidence only through their named opt-in disposable/live
commands. A focused probe while rebuilding a workflow
is useful evidence, but does not become a permanent test unless it satisfies
the repository checklist in [PYTEST_STYLE.md](PYTEST_STYLE.md#is-this-a-good-pytest).

Every goal must finish the active plan's full Validation test suite on the
final material tree. The suite includes all required permanent gates, named
service/browser acceptance, and independent reviews; an unrun or required
skipped live gate is not green. See [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md#validation-test-suite)
for the evidence model and completion rule.

The exact final aggregate is:

```bash
source source_me.sh && ./all_test.sh
```

Run it on the final material tree after the package's focused and connected gates are green.

## Prepare a handoff

- Review the diff and run the gates appropriate to the changed owner and active plan.
- Update frozen contracts and their consumers together; contract-only changes are incomplete.
- Preserve an existing mixed staged/unstaged worktree; do not stage unrelated changes. Only humans
  run `git commit` in this repository.
- Report the commands run, their results, and any unrun live gate so a reviewer can reproduce the
  evidence.
