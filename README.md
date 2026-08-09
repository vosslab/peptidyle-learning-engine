# Peptidyle Learning Engine

A backend-agnostic assignment platform for instructors who teach through repeated practice: students retry algorithmic questions until each one is correct, timers and grading stay on the server, and practice continues past completion.

**Status: advanced code-first implementation, not production-ready.** The last accepted QTI package
is WP-QTI-8: reviewed Canvas and Blackboard items can become canonical native flat source, private
grading, and provenance-aware Memory/PostgreSQL state atomically. A fresh six-migration real-role
baseline and focused frontend/backend security pass are green. WP-QTI-9 server routes are next;
course appearance, seven required flat-question families, remaining M5 integration, and M6
deployment are incomplete. See
`docs/active_plans/project_status_report_2026-08-09.md` for verified evidence, milestone posture,
blockers, and dependency order.

The database is still a pre-data baseline. Once an environment accepts durable data, later schema
changes must be forward migrations rather than edits to the six initial files. The maintained
Compose stack is for local development and currently uses the PostgreSQL bootstrap credential; it
is not a production deployment configuration.

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
- One backend-neutral question model behind every engine, so native algorithmic questions,
  WeBWorK, QTI, H5P, and a reviewed iMathAS provider use one adapter boundary even though their
  current runtime support differs.
- Per-question timing anchored to server timestamps, so the browser timer is display only and the
  server rules on whether an answer arrived before expiry.
- Capability validation before publication, so the platform can answer whether a question backend
  supports an assignment policy while the instructor is still editing.
- Mixed automatic and manual grading with generation-fenced score publication, followed by a
  separate course-local item analysis that never delays a learner-visible grade.
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
browser                         gateway       stateless server replicas
+---------------------------+               +----------------------------+
| Solid SPA (src/)          |               | axum API (crates/server)   |
|   domain.wasm:            | ------------> |   domain + grading         |
|   parameters, format      |               |   authoritative verdicts   |
|   validation, timer       |               +----------------------------+
|   no answers, no keys     |                   |          |           |
+---------------------------+                   v          v           v
                                          PostgreSQL   object store   private
                                          forced RLS   three buckets  renderer
                                               |
                                               v
                                          durable jobs
                                          scoring first,
                                          analysis later
```

Each crate names an exhaustive dependency list, so the boundary holds by construction rather than by
convention:

| Crate                         | Owns                                                                          | Depends only on                                  |
| ----------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------ |
| `crates/question_model`       | Question types, capabilities, identity, taxonomy                              | External crates                                  |
| `crates/domain`               | Attempt state machine, runs, timing, seeded generation, capability validation | `question_model`                                 |
| `crates/grading`              | Answer keys, checkers, correctness decisions (server only)                    | `question_model`, `domain`                       |
| `crates/objects`              | Object store trait, S3 and MinIO backends, keys, checksums                    | `question_model`                                 |
| `crates/learning-data-access` | Learning data access: contracts, PostgreSQL, migrations, and tenant isolation | `question_model`, `domain`, `objects`            |
| `crates/adapters/native`      | First-party generated questions and strict static PLE JSON                    | `question_model`, `domain`, `grading`            |
| `crates/adapters/webwork`     | Private renderer client, deterministic rendering, grading delegation          | `question_model`, `domain`, `grading`, `objects` |
| `crates/adapters/qti`         | Hardened package import and opt-in published runtime                          | `question_model`, `domain`, `grading`, `objects` |
| `crates/adapters/imathas`     | Contracted or self-hosted, server-brokered scored embed                       | `question_model`, `objects`                      |
| `crates/adapters/h5p`         | Package import into ungraded practice; scored execution is unavailable        | `question_model`                                 |
| `crates/export`               | Print model, DOCX and PDF writers                                             | `question_model`, `objects`                      |
| `crates/wasm`                 | The `wasm-bindgen` bridge, delegating every call to `domain`                  | `question_model`, `domain`                       |
| `crates/server`               | axum routes, auth, worker mode, composition root                              | Every crate above                                |

Two properties follow from that table. `crates/domain` reaches only `question_model`, so it has no
clock and no database, which is what lets the same code run on the server and in the browser. And
`crates/wasm` never reaches `crates/grading`, which is the answer-secrecy guarantee above.

The descriptive crate paths are `crates/learning-data-access` and `crates/project-tools`; their
Rust names are `learning_data_access` and `in_memory` where imported as code. Run repository-only
automation through `cargo tools`. See
[CODE_ARCHITECTURE.md](docs/CODE_ARCHITECTURE.md) and `docs/FILE_STRUCTURE.md`
for the ownership map.

## Quick start

For the shortest useful first success, install current Rust through `rustup`. The repository's
[rust-toolchain.toml](rust-toolchain.toml) selects stable Rust, rustfmt, Clippy, and the
`wasm32-unknown-unknown` target.

```bash
git clone https://github.com/vosslab/peptidyle-learning-engine.git
cd peptidyle-learning-engine
cargo test --workspace
```

Success is an exit status of zero after every Rust unit, integration, and documentation test. Test
counts intentionally are not frozen in this page.

The full repository gate also needs current Node.js and npm, plus Python 3.12 with pytest. Install
the browser dependencies once, then use the repository front door:

```bash
npm run setup
./check_codebase.sh
```

The final summary reports all 11 TypeScript, fixture, WebAssembly-boundary, Rust formatting, Clippy,
and test stages as `PASS`. Build the API, WebAssembly bridge, generated contracts, and Solid client
with:

```bash
./build.sh
```

A successful debug build ends with the client bundle in `dist/` and the WebAssembly bridge in
`dist_wasm/`. The local container stack is an advanced development path because it deliberately
requires local identities, credentials, and immutable gateway and renderer image choices. Read the
storage and health model in [docs/CONTAINER.md](docs/CONTAINER.md), then supply the current required
values from [containers/env.example](containers/env.example) rather than inventing defaults.

## One assignment through the system

The core path is implemented as a set of explicit ownership transitions:

```text
author draft
  -> publish one immutable problem version
  -> assign that exact version to a course
  -> issue a fresh server-seeded learner attempt
  -> persist an automatic result or a pending manual evaluation
  -> publish the newest scoring generation atomically
  -> rebuild the current course item analysis on a lower-priority job
```

The final analysis is course-owned and instructor-only. It reports aggregate difficulty,
discrimination, credit distribution, unanswered and pending-manual counts, and completion time. It
contains no learner identity, raw response, answer key, or grading implementation. A stale analysis
generation is discarded without delaying or rolling back the current grade.

## What exists today

| Area                                 | State                                                                                                                                                                           |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust domain and learning data access | Attempt, timing, scoring, manual-grading, item-analysis, retention, catalog, and worker contracts; in-memory and PostgreSQL implementations share conformance tests             |
| API server                           | Auth, catalog, course, assignment, run, submission, manual grade, item analysis, asset, export, workspace, and retention route groups                                           |
| WebAssembly bridge                   | Browser-safe generation, response-format validation, timer, and state behavior; grading remains outside its dependency closure                                                  |
| Browser client                       | Solid routes for courses, assignments, attempt loop, summary, library, authoring, flat-question editing, assignment editing, and gradebook                                      |
| PostgreSQL                           | Six domain-owned SQLx baseline migrations, forced RLS, least-privilege roles, retention fences, and disposable PostgreSQL acceptance                                            |
| Question engines                     | Native and static single-choice flat JSON implemented; WeBWorK private renderer client; QTI profiles through atomic conversion; contracted iMathAS broker; H5P is ungraded only |
| DOCX and PDF export                  | Deterministic student and answer-key artifact generation through the object-store boundary                                                                                      |
| Containers                           | Local-development PostgreSQL, MinIO, API replicas, worker, gateway, and private WeBWorK renderer; production runtime identities and deployment remain open                      |
| Worker runtime                       | Production drains six complete families through a family-filtered registry; reserved Render and generic Import work stays unclaimed until its complete implementation lands     |

The exact checkpoint, evidence, and remaining dependency order live in
`docs/active_plans/project_status_report_2026-08-09.md` and
[docs/active_plans/partial_commit_status.md](docs/active_plans/partial_commit_status.md). The full
architecture and milestone plan remain in
[docs/active_plans/implementation_plan.md](docs/active_plans/implementation_plan.md).

## Current limitations

- Flat-question JSON v1 supports static single choice. Multiple answer, fill-in-the-blank,
  multi-blank, numerical entry, matching, ordered list, and image hotspot remain required work.
- QTI profile parsing, conversion, provenance, and persistence are implemented, but the instructor
  upload/report/convert routes and UI are the next dependency-ordered packages.
- Course themes and the centered course-entry banner have an accepted plan but no implementation.
- File-upload responses deliberately fail closed until a server-issued, tenant/learner/attempt-bound
  upload capability exists.
- The local container topology is not a production security or deployment configuration.

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

Start with the product and operating boundaries:

- [docs/CODE_ARCHITECTURE.md](docs/CODE_ARCHITECTURE.md) - system shape, crate ownership, storage,
  API, browser, and security boundaries.
- [docs/CONTAINER.md](docs/CONTAINER.md) - local storage, bucket separation, health checks, and
  compose operations; required deployment selections live in `containers/env.example`.
- [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md) - server-only grading, tenant derivation,
  protected content, and authenticated route boundaries.
- [docs/QUESTION_MODEL.md](docs/QUESTION_MODEL.md) and
  [docs/ACTIVITY_MODEL.md](docs/ACTIVITY_MODEL.md) - published question and learner-activity
  contracts.
- [docs/CONTRACTS.md](docs/CONTRACTS.md) - frozen module ownership and atomic change rules.

For status and contribution work:

- [docs/active_plans/implementation_plan.md](docs/active_plans/implementation_plan.md) - milestone
  plan, module catalog, contracts, and acceptance gates; the source of truth for this build.
- `docs/active_plans/project_status_report_2026-08-09.md` - formal executive status, verification
  evidence, milestone posture, blockers, and next work.
- [docs/active_plans/partial_commit_status.md](docs/active_plans/partial_commit_status.md) - current
  implementation checkpoint, executable evidence, and remaining order.
- [docs/CHANGELOG.md](docs/CHANGELOG.md) - dated record of changes, decisions, and failures.
- [AGENTS.md](AGENTS.md) - working method, validation loop, and constraints for contributors and
  coding agents.
- [docs/RUST_STYLE.md](docs/RUST_STYLE.md), [docs/TYPESCRIPT_STYLE.md](docs/TYPESCRIPT_STYLE.md),
  and [docs/MARKDOWN_STYLE.md](docs/MARKDOWN_STYLE.md) - language and documentation conventions.

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
