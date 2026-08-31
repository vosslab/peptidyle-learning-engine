# Determinism contract

This document defines what PLE reproduces exactly, what it merely checks for
consistency, and what must remain server-owned. It applies to native generated
questions, WeBWorK renders, issued student presentations, cache entries, and
prefetch reservations.

The central rule is deliberately narrow: **the same immutable inputs must
reproduce the same authoritative source_object_reference.** It does not mean that every new
attempt receives the same presentation. A newly issued student attempt gets a
fresh seed and a fresh presentation nonce; resuming or reproducing that same
attempt uses the stored values.

## Contract layers

| Layer                         | Authoritative inputs                                                       | Exact result                                              | Owner                              |
| ----------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------- | ---------------------------------- |
| Question Variation Parameters | generator reference, definition, seed                                      | `QuestionVariationParameters` and SHA-256                 | `domain` and Wasm                  |
| Native issued question        | immutable question version, seed                                           | envelope and Question Attempt Reproduction Details        | trusted server backend             |
| WeBWorK safe render           | problem, immutable version, source source_object_reference, seed, renderer | safe cached envelope and sanitized markup                 | private adapter/renderer           |
| Student presentation          | answer-free envelope, asset bindings, stored nonce                         | Question Response Format, rendered IDs, descriptor digest | trusted server; browser may verify |
| Submission                    | authenticated attempt, idempotency key, student response                   | one stored receipt or conflict                            | trusted server/store               |

The first four rows are reproducibility and consistency contracts. The final
row is an authorization and lifecycle contract. No checksum authenticates a
student, replaces TLS, or makes a client-side grade authoritative.

## Immutable identity

Published question identity is the pair of durable problem and immutable
version IDs. A seeded `QuestionVariationDefinition` additionally carries a
`QuestionGeneratorReference` with a stable generator ID and additive generator
version. A changed generator implementation therefore requires a new generator
version and a new published question version; historical definitions remain
resolvable.

An issued `QuestionAttempt` records its immutable problem version, server-owned
seed, generated-parameter hash, and `QuestionAttemptReproductionDetails`. The Question Attempt Reproduction Details records the
Question Backend Version, generator where applicable, Source Object Reference,
Question Renderer Version where applicable, Question Grader Version, asset objects,
and rendered-question hash.
This is the audit record used to reject a rerender that no longer reproduces
the issued question.

The authoritative types are in
[`crates/question_model/src/generation.rs`](../crates/question_model/src/generation.rs),
[`crates/domain/src/generator.rs`](../crates/domain/src/generator.rs), and
[`crates/question_model/src/student_work.rs`](../crates/question_model/src/lib.rs).

## Seeded generation

`domain::generator::generate` is a pure function of a `QuestionSeed` and a
`QuestionVariationDefinition`. The implementation makes these compatibility
choices explicit:

- `ChaCha20Rng` receives a 256-bit key derived from the domain separator
  `peptidyle-learning-engine/generator/v1` and the stored 64-bit seed in
  little-endian order.
- Sampling consumes only `RngCore` bytes through PLE's own rejection sampler;
  it does not depend on `rand` distribution helpers.
- `BTreeMap` fixes parameter iteration and generated-output order.
- Integer ranges are inclusive and unbiased; decimal ranges are scaled integer
  values rendered as fixed-precision strings.
- Fixed and single-value parameters consume no random draw, so adding one does
  not perturb unrelated random values.
- Canonical generated output is `serde_json` bytes of `QuestionVariationParameters` and
  its hash is lowercase SHA-256 hexadecimal.

The browser Wasm module uses the same Rust `domain` code. It does not have an
independent TypeScript randomizer. This makes cross-target agreement a tested
property rather than an implementation convention.

### Browser Wasm boundary

The browser package is `wasm_bridge`, built as a `cdylib` for
`wasm32-unknown-unknown` by the production build. The lockfile-matched bindgen
tool emits the browser module under `dist/wasm/`; the sole handwritten host
boundary is [`src/wasm/index.ts`](../src/wasm/index.ts).
The generated JavaScript owns Wasm memory and strings. Rust owns the received
JSON values and returns serialized safe reports. Malformed public JSON becomes
a JavaScript string error; a structurally invalid but well-formed response
returns a report. No export accepts an answer key or produces correctness.

Flat v2 source is answer-bearing, so its parser and compiler remain in the
server-only native adapter. Browser parity therefore covers the actual public
boundary: answer-free `QuestionResponseFormat` values compiled from the current
flat v2 MC and MATCH Question Types, and `StudentResponse` values. Inline native,
generated-Node, and headless-browser cases cover valid selections and matching
permutations, empty-response boundaries, malformed JSON errors, and repeated
calls. They compare serialized reports exactly. This is a behavior test, not a
second flat-source reader or a persisted fixture contract.

The release plan does not currently define a browser bundle-size or startup
budget. The release build remains the source_object_reference under inspection, but a measured
byte count or timing is not yet an acceptance threshold.

The reviewed compatibility baseline is
[`crates/domain/tests/seed_vectors.json`](../crates/domain/tests/seed_vectors.json).
It currently covers `parameter-map@1` with seeds 0 through 63 and `u64::MAX`.
The same assertion implementation is used by the native test and the
headless-browser Wasm test. Regenerate the fixture only for a deliberate new
generator version or reviewed correction:

```bash
cargo run -p domain --example generate_seed_vectors -- --write
```

Review the complete fixture diff before accepting new hashes. The fixture is a
permanent compatibility baseline, not generated scratch output.

## Issued presentation

An issued presentation is answer-free and presentation-specific. Its v1
descriptor includes:

- descriptor version, immutable question version, and stored seed;
- a server-minted 16-byte presentation nonce;
- title, prompt blocks, public Question Response Format, item order, and response
  constraints;
- durable asset identity plus authored and selected-rendition checksums; and
- the rendered IDs and canonical public basis of every addressable item.

The server computes SHA-256 over the versioned binary descriptor and persists
the full 32-byte digest with the nonce. The student receives the nonce in the
answer-free envelope and a `pd1_` base64url token containing the first 128 bits
of the digest. Rebuilding the same envelope with the persisted nonce must
reproduce the stored full digest exactly.

A fresh nonce is intentional. It lets the server give one presentation-scoped
identity to each rendered item, even when the same logical question and seed
are issued at another time. Therefore `(version, seed)` alone identifies a
generated variant, while `(version, seed, presentation nonce)` identifies the
specific v1 presentation.

The codec and builder are owned by
[`crates/question_model/src/presentation/`](../crates/question_model/src/presentation/).
TypeScript calls the Rust-owned Wasm verifier; it must not reimplement the
descriptor codec, CRC, or SHA-256 rules.

### Rendered item IDs

Each addressable choice, blank, matching side, ordering item, Hotspot Surface,
or Hotspot Region receives a four-lowercase-hex `PresentationResponseItemReference`. It is CRC-16/CCITT-
FALSE over a domain-separated basis that includes the nonce, version, seed,
role, ordinal, durable ID, and SHA-256 of canonical public item content.

CRC16 is a compact correspondence value, not an identity or security token.
The builder permits at most 32 addressable items, checks all IDs for uniqueness
within the presentation, retries with a new nonce up to eight times, and fails
closed if it cannot issue an unambiguous presentation. Durable Response Item Reference and
other internal identities remain server-side.

### Checksum roles

| Value                                  | Detects or proves                                                    | Does not provide                                                 |
| -------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Source-source_object_reference SHA-256 | immutable source bytes match their published record                  | authorization or a rendered output                               |
| Generated-variant SHA-256              | same generator definition and seed produced the reviewed values      | a student presentation or grade                                  |
| Safe-render SHA-256                    | cached WeBWorK safe render has stable provenance                     | private replay state or student authorization                    |
| Full presentation SHA-256              | persisted descriptor agrees with a reconstructed public presentation | authentication, transport integrity, or pixel rendering          |
| `pd1_` 128-bit public token            | compact browser/server presentation-consistency comparison           | a durable secret or a substitute for the full stored digest      |
| Rendered-item CRC16                    | selected item corresponds to one unique object in this presentation  | collision resistance across presentations or a security boundary |
| Idempotency record                     | exact retry is replayed and changed retry conflicts                  | question correctness                                             |

## WeBWorK cache and replay

The WeBWorK adapter caches only safe rendered output in content object storage.
Its key is deterministic from `(problem, version, seed)` and validates cache
schema, immutable source source_object_reference, version, seed, student title, and nonempty
renderer identity. Cached bytes contain an answer-free shared envelope,
sanitized markup, source-source_object_reference binding, and renderer identity. They never
contain PG source, credentials, answer keys, or upstream field/value mapping.

The cache is a reproducibility optimization, not a promise that no renderer
work occurs. On a cache hit during **issue**, current code invokes the private
renderer once with the same source and seed to reconstruct its private replay
mapping and compares the resulting safe render with the immutable cached
render. That adapter work emits the `ple.webwork.cache` `renderer_call` and
`cache_hit` witnesses. In contrast, an already-issued attempt's active or
submitted `GET` returns its persisted presentation or receipt snapshot; it
does not call adapter `reproduce`, read the safe-render cache, call the
renderer, or emit either witness. This distinction is important for latency
estimates and operational evidence.

The Store persists a bounded `WebworkGradeReplayStateV1`: immutable
problem/version/source/seed/Question Renderer Version, presentation digest, and a
redacted mapping from presentation-scoped rendered item IDs to upstream fields and values. The
mapping is course-owned, validated, RLS-protected, and never serialized to the
browser or cache.

Issued native-flat and WeBWorK attempts also retain checksummed, server-only
first-grade contracts. A first grade validates its family-owned contract and
fails unavailable if required material is absent or corrupt; it does not reread
a current published Question definition, private flat grader, or renderer to repair an
earlier issuance.

The normal run route threads that private mapping from
`WebworkIssuedAttempt` through `IssuedAttemptMetadata`, prefetch promotion, and
attempt persistence. It stores a checksummed public presentation snapshot and
matching server-only grading envelope, then translates browser-rendered IDs
through that protected envelope before one private grade RPC. Submitted reads
and retries cross-check those persisted artifacts against their owning attempt;
they do not reproduce a safe envelope or call a renderer. Successful
submission and terminal instructor action delete the replay row in the same
Store transaction.

This is an implemented offline slice, not acceptance of WP-P1 through WP-P6.
The following remain planned integration and acceptance work:

- prove the one-call path against disposable PostgreSQL and the private live renderer; and
- expose `LearnerRunScreenV1` and its compact, type-free answer wire as the
  browser's authoritative active-attempt route.

The approved integration sequence and acceptance criteria live in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
Until the compact student wire cuts over, submission is still the tagged
`StudentResponse` body. Server-side response-shape validation uses the
checksummed issued snapshot and translates through its server-only grading
envelope, not an untrusted browser-selected question type or mutable renderer.

## Prefetch and replay

Prefetch is an authenticated, bodyless `POST` tied to the active predecessor
attempt. The server selects the next position and fresh seed, renders the
question, creates a course/student/run/predecessor-bound reservation, and
persists its parameter hash, provenance, and presentation binding. It does not
start the next timer or let the browser choose seed, version, backend, source,
or grading state.

When a reservation is reused, the server verifies its immutable version, seed,
parameter hash, provenance, and stored presentation binding. It rebuilds the
presentation with the persisted nonce and refuses if the full digest differs.
Promotion consumes the reservation atomically with successor issuance; a
committed receipt is the only authority that activates the next attempt.

This is why prefetch may prepare non-secret work early without weakening timing
or grading ownership. A fresh future attempt is not created by browser state;
only a matching, server-owned reservation can become one.

## Current verification

Run the narrow gates that prove the implemented layers:

```bash
# Native generated-parameter compatibility baseline.
cargo test -p domain --test test_determinism -- --nocapture

# Presentation descriptor, nonce, collision, and public-rebuild rules.
cargo test -p question_model presentation

# Native and generated-Node execution of the shared answer-free flat-v2 response corpus.
cargo test -p wasm_bridge --test flat_v2_response_format_native
./pipeline/build_wasm.sh
node tests/e2e/e2e_wasm_bridge.mjs

# Browser-Wasm proof: the canonical production browser suite loads the Wasm
# module from dist/ and the instructor scenario asserts visible wasm mode.
./run_playwright_tests.sh --scenario instructor_authoring
```

The fixed corpus is `crates/wasm/flat_v2_response_format_corpus.json`; native Rust,
generated Node bindings, and production browser Wasm consume it unchanged. The Node/Rust checks
prove generated-parameter and key-free public-response parity; the canonical instructor scenario
proves that the shipped `dist/` module initializes in Chromium and visibly reports `wasm` mode.
These gates do not prove end-to-end submission digest enforcement. Do not claim the planned compact
payload or one-RPC WeBWorK grade behavior from these checks. Those require the
payload-plan integration gates, Store conformance, private-renderer
request-count tests, and browser route tests specified in the active plan.
