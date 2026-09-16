# UI, UX, and usability audit

Date: 2026-09-16. Status: open findings from screenshot inspection.

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

Screenshots establish visible presentation at capture time. Live behavior, keyboard traversal,
accessible names, touch interaction, and current source remain verification work. Findings below
remain open until the corresponding rendered and interaction checks pass. No runtime audit or
participant study was conducted.

## Review coverage

- Student: initial screenshot findings recorded below; interaction checks remain open.
- Instructor: all 32 current PNG files inspected during this session; findings below concern
  visible laptop presentation, with fresh-capture and interaction checks still open.
- Sysadmin: laptop review findings can be added as the review expands.
- Public: entry and authentication surface findings can be added with their applicable screen targets.

## State and task clarity

### Student Question rendering coverage

Status: requested; captures and interaction evidence pending.

Review Student-facing delivery of one WeBWorK Question and each of the eight native PLE JSON
Question Types specified in [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md). Instructor Library
previews supplement this evidence; Student delivery captures establish the requested presentation.

| Example | Required visible content |
| --- | --- |
| WeBWorK | Backend-rendered prompt, response controls, and permitted feedback |
| MC | Single-answer choice selection |
| MA | Multiple-answer choice selection |
| FIB | One text-entry blank |
| MULTI-FIB | Multiple independently identifiable text-entry blanks |
| NUM | Numeric response and any relevant unit or tolerance instructions |
| MATCH | Prompts, shared choice bank, and visible assigned matches |
| ORDER | Items and their current ordering |
| HOTSPOT | Static visual asset and visible selection cues |

Capture representative examples on a laptop and narrow phone, with portrait-tablet and square
layout checks where the response arrangement changes. Inspect initial and entered-response states,
saved state, and permitted submitted/review feedback. Include long content where it tests wrapping
or alignment. Use meaningful teaching examples rather than empty controls.

Walk each response through keyboard-only interaction and pointer or touch as applicable. For MATCH
and ORDER, demonstrate assigning or moving, correcting, and clearing responses through the supported
methods. Verify focus, prompt/control association, saved-response persistence, and contrast against
actual rendered backgrounds. Record each Type separately, including unavailable Types as explicit
coverage gaps. Screenshots establish presentation; interaction receipts establish usable behavior.

### S01: accepted invitation retains pending-state actions

- Evidence: [invitation_accepted_laptop.png](../../screenshots/student/invitation_accepted_laptop.png)
  shows "Invitation accepted" alongside "Join this course", pending-state instructions, and an
  "Accept invitation" action.
- User need: recognize successful enrollment and proceed into the Course.
- Acceptance: after acceptance, show a coherent joined state and make opening the Course or
  Coursework the next primary action. Verify the transition in a live walkthrough.

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

### S04: Attempt history follows a tall settings summary

- Evidence: [assignment_overview_history_laptop.png](../../screenshots/student/assignment_overview_history_laptop.png)
  places "Previous attempts" at the bottom of the captured viewport after vertically stacked rules.
- User need: find recorded Attempts and results as readily as the next available work action.
- Acceptance: make previous Attempts easy to locate alongside a compact overview. Check laptop and
  narrow Student layouts with several Attempts and long titles.

## Layout and recognition

### S05: settings dominate Coursework entries and overviews

- Evidence: [course_completed_laptop.png](../../screenshots/student/course_completed_laptop.png)
  repeats due-time and time-zone information in a tall access section;
  [assignment_overview_unanswered_laptop.png](../../screenshots/student/assignment_overview_unanswered_laptop.png)
  stacks short labels and indented values across most of the viewport.
- Acceptance: use compact summaries with visually connected labels and values. Keep essential
  information and actions visible; disclose fuller rules through accessible expandable details.
  Check that Students can scan several Coursework entries and locate the start action promptly.

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
It also marks several future Ribbon destinations as deferred. File presence and filenames alone
establish neither current route behavior nor successful task completion. Reproduce findings on
the admitted interface before making implementation changes. Preserve useful teaching behavior
while correcting its presentation.

### I01: draft records concatenate distinct information

- Priority: high; finding visible in
  [question_drafts.png](../../screenshots/instructor/question_drafts.png).
- Titles run directly into descriptions and draft metadata. Repeated Delete actions appear on
  their own lines, while the title and description share one long link.
- Acceptance: use compact aligned records with distinct title, description, state, and actions.
  Keep opening or continuing a draft the clear routine action. Check long titles and descriptions.

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

### I07: search results repeat the title and fragment actions

- Priority: medium; evidence:
  [question_library_filtered.png](../../screenshots/instructor/question_library_filtered.png).
- One result has a title at the left and repeats it on a lower reference line. Open, reference,
  and Copy reference occupy loosely connected positions. The bulk-selection panel is nearly as
  tall as the result itself.
- Acceptance: give each result one clear title and an aligned metadata/action arrangement. Keep
  bulk-selection scope accurate while presenting routine controls and explanations compactly.

### I08: Browse starts with an unexplained empty Subject region

- Priority: medium; evidence:
  [question_library_browse.png](../../screenshots/instructor/question_library_browse.png).
- The page promises navigation from broad subject to topic, but Subjects is an empty tall panel.
  Tags and Question Types are lists of individually boxed entries; actual Question results are
  outside the captured viewport.
- Acceptance: explain an empty classification state and provide usable discovery paths. Keep
  facets compact enough to expose relevant results. Establish whether missing Subject data or
  rendering causes the blank region before assigning a correction.

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

### I12: empty Due Soon guidance lacks a direct next step

- Priority: medium; evidence:
  [assignments_due_soon_empty.png](../../screenshots/instructor/assignments_due_soon_empty.png).
- The page clearly states its seven-day window, but instructs the Instructor to set a due date
  without a nearby route into the relevant Course or Assessment workflow.
- Acceptance: offer a useful path to manage Coursework when the collection is empty, while
  retaining the time-window explanation. Verify wording for Courses with dates outside that window.

### I13: Gradebook recognition is based on IDs

- Priority: high; evidence: [gradebook.png](../../screenshots/instructor/gradebook.png).
- Rows use roster IDs and the same Assessment reference rather than Student names and an
  Assessment title. Introductory copy emphasizes immutable evidence and excluded implementation
  data rather than the Instructor's result-review task.
- Acceptance: show recognizable authorized roster names and Coursework titles, with IDs where
  useful. Explain result status in teaching language. Verify accurate not-started, active, and scored
  rows; establish the available name data before proposing new data collection.

### I14: generated Question wording joins separate words

- Priority: high; evidence:
  [webwork_chromosome_shapes.png](../../screenshots/instructor/webwork_chromosome_shapes.png)
  shows "chromosomemost", "chromosomewith", and "centromeresituated" at colored-text boundaries.
- Acceptance: preserve word spacing across markup boundaries in generated prompts and choices.
  Check the generator and rendered Backend output to locate ownership; inspect related examples.
  This is visible text evidence, independent of scientific-content correctness.

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
- Open detail: supported fields and defaults should follow actual metadata and task needs.

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

| ADAPT lesson | Apply to this usability audit | HG treatment |
| --- | --- | --- |
| Aligned tables and compact row actions | Strengthen acceptance checks for drafts, Courses, Templates, and Library results. | Already covered by density and alignment guidance. |
| Optional descriptions | Evaluate a description toggle or disclosure for long result lists. | Already covered by progressive disclosure. |
| Scoped bulk editing | Check that users can identify affected records and proposed changes before applying them. | Proposed addition: Bulk actions should make their affected scope and intended changes clear before application. |
| Sort indicators, pagination, page-size controls | Expand sorting checks to include visible order and preserved navigation state. | Sorting is already required. |
| Folder counts and selected context | Check whether existing classification, Starred, and Watched paths communicate useful counts and current selection. | Evaluate these existing paths before proposing folders. |
| Separate content and properties | Check that each editor keeps its task clear and teaching content prominent. | Already covered. |
| Long, inconsistent classification menus | Check narrowing of large vocabularies and recognition of selected terms. | Proposed addition: Classification controls should help users narrow large vocabularies and keep the selected hierarchy clear. |

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
