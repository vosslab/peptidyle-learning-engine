# QTI-JSONL flat-question companion

> **Authority:** This is a non-authoritative companion to
> [release_completion_plan.md](release_completion_plan.md). That plan owns package names, sequence,
> scope, acceptance, and release status. If the two documents differ, the release-completion plan
> controls. This companion preserves the detailed QTI-JSONL family contract for WP-RC4 and WP-RC5;
> it does not create a parallel `WP-FQ-*` work-package sequence.

## Status

The current immediate package is WP-RC3, shipped WeBWorK integration. After WP-RC3 acceptance,
WP-ARCH1 completes the source-module decomposition before WP-RC4 begins. WP-RC4 freezes and adopts
the owner-authored QTI-JSONL v1 contract; WP-RC5 then implements all flat families and Chapter 1
content in the release plan's stated order.

Before the first WP-RC5 family, accept the atomic learner render/response boundary in
[secure_question_grading_payload_plan.md](../decisions/secure_question_grading_payload_plan.md).
It makes an authenticated `QuestionAttemptId` and `Idempotency-Key` the only submission authority.
It is a prerequisite for family work, not a delay to current WP-RC3 live acceptance.

## Companion purpose

This document records the exact family and source-boundary details that WP-RC4 and WP-RC5 must
preserve. It does not authorize implementation before the release plan's prerequisites and does not
replace the authoritative files, owners, or validation gates there.

## Family contract

QTI Package Maker QTI-JSONL v1 is the only new flat-source vocabulary. Version 1 covers:

- multiple choice (MC);
- multiple answer (MA);
- fill-in-the-blank (FIB);
- multiple fill-in-the-blank (MULTI-FIB);
- numerical entry (NUM);
- matching (MATCH);
- ordered list (ORDER); and
- image hot spot (HOTSPOT).

The source contract includes optional overall correct/incorrect feedback and typed media references.
MC remains compatible with existing PLE flat `singleChoice` v1; its accepted bytes, parser
behavior, family ID, persistence, routes, editor, QTI profile bridge, and grading remain unchanged.
No background rewrite or in-place reinterpretation is required.

WP-RC5 implements MATCH first, then MA, FIB, NUM, ORDER, MULTI-FIB, media, and HOTSPOT. Each family
uses strict parsing, author edit and answer-free preview, compare-and-swap save, immutable
publication, accessible learner interaction, server grading, optional feedback, summary, retention,
and cleanup. A family advances only after its source fixture, protected answer key, complete
Memory/PostgreSQL author-to-learner path, and correct/incorrect behavior pass.

## Source and runtime boundary

The QTI-JSONL source remains authoring- and conversion-friendly. PLE runtime types remain optimized
for safe rendering, response validation, and grading.

```text
private QTI-JSONL source record
              |
              v
  versioned QTI-JSONL adapter
              |
              v
     native compiler boundary
          /             \
         v               v
answer-free public     grader-only key,
question model         feedback, and binding
```

- Only the adapter owns QTI-JSONL field names and source-version compatibility.
- Store, SQL, HTTP, generated clients, Solid components, and grading consume PLE types rather than
  parsing QTI-JSONL independently.
- The authored source remains private and answer-bearing. The public projection remains answer-free;
  private keys, feedback, and bindings stay behind the injected grader capability.
- Publication preserves the exact accepted source bytes, or the exact canonical form required by the
  owner specification, and binds the source and both compiled halves by checksum.
- A named adapter handles every later incompatible source version. Existing source is never silently
  reinterpreted, and unknown fields or versions are refused rather than guessed.
- QTI-JSONL line framing stays at the adapter/import/export edge. A bounded bank operation persists
  each accepted record through its own workspace/publication transaction; JSONL framing never enters
  the question model, grading API, or learner response format.

PLE response and grading contracts use opaque stable identifiers so display reordering cannot change
answer meaning. The adapter preserves a valid portable identifier when the accepted specification
supplies one; otherwise it derives deterministic PLE identifiers during compilation and freezes them
in the published projection.

## RC4 contract ownership

WP-RC4 owns contract freeze and strict adoption, with the product owner in QTI Package Maker owning
the external source specification and reference artifacts. The authoritative release plan names the
full owners, files, success conditions, and validation gates.

The accepted external contract must provide a named specification version and family; lossless answer
semantics; optional overall feedback; media references; HOTSPOT geometry and accessible fallback;
strict validation; and deterministic JSONL framing. It must include one valid reference record for
each family plus meaningful invalid boundaries. PLE accepts only the named version, rejects duplicate
members, unknown versions, and invalid family data, preserves exact source, and compiles answer-free
public and grader-only private values without redefining owner fields.

Within PLE, the RC4 adapter ownership boundary is:

- `crates/adapters/native/src/qti_jsonl/` for `mod.rs`, `schema.rs`, `parser.rs`, `compiler.rs`, and
  `media.rs`;
- `crates/adapters/native/src/qti_jsonl/families/` for responsibility-named family modules; and
- `crates/adapters/native/tests/qti_jsonl/` for normative valid, invalid, and historical sources.

These are complete parser/compiler owners, not empty scaffolding. Other layers receive PLE runtime
types. QTI XML, Canvas QTI, Blackboard QTI, arbitrary response-processing expressions, vendor
extension bags, executable markup, and plugin-defined graders are not the PLE domain model.

## RC5 family details

RC5 reuses existing PLE primitives when their semantics match the accepted QTI-JSONL specification.
A material mismatch changes the shared type deliberately and validates every consumer; it never adds
an adapter-only grading shortcut.

| Family    | Contract detail                                                                                                                                            |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MATCH     | Preserve accepted prompt/choice pairing, render and grade through the protected server boundary, and use one real reference fixture in the Chapter 1 path. |
| MA        | Preserve selection bounds and server-only accepted-answer association.                                                                                     |
| FIB       | Preserve accepted answer forms without leaking them into the learner payload.                                                                              |
| NUM       | Preserve numeric value, tolerance, and tolerance message semantics.                                                                                        |
| ORDER     | Preserve ordering meaning independently of display presentation.                                                                                           |
| MULTI-FIB | Map accepted blank markers and answer map exactly; refuse missing, duplicate, unknown, or malformed blanks.                                                |
| Media     | Resolve accepted references to typed PLE assets; preserve source and assets separately for provenance.                                                     |
| HOTSPOT   | Use accepted normalized geometry, scale-independent display, and a keyboard/list alternative; refuse unsupported geometry or interaction.                  |

Media bytes remain outside JSON. PLE object storage owns bytes, sniffed media type, size, checksum,
lifecycle, delivery authorization, replacement, publication, and cleanup. The adapter-owned binding
step resolves accepted source references to typed `AssetRef` values; it does not create a second media
vocabulary in the question model. Missing, unsafe, mismatched, or inaccessible assets fail with
actionable author recovery.

## Evidence and acceptance

RC4 and RC5 use the release plan's gates. This companion requires the resulting evidence to show:

- lossless QTI Package Maker contract round trips for all eight valid records and refusal at invalid
  boundaries;
- deterministic PLE compilation of the accepted records while existing v1 `singleChoice` bytes stay
  unchanged;
- no answer-bearing browser, Wasm, generated-contract, or learner payload output;
- Memory/PostgreSQL parity, forced RLS, tenant refusal, cleanup, and source-to-public/private checksum
  binding for each completed family;
- keyboard and screen-reader operation, including the HOTSPOT accessible alternative; and
- disposable PostgreSQL/RLS and object-store evidence, built-browser Playwright acceptance, and
  independent contract and family reviews before a milestone is accepted.

Chapter 1 acceptance remains the release plan's exact outcome: genetics and biochemistry each publish
four reviewed questions, WeBWorK MC, WeBWorK MATCH, flat MC, and flat MATCH, with immutable source,
license/provenance, correct/incorrect grading, and no answer-bearing learner payload.

## Reference map

- Authority, package sequence, owners, and release gates:
  [release_completion_plan.md](release_completion_plan.md).
- Required secure submission boundary:
  [secure_question_grading_payload_plan.md](../decisions/secure_question_grading_payload_plan.md).
- Durable source and answer-secrecy decisions:
  [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md#flat-question-source).
