# Plan: instructor assignment workspace and live Student view

## Status

Planning state: owner-directed and ready for implementation on 2026-08-26.

This plan is the binding execution plan for `WP-PROF-T6`. It follows accepted
`WP-PROF-T3`, `WP-PROF-LD3`, and `WP-PROF-T5` and becomes the immediate prerequisite for
`WP-PROF-G1`. The package creates one coherent Instructor assignment workspace before the grading
operations packages add more assignment-local work.

T6 owns forward capability migration `2026081848`. The migration permits persisted assignments with
empty ordered definitions while their lifecycle is `Draft` or `Archived`; a `Published` assignment
must satisfy publication readiness with at least one active deliverable position. The existing
assignment, policy, preview, and learner-projection records remain authoritative. Allocate this
migration before schema edits; land the T6-W1 shared domain contract first, then implement the
migration and PostgreSQL capability while enforcing the readiness boundary on every transition to or
while `Published`.

Parallel-plan ready: yes. The domain/API contract lands first; the Questions, Policies, workspace,
and Student-view surfaces can then proceed in focused lanes before connected integration.

## Context

The current course assignment card renders the assignment title as plain heading text. Its primary
Instructor action jumps directly to `/edit`, so a familiar object-selection gesture does nothing and
the destination has no assignment-local home. This violates the user's expectation that selecting
the named assignment opens that assignment.

The current `AssignmentEditorPage` also combines four different teaching tasks in one surface:

- assignment identity and ordered fixed-or-pool content;
- question discovery, reuse, addition, replacement, and pool inspection;
- run, disclosure, delivery, and lifecycle policy; and
- links to access modifiers and effective-delivery inspection.

Those tasks have different mental objects, mutation rules, and completion evidence. Keeping them in
one page makes the next action harder to identify and makes future grading-operation work compete for
the same oversized editor.

ADAPT supplies useful comparison evidence. Its assignment name opens a dedicated Questions route,
and Questions and Properties are separate child routes. It also exposes an Instructor/Student view
switch. Its implementation changes the authenticated identity to a generated fake-student account
and replaces the session token. Peptidyle adopts the discoverable navigation and separate task
surfaces while preserving its ordinary account, enrollment, immutable evidence, and live-demo model.

Peptidyle already has the stronger foundations needed for this design:

- `LearnerAssignmentDetail` is an answer-free learner projection and is already available to an
  authorized Instructor opening a learner-facing route;
- `WP-PROF-T3` owns non-mutating policy inspection over current live records;
- `WP-PROF-T5` owns fixed-or-pool assignment definitions and server-generated samples; and
- the canonical demo Student owns ordinary runs, submissions, receipts, grades, and Instructor-visible
  gradebook evidence.

## Objectives

1. Make the assignment title the obvious semantic entry point from the course assignment list.
2. Give every assignment a durable Instructor home with assignment-local navigation and status.
3. Put ordered questions and pools on a Questions page and delivery rules on a Policies page.
4. Let an Instructor select **Student view** while retaining the Instructor account and course
   authority.
5. Render Student view from the current live assignment and the same answer-free learner projection
   used by ordinary delivery.
6. Preserve ordinary enrolled Student activity as the only source of learner runs, submissions,
   receipts, scores, and gradebook evidence.
7. Replace complete-editor writes with revision-checked, capability-focused commands so each page
   updates only the part of the assignment it owns.
8. Make draft creation naturally support a multi-page workspace: create the draft, add questions,
   review policies, and publish when the server reports it ready.
9. Leave a stable assignment-local navigation seam for `WP-PROF-G1` through `WP-PROF-G5` without
   turning the course-level navigation into an assignment dashboard.

## Design philosophy

- **Match the teaching object.** A named assignment opens the assignment; local pages then expose
  the Instructor's distinct Questions, Policies, and Student-view tasks.
- **Make information scent explicit.** Link text, page headings, and navigation labels agree. The
  title link satisfies the intent of
  [WCAG 2.2 SC 2.4.4](https://www.w3.org/WAI/WCAG22/Understanding/link-purpose-in-context.html):
  a person can predict its destination from the link.
- **Use one live product model.** Student view reads the current ordinary assignment. The ordinary
  demo Student performs graded validation, and the ordinary Instructor observes that work.
- **Keep identity stable.** Student view is a server-authorized presentation mode for the current
  Instructor session. Role switching remains explicit account entry rather than an assignment-page
  side effect.
- **Keep mutations capability-focused.** Questions and Policies submit closed request types, carry
  the current assignment revision, and receive the new authoritative revision.
- **Preserve one aggregate revision.** Focused commands update one owned slice of the assignment but
  still serialize through the assignment's shared strong revision. Concurrent edits conflict and
  recover visibly instead of silently overwriting another page.
- **Allow honest incomplete drafts.** Draft creation records a real server-owned assignment with
  defaults. Publication validation, rather than creation ceremony, enforces the complete teaching
  contract.
- **Design the 1280 by 800 Instructor workspace.** Use compact local navigation, useful width, and
  visible next actions. Student delivery retains its existing responsive contract in the ordinary
  Student journey.
- **Prefer durable behavior gates.** Permanent tests protect route, authorization, mutation, and
  accessibility contracts. Visual composition, ADAPT comparison, route inventory, and wire
  inspection are one-time package evidence.

## Scope

- Instructor course assignment cards and assignment-title navigation.
- A new assignment-local Instructor home and shared assignment navigation.
- A Questions route for title, ordered fixed questions, pools, selection, reuse, replacement, and
  pool sample actions.
- A Policies route for learner instructions, run policies, disclosure, lifecycle, base schedule,
  limits, lateness, and links into access/accommodation and delivery inspection.
- A Student-view route that composes the current learner-facing assignment landing surface inside a
  clearly labeled Instructor context.
- A focused create-draft command and focused revision-checked content and policy commands.
- A shared learner assignment presentation component used by ordinary learner overview and
  Instructor Student view.
- Existing route, API-client, generated-contract, documentation, browser-journey, and screenshot
  updates required by the new canonical paths.
- Removal of the combined `/edit` route and combined editor page after every caller has moved to the
  canonical assignment workspace routes.

## Non-goals

- Run execution under an Instructor identity.
- A generated test-student account, implicit session replacement, or hidden role toggle.
- A second assignment, policy, question, or learner-data model for preview.
- Human scoring, score override, recalculation, learner-work inspection, item analysis, or the
  `WP-PROF-G1` through `WP-PROF-G5` operation surfaces.
- A new persistent preview record or preview-specific migration.
- New fake courses, assignments, learners, observations, or screenshot-count requirements.
- Pixel equivalence, timing thresholds, route-inventory assertions, or source-text assertions.

## User task model and evaluation method

The HCI method is a hierarchical task analysis followed by a cognitive walkthrough of the connected
1280 by 800 Instructor journey. The walkthrough asks four questions at each decision point: will the
Instructor know the next goal, notice the control, connect the control to the goal, and understand
the resulting state?

| Task                        | Trigger and precondition                             | Minimal visible path                                            | Completion evidence                                                                                           |
| --------------------------- | ---------------------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Open an assignment          | Instructor is on a managed course's Assignments page | Select the linked assignment title                              | Assignment home heading matches the selected title; Overview is current                                       |
| Organize questions          | Assignment home is open                              | Select Questions; add, reuse, reorder, pool, or replace; save   | Ordered server response and new revision are shown; reload preserves the order                                |
| Configure delivery          | Assignment home is open                              | Select Policies; change the relevant rule; save                 | Server-derived current state and revision are shown; conflicts preserve typed values                          |
| Inspect the learner landing | Assignment home is open                              | Select Student view                                             | Persistent mode cue, exact assignment title/instructions/delivery facts, and Return to assignment are visible |
| Validate live grading       | Ordinary Student is enrolled                         | Enter as Student; complete the assignment; return as Instructor | Student receives the server grade and Instructor sees the same score in the ordinary gradebook                |

Expected slips and recovery:

- A stale revision retains the page's entered values, identifies that the assignment changed, and
  offers **Reload latest assignment** before another save.
- A draft with no questions stays a valid draft. The page names **Add at least one question** as the
  next step, and publication remains unavailable until server validation succeeds.
- A Student-view request for an unrelated course/assignment pair returns the common not-found
  surface and no assignment facts.
- An Instructor who no longer manages the course returns to the course list through the existing
  access boundary.
- A transport failure keeps the current page and entered values and offers a semantic retry.

## Product interaction contract

### Course assignment list

- Render each Instructor-visible assignment title as a Solid router link to the assignment home.
- Use the assignment title as the link's accessible name; keyboard activation and pointer activation
  have the same destination.
- Show compact lifecycle, due-state, and question-count facts when already present in the list
  projection. The list does not make per-card requests for Instructor facts.
- Keep secondary card actions distinct: **Questions**, **Policies**, and **Student view** may appear
  as compact links when the 1280 by 800 composition remains scannable. The title remains the primary
  entry point.
- Pagination focus recovery targets the first appended assignment title link.

### Assignment home and local navigation

The assignment home is an Instructor summary, not a second editor. It shows:

- assignment title, lifecycle/current state, course, and current revision;
- question and pool counts;
- learner instructions and a compact effective schedule summary;
- draft-readiness or publication state with the next useful action; and
- links to Questions, Policies, Student view, access/accommodations, and delivery inspection.

One shared `<AssignmentWorkspaceNav>` renders **Overview**, **Questions**, **Policies**, and
**Student view** with semantic links and `aria-current="page"`. Access/accommodations and delivery
inspection stay contextual actions from Policies rather than expanding the primary local navigation.
Course management navigation remains above the assignment-local navigation and continues to answer
the broader "where am I in this course?" question.

### Questions page

Questions owns the assignment definition slice:

- assignment title;
- ordered fixed questions and selection groups;
- shared ProblemPicker selection and saved-selection reuse;
- direct Question ID entry as the occasional fallback;
- pool definition and server-generated pool sample;
- accessible reorder, add, remove, and future-run replacement; and
- one **Save questions and order** action.

The page does not render run, disclosure, schedule, lifecycle, entitlement, or accommodation fields.
It links to Policies when server publication readiness reports a policy issue.

### Policies page

Policies owns the complete delivery-policy slice:

- learner instructions;
- completion, continued-practice, variation, and grade policies;
- disclosure timing;
- lifecycle and scoring status;
- base availability, due, close, time-limit, attempt-limit, late-work, and deadline behavior; and
- current links to access/accommodations and the resolved delivery check.

One **Save assignment policies** action submits one closed policy request and receives one complete
server result. The server evaluates contextual consistency and returns field-linked validation so
the page can focus the first invalid control. Publication uses the same validation result and names
the Questions page when content remains incomplete.

### Student view

- The visible action is named **Student view**, matching the familiar teaching task.
- The page retains the Instructor session and requires current direct Instructor authority for the
  exact course and assignment.
- A persistent cue reads: **Student view - current live assignment. Use Student entry to submit
  graded work.** A **Return to assignment** link is adjacent and first in the page's local actions.
- The learner-facing assignment title, instructions, delivery details, question count, variation,
  and disclosure summary render through the same shared presentation component as the ordinary
  Student assignment overview.
- Instructor-only navigation, the cue, and return action wrap the shared learner content; they are
  not injected into the learner component.
- The ordinary Student component receives its real **Start or continue practice** action. The
  Instructor Student-view composition supplies an informational action slot that leads to live-demo
  Student entry guidance.
- The response is answer-free and `Cache-Control: no-store`. It includes no learner identity,
  enrollment, run, attempt, receipt, score, private source, grader, answer, UUID, or hidden snapshot
  identity.
- Standard Student view uses the assignment's course-wide base delivery. Policies links to the
  existing learner-derived delivery check when an Instructor needs Mary-specific entitlement or
  accommodation evidence.

### Create flow

**New assignment** asks for the assignment title, creates an ordinary persisted draft with server
defaults and zero questions, and routes directly to Questions. The page then gives the positive next
step **Add at least one question**, followed by **Review assignment policies**. The assignment home
shows the same readiness state after reload.

Draft and Archived assignments may retain an empty valid definition; Closed assignments retain
nonempty historical content without requiring an active deliverable position. Publication is the
boundary that requires at least one active deliverable position, supported capabilities, and valid
policies. Draft creation remains available throughout the multi-page workflow and does not depend on
browser-only unsaved state.

## Architecture and ownership

### Canonical browser routes

| Route                                                                      | Owner                 | Purpose                                                    |
| -------------------------------------------------------------------------- | --------------------- | ---------------------------------------------------------- |
| `/instructor/courses/:courseRef/assignments/new`                           | focused create page   | Create the persisted draft and enter Questions             |
| `/instructor/courses/:courseRef/assignments/:assignmentRef`                | assignment home       | Instructor overview and local navigation                   |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/questions`      | Questions page        | Title, ordered content, pools, and question selection      |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/policies`       | Policies page         | Instructions, policies, lifecycle, and base delivery       |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/student-view`   | Student-view page     | Current answer-free learner landing in Instructor context  |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/access`         | existing access page  | Entitlement and accommodations                             |
| `/instructor/courses/:courseRef/assignments/:assignmentRef/delivery-check` | existing preview page | Effective schedule, provenance, and learner-derived checks |

The current `/edit` route is replaced atomically. Source, tests, docs, and screenshot declarations
move to the canonical routes in the same package so the product has one path for each task.

### Domain and HTTP commands

Define closed, strict contracts in `question_model` and generate their TypeScript projections:

- `CreateAssignmentDraftRequest { title }`;
- `ReplaceAssignmentContentRequest { title, entries }`;
- `ReplaceAssignmentPoliciesRequest { audience, disclosurePolicy, policies, teachingSettings }`; and
- `InstructorStudentView` as the answer-free shared landing projection or a narrowly wrapped
  `LearnerAssignmentDetail` when source inspection proves the existing type is exact.

`teachingSettings.instructions` is the single owner of assignment instructions. The browser-safe
`audience` field carries public `CourseGroupReference` values; the server resolves those locators
under exact course authority before applying the policy update.

Migration `2026081848` updates assignment-definition capability validation for this aggregate boundary:
empty definitions are valid in Draft and Archived, Closed retains nonempty historical content without
requiring active delivery, and Published requires an active deliverable position. The domain exposes
publication readiness as blocking issues, and Store writes enforce it for every transition to or while
Published, including teaching-settings and focused Policies mutations.

The focused mutation routes are:

- `POST /api/courses/{course}/assignments/drafts`;
- `PUT /api/courses/{course}/assignments/{assignment}/content`; and
- `PUT /api/courses/{course}/assignments/{assignment}/policies`.

Both `PUT` requests require the current `If-Match` assignment revision and return the complete
authoritative editor detail plus the new `ETag`. Store commands update only their owned fields under
the shared assignment lock and revision. The Policies command updates assignment policy,
disclosure, instructions, base schedule, and lifecycle atomically so the page cannot display a
mixed saved result.

The Student-view read is nested under the course and assignment route. Its handler reuses the shared
learner projection builder with an explicit Instructor-base-policy input and returns `no-store`.
Ordinary learner delivery continues to supply the learner's effective entitlement and policy to the
same builder.

### Authorization and privacy contract

- **ASVS 8.1.1, 8.2.1, 8.2.2, 8.3.1, and 8.4.1:** every assignment workspace route resolves the
  authenticated tenant, current direct course membership, Instructor role, assignment, and exact
  course/assignment relationship at the trusted server layer. Browser route guards improve
  navigation and never supply authority.
- **ASVS 2.1.1-2.1.3 and 2.2.1-2.2.3:** strict generated request types, bounded title/instruction
  values, closed discriminants, entry limits, and contextual policy validation are documented and
  enforced before Store commands run.
- **ASVS 2.3.3:** a focused save advances one assignment revision and all of its owned fields in one
  transaction or leaves the previous revision intact.
- **ASVS 14.2.6 and 14.3.2:** Student view returns the minimum answer-free projection with
  `Cache-Control: no-store` and stores no preview data in browser storage.
- **ASVS 16.3.2 and 16.5.1-16.5.3:** authorization failures use the common non-enumerating response
  and private audit/log path; unexpected failures return generic user guidance while preserving
  entered browser state.
- Public `C-` and `A-` references are locators. Internal API UUIDs remain background details and
  never enter visible content, the address bar, user-copyable links, or error text.

### Browser module ownership

Create `src/pages/assignment_workspace/` as the focused owner for:

- route/session/course gate and loaded assignment context;
- assignment home;
- local navigation;
- Questions composition;
- Policies composition; and
- Student-view composition.

Reuse the current focused content list, ProblemPicker, policy panels, conflict-resolution helpers,
and repository adapter after moving only the ownership required by their new pages. Retire
`assignment_editor_page.tsx`, `assignment_editor_live_page.tsx`, and the saved-link strip when the
new pages own all callers. Keep each source below 1000 physical lines by capability, not by wrapper.

Extract the learner landing markup from `assignment_overview_page.tsx` into one shared presentational
component with explicit action slots. The ordinary learner page owns run start/resume and progress
queries. Student view owns the Instructor cue, return path, and Instructor-base-policy query.

## Work packages

| Work unit                                | Owner            | Depends on                              | Deliverable and acceptance                                                                                                                                                                   |
| ---------------------------------------- | ---------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `T6-W1` focused assignment commands      | Expert coder     | accepted T3/T5/LD3                      | Closed model/Store/HTTP contracts, incomplete persisted drafts, atomic policy save, strong revision conflicts, Memory/PostgreSQL parity; architecture review approves the aggregate boundary |
| `T6-W2` assignment shell and title entry | SolidJS coder    | `T6-W1`                                 | Linked card title, assignment home, local navigation, deep-link/reload behavior, compact 1280 by 800 composition                                                                             |
| `T6-W3` Questions page                   | SolidJS coder    | `T6-W1`, workspace context from `T6-W2` | Existing picker/content/pool/replacement capabilities moved to Questions; focused save and recovery work through visible controls                                                            |
| `T6-W4` Policies page                    | Full-stack coder | `T6-W1`, workspace context from `T6-W2` | Policy and teaching controls moved to Policies; one atomic save; contextual links reach access and delivery check                                                                            |
| `T6-W5` Student view                     | Full-stack coder | `T6-W1`, workspace context from `T6-W2` | Shared learner landing, stable Instructor identity, exact course authority, answer-free no-store response, clear mode and return cues                                                        |
| `T6-W6` integration and evidence         | Integrator       | `T6-W2` through `T6-W5`                 | Old combined route removed, existing connected journey updated, canonical docs/screenshots refreshed, focused reviews resolved, final Validation green                                       |

`T6-W3`, `T6-W4`, and `T6-W5` may run in parallel after `T6-W1` and the small shared workspace
context from `T6-W2` land. Each lane owns its named modules. The integrator owns route registration,
generated contracts, shared CSS, existing test edits, and final deletion of the combined editor.

## Acceptance criteria

### Functional and interaction

- Selecting an Instructor assignment title opens that assignment's Instructor home.
- Assignment-local navigation exposes Overview, Questions, Policies, and Student view with one
  current item.
- Questions contains question/pool authoring and no policy controls.
- Policies contains delivery and lifecycle policy and no question picker or ordered question list.
- Creating a new assignment persists a real empty draft before Questions opens; reload and deep link
  retain it.
- Publication reports incomplete Questions or Policies through field-linked, next-action guidance.
- Student view renders the current assignment title, instructions, delivery facts, variation, and
  disclosure summary through the shared learner landing component.
- Student view retains the Instructor session and returns directly to the same assignment home.
- An ordinary demo Student completes the same published assignment and the ordinary Instructor sees
  the resulting score in the gradebook.

### Accessibility and visual quality

- Assignment titles are semantic links with descriptive names and visible focus. Link text and the
  destination heading agree.
- Local navigation uses a `<nav>` label, semantic links, and `aria-current="page"`; keyboard order
  follows course context, assignment context, page task, and save action.
- Page headings and labels identify their task consistently, following
  [WCAG 2.2 SC 2.4.6](https://www.w3.org/WAI/WCAG22/Understanding/headings-and-labels.html).
- Save, conflict, validation, and reload outcomes use existing status/alert and focus-recovery
  patterns.
- Instructor and Student-view package visuals are reviewed only at the canonical 1280 by 800
  Instructor viewport. The ordinary Student journey retains its declared responsive profiles.
- The cognitive walkthrough completes every task in the task table without relying on internal IDs,
  hidden gestures, browser backtracking, or developer knowledge.

### Security and data integrity

- Student and unrelated-course direct navigation is refused before assignment facts are returned.
- A mismatched course and assignment pair is non-enumerating and performs no mutation.
- Focused requests reject unknown fields and stale revisions and never update an unowned assignment
  slice.
- Student-view responses contain no answer material, grader implementation, private source, PII,
  learner record, score, UUID, or hidden publication identity and are marked `no-store`.
- Opening or navigating Student view creates no enrollment, run, attempt, submission, receipt,
  gradebook row, analysis observation, session replacement, or preview record.
- Ordinary Student work continues to produce immutable evidence and Instructor-visible gradebook
  state through the existing live path.

## Test and verification strategy

### Permanent tests

- Question-model and Store conformance tests cover strict focused requests, valid empty draft state,
  content-only mutation, atomic policy mutation, shared-revision conflict, publication refusal for
  incomplete drafts, and issued-work structural fences.
- Selected PostgreSQL/RLS coverage proves direct-Instructor exact-course authority, cross-tenant and
  mismatched-course refusal, transactional policy save, and Memory/PostgreSQL behavior parity. It
  extends the existing assignment conformance/live owner instead of adding a new database harness.
- Server route tests cover role/data authorization, strict decoding, `If-Match`, generic refusal,
  `ETag`, `no-store`, and answer-free Student-view serialization.
- Focused Node tests cover route helpers, strict response decoding, page-model recovery, and local
  navigation state. They avoid DOM-source inventories and markup snapshots.
- Extend the existing connected `instructor_authoring` journey: Elena creates the persisted draft,
  opens it by title, saves Questions, saves Policies, opens Student view, returns, publishes, and
  later observes Mary's ordinary score. Use visible controls and existing live questions; add no
  fake evidence course or learner.
- Retain the existing ordinary `learner_delivery` journey for Mary's run, grading, repeat behavior,
  and Elena's gradebook evidence. T6 composes with it rather than duplicating it.

### One-time implementation evidence

- Graphify query/affected evidence and direct source inspection for the route/editor/projection
  boundaries.
- ADAPT route, assignment-name link, child Questions/Properties, and fake-student session-toggle
  comparison used only as design evidence.
- Manual 1280 by 800 cognitive walkthrough and semantic visual review of list, home, Questions,
  Policies, Student view, conflict, incomplete-draft, and recovery states.
- Browser-network inspection that Student view is same-origin, answer-free, no-store, and does not
  call learner-work mutation endpoints.
- Before/after source ownership and generated-contract inspection; these inventories remain dated
  package receipts rather than permanent assertions.

### Package and final gates

1. Run focused Rust model, Store, server, TypeScript, Node, and connected browser owners after their
   work unit lands.
2. Run the canonical production-shaped HTTPS journey after all package lanes integrate.
3. Run source-size, ASCII, formatting, lint, Markdown-link, and diff-hygiene gates.
4. Obtain independent architecture/security and HCI/accessibility review with every P0/P1 resolved
   and each lower finding resolved or recorded with owner disposition.
5. Run `source source_me.sh && ./all_test.sh` on the final material tree. A required skip or unrun
   Validation lane keeps `WP-PROF-T6` open.

## Risks and mitigations

| Risk                                            | Consequence                                      | Mitigation                                                                                                                  |
| ----------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| UI-only split keeps complete replacement writes | A stale page can overwrite the other page's work | Land focused Store/HTTP commands first; update only owned fields under one revision                                         |
| Student view is mistaken for graded execution   | Instructor expects work in the gradebook         | Persistent Student-view cue and positive link to ordinary Student entry; graded proof remains the connected Student journey |
| Student view drifts from learner delivery       | Preview gives false confidence                   | Extract one shared learner landing component and one server projection builder with explicit policy input                   |
| Empty drafts leak to learners                   | Incomplete content becomes visible               | Draft lifecycle remains Instructor-only; publication validation requires complete content and valid policies                |
| New local navigation becomes crowded            | Assignment tasks become harder to scan           | Four primary items only; access and delivery inspection remain contextual Policies actions                                  |
| Route replacement leaves stale callers          | Deep links or docs reach the removed editor      | Integrator updates route contract, callers, docs, and connected tests atomically, then removes the old route                |
| Extra tests manufacture state to satisfy counts | Demo becomes a test artifact                     | Reuse existing live journeys and questions; assert semantic transitions rather than record or screenshot counts             |

## Documentation close-out

- Record the durable assignment-title, Questions/Policies separation, and Student-view decisions in
  `docs/HUMAN_GUIDANCE.md`.
- Update `docs/LIVE_DEMO_SPEC.md` first as the lead behavior document.
- Update the route map and Instructor-surface architecture in
  `docs/active_plans/implementation_plan.md`, `docs/CONTRACTS.md`,
  `docs/CODE_ARCHITECTURE.md`, and `docs/FILE_STRUCTURE.md` when implementation lands.
- Update Instructor usage and screenshot guides only from the connected final surface.
- Record focused and final evidence in `docs/CHANGELOG.md`; keep changing package handoff only in
  `docs/active_plans/implementation_status.md`.

## Assumptions and recorded decisions

- "View as student" means the exact learner assignment landing surface in the current Instructor
  session. The ordinary enrolled Student workflow owns runs and grading.
- Standard Student view resolves course-wide base delivery. Specific-learner entitlement and
  accommodation inspection remains the existing audited delivery-check workflow.
- Assignment title belongs to the Questions/content slice; learner instructions belong to Policies.
- One assignment revision serializes all focused mutations.
- Empty draft persistence is a valid domain state; publication is the completeness boundary.
- The package owns forward migration `2026081848` for assignment-workspace draft and publication
  readiness capability. It adds no grading-operation persistence and does not consume the G1/G3
  migration reservations.
