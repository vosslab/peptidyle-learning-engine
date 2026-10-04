# Ribbon folder tabs + Ribbon-hierarchy breadcrumbs

The later [file-tab correction](../active_plans/reports/ribbon_file_tabs_2026_10_03.md)
supersedes the Tier 1 visual model recorded in this completed plan.

Completed October 3, 2026. Implementation, rendered review, and verification are recorded in
[ribbon_review_2026_10_02.md](../active_plans/reports/ribbon_review_2026_10_02.md).

## Context

Screenshots (`docs/screenshots/instructor/stacked-screenshot.webp`) show the selected Tier 1 tab
as a bordered box plus thick underline floating in the top row, and the selected Tier 2 task as a
taller, rounder, differently filled pill. Selection reads as an inserted button and the tiers use
unrelated visual grammar. Breadcrumbs are hand-coded per route and mostly read `Home / Page`,
dropping the Tier 1 and Tier 2 levels. Inside a Course, nothing in Tier 2 is selected.

## Corrected visual contract

The October 2 user clarification supersedes the original Tier 1-to-Tier 2 connection and
Tier 2 tint/underline treatment. The two tiers use different shapes and surface relationships.

- Tier 1: folder-shaped tabs float in the Ribbon with visible space above and below. The
  selected folder belongs to the Ribbon/background plane; unselected folders are shaded and
  quieter. In light mode this means lighter selected and darker muted siblings. Evaluate the
  analogous open-versus-recessed relationship in dark mode, without reversing colors blindly.
  A defined edge and stronger label keep selection clear. Tier 1 never physically joins Tier 2
  or the content.
- Tier 2: simple rectangular tabs sit directly on the content edge. The selected tab shares
  the content surface with an open bottom edge. Unselected tabs remain separate from content.
  Avoid pills, floating buttons, underline-only selection, and oversized selected states.
- Shape, placement, and surface relationships establish hierarchy. Width follows the label;
  Tier 1 need not be wider or substantially larger than Tier 2.
- Selected and resting states preserve their geometry and reserved row heights. Content starts
  at the same vertical position when selections change.
- Keep this model on narrow screens. Use the available Tier 2 width, reveal the selected item,
  and make additional choices discoverable through scrolling and edge cues.
- Use restrained surfaces and edges. Reference images explain the metaphor, not gradients,
  gloss, heavy shadows, bevels, or other decoration.
- Breadcrumbs: `Home / Tier 1 / Tier 2 / object context / Page`. Preserve useful ancestors.
  Collapse adjacent levels only when they show the same name. Sharing a URL preserves levels.
- Tier 1 and Tier 2 stay selected while the page is a descendant of them.

## Part A: Modular Ribbon CSS

Keep the existing navigation components and shared shell. Isolate the visual decisions:

- `src/ribbon/ribbon_tier_one.css`: folder shape, floating placement, resting/selected/hover
  surfaces, and Tier 1 responsive spacing.
- `src/ribbon/ribbon_tier_two.css`: rectangular tabs, content connection, and selected/hover
  surfaces.
- `src/ribbon/app_ribbon.css`: shared row geometry, scroll frames, identity, focus, and controls.
- `src/ribbon/app_ribbon_density.css`: common phone/touch, forced-color, breadcrumb, and
  reduced-motion behavior.

Use a few local size and surface tokens so either tier can change without altering the other.
Do not add runtime styling modes, duplicate components, or a second navigation model.

Tier 1 folders share a small raised shoulder and a fixed body. The selected surface matches
its surrounding Ribbon, with a defined edge and heavier label. Every folder stays clear of
both row boundaries, including its shoulder. Preserve the 44px touch target with enough
reserved row space for that separation.

Tier 2 tabs sit on the row floor. The selected top and side edges identify its boundary; its
bottom edge shares the content surface. Carry that surface through the breadcrumb prelude.
Resting labels are muted but readable, with stable label-width reservation. Keep plain
rectangular geometry distinct from the folder shoulder above.

## Part B: Tier 2 ancestry, selection, breadcrumbs

Ancestry source. The route contract already declares Tier 1 (`ribbon.tierOneArea`). Exact Tier 2
pages derive from the catalog (destination route equals current route). Only descendant routes
need more, so add an optional `tierTwoParent` to the same `ribbon` entry in
`src/route_contract.ts` (no separate table). URL-prefix derivation was rejected: it misses
`/courses/:id` routes and change proposals, and cannot tell active from inactive Courses.

- Plain parent ids: questionDetail -> searchQuestionLibrary; questionDraftEditor ->
  myDraftQuestions; myChangeProposals, changeProposalDetail -> myBlueprintCourses;
  assessmentOverview, assessmentAttempt -> allCoursework; assessmentAttemptSummary ->
  studentAttemptHistory.
- Context-resolved parents: instructor Course routes (courseAssessments, courseRoster,
  courseAppearance) resolve from Course lifecycle (active -> myActiveCourses, inactive ->
  myInactiveCourses); blueprintCourseDetail resolves from the existing
  `blueprintBreadcrumbParent` label. Until the context resolves, no Tier 2 ancestor is selected
  and no Tier 2 crumb is shown, so the Ribbon never shows a wrong parent. The breadcrumb row
  already reserves its height, so the later crumb causes no content shift.
- Student Course subpages keep selecting their `studentCourse:<id>` slot.
- A parent applies only if it is in the role's Tier 2 row for that Tier 1 area. Exact selection
  wins (Active Attempt, Latest Feedback).

Course lifecycle plumbing (inactive Course fix):

- `schemas/base_schema/50_functions/course_operations.sql`: `ple_api.read_course_summary`
  also returns `course.course_lifecycle_state`.
- Rust: decode it in `read_course_summary`
  (`crates/learning-data-access/src/postgres/course_instance.rs`), add `lifecycle_state` to
  `CourseSummary` and `CourseInstanceRouteSummary` (`crates/question_model/src/course.rs`), and
  copy it in `crates/server/src/course_instance.rs`. Reuse the existing
  `CourseInstanceLifecycleState` and `lifecycle_state()` decoder.
- Regenerate TS (`./build.sh` / cargo tsgen); update the route-summary decoder; `src/app.tsx`
  `ribbonLabelsFor` passes `courseLifecycleState`; add it to `RibbonContextLabels`.

Ribbon model (`src/ribbon/ribbon_contract.ts`, 862 lines): `taskAreasFor` receives context labels
and applies the ancestor rule. To stay well under the 1000-line limit, move `buildRoutePath` and
its helpers to `src/ribbon/ribbon_route_path.ts` and breadcrumb derivation to
`src/ribbon/ribbon_breadcrumbs.ts`; update import sites.

Breadcrumbs = `[Home, Tier 1?, Tier 2?, ...tail]`:

- Tier 1 crumb = selected tab (label, href); absent for areas without a tab (`account`).
- Tier 2 crumb = selected Tier 2 control (label, href); the Student Course slot uses the existing
  Course crumb (long name plus compact short name).
- Tail = today's per-route Course / Assessment / section / title crumbs, minus the collection
  parents now supplied by Tier 1 and Tier 2. Existing fallbacks stay.
- Terminal crumb is current; earlier crumbs link. Then drop the shallower of any adjacent pair
  with the same displayed name. Malformed-route `[Home]` fallback stays.

Representative trails:

| Route | Trail |
| --- | --- |
| instructorHome | Home / Courses / My Active Courses |
| courseRoster (active) | Home / Courses / My Active Courses / Biochemistry I / Students |
| courseRoster (inactive) | Home / Courses / My Inactive Courses / Biochemistry I / Students |
| blueprintCourseDetail | Home / Courses / My Blueprint Courses / Molecular Biology Blueprint |
| questionDetail | Home / Questions / Search Question Library / Catalytic triad |
| assessmentWorkspacePolicies | Home / Assessments / Biochemistry I / Problem Set 7 / Properties |
| studentCourseProgress | Home / Courses / Biochemistry I / Progress |
| assessmentAttempt | Home / Coursework / All Coursework / Biochemistry I / Problem Set 7 / Attempt |
| profile | Home / Profile settings |
| sysadminCourseInspection | Home / Courses (Tier 1 "Courses" + page "Courses" collapse) |

Instructor assessment-workspace routes declare Tier 1 Assessments, whose Tier 2 row has no parent
for a Course Assessment, so Tier 2 is absent and Course context follows.

## Part C: Tests

Update only tests that encode the changed behavior, plus any that fail:

- `tests/test_ribbon_contract.mjs`: new trails; ancestor selection (active vs inactive Course,
  unresolved lifecycle shows no ancestor, exact beats ancestor); same-name collapse.
- `tests/playwright/ribbon_density_evidence.mjs`: replace underline checks with the folder
  contract: Tier 1 floats on the Ribbon plane; Tier 2 joins content; both selections keep their boxes
  identical; selected weight differs; forced colors keeps a non-color channel. Drop the
  `tabUnderline` contrast indicator. Existing text-contrast checks stay.
- Rust tests constructing `CourseSummary` / `CourseInstanceRouteSummary` get the new field.

## Part D: Docs

- `docs/HUMAN_GUIDANCE.md` (user-stated): add to **Breadcrumbs interface**
  - Breadcrumbs preserve as many meaningful navigation levels as practical rather than
    collapsing the path to only broad and current pages.
  - Include each meaningful ancestor that gives the user a useful place to navigate back to.
  - Breadcrumbs represent navigation hierarchy, not unique URLs. Keep Tier 1 and Tier 2 levels
    even when they link to the same page.
  - Collapse adjacent breadcrumb levels only when they show the same name.

  and to **Ribbon and page layout**
  - Ribbon Tier 1 and Tier 2 selections remain selected when the current page is a descendant of
    those navigation choices.
- `docs/DESIGN_DECISIONS.md` Interface section: "Floating folders sit above content tabs"
  and "Breadcrumbs follow the Ribbon hierarchy" (route-declared `tierTwoParent`,
  lifecycle-resolved Course parent, same-name collapse).
- `docs/CHANGELOG.md` entry.

## Execution order

1. Lifecycle plumbing (SQL, Rust, generated TS, decoder); cargo tests.
2. Route contract `tierTwoParent`, ancestor selection, breadcrumb module, contract tests.
3. CSS and density evidence.
4. Docs, screenshots.

## Verification

- `cargo test` for touched crates; rebuild the Live Demo database from the edited base schema.
- `source ./source_me.sh && ./launchers/run_fast_checks.sh` (tsc, `tests/test_ribbon_contract.mjs`,
  line-limit gate).
- `node tests/playwright/ribbon_density_evidence.mjs`.
- `source ./source_me.sh && ./devel/capture_screenshots.sh`, then
  `docs/screenshots/instructor/crop-stack.sh`; review light/dark themes, phone, Student role, an
  active and an inactive Course page (ancestor highlight and trail). Tune Part A starting values
  from these renders.
- `source ./source_me.sh && ./launchers/all_test.sh` before reporting done.
