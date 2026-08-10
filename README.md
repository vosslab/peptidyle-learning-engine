# Peptidyle Learning Engine

A backend-agnostic assignment platform for instructors who teach through repeated practice: students retry algorithmic questions until each one is correct, timers and grading stay on the server, and practice continues past completion.

**Status: advanced code-first implementation, not production-ready.** WP-RC1 course appearance and
WP-RC2 production-seam closure are accepted. WP-RC3's private, source-pinned upstream WeBWorK
`/render_rpc` implementation and static reviews are complete, but it is not accepted until the live
build, authenticated PLE API path, and browser-boundary gates pass. Native questions and the regular
local stack continue to work without the optional renderer profile. QTI Package Maker WP-FQ-0 owns
the QTI-JSONL specification and reference artifacts; PLE adopts that contract through one versioned
adapter/compiler with MATCH first. See the
[current project status](docs/active_plans/project_status_report_2026-08-09.md) for verified
evidence, milestone posture, blockers, and dependency order.

The accepted six-file database baseline is frozen. Every later schema change is a forward migration;
the maintained course-appearance schema is the first one. The maintained Compose stack is for local
development and currently uses the PostgreSQL bootstrap credential; it is not a production
deployment configuration.

## Practice past completion

The central teaching promise is a mastery loop that does not disappear when an assignment is marked
complete. An instructor can keep completion, scoring, variation, continued practice, and feedback
disclosure as separate course policies, while students see one focused question at a time and can
continue practicing with fresh values.

<!-- screenshots:begin (managed by screenshot-docs) -->

![Peptide bond mastery assignment overview with fresh variation and a Start or resume practice control](docs/screenshots/peptide_bond_mastery_overview.png)
<!-- screenshots:end -->

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
[docs/CODE_ARCHITECTURE.md](docs/CODE_ARCHITECTURE.md) and
[docs/FILE_STRUCTURE.md](docs/FILE_STRUCTURE.md)
for the ownership map.

## Quick start

For the smallest verified first success, install current Rust through `rustup`. The repository's
[rust-toolchain.toml](rust-toolchain.toml) selects stable Rust, rustfmt, Clippy, and the
`wasm32-unknown-unknown` target. Clone the repository and run its Rust behavior suite:

```bash
git clone https://github.com/vosslab/peptidyle-learning-engine.git
cd peptidyle-learning-engine
cargo test --workspace
```

Success is an exit status of zero after the domain, grading, storage, adapter, server, and
documentation tests; counts intentionally are not frozen in this page. This path proves the
mastery and server-side contracts without requiring containers or local credentials.

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
`dist_wasm/`. The all-in-one local test command generates ignored local credentials, migrates and
seeds its database, starts the supported Podman services, waits for semantic health, and opens the
browser:

```bash
./launch_local_stack.sh
```

Paste an instructor or student value from `containers/local-login.txt` into the local sign-in form.
Native questions work through this default path. To build and start the optional private WeBWorK
profile as part of local integration work, use:

```bash
./launch_local_stack.sh --with-webwork
```

That profile builds the declared upstream sources and runs only the bounded, authored
`content/pilot/webwork/which_hydrophobic-simple.pgml` RadioButtons fixture; it is not a claim of
broad OPL compatibility or completed live acceptance. Read the storage, security, and health model
in [docs/CONTAINER.md](docs/CONTAINER.md).

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

| Area                                 | State                                                                                                                                                                                                                                      |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Rust domain and learning data access | Attempt, timing, scoring, manual-grading, item-analysis, retention, catalog, and worker contracts; in-memory and PostgreSQL implementations share conformance tests                                                                        |
| API server                           | Auth, catalog, course, assignment, run, submission, manual grade, item analysis, asset, export, workspace, and retention route groups                                                                                                      |
| WebAssembly bridge                   | Browser-safe generation, response-format validation, timer, and state behavior; grading remains outside its dependency closure                                                                                                             |
| Browser client                       | Solid routes for courses, assignments, attempt loop, summary, library, authoring, flat-question editing, assignment editing, and gradebook                                                                                                 |
| PostgreSQL                           | Six domain-owned SQLx baseline migrations, forced RLS, least-privilege roles, retention fences, and disposable PostgreSQL acceptance                                                                                                       |
| Question engines                     | Native and static single-choice flat JSON implemented; WeBWorK `/render_rpc` is implemented and statically reviewed, with live acceptance pending; QTI profiles through atomic conversion; contracted iMathAS broker; H5P is ungraded only |
| DOCX and PDF export                  | Deterministic student and answer-key artifact generation through the object-store boundary                                                                                                                                                 |
| Containers                           | Local PostgreSQL, MinIO, API, worker, and gateway; private source-pinned WeBWorK is an optional profile pending live acceptance; production runtime identities and deployment remain open                                                  |
| Worker runtime                       | Production drains six complete families through a family-filtered registry; reserved Render and generic Import work stays unclaimed until its complete implementation lands                                                                |

The exact checkpoint, evidence, and remaining dependency order live in
[docs/active_plans/project_status_report_2026-08-09.md](docs/active_plans/project_status_report_2026-08-09.md)
and
[docs/active_plans/partial_commit_status.md](docs/active_plans/partial_commit_status.md). The full
architecture and milestone plan remain in
[docs/active_plans/implementation_plan.md](docs/active_plans/implementation_plan.md).

## Current limitations

- Flat-question JSON v1 supports static single choice. Multiple answer, fill-in-the-blank,
  multi-blank, numerical entry, matching, ordered list, and image hotspot remain required work.
- QTI profile import is intentionally bounded to the reviewed Canvas and Blackboard subsets;
  broader vendor compatibility, imported media, and optional exporters remain deferred.
- The pending WeBWorK RC3 acceptance path supports only the licensed, user-authored single-radio
  PGML fixture in `content/pilot/webwork/`; matching and broader problem compatibility are assigned
  to WP-RC5 rather than inferred from the private renderer implementation.
- Course appearance intentionally supports one theme and at most one 1200 by 328 entry banner per
  course. Per-page themes, multiple banners, freeform CSS, SVG/animated uploads, and learner edits are
  out of scope because the accepted version already supplies safe, accessible course identity without
  active-content or styling injection.
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

Start with a local run and the system map:

- [docs/INSTALL.md](docs/INSTALL.md) - required tools, setup, verification, and the optional
  private WeBWorK profile.
- [docs/USAGE.md](docs/USAGE.md) - native local-stack walkthrough, sign-in, health, and validation
  commands.
- [docs/CODE_ARCHITECTURE.md](docs/CODE_ARCHITECTURE.md) - system shape, crate ownership, storage,
  API, browser, and security boundaries.
- [docs/FILE_STRUCTURE.md](docs/FILE_STRUCTURE.md) - repository map and the owner of each major
  directory.

Then use these focused boundaries and references:

- [docs/CONTAINER.md](docs/CONTAINER.md) - local storage, bucket separation, health checks, and
  compose operations; required deployment selections live in `containers/env.example`.
- [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md) - server-only grading, tenant derivation,
  protected content, and authenticated route boundaries.
- [docs/QUESTION_MODEL.md](docs/QUESTION_MODEL.md) and
  [docs/ACTIVITY_MODEL.md](docs/ACTIVITY_MODEL.md) - published question and learner-activity
  contracts.
- [docs/CONTRACTS.md](docs/CONTRACTS.md) - frozen module ownership and atomic change rules.
- [docs/DATABASE_STRUCTURE.md](docs/DATABASE_STRUCTURE.md) - implemented table relationships,
  proposed production identity/passkey tables, FERPA isolation, and pilot-to-scale estimates.
- [docs/ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md](docs/ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md)
  - current student no-mouse task model, fixes, executable evidence, and remaining human evaluation.

For status and contribution work:

- [docs/active_plans/implementation_plan.md](docs/active_plans/implementation_plan.md) - milestone
  plan, module catalog, contracts, and acceptance gates; the source of truth for this build.
- [docs/active_plans/project_status_report_2026-08-09.md](docs/active_plans/project_status_report_2026-08-09.md)
  - formal executive status, verification evidence, milestone posture, blockers, and next work.
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
