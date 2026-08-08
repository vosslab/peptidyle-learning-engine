# Peptidyle Learning Engine

A backend-agnostic assignment platform for instructors who teach through repeated practice: students retry algorithmic questions until each one is correct, timers and grading stay on the server, and practice continues past completion.

This repository is at milestone M0, the foundation stage. The Rust workspace compiles and passes its
gates, every module inside it is a documented stub, and the browser client is a placeholder shell.
There is no running server, no database schema, and no question backend wired up yet. What follows
describes the design being built, then states exactly what the code does today.

## Why this project

Most homework systems treat a submitted assignment as finished. Instructors using algorithmic
questions report the opposite behavior: students voluntarily rerun a completed assignment 30 or more
times, because every run generates fresh values around the same concept. A platform that ends at
completion cannot serve that, and a platform tied to one question format cannot serve a course that
already owns WeBWorK problems, QTI pools, and H5P activities.

The design answers both:

- Mastery runs as the default mode, where a student keeps working until every question is correct or
  an instructor-defined stopping condition is reached.
- Unlimited practice after completion, with completion, grading, variation, continued practice, and
  feedback disclosure as five independent policies an instructor combines freely.
- One backend-neutral question model behind every engine, so a first-party algorithmic generator,
  WeBWorK, QTI, and H5P all enter through the same adapter boundary.
- Per-question timing anchored to server timestamps, so the browser timer is display only and the
  server rules on whether an answer arrived before expiry.
- Capability validation before publication, so the platform can answer whether a question backend
  supports an assignment policy while the instructor is still editing.
- Exam export to DOCX and PDF, with separate student and answer-key artifacts.

## Two guarantees the structure enforces

**Grading is server-only, and the crate graph is what enforces it.** `crates/grading` holds every
answer key, checker, and correctness decision, and it sits outside the dependency closure of the
WebAssembly bridge that ships to the browser:

```toml
# crates/wasm/Cargo.toml
[dependencies]
question_model.workspace = true
domain.workspace = true
serde_json.workspace = true
wasm-bindgen.workspace = true
```

Adding `grading` to that list is the single change that would put an answer key in a browser, so it
is a compile-graph violation rather than a code-review judgment call. The bridge still does real
work: parameter generation, answer-format validation, timer display, and state transitions, none of
which carry information about correctness.

**Published content is shared and immutable; educational records are tenant-owned.** One published
problem version serves many instructors without being copied, while every course, enrollment, run,
attempt, and grade carries a tenant ID protected by database-enforced row-level security. That one
boundary is also the privacy boundary: deleting a course's student records destroys no reusable
content, because assignments reference shared problem versions instead of owning copies.

## Architecture at a glance

```text
browser                            server replicas
+---------------------------+      +----------------------------+
| Solid SPA (src/)          |      | axum API (crates/server)   |
|   domain.wasm:            | ---> |   domain + grading         |
|   parameters, format      |      |   authoritative verdicts   |
|   validation, timer       |      +----------------------------+
|   no answers, no keys     |          |           |          |
+---------------------------+          v           v          v
                                 PostgreSQL    job queue    object
                                 one cluster,      |        storage
                                 forced RLS        v      three buckets
                                              worker pool
```

Each crate names an exhaustive dependency list, so the boundary holds by construction rather than by
convention:

| Crate | Owns | Depends only on |
| --- | --- | --- |
| `crates/question_model` | Question types, capabilities, identity, taxonomy | External crates |
| `crates/domain` | Attempt state machine, runs, timing, seeded generation, capability validation | `question_model` |
| `crates/grading` | Answer keys, checkers, correctness decisions (server only) | `question_model`, `domain` |
| `crates/objects` | Object store trait, S3 and MinIO backends, keys, checksums | `question_model` |
| `crates/store` | Store trait, PostgreSQL backends, migrations, RLS context | `question_model`, `domain`, `objects` |
| `crates/adapters/native`, `webwork`, `qti`, `h5p` | Per-engine load, generate, grade delegation, capability declaration | `question_model`, `domain`, `grading`, `objects` |
| `crates/export` | Print model, DOCX and PDF writers | `question_model`, `objects` |
| `crates/wasm` | The `wasm-bindgen` bridge, delegating every call to `domain` | `question_model`, `domain` |
| `crates/server` | axum routes, auth, worker mode, composition root | Every crate above |

Two properties follow from that table. `crates/domain` reaches only `question_model`, so it has no
clock and no database, which is what lets the same code run on the server and in the browser. And
`crates/wasm` never reaches `crates/grading`, which is the answer-secrecy guarantee above.

## Quick start

The Rust workspace is the part that runs today. It needs `rustup`, which reads the pinned toolchain
from [rust-toolchain.toml](rust-toolchain.toml) and installs the `wasm32-unknown-unknown` target
automatically. The Python hygiene suite needs Python 3.12 and pytest.

```bash
git clone https://github.com/vosslab/peptidyle-learning-engine.git
cd peptidyle-learning-engine
cargo test --workspace
```

Every crate compiles and the workspace test run is green:

```text
     Running unittests src/lib.rs (target/debug/deps/server_core-...)
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   Doc-tests wasm_bridge
running 2 tests
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The three passing unit tests are the readiness rule the container health check is gated on: an empty
probe list reports degraded, never ready, because a process that has checked nothing has proven
nothing. The rest of the workspace is stubs, so its test counts are zero on purpose.

The gates a change is held to. These four pass today:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
source source_me.sh && pytest tests/
```

`./check_codebase.sh` runs the TypeScript side. It does not pass yet, and the Rust steps above still
have to be added to it; finishing that script is part of the open M0 work.

## What exists today

| Area | State |
| --- | --- |
| Cargo workspace, twelve crates | Compiles; `cargo fmt`, `clippy -D warnings`, and `cargo test` green |
| Rust modules | Documented stubs; the readiness rule in `crates/server/src/health.rs` is the only implemented logic |
| API server | Binds a port and serves `/health`, which reports degraded until real probes land |
| WebAssembly bridge | One trivial export, `bridge_version`, proving the toolchain path |
| Browser client | Solid shell with one placeholder component that states its own build status; no routes, no widgets |
| Database schema and row-level security | Not started |
| Adapters for native, WeBWorK, QTI, H5P | Crate skeletons and capability notes only |
| DOCX and PDF export | Crate skeleton only |
| Containers for api, postgres, minio | Not started |

Milestone order, entry criteria, and acceptance gates live in
[docs/active_plans/implementation_plan.md](docs/active_plans/implementation_plan.md). M0 is not
complete until containers come up and `/health` returns 200 behind a real `SELECT 1` and a bucket
probe.

## Repository layout

```text
crates/     Rust workspace: question model, domain, grading, storage, adapters, export, wasm, server
src/        TypeScript and SolidJS browser client
pipeline/   Browser bundle build script
tests/      Python hygiene suite, plus Playwright browser tests
docs/       Style guides, changelog, and the active implementation plan
devel/      Maintenance and release helper scripts
```

## Documentation

Setup and usage guides arrive with the first running service. Until then these are the useful routes:

- [docs/active_plans/implementation_plan.md](docs/active_plans/implementation_plan.md) - milestone
  plan, module catalog, contracts, and acceptance gates; the source of truth for this build.
- [docs/active_plans/customer-spec.md](docs/active_plans/customer-spec.md) - the product
  specification the plan implements.
- [docs/CHANGELOG.md](docs/CHANGELOG.md) - dated record of changes, decisions, and failures.
- [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md) - server-only grading, tenant derivation, and
  protected-content boundaries.
- [AGENTS.md](AGENTS.md) - working method, validation loop, and constraints for contributors and
  coding agents.
- [docs/RUST_STYLE.md](docs/RUST_STYLE.md) - Rust conventions the workspace is held to.
- [docs/TYPESCRIPT_STYLE.md](docs/TYPESCRIPT_STYLE.md) - TypeScript and Solid conventions for the
  browser client.
- [docs/PLAYFUL_TRAINING_GAME_STYLE.md](docs/PLAYFUL_TRAINING_GAME_STYLE.md) - student interface
  requirements, including the wrong-answer screen.
- [docs/COLOR_CONTRAST_ACCESSIBILITY.md](docs/COLOR_CONTRAST_ACCESSIBILITY.md) - contrast rules the
  interface is measured against.

## License

Code is licensed under the GNU Affero General Public License v3, in
[LICENSE.AGPL-3.0.md](LICENSE.AGPL-3.0.md), which the root `LICENSE` symlink points to. Because this
is a hosted platform, AGPL matters in practice: running a modified version as a network service
carries an obligation to offer that modified source to its users.

Non-code material such as documentation text and figures is licensed under Creative Commons
Attribution 4.0, in [LICENSE.CC-BY-4.0.md](LICENSE.CC-BY-4.0.md).

## Author

Neil R. Voss, Associate Professor of Biology at Roosevelt University, who also maintains the
[Biology Problems](https://biologyproblems.org) open educational resource project of computationally
generated problem sets. Background in [docs/AUTHORS.md](docs/AUTHORS.md); reachable on
[Bluesky](https://bsky.app/profile/neilvosslab.bsky.social).
