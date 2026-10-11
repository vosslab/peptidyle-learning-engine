# Test pruning, 2026-10-11

The final suite retains 55 explicitly declared repo-owned tests, with 1,664 fewer declarations
and 231 whole test-bearing files removed. The initial exceptional-value standard was too restrictive:
a targeted reassessment restored 36 inexpensive tests for meaningful, stable behavior. Those
restorations add 857 lines without restoring dependencies or shared test infrastructure.

Maintenance burden, rather than deletion volume, is the decision criterion. The reductions below
do not prove that every removed test was worthless or that the remaining suite covers the product.

## Comparable inventory

Counts use the same declaration rules before and after: Python `test_` functions, Rust test
attributes (including Tokio and wasm-bindgen), Node/Playwright `test` or `it` declarations, and
OpenTofu `run` blocks. Parameter combinations, Playwright steps, assertions, and standalone
script checks and doctests are not counted as additional named tests. The initial browser estimate included
four suite/skip declarations; this table corrects that overcount in both snapshots.

| Repo-owned lane | Before | After | Named tests removed | Whole files containing named tests removed |
| --- | ---: | ---: | ---: | ---: |
| Python | 295 | 12 | 283 | 37 |
| Root Node, exercising TypeScript | 583 | 19 | 564 | 111 |
| Rust, including inline tests | 798 | 22 | 776 | 63 |
| Browser | 20 | 2 | 18 | 15 |
| Connected Node scripts | 21 | 0 | 21 | 4 |
| OpenTofu | 2 | 0 | 2 | 1 |
| **Total** | **1,719** | **55** | **1,664 (96.8%)** | **231** |

The remaining twelve Python functions collect 16 parameterized cases. The user-edited standalone
Wasm bridge script remains unchanged; its
top-level assertions are outside the named-test count.

Additional removal is visible at file level:

- 396 files disappeared from the initial test/Rust/deployment inventory, plus the fast-UI launcher,
  cross-store lifecycle owner, and Wasm test-runner installer: **399 whole files deleted**.
- Root Node lost 112 executable files: 111 with named tests and one top-level assertion script.
- SQL oracle files fell from 16 to 2. Shell/browser scripts without named declarations were also
  deleted, so 1,664 is not a count of every removed executable check.
- At least **61 unambiguous support files, totaling 6,333 lines**, disappeared: fixture components,
  loaders, visual CSS, helper modules, registries, adapter fixtures, the fast-UI launcher, and the
  dedicated cross-store owner, and unused Wasm test-runner installer. This conservative subset
  excludes ambiguous standalone test scripts.
- Eleven Cargo dev-dependency sections and both remaining test-support features disappeared.
  The Rust gate no longer runs the obsolete test-support feature lane.
  Unused root Tower/wasm-bindgen-test declarations and Tokio's test-util feature were also removed.
- Trivial capability-set and version-getter doctest snippets were removed too. They produced three
  additional Rust test cases because wasm-bindgen duplicated the version example.
- The database owner shrank from 668 to 359 lines. It no longer dispatches 18 deleted Rust
  PostgreSQL tests, parses their output, or provisions their private test environment.
- The snapshotted repo-owned test/Rust paths have a net reduction of 97,972 physical lines.
  This includes fixtures, runners, and test blocks inside production files; it is not a claim
  that every line was an assertion. Documentation and other runtime cleanup are outside that count.

Vendored tests are excluded. The user's propagation refresh is authoritative.
`tests/conftest.py` matches its original bytes: no new hooks, policies, fixtures, or options.
Existing user edits to the Wasm bridge script and earlier pruning report were preserved.
No staging or commit was performed.

## What survived and why

These are assertion-level reasons, not claims of historically observed regressions. No retained
test is justified merely by its language, a security label, or a speculative future feature.
The consequences below are directly exercised by the tests; broad product coverage is not implied.

### Python: twelve functions

| Module | Retained functions and concrete protection |
| --- | --- |
| `tests/test_invitation_input.py` | `test_read_export_rejects_unsafe_recipient_boundaries`: rejects inconsistent recipient domains, unsafe origins, and credentials in URLs. `test_read_export_rejects_normalized_duplicate_addresses`: prevents duplicate-recipient delivery after normalization. `test_resolve_private_export_accepts_only_private_direct_json`: protects the private invitation input boundary. |
| `tests/test_invitation_sender.py` | `test_process_batch_continues_after_one_recipient_failure`: later recipients proceed after a return-value failure or exception, with private tokens redacted. `test_interruption_leaves_indeterminate_status`: interrupted delivery remains durably uncertain. `test_dry_run_never_calls_sender`: preview cannot send mail. |
| `tests/test_invitation_status_log.py` | `test_status_log_round_trip_is_owner_private`: saved delivery evidence round-trips privately. `test_pending_recipients_holds_sent_and_indeterminate_observations`: known-sent and uncertain deliveries are not retried automatically. Restored `test_force_resend_releases_held_recipients`, `test_pending_recipients_scopes_status_to_course`, and `test_sent_log_neutralizes_spreadsheet_formulas`: an explicit override permits resend, a different Course is not suppressed, and exported formula text remains inert. These reuse the existing local helpers and standard-library CSV reader. |
| `tests/test_invitation_cli.py` | `test_force_resend_requires_send_and_one_recipient`: the override cannot silently resend a whole batch or operate in preview mode. |

### Node/TypeScript: nineteen tests

`tests/test_question_draft_autosave.mjs` keeps four asynchronous durability assertions:

- Debounced edits save and acknowledge the newest snapshot.
- Edits made during an in-flight save remain serialized and receive the later acknowledgment.
- A failed write retains the newest snapshot for explicit retry.
- Navigation flush saves pending work before disposal.

The targeted reassessment restored these small direct modules (15 tests, 311 lines). They import
production functions and Node's built-in assertions, with no shared harness, browser, or service.

| Module | Tests | Concrete protection |
| --- | ---: | --- |
| `test_decoder.mjs` | 2 | Reject prototype-control keys and inherited fields; decoded dictionaries have no prototype. Hostile values were corrected to valid numbers so rejection cannot pass merely because the value decoder rejects an object. |
| `test_sysadmin_student_access_decoder.mjs` | 3 | Accept the scoped roster/audit record; reject extra private fields, invalid states, and missing audit evidence. |
| `test_feedback_disclosure_decoder.mjs` | 1 | Reject private answer data and nested provider transcripts in Student Feedback. Deferred feedback-policy and regrading-state cases remain removed. |
| `test_assessment_attempt_response_state.mjs` | 2 | A delayed validation cannot validate another Question; rejected raw input remains unsaveable. |
| `test_base_assignment_policy_autosave_model.mjs` | 4 | Coalesce later edits; preserve a newer draft after stale completion; retry the current draft after failure; preserve a conflicted draft until explicit reload. |
| `test_assessment_duration.mjs` | 2 | Apply the required 1.5-minute formula with whole-minute rounding; serialize valid minute overrides to seconds and reject malformed values. Exact UI wording remains removed. |
| `test_student_course_response_stats.mjs` | 1 | Reject private fields, impossible aggregate counts, and inconsistent response rankings. |

### Rust: twenty-two tests

- `multiple_answer_uses_choice_count_formula_through_private_grading` checks a mixed correct/wrong
  response's 0.6 score through the private grader and rejects duplicate selections.
- `matching_partial_and_empty_responses_use_all_authored_prompts_as_denominator` checks that partial
  matching credit uses all authored prompts, with zero credit for an empty response.
- `ordering_uses_stable_ids_and_preserves_complete_permutation_validation` checks order credit by
  stable IDs and rejects a duplicated item instead of scoring a malformed permutation.
- `withheld_question_answer_is_absent_from_student_feedback` checks both the domain value and
  serialized Student payload: the answer and explanation are absent. Positive feedback assertions
  were removed because that policy is still deferred.

The reassessment added 18 direct unit tests beside their owning code:

| Owning module | Restored tests | Concrete protection |
| --- | ---: | --- |
| `grading::ple_question_json` | 3 | FIB alternatives and literal matching modes; authored regex anchors/case and blank rejection; MULTI-FIB credit over all authored blanks. Reuses the existing grader helper. |
| `question_model::student_work::grading` | 1 | Current points recalculate from immutable recorded credit without altering that credit; this is distinct from deferred response regrading. |
| `domain::validation::authored_response` | 1 | Nonfinite numeric input is rejected. A separate ordering test remains removed because the existing grader already exercises the same duplicate-order validator. |
| `question_model::response` | 1 | Student response JSON rejects injected score, correctness, and provider fields. |
| `domain::teaching_authority` | 2 | Instructor authority requires the exact active Account/Course membership; foreign Course, foreign Account, and revoked memberships fail. Student access requires the exact owned record. |
| `question_model::course_term` | 2 | Reject malformed/impossible dates and reversed bounds; accept inclusive term endpoints. |
| `learning_data_access::authentication_email` | 3 | Normalize lookup while preserving delivery spelling, reject ambiguous mailboxes, and require an institutional Student domain. |
| `learning_data_access::authentication_ceremony` | 1 | Ordinary Student/Instructor login admits only its supported passwordless methods and refuses Sysadmin issuance. |
| `server_core::auth` | 4 | First-party cookie attributes, sensitive-cookie alias rejection, one exact Origin, and canonical browser-origin parsing. No fake session store, Router harness, or Tower dev dependency was restored. |

All Rust restorations use existing production dependencies and small local literals/helpers.
No test-support feature, shared configuration, or test-only dependency was restored.

### Browser: two real-stack journeys

- `auth_authorization.spec.ts` verifies signed-out access denial, seeded Instructor/Student entry,
  Student sign-out/reentry, no Sysadmin session cookie before second-factor verification, and
  Student denial at the Question Library route. It does not prove all server authorization paths.
- `sysadmin_student_data_access.spec.ts` verifies that unchecked and cancelled disclosure make no
  protected request, while explicit confirmation shows an audit receipt without the Student's name.
  Its observation is the real HTTPS UI and request boundary.

Fixture metadata, exact request-shape, and markup-count assertions were removed. The screenshot
publisher and its 28 source files remain operational tooling;
screenshot production is not counted as behavioral test coverage.

### Connected SQL and Wasm

- `database_baseline_security_catalog.sql` checks database grants, forced row-level security,
  separation of application/worker/migrator authority, protected authentication material, and
  scoped Student-data disclosure authority. Retired-symbol, trigger-name, and exact role/topology
  inventory checks were removed. Catalog assertions establish installed authority, not every query.
- `unrelease_connected_oracle.sql` and `e2e_unrelease_connected.sh` exercise deletion of Student
  Work with surviving unrelated data, followed by two real database sessions contending on the
  Assessment row. A Student start after Unrelease must fail and leave no Attempt. This destructive
  transition earns a real database check; it creates its own fixture after schema initialization.
- The database owner still checks rejection and rollback on unrelated schema contents, successful
  initialization/replay, and application-role verification. These determine whether installation
  can modify an unsuitable database or leave a partial installation.
- The pre-existing user-edited `e2e_wasm_bridge.mjs` remains intact. It includes a native presentation
  vector and mutation rejection across the Rust/Wasm boundary, plus existing validation/timing,
  capability, preview, and version assertions. Preservation of that active user edit is an explicit
  scope exception, not an assertion that every check in the script passed the retention standard.

## Removed protection and limits

The initial retention challenge removed lease/reset modules, specialized policy autosave checks,
and CSV formula coverage. The reassessment restored the inexpensive autosave and CSV behavior
checks because their concrete protection outweighs their small cost. Lease/reset machinery,
mock-store suites, and broad policy/shape matrices remain removed. This targeted reassessment
is not an assertion-level review of every deleted test.

Whole groups were removed before attempting individual salvage: source/SQL shape gates, decoder
and mock-client matrices, broad backend conformance suites, isolated UI fixture journeys,
pixel/layout/SSR checks, milestone evidence, and mock deployment-plan gates. Some removed tests
exercised valid behavior. Their breadth, overlap, and maintenance cost did not earn permanent
retention under the requested aggressive policy.

The remaining suite does **not** establish full Assessment lifecycle correctness, all feedback
policies, import-format behavior, keyboard/touch accessibility, all authorization cases, Blueprint
fork/publication behavior, Course Appearance cross-store durability, or deployment correctness.
Those are known coverage losses, not newly discovered product defects.

The expiry SQL oracle depended on fixtures created by a deleted Rust assessment-access test.
It and its dispatch were deleted, rather than creating replacement setup machinery. There is no
retained connected proof of expiry grading or direct finalization. The old Star-name privacy
script used obsolete identity/session fixtures; it was removed without asserting current HTTP
privacy coverage. The Wasm export-name allowlist was removed because names alone did not establish
that answers were absent from browser artifacts.

Initial mechanical Rust pruning accidentally cut production code after a test-only helper in
`acceptance-runtime`. The original production tail was restored before validation. A fresh,
syntax-aware audit then compared production declarations independently of compilation: 202 changed
extant Rust source files preserved production tokens after excluding test declarations and
formatting, apart from the intentional removal of the dedicated cross-store runtime profile.
Wholly deleted Rust sources were test or test-support modules. This is structural evidence, not
runtime proof of every production behavior.

## Verification

- `devel/dist_clean.sh` removed build outputs, generated files, Node dependencies, Cargo artifacts,
  and pytest cache after stopping the disposable Live Demo. Dependency installation and the full
  host build passed; the cold build took 63 seconds.
- `source source_me.sh && ./launchers/all_test.sh` passed after all restorations: schema generation
  and style, default/all-feature Rust checks/tests, strict Clippy, Wasm target compilation,
  TypeScript/lint/formatting, 19 Node tests, and 9,067 pytest cases (16 repo-owned invitation cases).
- The same aggregate passed disposable PostgreSQL initialization/replay, selected privilege/RLS
  boundaries, and Unrelease concurrency. The earlier exclusive-lease blockage is resolved.
- Standalone Wasm bridge assertions passed against the fresh debug build. The earlier pruning
  pass also checked a release build; the clean pass does not relabel that earlier release evidence.
- Focused `question_model` tests and strict Clippy also passed after moving the restored
  unknown-field assertion from the deferred iMathAS variant to native numeric inputs.
- A syntax-aware token comparison confirmed unchanged production code in all nine Rust files
  touched by restoration, independently of the passing compiler and tests.
- A fresh disposable Live Demo initialized and passed both retained production-browser scenarios:
  Student authorization and confirmed, audited Sysadmin access to Student records.
- Screenshot regeneration reused that freshly verified stack and published all 257 captures:
  3 public, 76 Instructor, 172 Student, and 6 Sysadmin images. The manifest, receipt, atlas, and
  galleries passed static corpus verification. An independent visual sample of 18 fresh images
  across roles and viewport sizes found no blocking render failures; it was not an exhaustive
  visual review of every capture.
- The disposable Live Demo was stopped after capture and its owned containers were removed.
- Final `python3 -m pytest tests/ -q` passed all 9,067 cases after screenshot publication. An extra
  invocation that omitted `tests/` failed collection because it traversed `OTHER_REPOS`; correcting
  the command to the repository's documented scope required no code or configuration change.

No replacement testing framework, skipped-test archive, generic test policy, or conftest mechanism
was introduced. One-time inventory/recovery checks were removed from `tests/_temp/` at closeout.
