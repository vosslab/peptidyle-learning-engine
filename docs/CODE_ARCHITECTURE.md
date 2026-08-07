# Code architecture

How the Peptidyle Learning Engine is put together and why the boundaries sit
where they do. The authoritative plan is
[active_plans/implementation_plan.md](active_plans/implementation_plan.md);
this document is the working map.

Current state: M0 foundation. The Cargo workspace compiles, the Solid client
builds, and the container stack runs, but every domain module is a documented
stub. Contracts freeze in M1.

## Shape of the system

```text
browser                     ALB          stateless replicas
+------------------------+           +----------------------------+
| Solid SPA (src/)       |  +-----+  | api x N (crates/server)    |
|  student/ instructor/  |->| ALB |->|   axum, native Rust        |
|  +------------------+  |  +-----+  |  +----------------------+  |
|  | domain.wasm      |  |           |  | domain + grading     |  |
|  | params, format   |  |           |  | authoritative        |  |
|  | validate, timer  |  |           |  +----------------------+  |
|  | NO answers       |  |           +----------------------------+
|  +------------------+  |             |            |         |
+------------------------+             v            v         v
        ^                       PostgreSQL      jobs queue   S3
        | public assets         one cluster:        |      3 buckets
   CloudFront (immutable)       shared content      v
        |                       + tenant rows   worker x N
   /api/assets (authorized)     + forced RLS    export, import,
                                                renderer fill
                                                     |
                                                     v
                                              renderer x N
                                              WeBWorK PG, private
```

## Two guarantees the structure enforces

Both are enforced by the shape of the code, not by review discipline.

- **Grading is server-only.** `crates/grading` holds answer keys and
  correctness decisions. It is not in the dependency closure of `crates/wasm`,
  so shipping an answer key to the browser is a compile-time impossibility
  rather than something a reviewer has to notice. Adding `grading` to
  `crates/wasm/Cargo.toml` is the single change that would break this.
- **Records are tenant-owned, content is shared.** Published problems and their
  versions carry no tenant ID and are immutable. Courses, assignments,
  enrollments, runs, attempts, and grades carry a tenant ID on every row under
  `FORCE ROW LEVEL SECURITY`, with the tenant context set from the
  authenticated session and never from a request parameter.

## Crate boundaries

Each crate's dependency list is exhaustive: the allowed set is the whole set,
so a `Cargo.toml` matching this table satisfies the boundary by construction.

| Crate | Owns | Depends only on |
| --- | --- | --- |
| `crates/question_model` | Question types, capabilities, identity, taxonomy | external crates |
| `crates/domain` | Attempt state machine, run and completion rules, timing verdict, seeded generation, capability validation | `question_model` |
| `crates/grading` | Answer keys, checkers, correctness decisions | `question_model`, `domain` |
| `crates/objects` | `ObjectStore` trait, S3 and MinIO backends, key construction, checksums | `question_model` |
| `crates/store` | `Store` trait, PostgreSQL backend, migrations, RLS context | `question_model`, `domain`, `objects` |
| `crates/adapters/{native,webwork,qti,h5p}` | Per-engine load, generate, grade delegation, capability declaration | `question_model`, `domain`, `grading`, `objects` |
| `crates/export` | Print model, DOCX and PDF writers | `question_model`, `objects` |
| `crates/wasm` | `wasm-bindgen` bridge delegating to `domain` | `question_model`, `domain` |
| `crates/server` | axum routes, auth, worker mode, composition root | every crate above |

Two properties follow from that table. `crates/domain` reaches only
`question_model`, so it has no clock and no database: `chrono` is declared with
`default-features = false`, which drops the `clock` feature, and time arrives
as a parameter. That is what lets the same code run in a browser and makes the
seed-parity test meaningful. `crates/wasm` reaches only `question_model` and
`domain`, which is the grading guarantee above.

Two drivers are owned rather than shared. `sqlx` is declared only in
`crates/store` and `aws-sdk-s3` only in `crates/objects`; `crates/server`
enables them through features and names neither, so the database and object
clients stay replaceable behind their traits.

## Object storage

| Bucket | Holds | Serving | Retention |
| --- | --- | --- | --- |
| `content` | source packages, shared assets, cached renders | CDN and immutable URLs for public content, 60-minute authorized URLs for secure content | indefinite, versioned |
| `student-records` | exports, uploaded responses, annotated exams | 5-minute authorized URLs, always logged | explicit expiration and deletion |
| `temp-processing` | extraction and conversion workspaces | never served | lifecycle rule, days |

Keys are built only from identity types and versions, never from a
caller-supplied string. Checksums are computed on write and verified on read.

## Containers

| Service | Image | Purpose |
| --- | --- | --- |
| `api` | built from `containers/Containerfile.api` | axum API server |
| `postgres` | `postgres:17-alpine` | shared content and tenant-owned records |
| `minio` | `quay.io/minio/minio` | S3-compatible object storage |
| `createbuckets` | `quay.io/minio/mc` | one-shot bucket creation |

Details and commands are in [CONTAINER.md](CONTAINER.md); the macOS virtual
machine setup is in [MACOS_PODMAN.md](MACOS_PODMAN.md).

`/health` returns 200 only after a real `SELECT 1` and a real `HeadBucket`
request, and 503 naming the failing dependency otherwise. A health check that
reports on process liveness rather than dependency reachability tells an
orchestrator nothing.

## Browser client

`src/` is a TypeScript and SolidJS single-page application.
[pipeline/build.mjs](../pipeline/build.mjs) builds it with the esbuild
JavaScript API plus `esbuild-plugin-solid`, because Solid compiles JSX to
direct DOM operations through a Babel preset that the esbuild CLI cannot load.

That was measured rather than assumed. Running the CLI against this source with
`--jsx=automatic` fails with three errors of the form
`No matching export in "node_modules/solid-js/dist/solid.js" for import "jsx"`,
so the wrong path fails loudly at build time rather than shipping a bundle that
breaks in the browser.

The client shares generation, format validation, and timer logic with the
server through the WebAssembly bridge that
[pipeline/build_wasm.sh](../pipeline/build_wasm.sh) produces, so the two cannot
drift apart. Build output lands in `dist/`, with the bridge under `dist/wasm/`.

## Gates

`./check_codebase.sh` runs both toolchains: TypeScript typecheck, wider
typecheck, ESLint at zero warnings, Prettier, Node unit tests, then
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo test --workspace`. A missing `cargo` emits a loud SKIP rather than
passing silently.

`pytest tests/` runs the repository hygiene suite. Browser tests live in
`tests/playwright/`, and slower whole-system checks in `tests/e2e/`, both
outside the fast lane.
