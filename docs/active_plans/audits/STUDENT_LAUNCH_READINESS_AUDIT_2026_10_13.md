# Student launch readiness audit

Date: 2026-10-13. Reviewed revision: `2b975cc7d16af98855f4006334ddf4e5df099bcd`.

## Current resolution

The five findings below are resolved by the
[WeBWorK Student work plan](../decisions/webwork_student_work_plan.md) implementation and real
Student browser acceptance on October 13. The original audit evidence remains below as the baseline.

| Finding | Implemented and verified outcome |
| --- | --- |
| S1 | WeBWorK autosaves; immediate Question navigation preserves work. Fresh-session resume restores the response and option order. |
| S2 | A failed native save keeps the Student on the page; Ribbon Retry saves and continues. Browser Back's leave warning preserves unsaved input when dismissed. Explicit discard keeps the last accepted response. |
| S3 | Failed saves and missing capture replies remain retryable without losing the editable response or remounting its document. Newer edits survive delayed save acknowledgements and failures. |
| S4 | A lost acknowledgement after server submission reconciles to the accepted Attempt review. |
| S5 | Completed native and WeBWorK review shows original Question context, recorded work, correctness, and points. Correct-answer disclosure remains independently controlled. |

The same acceptance verified native FIB final-character navigation and clearing, server expiry of
accepted work, distinct new-Attempt seeds, unanswered review, foreign-Student denial, no review
writes, and renderer outage/Retry. The native Instructor Draft leave-save/retry flow also passed.
Receipts are in `output_launch_audit/student_work/`; the plan records repository and visual gates
separately. These results address Student work and do not change the earlier hosting/email scope.

## Original student readiness decision

**Baseline decision: fix the response-preservation and recovery defects below before graded classroom use.**
This pass found four reproducible reliability defects and one observed learning-usability gap.
They concern ordinary Student work, independently of email accounts, hosting, or initial Sysadmin
setup. The earlier [LAUNCH_READINESS_AUDIT_2026_10_13.md](LAUNCH_READINESS_AUDIT_2026_10_13.md)
overweighted operational readiness; its green gates did not exercise these failure paths.

The method was a cognitive walkthrough with targeted browser fault injection against the real
disposable stack and built application. Synthetic Students used ordinary authorized Course and
Assessment flows. A small three-Question practice Assessment was created through visible Instructor
controls: a native FIB, a published WeBWorK Question, and a native MC. The base time was two minutes;
one Student received a 2X accommodation. No production code or permanent test was changed.

## Findings by student consequence

| ID | Priority | Student experience | Evidence |
| --- | --- | --- | --- |
| S1 | High | A WeBWorK answer disappears when the Student changes Questions without explicitly saving. | Reproduced with a working connection. |
| S2 | High | After a save failure, leaving for Courses discards the answer without warning. | Reproduced with a rejected response-save request. |
| S3 | High | After a WeBWorK save failure, Save stays disabled and Submit cannot proceed after connectivity returns. | Reproduced; changing Questions recovered the control. |
| S4 | Medium | A successfully submitted Attempt is reported as not submitted when its acknowledgement is lost. | Reproduced after an actual server HTTP 200 commit. |
| S5 | Medium | Native Attempt review omits the original Question prompts, weakening review and learning from mistakes. | Observed in the loaded review and confirmed in its data contract. |

### S1. Answers disappear during navigation

**Trigger:** enter a WeBWorK answer, choose another Question, then return. No network failure is
needed. The Student's selected radio changed from checked to unchecked; the server's saved response
was `null` both before and after navigation. Native controls autosave, so the same visible task has
different preservation behavior depending on its Question Backend.

**Cause:** [ple_bridge.js](../../../src/public/ple_bridge.js), lines 214-230, captures the form on
form submission or an explicit parent capture request. It does not report ordinary input edits.
[assessment_attempt_page.tsx](../../../src/pages/assessment_attempt/assessment_attempt_page.tsx), lines 280-285,
navigates using only the already captured parent response. The backend capture runs for whole-Attempt
submission, but not this transition. This is particularly relevant to the canonical Genetics PGML
content, which uses the backend-document path.

**Required outcome:** entering a complete backend-owned answer and moving among Questions preserves
the answer through the owning response-capture/save contract. A failed save preserves the input and
provides an understandable recovery choice. Keep backend response semantics with the backend.

**Acceptance:** repeat the normal enter/next/back/reload journey on a native Question and a
WeBWorK Question, verifying the actual server response and restored control in each case. Explicit
Save should remain available without being a hidden prerequisite for preserving one backend's work.

### S2. Leaving loses failed saves

**Trigger:** reject the native response PUT to simulate a connection problem. The page says
"Your response is still here. Save did not finish". Clicking Courses immediately leaves with no
dialog. Returning shows no selected answer and no saved response.

The same failure correctly prevents Question-to-Question navigation, retaining the local answer.
The defect is the missing protection across the surrounding application navigation.

**Cause:** [assessment_attempt_page.tsx](../../../src/pages/assessment_attempt/assessment_attempt_page.tsx), lines
258-268 and 440-462, keeps failed input in component state but has no route-leave or browser-unload
protection. Cleanup cancels the pending save timer. The existing Instructor editor protection in
[unsaved_changes_guard.tsx](../../../src/components/unsaved_changes_guard.tsx) demonstrates that
the application already has an owning navigation mechanism; this audit does not prescribe copying
it unchanged into Student delivery.

**Required outcome:** navigation gives the Student a way to preserve or explicitly discard unsaved
work. The message must make the actual preservation boundary clear. This does not require a general
offline application or a durable browser response cache.

**Acceptance:** fail a save, attempt to leave by Ribbon, browser Back, and reload, restore connectivity,
then save and resume. Verify that ordinary navigation cannot silently discard the pending answer.
Only the Ribbon departure and return were reproduced in this audit; Back/reload are correction gates.

### S3. Save failure disables recovery

**Trigger:** enter a WeBWorK answer and explicitly Save while its response PUT fails. Restore
connectivity. Save remains disabled; submitting the Attempt stays on the Attempt with a save error.
Changing to another Question and returning re-enables the button. That is an undisclosed workaround,
not an adequate Student recovery action under a running timer.

**Cause:** [backend_owned_document.tsx](../../../src/components/question_response_controls/backend_owned_document.tsx),
lines 205-207, 225-236, and 288-295, moves a failed save into `failed`, then permits capture and the
Save action only in `ready` or `saved`. The ordinary failure message cannot lead to a direct retry.

**Required outcome:** a failed save retains the editable answer and a working retry action.
Whole-Attempt submission can retry the same preserved response after connectivity returns.

**Acceptance:** fail and retry one native and one backend-owned save without changing Questions or
reloading, then submit and verify the recorded answer and score. Preserve the distinction between
an unavailable Question document and an available document whose response save failed.

### S4. Submission status can be false

**Trigger:** allow the real submission POST to finish, then drop only its browser response. The
server returned HTTP 200 with `submissionState: "submitted"`; an authorized progress read confirmed
the finalized Questions. The browser nevertheless displayed:

```text
This Attempt was not submitted: Failed to fetch
```

Retrying Submit recovered the submitted review. No duplicate submission or lost accepted work was
observed. The defect is false certainty at the moment a Student needs proof of completion.

**Cause:** [assessment_attempt_page.tsx](../../../src/pages/assessment_attempt/assessment_attempt_page.tsx), lines
336-347, treats an unknown transport outcome as proof of non-submission. The server already handles
an already-submitted Attempt in [submission.rs](../../../crates/server/src/assessment_delivery/submission.rs),
lines 192-195.

**Required outcome:** distinguish rejected submission from unconfirmed acknowledgement. Reconcile
with the authoritative Attempt state and show the accepted receipt when available.

**Acceptance:** separately reject a request before it arrives and lose its response after commit.
Both paths must show truthful status and support recovery without inventing another Attempt.

### S5. Review lacks question context

**Observed:** the native review displayed "Question 1", a response such as "Cystic fibrosis", its
score, and permitted feedback. It omitted the original prompt and answer choices. Unanswered
Questions appeared as numbers and zero scores with no question to revisit. A second completed
Assessment similarly showed "replication" as a response/correct answer without its native prompt.

For students using practice to learn, numbers and isolated answers are insufficient context for
understanding mistakes. This is a learning-usability finding, not a claim that answer disclosure
policy should be bypassed. A backend's authorized answer-review document may include more context;
the observed omission concerns the native review.

**Cause:** [assessment_attempt_history.ts](../../../src/api/assessment_attempt_history.ts), lines
14-27, provides response and feedback fields but no original Question presentation. The view in
[assessment_attempt_summary_page.tsx](../../../src/pages/assessment_attempt/assessment_attempt_summary_page.tsx),
lines 87-153, renders those fields under numbered headings.

**Required outcome:** give the Student enough of the exact issued Question to interpret their
recorded response, score, and permitted explanation. Preserve independent correct-answer and
feedback permissions and the backend-owned presentation boundary.

**Acceptance:** after submission, a Student can identify what was asked, compare their recorded
response with the prompt, and use permitted feedback without remembering the Question from the
timed session. Validate this on native and backend-owned content with representative Students.

## Checks that passed

| Check | Fresh result and limit |
| --- | --- |
| Native save persistence | An accepted selected answer was restored in a fresh browser session. |
| Failed save within Question navigation | A failed native save kept the Student on the Question with the answer intact. |
| Submission retry | Retrying after the lost acknowledgement opened the existing submitted review. |
| Individual accommodation | Two-minute base produced about 119.8 seconds for the standard Student and 239.8 seconds for the 2X Student at initial context reads. |
| Disconnected deadline | Closed the Student browser after saving one correct native response. After expiry, Instructor Gradebook showed Completed and scored, 1/3, before the Student reconnected. Returning Student review agreed. |
| Automated accessibility | Axe scans with WCAG A/AA tags found zero violations in the sampled native Attempt and backend-owned Question state. This is not a full keyboard or screen-reader evaluation. |
| Phone review width | The loaded native review's document width equaled the 390 CSS-pixel viewport. This is a sampled review check, not coverage of every Question Format or device. |

The earlier incomplete-answer navigation hypothesis was not reproduced with the tested FIB. Empty
text saved as an empty response and navigation continued. It is not included as a confirmed defect.
Some one-time scripts initially failed on a new-tab link, exact field labels, or an assumption that
reload selects the first saved Question. Those harness assumptions were corrected; they are not
product findings. The final probes waited for loaded review content rather than its loading shell.

## Task and guideline ledger

The users modeled here are remote biology Students working on a laptop or phone under an Attempt
timer. The primary outcomes are preservation of entered work, truthful completion, fair timing,
and enough context to learn from returned work. No Student participants were recruited.

| Task | Observable completion criterion | Result |
| --- | --- | --- |
| Answer and change Questions | The same answer returns and is recorded by the server. | S1 fails on WeBWorK; native accepted-save control passes. |
| Recover from a connection failure | Input remains available and a visible retry works. | S2 and S3 fail. |
| Submit the whole Attempt | The Student receives truthful accepted, rejected, or unconfirmed status. | S4 fails on lost acknowledgement; retry passes. |
| Resume and expire | Server deadline remains authoritative; saved work survives disconnection. | Sampled accommodation and disconnected-expiry checks pass. |
| Learn from returned work | The Student can identify the Question behind a score and response. | S5 remains open. |
| Use accessible controls | Names/status are exposed; critical actions work with assistive input. | Automated samples pass; full keyboard and representative screen-reader sessions remain unproven. |

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), lines 1852-1903, owns response persistence, automatic
submission, and timing. [NO_MOUSE_ACCESSIBILITY_CONTRACT.md](../../NO_MOUSE_ACCESSIBILITY_CONTRACT.md)
requires keyboard-accessible recovery. W3C's
[status-message guidance](https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html) and
[error-prevention guidance for test responses](https://www.w3.org/WAI/WCAG22/Understanding/error-prevention-legal-financial-data.html)
inform the recovery and confirmation criteria. The observed defects are not presented as a complete
WCAG conformance determination.

## Evidence and next work

Ignored local receipts and screenshots remain in `output_launch_audit/student/`:

- `observations.json`: failed-save navigation, accepted-save restoration, lost submission acknowledgement.
- `recovery_observations.json`: backend navigation loss, disabled retry, accommodation, loaded review.
- `deadline_receipt.json`: Instructor-observed background completion and subsequent Student review.
- `native_attempt_accessibility.json` and `backend_accessibility.json`: sampled automated scans.
- `failed_save_before_leaving.png`, `failed_save_after_return.png`,
  `backend_answer_before_navigation.png`, `backend_answer_lost_after_navigation.png`,
  `backend_save_retry_disabled.png`, `submission_response_lost.png`, and
  `loaded_attempt_review.png`: synthetic Student evidence.

The temporary probes were removed and the owned disposable stack was stopped. Tracked screenshots,
application code, schema, dependencies, and permanent tests were unchanged. The full suite and 257
capture replay from the first audit were not repeated; targeted failure-path evidence was needed.

Prioritize S1-S3, then truthful submission reconciliation and useful review context. Recheck the
specific failing journeys after corrections. Remaining Student acceptance includes representative
keyboard/screen-reader use, session-expiry recovery, concurrent tabs/devices, all required Question
Formats on phones, and classroom-sized simultaneous submission. These are untested risks, not
additional diagnosed defects. Mailbox and one-time administration work can proceed separately.
