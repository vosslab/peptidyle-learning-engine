# WP-V1 deterministic simulator decisions

## Status

Independently ACCEPTED on 2026-08-10. See the
[independent review](../audits/wp_v1_deterministic_decisions_review.md).
This package owns a small offline simulator utility only; it does not import browser,
product, Rust, Wasm, account, enrollment, score, SQL, checksum, or canonical-JSON code.

## Contract

- An explicit unsigned 32-bit master seed and a lowercase ASCII label create one named stream.
- FNV-1a mixes the seed and label; splitmix32 expands that value into sfc32 state.
- Each stream is independent, so consuming one label cannot affect another label.
- Selection rejects invalid bounds and uses rejection sampling to avoid modulo bias.
- Allocation does not mutate its candidate collection, and report identifiers sort on a copied
  list.

## Owned files

- `tests/playwright/simulator/rng.ts`
- `tests/test_simulator_rng.mjs`

## Validation evidence

On 2026-08-10, `npx --yes tsx --test tests/test_simulator_rng.mjs` passed all six
focused tests. They cover the fixed replay vector, named-stream isolation, invalid
input refusal, explicit rejection sampling, the `2^32` selection bound, allocation,
choice replay, and nonmutating public-identifier ordering.

`npx --yes prettier --check tests/playwright/simulator/rng.ts tests/test_simulator_rng.mjs`
passed. The isolated strict TypeScript command passed:
`npx --yes --package typescript tsc --ignoreConfig --strict --noEmit --target ES2022`
`--module NodeNext --moduleResolution NodeNext tests/playwright/simulator/rng.ts`.
ASCII, fewer-than-100-character line, Markdown-link, and diff-whitespace checks also
passed for this workstream.

Repository-wide `npx tsc -p tsconfig.lint.json` and ESLint remain blocked only because
the checkout lacks `node_modules` and generated modules. They were not represented as
passing gates; the focused temporary-package validations above establish this package's
local correctness.
