# Instructor checkpoint security and HCI review

## Scope and verdict

**CHANGES REQUIRED.** This source-only review examined the new instructor setup
checkpoint writer, the walkthrough runner's failure receipt reader and cleanup,
the keyboard-only J11/J12/J13 child, its live-input boundary, and their focused
offline tests. It did not start Podman, run a live browser journey, inspect any
runtime report, or modify implementation.

The checkpoint vocabulary is closed, the browser writes each value only after
the matching visible assertion, and the runner redacts child stdout/stderr.
However, the file-replacement guarantee requested for this boundary is not
implemented or tested. A same-mode regular replacement after the atomic rename
can pass the TypeScript writer's final check, and the Python reader has no
recorded expected checkpoint-file identity. This does not meet the required
before/after parent-and-file replacement closure, so it is not ready for a
live rerun.

## Confirmed boundary

- `INSTRUCTOR_SETUP_CHECKPOINTS` contains exactly seven fixed ASCII values:
  `signed_in`, `course_created`, `course_opened`, `student_active`,
  `assignment_editor_opened`, `catalog_result_selected`, and
  `assignment_created`.
- Each `writeInstructorSetupCheckpoint` call follows a successful visible
  Playwright assertion for the corresponding sign-in, course, roster, editor,
  catalog-policy, or assignment-link outcome. The child uses the visible
  keyboard path and does not write IDs, titles, aliases, credentials, selectors,
  or child output to the checkpoint.
- The runner creates the checkpoint as a sibling in its mode-0700 temporary
  state directory with mode 0600. It supplies the instructor checkpoint path
  only to the instructor child, then removes that environment variable before
  launching J1 and later student children.
- On a `playwright_instructor_setup` failure, the report emits only
  `instructorCheckpoint` with one closed value or `unavailable`; child stdout
  and stderr are discarded. Parent replacement fails closed and cleanup refuses
  to delete a replacement directory.
- The Python reader uses descriptor-relative `O_NOFOLLOW` opens, validates the
  bound parent device/inode, regular-file type, exact modes, 64-byte bound,
  ASCII, one trailing LF, no CR, and the closed vocabulary. Existing tests cover
  unknown, non-ASCII, CR, multiline, oversize, mode, symlink, parent replacement,
  cleanup, and report redaction.

## Findings

### P1 - Instructor checkpoint file replacement is accepted after atomic rename

- **Locations:** `tests/playwright/simulator/instructor_setup_checkpoint.ts:98-115`,
  `tests/walkthrough/walklib/runner.py:563-612`.
- **Evidence:** The TypeScript writer records no device/inode for the temporary
  file before `renameSync`. After its test seam runs, it checks only that the
  named checkpoint is a non-symlink regular mode-0600 file. A same-parent,
  same-mode replacement therefore satisfies the final writer check. The Python
  reader binds the parent device/inode, but has no expected checkpoint-file
  device/inode and likewise accepts any same-parent regular mode-0600 file
  containing a closed value.
- **Impact:** A same-user process able to replace the named file can cause the
  failure receipt to report a forged but vocabulary-valid stage. This is exactly
  the hostile file-replacement case that the checkpoint is intended to close.
- **Required repair:** Preserve the temporary file's `fstat` device/inode before
  rename and require the named post-rename file to match it after fsync and after
  the test seam. Bind that resulting identity in the runner when the instructor
  child returns, and require the reader's descriptor and named path to match the
  expected file identity before consuming bytes. Keep the descriptor-relative
  no-follow open and parent identity checks.

### P1 - No hostile instructor file-replacement test proves the required closure

- **Locations:** `tests/playwright/simulator/instructor_setup_checkpoint.spec.ts:38-74`,
  `tests/test_ui_walkthrough_runner.py:583-642`.
- **Evidence:** The TypeScript suite tests invalid values, unsafe mode, symlink,
  and parent replacement, but not a replacement of the checkpoint file after
  rename. The Python suite tests parent replacement for instructor checkpoints;
  its same-mode file-replacement test covers only the separate J1 checkpoint.
- **Impact:** The omitted attack is not guarded by a permanent focused test, so
  a future repair cannot demonstrate the file-identity contract or prevent its
  regression.
- **Required repair:** Add deterministic hostile tests that replace the
  instructor checkpoint with a regular mode-0600 closed-value file after rename
  and between the reader's parent/file operations. Require an unsafe writer
  result or an `unavailable` receipt, preserve the replacement during cleanup,
  and assert that no replacement content enters the report.

## Focused non-live validation

| Check | Result |
| --- | --- |
| `source source_me.sh && python3 -m pytest -q tests/test_ui_walkthrough_runner.py tests/test_ui_walkthrough_harness_independence.py` | PASS: 49 tests, 9 subtests |
| `npx playwright test tests/playwright/simulator/instructor_setup_checkpoint.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts --workers=1` | PASS: 9 tests |
| `npx tsc --noEmit -p tsconfig.lint.json` | PASS |

These passing gates do not cover the P1 file-replacement attack above.

## Rerun decision

**NOT ACCEPTED TO RERUN.** Repair both P1 findings, rerun the focused non-live
gates, and obtain a fresh security review before invoking the real Podman and
Playwright walkthrough.

## Re-review - bind-after-child revision

**STILL NOT ACCEPTED TO RERUN.** The revision correctly strengthens the local
writer: after rename it holds a no-follow descriptor and compares the final
named file device/inode to that committed descriptor. Its new hostile test
proves that a swap during the writer's own post-rename check fails closed. The
runner also rejects a swap made after its explicit bind step.

That does not close the requested cross-process boundary. The runner creates
and records the empty checkpoint inode in `prepare_journey_state`, but the
atomic TypeScript rename intentionally replaces that inode. On child failure,
`run_required` calls `bind_instructor_setup_checkpoint_identity` only after the
external Playwright command has returned. That function records the device/inode
of whichever regular mode-0600 file is named then, replacing the creation-time
identity. A same-user actor can therefore replace the completed checkpoint
after the child exits but before this bind; the replacement becomes the trusted
identity and a later failure receipt accepts its closed vocabulary value.

The new Python test replaces only after calling the bind method, so it proves
the later interval but not the vulnerable one. The required test must make the
failed child command write a valid checkpoint, replace it before returning to
the runner, then prove the report is `unavailable` and cleanup preserves the
replacement.

Closing that interval requires a trust anchor that exists before the external
child returns. Recording the pre-created inode alone conflicts with the current
atomic rename, because a successful rename must change it. Use a protocol that
lets the runner retain a stable descriptor/identity through the child update
(for example a runner-owned pre-opened checkpoint updated in place through a
controlled inherited descriptor), or another authenticated handoff whose
identity is established before command completion. Do not bind a pathname only
after the child returns and call that original identity.

### Re-review validation

| Check | Result |
| --- | --- |
| `source source_me.sh && python3 -m pytest -q tests/test_ui_walkthrough_runner.py tests/test_ui_walkthrough_harness_independence.py` | PASS: 44 tests, 9 subtests |
| `npx playwright test tests/playwright/simulator/instructor_setup_checkpoint.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts --workers=1` | PASS: 10 tests |
| `npx tsc --noEmit -p tsconfig.lint.json` | PASS |

## Re-review - inode-preserving repair

**ACCEPTED TO RERUN.** The repair removes the vulnerable bind-after-child
protocol. The runner now records the checkpoint's regular mode-0600
device/inode when it creates the file in the original mode-0700 state parent;
it does not overwrite that trust anchor after Playwright exits. The TypeScript
writer opens that same named inode with no-follow protection, verifies the
named path and descriptor identities agree before truncation, updates it in
place, fsyncs the file and parent descriptor, and rechecks the original parent
and file identities after the hostile hook. The runner reader opens
descriptor-relatively with no-follow protection and consumes a value only when
the descriptor identity remains the creation-time identity.

This closes the previously identified cross-process rename gap: replacing the
file after the child has written but before its command returns yields a
different inode, which the pre-established runner identity rejects. The durable
runner test performs that exact sequence using a same-mode hard link to a
forged external file, then proves the report contains only `unavailable`, does
not contain the forged stage or child output, preserves the external hard-link
target, and removes only the owned private state directory. The TypeScript
tests retain parent-replacement and same-parent file-replacement checks after
the in-place write. These are behavior-level filesystem security and TOCTOU
tests; this review did not require source-path, timeout, or other brittle
wiring proofs.

### Final focused validation

| Check | Result |
| --- | --- |
| `source source_me.sh && python3 -m pytest -q tests/test_ui_walkthrough_runner.py tests/test_ui_walkthrough_runner_cleanup.py tests/test_ui_walkthrough_harness_independence.py tests/test_ui_walkthrough_v2_report_contract.py` | PASS: 53 tests, 11 subtests |
| `npx playwright test tests/playwright/simulator/instructor_setup_checkpoint.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts --workers=1` | PASS: 10 tests |
| `npx tsc --noEmit -p tsconfig.lint.json` | PASS |
| Targeted ESLint and Prettier | PASS |

This accepts the source and focused non-live checkpoint boundary for the next
real Podman and Playwright rerun. It does not itself claim a live walkthrough
result.

## Re-review - visible login diagnostic

**ACCEPTED TO RERUN.** The authoritative WP-E2 contract now explicitly permits
eight closed instructor-failure values. It defines `login_visible` narrowly as
the rendered local-development credential control before any credential value
is entered. The child writes it immediately after the control's visible
assertion and before focus or fill; the value contains no credential, title,
alias, selector, identifier, or child output. Both TypeScript and Python
allowlists contain the same eight values, and the existing inode, ASCII,
single-line, mode, parent, and redacted-receipt boundaries still apply.

No extra permanent source snapshot test is needed for this small runtime
diagnostic vocabulary change. The durable security and TOCTOU tests remain the
relevant regression coverage.

### Diagnostic-delta validation

| Check | Result |
| --- | --- |
| Focused Python runner, cleanup, harness, and report-contract suites | PASS: 41 tests, 2 subtests |
| Focused Playwright checkpoint and live-config suites | PASS: 10 tests |
| Strict TypeScript, targeted ESLint, and Prettier | PASS |

This review authorizes the next real Podman and Playwright rerun. It does not
claim that rerun has passed.
