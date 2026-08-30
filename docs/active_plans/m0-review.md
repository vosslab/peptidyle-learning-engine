> **Historical review input, not current instructions.** Current authority is
> [implementation_plan.md](implementation_plan.md),
> [release_completion_plan.md](active/release_completion_plan.md), and
> [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md). [m0-results.md](m0-results.md) is concluded evidence.

# Code review: M0 foundation against implementation_plan.md

## Context

The goal is to implement the Peptidyle Learning Engine against
`docs/active_plans/implementation_plan.md` as source of truth, in milestone order, preserving
server-only grading, actor-scoped access, immutable published content, and the
TypeScript/Solid/Rust/WASM/PostgreSQL/object-storage/container architecture.

Work so far is the M0 foundation: a 13-crate Cargo workspace (128 lines of Rust, all stubs), a
4-file Solid app shell, a modified `build_github_pages.sh`, and the plan documents. This review
checks that work against the M0 work packages WP-F1 through WP-F6 and their exit criteria.

**Verdict: M0 is not met and the workspace does not compile.** Four crates carry hard syntax
errors, one crate calls an undeclared dependency, the Solid entry point calls a Solid API that
does not exist, and the build pipeline the plan mandates is an empty file. M1 must not start.
No changelog entry exists for any of this work, so the AGENTS.md validate-then-record loop has
not run.

Nothing here is deep architectural damage. The crate boundary table is mostly encoded correctly
and the security-critical property (grading outside the WASM dependency closure) is intact. The
gap is that the foundation was staged and reported without ever passing a gate.

## M0 work package status

| WP    | Acceptance                                                                                                                                      | Status      | Evidence                                                                                      |
| ----- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------- |
| WP-F1 | Every crate exists and compiles empty; current edition; `Cargo.lock` committed                                                                  | FAIL        | 4 crates have syntax errors; `edition = 2021` not 2024; `Cargo.lock` committed (328 packages) |
| WP-F2 | `wasm-bindgen` output to gitignored staging dir; trivial export callable from Node; toolchain pinned                                            | FAIL        | No staging dir, no Node call path, no `rust-toolchain.toml`                                   |
| WP-F3 | Build delegates to `node pipeline/build.mjs` with `esbuild-plugin-solid`; tsconfig jsx fields; `src/log.ts`; placeholders filled; `clean` fixed | FAIL        | Every sub-criterion unmet; `pipeline/build.mjs` is 0 bytes                                    |
| WP-F4 | `containers/Containerfile.api`, `containers/compose.yaml`, `/health` behind real `SELECT 1` + bucket probe                                      | NOT STARTED | No `containers/` directory                                                                    |
| WP-F5 | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` in `check_codebase.sh`                                             | NOT STARTED | `check_codebase.sh` still has 5 TypeScript-only steps                                         |
| WP-F6 | README first paragraph passing its test; `docs/CODE_ARCHITECTURE.md`; `pytest tests/` green                                                     | FAIL        | `README.md` is 0 bytes; no `docs/CODE_ARCHITECTURE.md`                                        |

## Blockers: the workspace does not compile

Each of these is a hard `rustc` error, not a lint.

| File:line                                    | Defect                                                                             | Fix                                                                                     |
| -------------------------------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `crates/objects/src/lib.rs:4`                | `pub mod memory;    / MemoryObjectStore ...` - a single `/` is not a comment token | `//`                                                                                    |
| `crates/learning-data-access/src/lib.rs:4-5` | `pub mod postgres;   # pg_backend ...` - Python comment syntax in Rust, two lines  | `//`                                                                                    |
| `crates/export/src/lib.rs:3-4`               | `pub mod docx;   Microsoft Word format output` - bare prose, no comment marker     | `//`                                                                                    |
| `crates/server/src/main.rs:7`                | Literal `\n` escape pasted into source outside any string                          | Real newline                                                                            |
| `crates/server/src/main.rs:6`                | `-> anyhow::Result<()>` but `anyhow` is absent from `crates/server/Cargo.toml`     | Declare `anyhow` in the workspace and the crate, or return `Result<(), std::io::Error>` |
| `crates/server/src/main.rs:7`                | `addr` bound and never used - fails `clippy -D warnings` even after the syntax fix | Use it in the axum bind, or drop it until MOD-API lands                                 |

The same four files also carry 24 trailing-whitespace lines across 11 `.rs` files, so
`cargo fmt --check` fails independently of the syntax errors.

## Blockers: the frontend cannot build or run

- `src/main.tsx:4` calls `createRoot(document.getElementById("root")!, <App />)`. `solid-js/web`
  exports `render`, not `createRoot`, and the argument order is React-shaped. The correct call is
  `render(() => <App />, document.getElementById("root")!)`.
- `pipeline/build.mjs` is 0 bytes and `pipeline/build.mjs.tmp` (also 0 bytes) is staged beside it.
  The plan (WP-F3) and `docs/TYPESCRIPT_STYLE.md` both require the JS-API path here because the
  esbuild CLI cannot load `esbuild-plugin-solid`.
- `esbuild-plugin-solid` is not in `package.json` at all.
- `build_github_pages.sh:32-37` still uses the CLI with `--jsx=automatic`, which emits
  `react/jsx-runtime` imports for a Solid app, and omits `--format=esm`, `--minify`, and
  `--sourcemap` from the canonical command. It also never copies a `.wasm` asset.
- `src/App.jsx` is a `.jsx` file. The ESLint and Prettier globs in `check_codebase.sh:194,197`
  cover `{ts,tsx,mts,cts,js,mjs,cjs}` - `.jsx` is in neither, so the app's only component file sits
  outside every gate. Rename to `src/app.tsx` (snake_case per `docs/TYPESCRIPT_STYLE.md`).
- `tsconfig.json:24` is `include: ["**/*.ts"]`, which excludes `.tsx` entirely, so `npx tsc
--noEmit` never type-checks the entry point. WP-F3 also requires `"jsx": "preserve"`,
  `"jsxImportSource": "solid-js"`, and an `exclude` for `OTHER_REPOS` and `target`.
- `src/index.html:8` loads `/main.js` with a leading slash, which 404s on a GitHub Pages project
  subpath. Use `main.js`.
- `package.json:2-3` still holds `__REPO_NAME__` and `__REPO_VERSION__`; `VERSION` is `26.08`.
- `package.json:12` points `clean` at `./dist_clean.sh`; the file is `devel/dist_clean.sh`.
- `solid-js` and `@solidjs/router` use `^` pins in `devDependencies` against the `>=` policy in
  `docs/TYPESCRIPT_STYLE.md`, and both are runtime deps rather than dev deps.
- No `src/log.ts`, which WP-F3 requires because `no-console: warn` meets `--max-warnings 0`.

## Contract and boundary findings

The plan's crate table (`implementation_plan.md:359-369`) is the contract. Deviations found:

- **`crates/domain` gains a clock.** `crates/domain/Cargo.toml:9` pulls `chrono` with default
  features, which includes `clock` and therefore `Utc::now()`. The plan's load-bearing property is
  that domain "has no clock and no database ... time and storage arrive as parameters"
  (`implementation_plan.md:371-373`). Use `chrono = { version = "0.4", default-features = false,
features = ["serde"] }` so the absence is structural, exactly as WP-F1 asks ("real absences, not
  comments").
- **The database driver sits in the wrong crate.** `sqlx` is declared in
  `crates/server/Cargo.toml:18` and nowhere in `crates/learning-data-access`, but the plan gives `crates/learning-data-access`
  ownership of "PostgreSQL backends, migrations, RLS context management". Move it before MOD-SCHEMA
  starts, or MOD-STO will be written against a dependency it does not own.
- **Grading isolation is correct - keep it that way.** `crates/wasm` depends only on
  `question_model`, `domain`, `serde_json`, `wasm-bindgen`. That is the compile-time guarantee the
  security model rests on. WP-C6 later needs a test asserting it; nothing here weakens it today.
- **Edition.** `Cargo.toml:20` sets `edition = "2021"` while `rust-version = "1.85"` makes edition
  2024 available. WP-F1 says "current edition". Members also do not inherit `rust-version`
  (`rust-version.workspace = true` is missing everywhere).
- **Workspace dependency table is half-used.** Root declares `sqlx` and `xml-rs`, but
  `crates/server/Cargo.toml:18` and `crates/adapters/qti/Cargo.toml:12` redeclare them inline with
  duplicate version strings. The internal-crate aliases `export_crate`, `wasm_bridge`,
  `server_core`, and `adapter_*` are declared at root and never consumed by any member.
- **Missing allowed deps.** `crates/learning-data-access` and `crates/export` are both permitted `objects` and
  neither declares it. Harmless while stubbed; add when the traits land so the boundary is
  explicit.
- **Public items are undocumented.** `crates/adapters/native/src/generator.rs:4` and
  `crates/wasm/src/lib.rs:7` expose `pub fn` with no `///` doc comment, against
  `docs/RUST_STYLE.md` section 13, which WP-C1 will enforce by review anyway.

## Repository and git-state findings

- **16 empty junk directories at the repo root**: `dy/ e/ ea/ g-e/ gi/ i/ in/ LE/ le-l/ MS/ n/ ne/
pt/ rc/ rn/ ROB/`, plus `ne/s` and `MS/p`. The fragments spell pieces of the repo path, so a
  mangled `mkdir` produced them. Git cannot see empty directories, so they will never show in
  `git status` and will silently persist.
- **Temp files in the index**: `crates/adapters/webwork/CARGOFILEtmp.txt`,
  `crates/adapters/webwork/Cargo.tmp`, `pipeline/build.mjs.tmp`. All three are staged; the first
  two no longer exist on disk.
- **Ten staged-but-deleted paths** (index and worktree disagree): the `config.rs`, `generate.rs`,
  `load.rs`, `importer.rs`, and `renderer_client.rs` stubs under `crates/adapters/*` plus the two
  temp files. A commit right now would resurrect files that were deliberately removed.
- **`crates/server/src/auth.rs` is untracked and empty** while `crates/server/src/lib.rs:4`
  declares `pub mod auth;`. A fresh clone of the current index fails to build even after the syntax
  fixes. `crates/learning-data-access/src/rls.rs` is untracked, empty, and declared by nobody.
- **`README.md` is 0 bytes.** `tests/test_readme_first_paragraph.py` exists and will fail, so
  `pytest tests/` cannot be green (WP-F6 exit criterion).
- **Six `.toml` files carry trailing whitespace** (`Cargo.toml`, `crates/grading`, `crates/objects`,
  `crates/server`, `crates/adapters/webwork`, `crates/adapters/qti`). `.toml` is in the
  `tests/test_whitespace.py` extension list, so that gate fails too.
- **`docs/CHANGELOG.md` has no entry** for the Cargo workspace, the Solid shell, or the build
  script change. Its only entry is the earlier `OTHER_REPOS/README.md` work.
- **Active-plans filing**: all six planning docs sit at the root of `docs/active_plans/` rather
  than the closed-set subdirectories in `docs/REPO_STYLE.md`. `reviewer_commments*.md` (three
  files) is misspelled and `customer-spec.md` uses a hyphen where the convention is snake_case.
  The plan's own closure requirement is a copy at
  `docs/active_plans/active/peptidyle_platform_build.md` with per-milestone status.
- **Root symlink `implementation_plan.md`** is untracked and duplicates the docs path. Reference
  the real path instead.
- **License choice is unrecorded.** `LICENSE` symlinks to `LICENSE.AGPL-3.0.md` and
  `LICENSE.CC-BY-4.0.md` sits beside it, while `docs/REPO_STYLE.md` documents GPLv3 / LGPLv3 /
  CC-BY-SA-4.0 as the defaults. AGPL is a defensible choice for a hosted platform; it needs a
  changelog decision entry rather than a silent deviation.

## Remediation order

Follow the plan's own patch sequence. Do not start M1 (WP-C1) until the M0 exit gate is green.

1. **Patch 1 rework - WP-F1 + WP-F2.** Fix the five compile errors; run `cargo fmt`; set edition
   2024 and inherit `rust-version`; make `chrono` `default-features = false`; move `sqlx` to
   `crates/learning-data-access`; collapse `xml-rs`/`sqlx` onto `workspace = true`; delete the unused internal
   aliases from the root workspace table; commit `crates/server/src/auth.rs` or drop the `pub mod
auth;` line; delete `crates/learning-data-access/src/rls.rs` until MOD-SCHEMA needs it. Add
   `rust-toolchain.toml`, the `wasm-bindgen` staging dir (gitignored), and one trivial export
   callable from Node.
2. **Git hygiene, same patch.** Unstage the three temp files, reconcile the ten staged-deleted
   paths, and remove the 16 empty root directories. `git status --porcelain` should show only
   intended work.
3. **Patch 2 rework - WP-F3.** Write `pipeline/build.mjs` around `esbuild.build()` with
   `esbuild-plugin-solid`; add that dependency; point `build_github_pages.sh` at `node
pipeline/build.mjs`; fix `src/main.tsx` to `render(() => <App />, el)`; rename `src/App.jsx` to
   `src/app.tsx`; add the tsconfig `jsx`, `jsxImportSource`, and `exclude` fields and widen
   `include` to `.tsx`; add `src/log.ts`; fix `src/index.html` to relative `main.js`; fill the
   `package.json` placeholders from `VERSION`; repoint `clean` at `devel/dist_clean.sh`; move
   `solid-js` and `@solidjs/router` to `dependencies` with `>=` pins.
4. **Patch 3 - WP-F4 + WP-F5.** `containers/Containerfile.api` and `containers/compose.yaml`
   (PostgreSQL 17, MinIO, named volumes, three buckets, run-time credentials); `/health` gated on a
   real `SELECT 1` plus a bucket probe; add the three Rust steps to `check_codebase.sh` through the
   existing `step_run` helper with a loud `SKIP` when `cargo` is absent; write `docs/CONTAINER.md`
   and `docs/MACOS_PODMAN.md`.
5. **Patch 4 - WP-F6.** Write `README.md` (first paragraph pure prose, under 250 characters);
   add `docs/CODE_ARCHITECTURE.md` with the container, boundary, bucket, and crate tables; strip
   the `.toml` trailing whitespace; file the active-plans docs into their subdirectories with `git
mv` and fix the misspellings; add the missing `docs/CHANGELOG.md` entries, including the AGPL
   decision.

## Verification

Run in this order; each must pass before the next patch starts.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./check_codebase.sh
pytest tests/
./build_github_pages.sh
```

Then confirm by inspection:

- `dist/main.js` exists, is ESM, and contains no `react/jsx-runtime` import.
- The `.wasm` asset is present in `dist/` and the trivial export is callable from Node.
- `git status --porcelain` shows no `.tmp` paths, no `AD` rows, and no untracked empty `.rs` files.
- `ls` at the repo root shows none of the 16 junk directories.

For WP-F4, after compose is up:

```bash
podman compose -f containers/compose.yaml up -d
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:3000/health
```

`/health` must return 200 only when both the `SELECT 1` and the bucket probe succeed, and must
fail when either backing service is stopped - test both stopped cases, since a `/health` that
returns 200 regardless is the exact defect the criterion is written against.
