# Determinism contract

This document defines the narrow reproducibility PLE needs without inventing a
general historical replay system. Product intent comes from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

## Core rule

An open Assessment Attempt must keep the exact Published Question Revision and,
for a Pool, the exact Pool Revision and selected Published Question Revision
delivered to the Student. It also keeps the randomization value or opaque
Question Backend state required to resume and interpret the response.

This does not mean every new Attempt receives the same variant. A new Attempt
may use new server-owned randomization according to Assessment settings.
Resuming the same Attempt uses its retained values.

## Contract layers

| Layer | Authoritative input | Required result | Owner |
| --- | --- | --- | --- |
| Native static render | Exact Question Revision and authored answer-choice-order setting | Same answer-free static native presentation | Native Question Backend |
| Backend-owned render | Exact source plus backend-owned state | Opaque presentation and state sufficient for the backend to resume/interpret | Selected Question Backend |
| Assessment selection | Exact Assessment position and Question/Pool Revision | Fixed Question selection for that Attempt | PLE server and Store |
| Response save | Authenticated open Attempt, position, and complete response | One replaceable saved response | PLE server and Store |
| Whole submission | Exact Attempt and saved responses | One submitted Attempt with all saved responses finalized together | PLE server and Store |

Checksums can detect mismatched valid data. They do not authenticate a Student,
authorize a record, protect transport, or decide a grade.

## Revision identity

Question ID identifies a stable Published Question lineage. Question Revision
Reference identifies exact immutable source. Pool Revision Reference identifies
exact immutable Pool membership. These are the content identities retained in
Student Work.

Current implementation may store `QuestionAttemptReproductionDetails`, renderer
versions, rendered-document digests, or presentation tokens. Preserve a field
only when the responsible backend needs it to interpret the delivered Question
or verify a concrete boundary. Human Guidance does not require software-version
snapshots, pixel reproduction, rendered-page archives, or a universal replay
service.

## Native PLE Question JSON

Native PLE Question JSON is static, private, unpublished, unversioned, and
strictly validated. It receives no random seed. The trusted native backend uses
the exact Question Revision and authored answer-choice-randomization setting to
produce an answer-free presentation. The browser receives
no Answer Key or source and does not implement a second source parser.

The browser-safe Wasm facade may validate public response shape or other
answer-free values. It never accepts an Answer Key, generates correctness, or
becomes grading authority.

## Presentation correspondence

A server may bind a presentation to an Attempt position using a nonce,
checksum, or short presentation-scoped references. These values prevent a
response from being applied to the wrong valid presentation. They are
consistency data, not public identity or security credentials.

The exact algorithm and field set belong to the implementation contract. Do not
preserve them as product concepts after the owning implementation changes.

## WeBWorK

PLE sends exact trusted PG/PGML source and randomization state to the private
renderer. It stores or forwards the opaque presentation/state needed by the E1
contract. The browser captures bounded ordered form pairs without interpreting
controls. The renderer alone interprets them and returns the immutable credit
fraction.

PLE has no control-specific replay map or parallel native interpretation. A
cache or rerender cannot silently replace the opaque state bound to an existing
Attempt.

## Attempt continuity

The Student sees one Question at a time and can navigate among all fixed
positions. Reconnect and reload restore the same open Attempt, selected
Questions, saved responses, and deadline. They do not create a successor
Question or reset time.

At expiration, the server submits the whole Attempt, finalizes the complete
responses saved before the deadline, and leaves other Questions unanswered.
This is ordinary Assessment behavior, not a separate recovery state.

## Evidence

Tests should prove deterministic output only where the backend promises it,
exact Attempt selection continuity, mismatch refusal, saved-response recovery,
and whole-Assessment submission. Fixture vectors are permanent only when they
define a stable external format; implementation-specific hashes, call order,
or current inventories are one-time evidence.
