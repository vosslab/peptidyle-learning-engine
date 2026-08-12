# Flat-question family evolution companion

> **Authority:** This is a non-authoritative companion to
> [release_completion_plan.md](release_completion_plan.md). That plan owns package names, sequence,
> milestone acceptance, and release status. This companion records the detailed source/runtime
> contract and remaining family work.

## Status

PLE flat-question JSON version 2 now implements the eight required runtime families: MC, MA, FIB,
MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT. It is the only native flat source reader.

The implemented slice includes strict answer-bearing source parsing, public/private compilation,
family registration, answer-free publication validation, browser decoding, key-free response
validation, accessible learner controls, and isolated all-or-nothing server grading. It does not
complete WP-RC5: the visual author editor currently exposes v2 single choice, the secure learner
payload cutover remains a prerequisite, and all-family PostgreSQL/object-store acceptance, pilot
content, hotspot pointer authoring, and independent family review remain open.

## Source decision

PLE flat-question JSON version 2 is the internal all-family source contract. It follows the reviewed
lossless semantics of QTI Package Maker's `MC`, `MA`, `MATCH`, `NUM`, `FIB`, `MULTI_FIB`, and
`ORDER` item models while keeping PLE metadata, policies, stable IDs, and public/private compilation
explicit. HOTSPOT is a bounded PLE extension because the reviewed QTI Package Maker item model does
not define it.

This decision does not claim an external QTI-JSONL contract. If QTI Package Maker later supplies a
normative QTI-JSONL specification and fixtures, a dedicated adapter may translate them into the same
PLE compiler outputs. External line framing and vendor vocabulary do not enter the question model,
grading API, storage schema, or learner response format.

## Source and runtime boundary

```text
private PLE flat JSON source
             |
             v
 strict versioned native compiler
          /             \
         v               v
answer-free public     grader-only key,
question model         feedback, and binding
```

- `crates/adapters/native/src/flat_question.rs` owns the stable v2-only facade.
- `crates/adapters/native/src/flat_question/v2.rs` owns the closed version 2 source shapes and
  compiler.
- Stores, SQL, HTTP, generated clients, Solid components, and grading consume PLE runtime types;
  they do not parse source JSON independently.
- Authored and published source remains private and answer-bearing. Public questions contain only
  render content, response shape, policies, metadata, and typed asset references.
- Grader material binds the private key and feedback to the exact public question by SHA-256.
- Existing source is never silently reinterpreted. Unknown fields, duplicate members, and unknown
  versions refuse.

PLE runtime response IDs are semantic and independent of display labels or positions. The secure
payload package may project attempt-specific rendered item IDs and a presentation digest at the
browser boundary without changing these durable source identities.

## Family contract

| Family    | Implemented source/runtime contract                                                                                  | Remaining milestone work                                                                    |
| --------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| MC        | Exactly one stable choice ID; v2 compiles to native radio choices                                                    | Extend the v2 visual author form                                                            |
| MA        | Unique correct choice set; public checkboxes disclose no correct IDs or correct count                                | Add visual author form and full path acceptance                                             |
| FIB       | One or more accepted strings, explicit match mode, bounded entry                                                     | Add visual author form and feedback presentation                                            |
| MULTI-FIB | Stable blank IDs, labels, accepted strings, match modes, and per-blank bounds                                        | Add inline visual authoring and complete screen-reader review                               |
| NUM       | Finite answer, exact/absolute/relative/significant-figures tolerance, optional unit                                  | Add visual author form and tolerance explanation text                                       |
| MATCH     | Explicit prompt/choice IDs and one-to-one private pairing; native radio groups per prompt                            | Add visual author form and Chapter 1 source                                                 |
| ORDER     | Stable item IDs and an exact private permutation; accessible move controls                                           | Add visual author form and full path acceptance                                             |
| HOTSPOT   | Immutable asset/checksum, normalized nonoverlapping candidate regions, private correct set, and keyboard region list | Add secure media selection, pointer overlay authoring/interaction, and object lifecycle E2E |

The family compiler currently uses all-or-nothing scoring. Partial-credit policy remains server-owned
and requires a deliberate grading contract rather than browser-supplied component scores.

## Media and hotspot rules

Media bytes remain outside JSON. PLE object storage owns bytes, sniffed media type, size, checksum,
lifecycle, delivery authorization, replacement, publication, and cleanup. Version 2 HOTSPOT source
references an existing immutable asset UUID and lowercase SHA-256 checksum.

Hotspot geometry uses integer coordinates from 0 through 10,000. Candidate rectangles must be
nonempty, contained by the normalized surface, nonoverlapping, uniquely identified, and visibly
labeled. The candidate-region list is the primary no-mouse interaction. A pointer overlay is a later
enhancement, not a replacement for the accessible path.

## Remaining dependency order

1. Accept the atomic learner render/response boundary in
   [secure_question_grading_payload_plan.md](../decisions/secure_question_grading_payload_plan.md).
2. Add family-specific instructor authoring controls without widening the closed source decoder.
3. Run one complete Memory/PostgreSQL/object-store author-to-learner path for every family, including
   correct, incorrect, retry, retention, cleanup, and tenant-refusal behavior.
4. Add the exact Chapter 1 genetics and biochemistry content and the separate WeBWorK MATCH path.
5. Complete keyboard, screen-reader, 320 px, zoom, and pointer/alternative HOTSPOT review.
6. Run the full repository and disposable integration gates, then obtain independent family and
   content reviews before accepting WP-RC5.

## Evidence required for acceptance

- version 1 canonical compatibility remains unchanged;
- every version 2 family has valid and meaningful invalid source fixtures;
- public, generated, Wasm, cache, and learner payloads contain no accepted answer or private
  feedback;
- the exact public/private checksum binding refuses substitutions;
- student responses contain only learner-provided values or rendered item IDs;
- server grading proves correct and incorrect outcomes for every family;
- Memory and PostgreSQL behavior, forced RLS, cleanup, and immutable source/object binding agree;
- every learner control works through the platform keyboard contract; and
- the two Chapter 1 assignments each publish their four reviewed questions with license and
  provenance.

## Reference map

- Internal source contract: [QTI-JSON_OBJECT_FORMAT.md](../../QTI-JSON_OBJECT_FORMAT.md).
- Runtime response types: [QUESTION_MODEL.md](../../QUESTION_MODEL.md).
- Submission boundary: [secure_question_grading_payload_plan.md](../decisions/secure_question_grading_payload_plan.md).
- Keyboard contract: `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md`.
- Durable source and answer-secrecy decisions:
  [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md#flat-question-source).
