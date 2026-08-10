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
- Keep each source file below 1000 lines. Move a complete capability to a focused module before a
  parent file becomes an implementation warehouse.
- Follow the language and test rules in [TYPESCRIPT_STYLE.md](TYPESCRIPT_STYLE.md),
  [RUST_STYLE.md](RUST_STYLE.md), and [PYTEST_STYLE.md](PYTEST_STYLE.md). This guide does not
  duplicate their detailed conventions.

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

The check gate is deliberately not a product build. It refreshes the ignored generated TypeScript
projections, then runs TypeScript typechecks, ESLint, Prettier, Node tests, Rust formatting,
Clippy, and workspace tests. A missing Rust toolchain is reported as `SKIP`; it is not a successful
Rust verification.

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

## Run local services

Use the launcher when a work package needs the supported local PostgreSQL, MinIO, API, gateway, or
optional WeBWorK renderer:

```bash
./launch_local_stack.sh --check
./launch_local_stack.sh --no-open
./launch_local_stack.sh --with-webwork --no-open
```

`--check` validates configuration without changing state. A normal launcher run can create ignored
local credentials and starts the supported stack; do not copy those local secret files into source
control. [CONTAINER.md](CONTAINER.md) documents the stack, configuration, recovery, and service
commands.

## Prepare a handoff

- Review the diff and run the gates appropriate to the changed owner and active plan.
- Update frozen contracts and their consumers together; contract-only changes are incomplete.
- Preserve an existing mixed staged/unstaged worktree; do not stage unrelated changes. Only humans
  run `git commit` in this repository.
- Report the commands run, their results, and any unrun live gate so a reviewer can reproduce the
  evidence.
