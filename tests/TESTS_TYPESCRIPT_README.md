# TypeScript app quickstart

## What this repo is

This is a Solid TypeScript client within a server platform. You write client
code in `src/`; `./build.sh` produces the derived browser bundle in `dist/`.
`dist/` is not a GitHub Pages artifact and is not source. The root scripts are
the supported commands; `npm run build`, `npm run check`, and `npm run clean`
mirror their corresponding repository commands as optional conveniences.

## Front door shell scripts

| Script                        | What it does                                                                          |
| ----------------------------- | ------------------------------------------------------------------------------------- |
| `./devel/setup_typescript.sh` | Install the checked-in JavaScript dependencies.                                       |
| `./check_codebase.sh`         | Fast Node gate: typecheck, ESLint, Prettier check, and Node tests.                    |
| `./build.sh`                  | Build Rust, Wasm, generated TypeScript inputs, fixtures, and the Solid client bundle. |
| `./devel/clean_build.sh`      | Remove rebuildable client output.                                                     |
| `./devel/dist_clean.sh`       | Remove the derived `dist/` output.                                                    |
| `./run_live_demo.sh`          | Start or stop the typed local live-demo controller.                                   |

Run `./check_codebase.sh --help` or `./build.sh --help` for their exact options.
`./run_live_demo.sh --help` documents the local-stack lifecycle. There is no
supported static-preview or GitHub Pages command.

The retained `./run_playwright_tests.sh` wrapper is not a developer quickstart
or current acceptance entry point. Its configuration requires private input
from the unavailable production-browser owner. Do not invoke it as evidence
that a local build, browser workflow, or production journey is accepted.

## Repo layout you edit

- `src/main.tsx` is the browser entry point.
- `src/index.html` is the page shell.
- `src/style.css` holds shared styles.
- `dist/` is the generated bundle; treat it as build output, not source.
- `tests/` holds every test tier described below.

## Test tiers and homes

The repo has four test tiers. Pick the home by what you are testing.

- Fast pytest hygiene under `tests/` covers markdown links, ASCII compliance,
  and file naming. These are cross-ecosystem checks, not the TypeScript
  toolchain. Run them with `source source_me.sh && python3 -m pytest tests/`. One guard, the test naming check,
  enforces test file naming under `tests/e2e/` and `tests/playwright/`.
- Node unit tests live in `tests/test_*.mjs`. Add one by dropping a
  `test_<name>.mjs` into `tests/`; `./check_codebase.sh` picks it up
  automatically through `node --import tsx --test 'tests/test_*.mjs'`.
- `tests/playwright/` holds browser-oriented evidence. The current focused
  scripts are not the missing real-stack production-browser acceptance owner;
  see `docs/TEST_EVIDENCE_MODEL.md` for the boundary.
- Whole-system E2E lives under `tests/e2e/` and runs directly, excluded from
  pytest. See `E2E_TESTS.md` for the non-browser E2E conventions.

## Daily run order

A typical edit loop runs the tiers in this order:

- Edit files under `src/`.
- Run `./check_codebase.sh` for the fast gate.
- Run `./build.sh` when generated Rust/Wasm inputs or the browser bundle must
  be current.
- Use the named local-stack command for a service boundary only when that
  boundary is in scope. The current browser acceptance owner is unavailable.

## Common first run failures

- `npx tsc -p tsconfig.lint.json` exits with TS18003 when `tests/` and `tools/`
  contain no `.ts` files. Seed a small `.ts` stub or narrow the include list in
  the consumer-owned `tsconfig.lint.json`.
- A fresh `npm install` must run esbuild's postinstall step. The `allowScripts`
  block in `package.json` already permits it, so let the install complete.

## Where to add tests

Keep the TypeScript toolchain checks (typecheck, lint, format, Node tests)
inside `./check_codebase.sh`, and keep the pytest tier under `tests/` thin and
cross-ecosystem. That split keeps each ecosystem verified by its own tools.

## Where to read more

For build-system, dependency, and style conventions in depth, see
[../docs/TYPESCRIPT_STYLE.md](../docs/TYPESCRIPT_STYLE.md).
