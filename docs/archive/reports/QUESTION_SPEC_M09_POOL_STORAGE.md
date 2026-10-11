# M09 Question Pool Storage Report

## Result

Ordinary Question Pools now store an unordered set of exact Published Question Revision Tuples.
The Pool member primary key permits one Revision per Published Question, while the Revision foreign
key preserves the exact pin. Creation and save no longer accept interchangeability attestation.
Save compares the old and requested tuple sets without regard to input order; identical sets retain
their Edit Number, and a changed set advances it once.

Question Type validation reads the exact `question_revision_metadata` tuple, following M06's
storage contract. The member-set projection represents the unordered set. The ordinary Pool editor
and detail table provide spreadsheet-style display sorting through the shared record/table
interface; changing either local sort leaves the member set, random selection, and Pool Edit Number
unchanged. Attempt selection rows retain
`selection_position`, while pooled issue identity uses the saved selection's
Pool ID, Edit Number, and exact Published Question Revision Tuple. Existing Attempts continue to
use saved selections instead of rereading current Pool membership.

Candidate tuples are canonicalized internally before entropy-based sampling so caller or storage
order cannot affect the draw. This is an implementation detail, not display or Assessment order.
The initial source handoff incorrectly retained `selected_question_order` and redefined
`QuestionPoolOrder` as canonical Question ID display order. HG1295-96 supports a spreadsheet-style
Pool editor sort whose state affects only displayed rows. The obsolete field, enum, parameter,
validation, serializer, decoder, and user option are removed by the correction tracked in
[`QUESTION_SPEC_M09_ORDER_CORRECTION.md`](QUESTION_SPEC_M09_ORDER_CORRECTION.md). The existing
Assessment entry order and Attempt-local positions remain.

Removed persistent Pool member position fields and the creation/edit attestation control, request,
and storage. The earlier member editor removed manual reordering and displayed a deterministic
Question ID sort; the order correction replaces that fixed display policy with spreadsheet-style
display sorting. Human Guidance states that Pools contain Published Questions only, cannot nest,
and hold unordered exact tuples unique by Question ID.

## Verification

Current M09 member-editor evidence: focused Node tests passed 13/13, `npx tsc --noEmit -p
tsconfig.json` passed, `question_model question_pool_library` passed 2 tests, the PostgreSQL
`learning-data-access` check passed, and the bounded member-replacement test passed 1 test. The
local controlled/stubbed editor browser harness, scoped rustfmt, and `git diff --check` passed.
`source ./source_me.sh && cargo check -p server_core --lib` passes with three warnings: unused
`now`, dead `is_ple_question_json_request`, and dead `instructor`. Inline authoring tests were split
into `crates/server/src/authoring_tests.rs` to keep `authoring.rs` under 1,000 lines (879 lines;
test module 150 lines). `cargo test -p server_core --lib authoring::tests` passes 4/4. The broader
`authoring` filter ran 17 tests: 16 passed and one unrelated publication hotspot test failed on stale
fixture JSON `language`. Connected PostgreSQL/API/production-browser
acceptance and M05/M06 remain pending; no tsgen run was performed for this editor task.

The controlled browser harness exercises Remove during reload: Add is disabled and guarded
while reload is pending; Remove remains available except while saving/adding or at the last member.
When Remove changes the draft, its identity guard discards the stale reload and preserves the
exact tuple set and Edit Number. Pending-Add/reload order tests are synthetic model coverage and
are not reachable through current UI controls. Fresh SPEC review
`m09_spec_sql_hunk_recheck` (2026-10-06) passed, confirming the ordinary editor
contract, exact SQL hunk ownership note, and current store -> `ple_api` wrapper
-> protected `ple_data` function call path. The separate fresh QUALITY
re-review remains pending. The direct `ple_data` grant versus current wrapper
clarification has been asked and remains unanswered. Connected DB/API/production-browser,
M05/M06, and full milestone acceptance remain pending.

## Acceptance limits

No connected PostgreSQL Pool save, Attempt resume, or production browser journey was run for this
milestone. Isolated Playwright checks cover read-only Pool-detail sorting and editable draft sorting;
the editable check verifies the exact Save payload and unchanged acknowledged Edit Number before
Save. Source and focused checks do not establish database runtime acceptance. Generated TypeScript
contracts were refreshed in earlier root-coordinated work; no tsgen was run for this editor task.
The `ple_app` callable grant and SQL Owner/Sysadmin predicates are now present in source, but connected role and Save acceptance remain
pending. Dependency acceptance for M05 and M06 and later M10/M11 work remain separate gates.
