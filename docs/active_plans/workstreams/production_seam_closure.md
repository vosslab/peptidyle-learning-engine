# WP-RC2 production seam closure

> **Historical accepted package.** This record is retained as acceptance evidence, not current task
> direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

## Status

Accepted on 2026-08-09. WP-RC2 removes misleading production seam names and
implicit capability fallbacks without changing the established product boundary.
The next dependency is WP-RC3, shipped upstream WeBWorK integration. This
workstream preserves the mixed staged and unstaged worktree: it does not stage,
commit, reset, or discard any user-owned change.

The authoritative release package is
[release_completion_plan.md](../active/release_completion_plan.md#wp-rc2-remove-placeholder-production-seams).

## Decisions

| Topic              | Version 1 decision                                                                                      | Why this version succeeds                                                                               |
| ------------------ | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Production names   | A checked-in adapter module uses its actual responsibility in its filename.                             | Code search, imports, and the public facade describe the same real boundary.                            |
| Catalog capability | `resolve_catalog_problem` and `search_catalog` are required `CatalogStore` methods.                     | Every production store and focused test adapter declares its catalog behavior explicitly.               |
| Feedback release   | Current feedback reads persisted release state and passes it to one projection policy.                  | A durable instructor release transition, not a fabricated receipt flag, controls authorized disclosure. |
| Test doubles       | Recorded renderers, mocks, fixtures, and local clocks remain only under test/local owners.              | Deterministic boundary testing remains available without masquerading as production service behavior.   |
| Typed refusals     | `Unavailable` and `Unsupported` remain when they represent an outage or intentionally refused contract. | A bounded truthful failure preserves grading, provenance, and tenant safety.                            |

## Completed implementation

| Owner               | Files                                                                                                                  | Working behavior                                                                                                                                                           | Success condition                                                                                                   | Validation                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `rust-code-expert`  | `crates/adapters/h5p/src/import.rs`, `crates/adapters/h5p/src/lib.rs`                                                  | The existing key-free H5P practice importer is exported as `adapter_h5p::import`, retaining bounded archive handling and ungraded behavior.                                | No H5P production module is named `stub`.                                                                           | H5P tests and workspace compilation pass.                                            |
| `rust-code-expert`  | `crates/adapters/qti/src/parser.rs`, `crates/adapters/qti/src/parser/tests.rs`, `crates/adapters/qti/src/lib.rs`       | The strict bounded ZIP/XML parser pipeline is exported as `parser`; parser tests remain beside that owner.                                                                 | No QTI production parser uses a `stub` name.                                                                        | QTI tests and workspace compilation pass.                                            |
| `rust-code-expert`  | `crates/adapters/webwork/src/renderer_contract.rs`, `lib.rs`, `http_renderer.rs`, and server importers                 | The server-only renderer request, response, identity, failure, render, and grade contract has one explicit owner shared by HTTP and recorded renderers.                    | No production WeBWorK path imports `pg_parser_stub`.                                                                | WeBWorK/server tests and workspace compilation pass.                                 |
| `rust-code-expert`  | `crates/adapters/native/src/lib.rs`                                                                                    | The empty unreferenced `renderer_stub` module declaration is removed.                                                                                                      | Native exposes only implemented generation and grading boundaries.                                                  | Native tests and workspace compilation pass.                                         |
| `rust-code-expert`  | `crates/learning-data-access/src/lib.rs`, Memory/PostgreSQL catalog owners, and focused test stores                    | Catalog resolve/search have no default `Unavailable` body. Every Store implementation supplies its behavior; named test adapters supply intentional limited behavior.      | An omitted catalog capability cannot compile silently.                                                              | Data-access catalog conformance and server catalog/run tests pass.                   |
| `rust-code-expert`  | `crates/server/src/run.rs`, `crates/server/src/feedback.rs`, run-route tests                                           | Current run-summary feedback passes persisted release state into the sole `project_feedback` policy function. Initial submission receipts remain immutable and unreleased. | On-release feedback unlocks only after the authorized durable transition; deferred policy remains completion-gated. | Focused locked/unlocked, foreign/student refusal, and idempotent-release tests pass. |
| Documentation owner | `docs/CONTRACTS.md`, `docs/CODE_ARCHITECTURE.md`, `docs/FILE_STRUCTURE.md`, active plans, status report, and changelog | Repository maps name the implemented modules and required Store/feedback contracts.                                                                                        | No current documentation presents an accepted production boundary as a stub or future handoff.                      | Markdown links, ASCII/whitespace, Prettier, and diff checks pass.                    |

## Acceptance evidence

- The human-reviewed closure scan found no maintained production file that is empty or named
  `stub`, no production `todo!` or `unimplemented!`, no placeholder return data, and no default
  trait method hiding catalog lookup/search behavior. Remaining matches are classified as
  bounded typed refusals, parser vocabulary, or test/local doubles.
- Focused adapter, Store, and server suites passed with the renamed modules and explicit Store
  implementations.
- `cargo fmt --check`, strict workspace Clippy, and workspace tests passed.
- `./check_codebase.sh` passed all 11 stages.
- `source source_me.sh && python3 -m pytest -q tests/` passed 1,733 tests.
- `git diff --check` and `git diff --cached --check` passed.
- Independent review found no P0/P1 finding.

## Out-of-scope decisions

| Excluded work                                                          | Decision                                        | Why version 1 succeeds without it                                                                       |
| ---------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| New H5P, QTI, native, or WeBWorK feature behavior                      | RC2 renames and closes existing ownership only. | WP-RC3 through WP-RC6 own protocol, source-contract, family, and profile evolution.                     |
| Remote replacements for deterministic local/test doubles               | Keep named test/local doubles.                  | They test bounded behavior without becoming production composition.                                     |
| Reclassifying typed `Unavailable` or `Unsupported` failures as defects | Retain truthfully modeled failures.             | Fail-closed behavior is safer than fabricated success.                                                  |
| Permanent forbidden-word source-string tests                           | Use the reviewed closure scan for this package. | A brittle scan would reject legitimate parser vocabulary and test fixtures instead of testing behavior. |

## Residual release boundary

WP-RC2 is accepted, but the project is not released. WP-RC3 must replace the existing bounded
renderer boundary with a digest-pinned, private upstream `/render_rpc` deployment and its live
render/grade evidence. The remaining WP-RC4 through WP-RC12 packages retain their assigned owners,
artifacts, success conditions, and validation in the release-completion plan.
