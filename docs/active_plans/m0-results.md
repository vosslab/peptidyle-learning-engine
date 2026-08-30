# M0 results

What milestone M0 actually established, measured, and disproved. The plan in
[implementation_plan.md](implementation_plan.md) is a hypothesis; this file is
the evidence that came back from testing it.

Written because a plan can sound more certain than the evidence supports. Every
claim below is paired with the command that produced it, so a later reader can
re-run it rather than trust it. Where the plan turned out to be wrong, that is
recorded plainly rather than quietly fixed.

Date: 2026-08-06. Chronological detail is in [../CHANGELOG.md](../CHANGELOG.md).

## Exit criteria, measured

The plan's M0 exit criteria and what running them produced.

| Criterion                                             | Result                                        | Command                                 |
| ----------------------------------------------------- | --------------------------------------------- | --------------------------------------- |
| `./check_codebase.sh` green                           | 8 PASS, 1 SKIP                                | `./check_codebase.sh`                   |
| `cargo fmt --check`                                   | PASS                                          | in the gate above                       |
| `cargo clippy -- -D warnings`                         | PASS                                          | in the gate above                       |
| `cargo test`                                          | PASS, 3 unit + 3 doc tests                    | in the gate above                       |
| `pytest tests/` green                                 | 650 passed in 0.97s                           | `source source_me.sh && pytest tests/`  |
| `/health` 200 behind real `SELECT 1` and bucket probe | PASS, and 503 proven in the failure direction | `podman compose ... up -d`, then `curl` |

The SKIP is `test:node`: no `tests/test_*.mjs` files exist yet. It is reported
rather than hidden, because a gate that silently passes an absent suite is
worse than one that says nothing ran.

## What the plan got right

- **The crate boundary holds.** `crates/wasm` reaches only `question_model` and
  `domain`. `crates/grading` sits outside that closure, so an answer key cannot
  reach the browser by accident. This is structural, not a review rule.
- **The esbuild CLI is a dead end for Solid.** Confirmed by running it, not by
  citation.
- **Cursor pagination, RLS, and the three-bucket split** remain untested but
  also unchallenged; nothing in M0 touched them.

## What the plan got wrong

- **"`crates/domain` has no clock" was false as written.** The dependency table
  was satisfied while the property it existed to guarantee was not: `chrono`
  was declared with default features, which include `clock`, so `Utc::now()`
  was callable inside `domain`. Fixed with `default-features = false`.
  - Lesson worth carrying forward: a boundary table constrains _which crates_ a
    crate may name, not _which capabilities_ those crates bring. Feature flags
    leak capability through an allowed edge. When the plan states a property
    ("no clock", "no database"), the property needs its own check.
- **The CLI failure mode was misdescribed.** Both the plan's framing and my own
  first draft of the build comments said the esbuild CLI "emits React runtime
  imports and produces a bundle that fails at load." Measured behavior:

  ```text
  npx esbuild src/main.tsx --bundle --jsx=automatic --outfile=/tmp/probe.js
  X [ERROR] No matching export in "node_modules/solid-js/dist/solid.js" for import "jsx"
  (3 errors)
  ```

  The conclusion survives, the mechanism does not: it fails loudly at build
  time. Comments in `build.sh`, `pipeline/build.mjs`, and
  [../CODE_ARCHITECTURE.md](../CODE_ARCHITECTURE.md) were corrected.

- **M0 was previously reported complete without ever passing a gate.** The Rust
  workspace did not compile (five defects, including Python `#` comments in a
  `.rs` file and a literal `\n` escape in source), the Solid entry point called
  an API that does not exist, and `pipeline/build.mjs` was an empty file. The
  procedural lesson outweighs the technical one: run the gate before reporting
  the milestone.

## What only running it could have found

None of these appear in the plan. Each came from executing a step rather than
reasoning about it.

- **`uuid` will not compile for `wasm32-unknown-unknown`** without an explicit
  randomness source. The tempting fix is to enable a browser RNG feature. The
  correct reading is that the build error was telling the truth: the browser
  does not mint identifiers, because a `ProblemId` is constructed on the
  publish transition, server-side. Generation now sits behind a
  `question_model/generate` feature that the server enables and the WASM bridge
  does not. A compile error became a boundary.
- **Podman does not carry an image-level `HEALTHCHECK` onto the container.**
  `podman inspect ... {{.State.Health.Status}}` returned nil after a compose
  build. The health check is now declared in `containers/compose.yaml` as well.
  A health check that is silently not running is worse than none.
- **The generated Node WASM flavor is CommonJS**, and the repo's
  `"type": "module"` made Node parse it as ESM (`exports is not defined`).
  Fixed by scoping the module system with a `{"type":"commonjs"}` file in the
  output directory.
- **`wasm32-unknown-unknown` installs itself** when listed under `targets` in
  `rust-toolchain.toml`. No manual `rustup target add` step is needed.
- **`tsconfig.lint.json` matched no files**, so `tsc` exited 2 with TS18003 and
  typed linting of `playwright.config.ts` broke. The template documents this
  trap; M0 walked into it.

## Measurements

First numbers for this repository. They are baselines to compare against, not
targets.

Read build times as information. They vary with cache state, machine load, and
what changed, so they are most useful as a trend across runs on one machine.
`build.sh` reports durations; correctness lives in `./check_codebase.sh`.

Build, warm cache, debug profile (`./build.sh`):

| Stage  | Time  |
| ------ | ----- |
| rust   | 1.41s |
| wasm   | 1.72s |
| tsgen  | 0.47s |
| client | 0.49s |
| total  | 4.40s |

Artifacts:

| Artifact                                                | Size    |
| ------------------------------------------------------- | ------- |
| `dist_wasm/web/ple_bridge_bg.wasm` (one trivial export) | 19 KB   |
| `dist_wasm/web/ple_bridge.js` (generated glue)          | 5 KB    |
| `dist/main.js` (Solid shell, minified)                  | 11.5 KB |

Gates:

| Gate                        | Time     |
| --------------------------- | -------- |
| `pytest tests/` (650 tests) | 0.97s    |
| `cargo test --workspace`    | under 1s |

The WASM size matters most. The architecture rests on shipping `domain` to the
browser, and 19 KB for an empty bridge is the floor that real generation,
validation, and timing logic gets added to. Re-measure at WP-C5, when the
seed-vector work gives the bridge something to do.

## Decisions taken during M0

Each of these departs from the plan as written. Recorded here so a later
reviewer can disagree with the reasoning rather than guess at it.

- **Generate TypeScript ourselves.** WP-C1 called for `ts-rs` derives. Measured
  problem: ts-rs output is not `prettier --check` clean, and M1 requires
  generated TypeScript to pass Prettier _unchanged_, which a third-party
  generator gives no way to control. `crates/project-tools` now parses the model with
  `syn` and emits Prettier-shaped TypeScript directly. Side benefit:
  `question_model`, the product's root contract, carries no codegen dependency.
- **Parse QTI ourselves.** `xml-rs` was dropped before it was used. The
  rejection rules for hostile uploads are then ours to state and test rather
  than inherited from a general parser's defaults. This raises the stakes of
  the M4 hostile-corpus gate, which is the honest cost of the choice.
  Reference material for that work: `~/nsh/PROBLEMS/qti-package-maker/`.
- **Generate WASM glue from the workspace.** The first attempt installed a
  `wasm-bindgen` CLI through Homebrew and compared versions in the build
  script. Replaced by `crates/project-tools` depending on `wasm-bindgen-cli-support`,
  so `Cargo.lock` pins generator and crate together. A structural guarantee
  replaced a runtime check.
- **One master build script.** `build_github_pages.sh` was removed along with a
  stray `build_rust.sh`; `./build.sh` builds rust, wasm, tsgen, and client in
  dependency order and reports per-stage timing. This repository ships a server
  platform, so a GitHub Pages framing was template inertia: `dist/` is a client
  bundle the API serves, not a static site.
- **`npm run clean` points at `devel/clean_build.sh`**, not `dist_clean.sh` as
  WP-F3 specifies. `dist_clean.sh` is the deep reset that also removes
  `node_modules` and `target/`. `clean:dist` exposes it under its own name.
- **`package.json` version is `26.8.0`** while `VERSION` is `26.08`, because
  npm rejects `26.08` as invalid semver.

## Still untested

Claims the plan makes that M0 did not touch. Listed so they do not read as
settled.

- One PostgreSQL cluster serving 10 million problems and 1,000 instructors.
- "WeBWorK is the likely bottleneck."
- Server-side grading latency at p50, p95, p99 (M3).
- Native-versus-WASM seed parity (WP-C5). This is the next real experiment, and
  it is the gate that underwrites both the render cache and the reproducibility
  record.
- Partition pruning on a large synthetic attempt table (M5).
- The 256 KB operational payload threshold, which the plan already marks as
  needing profiling.
- Foreign-scope isolation returning zero rows under an unaffiliated actor (M2). The
  schema does not exist yet.

## Reproducing this

```bash
./check_codebase.sh
source source_me.sh && pytest tests/
./build.sh
bash tests/e2e/e2e_run_all.sh
podman compose -f containers/compose.yaml --env-file containers/env.local up -d
curl -s http://localhost:3000/health
```

For the health check, verify the failure direction too. A `/health` that only
ever returns 200 has proven nothing:

```bash
podman compose -f containers/compose.yaml stop postgres
curl -s http://localhost:3000/health   # {"status":"degraded","failing":["postgres"]}
```
