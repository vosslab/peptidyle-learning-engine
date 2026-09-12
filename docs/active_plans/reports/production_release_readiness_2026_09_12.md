# Production release readiness: Student and Instructor experience

Reviewed September 12, 2026, against commit
`4ed6b2363cafc7e0df69b96e2c17e7c917772e54`.

## Conclusion and scope

The core teaching workflow is substantially implemented. The most valuable remaining work is
closing a small set of interaction and integration gaps, not adding every future capability.
The strongest concerns found here are unsaved Assignment policies at release time, navigation
that hides working editors, incomplete Student preview, and the limited scope of WeBWorK support.

The first production release is for Neil alone. Other users are not expected before January 2027.
Accounts, public authentication, and onboarding are deliberately outside this review's action
list. Student experience still matters now because the solo release is the opportunity to try
the teaching workflows before a class depends on them.

This is a read-only source and evidence inspection, with this report as its only repository
change. No application code, existing documentation, database, services, or deployment settings
were changed. No test suite or browser journey was run. Existing passing results below are
reported repository evidence, not fresh execution or a production certification.

The review follows [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md),
[TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md), and the current implementation.
[TODO.md](../../TODO.md) lists future capabilities; [ROADMAP.md](../../ROADMAP.md) records
completed database-baseline gates. Those completed gates should not be reopened merely because
a historical UI gallery shows a larger product.

## What appears substantially done

The current source and September 12 [CHANGELOG.md](../../CHANGELOG.md) support these boundaries:

- Course Instance creation and listing, Blueprint Draft editing and publication, and native PLE
  Question authoring/publication have real application paths.
- Assignment Questions and Policies use the direct current Assignment resource. Save, release,
  release validation, released edits, and title-confirmed Assignment Unrelease have backend
  implementations. Existing Attempts retain their effective evidence.
- Student Course and Assignment entry, one-Question delivery, response saving, whole-Attempt
  submission, retained history, and Instructor Gradebook have real application paths.
- Course Appearance, roster operations, Question Library search, and Assignments Due Soon are
  backed capabilities, not just illustrations.
- The current database baseline, default installation data, opt-out, and restore/migrate/verify
  boundaries have recorded acceptance evidence.

These are statements about implemented boundaries, not a claim that every option and failure
state in each journey has been personally exercised. The distinctions below matter more than
another broad claim that the whole application is either finished or unfinished.

## Highest-value work before relying on the release

### 1. Resolve unsaved edits before Assignment release or navigation

- [ ] Make the saved/unsaved distinction unmistakable in Assignment Questions and Policies.
- [ ] Prevent an Instructor from accidentally releasing older saved policies while newer values
  remain visible in the form. Either require an explicit save first or offer an explicit
  save-and-release operation with the correct conflict handling.
- [ ] Protect typed work when switching editor sections or leaving the page; extend the same
  review to Question and Blueprint Draft editors.

Evidence: [assignment_workspace_policies_page.tsx](../../../src/pages/assignment_workspace/assignment_workspace_policies_page.tsx)
holds policy inputs in local signals. `release()` uses the saved workspace ETag, not
`currentInput()`, and the release button is not gated on unsaved changes. Release validation also
checks saved state. [assignment_workspace_questions_page.tsx](../../../src/pages/assignment_workspace/assignment_workspace_questions_page.tsx)
holds edits locally and provides an ordinary link to Policies. No navigation/unload protection
was found in the current `src` search. Save-conflict messages already preserve local input until
the Instructor explicitly reloads; retain that useful behavior.

User consequence: an Instructor can believe the due date or feedback rule on screen is the one
being released, or lose an unsaved Question arrangement by following the next-step link.
This is a source-established mismatch; the exact browser transition still needs replay.

Completion check: change a saved due date and feedback rule, then try Release, Check delivery,
Edit Questions, browser Back, and reload. Every action must either use explicitly saved values
or clearly protect/explain the unsaved work. Inspect the subsequent Student Attempt to confirm
which policy actually took effect.

### 2. Reconcile Ribbon navigation with working Assignment pages

- [ ] Admit the already-backed Assignment Overview and Questions destinations through the
  normal capability evidence process.
- [ ] Check the round trip Questions -> Policies -> Overview -> Course Assignments without
  typing URLs or relying on browser Back.
- [ ] Refresh the generated destination ledger after correcting its source registry.

Evidence: [capability_registry.ts](../../../src/ribbon/capability_registry.ts) still marks
`assignmentOverview` and `assignmentQuestions` unbacked, while
[assignment_workspace_live_page.tsx](../../../src/pages/assignment_workspace/assignment_workspace_live_page.tsx)
mounts both against `getLiveAssignmentWorkspace` and the real
[assignment_release.rs](../../../crates/server/src/assignment_release.rs) router.
[app_ribbon.tsx](../../../src/ribbon/app_ribbon.tsx)::visibleControl omits unavailable controls.
The pages remain reachable through in-page links, so this is navigation inconsistency, not
absence of the editors or backend.

Completion check: all relevant editor destinations are visibly reachable and current-page
indication agrees with the displayed page. Update
[RIBBON_DESTINATION_LEDGER.md](../../ux/RIBBON_DESTINATION_LEDGER.md) through its generator,
not by hand-editing the generated table.

### 3. Treat WeBWorK as a bounded integration that needs a teaching walkthrough

- [ ] Complete the visible answer -> save -> reload -> submit -> result -> Instructor Gradebook
  journey with representative Questions from the content Neil actually intends to teach with.
- [ ] Decide the required feedback experience and make unsupported teaching feedback clear.
- [ ] Prove the user-visible outcome and recovery path when the renderer fails during grading.

Answer to "Does WeBWorK fully work as expected?": not established. M12 accepted the bounded
opaque lifecycle: PLE hosts the renderer's backend-owned document and passes its ordered form
pairs back for grading without projecting PG controls into PLE-native controls. That connected
evidence does not replace the teaching, feedback, outage-recovery, reload, or Gradebook walkthrough
needed before relying on content in class.

| Boundary | Current evidence and limit |
| --- | --- |
| Render and presentation | [issue.rs](../../../crates/adapters/webwork/src/lib/issue.rs) renders once and stores the exact backend-owned document. [webwork_document_route.rs](../../../crates/server/src/webwork_document_route.rs) serves that immutable document only to the Student's issued position. [backend_owned_document.tsx](../../../src/components/question_response_controls/backend_owned_document.tsx) presents it in a generic iframe. The current source is intentionally not a PG-control parser or a PLE-native response projection. |
| Response and grading | [ple_bridge.js](../../../src/public/ple_bridge.js) serializes complete form data as ordered `[name, value]` pairs, including legitimate PG hidden fields. [client.rs](../../../crates/adapters/webwork/src/http_renderer/client.rs) validates bounded ordered pairs and forwards them to the renderer without recognizing interaction types. [webwork_grading.rs](../../../crates/learning-data-access/src/postgres/webwork_grading.rs) loads only the persisted backend-owned payload; [worker.rs](../../../crates/server/src/worker.rs) reconstructs that opaque response for the one renderer grade call. |
| Educational metadata | The published revision's author-declared Question Type is used for library presentation. It is separate from WeBWorK controls and does not restrict how a PG Question renders or submits. |
| Current durable boundary evidence | [tests.rs](../../../crates/adapters/webwork/src/lib/tests.rs) asserts one render and one grade of a backend-owned payload, and refusal of native PLE response semantics. It is a compact contract suite, not a catalog of PG interactions. |
| Connected/browser evidence | [webwork_opaque_e2e_findings.md](webwork_opaque_e2e_findings.md) records the accepted one-time opaque curl and built-browser lifecycle: a generic iframe control saved and finished on a fresh Attempt without browser errors, and worker-to-history propagation recorded the result. Projection-era end-to-end machinery that assumed PLE-native response shapes or radio controls is retired; no replacement fixture corpus was created. |
| Teaching feedback | The completed submission and recorded score are distinct from optional Question-authored feedback. The actual feedback experience remains a product decision to walk through with the representative M12 Questions. |
| Renderer outage | [worker.rs](../../../crates/server/src/worker.rs) marks the affected grading Job failed when the renderer cannot return an evaluated result. It does not retry automatically. The Instructor recovery journey remains unproven until the connected walkthrough. |

Use a small representative set with materially different backend-controlled behavior. Exercise
correct, incorrect, and partial credit where useful; save/reload; final submission; recorded
score; and renderer failure/recovery. Include scientific notation, subscripts, superscripts,
emphasis, images, and equations when the selected instructional Questions use them. This is
connected behavior evidence, not a permanent compatibility catalog.

Completion means that the displayed Question remains scientifically meaningful, saved responses
survive reload, expected scores agree in Student history and Instructor Gradebook, disclosure
rules behave as described, and failed grading does not leave the learner guessing whether work
was submitted. M12 records the bounded lifecycle evidence; this report retains the broader
teaching walkthrough as a release decision requirement.

### 4. Provide a trustworthy Instructor view of Student delivery

- [ ] Finish the answer-free Student View, or explicitly retain the current narrow delivery
  check without representing it as a full preview.
- [ ] Make the preview useful for both native PLE and supported WeBWorK Questions.

Evidence: [assignment_workspace_student_view_page.tsx](../../../src/pages/assignment_workspace/assignment_workspace_student_view_page.tsx)
explicitly renders "Student view unavailable".
[assignment_preview_page.tsx](../../../src/pages/assignment_preview_page.tsx) renders saved title,
instructions, and Question descriptions; it does not display the actual Question interactions.
Its "Assignment delivery check" label is appropriately narrower than a full preview.

User consequence: an Instructor cannot currently inspect the complete learner-facing rendering
from that page before release. This is especially significant for untested WeBWorK content.

Completion check: inspect an Assignment while remaining an Instructor, see the intended Student
presentation and relevant policy facts, and confirm that inspection creates no Student Work or
grades. Until that exists, use a real disposable Student journey for content acceptance.

### 5. Show Students the deadline information needed to decide what to do

- [ ] Add due/availability/closing information to the actual Student landing and pre-start
  projection, not just an unused presentation helper.
- [ ] Show unambiguous local times with a visible time zone; exercise a different Student zone
  and a daylight-saving boundary before timed/deadline-sensitive classroom use.
- [ ] Review blocked-start messages so a Student knows when to return or what action to take.

Evidence: [student_course_landing_page.tsx](../../../src/pages/student_course_landing_page.tsx)
shows Assignment title and progress but no deadlines.
[assignment_overview_page.tsx](../../../src/pages/assignment_overview_page.tsx) displays Question
count, points, time limit, and generic start-decision messages. Its current
[assignment_attempt_issuance.ts](../../../src/api/assignment_attempt_issuance.ts) access type does
not carry due/available/closing timestamps. These omissions are not proof that server deadline
enforcement is absent; they are missing information at the learner's decision point.

Also format the Instructor workspace's raw `dueAt` value into readable local date/time in
[assignment_workspace_overview_page.tsx](../../../src/pages/assignment_workspace/assignment_workspace_overview_page.tsx).
Keep the authoritative time zone visible rather than asking users to interpret storage syntax.

Completion check: without opening another page or asking the Instructor, a Student can say when
the Assignment is due, whether it can be started, and what its time limit means. No deadline
must remain a clear, valid state. This can follow the solo launch but should precede real use
of deadline-sensitive Assignments.

### 6. Make grading progress and failure understandable; remove avoidable delay

- [ ] Distinguish accepted/submitted work, grading in progress, grading failure, and completed
  grading in the actual Student and Instructor pages.
- [ ] Establish a bounded Instructor/operator action for a failed automated grading job without
  requiring the Student to redo successfully accepted work.
- [ ] Rework the unconditional native PLE post-lease delay while preserving shutdown/recovery
  guarantees, then measure a realistic submission burst.

Evidence: [worker.rs](../../../crates/server/src/worker.rs) waits three seconds after every native
PLE job claim in a sequential loop. This limits one worker to at most about 20 jobs per minute
before object reads and grading costs. That is a source-derived ceiling, not a measured benchmark.
The same file makes WeBWorK grading failure terminal for that job. The Student landing currently
has coarse progress labels; selected history uses generic unavailable-score copy.

Completion check: watch one submission to completion, then a small concurrent burst, with the
actual worker processes running. Interrupt a renderer/worker only in a disposable acceptance
environment. The Student should know that submission succeeded independently of grading, and
the Instructor should be able to identify and resolve failed grading. Manual grading as a new
product capability is not required to close this automated-grading recovery gap.

## Smaller fixes and explicit product choices

### 7. Keep the existing banner visible after saving a theme

- [ ] Correct the Course Appearance response/cache mismatch and replay banner-first, theme-second.

[course_appearance.rs](../../../crates/server/src/course_appearance.rs) returns a successful
theme write as an aggregate appearance view with `banner: None`.
[course_appearance_page.tsx](../../../src/pages/course_appearance_page.tsx) replaces cached
appearance with that response. The UI can therefore make an existing banner disappear until
reload even though the theme operation did not delete its stored banner.

Completion check: upload and save a banner, change/save the theme, then inspect the current page,
Course list, and reloaded view. The banner must remain present throughout. This is a focused
integration repair, not a reason to rebuild Course Appearance.

### 8. Keep unimplemented management pages distinct from broken live features

- [ ] Make Grade Settings and Teaching Operations routes truthful if reached directly.
- [ ] Implement their missing backend workflow only if the first teaching release needs it.

[course_grade_settings_page.tsx](../../../src/pages/course_grade_settings_page.tsx) attempts to
load grade scheme and totals through [response.ts](../../../src/api/http_client/response.ts).
The `/api/courses/.../grade-scheme` and `/gradebook-totals` handlers are not present in the current
server composition. [teaching_operations.ts](../../../src/api/http_client/teaching_operations.ts)
also names teaching-management requests without current matching server handlers.
The Ribbon correctly withholds these capabilities today.

Do not describe these as working merely because a page/component exists. Equally, do not
describe the working [live_gradebook.rs](../../../crates/server/src/live_gradebook.rs) as absent.
If course-level weighting/categories are unnecessary for the first release, leave them deferred
and replace misleading retry-to-nowhere states with an accurate unavailable destination.

### 9. Close remaining authoring lifecycle UI only where Neil needs it

- [ ] Provide visible Question and Blueprint archive/restore controls if they are needed to
  clean up authored content. Apply the contracted Danger Zone treatment to destructive actions.
- [ ] Check the owner workflow for revising an already-published Question, including an edit
  reason and clear treatment of existing pinned use.

Archive/restore handlers exist in [question_library.rs](../../../crates/server/src/question_library.rs)
and [blueprint_course.rs](../../../crates/server/src/blueprint_course.rs), but the inspected
[question_detail_page.tsx](../../../src/pages/question_detail_page.tsx) and
[blueprint_course_workspace.tsx](../../../src/features/blueprint_course/blueprint_course_workspace.tsx)
do not expose equivalent archive/restore actions. The
[authoring.rs](../../../crates/server/src/authoring.rs) publication-revision handler likewise
does not by itself establish a discoverable browser revision workflow.

This is partial wishlist completion: backend support is real; convenient Instructor operation
is not established. Assignment Unrelease already has a dedicated Danger Zone and should not be
listed as wholly unimplemented. Full Fork, Question Change Proposal, and broader collaborative
stewardship can remain separate future capabilities.

### 10. Finish narrow operational edges that become visible product failures

- [ ] Add or identify the executable cleanup path for expired staged banner uploads.
- [ ] Validate the actual deployment's renderer and grading workers, not just API readiness.
- [ ] Preserve authored content across normal restart and prove backup/restore on the intended
  persistent deployment before relying on it as the only copy.

The staged banner lifecycle in [course_media.sql](../../../schemas/base_schema/course_media.sql)
has expiry checks, but no executable expired-upload cleanup consumer was found in the inspected
worker composition. Request-time cleanup is not the same as reclaiming abandoned uploads.

If using the repository AWS target, [compute.tf](../../../deploy/opentofu/compute.tf) has concrete
drift from [composition.rs](../../../crates/server/src/composition.rs) and
[application.rs](../../../crates/server/src/application.rs): the task wiring lacks required
browser-origin/renderer-version-file setup, uses the generic `--worker` process rather than the
actual native PLE and WeBWorK grading workers, and provides worker database environment names
that do not match the current composition's `DATABASE_URL` input. The WeBWorK worker's object
store construction also needs checking against that production target. A healthy local compose
stack does not close those gaps. This is conditional deployment work, not a requirement to
adopt AWS for the solo release.

## Current UI page map: distinguish implementation from historical intent

| User task or destination | Current assessment |
| --- | --- |
| Choose/create a Course Instance | Backed workflow; include it in the final Instructor task walkthrough. |
| Edit and publish a Blueprint Draft | Backed; naming refinements and visible archive/restore remain separate. |
| Create/publish a native PLE Question | Backed; do not infer generic PG authoring or published-Question editing from it. |
| Search Question Library | Backed; My Questions, Starred, and Watched are separate TODO capabilities. |
| Edit Assignment Questions/Policies | Backed; unsaved-state and Ribbon inconsistencies need attention. |
| Release/Unrelease Assignment | Backed; preserve confirmation and retained-work semantics while fixing release UX. |
| Instructor Student View | Explicitly unavailable; the separate delivery check is description-only. |
| Student Course/Assignment entry | Backed; deadline information is missing from the current visible projection. |
| Student answer/save/submit/history | Backed; complete real-content, failure, and feedback walkthroughs, particularly WeBWorK. |
| Instructor Gradebook | Backed; course grade settings and grading recovery are different capabilities. |
| Course Appearance | Backed; theme-save/banner mismatch and production-route accessibility evidence remain. |
| Grade Settings / Teaching Operations | Mounted pages without complete corresponding backend workflows; withheld by Ribbon. |
| Templates / public Blueprint search / saved Question lists | Genuinely future capabilities; no need to invent launch blockers from their placeholders. |

Map sources: [routes.ts](../../../src/routes.ts),
[capability_registry.ts](../../../src/ribbon/capability_registry.ts),
[composition.rs](../../../crates/server/src/composition.rs), and the linked page owners above.
[INSTRUCTOR_PAGE_VISUALS.md](../../INSTRUCTOR_PAGE_VISUALS.md) and
[STUDENT_PAGE_VISUALS.md](../../STUDENT_PAGE_VISUALS.md) explicitly describe historical references.
[SCREENSHOT_ATLAS.md](../../SCREENSHOT_ATLAS.md) and the generated destination ledger also need
reconciliation with recently completed Assignment/Draft work. Historical captures are not proof
that a current route or backend exists; a deferred capture is not proof that it does not.

## A small experience acceptance pass

Method: task-oriented expert inspection using learnability, visible state, consistent navigation,
error prevention, and recovery. This report used source traces and existing evidence. It did not
observe participants, measure task time, inspect a freshly rendered browser, or establish
accessibility conformance. Confusing-state findings are inspection findings, not measured rates
of Student confusion.

Use the following scenarios in an owned disposable environment. The first participant can be
Neil using the normal Instructor and Student interfaces. Before classroom use, include someone
unfamiliar with the application; do not coach them through missing cues.

| Scenario | User need and acceptance evidence |
| --- | --- |
| Prepare tomorrow's Assignment | Create/select content, edit Questions and Policies, leave and return, preview, and release. Complete without typed URLs, lost edits, or uncertainty about which policy was released. |
| Complete an unfamiliar Assignment | From the Course list, identify the deadline and time limit, start, answer, navigate, reload, and submit. Clearly distinguish saved responses from final submission and later grading. |
| Teach with WeBWorK | Use the content matrix in item 3. Verify readable scientific content, actual browser submission, expected scores, history, and Gradebook; explicitly record unsupported formats/feedback. |
| Recover from a failure | In the disposable environment, exercise a failed save, stale editor, and renderer outage. Preserve accepted/typed work where appropriate and provide a specific next action. |
| Work without a mouse or on a narrow screen | Traverse Question selection, matching controls, save/submit, feedback, dialogs, and error recovery. Inspect focus, names, status announcements, and reflow on the built application. |

For each scenario, record completion, interventions, lost work, misleading states, and whether
the user can explain the outcome. Record elapsed time as diagnostic information, not an arbitrary
hard gate. Any lost work, wrong-policy release, unusable required Question, or incorrect score
blocks that workflow until resolved. Minor cosmetic issues can remain optional.

[COURSE_APPEARANCE_ACCESSIBILITY_AUDIT.md](../../ux/COURSE_APPEARANCE_ACCESSIBILITY_AUDIT.md)
explicitly limits its automated component evidence and leaves production-route accessibility
open. [STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md](../../ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md)
also needs to be interpreted against the current delivery flow, not treated as a blanket pass.
Reuse the repository's semantic browser/axe checks and add the manual task walkthrough; do not
turn screenshots or layout counts into permanent acceptance requirements. Follow
[TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md) for test lifetime and ownership.

## Could wait: do not turn the future wishlist into a launch checklist

The following remain reasonable follow-on work from [TODO.md](../../TODO.md): public Blueprint
Course search; My Questions, Starred, and Watched; Assignment Templates; Blueprint short/long
name editing; Question Change Proposal; and complete Course Retention. Prioritize them when the
associated real task is needed. Retention needs an explicit operational decision before actual
Student Work reaches its contracted lifecycle, not speculative schema scaffolding now.

Co-Instructor management, broader content stewardship, accommodations needed by actual learners,
course-grade weighting, and additional backend formats should be scheduled against their first
real use. None should obscure a broken save, preview, submission, or grading path today.

Suggested order: fix unsaved-policy release and Assignment navigation; perform the WeBWorK
content walkthrough; decide preview/feedback requirements; fix the banner mismatch and grading
delay; then run the small end-to-end experience pass. Address the actual deployment wiring if
moving off the local stack. The final production decision remains Neil's explicit decision under
[ROADMAP.md](../../ROADMAP.md), including the base-schema freeze; this report does not make it.
