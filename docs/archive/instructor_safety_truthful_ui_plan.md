# Plan: Instructor safety and truthful UI

Status: completed 2026-09-12. Archived after Phase 1 implementation.

Closing evidence: the connected Assignment browser and service flows, the Course
Appearance browser scenario, focused Rust/Node/type/ledger gates,
`./check_codebase.sh`, and the final aggregate suite passed. The aggregate
covered generated TypeScript contracts, Rust checks and tests, Python tests,
the disposable PostgreSQL baseline and installation-data paths, and Course
Appearance PostgreSQL/MinIO acceptance.

## Context

The September 12 readiness review found four places where the Instructor interface tells the
Instructor something untrue. Each one is small on its own; together they make the product feel
unreliable to the one person who has to trust it most. What the Instructor experiences today,
and what correct behavior looks like:

1. **Assignment editing can lose or mislead.** In the Assignment Properties Editor (due date,
   time limit, attempts, late work, feedback rules) an Instructor types a new value and sees it
   on screen, but nothing is saved until an explicit Save button is pressed. The Release button
   is enabled regardless, so an Instructor can release the Assignment with the old date while
   looking at the new one. In the Assignment Question Editor, reordering or removing Questions
   and then clicking any other link silently throws the work away. Correct behavior: a valid
   policy change is saved as soon as it is made and the screen says so (`Saved`, `Saving`,
   `Invalid`, and so on); Release is offered only when everything on screen is saved; leaving a
   page with unsaved Question changes asks first.

2. **Saving a theme hides the Course Banner.** The Instructor picks a new color theme, saves,
   and the banner they uploaded earlier vanishes until they reload the page. The server's
   theme-save reply omits the banner, and the browser trusts that reply. Correct behavior: the
   reply describes the whole Course Appearance, banner included, so the page stays right.

3. **Two hidden pages still answer their URL, badly.** Grade Settings (choose "total points"
   or weighted grade categories, see projected totals, export CSV) and Teaching Operations
   (invite a co-Instructor, list active and pending instructors) exist only as browser code:
   about 1,500 lines of pages, client methods, decoders, and generated types with no server
   route, no Store, and no SQL behind them, and neither page appears in Human Guidance. The
   Ribbon already hides them, but typing or bookmarking their URL shows a page that spins and
   fails while calling endpoints the server has never had. Correct behavior: the code that
   cannot work is gone; the URLs fall through to the ordinary not-found page; the Ribbon labels
   stay reserved for the day a real design and backend arrive.

4. **Two working pages are hidden.** Assignment Overview and Assignment Questions are fully
   served by the backend, but the Ribbon's capability registry still lists them as "unbacked",
   so the only way to reach them is a typed URL. Correct behavior: they appear in the Ribbon
   like Policies does.

Items 1 and 3 share one flaw: browser code written ahead of any backend. Item 1 carries a dead
"save policies" client for an endpoint that never existed, next to the live whole-Assignment
save, which is how the policy screen ended up with no trustworthy save path. Item 3 is two whole
pages in the same condition. Item 4 is the mirror image: real backend, registry never updated.

## Objectives

What an Instructor can rely on when this plan is done:

- What I see in the Assignment Properties Editor is what is saved, or the screen tells me it is
  still saving, was refused, failed, or collided with another editor's change. Release is
  offered only when everything is saved.
- My Question reordering and removals cannot disappear by accident; leaving the page asks me
  to save, discard, or stay.
- Saving a theme leaves my banner where it was.
- A URL for a feature that does not exist behaves like any other unknown URL.
- Assignment Overview, Questions, and Policies are one click apart in the Ribbon.

## Design philosophy

Fix the design, not the symptom, and give one layer ownership of each decision.

- **Policies get their own save operation.** Today every save sends the whole Assignment
  (title, policies, and the Question list). Autosaving a single date through that path would
  resend the Question list on every keystroke and would let a stale Question list overwrite a
  newer one. A save that carries only the policy fields is smaller, cannot touch Questions, and
  matches the narrow "inline save" the repository already uses for title and due date. The
  server keeps its existing rules (Edit Number check, date-order and positive-limit checks,
  re-validation of a Released Assignment); the browser learns nothing new about validation. The
  alternative -- have Rust read the current Assignment, merge in the policies, and call the
  whole-Assignment save -- was set aside: it doubles database work per keystroke and moves
  merge logic out of the layer that owns the invariants.
- **The browser owns what it shows; the server owns what is true.** The browser keeps the
  Instructor's typed values, checks obvious syntax locally, shows a persistence status, and
  guards navigation. The server decides whether a save is accepted. Neither layer second-guesses
  the other.
- **Remove speculative browser code instead of gating it.** PLE is pre-production with no
  users (Human Guidance), so scaffolding that has no backend and no guidance entry is a
  liability, not an asset: it misleads visitors, needs maintenance, and invites someone to
  "finish" it without a product decision. Deleting the two pages leaves the Ribbon's reserved
  labels (DESIGN_DECISIONS keeps Grade Settings as a Course Setup Task label) and the
  registry's "no declared route" entries, the same shape Starred and Watched use today. A future
  Teaching Team or grade-weighting feature starts from its own approved design. The gate
  alternative (keep the pages, short-circuit their mount from the registry) was set aside: it
  preserves code nobody asked for and adds a mechanism to keep it hidden.
- **Autosave means saving valid changes promptly, with no undo.** Restoring an earlier value
  means typing it again. Structural Question edits keep an explicit Save because they are
  coherent multi-step changes, not single values.
- **Evidence before commitment.** The one mechanism this plan has not used before -- pausing a
  navigation, saving, then resuming it -- gets a small throwaway spike first, with a simpler
  fallback already chosen.

## Scope

Four bounded areas of work, each landing as its own patch:

- **Assignment persistence safety.** A policy-only save on the server; an autosaving
  Assignment Properties Editor with visible persistence status and a Release button that waits
  for saved state; a dirty flag and leave-page prompt on the Assignment Question Editor;
  removal of the dead legacy policy client.
- **Course Appearance coherence.** The theme-save reply includes the current banner.
- **Remove backend-less pages.** Delete Grade Settings and Teaching Operations pages, routes,
  client methods, decoders, and their Rust model types; keep the reserved Ribbon labels.
- **Assignment Ribbon navigation.** Mark Overview and Questions as backed in the registry and
  regenerate the navigation ledger from it.

Each patch updates the contract documents and changelog that its behavior touches.

## Non-goals

- Keep policy value recovery as re-entry; undo history stays out.
- Keep Question ordering, addition, removal, and title on explicit Save.
- Keep form handling local to the two editors; a generalized form framework stays out.
- Keep `read_appearance`, `promote_banner`, and `remove_banner` unchanged.
- Keep Teaching Team and grade-weighting features for their own future plans with a product
  design first.
- Keep the Ribbon shape unchanged apart from admitting two destinations.
- Keep `edit_header` behavior as is; its 404 versus documented 428/400 is a separate follow-up.
- Keep generated tsgen-root types unchanged apart from the removals in WP-C1; new DTOs are
  hand-written in `src/api/` like the existing workspace DTOs.
- Keep Student runtime routes unchanged. The one new route is Instructor-authorized through
  active Instructor Course Membership.
- Keep the Instructor Student View with its owning plan.

## Terminology alignment

Canonical names from `docs/HUMAN_GUIDANCE.md` and `docs/TERMINOLOGY_CONTRACT.md` used in this
plan, with the current source names they map to:

| Canonical term                                   | Current source name                                                                 | Notes                                                                         |
| ------------------------------------------------ | ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Assignment Properties Editor                     | `assignment_workspace_policies_page.tsx`, route section `policies`                  | timing, release, attempts, late work, disclosure; hosts Danger Zone Unrelease |
| Assignment Question Editor                       | `assignment_workspace_questions_page.tsx`, route section `questions`                | select, add, remove, order Questions; explicit Save                           |
| Base Assignment Policy                           | policy fields of `SaveLiveAssignmentInput` minus `title`/`entries`                  | the slice the new save operation owns                                         |
| Assignment Edit Number                           | `AssignmentEditNumber`, `If-Match` / `ETag`                                         | concurrency precondition on every save and release                            |
| Assignment Status                                | `workspace.status` Unreleased / Released                                            | Release button visible while Unreleased                                       |
| Assignment Release Validation / Issues           | `release-validation` route, `AssignmentReleaseValidation`, `AssignmentReleaseIssue` | `Check release` keeps this meaning                                            |
| Course Appearance (Course Theme + Course Banner) | `CourseAppearanceView`                                                              | one course-scoped read model; accepted changes replace it                     |
| Instructor Student View                          | route `assignmentWorkspaceStudentView`                                              | Instructor-only surface under `/instructor/...`; owned by a separate plan     |

Browser persistence labels (`Saving`, `Saved`, `Invalid`, `Not accepted`, `Save failed`,
`Conflict`) and the autosave reducer's `persistence` field are browser interaction state and
visible status messages, which the Terminology Contract treats as ordinary technical vocabulary.
Route file names stay as they are; new identifiers use the canonical nouns
(`BaseAssignmentPolicy...`). Singular form verified: the Terminology Contract defines
**Base Assignment Policy** as one complete authored configuration (L1076-1078), so
`SaveBaseAssignmentPolicyInput` and `saveBaseAssignmentPolicy` follow it.

## Current state summary

| Item         | Layer                 | Current behavior                                                                                                                               | Evidence                                                                                                                                                                                                                                                                   |
| ------------ | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 policies   | browser               | per-field signals, explicit Save, Release enabled whenever `!needsReload()`                                                                    | `assignment_workspace_policies_page.tsx:46-60, 106-134, 405-417`                                                                                                                                                                                                           |
| 1 structural | browser               | move/remove/add set a message only; no dirty flag                                                                                              | `assignment_workspace_questions_page.tsx:115-152`                                                                                                                                                                                                                          |
| 1 server     | SQL/Rust              | whole-workspace `ple_data.save_assignment` requires all 26 keys + entries; narrow precedent `save_assignment_inline`                           | `schemas/base_schema/assignments.sql:567-748`                                                                                                                                                                                                                              |
| 1 dead code  | browser               | `saveAssignmentPolicies` client and `assignment_editor_*` target a route with no server handler                                                | `src/api/http_client/request.ts:216-262, 389`; `src/api/contracts.ts:42`                                                                                                                                                                                                   |
| 7            | Rust                  | `update_theme` drops banner                                                                                                                    | `crates/server/src/course_appearance.rs:134-147`                                                                                                                                                                                                                           |
| 8            | browser + Rust models | pages mount and call unregistered endpoints; no Store, no SQL; registry evidence cites nonexistent `ApiClient.getCourseGradeSettings`          | `src/routes.ts:77,80`; `src/pages/course_grade_settings_page.tsx`, `teaching_operations_page.tsx`, `teaching_team_panel.tsx`; `crates/question_model/src/course_grade.rs`, `teaching_operations.rs`, `crates/domain/src/course_grade.rs`; `capability_registry.ts:434-441` |
| 2            | browser               | Overview/Questions `unbacked`; Questions runtime calls are workspace GET, picker GET, save PUT, reload GET, all in `assignment_release_router` | `capability_registry.ts:406-415`; `assignment_workspace_questions_page.tsx:102,139,157`                                                                                                                                                                                    |

Registry shape check: unbacked entries with no declared route already exist (Starred, Watched,
Blueprint Updates, My Assignment Templates); `createRibbonCapabilityEntry`
(`capability_registry.ts:128-157`) requires a route only for backed entries, so removing the two
routes and keeping their catalog and registry entries is an established pattern. Ribbon harness
tests select `teachingOperations` as a tab (`tests/playwright/ribbon_m8_integration_evidence.mjs`)
and keep working because the catalog tab entry (`ribbon_catalog.ts:196-198`) stays. The Student
View page renders a static "Student view unavailable" heading with zero API calls
(`assignment_workspace_student_view_page.tsx:17`) and stays as is.

## Architecture boundaries and ownership

- Browser owns draft values, local syntax validity, persistence status display, navigation
  guards, and unavailable-surface rendering.
- Rust service owns authorization, `If-Match` comparison, request shape, and the public
  workspace projection.
- SQL owns cross-field policy invariants (schedule ordering, positive limits), the Edit Number
  fence, and Released-Assignment re-validation.
- Capability registry (`src/ribbon/capability_registry.ts`) is the single authority for
  destination availability; a declared route implies a page that works.
- Generated ledger `docs/ux/RIBBON_DESTINATION_LEDGER.md` (marker-bounded section) is owned by
  `devel/generate_ribbon_destination_ledger.mjs`; prose outside the markers is hand-edited.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Review boundary                                                        |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| M1 / A                 | `schemas/base_schema/assignments.sql`, `assignment_operations.sql`; `crates/learning-data-access/src/assignment_release.rs`, `postgres/assignment_release.rs`, `postgres/assignment_workspace_save.rs`; `crates/server/src/assignment_release.rs`; `src/api/assignment_release.ts`, `src/api/http_client/assignment_release.ts`, `src/api/decoders/assignment_release.ts`; `src/pages/assignment_workspace/*policies*`, `assignment_workspace_live_page.tsx`                                                                                                            | Base Assignment Policy slice; explicit Save stays for structural edits |
| M1 / A                 | `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx`, new `unsaved_changes_guard.tsx`                                                                                                                                                                                                                                                                                                                                                                                                                                                               | dirty guard                                                            |
| M2 / B                 | `crates/server/src/course_appearance.rs`, `course_appearance/tests.rs`; `tests/test_course_appearance_view_client.mjs`; `tests/playwright/e2e/course_appearance_propagation.spec.ts`                                                                                                                                                                                                                                                                                                                                                                                    | `update_theme` response                                                |
| M3 / C                 | `src/routes.ts`, `src/route_contract.ts`; `src/pages/course_grade_settings_*`, `teaching_operations_page.tsx`, `teaching_operations/`, `teaching_team_*`; `src/api/client.ts`, `http_client.ts`, `http_client/{request,response,error,teaching_operations}.ts`, `decoders/{course_grade,teaching_operations}.ts`; `crates/question_model/src/{course_grade,teaching_operations}.rs`, `lib.rs`, `crates/domain/src/course_grade.rs`; `generated/api/CourseGradeScheme*.ts`, `InstructorCourseInvitation*.ts`; registry entries L255-262, L434-441; tests listed in WP-C1 | removal; catalog and registry entries stay as "no declared route"      |
| M4 / D                 | `src/ribbon/capability_registry.ts:406-415`; ledger regeneration; `docs/ux/*`; `tests/test_ribbon_*.mjs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                | two entries flipped                                                    |

## Milestone plan

| M   | Title                         | Summary                                               | Goal                                                                   |
| --- | ----------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------- |
| M1  | Assignment persistence safety | policies autosave + structural dirty guard            | Instructor releases only persisted policies and keeps structural edits |
| M2  | Course Appearance coherence   | full read model from theme update                     | theme save keeps the Course Banner                                     |
| M3  | Remove backend-less pages     | delete two pages and their dead client and model code | no URL reaches code that cannot work                                   |
| M4  | Assignment Ribbon navigation  | flip two registry entries, regenerate ledger          | Overview/Questions/Policies reachable with guards intact               |

M1, M2, M3 run in parallel with exclusive file ownership. M4 runs after M1 (guard exists before
new navigation paths) and after M3 (M3 reads the registry, M4 edits it).

### Milestone: M1 Assignment persistence safety

- Depends on: none.
- Deliverables: policies endpoint + SQL function; autosave model + editor; dead client removal;
  Question Editor dirty guard.
- Workstreams: A.
- Entry criteria: doer runs the WP-A1/WP-A2 focused commands once and notes any pre-existing
  failure in the patch report.
- Exit criteria: Assignment Properties Editor distinguishes saved / saving / invalid / not
  accepted / failed / conflict; Check release and Release enabled only in `saved`; structural
  edit leaves only through Save or deliberate discard; failed Save keeps the guard.
- Parallel-plan ready: yes. WP-A1 and WP-A2 own disjoint files.

### Milestone: M2 Course Appearance coherence

- Depends on: none.
- Deliverables: `update_theme` returns banner; handler, client, and browser tests.
- Workstreams: B.
- Entry criteria: `cargo test -p server course_appearance` green.
- Exit criteria: theme save with an existing Course Banner keeps it visible before reload;
  banner-free Course stays banner-free.
- Parallel-plan ready: yes. Single work package.

### Milestone: M3 Remove backend-less pages

- Depends on: none.
- Deliverables: Grade Settings and Teaching Operations pages, routes, client methods, decoders,
  Rust model types, generated types, and their tests removed; registry entries reworded to
  "no declared route"; ledger regenerated.
- Workstreams: C.
- Entry criteria: consumer list verified (done, see WP-C1 touch points).
- Exit criteria: `/instructor/courses/<ref>/grade-settings` and `/teaching-operations` render
  the ordinary not-found page; `npx tsc --noEmit`, `cargo check`, and `./check_codebase.sh`
  green; Ribbon shows both labels as Unavailable with no href.
- Parallel-plan ready: yes. Single work package.

### Milestone: M4 Assignment Ribbon navigation

- Depends on: M1, M3.
- Deliverables: two backed registry entries, regenerated ledger, prose updates, ribbon tests.
- Workstreams: D.
- Entry criteria: M1 exit criteria met; M3 landed.
- Exit criteria: Questions -> Policies -> Overview -> Course Assignments reachable by Ribbon
  clicks; dirty guard fires on a Ribbon click from a dirty Question Editor.
- Parallel-plan ready: no. Single serial patch after two dependencies.

## Workstream breakdown

### Workstream: A Assignment persistence safety

- Goal: persisted-or-labeled policy state; guarded structural edits.
- Owner: cross-stack coder (SQL, Rust, TypeScript).
- Work packages: WP-A1, WP-A2.
- Needs: current ETag plumbing in `assignment_workspace_live_page.tsx:150-170`; `edit_header`,
  `workspace_response`, `store_error` in `crates/server/src/assignment_release.rs:428-541`.
- Provides: `saveBaseAssignmentPolicy` context method, autosave model, persistence states,
  dirty guard.
- Review boundary, when modifying the repository: Base Assignment Policy slice and Question
  Editor guard.

### Workstream: B Course Appearance coherence

- Goal: complete read model after theme update.
- Owner: Rust coder with browser test verification.
- Work packages: WP-B1.
- Needs: `CourseBannerStore::read_current_course_banner`
  (`crates/learning-data-access/src/course_banner.rs:208`).
- Provides: truthful `CourseAppearanceView` from `PUT /api/courses/{course}/appearance`.
- Review boundary, when modifying the repository: `update_theme`.

### Workstream: C Remove backend-less pages

- Goal: no browser or model code remains for features that have no backend and no guidance.
- Owner: browser coder with Rust model cleanup.
- Work packages: WP-C1.
- Needs: consumer list (WP-C1 touch points); `cargo tools tsgen` to regenerate `generated/api/`.
- Provides: smaller route table; registry entries in the established "no declared route" shape.
- Review boundary, when modifying the repository: removal only; catalog labels and registry
  ids stay.

### Workstream: D Assignment Ribbon navigation

- Goal: admit Overview and Questions.
- Owner: browser navigation coder.
- Work packages: WP-D1.
- Needs: M1 guard; M3 landed; generator
  `node --import tsx devel/generate_ribbon_destination_ledger.mjs`.
- Provides: backed entries, regenerated ledger, updated prose.
- Review boundary, when modifying the repository: two registry entries, ledger, docs, tests.

## Work packages

### Work package: WP-A1 Base Assignment Policy autosave

- Owner: workstream A coder.
- Touch points:
  - SQL: `ple_data.save_assignment_policies(p_course, p_assignment, p_expected_edit bigint,
p_policies jsonb)` in `schemas/base_schema/assignments.sql` beside `save_assignment_inline`
    (L704-748). Required keys: `instructions`, `due_at`, `available_at`, `closes_at`,
    `late_work_rule`, `assignment_attempt_time_limit_seconds`, `attempt_limit`,
    `activity_rules`, `student_feedback_release_rule`. Title and entries stay untouched. Same
    stale edit `40001`, same Released re-validation via `validate_assignment_release`, same
    Edit Number increment only when a value changed. Table CHECKs stay the cross-field
    authority. `ple_api.save_assignment_policies` wrapper + grant in
    `assignment_operations.sql` beside L390 and L430-453. Pre-production: edit the base schema
    module directly.
  - Rust store: `SaveBaseAssignmentPolicyInput` (`deny_unknown_fields`, field types copied
    from `SaveLiveAssignmentInput` L36-73 minus `title`/`entries`); trait method
    `save_base_assignment_policy(token, course, assignment, expected_edit_number, input)`;
    Postgres impl after `save_live_assignment_inline` (L343-409) resolving Local Date and Time
    via `LocalDateAndTime::resolve_in_account_time_zone`, then re-read through
    `load_assignment_workspace_rows`. Policies JSON encoder beside `assignment_values_json`.
  - Rust server: `PUT /api/course-instances/{course}/assignments/{assignment}/policies` between
    inline (L263-289) and validate (L290); reuse `edit_header`, `workspace_response`,
    `store_error`. The payload carries no entries, so the HMAC entry check is unused here.
  - Browser client: `saveBaseAssignmentPolicy` copying `saveLiveAssignmentInline`
    (`src/api/http_client/assignment_release.ts:122-143`); reuse `quotedStrongEtag`,
    `requireWorkspaceEtag`, `LiveAssignmentWorkspaceConflictError`; decoder.
  - Browser model: new `src/pages/assignment_workspace/base_assignment_policy_autosave_model.ts`
    reducer, shape after `question_json_editor_model.ts:33-66`. State
    `{ persistence: saved | saving | invalid | rejected | failed | conflict, draft, draftSeq,
inFlightSeq, lastAccepted, pending, errors }`. Rules:
    - `draft` is always the visible value; every change writes `draft` first and increments
      `draftSeq`. Page field signals are written from the user's input; server responses update
      `lastAccepted` and the ETag.
    - Valid change while `saving` -> replaces `pending`; otherwise starts a save tagged with the
      current `draftSeq` as `inFlightSeq`. One request in flight; later valid changes coalesce.
    - Invalid change -> `invalid`, `pending` cleared, no request. An in-flight request runs to
      completion.
    - `saveSucceeded(workspace, seq)` -> always update `lastAccepted` and the ETag (the Edit
      Number advanced on the server). Then:
      - `seq === draftSeq` and draft valid -> `saved`.
      - `pending` present -> start it (new `inFlightSeq`), stay `saving`.
      - draft invalid -> stay `invalid`; visible invalid draft untouched.
      - `seq < draftSeq` with no pending -> stay in the current non-saved state; draft untouched.
    - `saveRejected` (422: server invariant such as schedule ordering or a Released
      Assignment's release rule) -> `rejected`, draft kept. Status text names the rejected
      group generically ("The server did not accept these policy values"); the next valid change
      sends normally. Local validation covers the known cases (date syntax, positive integers),
      so `rejected` is the cross-field remainder.
    - `saveFailed` (network, 5xx, 503) -> `failed`, draft kept; later edits update `draft`
      only; `Retry` sends the current draft.
    - `saveConflict` (412) -> `conflict`, draft kept; `Reload server state` replaces draft and
      ETag with the fresh workspace, then `saved`. Another editor's values are unknown and must
      be seen before overwriting (same posture as the Question Editor L157-159).
    - Derived `allEditsPersisted = persistence === saved`.
    - Released Assignments: the Assignment Properties Editor already accepts saves while
      Assignment Status is Released (only Release is status-gated, L405), and the Terminology
      Contract states the server accepts changes to a Released Assignment when the result passes
      Assignment Release Validation. Autosave keeps that contract; a released edit that fails it
      lands in `rejected` with the draft kept.
  - Browser editor: replace `busy/message/needsReload/validationFailed` (L56-60) and the Save
    button (L397-404). Triggers: select / checkbox / committed date-time / committed numeric ->
    immediate; instructions textarea -> blur or a short idle interval (tunable constant).
    Status `<p role="status">` shows `Saving` / `Saved` / `Invalid` / `Not accepted` /
    `Save failed` / `Conflict` with `Retry` (failed) or `Reload server state` (conflict). Check
    release and Release (L405-417) enabled when `allEditsPersisted` and Assignment Status is
    Unreleased; `Check release` keeps its Assignment Release Validation meaning
    (`validateRelease` L157-183) and still surfaces Assignment Release Issues. Add
    `saveBaseAssignmentPolicy(input)` to `AssignmentWorkspaceContextValue`
    (`assignment_workspace_live_page.tsx:34-43`).
  - Removal: `saveAssignmentPolicies` and `/api/courses/{id}/assignments/{id}/policies` client
    (`src/api/http_client/request.ts:216-262, 389`), `AssignmentPoliciesInput`
    (`src/api/contracts.ts:42`), unmounted `assignment_workspace_policy_panel.tsx`,
    `src/pages/assignment_editor_*.ts(x)`, unused `AssignmentPolicyFeedback` /
    `assignmentPoliciesValidationFeedback` in `assignment_workspace_policy_model.ts` unless
    adopted, and their tests.
  - Docs: route row in `docs/API_CONTRACTS.md` (L81-95); `docs/CONTRACTS.md:44-51` note that
    the policies save shares the Assignment Edit Number precondition;
    `docs/CONCURRENCY_CONTRACTS.md:53` Assignment Edit Number from "Planned" to current.
- Depends on: none.
- Acceptance criteria: saved workspace matches every valid visible value after `Saved`; two rapid
  valid changes converge on the last value with one request in flight; invalid input stays
  visible, sends nothing, and disables Release; stale ETag yields `Conflict` and the server
  value stands; title and entries unchanged after a policies save.
- Evidence or review, when useful:
  - Rust Postgres (`--ignored`, disposable stack) in new
    `crates/learning-data-access/tests/assignment_policies_postgres.rs` following
    `course_banner_saga_postgres.rs`: title + entries preserved; stale Edit Number -> conflict;
    `closes_at < due_at` -> invalid; Released Assignment re-validates.
  - Rust server unit: stale `If-Match` -> 412 on the new route (extend
    `lifecycle_and_edit_number_conflicts_have_distinct_statuses`).
  - Node `tests/test_base_assignment_policy_autosave_model.mjs`: coalescing; invalid blocks;
    stale success after a newer draft keeps the draft and stays unsaved; success while draft
    invalid stays `invalid` with ETag advanced; 422 -> `rejected`, next valid edit sends;
    failed -> edit -> Retry sends current draft; conflict keeps draft; `allEditsPersisted` only
    in `saved`. Client test in `tests/test_assignment_client.mjs`.
  - Browser: extend `tests/playwright/e2e_live_demo_assignment_release_browser.mjs` policies
    step (L123): change select -> `Saved` -> reload -> persisted; invalid date -> Release
    disabled.
  - Permanent tests assert behavior; serde `deny_unknown_fields` and the SQL key allowlist
    already enforce shape.
- Obvious follow-ons: changelog entry; independent reviewer pass on the autosave model.

### Work package: WP-A2 structural edit dirty guard

- Owner: workstream A coder.
- Touch points:
  - `assignment_workspace_questions_page.tsx`: `dirty` signal set by `move/remove/add`
    (L115-130) and title edits; cleared on successful `save()` (L132-152); kept on failure.
  - New `src/pages/assignment_workspace/unsaved_changes_guard.tsx` using `useBeforeLeave` from
    `@solidjs/router`. Verified API (`node_modules/@solidjs/router/dist/types.d.ts:151-158`):
    `preventDefault()` cancels, `retry(force)` re-issues navigation; popstate flows through the
    same lifecycle. Flow: dirty -> `preventDefault`, open prompt; `Stay` closes; `Discard and
continue` -> `retry(true)`; `Save and continue` -> await `save()`, success -> `retry(true)`,
    failure -> stay with the existing save error shown.
  - `window.beforeunload` for tab close / hard reload (native prompt).
  - `Discard` abandons mounted local state; remount reloads through the existing loader
    (`assignment_workspace_live_page.tsx:146`).
  - Prompt: reuse an existing dialog under `src/features/` when one exists; else native
    `<dialog>` with `aria-labelledby` and focus returned to the trigger.
  - Release lives in the Assignment Properties Editor, reached from a dirty Question Editor only
    through the guard, so the guard is the structural interlock.
- Depends on: none.
- Acceptance criteria: reorder then Ribbon click prompts; `Stay` keeps order; `Discard` lands on
  the target and returning shows server order; successful Save clears the guard; failed Save
  keeps it. `Save and continue` is a convenience; the objective is deliberate discard, so the
  two-choice fallback fully satisfies this work package.
- Evidence or review, when useful:
  - Spike (first step, `tests/_temp/`): prove `retry(true)` resumes a Ribbon click and a back
    navigation. Fallback when it fails: two-choice prompt (Discard / Stay), noted in the
    changelog. Remove the spike in the same patch.
  - Node: extend `tests/test_assignment_workspace_questions.mjs` for dirty transitions.
  - Browser: guard step in the live-demo script.
- Obvious follow-ons: changelog entry.

### Work package: WP-B1 return coherent Course Appearance

- Owner: workstream B coder.
- Touch points: `crates/server/src/course_appearance.rs:134-147` -- after
  `themes.update_course_theme`, call `banners.read_current_course_banner` and return
  `CourseAppearanceView { theme, banner }`. Browser stays unchanged (`saveTheme` at
  `course_appearance_page.tsx:77-80` already replaces the read model through
  `replaceCourseAppearance`, `route_scope_controller.ts:207-223`).
- Depends on: none.
- Acceptance criteria: theme save with Course Banner present returns it; without one returns
  `null`; browser shows the Banner immediately after theme save; reload matches.
- Evidence or review, when useful: Rust handler test in `course_appearance/tests.rs` using the
  existing mock `CourseBannerStore` (L24-160) plus a minimal theme-store mock; Node
  `tests/test_course_appearance_view_client.mjs:49-55` banner-present fixture; Playwright
  `course_appearance_propagation.spec.ts` new step after L124: save theme again, assert Banner
  visible before reload.
- Obvious follow-ons: changelog entry.

### Work package: WP-C1 remove Grade Settings and Teaching Operations

- Owner: workstream C coder.
- Touch points (delete unless noted; use `git rm` for tracked files):
  - Pages: `src/pages/course_grade_settings_page.tsx`, `course_grade_settings_page.css`,
    `course_grade_settings_model.ts`, `teaching_operations_page.tsx`,
    `teaching_operations/teaching_operations_panels.css`, `teaching_team_panel.tsx`,
    `teaching_team_panel.css`, `teaching_team_model.ts`.
  - Routes: `src/routes.ts:77,80` entries; `src/route_contract.ts` `courseGradeSettings`
    (L352-363) and `teachingOperations` (L383-389). Check `tests/test_route_params.mjs:89` and
    `tests/test_frontend_contract.mjs:59`, which enumerate route ids, and drop those rows.
  - Browser API: `src/api/http_client/teaching_operations.ts` (whole file); grade-scheme,
    gradebook-totals, and grade-export methods in `src/api/http_client/response.ts`
    (L468, L489) and `request.ts` (L294, L318); matching entries in `src/api/client.ts`,
    `src/api/http_client.ts`, `src/api/http_client/error.ts`; decoders
    `src/api/decoders/course_grade.ts` and `decoders/teaching_operations.ts`.
  - Rust tsgen roots: `crates/question_model/src/course_grade.rs`,
    `crates/question_model/src/teaching_operations.rs`, their `mod` lines in
    `crates/question_model/src/lib.rs`, and `crates/domain/src/course_grade.rs` (confirm each
    has no consumer outside the removed set with `grep -rn` before deleting). Run
    `cargo tools tsgen` so `generated/api/CourseGradeScheme*.ts` and
    `InstructorCourseInvitation*.ts` disappear.
  - Registry (keep, edit): `capability_registry.ts` `teachingOperations` (L255-262) and
    `gradeSettings` (L434-441) become plain "no declared route, page, client method, or
    registered handler" entries in the same shape as `blueprintUpdates` (L264-265), with the
    nonexistent `ApiClient.getCourseGradeSettings` evidence removed. Catalog entries in
    `ribbon_catalog.ts` (tab L196-198, task L454-465) stay so reserved labels and Ribbon
    harness tests keep their geometry.
  - Tests: delete `tests/test_course_grade_settings.mjs`,
    `tests/test_teaching_operations_decoder.mjs`, `tests/test_teaching_team_model.mjs`; update
    `tests/support/ribbon_deferred_content_harness.tsx` and
    `tests/playwright/ribbon_deferred_content_evidence.mjs`, which import the grade-settings
    page; keep `tests/playwright/ribbon_m8_integration_evidence.mjs` unchanged (uses the tab).
  - Ledger: regenerate with `node --import tsx devel/generate_ribbon_destination_ledger.mjs`;
    reword the two prose sections (`docs/ux/RIBBON_DESTINATION_LEDGER.md:105-108, 180-183`) to
    "reserved label, no route".
- Verified boundaries (2026-09-12): `crates/domain/src/course_grade.rs` is consumed only by
  `question_model/src/course_grade.rs`, which is consumed only by its `lib.rs`; Live Gradebook
  reads `ple_api.read_course_gradebook` and uses none of these types. The co-Instructor concept
  (Human Guidance L218, Terminology Contract "Instructor Course Invitation" L374-383,
  DESIGN_DECISIONS Teaching Team L800-839) has no schema, Store, or route today; this removal
  deletes a browser scaffold and leaves that documented future capability untouched. A future
  Teaching Team plan starts from those documents.
- Depends on: none.
- Acceptance criteria: the two URLs render the ordinary not-found page; Ribbon shows both
  labels Unavailable with no href; `npx tsc --noEmit`, `cargo check`, `./check_codebase.sh`,
  and ledger `--check` green; `grep -rn 'grade-scheme\|gradebook-totals\|instructor-course-invitation' src crates generated` returns nothing.
- Evidence or review, when useful: existing `tests/test_ribbon_capability_registry.mjs` still
  passes (its unbacked-id list at L97-111 gains the two ids); Playwright direct-navigation check
  for the not-found surface in the shared browser flow.
- Obvious follow-ons: changelog entry under Removals and Deprecations naming both pages and
  the reason (no backend, no guidance); `docs/FILE_STRUCTURE.md` rows for the deleted files.

### Work package: WP-D1 admit Overview and Questions

- Owner: workstream D coder.
- Touch points:
  - Re-run backing trace: grep `client.` in `assignment_workspace_overview_page.tsx` and
    `assignment_workspace_questions_page.tsx`; every ordinary user action (load, pick question,
    reorder, remove, save, reload) reaches a route registered in `production_router_from_env`.
    Flip proceeds when that holds.
  - `capability_registry.ts:406-415`: flip both to `kind: "backed"` copying the
    `assignmentPolicies` entry (L416-428); `clientMethod:
"ApiClient.getLiveAssignmentWorkspace"`, Questions also cites
    `listLiveAssignmentQuestionPicker`.
  - Regenerate ledger with `node --import tsx devel/generate_ribbon_destination_ledger.mjs`;
    verify with `node tests/e2e/e2e_ribbon_destination_ledger.mjs`.
  - Hand-edit prose outside the markers: `docs/ux/RIBBON_DESTINATION_LEDGER.md`,
    `docs/ux/RIBBON_TASK_MODEL.md`, `docs/ux/FRONTEND_CAPABILITY_INTEGRATION.md:120`.
- Depends on: WP-A2 (guard), WP-C1 (registry read landed).
- Acceptance criteria: Ribbon shows Overview / Questions / Policies with hrefs; `aria-current`
  follows route; Questions -> Policies -> Overview -> Course Assignments by clicks only; dirty
  guard fires on Ribbon click from a dirty Question Editor.
- Evidence or review, when useful: `tests/test_ribbon_capability_registry.mjs:113-160` backed
  assertions; `tests/test_ribbon_contract.mjs` assignment group hrefs and selection; browser
  navigation step in the live-demo script.
- Obvious follow-ons: changelog entry.

## Acceptance criteria and gates

- Per-patch gate: the work package's focused commands (below) green; `./check_codebase.sh` for
  any Python or Markdown touched; one changelog entry.
- Integration gate (plan exit): each invariant has one owning check; the aggregate suite
  supplies regression coverage.

| Invariant                                                | Owning check                                                |
| -------------------------------------------------------- | ----------------------------------------------------------- |
| Visible policy value persisted or labeled; Release gated | autosave model test + live-demo policies step               |
| Structural edit leaves only through Save or discard      | dirty model test + live-demo guard step                     |
| Theme save keeps Course Banner                           | handler test + appearance Playwright step                   |
| No URL reaches backend-less code                         | grep-empty check + not-found Playwright step                |
| Assignment pages reachable                               | ribbon tests + ledger `--check` + live-demo navigation step |

- Independent review gate, when useful: reviewer agent pass over WP-A1 (autosave model and SQL
  function) before M4 starts; findings fixed in owning code.

## Test and verification strategy

Focused commands per work package:

```bash
# WP-A1
cargo test -p learning-data-access --test assignment_policies_postgres -- --ignored
cargo test -p server assignment_release
node --import tsx --test tests/test_base_assignment_policy_autosave_model.mjs tests/test_assignment_client.mjs
# WP-A2
node --import tsx --test tests/test_assignment_workspace_questions.mjs
# WP-B1
cargo test -p server course_appearance
node --import tsx --test tests/test_course_appearance_view_client.mjs
# WP-C1
npx tsc --noEmit
cargo check
node --import tsx --test tests/test_ribbon_capability_registry.mjs tests/test_route_params.mjs tests/test_frontend_contract.mjs
node tests/e2e/e2e_ribbon_destination_ledger.mjs
# WP-D1
node --import tsx --test tests/test_ribbon_capability_registry.mjs tests/test_ribbon_contract.mjs
node tests/e2e/e2e_ribbon_destination_ledger.mjs
```

Shared browser flow at plan exit:

```bash
bash tests/e2e/e2e_live_demo_assignment_release.sh --browser
./devel/run_playwright_tests.sh --build --grep "appearance|not-found"
```

Aggregate once at plan end:

```bash
source source_me.sh && ./launchers/all_test.sh
```

Permanent tests protect behavior: preservation of title and entries, conflict, invalid
schedule, coalescing, persistence across reload, release gating, dirty transitions, Course
Banner retention, navigation reachability. `tests/_temp/`
holds the WP-A2 spike and is emptied at milestone close.

## Risk register

| Risk                                          | Impact                 | Trigger                                               | Owner | Mitigation                                                                                                    |
| --------------------------------------------- | ---------------------- | ----------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------- |
| Autosave race or silent overwrite             | stale policy released  | out-of-order responses                                | A     | one in-flight + coalesce; sequence tags; ETag on every PUT; 412 -> conflict requires reload                   |
| Invalid local value looks saved               | Release with bad dates | partial date accepted                                 | A     | validity is a persistence state; Release only in `saved`; SQL CHECKs                                          |
| `retry(true)` skips some navigation kinds     | guard blocks or leaks  | back/forward or Ribbon click                          | A     | `tests/_temp/` spike first; fallback two-choice prompt                                                        |
| Collision with another lane on generated TS   | merge churn            | tsgen regeneration                                    | A, C  | A adds no tsgen roots; C removes six generated files in one patch and regenerates once                        |
| Removed Rust model type has a hidden consumer | `cargo check` failure  | deleting `course_grade.rs` / `teaching_operations.rs` | C     | `grep -rn` each type name across `crates/` before deletion; keep any type with a live consumer and record why |
| C1 and D1 both touch registry                 | conflicting edits      | parallel dispatch                                     | D     | D1 after C1                                                                                                   |
| Dead-code removal leaves a dangling import    | build failure          | removal patch                                         | A     | `npx tsc --noEmit` before handoff; `check_codebase.sh`                                                        |

## Rollout and release checklist

- [x] Land WP-A1, WP-A2, WP-B1, WP-C1 as separate patches, each with its docs and changelog
      entry. Each doer runs the focused commands once before editing and notes any pre-existing
      failure in the patch report.
- [x] Reviewer pass on WP-A1 (autosave model, SQL function); fix findings in owning code.
- [x] Confirm M1 exit criteria; land WP-D1.
- [x] Run the shared browser flow.
- [x] Run the final aggregate suite.
- [x] Tick the corresponding items in the parent plan's checklist; archive this plan.

## Documentation close-out requirements

- Active plan / progress tracker: this completed file is archived at
  `docs/archive/instructor_safety_truthful_ui_plan.md`; the parent plan points there.
- docs/CHANGELOG.md entry: one per work package under the correct category; M4 entry notes
  ledger regeneration.
- Contracts: `docs/API_CONTRACTS.md` route row; `docs/CONTRACTS.md:44-51` policies save shares
  the Assignment Edit Number precondition; `docs/CONCURRENCY_CONTRACTS.md:53` Assignment Edit
  Number from "Planned" to current.
- Ribbon docs: prose outside ledger markers, `RIBBON_TASK_MODEL.md`,
  `FRONTEND_CAPABILITY_INTEGRATION.md`; `docs/FILE_STRUCTURE.md` rows for removed files.
- Archive / closure notes: on plan exit `git mv` this file to `docs/archive/` and record the
  closing evidence in the changelog.

## Patch plan and reporting format

Each patch carries its own documentation: the contract rows, prose, and changelog entry that
its behavior changes.

- Patch 1: WP-A1 SQL + Rust + client, with `docs/API_CONTRACTS.md` row, `docs/CONTRACTS.md`
  note, `docs/CONCURRENCY_CONTRACTS.md:53` update.
- Patch 2: WP-A1 autosave model + editor + dead-code removal.
- Patch 3: WP-A2 spike then guard; spike removed from `tests/_temp/` in the same patch.
- Patch 4: WP-B1.
- Patch 5: WP-C1 removal with tsgen regeneration, ledger regeneration, and
  `docs/FILE_STRUCTURE.md`.
- Patch 6: reviewer pass over patches 1-2; findings fixed in owning code.
- Patch 7: WP-D1 with ledger regeneration and Ribbon prose (after patches 3, 5, 6).
- Plan close-out: shared browser flow, aggregate suite, parent checklist, `git mv` to archive.

Patch 2 follows patch 1 (the editor consumes the client from patch 1). The patch 1-2 sequence,
patch 3, patch 4, and patch 5 may run in parallel (three doers; 1-2 and 3 share workstream A on
disjoint files). Each patch report states: work package and files owned; behavior completed; focused
gates run; pre-existing or new failures; next work package unblocked.

## Open questions and decisions needed

- Manager/subagent decision procedure:
  - Decision owner or dedicated class: workstream A coder, via the WP-A2 spike.
  - Evidence and decision rule: when `retry(true)` resumes both a Ribbon click and a back
    navigation in the spike, keep the three-choice prompt; otherwise ship Discard / Stay and
    record the reason in the changelog.
- Non-blocking follow-up: `edit_header` returns concealed 404 for missing or invalid `If-Match`
  (`crates/server/src/assignment_release.rs:489-496`) while `docs/API_CONTRACTS.md:27-30`
  documents 428/400. The new route follows current handler behavior; reconcile in a separate
  patch.
- Non-blocking follow-up: 422 body carries no field detail; browser local validation covers the
  known cases.
