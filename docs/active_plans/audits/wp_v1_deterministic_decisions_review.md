# WP-V1 deterministic decisions independent review

## Type Safety

- ACCEPTED: `rng.ts` has strict, narrow exported contracts. `MasterSeed` is branded only
  by `validate_master_seed`; no `any`, mutable module state, or unchecked collection read
  crosses the module boundary. The only cast is the documented brand constructor.
- ACCEPTED: all sfc32 state transitions, FNV-1a multiplication, splitmix32 mixing, and
  selection arithmetic normalize to unsigned 32-bit values. The implementation matches the
  workstream wording: FNV-1a combines the seed and label, splitmix32 expands that result, and
  sfc32 produces decisions.
- ACCEPTED: `sort_public_identifiers` copies before sorting and explicitly breaks equal-identifier
  ties by original index. Allocation similarly copies candidates before removal.

## Module Boundaries

- ACCEPTED: the offline utility imports nothing. Its Node test imports only the utility. No
  browser, product, Rust, Wasm, account, enrollment, score, SQL, checksum, or canonical-JSON
  dependency or concept was introduced.
- ACCEPTED: named streams carry all mutable state per instance. `create_named_stream` derives each
  instance solely from its validated seed and label, so there is no global or cross-label coupling.
- ACCEPTED: the workstream now records the actual focused command, six-test result, isolated strict
  TypeScript check, formatting, and the absent-dependency baseline without claiming the unavailable
  repository-wide gates passed. Its status correctly leaves independent re-review pending.

## Compile-Time Errors

- ACCEPTED: isolated strict compilation of `tests/playwright/simulator/rng.ts` succeeded with the
  recorded NodeNext flags. The focused Node test also passed: 6 tests, 6 passes, 0 failures.
- ACCEPTED: Prettier, ASCII checks, and no-index diff checks passed for the two source files and
  the workstream artifact.
- NOTE: repository `node_modules` is absent. Consequently the canonical `node --import tsx` and
  `npx tsc -p tsconfig.lint.json` commands cannot resolve locally installed dependencies. This is a
  missing dependency baseline, not a WP-V1 code failure. Equivalent temporary-package checks
  passed.
- ACCEPTED: every owned line is fewer than 100 characters. The corrected source, test, and
  workstream pass Prettier, ASCII, line-length, Markdown-link, and diff-whitespace checks.

## Type-Level Tests

- ACCEPTED: the fixed replay vector is an external oracle rather than a self-comparison. The named
  stream isolation case first captures `observer.review`, consumes two independently-created
  `learner.answer` streams, and proves the observer replay remains unchanged. Allocation and report
  ordering also prove nonmutation with fixed expected outputs.
- ACCEPTED: a scripted stream returns the rejected tail value `0xffff_ffff` and then `5` for a
  bound of three; the test proves the returned index and exactly two stream consumptions. It also
  accepts `0x1_0000_0000`, rejects the value immediately above it, and preserves the fixed replay
  and cross-stream-isolation proofs.

Result: ACCEPTED. WP-V1 meets its bounded deterministic-decision contract. Repository-wide
TypeScript and lint gates remain an honestly recorded dependency baseline, not an acceptance claim.
