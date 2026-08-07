# Changelog

## 2026-08-06

### Additions and New Features

- Brought the M0 foundation to a compiling, gated state across both toolchains:
  the 13-crate Cargo workspace, the Solid browser client, the WebAssembly build
  path, the container stack, and the extended check gate.
- Added `crates/server/src/health.rs`: readiness is a pure function over probe
  results, plus `probe_over_http`, which lets the API binary health-check
  itself so the runtime image needs no `curl` or `wget`.
- Added real dependency probes behind `/health`: `store::postgres::ping` issues
  `SELECT 1`, and `objects::minio::probe_bucket` issues `HeadBucket`. The
  endpoint returns 200 only when both answer and 503 naming the failing
  dependency otherwise.
- Added `containers/Containerfile.api` (two-stage build, `--locked`, non-root
  runtime user) and `containers/compose.yaml` (PostgreSQL 17, MinIO, named
  volumes, one-shot creation of the `content`, `student-records`, and
  `temp-processing` buckets), with `containers/env.example` as the credential
  template.
- Added `pipeline/build_wasm.sh`, which compiles `crates/wasm` for
  `wasm32-unknown-unknown` and emits both `wasm-bindgen` flavors into the
  gitignored `dist_wasm/` staging directory: `web` for the browser client and
  `node` for the WP-F2 gate.
- Added `pipeline/build.mjs`, the esbuild JavaScript-API build for the Solid
  client, with content-hash cachebusting on the script and stylesheet URLs.
- Added `rust-toolchain.toml` pinning the compiler to 1.96.0 with the
  `wasm32-unknown-unknown` target, and a repo-root `Brewfile` for `podman`.
- Added `crates/xtask`, the workspace-local `wasm-bindgen` glue generator. It
  is build tooling, outside the product boundary, and no shipped crate depends
  on it.
- Added `tests/e2e/e2e_run_all.sh` as the non-browser E2E runner. It carries
  the `e2e_` prefix because `tests/test_test_naming_conventions.py` requires it
  for shell files under `tests/e2e/`, which takes precedence over the
  `run_all.sh` name suggested in `docs/E2E_TESTS.md`.
- Added `src/log.ts` as the single logging surface for `src/`, `src/app.tsx` as
  the app shell, and base styling in `src/style.css`.
- Added `tests/e2e/e2e_wasm_bridge.mjs`, which loads the generated bridge in
  Node and asserts a value that crossed the Rust boundary.
- Added `docs/CODE_ARCHITECTURE.md`, `docs/CONTAINER.md`, and
  `docs/MACOS_PODMAN.md`.
- Added `docs/active_plans/m0-results.md`, the evidence record for M0: which
  plan assumptions held, which were disproved, what only running the steps
  revealed, the first build and artifact measurements, and the claims that
  remain untested.
- Added `build.sh`, the single master build. It runs rust, wasm, tsgen, and
  client in dependency order and reports per-stage timing, because the useful
  question is where the whole build spends its time and cargo only sees its own
  stage.
- Added `crates/xtask/src/tsgen.rs`, the repo's own TypeScript generator. It
  parses the question model with `syn` and emits Prettier-shaped TypeScript
  into `src/api/generated/`.
- Completed WP-C1, the question model and taxonomy (MOD-QM). `question_model`
  now carries identity (`WorkspaceId`, `ProblemId`, `VersionId`, `AssetId` as
  distinct newtypes), `Capability` and `BackendCapabilities`, answer shapes
  (tolerance, text matching, selection cardinality), `ResponseDefinition` and
  `StudentResponse`, `ContentBlock` and `QuestionEnvelope`, generation specs,
  the question and run policies, and `QuestionDefinition` itself. 34 types, 23
  unit tests, 2 doc tests.
- Completed WP-C2, identity and lifecycle (MOD-ID). `Lifecycle` has three
  states (draft, published, withdrawn), and every change passes through one
  fallible `apply` function. Publishing is the transition that assigns a
  `ProblemId`; republishing a published version is refused, because changing
  published content means publishing a new version. Withdrawal keeps existing
  assignments working and stops new ones.
- Added `docs/PROBLEM_IDENTITY.md`, the WP-C2 document: the four identifiers,
  the draft rule, the transition table, and why immutability pays for itself.
- Added `docs/QUESTION_MODEL.md`, the WP-C1 contract document: what belongs in
  the model versus in `grading`, the wire format, and the Rust-to-TypeScript
  mapping table.
- Generated TypeScript for all 34 boundary types into `src/api/generated/`,
  passing `tsc --noEmit`, ESLint, and `prettier --check` unchanged.
- Added `.cargo/config.toml` with a `cargo tsgen` alias. Committed, not
  ignored: it is project configuration, and cargo's caches live in `~/.cargo`.

### Behavior or Interface Changes

- `check_codebase.sh` now runs three Rust steps after the TypeScript steps:
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace`. A missing `cargo` emits a loud SKIP, matching
  the script's existing honesty convention.
- `build_github_pages.sh` now delegates to `node pipeline/build.mjs`, because
  the esbuild CLI cannot load `esbuild-plugin-solid`.
- The workspace moved to edition 2024 (`rust-version` 1.85 already allowed it)
  and every member now inherits `version`, `edition`, and `rust-version` from
  `[workspace.package]`.
- `crates/domain` declares `chrono` with `default-features = false`, dropping
  the `clock` feature. The plan's load-bearing property is that `domain` has no
  clock; with default features it had one.
- `sqlx` moved from `crates/server` to `crates/store`, and `aws-sdk-s3` is
  reached only through `crates/objects`. Both are feature-gated, and
  `crates/server` names neither: it uses the `store::postgres::Pool` and
  `objects::minio::S3Client` aliases instead.
- `package.json` placeholders are filled: name `peptidyle-learning-engine` and
  version `26.8.0`. `clean` now points at `devel/clean_build.sh` and a new
  `clean:dist` points at `devel/dist_clean.sh`.
- `solid-js` and `@solidjs/router` moved from `devDependencies` to
  `dependencies` with `>=` pins, matching the policy in
  `docs/TYPESCRIPT_STYLE.md`.
- `tsconfig.json` gained `"jsx": "preserve"`, `"jsxImportSource": "solid-js"`,
  an `exclude` list covering `OTHER_REPOS`, `target`, `dist`, and `dist_wasm`,
  and an `include` list that covers `.tsx`.
- `src/index.html` loads `main.js` and `style.css` by relative path. A leading
  slash resolves to the user-site root on a GitHub Pages project site and 404s.

### Fixes and Maintenance

- Fixed five defects that stopped the workspace from compiling at all: a single
  `/` used as a comment in `crates/objects/src/lib.rs`, Python `#` comments in
  `crates/store/src/lib.rs`, unmarked prose after `pub mod docx;` in
  `crates/export/src/lib.rs`, a literal `\n` escape pasted into
  `crates/server/src/main.rs`, and a call to `anyhow::Result` with `anyhow`
  undeclared.
- Fixed `src/main.tsx`, which called `createRoot(element, <App />)`.
  `solid-js/web` exports `render`, and it takes a component function; the old
  call would have rendered a static snapshot outside a reactive root.
- Renamed `src/App.jsx` to `src/app.tsx`. As a `.jsx` file it matched neither
  the ESLint nor the Prettier glob, so the only component in the app sat
  outside every gate.
- Fixed `tsconfig.lint.json`, whose `include` list matched no files and made
  `tsc` exit 2 with TS18003, which in turn broke typed linting of
  `playwright.config.ts`.
- Stripped trailing whitespace from the six `.toml` files that carried it, and
  reformatted every `.rs` file with rustfmt.
- Added `containers/env.local` and `.env` to `.gitignore`.

### Removals and Deprecations

- Removed three temporary files from the index that were never meant to be
  tracked: `crates/adapters/webwork/CARGOFILEtmp.txt`,
  `crates/adapters/webwork/Cargo.tmp`, and `pipeline/build.mjs.tmp`.
- Reconciled ten paths that were staged as added while deleted from the working
  tree, so a commit could not resurrect files that had been removed.
- Removed 16 empty directories accidentally created at the repository root
  (`dy/`, `e/`, `ea/`, `g-e/`, `gi/`, `i/`, `in/`, `LE/`, `le-l/`, `MS/`, `n/`,
  `ne/`, `pt/`, `rc/`, `rn/`, `ROB/`). Git cannot see empty directories, so
  they would never have appeared in `git status`.
- Removed the untracked root symlink `implementation_plan.md`, which duplicated
  `docs/active_plans/implementation_plan.md`.

### Decisions and Failures

- The prior M0 state was reported as complete but had never passed a gate: the
  Rust workspace did not compile, the Solid entry point called an API that does
  not exist, and `pipeline/build.mjs` was an empty file. Recorded here because
  the lesson is procedural rather than technical: run the gate before reporting
  the milestone.
- `npm run clean` points at `devel/clean_build.sh`, not `devel/dist_clean.sh`
  as WP-F3 specifies. `dist_clean.sh` is the deep reset that also removes
  `node_modules` and `target/`, and its own header documents `clean_build.sh`
  as the everyday cleaner wired to `npm run clean`. The plan's underlying
  defect (the script path did not exist) is fixed, and `clean:dist` exposes the
  deep reset under its own name.
- `package.json` carries version `26.8.0` rather than the `26.08` in `VERSION`.
  npm rejects `26.08` as invalid semver, so the CalVer value is expressed in
  the nearest valid form.
- Build timings reported by `build.sh` are information: they show where time
  went so a stage that grows becomes visible. Correctness lives in
  `./check_codebase.sh`, and the timings are most useful read as a trend across
  runs on one machine.
- `ts-rs` and `xml-rs` were both dropped in favor of our own implementations,
  at the owner's direction. Evidence that prompted the first: ts-rs output is
  not `prettier --check` clean, and M1 requires generated TypeScript to pass
  Prettier unchanged, which a third-party generator gives no way to control.
  The QTI parser is now ours to write, which raises the stakes of the M4
  hostile-corpus gate; that is the accepted cost.
- `uuid` refuses to compile for `wasm32-unknown-unknown` without an explicit
  randomness source. Rather than granting the browser an RNG, identifier
  generation moved behind a `question_model/generate` feature that the server
  enables and the WASM bridge does not. The browser never mints identifiers: a
  `ProblemId` is constructed on the publish transition. A compile error became
  a boundary.
- `build_github_pages.sh` was removed along with a stray `build_rust.sh`, and
  `./build.sh` replaces both. This repository ships a server platform, so the
  GitHub Pages framing was template inertia; `dist/` is a client bundle the API
  serves, not a static site. `deploy-pages.yml` was removed for the same
  reason.
- The `wasm-bindgen` glue is generated by a workspace crate (`crates/xtask`,
  depending on `wasm-bindgen-cli-support`) rather than an externally installed
  `wasm-bindgen` binary. The generator and the `wasm-bindgen` crate compiled
  into the module must be the same version; taking the generator from the
  workspace makes `Cargo.lock` pin both, so they cannot drift. The first
  attempt used a Homebrew CLI plus a version comparison in the build script,
  which was a check for a problem the structure can prevent outright.
- The generated Node flavor failed to load: it is CommonJS, and the repo-root
  `package.json` sets `"type": "module"`, so Node parsed the `exports.`
  assignments as ESM. `pipeline/build_wasm.sh` now writes
  `{"type":"commonjs"}` into `dist_wasm/node/`, which scopes the module system
  to that directory instead of renaming generated files after the fact.
- The container health check is declared in `containers/compose.yaml` as well
  as in the Containerfile: podman compose did not carry the image-level
  `HEALTHCHECK` onto the running container, leaving `State.Health` unset.
- Two Swift build scripts for an unrelated application, `build_debug.sh` and
  `build_release.sh`, are staged at the repository root. They reference
  `swift build` and an `AppCloser` binary and appear to have arrived by
  accident. Left in place pending the owner's decision.

### Developer Tests and Notes

- WP-C1 tests caught two defects that would have shipped a wrong wire format.
  First, serde's `rename_all` renames enum *variants* and leaves the fields
  inside them alone, so `graceSeconds` was serializing as `grace_seconds` while
  sibling structs used camelCase; every tagged enum now also declares
  `rename_all_fields`. Second, the generator's serde-attribute reader stopped
  at the first entry it did not recognize, so a `rename_all` written after
  `tag` was invisible to it and the TypeScript disagreed with the JSON.
- A third defect came from a brittle assertion rather than the code: a test
  searched the serialized JSON for `mass` and matched inside `molar_mass`. The
  assertion now searches for the quoted key.
- Identifier types were originally declared inside a `macro_rules!` body, which
  made them invisible both to the TypeScript generator, which reads source, and
  to a reader skimming for the contract. The four structs are now written out,
  with `impl_identifier!` supplying the shared behavior.

- `./check_codebase.sh`: 8 checks pass, 1 SKIP (`test:node`, no
  `tests/test_*.mjs` present yet). The Rust steps are `cargo:fmt`,
  `cargo:clippy`, and `cargo:test`.
- `cargo test --workspace`: 3 unit tests and 3 doc tests pass, 0 failures.
- `pytest tests/`: 645 passed in 1.09s.
- `podman compose -f containers/compose.yaml --env-file containers/env.local up -d`
  brings up `api`, `postgres`, `minio`, and the one-shot `createbuckets`.
  `mc ls local` lists `content/`, `student-records/`, and `temp-processing/`.
- The WP-F4 health gate was verified in both directions, which is the part that
  matters: `/health` returns 200 `{"status":"ready"}` with both services up;
  503 `{"failing":["postgres"],...}` with PostgreSQL stopped; 503
  `{"failing":["postgres","object-store"],...}` with both stopped; and 200
  again after both restart.
- The built bundle was checked for Solid rather than React output: zero
  occurrences of `react` or `jsx-runtime` in `dist/main.js`, with Solid's
  `insert` calls present.
- Tested the esbuild CLI claim rather than repeating it. `npx esbuild
  src/main.tsx --bundle --jsx=automatic` fails with three errors of the form
  `No matching export in "node_modules/solid-js/dist/solid.js" for import
  "jsx"`. The CLI path is unusable, as the plan says, but the failure mode is a
  hard build error, not the silent React-flavored bundle described in the first
  draft of these comments. The build script, `pipeline/build.mjs`, and
  `docs/CODE_ARCHITECTURE.md` were corrected to state the measured behavior.
- Learned during WP-F2 that `wasm32-unknown-unknown` was not installed, and
  that listing it under `targets` in `rust-toolchain.toml` makes rustup fetch
  it automatically on the next cargo invocation; no manual `rustup target add`
  was needed.
- `./pipeline/build_wasm.sh` produces `dist_wasm/web/` (19 KB `.wasm` plus 5 KB
  of glue) and `dist_wasm/node/`.
- `bash tests/e2e/e2e_run_all.sh`: 1 passed, 0 failed. The bridge returns
  `0.1.0` to Node, read from the Rust side's `CARGO_PKG_VERSION`.
- `./build_github_pages.sh` produces `dist/` with `index.html`, `main.js`
  (11.5 KB), `main.js.map`, `style.css`, `wasm/`, and `.nojekyll`, and the
  script URL carries the bundle content hash (`main.js?v=352b012a`).
