# Contract register

This register turns the module catalog in the
[active implementation plan](active_plans/implementation_plan.md) into an
ownership and change-control boundary. It records all 36 catalog modules. It
does not mark later milestone behavior as implemented.

## How to read the register

Contract source entries use three states:

- **Frozen** means a callable type, trait, route, or facade exists and current
  consumers may compile against it.
- **Stubbed** means the module boundary and source location compile, but later
  milestone behavior remains deliberately absent.
- **Reserved** means this register row is the current frozen contract and names
  the source path owned by the future lane. A reserved entry is not evidence
  that the source file exists.

Owners are plan roles, not permanent people. One role owns each contract.
Consumers lists direct module consumers. MOD-DEPLOY consumes the whole system
and is implicit in every row unless it is the only consumer.

## Domain contracts

| ID        | Contract source and state                                                                                                                     | Owner          | Direct consumers                                                                                                                                                                  | Stub while waiting                              |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| MOD-QM    | Frozen: `crates/question_model/src/lib.rs` and generated `generated/api/` types                                                               | `architect`    | MOD-ID, MOD-RUN, MOD-STATE, MOD-TIME, MOD-SCORE, MOD-CAP, MOD-GEN, MOD-GRD, MOD-OBJ, MOD-STO, MOD-ADP-NAT, MOD-ADP-WW, MOD-ADP-QTI, MOD-ADP-H5P, MOD-EXPORT, MOD-WASM, MOD-CLIENT | n/a; root contract                              |
| MOD-ID    | Frozen: `crates/question_model/src/identity.rs` and `lifecycle.rs`                                                                            | `architect`    | MOD-OBJ, MOD-STO, MOD-SCHEMA, MOD-API-CAT                                                                                                                                         | n/a                                             |
| MOD-RUN   | Frozen: `crates/question_model/src/activity.rs`, `run_policy.rs`, and `crates/domain/src/run.rs`; compatibility scoring re-export in `run.rs` | `architect`    | MOD-STATE, MOD-SCORE, MOD-STO, MOD-SCHEMA, MOD-API-RUN, MOD-STATS                                                                                                                 | n/a                                             |
| MOD-STATE | Frozen: `crates/domain/src/attempt.rs` and `completion.rs`; compatibility completion re-export in `run.rs`                                    | `expert_coder` | MOD-GRD, MOD-WASM, MOD-API-RUN                                                                                                                                                    | n/a; pure transition contract                   |
| MOD-TIME  | Frozen: `crates/domain/src/timing.rs`                                                                                                         | `expert_coder` | MOD-WASM, MOD-API-RUN                                                                                                                                                             | n/a; pure verdict contract                      |
| MOD-SCORE | Frozen: `crates/domain/src/scoring.rs`                                                                                                        | `expert_coder` | MOD-STO, MOD-API-RUN                                                                                                                                                              | n/a; batch selection and incremental projection |
| MOD-CAP   | Frozen: `crates/question_model/src/capability.rs`, `crates/domain/src/policy.rs`, and the committed violation table                           | `expert_coder` | MOD-WASM, MOD-API-CAT, MOD-UI-EDITOR                                                                                                                                              | n/a; complete violation list contract           |
| MOD-GEN   | Frozen: `crates/domain/src/generator.rs` and `crates/domain/tests/seed_vectors.json`                                                          | `expert_coder` | MOD-ADP-NAT, MOD-WASM                                                                                                                                                             | n/a; parity evidence owned by `tester`          |
| MOD-GRD   | Frozen server boundary: `crates/grading/src/lib.rs`, `key.rs`, and `checker.rs`                                                               | `expert_coder` | MOD-ADP-NAT, MOD-API-RUN                                                                                                                                                          | n/a; server-only                                |

## Storage and adapter contracts

| ID          | Contract source and state                                                                                                            | Owner          | Direct consumers                                                                                                                 | Stub while waiting                        |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------ | -------------- | -------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| MOD-OBJ     | Frozen: `crates/objects/src/lib.rs`                                                                                                  | `expert_coder` | MOD-ADP-WW, MOD-ADP-QTI, MOD-EXPORT, MOD-API-ASSET, MOD-RETENTION                                                                | `MemoryObjectStore`                       |
| MOD-STO     | Frozen: `crates/store/src/lib.rs`                                                                                                    | `expert_coder` | MOD-SCHEMA, MOD-API-AUTH, MOD-API-CAT, MOD-API-COURSE, MOD-API-RUN, MOD-API-ASSET, MOD-WORKER, MOD-STATS, MOD-RETENTION, MOD-LTI | `MemoryStore`                             |
| MOD-SCHEMA  | Reserved: `schemas/migrations/`; RLS boundary starts in `crates/store/src/rls.rs`                                                    | `expert_coder` | MOD-STO                                                                                                                          | n/a; conformance remains on `MemoryStore` |
| MOD-ADP-NAT | Stubbed: `crates/adapters/native/src/lib.rs`, `generator.rs`, and `renderer_stub.rs`                                                 | `expert_coder` | MOD-API-CAT, MOD-API-RUN, MOD-WORKER                                                                                             | n/a                                       |
| MOD-ADP-WW  | Stubbed: `crates/adapters/webwork/src/lib.rs` and `pg_parser_stub.rs`                                                                | `expert_coder` | MOD-API-CAT, MOD-API-RUN, MOD-WORKER                                                                                             | recorded renderer fixtures                |
| MOD-ADP-QTI | Stubbed: `crates/adapters/qti/src/lib.rs` and `parser_stub.rs`                                                                       | `expert_coder` | MOD-API-CAT, MOD-WORKER                                                                                                          | `MemoryObjectStore`                       |
| MOD-ADP-H5P | Stubbed: `crates/adapters/h5p/src/lib.rs` and `import_stub.rs`                                                                       | `expert_coder` | MOD-API-CAT, MOD-WORKER                                                                                                          | n/a; ungraded capability declaration      |
| MOD-EXPORT  | Stubbed: `crates/export/src/lib.rs`, `docx.rs`, and `pdf.rs`                                                                         | `coder`        | MOD-WORKER, MOD-API-ASSET                                                                                                        | published fixture version                 |
| MOD-WASM    | Frozen: `crates/wasm/src/lib.rs` and browser facade `src/wasm/index.ts`, including key-free format, timer, and capability evaluation | `expert_coder` | MOD-UI-WIDGETS, MOD-UI-EDITOR                                                                                                    | n/a; exact export allowlist               |

## API and service contracts

The browser signatures in `src/api/client.ts` are the current route-group
contracts. Mock handlers implement them until M2 replaces the transport with
Rust routes. The mock is a dependency stub, not a second public API.

| ID             | Contract source and state                                                                                                 | Owner          | Direct consumers                                    | Stub while waiting                         |
| -------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------- | --------------------------------------------------- | ------------------------------------------ |
| MOD-API-AUTH   | Stubbed: `crates/server/src/auth.rs`; frozen client method in `src/api/client.ts`                                         | `expert_coder` | MOD-CLIENT, MOD-LTI                                 | `MemoryStore` and mock auth handler        |
| MOD-API-CAT    | Frozen client methods and mock routes in `src/api/client.ts` and `src/api/mock/handlers.ts`; reserved server `catalog.rs` | `expert_coder` | MOD-CLIENT                                          | `MemoryStore` and mock catalog handler     |
| MOD-API-COURSE | Frozen client methods and mock routes in `src/api/client.ts` and `src/api/mock/handlers.ts`; reserved server `course.rs`  | `expert_coder` | MOD-CLIENT                                          | `MemoryStore` and mock course handler      |
| MOD-API-RUN    | Frozen client methods and mock routes in `src/api/client.ts` and `src/api/mock/handlers.ts`; reserved server `run.rs`     | `expert_coder` | MOD-CLIENT                                          | `MemoryStore` and mock run handler         |
| MOD-API-ASSET  | Frozen client method and mock route in `src/api/client.ts` and `src/api/mock/handlers.ts`; reserved server `asset.rs`     | `expert_coder` | MOD-CLIENT, MOD-UI-RENDER                           | `MemoryObjectStore` and mock asset handler |
| MOD-WORKER     | Reserved: `crates/server/src/worker.rs`                                                                                   | `expert_coder` | MOD-API-CAT, MOD-API-RUN, MOD-EXPORT, MOD-RETENTION | `MemoryStore`                              |
| MOD-STATS      | Reserved: `crates/domain/src/statistics.rs` plus `Store` projections                                                      | `expert_coder` | MOD-API-CAT, MOD-RETENTION                          | `MemoryStore`                              |
| MOD-RETENTION  | Reserved: `crates/server/src/retention.rs`                                                                                | `expert_coder` | MOD-WORKER                                          | `MemoryStore`                              |

## Browser contracts

| ID               | Contract source and state                                                                                                          | Owner       | Direct consumers                                                             | Stub while waiting                         |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------- | ------------------------------------------ |
| MOD-CLIENT       | Frozen: `src/api/client.ts`, `contracts.ts`, and `runtime.tsx`                                                                     | `coder`     | MOD-UI-SHELL, MOD-UI-ATTEMPT, MOD-UI-BROWSE, MOD-UI-EDITOR, MOD-UI-GRADEBOOK | `src/api/mock/handlers.ts` and `client.ts` |
| MOD-UI-SHELL     | Frozen route and boundary contract: `src/route_contract.ts`, `routes.ts`, and `app.tsx`                                            | `architect` | none; browser composition root                                               | mock handlers                              |
| MOD-UI-WIDGETS   | Frozen reference signature: `src/components/multiple_choice_response.tsx`                                                          | `coder`     | MOD-UI-RENDER                                                                | reference multiple-choice widget           |
| MOD-UI-RENDER    | Stubbed envelope mapping in `crates/question_model/src/envelope.rs` and `src/pages/run_page.tsx`; reserved `question_renderer.tsx` | `coder`     | MOD-UI-ATTEMPT, MOD-UI-EDITOR                                                | published fixture envelopes                |
| MOD-UI-ATTEMPT   | Stubbed reference flow: `src/pages/run_page.tsx`                                                                                   | `coder`     | MOD-UI-SHELL                                                                 | mock handlers                              |
| MOD-UI-BROWSE    | Stubbed route surface: `src/pages/contract_pages.tsx`                                                                              | `coder`     | MOD-UI-SHELL                                                                 | mock handlers                              |
| MOD-UI-EDITOR    | Stubbed route surfaces: `src/pages/contract_pages.tsx`                                                                             | `coder`     | MOD-UI-SHELL                                                                 | mock handlers                              |
| MOD-UI-GRADEBOOK | Stubbed route surface: `src/pages/contract_pages.tsx`                                                                              | `coder`     | MOD-UI-SHELL                                                                 | mock handlers                              |

## Platform contracts

| ID         | Contract source and state                                                                             | Owner          | Direct consumers      | Stub while waiting   |
| ---------- | ----------------------------------------------------------------------------------------------------- | -------------- | --------------------- | -------------------- |
| MOD-LTI    | Reserved: `crates/server/src/lti.rs`                                                                  | `expert_coder` | none; platform edge   | LMS sandbox fixtures |
| MOD-DEPLOY | Stubbed local contract: `containers/compose.yaml`; reserved production infrastructure under `deploy/` | `expert_coder` | none; deployment edge | n/a                  |

## Shared artifact ownership

These artifacts have one writer. Consumers may read or validate them but must
not create a competing generator or copy.

| Artifact                                              | Owning module |
| ----------------------------------------------------- | ------------- |
| `crates/domain/tests/seed_vectors.json`               | MOD-GEN       |
| `crates/domain/tests/capability_violation_cases.json` | MOD-CAP       |
| `tests/fixtures/published_problem/`                   | MOD-QM        |
| `schemas/migrations/`                                 | MOD-SCHEMA    |
| `tests/test_wasm_export_allowlist.mjs`                | MOD-WASM      |
| `src/api/mock/handlers.ts`                            | MOD-CLIENT    |
| `containers/compose.yaml`                             | MOD-DEPLOY    |

Generated TypeScript under `generated/api/` and `generated/fixtures/` is
derivative. Its Rust model or fixture generator is the contract owner. The
generated output stays ignored and is never edited by a consumer lane.

## Frozen-contract change rule

A frozen contract change must land atomically. The same patch must:

1. update this register;
2. update the owning source contract;
3. update every direct consumer named in that row, including its stub;
4. regenerate derivative types or fixtures through their owning generator;
5. update conformance, secrecy, parity, or browser evidence affected by the
   change; and
6. record the behavior or decision in `docs/CHANGELOG.md`.

A contract change without every consumer is blocking. Do not merge a producer
first and repair consumers in later lane patches. Additive wire changes still
follow this rule because exhaustive TypeScript unions and Rust matches can make
an apparently additive variant a consumer-breaking change.

## Boundary invariants

- Rust modules, functions, and fields use snake case; Rust types and variants
  use upper camel case. Serde converts browser wire fields and discriminants to
  lower camel case. Raw wasm-bindgen snake-case exports stop at
  `src/wasm/index.ts`.
- MOD-GRD is server-only. It may never enter the MOD-WASM dependency closure,
  generated browser types, mock payloads, or client source.
- Published problem versions are shared and immutable. Educational records
  carry direct tenant ownership and cross only a server-authorized boundary.
- List contracts use bounded cursors. No public contract introduces an offset
  or an unbounded list operation.
- A newly issued parameterized attempt receives a fresh server-owned seed.
  Resume, re-render, audit, and debugging of that same attempt reuse its stored
  seed and provenance.
