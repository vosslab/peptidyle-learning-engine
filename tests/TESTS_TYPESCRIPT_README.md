# TypeScript app quickstart

## What this repo is

PLE is a server platform. You write the browser client in `src/`; `./build.sh`
creates production `dist/`, which the PLE gateway serves with its API. Browser
testing uses that same artifact through a disposable real stack. The npm aliases
mirror supported commands as an optional convenience.

## Front door shell scripts

| Script | What it does |
| --- | --- |
| `./check_codebase.sh` | Fast gate: typecheck, lint, format check, Node unit tests. |
| `./build.sh` | Build Rust, Wasm, generated contracts, fixtures, and `dist/`. |
| `source source_me.sh && python3 local_stack.py start --no-open` | Build and start ordinary local PLE. |
| `./run_playwright_tests.sh` | Run browser tests; builds `dist/` as needed. |
| `./devel/clean_build.sh` | Wipe build outputs. |

Run `./check_codebase.sh --help` for usage. `local_stack.py` owns the ordinary
local service lifecycle. `./run_playwright_tests.sh` owns a fresh disposable
HTTPS browser stack and accepts `--build` to force its production-artifact rebuild.

## Repo layout you edit

- `src/main.ts` is the entry point (use `src/main.tsx` for JSX or Solid).
- `src/index.html` is the page shell that loads `dist/main.js`.
- `src/style.css` holds the styles, copied into `dist/` at build time.
- `dist/` is the generated bundle; treat it as build output, not source.
- `tests/` holds every test tier described below.

## Test tiers and homes

The repo has four test tiers. Pick the home by what you are testing.

- Fast pytest hygiene under `tests/` covers markdown links, ASCII compliance,
  and file naming. These are cross-ecosystem checks, not the TypeScript
  toolchain. Run them with `pytest tests/`. One guard, the test naming check,
  enforces test file naming under `tests/e2e/` and `tests/playwright/`.
- Node unit tests live in `tests/test_*.mjs`. Add one by dropping a
  `test_<name>.mjs` into `tests/`; `./check_codebase.sh` picks it up
  automatically through `node --import tsx --test 'tests/test_*.mjs'`.
- Browser tests live under `tests/playwright/`. Run them with
  `./run_playwright_tests.sh`. See `docs/PLAYWRIGHT_USAGE.md` for the browser
  test conventions.
- Whole-system E2E lives under `tests/e2e/` and runs directly, excluded from
  pytest. See `E2E_TESTS.md` for the non-browser E2E conventions.

## Daily run order

A typical edit loop runs the tiers in this order:

- Edit files under `src/`.
- Run `./check_codebase.sh` for the fast gate.
- Run `source source_me.sh && python3 local_stack.py start --no-open` and use
  the ordinary PLE gateway in a browser.
- Run `./run_playwright_tests.sh` to confirm browser behavior.

## Run PLE locally

Use the local-stack controller for a normal, API-backed PLE installation. It
builds and starts the application, database, and storage services together.
Use `source source_me.sh && python3 local_stack.py acceptance` for the complete
opt-in browser acceptance suite.

## Common first run failures

- `npx tsc -p tsconfig.lint.json` exits with TS18003 when `tests/` and `tools/`
  contain no `.ts` files. Seed a small `.ts` stub or narrow the include list in
  the consumer-owned `tsconfig.lint.json`.
- Playwright needs `dist/` built before it serves the app. The runner
  auto-builds when `dist/` is missing, or pass `--build` to force a rebuild.
- A fresh `npm install` must run esbuild's postinstall step. The `allowScripts`
  block in `package.json` already permits it, so let the install complete.

## Where to add tests

Keep the TypeScript toolchain checks (typecheck, lint, format, Node tests)
inside `./check_codebase.sh`, and keep the pytest tier under `tests/` thin and
cross-ecosystem. That split keeps each ecosystem verified by its own tools.

## Where to read more

For build-system, dependency, and style conventions in depth, see
[../docs/TYPESCRIPT_STYLE.md](../docs/TYPESCRIPT_STYLE.md).
