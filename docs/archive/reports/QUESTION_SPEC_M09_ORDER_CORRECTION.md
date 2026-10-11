# M09 Question Pool Order Correction

## Scope and handoff

This is a fresh M09 correction to the earlier Pool-storage implementation. The
source handoff inspection found that `QuestionPoolSelectedQuestionOrder` and
`selected_question_order` had been retained and redefined as canonical Question
ID display order. That field and the `QuestionPoolOrder`/`RandomOrder` selector
are unsupported product behavior. HG1295-96 describes spreadsheet-style Pool
row sorting as display-only: it leaves the saved tuple set and random selection
unchanged.

The coordinator-approved boundaries and ownership split are recorded first in
[`M09_ORDER_BOUNDARY.md`](output_question_spec/M09_ORDER_BOUNDARY.md).
This correction removes the obsolete order policy across SQL, Rust,
API/TypeScript, Blueprint mapping, history, and fixtures. Pool-row display
sorting is implemented on the existing read-only Pool detail and on the
ordinary Owner/Sysadmin Pool draft editor. Both use local state with the shared
record/table interface. In the editor, displayed rows are a sorted copy;
Add/Remove and Save continue to operate on the exact tuple draft set. M09 owns
this ordinary editor. M10/M11 direct Pool-reference and forced-copy work remains
with its current owners.
This work preserves ordered Assessment entries, Attempt-local
`issued_position`/`selection_position`, random sampling without replacement,
exact Revision Tuples, and the existing permission, Bloom, and source-Pool
contracts. Canonical tuple ordering remains only an internal step to make a
random sample independent of candidate input order.

The first fresh SPEC review caught remaining external assumptions that Pool
members arrive in Question-ID order. This correction removes those assumptions
from Pool API reads, Blueprint membership reads, Assessment snapshot decoding,
and the TypeScript detail decoder. Pool member responses and input decoding now
accept an unordered set; Assessment entries still group by their authored
position. The selector alone canonicalizes candidate tuples for reproducible
sampling.

The shared-Pool editor must save only Add/Remove changes and the selected exact
Revision Tuple set. Its integrated display sort must not dirty the Pool, change
the tuple set, or advance the Pool Edit Number. No new required selection is
introduced.
M10/M11 direct Pool-reference and forced-copy work, M05 nullable
metadata decoders, M07 Question parents/authoring, and M24 search renames remain
with their current owners; this correction edits only order-related lines in
shared files. Generated TypeScript is root-coordinator-owned. Once SQL/Rust
source ownership was clear, the coordinator regenerated the contracts:
`cargo tsgen` now reports 409 generated types, down from the prior 416.

The ordinary shared Pool detail is the Pool member editor for the Pool Owner
and Sysadmin. Assessments retain direct IDs for ordinary Pools and keep their
own selection counts.

## Implementation evidence

The SQL lane removed the obsolete enum, column, SQL arguments and validation,
projections, mappings, and fixture values. The Rust lane removed the Pool order
domain/API field and parameter; Assessment entries and Attempt-local positions
remain ordered. The browser lane removed old fields, choices, serialization,
decoding, mapping, history, and fixture entries. It retained three explicit
negative assertions proving legacy payloads/options are absent. A source search
across `schemas`, `crates`, `src`, `generated`, `content`, and `tests` found
only those two browser negative-payload fixtures and the Rust serialization
absence assertion; it found no production policy reference.

The selector canonicalizes candidate tuples internally and samples without
replacement. Its tests cover input-permutation independence and an
entropy-driven change in the selected draw. The read-only detail table and
editable Pool draft table each sort only display copies. Draft membership
replacement, Save serialization, and dirty-state checks do not consume the
selected display order.

Focused checks and current limits:

- Seven focused Node suites pass 40/40. The selector tests pass 3/3,
  including candidate-input permutation independence and an entropy-driven
  random draw. All 165 `question_model` tests pass.
- The bounded member-replacement test passes 1/1. The PostgreSQL-feature
  `learning-data-access` check passes. `cargo check -p server_core --lib` passes
  with three warnings: unused `now`, dead `is_ple_question_json_request`, and
  dead `instructor`. Inline authoring tests were split into
  `crates/server/src/authoring_tests.rs`; `authoring.rs` is 879 lines and the
  test module is 150 lines, within the source limit. The focused
  `authoring::tests` command passes 4/4. The broader `authoring` filter ran 17
  tests, with 16 passing and one unrelated publication hotspot test failing on
  stale fixture JSON `language`.
- `npx tsc --noEmit -p tsconfig.json` passes. Schema style, scoped rustfmt,
  Prettier, and `git diff --check` pass.
- Both controlled local browser checks pass. The editable harness confirms
  display sorting preserves the exact tuple-set Save payload and Edit Number 7;
  the stubbed Save receipt advances the acknowledged Edit Number to 8.
- Add is disabled during reload and `addSelection` guards reload. Remove remains
  available during reload except while saving or adding, and when the draft has
  only its last member. If Remove changes the draft during reload, the draft
  identity guard discards the stale reload and preserves the exact tuple set
  and Edit Number. Pending-Add/reload orders are exercised only in synthetic
  model tests; current UI controls cannot reach those orders.
- Connected database/API/production-browser acceptance, M05/M06 dependency
  acceptance, and repository-wide test suites remain pending. The M09 editor
  is the ordinary Pool editor. M11 preserves direct ordinary Pool references
  and explicit fork paths; no Blueprint-specific member editor is part of M09.
  The latest fresh SPEC pass is `m09_spec_sql_hunk_recheck` (2026-10-06). It
  confirmed the ordinary editor contract, exact SQL hunk ownership note, and
  current store -> `ple_api` wrapper -> protected `ple_data` function call
  path. The older QUALITY rejection concerned docs and the source line-limit
  violation; the test split and evidence corrections are applied, and a fresh
  separate QUALITY re-review remains pending. The direct `ple_data` grant
  versus current `ple_api` wrapper clarification has been asked and remains
  unanswered.

Both read-only and editable display-sort browser proofs passed. The SQL callable
wrapper, exact `ple_app` wrapper grant, catalog/app-call oracle assertion, active
Instructor-or-Sysadmin check, and post-lock Pool-owner check are present in source.
Connected PostgreSQL Save and API/production-browser acceptance remain pending;
source checks do not establish connected role or database behavior. The ordinary UI remains
Owner/Sysadmin-gated, with no new authorization path in the order correction.

## Readiness marker

**2026-10-06 - source correction and focused editor proof implemented; fresh
SPEC passed as `m09_spec_sql_hunk_recheck`; a separate fresh QUALITY re-review
remains pending.** The connected database Save gate remains
open under the dependencies above. No full database run was attempted. The final source review is
recorded in [M09_ORDER_FINAL_REVIEW.md](output_question_spec/M09_ORDER_FINAL_REVIEW.md); the
coordinator log remains transient local output.

## Acceptance limits

Required focused proof: canonical candidate-input permutation independence and
an entropy-driven random draw; absence of the obsolete user option; and both
read-only and editable browser sorting that preserve the displayed exact tuple
set and Edit Number. Editable Save payload stability has a stubbed browser proof.
Model, Node, schema-style, and format checks are in scope. A full database run
is out of scope; connected PostgreSQL Save acceptance remains pending.
