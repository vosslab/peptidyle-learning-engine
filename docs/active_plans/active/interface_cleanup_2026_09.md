# Plan: interface cleanup from the 2026-09-09 interface notes

<!-- Drafted with blueprint-plan-drafter. On approval, publish to
docs/active_plans/active/interface_cleanup_2026_09.md per docs/REPO_STYLE.md. -->

## Context

[docs/active_plans/2026-09-09-notes.txt](../2026-09-09-notes.txt)
records the owner's consolidated pass over the live interface, split by the rule that Student
sections describe what Students see and do while Instructor sections describe what Instructors see
and configure.

A three-agent read of the current surface shows the notes mix three different kinds of work, and
treating them as one list is the main risk to this cleanup:

1. **Real cleanup.** Density, vocabulary, labels, defaults, icons, breadcrumbs.
2. **Wiring what already exists.** Several notes items are already built and simply are not mounted
   on the path that uses them, or are mounted twice. `src/pages/assignment_attempt_page.tsx`
   already delivers one question per page, a `role="timer"` countdown, and a `Question n of m`
   position, but its Ribbon capability is declared unbacked and the live Student path never reaches
   it. `src/components/student_assignment_presentation.tsx` already renders every fact the notes
   want on the Start Assignment page, and its only consumer is the Instructor preview.
   `src/pages/course_assignments_page.tsx` is a complete themed paginated Assignment list that
   nothing imports.
3. **New product capability.** Three notes items have no data behind them at all: Active versus
   Inactive Courses (no course status exists in the browser or the read model), public Blueprint
   Course search, and cross-course Assignments Due Soon. Instructor Profile preferences likewise
   have no record: `InstructorAccountSummary` carries reference, state, and last sign-in only, and
   every time zone in the product is Course-owned.

Document ownership also needed a decision, and the owner has now given it: only
[docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and
[docs/TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md)
are human-approved, and `docs/INTERFACE_TERMINOLOGY.md` should hold terminology rather than
Ribbon-level structural restrictions. That removes the block on the notes' Ribbon shape and makes
the first task a documentation ownership repair rather than a negotiation.

## Objectives

- Deliver the Student Attempt surface the notes describe: one question per page, a navigation bar
  showing whether each question's response is saved, and a subtle timer that does not raise
  anxiety.
- Remove Instructor-only vocabulary and typed Course Instance references from Student-visible text.
- Give the Start Assignment page its four facts plus previous attempts, under the Instructor's
  answer-disclosure setting.
- Retire the duplicate Instructor Assignment surface so one editing path exists.
- Implement the settled Instructor Ribbon: three tabs, icons, the settled task rows, breadcrumbs.
- Raise Instructor list density to spreadsheet scanning and show Course identity in list rows.
- Apply the notes' Assignment Settings defaults and separate the results-disclosure controls.
- Give the Instructor one configured IANA zone that owns wall-clock entry and display, and move the
  profile image into Profile preferences.
- Keep the three capability-shaped notes items visible as scoped follow-on work rather than
  half-implementing them inside a cleanup.

## Design philosophy

Prefer wiring and retiring over building. Two of the three largest notes items already exist in the
tree and are reachable only from the wrong place, so the first move on each is to mount or delete,
not to write a new component. That is **fix the design, not the symptom**: adding a timer and a
navigation bar to `assignment_overview_page.tsx` while `assignment_attempt_page.tsx` already has
both would create a second Student delivery lane, exactly the duplication the Assignment Workspace
pages are already suffering from.

The plan therefore front-loads one serial documentation and retirement gate, then runs Instructor
chrome and Student delivery as independent lanes. The accepted trade-off is that M0 and M1 delay
Instructor work by two bounded packages. The rejected alternative was implementing the notes
page-by-page and reconciling afterward; `docs/ux/RIBBON_RETIREMENT_RESPONSIBILITY_INVENTORY.md`
and the six-pass Course Appearance audit are the local evidence that reconciliation-after-the-fact
costs more.

Capability-shaped items are admitted only when their cost is measured, not assumed. Adding a Course
Active/Inactive status to satisfy a task-row label would put a schema decision inside a visual
change, against **atomic task decomposition**. Assignments Due Soon is different: the owner wants
it, and whether it is aggregation over already-queryable fields or new domain design is a question
the repository can answer, so the plan measures before classifying it.

State ownership positively. Acceptance criteria name who owns a thing rather than who may not
touch it, per **prompt positively** in `docs/REPO_STYLE.md`: "Question presentation owns
answer-choice randomization" dispatches more reliably than a prohibition on Assignment controls.
Security and privacy boundaries keep their explicit exclusions, because there the excluded thing is
the requirement.

Keep it simple, aggressively. Prefer the smallest coherent design that satisfies the actual
requirement and the failure modes the repository actually shows. Mechanisms, abstractions,
policies, state, and tests earn their place by solving a demonstrated need. Two consequences run
through this plan: Assignments Due Soon states its requirement and leaves the shape to its
implementer rather than prescribing SQL, envelopes, and indexes; and the time-zone work settles on
one owner for wall-clock presentation instead of building an override hierarchy that would become
permanent cruft.

The notes' Blackboard Ultra section is mixed material and the plan treats it that way. Four of its
entries are now explicit owner decisions and appear under resolved decisions; the remainder stays
reference. Implementing a Blackboard setting because it appears in the notes would ship product
policy the owner never chose, so every settings field in M4 traces to a named owner decision.

Two items turned out to be domain-model questions wearing UI clothing: the time-zone ownership the
notes question, and whether a Course Instance Assignment may diverge from its Blueprint parent.
Both were audited before planning rather than after, per **use the scientific method**. Storage
turned out to be correct and divergence turned out to be unconstrained, so neither becomes a
rewrite; what remains is recorded as a decision in M0.

- Evidence strategy for uncertain methods: density, timer prominence, and navigation-bar legibility
  are visual questions. Each carries browser evidence at the canonical profiles in
  [docs/UI_DESIGN_GUIDE.md](../../UI_DESIGN_GUIDE.md)
  (1280 by 800 Instructor; 1280 by 800 and 800 by 1280 Student; narrow-phone guard) plus
  `image_evaluator` review, rather than a pixel equivalence gate, per HUMAN_GUIDANCE.md on avoiding
  arbitrary numeric gates.

## Scope

- Move Ribbon structural rules out of `docs/INTERFACE_TERMINOLOGY.md` into `docs/UI_DESIGN_GUIDE.md`
  and record the settled Ribbon shape and Assignment surface names in `docs/DESIGN_DECISIONS.md`.
- Retire `src/pages/assignment_release_page.tsx` as a route target and resolve the orphan
  `src/pages/course_assignments_page.tsx`.
- Repair the two broken `/access` links and open Assignment Preview in a new browser tab.
- Implement the settled tabs, task rows, sprite glyphs, and breadcrumbs for deep routes.
- Convert Instructor Course and Assignment lists to dense rows and show Course Theme identity.
- Add inline Assignment Title and Due Date editing to the Course Assignment list.
- Apply the notes' Assignment Settings defaults, surface `Randomize question order` as the
  Assignment setting, and place answer-choice randomization on the Question where it belongs.
- Promote the one-question Attempt lane to the live Student path with a question navigation bar,
  per-question saved state, and the subtle timer.
- Mount the existing answer-free assignment facts component on the Student Start Assignment page
  and extend `LiveAssignmentAccess` with the facts and previous-attempt history it needs.
- Auto-enter the single active Course for a Student enrolled in exactly one.
- Remove Question Titles, Released Assignment status, and `C-n`-style references from
  Student-visible content.
- Replace Course-owned wall-clock time with account-owned zones: an Instructor zone that interprets
  entered wall-clock values and drives Instructor display, a Student zone that drives Student
  display and defaults from the Instructor's at enrollment, a zone-free local date-and-time input
  DTO, and retirement of the Course zone field and `CourseLocalDateAndTime`.
- Repair the three display sites that fall back to the browser's zone, and name the governing zone
  on the due-date editor.
- Add Instructor Profile preferences with that IANA zone and a cropped thumbnail presented through a
  consistent rounded-rectangle silhouette.
- Represent Active and Inactive Courses, where Inactive means a previous-semester Course with
  FERPA-sensitive Student data stripped and non-sensitive metadata retained, and surface both as
  Courses task-row destinations.
- Build a cross-course Assignments Due Soon view: upcoming Assignments from the Courses the
  Instructor currently teaches, with Course identity and due time visible, usefully bounded by due
  time. Use the smallest existing data-access and routing pattern that satisfies that.
- Add axe coverage for the Student surfaces this plan changes and refresh the affected screenshot
  corpus entries.
- Record accepted evidence in `docs/CHANGELOG.md`.

## Non-goals

- Build the three Genetics assignments from `~/nsh/PROBLEMS/biology-problems-website/site_docs/genetics/topic01-03/`.
  That is content authoring with its own point and timing rules (2 minutes per question, matching
  2 points, multiple choice 1 point); route it to a separate content plan.
- Introduce public Blueprint Course search. It has no data behind it; this plan reserves its
  task-row position and leaves it Unavailable.
- Back the Starred, Watched, and My Questions destinations. They keep their reserved future
  positions in the settled task row.
- Add a learning-tree or branching assignment block. The notes explicitly hold that back.
- Implement Blackboard Ultra's full settings inventory. Most of that section is reference material.
  The owner has since promoted four of its entries to PLE requirements, listed under resolved
  decisions; the rest stay reference. Final grade calculation is omitted unless the current grading
  model already needs it.
- Change grading, answer secrecy, authorization, or retention. Every route keeps its exact
  authorization path, and disclosure defaults change only where the notes name them.
- Add Student upload capability of any kind.
- Port interface work to Rust and Wasm for speed. Wasm participation stays inside the existing
  `crates/wasm` attempt-timing export the timer already uses.
- Redesign the Course Appearance theme or banner contract.

## Current state summary

Frontend is SolidJS in `src/` with a frozen route table at `src/route_contract.ts:90-396`, mapped
to components in `src/routes.ts:44-83`. Routing and the Ribbon are entirely TypeScript. The Rust
server is JSON-only; `crates/wasm` exports deterministic answer-free work including attempt timing.

Instructor findings:

- One Course index at `/` renders cards, not rows: `.card-grid` plus `.course-card` at
  `src/pages/course_list_page.tsx:254-256` and `src/style.css:289-304`. No theme value appears in a
  row, and `/` is Product scope, so no course theme resolves there at all.
- No Course status exists anywhere in the browser; `archived` is an Assignment status only.
- Two live surfaces are both titled "Assignment Workspace". The five section routes
  (`overview`, `questions`, `policies`, `student-view`, `grading-operations`) at
  `src/route_contract.ts:276-336` are the split the owner remembers making. The older
  `src/pages/assignment_release_page.tsx` still exists, holds `Validate Assignment` (line 316) and
  `Open Assignment Preview` (line 324), and is the page `src/pages/course_instance_page.tsx:36`
  actually links to.
- `src/pages/course_assignments_page.tsx` is a complete themed paginated Assignment list with
  per-row links, imported by nothing.
- Assignment overview and policies both link to `/access`, which is not in `ROUTE_CONTRACT` and
  falls through to Not Found.
- No inline editing exists anywhere; Assignment Title is editable only on the questions page and
  Due Date only on the policies page.
- Disclosure already has six independent controls with five timings
  (`crates/question_model/src/assignment_activity_rules.rs:47-77`), which is close to the notes'
  request. Randomization exists as question order, pool reuse, and variation; per-choice shuffle is
  absent. Late work defaults to Accept, and due, close, time limit, and attempt limit all default
  to `None`.
- No Instructor profile or preferences page, and no Ribbon destination for one.

Student findings:

- The live path renders many questions per page: a `For` over `current().questions` at
  `src/pages/assignment_overview_page.tsx:288-350`. The single-question route is the same list
  narrowed by nonce.
- The unbacked attempt lane at `src/pages/assignment_attempt_page.tsx` already has one question per
  page, a `role="timer"` countdown at lines 517-522, and `Question n of m` at 523-530. Its Ribbon
  capability is declared unbacked at `src/ribbon/capability_registry.ts:272-279`.
- No question navigation bar and no per-question answered indicator exist on any Student route.
- The Start Assignment page hard-codes `<h1>Assignment</h1>` and shows one status sentence and a
  button. `LiveAssignmentAccess` carries only `startDecision`
  (`src/api/assignment_attempt_issuance.ts:11-14`), so the notes' facts have no field to arrive in.
- Instructor vocabulary reaches Students in at least five places: `Course Instance {reference}` at
  `src/pages/student_courses_page.tsx:12` and `student_course_invitations_page.tsx:14`,
  `Released Assignment` at `student_course_landing_page.tsx:27` and `:109`, and the library Question
  Title at `assignment_overview_page.tsx:298`.
- `/` always renders the Student course list; there is no single-course auto-entry.
- The Attempt task row's only task targets `courseAssignments`, an Instructor-only route, masked
  today only because the capability is unbacked.
- A likely defect: the Student must press Start Assignment twice, because `issued()` is
  component-local signal state discarded on navigation (`assignment_overview_page.tsx:94`, `:133`).

Domain findings from the three follow-up audits:

- Time zones: storage is already the correct model. Every schedule, attempt, and submission column
  is `timestamptz`; no naive column exists; no migration performs zone arithmetic. The wall-clock
  wire type `CourseLocalDateAndTime` travels unconverted and the server resolves it in the Course
  zone at `crates/question_model/src/assignment/teaching_settings_local.rs:561`, refusing DST gaps
  and ambiguity rather than coercing them. What is per-Course rather than per-person is the
  *preference*, and no per-account preference storage exists at all: the account table's three
  columns are trigger-immutable. Three display sites fall back to the browser zone by omitting a
  `timeZone` option, so an activity time can disagree with a due time inches away. The wired
  due-date editor never names the zone the Instructor is typing in.
- Assignment state: an Assignment carries its current authored state plus an Assignment Edit
  Number, and release snapshots an immutable `assignment_revision`. The save path refuses any
  Assignment that is not `unreleased`, so today a released Assignment cannot be retimed at all and
  gets exactly one revision. That directly constrains the notes' inline due-date editing.
- Blueprint divergence: not blocked, and not even representable. Course Instance creation
  materializes no Assignments from the Blueprint, so an instance Assignment is entirely
  instance-owned with its own selection and revision sequence. The `assignment.source_blueprint_*`
  columns are the *Course's* origin revision stamped onto every Assignment, including ones typed
  from scratch, so they are not a per-assignment lineage. The divergence vocabulary in
  `crates/question_model/src/blueprint_operations/` has no persistence home.
- Assignments Due Soon: aggregation over existing data. `ple_data.assignment` already carries
  `course_id`, `due_at`, and `assignment_status`; the cross-course Instructor membership join
  already exists verbatim in `ple_api.list_live_demo_course_instances`; the authorization predicate
  is the set-valued form of the same `current_course_instructor`. No table, column, or predicate is
  new. The one plausible addition is an index on `ple_data.assignment (course_id, due_at)`.

Styling and evidence findings:

- Tokens live in `src/style.css:3-84`; the spacing scale is `--ple-space-1` through `-7` with
  partial adherence. Course theme derives 15 emitted properties from three anchors through
  `THEME_MIX` in `src/features/course_appearance/course_theme_registry.ts`.
- Font Awesome Free solid is already present as a build-time subsetted sprite with a closed
  16-glyph vocabulary in `src/ribbon/ribbon_icons.ts`, Ribbon-only, with a test forbidding CDN
  fetches. The notes' icon request is therefore glyph selection, not a new dependency.
- `--ple-theme-primary` is consumed at `src/style.css:537` on a Student-visible element but never
  emitted; `docs/TODO.md` already routes this to the theme-system plan.
- One axe test exists and it scopes an Instructor page. No Student surface has axe coverage, and
  `check_codebase.sh` runs no accessibility gate.

## Architecture boundaries and ownership

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M0 documentation | `docs/INTERFACE_TERMINOLOGY.md`, `docs/UI_DESIGN_GUIDE.md`, `docs/DESIGN_DECISIONS.md` | `architect` decision review |
| M1 retirement and surface split | `src/pages/assignment_release_page.tsx`, `course_instance_page.tsx`, `course_assignments_page.tsx`, `assignment_workspace_questions_page.tsx`, `src/route_contract.ts` | `reviewer` plus Playwright selector sweep; `image_evaluator` on the composition surface |
| M2 Ribbon and dense bar | `src/ribbon/*`, `src/application_shell.tsx`, `src/route_contract.ts`, `devel/build_ribbon_icon_sprite.mjs`, ledger generator | Ribbon ledger and e2e contract tests; `image_evaluator` at three profiles |
| M3 density | `src/pages/course_list_page.tsx`, `course_instance_page.tsx`, `src/style.css` | `image_evaluator` at the canonical profiles |
| M4 settings | `assignment_workspace_policies_page.tsx`, `assignment_workspace_policy_panel.tsx`, `crates/question_model/src/assignment*.rs`, `crates/question_model/` Question presentation and the Draft Question editor | `reviewer` plus focused Rust default tests; `reviewer` on the issuance path for answer order |
| M5 Student delivery | `assignment_attempt_page.tsx`, `assignment_overview_page.tsx`, `capability_registry.ts`, `crates/wasm` timing export | keyboard journey plus student axe |
| M6 vocabulary | `student_courses_page.tsx`, `student_course_landing_page.tsx`, `student_course_invitation*.tsx` | `reviewer` plus screenshot refresh |
| M7 Start facts | `src/api/assignment_attempt_issuance.ts`, `student_assignment_presentation.tsx`, server assignment access route | API contract review |
| M12 answer randomization | `crates/question_model/` Question presentation, Draft Question authoring and editor, issuance | `reviewer` on issuance and the issued binding |
| M13 account zones | `crates/learning-data-access/src/instructor_account.rs`, the Student Account and enrollment path, one forward migration | `architect` plus schema allocation |
| M14 wall-clock input | `crates/question_model/src/assignment/teaching_settings_local.rs`, the assignment Store paths, generated bindings | `reviewer`; existing DST refusal tests |
| M15 zone display | `student_assignment_presentation.tsx`, `gradebook_assignment_attempt_chooser.tsx`, `instructor_accounts_page.tsx`, the due-date editor | `reviewer` sweep of formatter call sites |
| M16 Course zone retirement | `schemas/migrations/`, `course_term.rs`, `course.rs`, the enumerated read sites | `architect` |
| M17-M18 Profile | new route and page, the existing upload and rendition boundary | `reviewer`; Course Banner evidence pattern |
| M9 evidence | `tests/playwright/*`, `docs/screenshots/*`, `docs/CHANGELOG.md` | `tester` |
| M10 Due Soon | the assignment Store and route family plus a new page, shape chosen by its owner | `reviewer` on the membership join |
| M19 Course activity | the existing retention model, the Course read model and Store, `course_list_page.tsx` | `architect` on the FERPA boundary |
| M11 inline editing | the surviving Course Assignment list, the assignment save path | `reviewer` on the relaxed status check |

## Milestone plan

Numbers are labels; `Depends on` is the order. The table is listed in dispatch order.

| M | Title | Depends on | Goal |
| --- | --- | --- | --- |
| M0 | Document ownership and settled decisions | none | One visible-name truth before code |
| M1 | One Assignment path, composition split from delivery | M0 | One Instructor editing path |
| M6 | Student vocabulary and course entry | none | Student-only language |
| M5 | Student one-question delivery | M0 | The notes' Student core |
| M12 | Question-owned answer randomization | M0 | Choice order lives with the choices |
| M13 | Account-owned time zones | M0 | Zones belong to people |
| M14 | Wall-clock input interpretation | M13 | A deadline is an instant |
| M15 | Account-zone display | M13 | Each reader sees their own clock |
| M2 | Settled Ribbon shape and dense top bar | M0, M1 | Notes' navigation, honestly backed |
| M3 | Instructor list density and identity | M0 | Spreadsheet scanning |
| M16 | Retire the Course zone | M14, M15 | One wall-clock concept |
| M17 | Instructor Profile page | M13, M2 | A place to own the zone |
| M18 | Profile thumbnail | M17 | Consistent identity image |
| M4 | Assignment settings defaults | M1, M13 | Defaults that match teaching |
| M11 | Inline Assignment title and due-date editing | M4, M3 | Retime from the list |
| M7 | Start Assignment facts | M5, M6 | Informed start decision |
| M10 | Assignments Due Soon | M2 | Answer "what needs attention" |
| M19 | Active and Inactive Courses | M0, M2, M3 | Previous semesters stay reachable |
| M9 | Evidence close-out | the milestones it captures | Accepted receipts |

### Milestone: M0 document ownership and settled names

- Depends on: none.
- Deliverables: `docs/INTERFACE_TERMINOLOGY.md` reduced to terminology; Ribbon structural rules
  moved into `docs/UI_DESIGN_GUIDE.md`; new `docs/DESIGN_DECISIONS.md` entries recording the
  settled tab set, the settled task rows, and the Assignment surface names; amendments to the
  existing `Product navigation exposes Questions through one library surface` and
  `The Application Shell owns one Ribbon for every Product Role` entries.
- Also records the three domain decisions the notes exposed: time-zone ownership, Assignment state
  without revision history, and the Blueprint provenance column's real meaning.
- Workstreams: WS-DOC.
- Entry criteria: none.
- Exit criteria: no document states a Ribbon tab set that another document contradicts; the three
  domain decisions are recorded with their audited file locations;
  `tests/test_markdown_links.py` passes.
- Parallel-plan ready: no. One owner writes all four documents so they cannot disagree.

#### M0 completion receipt (2026-09-10)

- Status: complete. `docs/INTERFACE_TERMINOLOGY.md`, `docs/UI_DESIGN_GUIDE.md`, and
  `docs/DESIGN_DECISIONS.md` now agree on terminology ownership, Ribbon structure, and the
  settled interface and domain decisions.
- Narrow gate: `source source_me.sh && python3 -m pytest tests/test_markdown_links.py` passed
  with 228 tests; `git diff --check` passed.
- Independent review: the M0 documentation re-review passed with no blocker, major, or minor
  plan-conformance finding.
- Remaining milestones: pending; this receipt records documentation decisions only and does not
  claim delivery of their product behavior.

### Milestone: M1 one Assignment path, composition separated from delivery

- Depends on: M0, because the retirement decision and the task split are recorded there.
- Deliverables: one Instructor Assignment editing path; a composition surface whose primary focus
  is selecting, adding, removing, and ordering Questions; a delivery surface holding timing,
  release, scoring, attempts, randomization, late-work, and disclosure; `course_instance_page.tsx`
  links pointed at the surviving routes; `/access` links repaired or removed; Assignment Preview
  opening in a new tab; the orphan list page resurrected or deleted; Playwright selectors and the
  screenshot corpus updated for any changed heading.
- Workstreams: WS-RETIRE, WS-COMPOSE.
- Entry criteria: M0 exit.
- Exit criteria: exactly one route renders each Assignment surface; question arrangement is the
  visually dominant work on the composition surface and no settings form competes with it; no route
  target is unreachable from a link; `./check_codebase.sh` and the Ribbon e2e contract tests pass.
- Parallel-plan ready: no. Retirement, link repair, and the surface split touch the same files.

#### M1 completion receipt (2026-09-10)

- Status: complete. The surviving Instructor Assignment path separates Question composition from
  delivery settings; the retired duplicate surface, orphan path, and dead internal links are
  resolved. Assignment Preview remains keyboard reachable and opens in a new tab.
- Accepted gates: `./check_codebase.sh` passed with 389/389 Node tests; `source source_me.sh &&
  python3 -m pytest tests/test_markdown_links.py tests/test_ascii_compliance.py
  tests/test_whitespace.py` passed with 1,948 tests; and `git diff --check` passed.
- Connected evidence: the Ribbon E2E gate passed, and both the service-only and browser-only
  Assignment Release gates passed. Composition image review and keyboard ordering, save, and reload
  evidence passed.
- Independent review: a fresh M1 review passed with no blocker or major finding.
- Remaining scope: final screenshot recapture, receipt, and atlas publication remain M9 work and
  are not claimed by this receipt.

### Milestone: M2 settled Ribbon shape and dense top bar

- Depends on: M0 for the settled shape; M1 for the surviving Assignment task destinations.
- Deliverables: the three Instructor Product tabs living in one information-dense top bar beside the
  identity and account controls; the settled Courses, Questions, and Assignments task rows; a
  glyph plus text on every visible navigation item; breadcrumbs for routes below the Ribbon's
  navigation levels; regenerated `docs/ux/RIBBON_DESTINATION_LEDGER.md`.
- Workstreams: WS-RIBBON, WS-CRUMB.
- Entry criteria: M0 and M1 exit.
- Exit criteria: the dense single bar carries identity, role, account, the three tabs, and the
  account controls at 1280 by 800 with visible focus, reading-order tab traversal, and no
  horizontal overflow; the tablet and narrow-phone profiles keep every control reachable, with a
  separate Tab Row retained only where one of those profiles shows a concrete failure and the
  failure is named; unbacked destinations resolve Unavailable; the generated ledger matches the
  catalog; the icon sprite build test passes from the same-origin sprite.
- Parallel-plan ready: yes.

### Milestone: M3 Instructor list density and identity

- Depends on: M0 only, for the density decision record. Independent of M1 and M2.
- Deliverables: dense Course and Assignment rows driven by existing `--ple-*` tokens; Course Theme
  identity visible per Course row; a decision on how theme identity resolves on a Product-scope
  route.
- Workstreams: WS-DENSITY.
- Entry criteria: M0 exit.
- Exit criteria: `image_evaluator` review at 1280 by 800 finds no clipping, contrast, or target
  regression; token changes stay in `src/style.css` rather than page-local numbers.
- Parallel-plan ready: yes.

#### M3 completion receipt (2026-09-10)

- Status: complete. Instructor Course and Assignment lists use dense semantic rows with a
  title-first, metadata-second, bounded-action pattern. Student Course cards remain on their
  existing card family.
- Product-scope Course Theme identity is deliberately narrow: the Instructor-authorized summary
  supplies only the closed `theme` value, each row names its Theme in text and carries a local
  accent boundary, and no Course appearance variables are mounted at Product scope.
- Focus and accessibility evidence: the independent image evaluation passed at 1280 by 800,
  768 by 1024, and 320 by 640, including a forced-colors sample. Rows had no clipping or
  row-originated overflow; narrow actions were 44px high, focus remained visible, and text plus
  the boundary preserved non-color Theme identity. Its fixture header was intentionally
  non-production, so its narrow-header overflow is excluded from this M3 row finding and remains
  a separate Ribbon concern.
- Focused checks: 11 Node Course-summary/theme/scope tests, learning-data-access and server-core
  Cargo checks, format, diff, and source-line checks passed. TypeScript passed before unrelated
  concurrent M2 Ribbon drift; the independent M3 review found no M3 TypeScript error.
- Independent review: code, authorization, decoder, semantic-row, and shared-CSS checks passed;
  the initially pending visual gate now passes.

### Milestone: M4 Assignment settings defaults

- Depends on: M1 for the surviving settings page; M13, because a 11:59 PM default acquires meaning
  once an Instructor zone interprets it.
- Deliverables: due time defaulting to 11:59 PM interpreted in the Instructor's zone; late work
  defaulting to reject; prohibit late submissions and prohibit new attempts after due both
  defaulting on; one question at a time presented as fixed; the optional `Randomize question order`
  setting surfaced clearly; time limit presented as an Assignment Setting; Instructor-controlled
  previous-attempt answer visibility.
- Workstreams: WS-DEFAULTS.
- Entry criteria: M1 exit; M13 exit.
- Exit criteria: Rust `Default` impls and browser draft defaults agree; a saved Assignment
  round-trips every changed field; the six disclosure controls keep their current independence and
  their answer-bearing defaults; Assignment settings own schedule, attempts, and Question order,
  and Question presentation owns answer-choice order.
- Parallel-plan ready: yes.

### Milestone: M11 inline Assignment title and due-date editing

- Depends on: M4 for due-date semantics; M3 for the dense row pattern; M0 for the mutable
  current-state decision.
- Deliverables: Assignment Title and Due Date editable in place from the Course Assignment list,
  including on a released Assignment.
- Workstreams: WS-INLINE.
- Entry criteria: M4 and M3 exit.
- Exit criteria: an Instructor retimes a released Assignment from the list; the save records
  current state with no revision or undo entry; issued and graded Student evidence keeps its
  existing pinning; keyboard operation is complete; a failed save preserves the typed value and
  states what to fix.
- Parallel-plan ready: no. One surface, one owner.

### Milestone: M12 Question-owned answer randomization

- Depends on: M0 for the recorded ownership decision. Independent of every Assignment milestone.
- Deliverables: a PLE-native Question declares whether its answer choices are randomized when
  presented; the declaration is authored where the Question is authored; issuance applies it.
- Workstreams: WS-QRAND.
- Entry criteria: M0 exit.
- Exit criteria: Question presentation is the sole owner of answer-choice order; a shuffled and a
  fixed-order Question each round-trip through authoring and issuance; the issued binding records
  the presented order so grading and recovery stay exact; Question Backend questions keep their
  backend's presentation, which PLE does not modify.
- Parallel-plan ready: no. Model, authoring, and issuance are one slice.

#### M12 completion receipt (2026-09-10)

- Status: complete. PLE-native Question source authoring alone owns private `randomizeChoices`:
  omitted legacy declarations read as false, new declarations round-trip explicitly, and
  non-choice declarations are refused. Generated and issued public response contracts contain no
  randomization-policy field.
- Issuance evidence: native choice order is derived deterministically from the durable nonce and
  stable authored choice IDs before opaque bindings are minted. The persisted binding reproduces
  the presented order and grades the same semantic choices; Question Backend presentation remains
  backend-owned.
- Focused checks: Rust presentation and PLE Question JSON tests, 37 Node authoring tests, unchanged
  `cargo tools tsgen` output, `npx tsc --noEmit`, the 966-line presentation builder check, and both
  diff checks passed.
- Independent review: the fresh M12 re-review passed after verifying fixed-nonce order,
  source-vector reorder invariance, persisted-binding reproduction, and the absent public policy
  field.
- Shared-gate boundary: the repository-wide source-file limit reported only concurrent
  `src/style.css` at 1008 lines; M12's builder is 966 lines.

### Milestone: M13 account-owned time zones

- Depends on: M0 for the recorded zone model.
- Deliverables: an IANA zone on the Instructor Account and on the Student Account, the Student's
  defaulting from the Instructor's at enrollment and thereafter owned by the Student; one allocated
  forward migration.
- Workstreams: WS-ZONE.
- Entry criteria: M0 exit; a migration allocation recorded.
- Exit criteria: both account kinds carry a valid IANA zone; the column enforces IANA membership in
  the schema as well as in Rust; account setup establishes a valid zone before wall-clock entry
  becomes available; stored instants stay usable and renderable regardless of preference state,
  since they carry no zone; a clean-volume run and a second no-op run both pass.
- Parallel-plan ready: no. One schema slice.

#### M13 completion receipt (2026-09-10)

- Status: complete. The private account-owned preference relation gives every Instructor and Student
  an exact installed-IANA zone. Rust and SQL both enforce installed-IANA membership; the contract
  intentionally accepts an installed exact backward-link spelling such as `US/Central`.
- Ownership and lifecycle: the authenticated capability reads only the caller's preference. Account
  creation establishes the default for every Account. Authorized Instructor roster creation supplies
  an Instructor-zone default only for a new Student; later claim, import, and re-enrollment preserve
  the Student-owned value.
- Data boundary: the preference relation is private with forced RLS and no direct application-table
  access. This storage change does not rewrite stored deadline instants, which carry no zone.
- Acceptance evidence: the disposable PostgreSQL 17 run applied the migration cleanly, the second
  invocation completed as a compatible no-op, and the connected oracle covered direct-table/RLS
  denial, self-only reads, every-Account defaults, and the roster lifecycle. The runner removed its
  exact container, volume, and network afterward.
- Checks and reviews: focused Rust AccountTimeZone tests passed; `source source_me.sh && python3 -m
  pytest tests/` passed with 6120 tests; `git diff --check` passed. Independent architecture and
  security reviews both passed with no findings.

### Milestone: M14 wall-clock input interpretation

- Depends on: M13, because interpretation reads an account zone.
- Deliverables: a zone-free local date-and-time input DTO replacing `CourseLocalDateAndTime`, taking
  its interpretation zone from the authenticated account.
- Workstreams: WS-ZONE.
- Entry criteria: M13 exit.
- Exit criteria: account zones are the sole wall-clock interpretation context; a stored deadline is
  an instant; the resolver keeps refusing DST gaps and ambiguity; the browser sends an unconverted
  wall-clock string and performs no zone math; the existing zone-mismatch guard either becomes the
  live path or is removed, since it has no production caller today.
- Parallel-plan ready: no.

#### M14 completion receipt (2026-09-10)

- Status: complete. `LocalDateAndTime` is a zone-free local wall-clock DTO. The browser preserves
  and sends its raw value without zone math, while the authenticated Instructor's `AccountTimeZone`
  resolves and projects it inside the authorized Store transaction; the stored deadline remains a
  `timestamptz` instant.
- Resolver evidence: direct Account-zone tests cover a normal exact-millisecond round trip, term
  bounds, spring DST gaps, fall ambiguity, local order, and resolved absolute order.
- Connected acceptance: a disposable run used an Instructor zone different from the Course's former
  zone, performed a CAS save and reload, and verified the expected instant, stored `timestamptz`,
  returned milliseconds, and local projection. Foreign and missing Course contexts remain
  concealed; both fresh and compatible no-op acceptance runs passed.
- Checks and review: 38 focused Rust tests, 25 focused Node tests, `cargo tools tsgen`,
  `npx tsc --noEmit`, and `git diff --check` passed. The independent M14 re-review passed.
- Remaining scope: M15 owns account-zone display, M16 retires the legacy Course-zone concept, and
  M17-M18 own Profile preferences and the thumbnail.

### Milestone: M15 account-zone display

- Depends on: M13 for the zones to render in.
- Deliverables: Instructor surfaces rendering in the Instructor's zone and Student surfaces in the
  Student's; the three browser-local leak sites repaired; the governing zone named on the due-date
  editor.
- Workstreams: WS-TZFIX.
- Entry criteria: M13 exit.
- Exit criteria: every wall-clock display names the account zone it renders in; two timestamps on
  one surface agree; the attempt countdown keeps consuming a server-computed remaining duration.
- Parallel-plan ready: yes, per surface.

### Milestone: M16 retire the Course zone

- Depends on: M14, which removes the last interpretation dependency on it; M15, which removes the
  last display dependency.
- Deliverables: the Course zone field and every read of it removed; one allocated forward
  migration; a recorded resolution for the course-term calendar bound the field participates in.
- Workstreams: WS-ZONE.
- Entry criteria: M14 and M15 exit; a migration allocation recorded.
- Exit criteria: account zones are the only wall-clock concept in the schema, the Rust model, and
  the browser; the calendar bound is preserved by another means or deliberately dropped with the
  reason recorded; the migration is forward-only with a clean-volume and no-op run.
- Parallel-plan ready: no.

### Milestone: M17 Instructor Profile page

- Depends on: M13 for the stored zone; M2 for the Context Control that reaches it.
- Deliverables: an Instructor Profile route and page reading and writing the account zone.
- Workstreams: WS-PROFILE.
- Entry criteria: M13 exit; M2 exit.
- Exit criteria: an Instructor sets their zone and sees existing deadlines re-render in it while
  their instants hold; the page states that effect plainly; the route is a Ribbon Context Control
  rendering with No Selected Ribbon Tab, matching the existing Context Control routes.
- Parallel-plan ready: no.

### Milestone: M18 profile thumbnail

- Depends on: M17 for the page that hosts it.
- Deliverables: a cropped profile thumbnail from an arbitrary useful source image, presented behind
  a consistent rounded-rectangle silhouette.
- Workstreams: WS-PROFILE.
- Entry criteria: M17 exit.
- Exit criteria: a non-square source produces a correct centered thumbnail; the server owns the
  crop, rendition, and delivery, reusing the Course Banner boundary; the thumbnail renders behind
  one consistent silhouette wherever it appears; Student upload capability stays absent.
- Parallel-plan ready: no.

### Milestone: M5 Student one-question delivery

- Depends on: M0 for the recorded delivery decision. Independent of every Instructor milestone.
- Deliverables: the one-question attempt lane on the live Student path; a question navigation bar
  showing per-question saved state; the subtle timer with its existing Wasm timing source; the
  `Question n of m` Context Row position the design guide already specifies; the double-press
  Start defect fixed; the Attempt task row pointed at a Student-reachable destination.
- Workstreams: WS-DELIVERY, WS-NAVBAR.
- Entry criteria: M0 exit.
- Exit criteria: a keyboard-only journey completes an attempt; whether each question's response is
  saved is visible and truthful after reload; the timer reads remaining time rather than an
  absolute deadline; axe reports no serious or critical finding on the Student attempt surface.
- Parallel-plan ready: yes, once WS-DELIVERY has declared the surviving lane.

### Milestone: M6 Student vocabulary and course entry

- Depends on: none. Runs from day one.
- Deliverables: Instructor nouns and typed references removed from Student-visible text; library
  Question Titles withheld from Students; single-active-course auto-entry.
- Workstreams: WS-VOCAB.
- Entry criteria: none.
- Exit criteria: a Student-visible text sweep finds no `Course Instance`, `Released Assignment`, or
  `C-n` string; a Student with one course lands on that course; a Student with two still chooses.
- Parallel-plan ready: yes.

#### M6 completion receipt (2026-09-10)

- Status: complete. Student pages and delivery headings use Student language, withhold authored
  library Question Titles, and route a resolved single-current-Course account to its Course while
  retaining the zero-Course empty state, multi-Course chooser, and explicit chooser return path.
- Focused checks: `npx tsc --noEmit -p tsconfig.json`, the two-case attempt recovery test, M6
  Prettier checks, and `git diff --check` passed. The initial M6 repository gate also passed with
  typecheck, lint, format, and 362 Node tests.
- Browser evidence: the compiled production-component journey
  `node --import tsx tests/playwright/student_course_entry_m6_evidence.mjs` passed on rerun. It
  covers zero, one, chooser, many, and landing-back behavior.
- Independent review: the M6 re-review passed after the Student attempt route stopped rendering
  the authored Question Title and the updated browser assertions covered the visible headings.
- Shared-gate boundary: later full-suite and repository-gate failures belonged to concurrent M12
  and human-guidance formatting work and are not claimed as M6 evidence.

### Milestone: M7 Start Assignment facts

- Depends on: M5 for the surviving Student delivery lane; M6 for the vocabulary rules the new copy
  must follow.
- Deliverables: `LiveAssignmentAccess` extended with question count, points possible, time limit,
  and previous-attempt history; the existing answer-free facts component mounted for Students; the
  assignment title replacing the hard-coded heading; previous attempts viewable with answers gated
  by the Instructor's disclosure setting.
- Workstreams: WS-ACCESS.
- Entry criteria: M5 and M6 exit.
- Exit criteria: the response carries no answer, key, or correctness beyond the disclosure setting;
  a foreign or anonymous caller still receives the current concealed refusal.
- Parallel-plan ready: no. One owner spans the server route, the contract, and the page.

### Milestone: M9 evidence close-out

- Depends on: every milestone whose surface it captures.
- Deliverables: axe coverage for the changed Student surfaces; refreshed screenshot corpus entries
  and manifest digest; `docs/CHANGELOG.md` entries; a recommendation on whether an accessibility
  gate belongs in `check_codebase.sh`.
- Workstreams: WS-EVIDENCE.
- Entry criteria: the milestones it captures have passed their own exit criteria.
- Exit criteria: the corpus receipt matches the captured set; no serious or critical axe finding on
  a changed Student surface.
- Parallel-plan ready: yes, per captured surface.

### Milestone: M10 Assignments Due Soon

- Depends on: M2 for the Assignments tab that holds the destination; WP-DOC3 for the display-zone
  rule.
- Deliverables: a cross-course Due Soon view for the current Instructor, with Course identity and
  due time per row, bounded usefully by due time. The implementation owner chooses the data-access
  and routing shape from existing repository patterns. The audit found the needed data and the
  cross-course membership join already present, so no new abstraction is assumed.
- Workstreams: WS-DUESOON.
- Entry criteria: M2 exit; WP-DOC3 recorded.
- Exit criteria: an Instructor sees only Assignments from Courses they currently teach; a revoked
  membership drops rows immediately; each row shows Course identity and due time in the governing
  zone; whatever migration or index the chosen shape needs has an allocation and passes a fresh and
  a no-op run.
- Parallel-plan ready: no. One vertical slice, one owner.

### Milestone: M19 Active and Inactive Courses

- Depends on: M0 for the recorded definition; M2 for the Courses task row that holds both
  destinations; M3 for the row pattern the lists reuse.
- Deliverables: the smallest durable representation of Course activity that supports My Active
  Courses and My Inactive Courses, derived from the existing retention model rather than a new
  lifecycle machine; both task-row destinations backed and Available; a finding on how a Course
  becomes Inactive, whether by term end, by an explicit Instructor action, or by the existing
  retention transitions.
- Workstreams: WS-COURSESTATE.
- Entry criteria: M0, M2, and M3 exit; whatever migration allocation the chosen representation
  needs.
- Exit criteria: an Instructor sees current Courses under Active and previous-semester Courses under
  Inactive; an Inactive Course exposes its non-sensitive metadata and no FERPA-sensitive Student
  data; the representation adds no lifecycle state the definition does not require.
- Parallel-plan ready: no. Representation and both lists are one slice.

## Workstream breakdown

### Workstream: WS-DOC

- Goal: one visible-name and one structural truth across the four interface documents.
- Owner: `architect`.
- Work packages: WP-DOC1, WP-DOC2, WP-DOC3.
- Needs: the owner's settled answers, recorded below under resolved decisions.
- Provides: the names and structure every later workstream implements.
- Review boundary, when modifying the repository: documentation only; no source change.

### Workstream: WS-RETIRE

- Goal: exactly one Instructor Assignment editing path, with no dead link.
- Owner: `expert_coder`.
- Work packages: WP-RET1, WP-RET2, WP-RET3.
- Needs: WS-DOC settled names.
- Provides: stable Assignment routes for WS-RIBBON and WS-DEFAULTS.
- Review boundary, when modifying the repository: `reviewer` on the deleted surface and every
  Playwright selector that referenced it.

### Workstream: WS-COMPOSE

- Goal: Assignment composition and Assignment delivery read as two different Instructor jobs.
- Owner: `coder` with the `ui-ux-engineer` skill.
- Work packages: WP-COM1.
- Needs: WS-RETIRE surviving routes.
- Provides: the composition surface WS-INLINE and WS-DEFAULTS build around.
- Review boundary, when modifying the repository: `image_evaluator` on which work dominates the
  composition surface at 1280 by 800.

### Workstream: WS-RIBBON

- Goal: the settled tabs, task rows, and glyphs, with unbacked destinations still honest.
- Owner: `coder`.
- Work packages: WP-RIB1, WP-RIB2, WP-RIB3, WP-RIB4.
- Needs: WS-DOC structure; WS-RETIRE destinations.
- Provides: the Ribbon Context Control position WS-PROFILE needs.
- Review boundary, when modifying the repository: regenerated ledger plus the Ribbon e2e contract
  tests as the reviewable artifact.

### Workstream: WS-CRUMB

- Goal: breadcrumbs for routes below the Ribbon's two navigation levels.
- Owner: `coder`.
- Work packages: WP-CRUMB1.
- Needs: WS-DOC placement decision (Context Row versus shell).
- Provides: nothing downstream.
- Review boundary, when modifying the repository: keyboard and landmark evidence, because the shell
  already owns focus transfer and the skip link.

### Workstream: WS-DENSITY

- Goal: spreadsheet-density Instructor lists that show Course identity.
- Owner: `coder` with the `css-creative-expert` skill.
- Work packages: WP-DEN1, WP-DEN2.
- Needs: WS-DOC density record.
- Provides: the row pattern WS-INLINE edits in place.
- Review boundary, when modifying the repository: `image_evaluator` visual report.

### Workstream: WS-DEFAULTS

- Goal: the notes' Assignment Settings defaults and separated disclosure.
- Owner: `expert_coder`.
- Work packages: WP-DEF1, WP-DEF2, WP-DEF3.
- Needs: WS-RETIRE surviving settings page; WS-ZONE account zones for the due-time default.
- Provides: the settings surface WS-ACCESS reads for Student-facing facts.
- Review boundary, when modifying the repository: `reviewer` on every changed default, with
  explicit attention to disclosure.

### Workstream: WS-INLINE

- Goal: inline Assignment Title and Due Date editing from the Course Assignment list.
- Owner: `coder`.
- Work packages: WP-INL1.
- Needs: WS-DENSITY row pattern; WS-DEFAULTS due-date semantics.
- Provides: nothing downstream.
- Review boundary, when modifying the repository: keyboard operation and save-failure recovery.

### Workstream: WS-DELIVERY

- Goal: one Student delivery lane, one question per page, with a truthful timer.
- Owner: `expert_coder`.
- Work packages: WP-DEL1, WP-DEL2, WP-DEL3.
- Needs: WS-DOC delivery decision.
- Provides: the surviving lane WS-NAVBAR and WS-ACCESS build on.
- Review boundary, when modifying the repository: `reviewer` plus a recorded keyboard journey.

### Workstream: WS-NAVBAR

- Goal: a question navigation bar that shows saved state per question.
- Owner: `coder` with the `ui-ux-engineer` skill.
- Work packages: WP-NAV1, WP-NAV2.
- Needs: WS-DELIVERY surviving lane and its per-question state source.
- Provides: nothing downstream.
- Review boundary, when modifying the repository: state-matrix evidence for unanswered, saved, and
  current.

### Workstream: WS-VOCAB

- Goal: Student-visible text that uses no Instructor noun.
- Owner: `coder`.
- Work packages: WP-VOC1, WP-VOC2.
- Needs: nothing.
- Provides: the copy rules WS-ACCESS follows.
- Review boundary, when modifying the repository: `reviewer` text sweep.

### Workstream: WS-ACCESS

- Goal: the Start Assignment facts and previous attempts, answer-free.
- Owner: `expert_coder`.
- Work packages: WP-ACC1, WP-ACC2.
- Needs: WS-DELIVERY lane; WS-VOCAB copy rules; WS-DEFAULTS disclosure semantics.
- Provides: nothing downstream.
- Review boundary, when modifying the repository: independent security review of the widened
  response, because it adds Student-visible score history.

### Workstream: WS-QRAND

- Goal: Question presentation owns answer-choice order.
- Owner: `expert_coder`.
- Work packages: WP-QRAND.
- Needs: the M0 ownership decision.
- Provides: nothing downstream; Assignment settings deliberately depend on none of it.
- Review boundary, when modifying the repository: `reviewer` on the issuance path, because
  presented choice order already participates in the immutable issued binding.

### Workstream: WS-ZONE

- Goal: account-owned zones replace Course-owned wall-clock time end to end.
- Owner: `expert_coder`.
- Work packages: WP-ZONE1, WP-ZONE2, WP-ZONE3.
- Needs: the M0 zone model; migration allocations.
- Provides: the Instructor zone that WS-DEFAULTS' 11:59 PM default depends on, and the Student zone
  that Student schedule display depends on.
- Review boundary, when modifying the repository: `architect` on schema and the retired Course
  concept; the existing DST refusal tests are the regression boundary.

### Workstream: WS-TZFIX

- Goal: one deliberate zone per rendered timestamp.
- Owner: `coder`.
- Work packages: WP-TZ2.
- Needs: the M0 zone-ownership decision.
- Provides: a consistent display baseline WS-PROFILE can then re-point at the Instructor zone.
- Review boundary, when modifying the repository: `reviewer`; browser-only, no server change.

### Workstream: WS-PROFILE

- Goal: Instructor-owned time zone and profile image.
- Owner: `expert_coder`.
- Work packages: WP-PRO1, WP-PRO2.
- Needs: WS-RIBBON Context Control; WS-TZFIX display baseline; a migration allocation.
- Provides: nothing downstream.
- Review boundary, when modifying the repository: `architect` on the schema and authority path.

### Workstream: WS-DUESOON

- Goal: one cross-course due list for the current Instructor, over data that already exists.
- Owner: `expert_coder`.
- Work packages: WP-DUE1, WP-DUE2.
- Needs: WS-RIBBON destination; WP-DOC3 display-zone rule; a migration allocation.
- Provides: nothing downstream.
- Review boundary, when modifying the repository: `reviewer` on the membership join and the
  revocation case, because this is the first Instructor list spanning courses.

### Workstream: WS-COURSESTATE

- Goal: Active and Inactive Courses as a real, minimal domain distinction.
- Owner: `expert_coder`.
- Work packages: WP-CST1, WP-CST2.
- Needs: the M0 definition; WS-DENSITY row pattern; WS-RIBBON destinations.
- Provides: nothing downstream.
- Review boundary, when modifying the repository: `architect` on the representation, because this
  touches the FERPA retention boundary.

### Workstream: WS-EVIDENCE

- Goal: accepted visual, accessibility, and changelog receipts.
- Owner: `tester` with `playwright_operator` for capture.
- Work packages: WP-EVI1, WP-EVI2, WP-EVI3.
- Needs: the captured surfaces to be complete.
- Provides: the acceptance record.
- Review boundary, when modifying the repository: the corpus receipt and manifest digest.

## Work packages

### Work package: WP-DOC1 reduce the interface terminology document to terminology

- Owner: `architect`.
- Touch points: `docs/INTERFACE_TERMINOLOGY.md`, `docs/UI_DESIGN_GUIDE.md`.
- Depends on: none.
- Acceptance criteria: the terminology document defines names and their semantic ownership only;
  every tab set, task order, placement, and reservation rule it currently states lives in the
  design guide; no rule is lost in the move.
- Evidence or review, when useful: `reviewer` diff read for dropped rules.
- Obvious follow-ons: WP-DOC2.

### Work package: WP-DOC2 record the settled interface decisions

- Owner: `architect`.
- Touch points: `docs/DESIGN_DECISIONS.md`, `docs/UI_DESIGN_GUIDE.md`.
- Depends on: WP-DOC1, which frees the structural rules to change.
- Acceptance criteria: entries record the three-tab Instructor Product Ribbon as the owner's
  taxonomy decision rather than a derivation of the current route hierarchy; the six-task Questions
  row in the owner's order; the Courses and Assignments task rows with their unbacked positions
  named; the Assignment composition-versus-delivery task split, with visible names marked
  secondary; the single Student delivery lane; the icon-plus-text rule for every visible Ribbon
  item; each entry carries Decision, Why, Consequence, and Owner.
- Evidence or review, when useful: `tests/test_markdown_links.py`.
- Obvious follow-ons: every later milestone reads these entries as its specification.

### Work package: WP-DOC3 record the domain decisions the notes exposed

- Owner: `architect`.
- Touch points: `docs/HUMAN_GUIDANCE.md`, `docs/DESIGN_DECISIONS.md`,
  `docs/TERMINOLOGY_CONTRACT.md` if a term changes.
- Depends on: none. Runs beside WP-DOC1 and WP-DOC2.
- Acceptance criteria: five decisions are recorded with Decision, Why, Consequence, and Owner,
  transcribing the owner's settled positions from this plan's resolved decisions rather than
  reopening them.
  1. Time zone: the six-step account-owned model, the deadline as a zone-free instant, the retired
     Course concept, and the rule that changing a profile zone re-renders but never moves an
     existing deadline.
  2. Assignment and Draft Question state: one current editable state with no revision history or
     undo; immutable snapshots survive only where Student attempts, issued Questions, or grading
     evidence concretely depend on them.
  3. Active and Inactive Courses: Inactive is a previous-semester Course with FERPA-sensitive
     Student data stripped and non-sensitive metadata retained.
  4. Randomization ownership: an Assignment may randomize Question order; a Question declares
     whether its own answer choices are randomized when presented, and no Assignment control
     overrides that.
  5. The Blueprint provenance column `assignment.source_blueprint_*` is course-level origin copied
     onto every Assignment, including hand-authored ones, and is therefore not a per-assignment
     lineage basis.
- Evidence or review, when useful: `reviewer` confirming each entry cites the audited file
  locations rather than restating the notes. Decision 2 needs one implementation-boundary finding
  recorded beside it: which existing consumers of `assignment_revision` and
  `assignment_revision_entry` actually require the snapshot. That finding sizes the work; it does
  not reopen the decision.
- Obvious follow-ons: WP-INL1, WP-PRO1, WP-DUE1 all read these entries as their specification.

### Work package: WP-RET1 retire the duplicate Assignment editing page

- Owner: `expert_coder`.
- Touch points: `src/pages/assignment_release_page.tsx`, `src/route_contract.ts`,
  `src/routes.ts`, `src/pages/course_instance_page.tsx`.
- Depends on: WP-DOC2 for the surviving names.
- Acceptance criteria: one surface per Assignment section; the Create Assignment path preserved;
  every capability the retired page held (release, validate decision, preview) either lives on a
  surviving surface or is recorded as deliberately dropped; Playwright selectors and screenshot
  scenarios referencing the retired heading updated.
- Evidence or review, when useful: `reviewer`; `./check_codebase.sh`; Ribbon e2e contract tests.
- Obvious follow-ons: WP-RET2, WP-RET3.

### Work package: WP-RET2 resolve the orphan Assignment list page

- Owner: `expert_coder`.
- Touch points: `src/pages/course_assignments_page.tsx`, `src/pages/course_instance_page.tsx`,
  `src/routes.ts`.
- Depends on: WP-RET1, which settles which Assignment routes exist.
- Acceptance criteria: the orphan is either mounted as the Course Assignment list, keeping its
  pagination and themed identity, or deleted; no unreferenced page component remains; pyflakes-
  equivalent lint and `tsc` pass.
- Evidence or review, when useful: `reviewer` on the mount-versus-delete rationale.
- Obvious follow-ons: WP-DEN2 styles whichever list survives.

### Work package: WP-RET3 repair dead links and preview behavior

- Owner: `coder`.
- Touch points: `src/pages/assignment_workspace/assignment_workspace_overview_page.tsx`,
  `assignment_workspace_policies_page.tsx`, the preview control.
- Depends on: WP-RET1.
- Acceptance criteria: no Assignment surface links to a route absent from `ROUTE_CONTRACT`;
  Assignment Preview opens in a new browser tab and remains keyboard reachable with a discoverable
  return path.
- Evidence or review, when useful: a route-link sweep asserting every internal link resolves.
- Obvious follow-ons: consider a permanent test asserting every rendered internal link matches a
  contract route, if it can stay fast and offline per PYTEST_STYLE.md.

### Work package: WP-COM1 make question arrangement the primary editing work

- Owner: `coder` with `ui-ux-engineer`.
- Touch points: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx`,
  `assignment_workspace_policies_page.tsx`, their stylesheets.
- Depends on: WP-RET1, which settles the surviving routes.
- Acceptance criteria: the composition surface gives selecting, adding, removing, and ordering
  Questions the primary work surface, with one dominant primary action; delivery and policy fields
  do not appear on it beyond what composition needs; ordering works by keyboard through directional
  controls or a direct position selector, not by dragging alone, per the design guide; the
  Assignment Title field's current home on the questions page is either kept deliberately or moved
  with the reason recorded.
- Evidence or review, when useful: `image_evaluator` at 1280 by 800; a keyboard reordering journey.
- Obvious follow-ons: WP-INL1 edits the same rows from the Course list.

### Work package: WP-RIB1 implement the settled tab set

- Owner: `coder`.
- Touch points: `src/route_contract.ts`, `src/ribbon/ribbon_catalog.ts`,
  `src/ribbon/ribbon_schema.ts`, `src/ribbon/capability_registry.ts`.
- Depends on: WP-DOC2, WP-RET1.
- Acceptance criteria: the Instructor Product Ribbon presents the settled three tabs; Blueprint
  Courses reaches its route from the Courses task row; unbacked positions resolve Unavailable and
  render no link.
- Evidence or review, when useful: regenerated `docs/ux/RIBBON_DESTINATION_LEDGER.md`;
  `tests/e2e/e2e_ribbon_destination_ledger_contract.mjs`.
- Obvious follow-ons: WP-RIB2, WP-RIB3.

### Work package: WP-RIB4 compose the dense top bar

- Owner: `coder` with the `css-creative-expert` skill.
- Touch points: `src/ribbon/app_ribbon.tsx`, `src/ribbon/app_ribbon.css`,
  `src/application_shell.tsx`.
- Depends on: WP-RIB1 for the tab set.
- Acceptance criteria: identity, product role, account label, the three tabs, and the account
  controls occupy one dense bar; the composition holds at 1280 by 800 and stays reachable at the
  tablet and narrow-phone profiles; focus order follows reading order; the shell keeps its skip
  link and `#main-content` focus transfer; content sits higher on the screen than the two-row
  arrangement placed it. Build this first and record any profile where it genuinely fails, since a
  retained separate Tab Row needs that named failure as its justification.
- Evidence or review, when useful: `image_evaluator` at all three profiles; the existing Ribbon
  responsive evidence script asserting no horizontal overflow; a keyboard traversal capture.
- Obvious follow-ons: WP-CRUMB1 sits below whichever bar arrangement survives.

### Work package: WP-RIB2 implement the settled task rows

- Owner: `coder`.
- Touch points: `src/ribbon/ribbon_catalog.ts`, `src/route_contract.ts`.
- Depends on: WP-RIB1.
- Acceptance criteria: the Questions row presents the owner's six tasks in his stated order with
  Starred and Watched reserved; the Courses and Assignments rows present their settled tasks;
  declared task-group topology still decides row reservation rather than admission.
- Evidence or review, when useful: `tests/e2e/e2e_ribbon_app_component.mjs`.
- Obvious follow-ons: none.

### Work package: WP-RIB3 extend the icon sprite for the settled tabs

- Owner: `coder`.
- Touch points: `src/ribbon/ribbon_icons.ts`, `devel/build_ribbon_icon_sprite.mjs`.
- Depends on: WP-RIB1.
- Acceptance criteria: every visible Ribbon navigation item carries a Font Awesome glyph plus its
  text label; the closed sprite vocabulary is extended as needed to cover them; no visible item is
  left text-only beside icon-and-text peers; the sprite stays same-origin and build-time
  subsetted; the no-CDN test passes.
- Evidence or review, when useful: `tests/e2e/e2e_ribbon_icon_sprite_build.mjs`.
- Obvious follow-ons: none.

### Work package: WP-CRUMB1 add breadcrumbs for deep routes

- Owner: `coder`.
- Touch points: `src/ribbon/app_ribbon.tsx` or `src/application_shell.tsx`, per the WP-DOC2
  placement decision; `src/ribbon/ribbon_contract.ts` for label sources.
- Depends on: WP-DOC2.
- Acceptance criteria: routes below the two Ribbon levels show an ordered trail built from the
  labels the Context Row already resolves; the trail uses real links; it adds no Ribbon row and
  moves no visible control; a deferred label does not shift geometry.
- Evidence or review, when useful: identity and geometry evidence at 1280 by 800 plus the narrow
  guard, matching the existing Ribbon evidence pattern.
- Obvious follow-ons: none.

### Work package: WP-DEN1 convert Instructor lists to dense rows

- Owner: `coder` with `css-creative-expert`.
- Touch points: `src/pages/course_list_page.tsx`, `src/style.css`.
- Depends on: WP-DOC2 density record.
- Acceptance criteria: Course rows scan as a table rather than cards; density comes from existing
  `--ple-*` tokens or new shared tokens, not page-local numbers; labels, focus, and touch targets
  survive the compact and narrow arrangements.
- Evidence or review, when useful: `image_evaluator` at 1280 by 800; computed-style check that the
  row rhythm derives from tokens.
- Obvious follow-ons: WP-DEN2.

### Work package: WP-DEN2 show Course Theme identity in list rows

- Owner: `coder`.
- Touch points: `src/pages/course_list_page.tsx`,
  `src/features/course_appearance/course_theme_registry.ts`.
- Depends on: WP-DEN1.
- Acceptance criteria: each Course row carries a visible theme identity that is not color alone;
  the Product-scope route resolves per-row appearance without claiming a course scope it does not
  have; contrast stays within the guide's 5.5:1 floor and 8.25:1 ceiling.
- Evidence or review, when useful: `image_evaluator`; forced-colors sample.
- Obvious follow-ons: consider the same treatment for the Blueprint Course list.

### Work package: WP-DEF1 apply the notes' schedule and late-work defaults

- Owner: `expert_coder`.
- Touch points: `crates/question_model/src/assignment.rs`,
  `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx`.
- Depends on: WP-RET1 for the surviving settings page; WP-ZONE1 and WP-ZONE2, because a default of
  11:59 PM has no meaning until an Instructor zone exists to interpret it.
- Acceptance criteria: a new Assignment's due time defaults to 11:59 PM interpreted in the
  Instructor's zone; late work defaults to reject; prohibit late submissions and prohibit new
  attempts after due both default on; the browser continues to send an unconverted wall-clock
  string so the server owns zone resolution.
- Evidence or review, when useful: focused Rust default tests; a saved round-trip.
- Obvious follow-ons: WP-DEF2.

### Work package: WP-DEF2 separate the results-disclosure settings

- Owner: `expert_coder`.
- Touch points: `crates/question_model/src/assignment_activity_rules.rs`,
  `src/pages/assignment_workspace/assignment_workspace_policy_panel.tsx`.
- Depends on: WP-DEF1.
- Acceptance criteria: the existing six independent disclosure controls keep their independence;
  previous-attempt answer visibility is Instructor-controlled, which is the one disclosure item the
  owner named; class statistics keeps its Never default; no default widens answer disclosure. The
  Blackboard four-setting grouping is reference only and does not replace the current six controls
  unless a separate owner decision asks for it.
- Evidence or review, when useful: independent `reviewer` pass focused on secrecy.
- Obvious follow-ons: WP-DEF3.

### Work package: WP-DEF3 present the settled presentation options

- Owner: `coder`.
- Touch points: `assignment_workspace_policy_panel.tsx`, `assignment_activity_rules.rs`.
- Depends on: WP-DEF2.
- Acceptance criteria: one question at a time is presented as fixed; `Randomize question order` is
  the optional Assignment setting and maps onto the existing `assignmentQuestionOrderRule`;
  Assignment settings own schedule, attempts, and Question order, while Question presentation owns
  answer-choice order; the mapping from the notes' vocabulary to the existing order, pool, and
  variation rules is recorded alongside it. PLE shows one Question per page, so page-level
  randomization has no meaning here.
- Evidence or review, when useful: `reviewer` mapping note; a saved round-trip per new field.
- Obvious follow-ons: WP-QRAND places answer randomization on the Question. Grade category and
  attempts-allowed presentation stay out until the grading model needs them, per the owner's
  decision to omit final grade calculation.

### Work package: WP-QRAND declare answer randomization on the Question

- Owner: `expert_coder`.
- Touch points: `crates/question_model/` for the Question presentation property, the Draft Question
  authoring path and its editor page, the answer-free QuestionPresentation issuance path.
- Depends on: WP-DOC3 for the recorded ownership decision. Independent of every Assignment package.
- Acceptance criteria: Question presentation is the sole owner of answer-choice randomization. A
  PLE-native Question declares whether its choices are randomized when presented, authored where
  the Question is authored. Question Backend questions keep their backend's presentation, which
  WeBWorK and iMathAS own. Issuance applies the declaration when presenting native choices, and the
  issued binding records the presented order so grading and recovery stay exact. Derive the
  treatment of Question Types with no meaningful choice order from the existing Question model:
  absent field, rejected declaration, or documented inert declaration are all acceptable, and the
  package records which the model makes natural.
- Evidence or review, when useful: `reviewer` on the issuance path, since presented choice order
  already participates in the immutable issued binding; a round-trip for one shuffled and one
  fixed-order Question.
- Obvious follow-ons: if Question Publication Validation should reject a declaration on a Type that
  cannot use it, scope that with the validation work rather than widening this package.

### Work package: WP-INL1 add inline title and due-date editing

- Owner: `coder`.
- Touch points: the surviving Course Assignment list page.
- Depends on: WP-DEN1 for the row pattern; WP-DEF1 for due-date semantics; WP-DOC3 for the
  Assignment-state decision, because the current save path refuses any Assignment whose status is
  not `unreleased`, which is exactly the released Assignment an Instructor most wants to retime.
- Acceptance criteria: title and due date are editable in the row and save without leaving the
  page, including on a released Assignment, per the owner's decision that current state is
  changeable; the save adds no revision or undo record; a Student's in-flight or graded work keeps
  its existing pinning; keyboard operation is complete; a failed save preserves the typed value and
  states what to fix; the row returns to a read state on success; the typed wall-clock string still
  reaches the server unconverted so the server owns zone resolution.
- Evidence or review, when useful: state-matrix evidence for idle, editing, saving, and failed; a
  released-Assignment retime journey; `reviewer` on the relaxed status check.
- Obvious follow-ons: if relaxing the status check turns out to need a Store or SQL change beyond
  this package, split that into its own work package rather than widening this one.

### Work package: WP-DEL1 declare and promote the surviving Student delivery lane

- Owner: `expert_coder`.
- Touch points: `src/pages/assignment_attempt_page.tsx`, `src/pages/assignment_overview_page.tsx`,
  `src/ribbon/capability_registry.ts`, `src/route_contract.ts`.
- Depends on: WP-DOC2 delivery decision.
- Acceptance criteria: one Student route family delivers questions one at a time; the retired lane
  leaves no unreachable component; the Attempt task row targets a Student-reachable destination
  rather than an Instructor-only route.
- Evidence or review, when useful: `reviewer`; a keyboard-only journey through a complete attempt.
- Obvious follow-ons: WP-DEL2, WP-DEL3, WP-NAV1.

### Work package: WP-DEL2 fix the double Start Assignment press

- Owner: `coder`.
- Touch points: the surviving Student start and delivery components.
- Depends on: WP-DEL1.
- Acceptance criteria: starting an attempt requires one activation; issued state survives
  navigation within the attempt; a resumed attempt does not present the start control again.
- Evidence or review, when useful: a browser journey that starts, navigates, and resumes.
- Obvious follow-ons: none.

### Work package: WP-DEL3 present the subtle attempt timer

- Owner: `coder`, consulting `wasm-rust-expert` only if the timing export needs a change.
- Touch points: the surviving delivery component, `src/wasm/index.ts`, `crates/wasm/src/lib.rs`.
- Depends on: WP-DEL1.
- Acceptance criteria: the timer reads remaining time rather than an absolute deadline; it is
  quiet in normal state and announces politely rather than continuously; reduced motion removes
  animation without removing the state cue; the `Question n of m` Context Row position appears as
  the design guide already specifies.
- Evidence or review, when useful: an existing-export check first; a Rust change only if the
  current export cannot supply remaining time.
- Obvious follow-ons: none.

### Work package: WP-NAV1 add the question navigation bar

- Owner: `coder` with `ui-ux-engineer`.
- Touch points: the surviving delivery component; a new navigation component and its stylesheet.
- Depends on: WP-DEL1.
- Acceptance criteria: every question in the attempt is reachable from the bar; the current
  question is identified by more than color; the bar is keyboard operable in reading order and
  survives the tablet and narrow-phone profiles.
- Evidence or review, when useful: `image_evaluator`; axe on the attempt surface.
- Obvious follow-ons: WP-NAV2.

### Work package: WP-NAV2 show per-question saved state

- Owner: `coder`.
- Touch points: the navigation component; the per-question state source in
  `src/features/question_attempt/question_attempt_state.ts`.
- Depends on: WP-NAV1.
- Acceptance criteria: Student navigation represents unanswered and saved response state, one
  position per Question. Submission is an Assignment Attempt state, tracked where the attempt is
  tracked. The saved state survives reload; a non-color cue accompanies it; the existing warning
  about unsaved responses stays accurate or is replaced by real persistence.
- Evidence or review, when useful: state-matrix evidence for unanswered, saved, and current, plus
  a reload journey.
- Obvious follow-ons: if the current buffer is not persisted, scope persistence explicitly rather
  than showing a saved state the server does not hold.

### Work package: WP-VOC1 remove Instructor nouns from Student text

- Owner: `coder`.
- Touch points: `src/pages/student_courses_page.tsx`, `student_course_landing_page.tsx`,
  `student_course_invitations_page.tsx`, `student_course_invitation_page.tsx`,
  the surviving Student delivery components.
- Depends on: none.
- Acceptance criteria: no Student-visible string contains `Course Instance`,
  `Released Assignment`, or a `C-n`-style reference; library Question Titles are withheld from
  Students; score and attempt copy reads in Student language.
- Evidence or review, when useful: `reviewer` text sweep; refreshed Student screenshots.
- Obvious follow-ons: WP-VOC2.

### Work package: WP-VOC2 auto-enter a single active Course

- Owner: `coder`.
- Touch points: `src/pages/student_courses_page.tsx`, `src/pages/course_list_page.tsx`.
- Depends on: WP-VOC1, to avoid two edits to the same component.
- Acceptance criteria: a Student with exactly one current Course lands on that Course; a Student
  with none keeps the current empty state; a Student with two or more still chooses; the
  redirect leaves a way back to the course list.
- Evidence or review, when useful: three browser journeys, one per count.
- Obvious follow-ons: none.

### Work package: WP-ACC1 extend the Assignment access contract

- Owner: `expert_coder`.
- Touch points: `src/api/assignment_attempt_issuance.ts`, its decoder, the server assignment
  access route, and the Store projection behind it.
- Depends on: WP-DEL1, WP-VOC1, WP-DEF2.
- Acceptance criteria: the response carries question count, points possible, time limit, and
  previous-attempt history; it carries no answer, key, or per-item correctness beyond the
  Instructor's disclosure setting; anonymous, foreign, and non-member callers keep the current
  concealed refusal.
- Evidence or review, when useful: independent security review; a no-transport probe for each
  refused caller.
- Obvious follow-ons: WP-ACC2.

### Work package: WP-ACC2 present the Start Assignment facts

- Owner: `coder`.
- Touch points: the Student start surface,
  `src/components/student_assignment_presentation.tsx`.
- Depends on: WP-ACC1.
- Acceptance criteria: the page shows the Assignment title rather than a hard-coded heading, plus
  question count, points possible, time limit, and previous scores and attempts; the primary Start
  action stays visually dominant; the existing component is reused rather than duplicated; previous
  attempts open without exposing answers unless the setting allows it.
- Evidence or review, when useful: `image_evaluator` at the Student profiles; axe on the surface.
- Obvious follow-ons: none.

### Work package: WP-ZONE1 add account-owned IANA zones

- Owner: `expert_coder`.
- Touch points: `crates/learning-data-access/src/instructor_account.rs` and its PostgreSQL module,
  the Student Account and enrollment path, allocated forward migrations.
- Depends on: WP-DOC3 for the recorded model.
- Acceptance criteria: an Instructor Account and a Student Account each carry an IANA zone; the
  Student's defaults from the Instructor's at enrollment and is thereafter the Student's own; the
  column enforces IANA membership in the schema as well as in Rust, since the current Course zone
  column's guarantee lives only in Rust; the preference lands in a new table or a new mutable
  column, because the three existing account columns are trigger-immutable; account setup
  establishes a valid zone before wall-clock entry becomes available, and stored instants stay
  usable and renderable throughout because they carry no zone; a clean-volume migration run and a
  second no-op run both pass.
- Evidence or review, when useful: `architect` on schema and authority; fresh and no-op migration
  evidence.
- Obvious follow-ons: WP-ZONE2.

### Work package: WP-ZONE2 replace the Course-local input DTO

- Owner: `expert_coder`.
- Touch points: `crates/question_model/src/assignment/teaching_settings_local.rs`,
  `crates/question_model/src/course_term.rs`, the assignment Store read and write paths, the
  generated TypeScript bindings, `src/pages/assignment_workspace/assignment_workspace_policy_model.ts`.
- Depends on: WP-ZONE1, because interpretation needs an account zone to read.
- Acceptance criteria: the input DTO carries a plain local date and time, and the server takes its
  interpretation zone from the authenticated account; the resolver at the data-access boundary
  interprets in that zone and keeps refusing DST gaps and ambiguity; a stored deadline is an
  instant; the browser sends an unconverted wall-clock string and the server owns every conversion;
  the existing zone-mismatch guard either becomes the live path or is removed, since it has no
  production caller today.
- Evidence or review, when useful: `reviewer`; the existing DST refusal tests as the regression
  boundary; a round-trip in a zone other than the Course's former one.
- Obvious follow-ons: WP-ZONE3.

### Work package: WP-ZONE3 retire the Course zone field

- Owner: `expert_coder`.
- Touch points: `schemas/migrations/` for one forward migration, `course_schedule_revision`
  readers, `crates/question_model/src/course_term.rs`, `crates/question_model/src/course.rs`,
  every read site the audit enumerated.
- Depends on: WP-ZONE2, which removes the last interpretation dependency on it.
- Acceptance criteria: account zones are the only wall-clock concept present in the schema, the
  Rust model, and the browser; the course-term calendar bound the field currently participates in
  is preserved by another named means or recorded as deliberately dropped with its reason; the
  migration is forward-only with a clean-volume and no-op run.
- Evidence or review, when useful: `architect`; a grep-clean sweep for the retired names.
- Obvious follow-ons: none.

### Work package: WP-TZ2 repair the browser-local zone leaks

- Owner: `coder`.
- Touch points: `src/components/student_assignment_presentation.tsx`,
  `src/pages/gradebook_assignment_attempt_chooser.tsx`, `src/pages/instructor_accounts_page.tsx`,
  `src/pages/assignment_release_page.tsx` or its surviving replacement.
- Depends on: the M0 zone-ownership decision.
- Acceptance criteria: no rendered timestamp falls back to the browser zone by omitting a
  `timeZone` option; Instructor surfaces render in the Instructor's zone and Student surfaces in the
  Student's; the due-date editor names the zone the Instructor's typing is interpreted in; the
  attempt countdown keeps consuming a server-computed remaining duration rather than a wall clock.
- Evidence or review, when useful: `reviewer` sweep of every `Intl.DateTimeFormat` and
  `toLocaleString` call site.
- Obvious follow-ons: M17's Profile page gives the Instructor a place to change the zone this
  package renders in.

### Work package: WP-PRO1 add the Instructor Profile record and route

- Owner: `expert_coder`.
- Touch points: a new server route, `src/route_contract.ts`, `src/routes.ts`, a new page.
- Depends on: WP-ZONE1, which owns the account zone storage; WP-RIB1 for the Context Control.
- Acceptance criteria: an Instructor reads and saves their IANA zone through the Profile page, with
  the value validated by the existing strict exact-name check rather than free text; saving a new
  zone re-renders existing deadlines in it without moving them; the page states that effect plainly
  so an Instructor is not surprised; the Profile route is reachable as a Ribbon Context Control and
  renders with No Selected Ribbon Tab, matching the existing Context Control routes.
- Evidence or review, when useful: `reviewer`; a browser journey that changes the zone and confirms
  a deadline's rendered wall clock changes while its instant does not.
- Obvious follow-ons: WP-PRO2. Also decide whether the unused resolver carrying the zone-mismatch
  guard becomes the live write path or is removed, since it has no production caller today.

### Work package: WP-PRO2 add the profile thumbnail

- Owner: `expert_coder`.
- Touch points: the Profile page; the existing server-owned upload and rendition boundary used by
  Course Banner.
- Depends on: WP-PRO1.
- Acceptance criteria: a useful source image is accepted regardless of its aspect ratio, validated
  as decoded image bytes, cropped through the existing server-owned rendition machinery to one
  fixed thumbnail identity, and delivered under the caller's authorized scope; no caller selects a
  path, crop, or rendition; the thumbnail renders behind a consistent rounded-rectangle silhouette
  wherever it appears; Students gain no upload capability.
- Evidence or review, when useful: reuse of the Course Banner saga's evidence pattern; a
  non-square source image producing a correct centered thumbnail.
- Obvious follow-ons: none. The notes' original 256 by 256 square ceiling is superseded by the
  owner's crop decision; keep a size ceiling, drop the square requirement.

### Work package: WP-DUE1 add the cross-course due-window read path

- Owner: `expert_coder`.
- Touch points: chosen by the owner from existing patterns; the audit points at the assignment
  Store and route family and the cross-course membership join already used by the Course list.
- Depends on: WP-DOC3 for the display-zone rule.
- Acceptance criteria: the projection contains Course identity, Assignment identity, Assignment
  Status, and the due instant, for Courses the caller currently teaches; it reuses the existing
  membership predicate; it follows whatever list shape neighbouring Instructor lists already use.
  Keep it as small as the requirement allows: add pagination machinery, an index, or a new
  abstraction only if the repository or a measured need calls for it, and say which one did.
  Student identity, responses, and grading state stay outside the projection.
- Evidence or review, when useful: `reviewer` on authorization; a probe confirming a revoked
  membership drops its rows; migration evidence only if the chosen shape needs a migration.
- Obvious follow-ons: WP-DUE2.

### Work package: WP-DUE2 present the Due Soon page

- Owner: `coder`.
- Touch points: a new page, `src/route_contract.ts`, `src/routes.ts`,
  `src/ribbon/ribbon_catalog.ts`, `src/ribbon/capability_registry.ts`.
- Depends on: WP-DUE1.
- Acceptance criteria: the page lists due Assignments across Courses with the Course named per row
  and the due time rendered in the decided zone; it uses the dense row pattern from WP-DEN1; the
  empty state says what is absent and what to do next; the destination resolves Available in the
  generated ledger.
- Evidence or review, when useful: `image_evaluator` at 1280 by 800; regenerated ledger.
- Obvious follow-ons: none.

### Work package: WP-CST1 represent Course activity

- Owner: `expert_coder`.
- Touch points: chosen from the existing retention model; `docs/DATABASE_STRUCTURE.md` for any
  allocation; the Course read model and its Store.
- Depends on: WP-DOC3 for the definition.
- Acceptance criteria: the representation answers exactly one question, whether this Course is a
  current teaching context or a previous-semester one whose FERPA-sensitive Student data has been
  stripped; it derives from the existing retention transitions where they already express it rather
  than adding a parallel lifecycle; it adds no state the definition does not require; whether a
  Course becomes Inactive by term end, by explicit action, or by retention transition is recorded
  with the reason.
- Evidence or review, when useful: `architect` on the FERPA boundary; migration evidence if the
  chosen representation needs one.
- Obvious follow-ons: WP-CST2.

### Work package: WP-CST2 present the Active and Inactive lists

- Owner: `coder`.
- Touch points: `src/pages/course_list_page.tsx`, `src/route_contract.ts`,
  `src/ribbon/ribbon_catalog.ts`, `src/ribbon/capability_registry.ts`.
- Depends on: WP-CST1; WP-DEN1 for the row pattern.
- Acceptance criteria: both destinations resolve Available in the generated ledger; an Inactive
  Course row presents its non-sensitive metadata only; each list's empty state says what is absent
  and what to do next; both reuse the dense row pattern and the Course Theme identity treatment.
- Evidence or review, when useful: `image_evaluator`; regenerated ledger; a probe over the Inactive
  row projection confirming its fields are limited to non-sensitive Course metadata.
- Obvious follow-ons: none.

### Work package: WP-EVI1 add Student axe coverage

- Owner: `tester`.
- Touch points: `tests/playwright/`, following the existing Course Appearance axe pattern.
- Depends on: the Student surfaces this plan changes.
- Acceptance criteria: axe runs against the Student course list, course landing, start, and attempt
  surfaces; the gate fails on serious and critical impact; the run stays out of the fast pytest
  lane.
- Evidence or review, when useful: the axe report itself.
- Obvious follow-ons: WP-EVI3 records whether this belongs in `check_codebase.sh`.

### Work package: WP-EVI2 refresh the screenshot corpus

- Owner: `playwright_operator` with `image_evaluator`.
- Touch points: `tests/playwright/screenshot_corpus/`, `docs/screenshots/`, the capture manifest
  and its receipt.
- Depends on: every milestone whose captured surface changed.
- Acceptance criteria: each changed checkpoint is recaptured at its manifest viewport profiles; the
  receipt's digest and path set match the captured files; the coverage ledger still accounts for
  every route.
- Evidence or review, when useful: the manifest receipt.
- Obvious follow-ons: none.

### Work package: WP-EVI3 close out documentation

- Owner: `maintainer`.
- Touch points: `docs/CHANGELOG.md`, `docs/TODO.md`, the published plan under
  `docs/active_plans/active/`.
- Depends on: WP-EVI1, WP-EVI2.
- Acceptance criteria: changelog entries use the repository's six category headings; the deferred
  capability items appear in `docs/TODO.md` with their reasons; the plan moves to `docs/archive/`
  by `git mv` when closed.
- Evidence or review, when useful: `tests/test_markdown_links.py`.
- Obvious follow-ons: none.

## Acceptance criteria and gates

- Per-patch gate: `./check_rust.sh` when a crate changed; `./check_codebase.sh` always;
  `source source_me.sh && python3 -m pytest tests/test_markdown_links.py tests/test_ascii_compliance.py tests/test_whitespace.py`;
  `git diff --check`.
- Integration gate: the Ribbon destination ledger regenerated and matching; every internal link
  resolving to a contract route; one keyboard-only Student attempt journey passing; no serious or
  critical axe finding on a changed Student surface.
- Independent review gate, when useful: `architect` on M0, M13, M16, and M19; independent security review on
  WP-ACC1; `reviewer` on WP-DEF2 disclosure defaults and on every retirement.

## Test and verification strategy

Match the layer to the claim, per
[docs/TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md)
and PYTEST_STYLE.md:

- Fast offline checks for durable contracts: route-contract shape, Ribbon catalog and schema
  derivation, theme token derivation, and Rust `Default` impls. Assert behavior rather than counts
  or required-key lists. The settings defaults are a deliberate exception: 11:59 PM, late work
  rejecting, and both prohibit-after-due behaviors defaulting on are explicit product requirements
  now, so permanent tests assert them. Avoiding brittle tests means not asserting incidental
  constants; a chosen product default deserves a test, and changing the test is the right move if
  the owner later changes the default.
- Node e2e for the generated Ribbon ledger, the icon sprite build, and the component contract.
- Playwright for the Student keyboard journey, the start-and-resume journey, the three
  single-course counts, inline edit save and failure, and axe.
- Visual evidence through `image_evaluator` at the canonical profiles for density, the navigation
  bar, the timer, and theme identity in rows.
- Failure semantics: a failed per-patch gate blocks that patch; a failed integration gate blocks
  the milestone; a serious or critical axe finding on a changed Student surface blocks M9.
- Treat implementation-proof checks as temporary by default and promote one into the permanent
  suite only when it passes the PYTEST_STYLE.md checklist.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Promoting the attempt lane loses a capability the overview lane held | Student flow regresses | WP-DEL1 diff drops a state or recovery path the overview page owned | `expert_coder` | Inventory both lanes' states before deleting either; keep the retired lane's recovery paths as named acceptance rows |
| Retiring the release page drops release or validate behavior | Instructors cannot release | WP-RET1 removes a control with no surviving home | `expert_coder` | Enumerate the retired page's controls first; record each as moved or deliberately dropped |
| Task-row labels imply capability that does not exist | Instructors click dead links | Active/Inactive or public Blueprint search rendered as live links | `coder` | Keep unbacked positions Unavailable; the generated ledger is the check |
| Widened Assignment access response leaks answers | Grading secrecy failure | WP-ACC1 adds previous attempts with per-item detail | `expert_coder` | Independent security review; disclosure setting gates every added field |
| Density change breaks the narrow-phone guard | Student or Instructor clipping | WP-DEN1 token change | `coder` | Validate 1280 by 800 plus tablet and narrow guard after any token change, per the design guide |
| A second zone override hierarchy survives | Speculative flexibility becomes permanent cruft | A package keeps Course-zone authority beside the account zones | `architect` | Account zones are the sole wall-clock interpretation and presentation context; the Course field retires with the concept |
| An account reaches wall-clock entry with no valid stored zone | Entered deadlines land on an unintended instant | Migration, damaged state, or an incomplete setup leaves the zone unset | `expert_coder` | Account setup establishes a valid zone before wall-clock entry becomes available; stored instants stay usable and renderable throughout, since they carry no zone |
| Notes' defaults get treated as permanent test assertions | Brittle suite | A test asserts 11:59 PM or reject | `tester` | Assert behavior, not the tuned constant, per PYTEST_STYLE.md |
| Blackboard reference material implemented as PLE policy | Product policy the owner never chose | An M4 field traces to the notes' Blackboard section rather than a named owner decision | `reviewer` | Every settings field cites its resolved-decision line; unlisted Blackboard entries stay reference |
| A preference lands before the model it depends on | Two competing zone authorities and wrong due times | M17 starts before M13 and M14 | `architect` | M13 and M14 gate M17; the model decision is recorded in M0 before any preference lands |
| The released-Assignment status check blocks ordinary editing | The owner's settled mutable-state decision is not delivered | WP-INL1 meets the save path's `unreleased` refusal | `expert_coder` | Current authoring state is mutable by decision; the package relaxes that check and determines how issued and graded evidence stays pinned while it does |
| Blueprint provenance column is mistaken for per-assignment lineage | A future divergence check compares against the wrong baseline | Someone treats `assignment.source_blueprint_*` as an assignment-level source | `architect` | Record in M0 that this column is course-level origin copied onto every Assignment, including hand-authored ones |
| Plan drifts from implementation across nine milestones | Stale plan | A milestone lands without updating the published plan | `maintainer` | Update the published plan at each milestone exit alongside the changelog |

## Rollout and release checklist

- [x] M0 documents agree; no contradictory tab set remains.
- [x] One Instructor Assignment editing path; no dead internal link.
- [ ] Ribbon ledger regenerated and matching the catalog.
- [ ] One Student delivery lane with navigation bar, saved state, and timer.
- [x] Student-visible text carries no Instructor noun.
- [ ] Start Assignment shows title, counts, points, time limit, and previous attempts.
- [ ] A deadline is a zone-free instant; Instructor entry and Student display each use their own
      account zone; no Course-owned wall-clock concept remains.
- [ ] Changing a profile zone re-renders deadlines without moving them.
- [ ] Profile preferences persist with a clean-volume and no-op migration run.
- [x] Account-zone preference storage passes clean-volume and no-op migration acceptance; Profile
      editing remains M17 work.
- [x] Instructor wall-clock entry uses the authenticated account zone to store a deadline instant;
      M15 display and M16 Course-zone retirement remain separate milestones.
- [x] Question presentation owns answer-choice randomization on PLE-native Questions; backend
      questions keep their backend's presentation.
- [ ] Assignments Due Soon lists only Courses the Instructor currently teaches.
- [ ] Inactive Courses show non-sensitive metadata and no FERPA-sensitive Student data.
- [ ] Student axe coverage passing; screenshot receipt matching.
- [ ] `docs/CHANGELOG.md` records accepted evidence; deferred items routed to `docs/TODO.md`.
- [ ] Release acceptance remains governed by `docs/ROADMAP.md`; this plan authorizes no deployment.

## Documentation close-out requirements

- Active plan / progress tracker: publish this plan to
  `docs/active_plans/active/interface_cleanup_2026_09.md` and update it at each milestone exit;
  `git mv` it to `docs/archive/` at closure.
- docs/CHANGELOG.md entry: one dated block using the repository's six category headings, recording
  the retirements and the deferred capability items under Decisions and Failures.
- Archive / closure notes: keep the retirement inventory from WP-RET1 in the plan so a later reader
  can see which controls were deliberately dropped.

## Patch plan and reporting format

Patch order follows dependencies, not milestone numbers. Each patch is one independently verifiable
milestone.

- Patch 1: M0 documentation ownership and the five settled decisions.
- Patch 2: M1 retirement, link repair, and the composition surface.
- Patch 3: M6 Student vocabulary and single-course entry.
- Patch 4: M5 Student delivery, navigation bar, and timer.
- Patch 5: M12 Question-owned answer randomization.
- Patch 6: M13 account-owned time zones.
- Patch 7: M14 wall-clock input interpretation.
- Patch 8: M15 account-zone display.
- Patch 9: M2 Ribbon shape, dense top bar, and breadcrumbs.
- Patch 10: M3 density and Course identity.
- Patch 11: M16 retire the Course zone.
- Patch 12: M17 Instructor Profile page.
- Patch 13: M18 profile thumbnail.
- Patch 14: M4 Assignment settings defaults.
- Patch 15: M11 inline title and due-date editing.
- Patch 16: M7 Start Assignment facts.
- Patch 17: M10 Assignments Due Soon.
- Patch 18: M19 Active and Inactive Courses.
- Patch N: M9 evidence, changelog, and remaining repository-required work.

## Resolved decisions

All of these were stated by the owner during this planning session and belong in
`docs/HUMAN_GUIDANCE.md` and `docs/DESIGN_DECISIONS.md` at M0.

- Document ownership: `docs/HUMAN_GUIDANCE.md` and `docs/TERMINOLOGY_CONTRACT.md` are the
  human-approved authorities. `docs/INTERFACE_TERMINOLOGY.md` holds terminology; Ribbon structural
  rules move to `docs/UI_DESIGN_GUIDE.md`.
- Instructor Product Ribbon: `Courses | Questions | Assignments`. The three categories match what
  an Instructor actually manages. Blueprint Courses, active courses, rosters, and course settings
  belong under Courses; drafts, owned questions, and library search under Questions; templates,
  due-soon work, editing, release, and grading under Assignments. The current route hierarchy does
  not constrain this taxonomy.
- Ribbon layout: the tabs join the identity band as one information-dense top bar,
  `Peptidyle | Instructor | Name | Courses | Questions | Assignments | Profile | Sign out`. This is
  the owner's preferred design and reads as a traditional application menu bar while lifting content
  higher on the screen. Build the single dense bar first and keep a separate Tab Row only where
  responsive or focus-order evidence shows a concrete failure at a named viewport profile.
- Questions task row: `My Questions | My Draft Questions | Starred | Watched |
  Search Question Library | Browse Question Library`, in that order. Starred and Watched stay,
  reserved until backed.
- Ribbon icons: every visible Ribbon navigation item carries a Font Awesome glyph plus text. Mixing
  icon-and-text items with text-only peers reads as unfinished. Extend the existing subsetted
  same-origin sprite vocabulary as needed.
- Assignment editing: composition and delivery are two different Instructor jobs. The primary
  editing surface focuses on selecting, adding, removing, and ordering Questions; timing, release,
  scoring, attempts, randomization, late-work, and disclosure live on the settings surface. Visible
  names are secondary. The five section routes at `src/route_contract.ts:276-336` are already this
  split; the remaining work is retiring the duplicate older page and rebalancing the surfaces, not
  a code-wide rename.
- Assignment Settings requirements promoted from the Blackboard reference section: prohibit late
  submissions defaults on, and prohibit new attempts after due defaults on. Final grade calculation
  is omitted unless the grading model already needs it. The rest of that section stays reference
  material. Blackboard's two randomization entries are superseded by the two randomization
  decisions below: `Randomize question order` is the Assignment setting, and answer-choice order is
  Question-owned.
- Assignment Settings requirements from the notes' own PLE sections: due times default to 11:59 PM;
  late work defaults to reject; one question per page always; optional Instructor question-order
  shuffle; time limit is an Assignment Setting; previous-attempt answer visibility is
  Instructor-controlled.
- Assignments Due Soon is worth building rather than deferring. It answers a real cross-course
  Instructor question. Its cost is being measured before dispatch rather than assumed.
- Time-zone model. An Assignment deadline has no time-zone ownership at all; it is an instant.
  The full model:
  1. An Instructor has an IANA zone, for example `America/Chicago`.
  2. The Instructor enters a wall-clock value, for example `2026-09-20 11:59 PM`.
  3. The server interprets that value in the Instructor's zone, with the existing DST gap and
     ambiguity refusal.
  4. The resulting absolute instant is stored as `timestamptz`, which the schema already does.
  5. Each Student has an IANA zone, and the same instant is formatted in it.
  6. A Student's initial zone may default to the Instructor's at enrollment or setup; afterward it
     is that Student's own presentation preference.

  So the Instructor's zone is not a property of the Assignment. It is the interpretation context
  for wall-clock values that Instructor enters, plus that Instructor's display preference. A
  globally distributed online course needs no fictitious Course clock, so `CourseLocalDateAndTime`
  is the wrong concept: the input DTO should carry a plain local date and time and take its zone
  from the authenticated account, not from the Course model. The Course zone field is retired with
  it.
- Changing a profile zone never moves an existing deadline. Stored values are already instants, so
  only their displayed wall-clock representation changes. Values entered after the change are
  interpreted in the new zone.
- Assignment and Draft Question state: be conservative about tracking revisions. Both have one
  current editable state. Changing a due date, title, timing rule, or any other setting changes
  that state, with no revision history and no undo trail. Immutable snapshots are preserved only
  where Student attempts, issued Questions, or grading evidence concretely depend on them; the
  implementation reads those existing consumers and keeps only what they actually require.
- An Inactive Course is a previous-semester Course whose FERPA-sensitive Student data has been
  stripped while its non-sensitive Course metadata remains. Active and Inactive are therefore a
  real domain concept, not a list filter.
- A profile image is a consistent thumbnail, not a square-source requirement. Reuse the existing
  crop and rendition machinery and present the result through a consistent rounded-rectangle
  silhouette rather than rejecting useful source images for not already being square.
- The Assignment randomization setting is `Randomize question order`. Blackboard's `Randomize pages`
  does not enter PLE, because PLE always shows one Question per page.
- Question presentation owns answer randomization. A Question may declare whether its answer
  choices are randomized when presented. An Assignment may randomize Question order, but it does
  not override answer-order behavior across its Questions. Question order changes the sequence of
  Questions within one Assignment Attempt, which is Assignment business; choice order is a property
  of that Question's own choices, which is Question business. This also scales across Question
  Types: multiple-choice and multiple-answer can support shuffled choices, while numeric,
  free-response, matching, ordering, and hotspot have different or no meaningful choice-order
  behavior. An Assignment-wide switch would be meaningless or wrong for several of them.
- A Question Backend question is presented by its backend. A WeBWorK question is rendered by the
  WeBWorK renderer and PLE controls none of its presentation, so an answer-randomization
  declaration applies to PLE-native Questions only. The same holds for iMathAS. PLE does not claim
  a presentation control it cannot enforce.
- Blueprint divergence: a Course Instance Assignment is already fully instance-owned. Blueprint
  assignments are never materialized into an instance at all, so there is no constraint to remove.
  The Blueprint direction stays out of this plan.

## Open questions and decisions needed

This plan is finishable without another question to the owner. Every item below has a decision rule
the manager applies from repository evidence.

- Manager/subagent decision procedure:
  - Decision owner or dedicated class: `architect`, in WP-DOC2 and WP-DOC3.
  - Evidence and decision rule: use the owner's stated terminology wherever this plan's resolved
    decisions settle it. Where they do not, choose the smallest naming change consistent with the
    existing domain model, and record the choice. Where the notes and the current tree disagree on
    capability rather than naming, the capability stays reserved and Unavailable until its complete
    usable path is backed.
- Non-blocking follow-up, each an implementation investigation with its own decision rule:
  - Which visible names the Assignment surfaces carry. The owner has settled that names are
    secondary, so apply the decision rule above: use the owner's terms where stated, otherwise the
    smallest change consistent with the domain model.
  - Whether `Validate Assignment` survives. The notes lean toward removing it and the retired page
    is its only home, so WP-RET1 decides it while enumerating that page's controls.
  - Whether an accessibility gate belongs in `check_codebase.sh`, decided in WP-EVI3 from the
    measured runtime of the new axe coverage.
  - What replaces the course-term calendar bound that the retired Course zone field currently
    participates in, or whether that bound is deliberately dropped. WP-ZONE3 answers it; the
    account-zone model holds either way.
  - Whether Question Publication Validation should reject an answer-randomization declaration on a
    Question Type that cannot use it. WP-QRAND notes it; the validation work is its proper home.
