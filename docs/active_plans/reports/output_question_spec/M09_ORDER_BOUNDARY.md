# M09 Pool Order Correction Handoff

Status: coordinator-approved scope, 2026-10-06. This note is the first inspection
point for M09 order work. The full evidence report is
[`QUESTION_SPEC_M09_ORDER_CORRECTION.md`](../QUESTION_SPEC_M09_ORDER_CORRECTION.md).

## Product contract

- HG1295-96 says Instructors can sort Pool Questions like a spreadsheet. Sorting
  changes the displayed rows only; it does not change Pool contents or random
  selection.
- An Assessment has its own ordered entries. Remove the unsupported
  `QuestionPoolSelectedQuestionOrder`, `selected_question_order`,
  `selectedQuestionOrder`, `QuestionPoolOrder`, and `questionPoolOrder` policy
  from Pool selection and its persistence, APIs, serializers, decoders, and
  fixtures.
- Keep `issued_position` and Attempt-local `selection_position`. Keep random
  sampling without replacement. Canonical tuple ordering may make sampling
  independent of candidate input order; it is an internal deterministic step,
  not a Pool display order or Assessment choice.
- Treat Pool members as an unordered exact tuple set throughout storage,
  projections, API decoding, and Blueprint lineage mapping. No Pool reader may
  require or imply Question-ID member order. Keep Assessment entries ordered by
  their own authored position.
- Pool member editing must offer spreadsheet-style display sorting through an
  existing record/table interface. A sort toggle must not change selected
  Revision tuples, dirty the Pool editor, or advance the Pool Edit Number. The
  editor changes membership only through Add/Remove of exact Revision tuples;
  Save transmits the complete unordered selected tuple set. The editor adds no
  required choice.
- Preserve Owner/Sysadmin permissions, nullable Bloom metadata, source-Pool
  ancestry, and all direct Pool reference/forced-copy behavior owned by M10/M11.

## Work lanes and shared-file boundaries

- SQL lane: remove only obsolete Assessment Pool order type/column, parameters,
  validation, row mappings, and SQL fixture values. Preserve M10/M11 Pool
  reference and forced-copy edits in shared functions. Include SQL projections
  such as `pool_selection_rule` when they carry only the obsolete order field,
  then coordinate their Rust consumer result mappings. If removal requires a
  whole-function rewrite, stop that file and report the dependency.
- Rust lane: remove only order enum/field/parameter and associated model,
  conversion, and fixture code. Keep ordered Assessment entries, Attempt-local
  positions, and order-independent random selection.
- Browser lane: remove only obsolete order contracts/options/serialization and
  fixture data. M09 owns the ordinary Owner/Sysadmin Pool Add/Remove/Save editor
  on `question_pool_detail.tsx`. The detail table and editable draft table both
  use local display sorting through the existing record/table interface; the
  editor sorts a display copy and saves the unchanged exact tuple draft. The
  Assessment entry editor is in the M10/M11 direct-reference transition, and
  `blueprint_pool_members_editor.tsx` edits a Blueprint Assessment-owned Pool.
  Do not place Pool sorting in an Assessment editor. Do not edit generated
  TypeScript contracts.
- Root coordinator owns regeneration of `generated/` after SQL/Rust source
  ownership is clear. That regeneration has completed: `cargo tsgen` wrote 409
  generated types, down from the prior 416, with the old order field removed.
- M05 owns nullable metadata decoders, M07 owns Question parents/authoring, M24
  owns search renames, and M10/M11 own direct Pool references and forced-copy
  removal in shared files. Keep M09 edits on order-related lines only; preserve
  all their changes and raise any unavoidable whole-function overlap.

## Acceptance proof

Focused evidence must show canonical candidate-input permutation independence
and a real entropy-driven random draw; no obsolete user option remains; and
read-only and editable Pool browser sorts leave their exact displayed tuple set
and Edit Number unchanged. The editable Save request must keep the same exact
tuple set. Run model, Node, schema-style, and format checks only; no full
database run is part of this correction. Record connected browser/database proof
as pending if it is not available.

## Current readiness marker

2026-10-06: fresh SPEC reviews found and routed external member-order
assumptions in Pool projections, the editable draft model, and Save decoding.
Those assumptions are removed; model, decoder, and rendered editable browser
proof show that display sorting leaves the exact draft tuple set, Save payload,
and acknowledged Edit Number unchanged. Fresh SPEC confirmed the product,
implementation, and evidence documentation; the separate fresh QUALITY review
is pending. Connected PostgreSQL Save, API, and production
browser acceptance remain pending the shared M09 callable-role and permission
gate. Exact results and limitations are in the linked correction report and
`m09_order_correction_cli.log`.
