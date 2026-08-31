# WP-ARCH1 source module decomposition evidence

## Status

WP-ARCH1 was accepted on 2026-08-10 after the required independent PostgreSQL, server-security,
provider-security, TypeScript/HCI, test, size-policy, and final architecture reviews found no
unresolved P0/P1 issue. The final RC3 review then accepted the bounded WeBWorK package.

## Before and after inventory

The dated baseline contained 26 maintained Rust, TypeScript, TSX, Python, and test sources at 1,000
physical lines or more. The same repository-root inventory command now reports zero maintained code
violations. The permanent tracked-file policy passes 582 cases, including its 999-pass and
1,000-fail boundary tests.

The exact override list contains no maintained code. It contains only three frozen applied SQL
migrations and three documentation or history artifacts whose owners use migration immutability,
document structure, or changelog rotation instead of capability-module limits.

## Capability moves

### WP-SIZE1: persistence

- `learning-data-access` contracts moved behind the stable `lib.rs` facade into `src/contracts/`.
- Memory and PostgreSQL implementations now use capability owners for activity, authoring, courses,
  assignments, policy, feedback, runs, statistics, row decoding, transactions, and submissions.
- Conformance and live provenance tests were divided by durable behavior while retaining the same
  test cases and public Store contract.

### WP-SIZE2: server

- Composition-local identity, iMathAS launch state, run prefetch/query/submission behavior, and the
  large server test groups moved to focused owners behind their existing module paths.
- Catalog, course, workspace, retention, flat-publication, and run tests were divided by route or
  behavior ownership without changing router, authorization, status, header, or JSON contracts.

### WP-SIZE3: adapters and tooling

- iMathAS cache, WeBWorK HTML projection, adapter tests, statistics tests, and PDF tests moved to
  focused child modules.
- The E2E seed tool now separates native, WebWork, timing, scoring, record, and test ownership.
- `devel/bump_version.py` remains the stable command facade over focused CLI, contract, discovery,
  parsing, rewrite, and formatting modules.

### WP-SIZE4: browser

- `src/components/question_response_controls/` owns the Question Response Control dispatcher, keyboard,
  external-tool extension, and multiple-choice, numeric, ordering, short-text, file-upload, and shared
  response-control implementations.
- HTTP transport, errors, authentication, and bounded JSON handling moved behind `http_client.ts`.
- Mock handler families now own shared, authentication, catalog, courses, runs, authoring, and asset
  behavior separately.
- `decoders.ts` is a stable barrel over catalog/course, question-model, question-delivery, run, and
  shared decoder owners, with no child-to-facade back-edge.

## Permanent tests and one-time evidence

The permanent suite retains behavior, security, tenancy, serialization, Store conformance, browser
interaction, and the source-size architecture boundary. No new test freezes the extracted module
names, symbol inventory, exact file count, or facade layout.

The implementation used one-time symbol inventories, untracked-aware line inventories, compiler
feedback, and `bump_version.py` help/dry-run probes. Temporary splitter scripts and generated Python
cache directories were removed after use. Per `docs/PYTEST_STYLE.md`, these checks remain evidence,
not permanent tests; when their durable behavioral value was uncertain, no test was added.

## Validation evidence

The final integrated results are:

- untracked-aware maintained-source inventory: zero violations;
- `tests/test_source_file_line_limit.py`: 582 passed;
- `./check_codebase.sh`: all 11 stages passed, including generated contracts and fixtures,
  TypeScript, ESLint, formatting, 184 Node tests, crate boundaries, Rust formatting, strict Clippy,
  and workspace tests;
- complete repository Python suite: 2,451 passed;
- complete Playwright suite: 72 passed and 2 deliberately opt-in tests skipped;
- focused server suite: 189 passed and 3 explicitly ignored live fixtures;
- focused learning-data-access library suite: 70 passed;
- disposable PostgreSQL baseline: passed migration replay and checksum refusal, serialization retry,
  concurrent claims, course ownership, course appearance, partition pruning over 260,000 attempts,
  bounded summaries, QTI import/conversion/provenance, private flat grading, item analysis, manual
  grading, constraint inventory, and four-role RLS denial;
- focused adapter, domain, export, project-tool, TypeScript, and browser gates passed during their
  owning work packages;
- `python3 -m py_compile`, version-tool help, advanced-help, and dry-run probes passed as disposable
  CLI evidence; and
- working-tree and index whitespace checks are clean.

The first live baseline run exposed a stale exact test filter after the provenance test moved under a
behavior module: Cargo reported zero executed tests while returning success. The runner now executes
the complete dedicated integration-test binary rather than its former module path. A fresh complete
baseline then ran the provenance test and passed; its unique Compose project, database, container,
and volume were removed. This correction is a maintained acceptance-runner fix, not a new permanent
layout test.

The provider protocol environments were not rebuilt solely to prove file movement; their behavior
remains covered by unchanged focused contracts and previously accepted live package evidence. The
independent provider-security review accepted that evidence and found no provider-backed fresh run
necessary for this structural package.

## Independent acceptance

The independent PostgreSQL, server-security, provider-security, TypeScript/HCI, test, size-policy,
and final architecture reviews examined the transaction moves, authorization and wire boundaries,
provider secrecy, browser exports, closed gate contract, exact override list, and capability map.
They found no unresolved P0/P1 issue. The final architecture review records that overlapping RC3
WebWork, UUID-decoder, and keyboard feature changes are separate accepted packages, not semantic
changes smuggled into WP-ARCH1.
