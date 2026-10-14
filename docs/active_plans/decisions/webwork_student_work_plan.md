# WeBWorK Student work plan

Date: 2026-10-13. Status: complete; implemented and verified.
Autosave and save before navigation selected; crash recovery excluded.

## Goal and evidence

Students can enter answers, change Questions, recover from failed saves, and revisit completed work
without losing answers or changing the Question they saw. WeBWorK owns controls, response meaning,
rendering, and grading; PLE owns persistence, navigation, authorization, and stored outcomes.

The [STUDENT_LAUNCH_READINESS_AUDIT_2026_10_13.md](../audits/STUDENT_LAUNCH_READINESS_AUDIT_2026_10_13.md)
reproduced answer loss during ordinary WeBWorK navigation, disabled save recovery, loss on leaving a
failed native save, and false submission status after a lost acknowledgement. Source inspection
at the audit baseline found that ordinary WeBWorK edits did not notify PLE, navigation skipped backend capture, and one
`failed` phase conflates document availability with save failure.

Revisit has an existing foundation: `WebworkAdapter::resume_document` and
`HttpWebworkRenderer::render_saved_response` render the retained source and seed with saved response
pairs, without submitting for grading. PLE also retains the originally issued HTML, its hash, and
renderer identity. At the audit baseline, completed review offered a separate correct-answer document; that was
insufficient for reviewing the Question and the Student's recorded response.

The user's settled requirements, recorded in [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), are:

- Within an Attempt, MC/MA choice order and MATCH positions remain consistent through navigation,
  reload, resume, and review. WeBWorK gets a new seed only on a new Attempt.
- After submission and grading, Students can see the Question, their recorded response, and whether
  it is correct or incorrect even when the Instructor withholds correct answers. Existing partial
  credit remains visible; optional feedback and correct-answer disclosure remain separate.
- Reconstruct WeBWorK from its retained evidence. Screenshots are a last resort.
- Use autosave while working and save before navigation. Preserving unsaved work after a browser
  crash is an edge case outside this plan.
- Prefer adaptability over speculative edge-case handling. Keep ownership clear and add mechanisms
  for demonstrated needs.

## Selected approach

**Autosave while working; save before navigation.** Read the latest answer from the WeBWorK frame
and finish saving it before moving to another Question, leaving the Attempt, or submitting. This is
ordinary automatic saving. Successful saves continue immediately without confirmation dialogs.
Show recovery choices only when unsaved work would otherwise be lost.

Saving only on navigation leaves work unsaved while a Student stays on one Question, so autosave is
also required. Server-acknowledged work survives across sessions. Persistent device drafts and
browser-crash handling are outside the implementation and acceptance scope.

For revisit, use seeded re-rendering with the recorded response as the primary path. Retained original
HTML is the fallback for the issued prompt if replay is unavailable or changes its meaning/order.
It does not by itself contain later responses, and its referenced assets may still require recovery.
Label any unavailable response view honestly. Screenshot storage is outside this plan.

## ADAPT comparison

Inspection of the local `OTHER_REPOS/adapt/` source found a narrower feature:

- `resources/js/pages/questions.view.vue:7058` autosaves eligible open-ended text every 10 seconds
  through `submitText(false)`, suppressing the ordinary success modal.
- `resources/js/helpers/HandleTechnologyResponse.js:210` handles WeBWorK's
  `webwork.interaction.submitted` event. Its WeBWorK path handles completed submissions, rather than
  notifying ADAPT of each edit for an independent response save.
- `resources/js/pages/questions.view.vue:6868` changes Questions without first requesting a
  WeBWorK response save. At line 6904 it restores the saved renderer `sessionJWT` when loading the
  next Question. The token comes from a stored submission in
  `app/Http/Controllers/AssignmentSyncQuestionController.php:3264`.

These paths provide precedents for quiet autosave and restoring submitted WeBWorK work, but do not
establish WeBWorK autosave-on-edit or save-before-navigation. This is source inspection of the local
copy, not live ADAPT acceptance or an audit of its separately hosted renderer.

## Design

**Adaptability through clear boundaries.** Keep capture in the backend integration, saving in the
Attempt, and grading in the Question Backend. These responsibilities can evolve independently as
actual needs emerge. Add states, hooks, or recovery behavior only for a concrete requirement or
demonstrated failure; keep ordinary save, navigation, and review flows simple.

**One owner of persistence.** Native controls and backend documents report edits to the Attempt.
The backend bridge produces an opaque response; the Attempt owns pending/saving/saved/error status,
save ordering, and acknowledgements. The frame owns only readiness and capture. A failed network
save leaves an available document editable and retryable. Display "Saved" only for the exact latest
response acknowledged by the server.

**Save before navigation.** Reuse the existing answer-read/save sequence for Question navigation,
explicit Save, leaving the Attempt, and Submit. Flush the debounce, capture the current document,
settle the latest valid save, then proceed. Track edits that arrive during a save so an older
acknowledgement cannot authorize discarding newer input. Scope work to the exact Attempt, Question
position, mounted document, and edit generation. Coalesce repeated actions. Bound capture waits,
cancel them on unmount, and ignore late replies; a missed reply leaves input available with Retry.

**Autosave without interpreting controls.** The WeBWorK-owned bridge reports input/change events
after widgets have synchronized their form values. Keep ordered pairs, repeated names, and legitimate
hidden answer fields. Reuse the existing debounce and avoid re-saving unchanged snapshots. Exercise
real pilot widgets first: programmatic value changes do not necessarily emit an
[input event](https://developer.mozilla.org/en-US/docs/Web/API/Element/input_event).
If a supported widget needs a synchronization hook, place it in the WeBWorK integration. PLE does
not learn MC/MA/MATCH DOM semantics. Initial load or passive capture must not mark an untouched
Question answered. Preserve the existing backend definition of an evaluable response and native
incomplete-input rules; do not replace a saved answer with invalid local input.

**Recover without losing work.** Reuse the route guard to save before leaving. On failure, offer
Stay, Retry save and continue, or explicitly Discard unsaved changes. Explain that discard retains
the last accepted server response. Keep the latest local answer while the page remains open and
retry on explicit action. Reuse the existing warning on deliberate reload/close while unsaved.
Server expiry still finalizes accepted work; a retry cannot extend time.
An uncertain submission acknowledgement triggers authoritative state reconciliation and a truthful
"Checking submission" or unconfirmed message, using the existing idempotent submission path.

**Revisit the same issued Question.** Resolve the original Attempt position, immutable Question
revision/source, seed, and finalized response server-side. Re-render the original choices and
positions with that response in a read-only backend document. This operation creates no Attempt,
seed, save, or grading outcome. Use retained grading for correctness and points. Unanswered Questions
still show their prompt and choices with "Unanswered". The renderer integration owns read-only
control presentation; a separate no-write document route and sandbox enforce the boundary.

Question/response/correctness review is independent of revealing the correct answer. Align the
server's history projection and copied disclosure rules with the user's requirement, rather than
only changing the visible interface. Authorize the Student and Attempt at the endpoint and recheck
after renderer I/O, following the existing answer-review pattern. Withheld answers, explanations,
and optional feedback stay out of the returned document and API data. Native Questions use their
retained presentation for the same review experience and do not acquire WeBWorK seeds.

Seed equality must produce the same MC/MA and MATCH arrangement for the same source. Verify this
with and without saved responses and with correct-answer display enabled/disabled. If a source or
widget uses fresh client randomness or response-dependent random calls, fix that owning source or
WeBWorK integration; do not compensate with PLE control parsers. Renderer identity and retained HTML
help diagnose a demonstrated replay difference. A different tool or renderer version alone must not
block working replay. Judge replay by the original Question content, choice order, selected response,
and correct disclosure. Check source, macros, and required assets when those behaviors differ;
do not silently substitute the latest Question.

Security requirements stay narrow: validate closed frame messages, origin, exact source window,
capture identity, and existing payload bounds (ASVS 3.5.5, 2.2.1-2.2.2); enforce ownership and field
disclosure on the server (8.2.2-8.2.3, 8.3.1); retain `no-store` and safe retry errors
(14.3.2, 16.5.1-16.5.3). Answer keys and renderer credentials remain private.

## Approach and milestones

1. **Prove the capture and replay contract.** Focus temporary real-renderer probes on pilot MC, MA,
   and MATCH. Include a representative FIB despite its smaller content share: type and pause for
   autosave, add a final character and immediately navigate, then revisit the saved text. Check
   clearing and incomplete input against the owning backend's rules with truthful saved status.
   Include a widget that synchronizes hidden inputs if present in pilot content.
   Check event delivery, blank/untouched behavior, save/resume pairs, and order stability for the
   same seed. Compare displayed content, response, and order with the original Question. Select the
   smallest bridge or backend-owned hook that passes. Reuse existing timeout conventions where
   available; a capture deadline exists only to recover from a missing reply, not to impose a speed
   gate. Ground any needed value in observed behavior. Report unsupported pilot controls explicitly.
2. **Unify saving and recovery.** Implement bridge notification, separate capture and persistence
   states, ordered saves, and the common save-before-navigation operation. Verify changing an answer
   while a save is pending and retrying without remounting. At the audit baseline, the response PUT
   carried no expected revision and SQL used an upsert: test a delayed older request after a newer save. If it
   can overwrite newer work, add an expected saved-response revision at the existing save boundary,
   atomically compare it in SQL, and reconcile conflicts while retaining local input. Client edit
   generations alone are not a server ordering guarantee. This is a bounded investigation before
   claiming stale-request safety, not a general multi-device editing system.
3. **Save before leaving.** Apply the common save operation to Question jumps, Ribbon,
   browser Back, and submission. Verify failed-save recovery, explicit discard, timeout, and expiry.
   Reconcile lost submission acknowledgements. A failure keeps the Student's work and identifies
   the next action; an already finalized Attempt opens its existing review.
4. **Add faithful completed review.** Expose authorized Question-and-recorded-response documents
   separately from correct-answer documents; reuse the current resume renderer and history evidence.
   Add native prompt/choice context from its retained presentation. Preserve stable order and reveal
   recorded responses, correctness, and points independently of correct-answer settings. Verify
   read-only behavior and renderer-outage recovery without mutating accepted work.
5. **Verify and document.** Run the checks below, review the real Student pages, remove temporary
   probes, and update backend/API contracts, design decisions, the audit status, and changelog.
   Record each acceptance result separately; a passing build does not close a browser defect.

## File scope

- `src/public/ple_bridge.js` and `src/components/question_response_controls/`: edit/capture contract
  and document readiness. Preserve opaque Instructor preview and Draft-test behavior.
- `src/pages/assessment_attempt_page.tsx`, `assessment_attempt_finish.ts`, and
  `assessment_attempt_response_state.ts`: one save owner, navigation ordering, recovery, and status.
  Extract a focused helper if needed to keep files below 1000 lines.
- `src/components/unsaved_changes_guard.tsx`: reuse its existing save-on-leave behavior; change only
  when Student recovery requires it, retaining Instructor behavior.
- `src/pages/assessment_attempt_summary_page.tsx`, `src/api/assessment_attempt_history.ts`, related
  decoders, and `src/components/opaque_webwork_preview_frame.tsx`: read-only review and Retry.
- `crates/server/src/assessment_delivery/`, `crates/server/src/webwork_document_route.rs`,
  `crates/domain/src/student_feedback_release.rs`, and `crates/adapters/webwork/`: retained-source
  review, server disclosure, and backend-owned replay. Extend the maintained renderer fork only if
  current rendering cannot supply the required read-only view; avoid patches to vendored controls.
- `crates/learning-data-access/`, browser contract roots, response HTTP client, and
  `schemas/base_schema/`: review evidence or conditional atomic save revision support. Edit the
  pre-production schema directly only when needed; regenerate derived API/schema outputs normally.
- Focused Node/Rust tests and `tests/playwright/`: protect preservation, ordering, review, and
  disclosure contracts. Temporary renderer/browser probes belong in `tests/_temp/`.
- [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), [CONTRACTS.md](../../CONTRACTS.md),
  [API_CONTRACTS.md](../../API_CONTRACTS.md), [DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md),
  [WEBWORK_PG_RENDERER_API_USAGE.md](../../WEBWORK_PG_RENDERER_API_USAGE.md), and
  [CHANGELOG.md](../../CHANGELOG.md): authority and implementation documentation.

## Completion checks

The following observed outcomes close this plan. A failure keeps its milestone open and is fixed
at the owning bridge, persistence, source, or disclosure boundary. Use representative cases for
these distinct product needs, not every combination of format, seed, browser, and failure.
Expand coverage when a concrete failure or compatibility concern justifies it.

| Journey | Required evidence |
| --- | --- |
| Answer, Next, Back, reload, fresh session | WeBWorK and native responses remain recorded and restored without a required manual Save. |
| FIB typing and navigation | Text autosaves while working; an immediate Next after the final character preserves that character. Clearing and incomplete input follow backend rules and show truthful saved status. |
| Same Attempt replay | MC/MA choice order, MATCH positions, authored values, and selected answers agree on initial display, navigation, reload, resume, and completed review, including answers-withheld mode. |
| New Attempt | The prior Attempt retains its seed/order/work; a seeded backend receives its new Attempt seed. Different seeds need not produce visibly different permutations by chance. |
| Fail save, edit, reconnect, Retry | Latest input stays editable and saves without changing Questions; an older acknowledgement cannot mark newer input saved. |
| Slow/missing capture or delayed older PUT | Finite recovery, preserved input, and no stale overwrite or cross-Question response. |
| Leave with unsaved work | Ribbon and Back save before leaving or retain the page on failure. Deliberate reload/close uses the existing unsaved-work warning; successful in-app saves proceed without confirmation prompts. |
| Submit and lose acknowledgement | Authoritative review confirms accepted work; rejected/unconfirmed outcomes are described truthfully and retry safely. |
| Expire while disconnected | Accepted work is finalized once; late local input cannot overwrite it or restart the timer. |
| Review with correct answers withheld | Original prompt/options, own selected response, correctness, and points are visible. Unanswered Questions remain recognizable. Protected solutions are absent from HTML and API responses. |
| Review access and renderer failure | Another Student cannot fetch the document. Review never saves/regrades. Renderer failure offers Retry and original context where available without hiding all recorded results. |
| Keyboard and scope | Navigation and Retry work by keyboard; native delivery, opaque previews, and Draft testing retain their intended behavior. |

Retain a small set of behavior tests for actual data-loss, stale-write, and answer-disclosure
regressions. Keep representative source/renderer compatibility probes as one-time evidence unless
they protect a distinct stable contract. Gate on answer preservation, required ordering, usable
recovery, and authorized review. Byte/pixel equivalence, arbitrary thresholds, exhaustive matrices,
and version equality are not requirements for this work. A version-specific restriction needs a
demonstrated compatibility failure and a correction or recovery action. Existing source integrity
and security checks retain their established purpose.

During implementation, run focused owner tests per milestone, then
`source source_me.sh && ./launchers/all_test.sh`. Run real-stack fault-injection journeys separately
and use `source source_me.sh && ./devel/capture_screenshots.sh --verify` for rendered UI evidence;
inspect the changed Student views. Record offline, database, browser, and visual results separately.

## Implementation evidence

The bridge reports whole-form edits and the Attempt owns ordered autosave and save-before-navigation.
Save acknowledgements update persistence metadata without remounting the Question. A failed save
keeps the input editable. The demonstrated unconditional SQL upsert allowed delayed older requests
to replace newer work, so accepted saves now advance an atomically checked saved-response revision.
Explicit Retry retains local input and uses the authoritative revision returned on conflict.

Completed WeBWorK review has its own authorized read-only document route, replaying retained source,
seed, and response. Renderer failure can show the retained original prompt with an honest notice
that the recorded response could not be displayed. Native review includes retained prompts, options,
and isolated author content. Own response and correctness are mandatory completed-review facts;
Instructor-controlled correct answers and optional teaching feedback remain separate.

| Evidence | Result |
| --- | --- |
| Maintained renderer replay | Passed real MC, MA, MATCH, and synthetic FIB semantic checks: same-seed arrangement, restored response, and answer disclosure. |
| Focused owner checks | Passed Rust and TypeScript compilation, domain review-policy tests, response-state checks, formatting, and schema style. |
| Complete repository gate | Passed the final `./launchers/all_test.sh`: Rust, WASM, TypeScript/Node, 9,090 pytest cases, and fresh PostgreSQL schema, authority, and persistence acceptance, including the stale-save oracle. |
| Real Student fault journeys | Passed immediate navigation, fresh-session resume, native FIB typing and clearing, missing capture, delayed successful/failed saves, native Ribbon retry, WeBWorK discard, lost submission acknowledgement, expiry, and authorized read-only review. |
| Revisit and new Attempts | Passed retained order/selection, unanswered prompt/options, distinct new-Attempt seeds, and renderer outage followed by Retry. |
| Rendered UI review | Passed all 257 live screenshot captures and privacy checks. Inspected active and completed Student views, including phone layout and answer-withheld review. |

Renderer receipts are in `output_launch_audit/student_work/seed_replay.json` and
`renderer_probe.json`; gate logs are in `output_launch_audit/student_work_all_test.log` and
`student_work_checks_before_browser.log`. Final browser and screenshot logs are
`output_launch_audit/student_work_final_browser.log` and `student_work_screenshots_final.log`.
Native author-content and expiry receipts are in the same evidence directory as
`native_author_review.json` and `expiry_probe.json`.
`canonical_student_work.json` records the maintained browser scenario. `fault_probe.json` records
the one-time failure checks, including server verification of the latest response after delayed
acknowledgements. `revisit_probe.json` and `new_attempt_seeds.json` record replay, renderer recovery,
and the read-only database seed check. Native FIB clearing is accepted by its existing text-length
format rule and remains saved after navigation; the check does not impose an invented nonempty rule.
The live database gate found an outdated Template-copy function grant after the policy change;
its signature was corrected and the complete gate rerun passed.
Live Demo startup then exposed its strict provisioning client's missing saved-response revision;
the client now reads the revision, supplies it when saving, and checks the accepted revision.
The next live save exposed a missing camelCase mapping for the server request's revision field.
The mapping is corrected and a focused server test now accepts the actual browser request shape
while requiring its revision. A fresh review traced the remaining browser/server/database/client
contract and found no additional mismatch.
The immediate-edit/navigation browser check exposed an unintended reactive dependency: saving
refreshed progress, which reloaded the current Question and canceled navigation. Presentation
loading now reacts only to the selected Question position, preserving the mounted input across
save acknowledgements. The unchanged browser journey verifies that correction.
The full screenshot run also exposed navigation before the visible WeBWorK form had sent its
ready message. Capture now waits for that message within the existing recovery deadline. A focused
browser check holds a frame resource, navigates before readiness, and verifies the untouched
Question remains unanswered. The delayed-readiness probe failed before the correction and passed
afterward; its receipts are `navigation_before_ready_failure.json` and `navigation_probe.json`.
One local external PGML candidate contains invalid Perl quoting and fails
compilation; representative valid source was used instead. That source/revision discrepancy is
separate from PLE answer persistence. No external Question source was edited.
