# Determinism contract

This document defines what PLE reproduces exactly, what it merely checks for
consistency, and what must remain server-owned. It applies to static PLE
Question JSON, WeBWorK renders, issued student presentations, cache entries,
and prefetch reservations.

The central rule is deliberately narrow: **the same immutable inputs must
reproduce the same authoritative Source Object Reference.** It does not mean that every new
attempt receives the same presentation. A newly issued student attempt gets a
fresh seed and a fresh presentation nonce; resuming or reproducing that same
attempt uses the stored values.

## Contract layers

| Layer                           | Authoritative inputs                                                                 | Exact result                                                                                                                                                                             | Owner                              |
| ------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| Static PLE Question JSON render | immutable Question Revision, Question Seed                                           | Question Variation Presentation and Question Attempt Reproduction Details                                                                                                                | trusted server backend             |
| WeBWorK safe render             | Question, immutable Question Revision, source Object Reference, seed, renderer       | safe cached Question Variation Presentation                                                                                                                                              | private adapter/renderer           |
| Student issuance                | Question Variation Presentation, server-held Question Asset Renditions, stored nonce | Question Presentation and server-held Issued Question Presentation with Question Presentation Response Format, Presentation Response Item References, and Question Presentation Checksum | trusted server; browser may verify |
| Submission                      | authenticated attempt, idempotency key, student response                             | one stored receipt or conflict                                                                                                                                                           | trusted server/store               |

The first four rows are reproducibility and consistency contracts. The final
row is an authorization and lifecycle contract. No checksum authenticates a
student, replaces TLS, or makes a client-side grade authoritative.

## Stable lineage and revision identity

Question ID identifies the stable Published Question lineage. Question Revision
Reference, the pair of Question ID and immutable Question Revision Number,
identifies one exact source-bearing revision. Current static PLE Question JSON has no
Question-authored Question Variation Rule. WeBWorK and iMathAS retain their
backend-owned variation behavior. A future Question Generator requires
immutable registered source data and a complete publication-to-reproduction
path.

An issued `QuestionAttempt` records its immutable Question Revision Reference,
server-owned Question Seed, and `QuestionAttemptReproductionDetails`. The
reproduction details record Question Backend Version, Source Object Reference,
Question Renderer Version where applicable, Question Grader Version, asset
objects, and Rendered Question SHA-256.
This is the audit record used to reject a rerender that no longer reproduces
the issued question.

The authoritative types are in
[`crates/question_model/src/generation.rs`](../crates/question_model/src/generation.rs)
and [`crates/question_model/src/student_work.rs`](../crates/question_model/src/student_work.rs).

## Future source-owned generation

The browser has no independent PLE generation engine. A future generator is
admitted only with immutable registered Question Generator source data and the
complete trusted publication, issue, grading, repair, and reproduction path.
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

PLE Question JSON version 3 source is answer-bearing, so its parser and compiler remain in the
server-only PLE Question Backend. Browser parity therefore covers the actual public
boundary: answer-free `QuestionResponseFormat` values compiled from the current
PLE Question JSON version 3 MC and MATCH Question Types, and `StudentResponse` values. Inline PLE,
generated-Node, and headless-browser cases cover valid selections and matching
permutations, empty-response boundaries, malformed JSON errors, and repeated
calls. They compare serialized reports exactly. This is a behavior test, not a
second PLE Question JSON source reader or a persisted fixture contract.

The release plan does not currently define a browser bundle-size or startup
budget. The release build remains the source_object_reference under inspection, but a measured
byte count or timing is not yet an acceptance threshold.

Current static PLE Question JSON has no Question Generator vector fixture or
generator-regeneration command. Its existing reproduction evidence is the
server-owned PLE Question Backend issue/reproduce path in
[`crates/adapters/ple/src/lib/tests.rs`](../crates/adapters/ple/src/lib/tests.rs)
and the Question Presentation descriptor checks at the Rust/Wasm boundary. A
future source-owned Question Generator must introduce its own reviewed,
immutable evidence with the complete publication, issue, grading, repair, and
reproduction path.

## Issued presentation

An issued presentation is answer-free and presentation-specific. Its v1
descriptor includes:

- descriptor version, immutable question revision, and stored seed;
- a server-minted 16-byte presentation nonce;
- title, prompt blocks, public Question Response Format, item order, and response
  constraints;
- durable asset identity plus authored and selected-rendition checksums; and
- the Presentation Response Item References and deterministic public basis of every addressable Response Item.

The server computes SHA-256 over the versioned binary descriptor and persists
the full 32-byte Question Presentation Checksum with the nonce. The student receives the nonce in the
answer-free Question Presentation and a `pd1_` base64url token containing the first 128 bits
of the Question Presentation Checksum. Rebuilding the same Question Presentation with the persisted nonce must
reproduce the stored full Question Presentation Checksum exactly.

A fresh nonce is intentional. It lets the server give one presentation-scoped
Presentation Response Item Reference to each addressable Response Item, even when the same logical question and seed
are issued at another time. Therefore `(version, seed)` alone identifies a
generated variant, while `(version, seed, presentation nonce)` identifies the
specific v1 presentation.

The codec and builder are owned by
[`crates/question_model/src/presentation/`](../crates/question_model/src/presentation/).
TypeScript calls the Rust-owned Wasm verifier; it must not reimplement the
descriptor codec, CRC, or SHA-256 rules.

### Presentation Response Item References

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

| Value                                      | Detects or proves                                                                                         | Does not provide                                                                    |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Source Object Checksum                     | immutable source bytes match their published record                                                       | authorization or a rendered output                                                  |
| Generated-variant SHA-256                  | same Question Seed and backend-owned variation inputs produced the reviewed Question Variation Parameters | a student presentation or grade                                                     |
| Safe-render SHA-256                        | cached WeBWorK safe render has a stable Source Object Reference and Question Renderer Version             | private replay state or student authorization                                       |
| Question Presentation Checksum             | persisted descriptor agrees with a reconstructed public presentation                                      | authentication, transport integrity, or pixel rendering                             |
| `pd1_` Question Presentation Token         | compact browser/server presentation-consistency comparison                                                | a durable secret or a substitute for the full stored Question Presentation Checksum |
| Presentation Response Item Reference CRC16 | selected Response Item corresponds to one unique object in this presentation                              | collision resistance across presentations or a security boundary                    |
| Idempotency record                         | exact retry is replayed and changed retry conflicts                                                       | question correctness                                                                |

## WeBWorK cache and replay

The WeBWorK adapter caches only safe rendered output in content object storage.
Its key is deterministic from `(Question Revision Reference, Question Seed)` and validates cache
schema, immutable Source Object Reference, Question Revision Reference, Question Seed, student title, and nonempty
renderer identity. Cached bytes contain an answer-free shared Question Variation Presentation,
Source Object Reference binding, and renderer identity. They never
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

During issuance, the adapter holds bounded `WebworkQuestionAttemptReplayDetails`:
immutable Question Revision Reference, Source Object Reference, Question Seed, Question Renderer Version, Question Presentation
Checksum, and a redacted mapping from Presentation Response Item References to
upstream fields and values. The mapping is never serialized to the browser or
cache. A course-owned, validated, RLS-protected durable Attempt record is
required before a mounted WeBWorK delivery or grading route can rely on it.

Issued PLE Question JSON and WeBWorK attempts also retain checksummed, server-only
first-grade contracts. A first grade validates its Question Backend-specific contract and
fails unavailable if required material is absent or corrupt; it does not reread
a current published Question Revision, private PLE Question JSON grader, or renderer to repair an
earlier issuance.

The future delivery route will thread that private mapping from
`WebworkIssuedAttempt` through attempted-work persistence. It must store a
checksummed public presentation snapshot and matching server-only Question
Grading Input, then translate browser-supplied Presentation Response Item References through that protected
record before one private grade RPC. Submitted reads and retries must
cross-check those persisted artifacts against their owning Attempt;
they do not reproduce a safe Question Variation Presentation or call a renderer. Successful
submission and terminal instructor action delete the replay row in the same
Store transaction.

This is an implemented offline slice, not acceptance of the complete Question Presentation payload contract.
The following remain planned integration and acceptance work:

- prove the one-call path against disposable PostgreSQL and the private live renderer; and
- expose the current `StudentAssignmentAttemptScreen` answer-free screen
  contract and the compact, type-free answer wire as the browser's authoritative
  active Question Attempt route.

The approved integration sequence and acceptance criteria live in
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md).
Until the compact student wire cuts over, submission is still the tagged
`StudentResponse` body. Server-side response-shape validation uses the
checksummed issued snapshot and translates through its server-only grading
Question Grading Input, not an untrusted browser-selected question type or mutable renderer.

## Prefetch and replay

Prefetch is an authenticated, bodyless `POST` tied to the active predecessor
attempt. The server selects the next position and fresh seed, renders the
question, creates a Course/Student/Assignment Attempt/predecessor-bound reservation, and
persists its Question Attempt Reproduction Details, and presentation binding. It does not
start the next timer or let the browser choose seed, version, backend, source,
or grading state.

When a reservation is reused, the server verifies its immutable version, seed,
Question Attempt Reproduction Details, and stored presentation binding. It rebuilds the
presentation with the persisted nonce and refuses if the full Question Presentation Checksum differs.
Promotion consumes the reservation atomically with successor issuance; a
committed receipt is the only authority that activates the next attempt.

This is why prefetch may prepare non-secret work early without weakening timing
or grading ownership. A fresh future attempt is not created by browser state;
only a matching, server-owned reservation can become one.

## Current verification

Run the narrow gates that prove the implemented layers:

```bash
# Presentation descriptor, nonce, collision, and public-rebuild rules.
cargo test -p question_model presentation

# Rust and generated-Node execution of the shared answer-free Question Response Format Fixture Set.
cargo test -p wasm_bridge --test ple_question_json_response_format_native
./pipeline/build_wasm.sh
node tests/e2e/e2e_wasm_bridge.mjs

# Browser-Wasm proof: the canonical production browser suite loads the Wasm
# module from dist/ and the instructor scenario asserts visible wasm mode.
./run_playwright_tests.sh --scenario instructor_authoring
```

The fixed Question Response Format Fixture Set is `crates/wasm/ple_question_json_response_format_fixture_set.json`; Rust,
generated Node bindings, and production browser Wasm consume it unchanged. The Node/Rust checks
prove key-free public-response parity; the canonical instructor scenario
proves that the shipped `dist/` module initializes in Chromium and visibly reports `wasm` mode.
These gates do not prove end-to-end Question Presentation Checksum enforcement. Do not claim the planned compact
payload or one-RPC WeBWorK grade behavior from these checks. Those require the
payload-plan integration gates, Store conformance, private-renderer
request-count tests, and browser route tests specified in the active plan.
