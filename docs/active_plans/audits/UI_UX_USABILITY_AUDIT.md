# UI, UX, and usability audit

Date: 2026-09-16. Status: historical screenshot baseline with dated current-source and rendered
receipts. Findings without an explicit current disposition remain open.

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) owns durable product guidance. This audit records
concrete examples and acceptance checks for implementation review.

This audit covers specific interface findings across Student, Instructor, Sysadmin, and public
surfaces. Extend it with role-specific evidence and acceptance checks as reviews proceed. Keep
durable, reusable guidance in HG and observed implementation gaps here.

## Evidence scope

The initial findings come from inspected Student PNGs in [screenshots](../../screenshots/student/), supplemented
by the user's live-demo observations. The review uses heuristic inspection of recognition,
information density, responsive presentation, and visible state. It covers Student laptops,
portrait tablets, narrow phones, and square displays as specified in HG. Instructor and Sysadmin
workflows target laptop browsers.

The committed PNGs establish visible presentation at their capture time and are historical until
the planned full canonical replay replaces the current corpus. Live behavior, keyboard traversal,
accessible names, touch interaction, and current source remain verification work. Findings below
remain open until the corresponding rendered and interaction checks pass. The initial inspection
was not a runtime audit or participant study; dated receipts below add bounded runtime evidence.

## Review coverage

- Student: initial screenshot findings recorded below; interaction checks remain open.
- Instructor: all 32 current PNG files inspected during this session; findings below concern
  visible laptop presentation, with fresh-capture and interaction checks still open.
- Sysadmin: laptop review findings can be added as the review expands.
- Public: entry and authentication surface findings can be added with their applicable screen targets.

## State and task clarity

### Student Question rendering coverage

Status: bounded private Student presentation receipt completed; full interaction matrix and
Instructor-preview coverage remain open.

Review Student-facing delivery of one WeBWorK Question and each of the eight native PLE JSON
Question Types specified in [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). Instructor Library
previews supplement this evidence; Student delivery captures establish the requested presentation.

| Example   | Required visible content                                           |
| --------- | ------------------------------------------------------------------ |
| WeBWorK   | Backend-rendered prompt, response controls, and permitted feedback |
| MC        | Single-answer choice selection                                     |
| MA        | Multiple-answer choice selection                                   |
| FIB       | One text-entry blank                                               |
| MULTI-FIB | Multiple independently identifiable text-entry blanks              |
| NUM       | Numeric response and any relevant unit or tolerance instructions   |
| MATCH     | Prompts, shared choice bank, and visible assigned matches          |
| ORDER     | Items and their current ordering                                   |
| HOTSPOT   | Static visual asset and visible selection cues                     |

Capture representative examples on a laptop and narrow phone, with portrait-tablet and square
layout checks where the response arrangement changes. Inspect initial and entered-response states,
saved state, and permitted submitted/review feedback. Include long content where it tests wrapping
or alignment. Use meaningful teaching examples rather than empty controls.

Walk each response through keyboard-only interaction and pointer or touch as applicable. For MATCH
and ORDER, demonstrate assigning or moving, correcting, and clearing responses through the supported
methods. Verify focus, prompt/control association, saved-response persistence, and contrast against
actual rendered backgrounds. Record each Type separately, including unavailable Types as explicit
coverage gaps. Screenshots establish presentation; interaction receipts establish usable behavior.

Receipt (2026-09-16): the current authorized Student traversal produced 18 private unanswered
captures and privacy checks: WeBWorK plus all eight released native types at laptop and phone
widths. The captured HOTSPOT showed its revision-pinned raster surface and labeled selection cues.
This establishes bounded presentation coverage only, not saved/submitted interaction for every type,
and it does not promote the private captures to the official screenshot corpus. The separate
canonical HOTSPOT workflow now proves pointer and keyboard selection, Save/reload of the exact
issued Question Revision, whole-Attempt submission, and server-marked correctness. Receipts:
`/private/tmp/ple-resumed-types-20260916.md` and
`/private/tmp/ple-hotspot-connected-interaction-20260916.md`.

Current native-interaction evidence (2026-09-16): the owner-reported connected `8104` traversal
covered MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT in pointer and keyboard contexts:
Save/reload and ordinary return restored each recorded response at its exact issued pin, then one
whole-Attempt submission per context marked every native position Submitted. The independent
read-only review accepted this deliberately bounded private receipt and its submitted screenshots.
Root independently fetched current `https://localhost:8104/main.js`; its SHA256 matched local
`dist/main.js` (`6aa85b2e19f197c47896f6e3e136e2d15167ecb2b7fa7178666e538668f02e54`), and all 12
source-map entries below `src/components/question_response_controls/` matched current source. This
resolves the review's source-correspondence qualification only. No retained owner successful-run
stdout/stderr transcript exists, so the reported exit-0 execution and its assertion count remain
owner-reported rather than independently replayable transcript evidence. No official corpus
refresh, full responsive/touch/contrast matrix, new SQL/server claim, or broader UI closure follows.
Receipts: `/private/tmp/ple-native-connected-interaction-20260916.md` and
`/private/tmp/ple-native-interaction-receipt-review-20260916.md`.

The Instructor preview delivery defect is closed by the 2026-09-17 canonical HTTPS receipt under
I15. Student captures still do not establish Instructor layout consistency, preview height, or the
fresh official screenshot corpus.

### S01: accepted invitation retains pending-state actions

- Evidence: [invitation_accepted_laptop.png](../../screenshots/student/invitation_accepted_laptop.png)
  shows "Invitation accepted" alongside "Join this course", pending-state instructions, and an
  "Accept invitation" action.
- User need: recognize successful enrollment and proceed into the Course.
- Acceptance: after acceptance, show a coherent joined state and make opening the Course or
  Coursework the next primary action. Verify the transition in a live walkthrough.
- Receipt (2026-09-16): bounded connected acceptance on canonical local runtime `8258` used the
  ordinary visible Student demo path to claim a Course. The joined page rendered `You joined this
course`, `Invitation accepted.`, and one `Open course` action; keyboard Enter reached that
  claimed Course. A held real claim response could not present the old Course's joined result after
  navigation to another pending invitation; that route joined only through its own real claim.
  This supersedes the isolated-only status for this bounded transition, not the full invitation
  authorization, error, or mobile matrix. Receipts:
  `/private/tmp/ple-rebuilt-roster-ui-acceptance-20260916.md` and
  `/private/tmp/ple-student-invitation-joined-state-20260916.md`.

### S02: Coursework actions communicate little about the next step

- Evidence: [course_in_progress_laptop.png](../../screenshots/student/course_in_progress_laptop.png)
  and [course_completed_laptop.png](../../screenshots/student/course_completed_laptop.png) both use
  "Open Regular Assignment" despite different completion states.
- User need: recognize whether the next step continues work, reviews a result, or starts another
  permitted Attempt.
- Acceptance: use state-aware action labels that accurately describe their destinations, such as
  Resume or Review where applicable. Check completed Coursework with and without further Attempts.
- Receipt (2026-09-16): `src/pages/student_coursework_presentation.ts` and the Student Course
  landing page now choose state-aware coursework actions. The accepted component-browser receipt
  at 1280 and 390 pixels keyboard-activates Resume, Review, and Open, retaining existing axe and
  page-error checks; root `node --import tsx tests/playwright/student_course_entry_m6_evidence.mjs`
  exited 0. This bounded S02 receipt does not establish live perfect-score behavior, all Student
  interface acceptance, full HTTP/browser coverage, or global UI closure.

### S03: active-work progress is expressed as grading progress

- Evidence: [course_in_progress_laptop.png](../../screenshots/student/course_in_progress_laptop.png)
  reports "0 of 4 questions graded" for in-progress Coursework.
- User need: understand how much work is recorded during an active Attempt.
- Acceptance: distinguish saved or answered progress from grading progress using Student language.
  Verify the summary against the actual recorded state; use grading counts where results are relevant.
- Receipt (2026-09-16): [student_assessment_landing.sql](../../../schemas/base_schema/student_assessment_landing.sql)
  and [student_course_landing_page.tsx](../../../src/pages/student_course_landing_page.tsx) now
  carry an answer-free saved-response count and render active Coursework as `N of M responses
saved`; completed Coursework retains its existing grading and disclosed-score branch. Accepted
  source review, focused Cargo, strict TypeScript, and 1280/390 component-browser gates passed.
  A fresh canonical PostgreSQL 17 actual-role `ple_auth` -> `ple_app` proof observed no-Attempt
  zero, an ordinary Student save from `0` to `1`, replacement remaining `1`, a later empty Attempt
  returning `0`, and Student/Course isolation. Receipts:
  `/private/tmp/ple-student-saved-progress-slice-20260916.md`,
  `/private/tmp/ple-student-saved-progress-review-20260916.md`, and
  `/private/tmp/ple-student-saved-progress-sql-proof-20260916.md`.
  This is not connected acceptance: current `8104` did not run the HTTP/browser save-return-submit
  journey, so S03 remains open for that proof.

### S04: Attempt history follows a tall settings summary

- Evidence: [assignment_overview_history_laptop.png](../../screenshots/student/assignment_overview_history_laptop.png)
  places "Previous attempts" at the bottom of the captured viewport after vertically stacked rules.
- User need: find recorded Attempts and results as readily as the next available work action.
- Acceptance: make previous Attempts easy to locate alongside a compact overview. Check laptop and
  narrow Student layouts with several Attempts and long titles.
- Receipt (2026-09-16): independent source/render review accepted compact rules disclosure and
  details at `/private/tmp/ple-student-rules-disclosure-review.md`. Root
  `node /private/tmp/ple-student-rules-proof.mjs` exited 0 at 1280 and 390 pixels. This is
  component-only evidence: it does not establish full shell, theme, zoom, or all-Student-interface
  acceptance.

## Layout and recognition

### S05: settings dominate Coursework entries and overviews

- Evidence: [course_completed_laptop.png](../../screenshots/student/course_completed_laptop.png)
  repeats due-time and time-zone information in a tall access section;
  [assignment_overview_unanswered_laptop.png](../../screenshots/student/assignment_overview_unanswered_laptop.png)
  stacks short labels and indented values across most of the viewport.
- Acceptance: use compact summaries with visually connected labels and values. Keep essential
  information and actions visible; disclose fuller rules through accessible expandable details.
  Check that Students can scan several Coursework entries and locate the start action promptly.
- Receipt (2026-09-16): the same independent source/render review accepted the compact details
  component at 1280 and 390 pixels. Evidence is
  `/private/tmp/ple-student-rules-disclosure-review.md` and root
  `node /private/tmp/ple-student-rules-proof.mjs` exited 0. This does not establish full shell,
  theme, zoom, or all-Student-interface acceptance.

### S06: Question navigation consumes vertical working space

- Evidence: [assignment_attempt_saved_laptop.png](../../screenshots/student/assignment_attempt_saved_laptop.png)
  shows a vertical numbered list with tightly joined text such as "Question 1Saved" and controls
  visually inconsistent with PLE response actions.
- Acceptance: use horizontal numbered navigation with distinct current and saved cues, consistent
  PLE styling, and pagination for long sets. Verify every Question remains reachable by keyboard
  and on narrow layouts, with the prompt and response controls near the top of the working area.

### S07: phone navigation loses readable context

- Evidence: [course_not_started_phone.png](../../screenshots/student/course_not_started_phone.png)
  clips product and navigation text;
  [assignment_attempt_submitted_phone.png](../../screenshots/student/assignment_attempt_submitted_phone.png)
  reduces several breadcrumb levels to short fragments.
- Acceptance: adapt Student navigation to narrow widths while preserving recognizable current
  context, reachable navigation, and Profile access. Check long Course and Coursework names,
  intermediate widths, and enlarged text.
- Receipt (2026-09-16): the accepted bounded source and runtime fix in
  `src/ribbon/app_ribbon.tsx`, `src/ribbon/app_ribbon.css`, and `src/application_shell.tsx` has
  content-sized identity at upper left, role-aware flex navigation, and Profile at upper right.
  Current evidence is `/private/tmp/ple-student-nav-proof/after-report.json` and its after PNGs:
  full-width navigation, full breadcrumb labels with keyboard-reachable ancestors, and no document
  horizontal overflow at 320, 390, and 1280 pixels, including 320 pixels at 200% enlargement.
  Independent review accepted the P1/P2 corrections. This does not close global responsive or all
  S07 acceptance: Profile omits HG's permanent breadcrumb reservation and overview body wrapping
  remains awkward at 200% enlargement. Root reports typecheck/build pass for `6f09e3e8`; the fresh
  fast suite has 7,367 passes and four known Markdown-link failures.

### S08: review information is spread over many lines

- Evidence: [assignment_attempt_summary_laptop.png](../../screenshots/student/assignment_attempt_summary_laptop.png)
  and [assignment_attempt_submitted_phone.png](../../screenshots/student/assignment_attempt_submitted_phone.png)
  separate each Question's submission status, result, points, response, and feedback with repeated gaps.
- Acceptance: group each reviewed response and its permitted result information into a compact,
  clearly separated unit. Verify feedback disclosure follows Coursework settings.
- Receipt (2026-09-16): the accepted bounded implementation groups submitted Student history into
  compact, separated Question units with wrapping submission/result/points metadata and a labeled
  recorded response. Isolated actual-component browser proof at 1280 and 390 preserves permitted
  result information, withholds protected grade and teaching fields, contains long prose and code,
  retains Unanswered, and preserves keyboard Return access without document overflow. The source
  review accepts the unchanged server-projected disclosure conditions. This is not deployed demo
  `8147`, connected authorization or Coursework-settings acceptance, or global Human Guidance
  closure. Receipts: `/private/tmp/ple-parallel-review-density-20260916.md` and
  `/private/tmp/ple-parallel-summary-density-review-20260916.md`.

## Color evidence limitation

The green backgrounds and layered rounded surfaces warrant rendered theme review. A sampled
body-text foreground `#315047` and nearby background `#B5DBA9` from
[assignment_attempt_resumed_tablet.png](../../screenshots/student/assignment_attempt_resumed_tablet.png)
measure 5.77:1. This pair passes the preferred 5.5:1 text target; it establishes neither a whole-page
failure nor a whole-page pass. Neighborhood-averaged image sampling also blends anti-aliased text
with its background and is unsuitable as definitive text-contrast evidence.

Acceptance: verify actual foreground/background pairs and meaningful state cues in both rendered
schemes against [BIOME_THEME_PALETTES.md](../../BIOME_THEME_PALETTES.md). Include gradients, selected
states, control boundaries, and keyboard focus. Preserve recognizable text or shape cues alongside
color. Record computed-color evidence before closing any contrast finding.

## Instructor review

[ADAPT_UI_AUDIT.md](ADAPT_UI_AUDIT.md) records ADAPT observations and comparison comments.
HG owns PLE classification and validation decisions. PLE-specific findings remain in this audit.

The review covers the 32 PNGs in [instructor screenshots](../../screenshots/instructor/), including
the files inspected earlier in this session. The task model is: find reusable content, inspect it,
author or adopt it, assemble and release Coursework, and manage enrollment and results. Priority
below reflects friction at those steps, rather than implementation effort.

[current_capture_manifest.json](../../screenshots/current_capture_manifest.json) marks Assessment
Templates as needing a fresh capture and the legacy multi-tab Assignment workspace as retired.
Some manifest entries still describe required Ribbon destinations as deferred; those descriptions
are historical and do not override the current Ribbon contract below. File presence and filenames
alone establish neither current route behavior nor successful task completion. Reproduce findings
on the admitted interface before making implementation changes. Preserve useful teaching behavior
while correcting its presentation.

### Current Ribbon acceptance (C44)

The two requirements are distinct and both apply before screenshot refresh:

- every required Ribbon choice remains visible when its collection is empty, with an honest empty
  state and an obvious first action where applicable; and
- every required Instructor Ribbon choice remains visible when its target page is incomplete, but
  the unfinished destination is visibly unavailable rather than presented as usable.

Current source and the canonical HTTPS authorization/Ribbon scenario pass those behaviors for the
Instructor Ribbon. This is connected interaction evidence, not a claim that the old PNG corpus is
fresh. The next manifest replay must capture the same visible choices and replace stale
deferred/future descriptions with their current disposition.

### I01: draft records concatenate distinct information

- Priority: high; finding visible in
  [question_drafts.png](../../screenshots/instructor/question_drafts.png).
- Titles run directly into descriptions and draft metadata. Repeated Delete actions appear on
  their own lines, while the title and description share one long link.
- Acceptance: use compact aligned records with distinct title, description, state, and actions.
  Keep opening or continuing a draft the clear routine action. Check long titles and descriptions.
- Receipt (2026-09-16): independent review accepts the bounded Draft Questions presentation
  correction. At 1280 pixels, rows are compact aligned records with separate full titles,
  descriptions, state, and accessible Edit/Delete actions; the observed normal row height is
  55.06 pixels, compared with 59.67 pixels before the correction. The temporary component-browser
  receipt at `/private/tmp/ple-draft-list-proof.mjs` observes no document horizontal overflow at
  1280 pixels, 320 pixels, and 640 pixels with enlarged text. Keyboard Delete opens the existing
  confirmation with focus on Keep draft. These observations and a laptop screenshot were reviewed,
  but the harness does not assert every observation. Connected-route/CAS/error behavior,
  successful deletion, and actual dark-theme evidence remain unverified; simulated dark mode is
  not theme evidence.

### I02: roster management puts the roster below setup controls

- Priority: high; evidence:
  [course_roster_active.png](../../screenshots/instructor/course_roster_active.png) and
  [course_roster_pending_invitation.png](../../screenshots/instructor/course_roster_pending_invitation.png).
- A large empty import area and invitation-download panel push Current roster to the viewport
  bottom. Visible columns identify Students by roster ID rather than recognizable names.
- Acceptance: make current Students and their states easy to inspect at 1280 by 800. Present import
  and invitation tools compactly or through disclosure. Establish available roster-name data and
  permitted display before adding names; retain course-scoped IDs as useful supporting information.
- Receipt (2026-09-16): accepted 1280-pixel and keyboard evidence shows Current roster before
  Roster tools. The tools use a native disclosure that starts closed; keyboard activation reaches
  its import and invitation-export controls. A local malformed import shows the existing invalid
  input feedback and records no roster API write. Roster names remain an independent data and
  permitted-display gap.
- Empty-roster follow-up (2026-09-16): the empty explanation now has a local Import Students action
  that opens the same Roster tools disclosure and moves focus to its existing import field. A
  temporary canonical HTTPS walkthrough covered keyboard activation, 1280px and 320px layouts,
  and the transition from empty state to the ordinary populated Student table after one import.
  Full current-path roster authority and browser acceptance now pass after the repairs. The
  protected invitation-export route and owner-private attended-mailer dry run also pass without
  Mail delivery; no screenshot refresh follows.

### I03: successful roster import looks like an error

- Priority: high; evidence:
  [course_roster_pending_invitation.png](../../screenshots/instructor/course_roster_pending_invitation.png).
- "Roster import recorded" uses red text, a red edge, and a pink background despite describing
  successful import with expected pending invitations.
- Acceptance: distinguish recorded import, pending enrollment, and actual import errors through
  accurate wording and semantic styling. Verify success, partial failure, and rejected input states.
- Receipt (2026-09-16): source review accepts distinct success and error presentation: success
  uses local status semantics and the success style, while invalid import feedback uses error/alert
  semantics. The current runtime receipt proves only rejected local input; it does not submit an
  actual successful import, export, or revocation, and it does not establish partial-success
  behavior. Those outcome states remain open.

### I04: Question inspection puts metadata ahead of teaching content

- Priority: high; evidence:
  [published_question_detail.png](../../screenshots/instructor/published_question_detail.png) and
  [webwork_generated_example.png](../../screenshots/instructor/webwork_generated_example.png).
- Repeated title/reference text, separate Star and Watch rows, author, Backend, Revision, and
  explanations consume much of the captured working space before the Question.
- Acceptance: make the prompt and response presentation easy to inspect, with compact metadata
  and supporting actions. Preserve Question Backend rendering where its typography is necessary.
- Receipt (2026-09-16): at the bounded 1280 by 800 laptop viewport, the current detail page
  presents the native prompt or backend-owned WeBWorK preview before its compact support region.
  The support region then groups the copyable reference, metadata, generated-example note, and
  Star/Watch controls. The supplied independent review and
  `/private/tmp/ple-question-inspection-proof/report.json` accept prompt-first order for both
  backends; the two accompanying screenshots are temporary review evidence, not permanent assets.
  Native Star and Watch are keyboard-reachable without activation. The sandboxed WeBWorK preview
  retains its eight renderer-owned controls, without a protected-answer request or visible
  protected-answer text. The receipt reports no writes and no page errors. Root recorded frontend
  build `fa08ad78` with exit 0. This is a bounded I04 improvement, not all UI closure: the native
  capture displays its existing stem only and does not establish native response-control rendering
  for every Question Type, a submission workflow, or broader interaction/accessibility coverage.

### I05: publication success offers a generic destination

- Priority: high; evidence:
  [published_question_result.png](../../screenshots/instructor/published_question_result.png).
- Publication succeeds, but the breadcrumb still names a Draft Question, the header remains
  authoring-oriented, and the offered link opens the Library rather than visibly identifying a
  direct path to the newly Published Question.
- Acceptance: identify the published object and offer a direct inspection action. Keep heading,
  breadcrumb, status, and next actions coherent with the resulting state. Verify the live transition.
- Receipt (2026-09-16): independent review accepts the bounded connected desktop publication
  transition. At `https://localhost:8067`, fake Elena published draft `D-56` as `GC4P-S7SG`,
  Revision 1. The `Question published` heading receives DOM focus; neutral Question-authoring
  navigation remains; Save draft is absent; and the exact title, public ID, and Revision are
  visible. The direct `/library/GC4P-S7SG` link and keyboard Enter open that exact Question/title.
  `/private/tmp/ple-publication-browser-proof-result.md` reports exit 0, and independent review
  visually inspected both PNGs. This does not establish mobile, theme, broad accessibility,
  failure, concurrency, or all-authoring behavior.

### I06: assessment identity and records are hard to scan

- Priority: high; historical workspace evidence:
  [assignment_release_draft.png](../../screenshots/instructor/assignment_release_draft.png),
  [assignment_release_released.png](../../screenshots/instructor/assignment_release_released.png), and
  [course_assignment_workspace.png](../../screenshots/instructor/course_assignment_workspace.png).
- Generic Assessment breadcrumbs and editor headings provide little object identity. Ordered
  entries combine IDs, Revision, description, points, availability, scoring, and limits in prose.
  "Initial Teaching Team" also reads like setup terminology in an ordinary Course workspace.
- Acceptance: on the current admitted editors, keep the Assessment title and release state
  recognizable. Use aligned Question records with compact supporting details and clear actions.
  Reproduce on current routes; the manifest retires the legacy multi-tab workspace.
- Current bounded receipt (2026-09-16): the Course Instance workspace now leads with its ordered
  Assessments and adjacent Create Assessment action. Each compact row exposes its order, title,
  release state, public reference, due time, and local edit/lifecycle actions; Course metadata and
  administration follow as secondary details. Temporary actual-component evidence passed empty,
  loading, error, populated, long-label, keyboard-first action, and overflow cases at 1280 by 800
  and 320 pixels. Strict TypeScript, focused Node, formatting, and diff checks passed.
- Current C420 source receipt (2026-09-17): Course tools now includes Create Blueprint from Course
  Instance. Its compact dialog has only new Blueprint short name, long name, and shared
  classification; it preserves entered values on a safe failure, reports busy creation accessibly,
  and returns focus on cancel. The user-facing explanation distinguishes copying reusable structure
  from altering the source Course Instance and names its first-Adoption result. Focused client
  transport/decoder, TypeScript, ESLint, and Prettier checks pass. Canonical HTTPS browser evidence
  used exactly one child-route POST to create a distinct actor-owned Private Revision-1 Blueprint,
  showed Adoption count 1, and left the source Course addressable and unchanged. No screenshot
  refresh follows.
- Current editor receipt (2026-09-17): the already-authorized Assessment workspace now supplies
  its pathname-bound title to the persistent breadcrumb without another request. Question and
  Properties editors show the same title, release status, and Edit Number. Ordered entries use
  visible order, separate descriptions, aligned labeled facts, and named action groups at desktop
  and narrow widths. Temporary 390-pixel inspection and the canonical HTTPS Assessment release
  journey passed; the full production-browser suite also passed. Temporary images were removed,
  and this receipt does not refresh or approve the historical screenshot corpus.

### I07: search results repeat the title and fragment actions

- Priority: medium; evidence:
  [question_library_filtered.png](../../screenshots/instructor/question_library_filtered.png).
- One result has a title at the left and repeats it on a lower reference line. Open, reference,
  and Copy reference occupy loosely connected positions. The bulk-selection panel is nearly as
  tall as the result itself.
- Acceptance: give each result one clear title and an aligned metadata/action arrangement. Keep
  bulk-selection scope accurate while presenting routine controls and explanations compactly.
- Current receipt (2026-09-16): `CopyableQuestionId` in
  `src/components/copyable_question_id.tsx` now accepts `presentation="compact"`; only the
  `LibraryPage` result-row call in `src/pages/library_page.tsx` uses it. The row's existing `h2`
  remains the one visible Question title, while normalized public reference, Copy reference,
  polite copy status, selection, and Open question remain in their established components. The
  detailed default remains available to other callers.
- Scope: `updateSelection`, `selectLoadedQuestions`, and `openMetadataEditor` retain the existing
  bulk-ID workflow. It remains additional to per-row selection and Open question inspection, and
  its affected-loaded-Questions count, limit, and help text are not a connected bulk-operation
  acceptance claim. The source review records no changes to virtualization, search, pagination, or
  return-state symbols. Supplementary isolated-component proof and review:
  `/private/tmp/ple-library-compact-reference-20260916.md` and
  `/private/tmp/ple-library-reference-review-20260916.md` (reported TypeScript, Prettier, and
  scoped diff checks). This is actual-component evidence, not an authenticated/deployed/full
  Library UI closure.

### I08: Browse starts with an unexplained empty Subject region

- Priority: medium; evidence:
  [question_library_browse.png](../../screenshots/instructor/question_library_browse.png).
- The page promises navigation from broad subject to topic, but Subjects is an empty tall panel.
  Tags and Question Types are lists of individually boxed entries; actual Question results are
  outside the captured viewport.
- Acceptance: explain an empty classification state and provide usable discovery paths. Keep
  facets compact enough to expose relevant results. Establish whether missing Subject data or
  rendering causes the blank region before assigning a correction.
- Accepted bounded correction (2026-09-17): a 33-line, three-file frontend change gives the
  Subject region explicit loading, ready, empty, and error states. A settled zero-Subject result
  explains that no Subject categories are available and directs discovery through Tags, Question
  Types, and Search; ordinary facets, cascade behavior, keyboard use, and detail-return state are
  unchanged.
- The region is top-aligned, compact, and scrollable at `12rem`. A temporary actual-component
  Chromium proof passed at 1280 and 390 pixels without horizontal overflow; its harness was
  removed. Strict TypeScript, 51 focused Library/Ribbon Node tests, ESLint, scoped Prettier and
  diff checks passed, and independent review approved the bounded correction.
- Official screenshots and authenticated connected Library acceptance remain deferred. This does
  not claim global density-row completion or an SQL lock.

### I09: picker and Pool review obscure the selection task

- Priority: medium; evidence:
  [blueprint_question_picker.png](../../screenshots/instructor/blueprint_question_picker.png) and
  [question_pool_creation_review.png](../../screenshots/instructor/question_pool_creation_review.png).
- The picker spends much of its height on introduction and filters before results. Pool review
  remains under a Search Question Library heading and a large empty search panel. Its attestation
  says Questions are "interchangeable" with little explanation of that educational decision.
- Acceptance: keep selected Questions, selection order, and the next action easy to find. Give
  Pool review its own clear task identity and briefly explain the interchangeability requirement.
  Preserve search/filter access and verify selection persistence, inspection, and keyboard flow.
- Source corrected: review/create now has a focused primary Pool heading, compact ordered selection
  and metadata, an educational interchangeability explanation, and no competing Library chrome.
  The existing picker remains the search/filter access. Isolated actual-component plus parent-host
  proof at 1280px verifies focus, ordered selection, Title/Description retention, required-label
  widths, and no horizontal overflow; it is not an authenticated full `LibraryPage` mount.
  Connected browser verification of selection persistence, inspection, and keyboard flow remains
  pending. Receipt: `/private/tmp/ple-classification-search-pool-receipt-20260916.md`.

### I10: Blueprint state and availability messaging need reconciliation

- Priority: medium; evidence:
  [blueprint_courses.png](../../screenshots/instructor/blueprint_courses.png) and
  [blueprint_course_detail.png](../../screenshots/instructor/blueprint_course_detail.png).
- My Blueprint Courses labels its collection Available Blueprint Courses and discusses every
  active Instructor. The detail prominently presents Return to Private and Archive before the
  captured editing structure. Return to Private describes a server condition rather than plainly
  reporting eligibility; the list shows one adoption for the same named Blueprint.
- Acceptance: identify ownership, visibility, and action eligibility clearly on the current object.
  Keep structure editing and adoption easy to find, with consequential availability actions grouped
  separately. Verify same-object state and server authorization before claiming an eligibility bug.
- Current receipt (2026-09-16): the Blueprint detail now places its server-authorized Adopt or Edit
  action and reusable Blueprint Assessment structure before history, forks, proposals, comparison,
  and lifecycle administration. The lifecycle predicates and actions are unchanged; only their
  teaching-task versus administration placement differs. Focused Blueprint tests passed, and a
  temporary browser walkthrough on the canonical HTTPS Live Demo verified the populated DOM order,
  owner edit toggle, adoption target, long-title layout, and 1280px/640px presentation. The empty
  screenshot in that temporary receipt is layout-only evidence, not a claim that an empty Blueprint
  is reachable. This closes the reported hierarchy defect but not same-object lifecycle-state or
  authorization acceptance for I10.

### I11: setup and Template forms consume excess laptop height

- Priority: medium; evidence:
  [course_list.png](../../screenshots/instructor/course_list.png),
  [assignment_creation.png](../../screenshots/instructor/assignment_creation.png), and
  [assessment_template_editor.png](../../screenshots/instructor/assessment_template_editor.png).
- Course creation dominates the Course list; three Assessment creation choices occupy a large
  nested panel; Template editing nests multiple rounded surfaces. Template duration is labeled
  in seconds, and changing Type explains an exception through a long helper sentence.
- Acceptance: keep collection scanning and the selected form task visible together where useful.
  Use familiar duration units and concise, accurate constraints. Fresh Template capture is required
  by the manifest; evaluate the current form before retaining any layout-specific conclusion.
- Current receipt (2026-09-16): `TeachingCourseListPage` in
  `src/pages/course_list_page.tsx` uses `isCreationExpanded` and the native Create Course Instance
  disclosure to keep a populated Course collection visible before opening its existing form.
  `createCourseInstance` retains classification, empty/adopted-source, validation, duplicate-request
  guard, error, reset, and post-success focus behavior. `src/pages/course_list_page.css` owns the
  focused hidden/disclosure and responsive form rules; list loading and Try again remain outside it.
- Scope: the reported actual-component matrix covers closed/empty/adopted disclosure states,
  draft/error retention, pending submit prevention, retry, keyboard activation, 200% text, and
  no document horizontal overflow. Supplementary proof and independent source review are
  `/private/tmp/ple-course-create-disclosure-20260916.md` and
  `/private/tmp/ple-course-disclosure-review-20260916.md`; they report scoped TypeScript, ESLint,
  Prettier, source-line, and diff checks. `src/style.css` fixes `color-scheme: light` and its
  `--ple-surface`/`--ple-ink` light tokens under both browser preferences, so this is not a dark
  implementation or distinct-dark acceptance. It is actual-component evidence, not connected,
  deployed, or full-UI closure.
- Current Assessment Template receipt (2026-09-16): an empty Template collection exposes creation
  automatically. Once Templates exist, the collection remains first and creation collapses behind
  a keyboard-operable disclosure; successful creation collapses the form and selects the new
  Template. The Type constraint is now concise, while the existing minute-based duration,
  validation, save, error, conflict, and unsaved-change behavior remains intact. Strict TypeScript,
  focused client tests, formatting, diff checks, and temporary actual-component Chromium proofs at
  1280px and 390px passed. Receipt: `/private/tmp/ple-template-hierarchy-20260916.md`. Connected
  canonical Live Demo acceptance and the fresh manifest screenshot remain pending.

### I12: empty Due Soon guidance lacks a direct next step

- Priority: medium; evidence:
  [assignments_due_soon_empty.png](../../screenshots/instructor/assignments_due_soon_empty.png).
- The page clearly states its seven-day window, but instructs the Instructor to set a due date
  without a nearby route into the relevant Course or Assessment workflow.
- Acceptance: offer a useful path to manage Coursework when the collection is empty, while
  retaining the time-window explanation. Verify wording for Courses with dates outside that window.
- Receipt (2026-09-16): bounded connected acceptance on canonical local runtime `8258` rendered
  the honest empty state with its next-seven-days/Account-time-zone explanation and `Manage
Coursework`; keyboard Enter reached the admitted Instructor Course-management destination. A
  real Course Assessment deadline more than seven days out remained outside the empty page's
  window. This supersedes isolated-only status for that empty-state path, not all dates, routes,
  errors, or mobile keyboard coverage. Receipts:
  `/private/tmp/ple-rebuilt-roster-ui-acceptance-20260916.md` and
  `/private/tmp/ple-due-soon-empty-next-action-20260916.md`.

### I13: Gradebook recognition is based on IDs

- Priority: high; evidence: [gradebook.png](../../screenshots/instructor/gradebook.png).
- Rows use roster IDs and the same Assessment reference rather than Student names and an
  Assessment title. Introductory copy emphasizes immutable evidence and excluded implementation
  data rather than the Instructor's result-review task.
- Acceptance: show recognizable authorized roster names and Coursework titles, with IDs where
  useful. Explain result status in teaching language. Verify accurate not-started, active, and scored
  rows; establish the available name data before proposing new data collection.
- Receipt (2026-09-16): bounded connected acceptance on canonical local runtime `8258` used the
  ordinary Instructor demo path for CSV import and correction of required private Course-local
  roster names. The same roster ID and existing scored Work remained after correction. The live
  Gradebook showed roster names and Assessment titles as primary 1280-pixel text, supporting IDs
  smaller, and matched its real Not started, In progress, and Completed and scored rows. This
  supersedes isolated-only status for these connected paths, not a full Gradebook/UI/mobile,
  security, retention, or Human Guidance closure. Receipts:
  `/private/tmp/ple-rebuilt-roster-ui-acceptance-20260916.md`,
  `/private/tmp/ple-roster-name-backend-20260916.md`, and
  `/private/tmp/ple-roster-name-ui-20260916.md`.

### I14: generated Question wording joins separate words

- Priority: high; evidence:
  [webwork_chromosome_shapes.png](../../screenshots/instructor/webwork_chromosome_shapes.png)
  shows "chromosomemost", "chromosomewith", and "centromeresituated" at colored-text boundaries.
- Acceptance: preserve word spacing across markup boundaries in generated prompts and choices.
  Check the generator and rendered Backend output to locate ownership; inspect related examples.
  This is visible text evidence, independent of scientific-content correctness.
- Receipt (2026-09-16): both canonical website PGML artifacts and their PLE delivery copies restore
  only spaces after colored spans in `topic09-chromosome-shapes-matching` and
  `topic09-chromosome-shapes-which-one`; `pg_sha256` now identifies corrected local bytes while
  the immutable committed canonical URL/hash pins remain unchanged. Renderer lint, visible-text,
  visual inspection, correct/wrong/partial matching grading, and seeded output checks passed; the
  independent review accepted the bounded scope. No
  generator, vendored WeBWorK/renderer, PLE demo, or official screenshot refresh changed. A future
  upstream regeneration can overwrite the artifact-only correction absent separately authorized
  producer repair. Receipt: `/private/tmp/ple-chromosome-spacing-receipts-20260916.md`.

### I15: preview height and control styling vary across surfaces

- Priority: medium; evidence:
  [webwork_dna_structure.png](../../screenshots/instructor/webwork_dna_structure.png),
  [webwork_hla_genotype.png](../../screenshots/instructor/webwork_hla_genotype.png), and
  [webwork_monohybrid_matching.png](../../screenshots/instructor/webwork_monohybrid_matching.png).
- Short Questions occupy large blank preview regions. PLE-owned Star/Watch actions resemble
  default browser buttons, while other PLE actions have deliberate typography and styling.
- Acceptance: size previews for content and task, preserving correct Backend layout. Style
  PLE-owned controls consistently and verify focus and selected states. Distinguish Backend-owned
  controls from PLE controls when assigning changes.
- Implementation receipt (2026-09-17): Question Details and Instructor Student View now share one
  opaque WeBWorK preview frame. The frame begins at 160 px and accepts only a source-bound,
  safe 160--1200 px resize from its opaque renderer; the bridge measures visible form/top-level
  content plus the body's lower padding/border on load and with `ResizeObserver`, suppressing
  repeated messages. The old fixed 28 rem blank region is removed. Star and Watch now expose
  `aria-pressed`, selected styling, and a visible keyboard focus outline. This source receipt
  preserves the opaque sandbox and does not
  establish fresh connected screenshots or cross-question browser-height proof.
- Current delivery receipt (2026-09-17): a rebuilt canonical HTTPS Instructor workflow rendered a
  representative generated WeBWorK prompt, form, and focusable control in the intentional opaque
  sandbox with zero page errors. PLE CSS/bridge, renderer scripts, and the reviewed font loaded; the
  font carried anonymous wildcard CORS and cross-origin CORP. Parent DOM/storage remained denied,
  unapproved assets returned empty 404 responses, and no Student Work or grade route was requested.
  The former `frameElement` error is closed by omitting only the renderer's parent-telemetry loader
  from no-write previews; Student documents remain exact. Canonical HTTPS evidence measured the
  exact opaque source/origin/resize shape at 444 px with reachable no-overflow content, no Student
  Work or grade request, and keyboard Star/Watch `aria-pressed` toggle and restore. No screenshot
  refresh follows. Receipt: `/private/tmp/ple-connected-webwork-preview-receipt-20260916.md`.

### I16: Profile emphasizes avatar descriptions over account settings

- Priority: low; evidence: [profile_default.png](../../screenshots/instructor/profile_default.png).
- Time zone is displayed as text, while named avatar cards with visible descriptions dominate the
  viewport. The capture establishes neither a time-zone editing action nor the current avatar
  selection; it cannot prove those capabilities absent elsewhere.
- Acceptance: make current settings and available editing actions recognizable. Keep visual avatar
  selection efficient while retaining accessible descriptions. Check selected state and save feedback
  in the current Profile workflow.

### I17: Library sorting is not visible

- Priority: medium; user observation and evidence:
  [question_library_filtered.png](../../screenshots/instructor/question_library_filtered.png) and
  [question_library_browse.png](../../screenshots/instructor/question_library_browse.png) show no
  visible sort control. HG already requires Library sorting.
- Acceptance: expose the active order and a useful sort choice; preserve it through filters,
  pagination, and return from Question inspection. Verify equal values and missing metadata across
  several pages. Establish current route behavior before claiming functional absence.
- Current receipt (2026-09-16): Question Library now exposes Title (A-Z) as the default and Recently
  published as the alternate order whenever Browse or an active Search has results to order. The
  initial Search remains focused on entering a query. Both orders use required metadata and the
  public Question ID as a stable tie-break; versioned cursors bind the active order and reject
  changed-order continuation. The order survives filters, pagination, Browse-to-Search handoff,
  retry, and return from Question inspection. Malformed URL order state is rejected and recoverable
  without discarding valid filters. Focused Rust/Node contracts, TypeScript, lint/format, and
  temporary actual-component desktop/390px proofs passed. Fresh canonical HTTPS Live Demo proof
  then verified initial-Search simplicity, Browse and active-Search controls, descending publication
  order across 50 records, Backend-filter preservation, browser Back, and visible return navigation.
  The seed had no equal-timestamp pair and no next cursor, so durable focused tests remain the
  evidence for the tie-break and cursor behavior. Fresh official screenshots remain pending.

## Classification implementation questions

HG owns the approved hierarchy, editing authority, and name-validation requirements. This audit
retains the implementation questions rather than repeating those requirements:

- Define the occasional Discipline management workflow and remaining lower-level editing permissions.
- Define exact name limits, length units, and detailed formatting rules, including internal whitespace,
  capitalization, punctuation, and duplicate handling.
- Resolve cross-disciplinary classification and how existing metadata maps to the approved hierarchy.
- Verify consistent validation in editing and import paths, including boundary lengths, surrounding
  whitespace, and whitespace-only names.
- Demonstrate usable classification selection with long vocabularies, keyboard access, and clear context.

## Instructor color spot checks and retained strengths

Five solid colored-text values extracted from the chromosome-shape Question crop measure against
its white background as follows: `#A719DB` 5.52:1, `#B74300` 5.50:1, `#C80085` 5.53:1,
`#0067CC` 5.51:1, and `#D40000` 5.53:1. All pass the 4.5:1 normal-text floor. The bright appearance
alone supports no failure claim. These image-derived pairs are spot checks; computed colors and
all relevant states remain the closure evidence. Contrast compliance also leaves word spacing,
hierarchy, and semantic success/error styling as separate concerns.

Retain the useful distinctions already visible: separate Question and Properties editors,
answer-free Student View labeling, explicit generated-example labeling, visible Pool
interchangeability attestation, and a clearly stated Due Soon window. These are presentation
strengths, not proof of the corresponding runtime guarantees.

## ADAPT comparison lessons

[ADAPT_UI_AUDIT.md](ADAPT_UI_AUDIT.md) supplies the observations behind this comparison.
Most lessons reinforce existing HG. Use them to sharpen implementation acceptance checks;
the two suggested HG additions below remain proposals.

| ADAPT lesson                                    | Apply to this usability audit                                                                                      | HG treatment                                                                                                                  |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| Aligned tables and compact row actions          | Strengthen acceptance checks for drafts, Courses, Templates, and Library results.                                  | Already covered by density and alignment guidance.                                                                            |
| Optional descriptions                           | Evaluate a description toggle or disclosure for long result lists.                                                 | Already covered by progressive disclosure.                                                                                    |
| Scoped bulk editing                             | Check that users can identify affected records and proposed changes before applying them.                          | Proposed addition: Bulk actions should make their affected scope and intended changes clear before application.               |
| Sort indicators, pagination, page-size controls | Expand sorting checks to include visible order and preserved navigation state.                                     | Sorting is already required.                                                                                                  |
| Folder counts and selected context              | Check whether existing classification, Starred, and Watched paths communicate useful counts and current selection. | Evaluate these existing paths before proposing folders.                                                                       |
| Separate content and properties                 | Check that each editor keeps its task clear and teaching content prominent.                                        | Already covered.                                                                                                              |
| Long, inconsistent classification menus         | Check narrowing of large vocabularies and recognition of selected terms.                                           | Proposed addition: Classification controls should help users narrow large vocabularies and keep the selected hierarchy clear. |

ADAPT's tall filter forms, nested scrolling, and paragraph-heavy rows provide comparison cautions.
Evaluate their task cost rather than copying them as established solutions.

## Blueprint catalog opportunities

Status: catalog comparison directions; Sysadmin-only searchable boolean promotion approved in HG,
with implementation and rendered verification pending.

The user favors ADAPT's Public Courses, Commons, and Frameworks catalog setup as a possible
reference for Blueprint Courses. [ADAPT_UI_AUDIT.md](ADAPT_UI_AUDIT.md) records their visible
structure and inconsistent introductory explanations.

- Evaluate a consistent Blueprint discovery page with a brief purpose statement, compact filters,
  comparable rows, useful provenance, and clear inspect/adopt actions. Explain reusable course
  design and adoption in PLE language; preserve HG's public search and Blueprint lifecycle rules.
- Separate catalog meaning from search mechanics. Put a short definition near the heading and
  use contextual help for detailed search examples.
- The user describes the existing approach as a bazaar, with Stars and Watches. HG now adds a
  searchable boolean Promoted flag on Blueprint Courses, controlled exclusively by Sysadmins,
  as another discovery path. Verify actual promotion and Star/Watch capabilities before claiming
  they are implemented.
- Verify Sysadmin changes and Instructor search of the flag, including refusal of Instructor
  mutation. Keep promotion distinct from popularity, personal Stars, Watches, and verified
  educational quality. Check how discovery respects ordinary Blueprint visibility and archival rules.
- Acceptance direction: Instructors should understand why a Blueprint is highlighted, inspect its
  structure and provenance, and adopt it through the ordinary permitted workflow. Evaluate promoted
  and ordinary records together with long titles, empty results, and available search/sort controls.

The comparison concerns Blueprint discovery. HG owns the approved promotion rule; this audit owns
its implementation questions and acceptance evidence.

## Instructor verification sequence

1. Reconcile each relevant file with the capture manifest and reproduce on current admitted routes.
2. Prioritize broken reading boundaries, misleading outcome cues, object recognition, and next actions.
3. Walk find -> inspect -> author/adopt -> assemble/release -> roster/results on a 1280 by 800 laptop.
4. Check long titles, empty and populated lists, saved and failed forms, and permitted/restricted actions.
5. Verify keyboard focus, accessible labels, computed-color contrast, and light/dark presentation.
6. Record fresh screenshot and task-completion evidence beside each finding before closing it.
