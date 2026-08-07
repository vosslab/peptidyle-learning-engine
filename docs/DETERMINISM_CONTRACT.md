# Determinism contract

Seeded generation must produce byte-identical canonical output in native Rust
and browser WebAssembly. This is an exact contract, not a statistical test: the
render cache uses `(version_id, seed)`, and each historical attempt stores a
generated-parameter hash for reproducibility.

This replay contract does not authorize seed reuse for new work. Every newly
issued parameterized question instance receives a fresh server-owned seed so
students see new practice; only resume and re-render of the same attempt reuse
its recorded seed.

## Published identity

A seeded `RandomizationDefinition` pins all authoritative generation inputs:

- a stable generator ID;
- an additive generator version; and
- a `BTreeMap` of named parameter specifications.

Changing generator behavior creates a new generator version and a new published
problem version. An existing generator version and its committed seed hashes
remain unchanged so old assignments and attempts can still be reproduced.

`GeneratorReference` is shared by the published definition, generated variant,
and attempt provenance. The ID and version therefore travel together across
those boundaries.

## Random source

Generation uses `rand_chacha::ChaCha20Rng` directly. The `rand_chacha` generator
is deterministic and portable, with reference-vector testing. `StdRng` is not
suitable for this contract because its selected algorithm may change in any
release and may be platform-dependent.

A stored 64-bit seed is expanded to the 256-bit ChaCha20 seed by hashing the
domain separator `peptidyle-learning-engine/generator/v1\0` followed by the seed
in little-endian byte order. Sampling reads only `RngCore` bytes and uses a
local rejection sampler instead of version-sensitive distribution helpers.

## Stable output

- `BTreeMap` is required wherever iteration order can reach generated output.
- Integer ranges are inclusive and sampled without modulo bias.
- Decimal ranges are sampled as scaled integers and serialized as exact
  fixed-precision strings.
- Choice options retain authored order.
- Fixed and single-value parameters consume no random draw.
- `GeneratedVariant` is serialized with `serde_json` and hashed with SHA-256.
- Hashes are lowercase hexadecimal over the exact serialized bytes.

Floating-point tolerance is not accepted. Exact equality is required because a
near match would still select the wrong cache entry and fail the stored
reproducibility hash.

## Golden corpus

The reviewed golden table is `crates/domain/tests/seed_vectors.json`.
It currently covers `parameter-map@1` with 65 seeds: 0 through 63 and the
maximum `u64` value. Its definition exercises every current parameter branch:

- single, ranged, and full-width `i64` integer sampling;
- single, ranged, and zero-place decimal sampling;
- single and multiple choice selection; and
- fixed values.

Every registered generator must have at least 50 ordered seed entries and
cover every branch it implements. A mismatch stops at an error naming the
generator and first divergent seed.

This JSON file is intentionally tracked. It is a reviewed compatibility
baseline and work evidence, not disposable build output. Regenerate it only for
a deliberate new generator version or a reviewed correction, using the tracked
`crates/domain/examples/generate_seed_vectors.rs`:

```bash
cargo run -p domain --example generate_seed_vectors -- --write
```

Review the resulting diff before accepting the new hashes.

## Verification

The native test and browser test include the same assertion implementation from
`crates/domain/tests/determinism_support.rs`.

Run the native gate:

```bash
cargo test -p domain --test test_determinism -- --nocapture
```

Install the version-matched browser test runner once, then run the real
headless-Chromium gate:

```bash
./devel/setup_wasm_tests.sh
node tests/playwright/e2e_wasm_determinism.mjs
```

The setup command installs under ignored `target/tooling/`; it does not install
globally. The browser command is a local codebase test and does not deploy a
server.
