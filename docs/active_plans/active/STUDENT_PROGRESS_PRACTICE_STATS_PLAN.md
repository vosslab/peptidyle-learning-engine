# Student Progress and Response Stats

## Product contract

The Student Tier 1 Ribbon is **Coursework | Grades | Courses**. Tier 1 fixes the ordered
Tier 2 row; the selected Course supplies page content and destination context. Opening an
Assessment, Attempt, or review never changes that row.

| Tier 1 | Fixed Tier 2 |
| --- | --- |
| Coursework | All Coursework, Due Soon, Completed, Active Attempt |
| Grades | Scores, Response Stats, Attempt History, Latest Feedback |
| Courses | Progress |

Design for one active Course as the normal case and two as an occasional case. With one active
Course, `/student` opens its All Coursework page. With multiple active Courses, `/student` opens the
existing Course chooser. Course-scoped Tier 2 labels and order stay visible without a selection;
their links are unavailable until a Course is selected. Invitations stay in the Courses workflow.

Progress separates submitted completion from score perfection. Show every released Assessment in
the selected Course, including Assessments with no Attempts. An Assessment with Attempts but no
released, disclosed score remains visible in a **Score not released** state with its Attempt and
activity information. It is not classified as below 100%, and it must not disappear from Progress.
Only a released, disclosed current or best score below 100% is classified as below 100%. A 100%
score and a submitted Attempt are separate facts. Do not calculate a weighted Course grade.

Response Stats reports actual saved outcomes across eligible Assessment types in the Course; it is
not limited to Practice Question Assignments. It belongs under Grades because it describes how the
Student responded. Group by exact immutable Published Question revision and show only outcomes
permitted by the source Assessment's score and per-Question correctness disclosure rules. Do not
expose another Student's work, cohort statistics, or unreleased correctness. Missing display-duration
data is **Not recorded**, never zero or a whole-Assessment estimate.

Due Soon reuses the existing Instructor rolling seven-day convention. Server time determines
membership in the window; the Account time zone determines display, without an inline time-zone
label. Show the time-zone name only on Profile. This is a consistent product convention, not a claim
that seven days has special pedagogical meaning. Completed means at least one submitted Attempt; it
does not mean a perfect score. Due Soon and Completed may overlap.

Attempt History lists the signed-in Student's Attempts in the selected Course, newest first, with
cursor pagination and links to the existing Attempt review. Attempt feedback remains on its
Attempt review. Latest Feedback is a fixed Grades shortcut to the latest Attempt review with
Student-visible feedback; it stays disabled when no such review is available.

Active Attempt is a fixed Coursework shortcut. It stays disabled when no Attempt can be resumed.
When one or more can be resumed, it opens the Attempt with the latest recorded activity. A tie uses
Attempt start time and then Attempt ID; do not add a Course-wide Attempt chooser for this rare case.

Question display duration means approximate **time shown with the Question**. Measure monotonic
elapsed time only while the Question is current and the browser document is visible. Pause on
Question change, hidden document, leaving the Attempt, and submission. It does not measure attention
or effort and never affects grading.

## Scope and boundaries

- Keep Instructor Tier 2 unchanged.
- Add Course Progress, Response Stats, Due Soon, Completed, Active Attempt, and Course-wide Attempt
  History.
- Stabilize Student Tier 2 across Course, Assessment, Attempt, and review routes.
- Reuse existing Assessment progress, score disclosure, Course authorization, and RecordList
  contracts where they fit.
- Add cumulative display-duration persistence to Question Attempt Student Work, with authenticated
  checkpoints for the signed-in Student's open Attempt.
- Add representative screenshots and update the current product, API, and schema documentation.
- Keep invitations in the existing Courses workflow. Add no cross-Course dashboard, weighted grade,
  class analytics, permanent invitation navigation, or Attempt-specific Tier 2 row.

## Milestones and atomic work packages

### M1 - Settle the contract

**Packages:** M1a authorities; M1b plan supersession.

Update Design Decisions, Ribbon task model, this plan, and the changelog. Mark only the Student
investigation in `fixed_tier_1_to_tier_2_ribbon_contract.md` as superseded; preserve its Instructor
work. Record the unreleased-score state and the API/UI package boundaries in this plan.

**Exit:** Product language defines all Tier 2 mappings, Course entry, completion, disclosure, Due
Soon membership, and duration meaning.

### M2 - Stabilize Student navigation

**Packages:** M2a route and catalog contract; M2b Course chooser and one-Course entry.

Derive Student Tier 2 from Student Tier 1, not route task groups. Add Progress, All Coursework, Due
Soon, Completed, Active Attempt, Scores, Response Stats, Attempt History, and Latest Feedback
destinations. Preserve Course IDs in all scoped links. Keep the row labels and order stable when
Course selection is absent. Keep Instructor behavior unchanged.

**Exit:** Permanent contract coverage proves fixed labels/order across Student routes and Course
changes, including Attempts and reviews. One-Course and two-Course entry work.

### M3 - Deliver Course Progress

**Packages:** M3a Progress API and data access; M3b Progress page and route.

M3a adds `GET /api/student/course-instances/{course_instance_id}/progress`. It returns the signed-in
Student's released Course Assessments, Attempt counts and activity, score state/freshness, and
released current/best/latest scores only where disclosure permits. It distinguishes no Attempts,
Attempts with no released/disclosed score, and released scores below or at 100%.

M3b builds the Progress page from that contract. Keep submitted completion distinct from perfection.
Show Attempts with unreleased scores under **Score not released** so they remain useful and visible;
do not classify them as below 100%. Include Assessments with no Attempts as not started. Do not show
cohort values or a weighted Course grade.

**Exit:** API tests prove self-only Course access and disclosure. Page contract/browser evidence
proves all three progress states and preserves the unreleased-score item.

### M4 - Deliver Course Attempt History

**Packages:** M4a paginated API and data access; M4b Grades page and review links.

M4a adds `GET /api/student/course-instances/{course_instance_id}/assessment-attempts` under the
repository cursor-pagination contract. Return only the signed-in Student's Attempts in the
authorized Course, newest first, with Assessment identity and disclosed scores where permitted.

M4b builds Grades -> Attempt History and links each item to its existing Attempt review. Reuse
per-Assessment previous-Attempt behavior and RecordList conventions where they fit.

**Exit:** API coverage proves authorization, ordering, disclosure, and pagination. A temporary
40-Attempt fixture shows that older Attempts remain reachable; the page links to authorized reviews.

### M5 - Add Coursework views

**Packages:** M5a Due Soon and Completed filters; M5b routes and page links.

Keep All Coursework as the existing Course list. Due Soon follows the existing rolling seven-day
window with server-owned membership and Account-zone display. Completed contains Assessments with
at least one submitted Attempt. The views may overlap; membership does not change Assessment
lifecycle state or available actions.

**Exit:** Permanent coverage protects window boundaries and submitted completion; browser evidence
confirms existing actions and statuses remain available.

### M6 - Deliver outcome-based Response Stats

**Packages:** M6a Response Stats API and aggregation; M6b Response Stats page.

M6a adds `GET /api/student/course-instances/{course_instance_id}/response-stats`. Group by exact
immutable Published Question revision. Return full-credit, partial-credit, incorrect, unanswered,
disclosed-attempt, and not-full-credit counts only as allowed by each Assessment's disclosure
policy. Rank by not-full-credit count and include visible numerator and denominator.

M6b adds Grades -> Response Stats, showing Course-scoped outcomes and a relevant existing Attempt
review link. It displays no cohort metrics or hidden correctness.

**Exit:** API coverage proves self-only aggregation, exact-revision grouping, score disclosure, and
Course isolation. Page evidence shows counts and review links.

### M7 - Persist Question display duration

**Packages:** M7a base schema and SQL checkpoint; M7b authenticated Rust/API integration and types.

Add nullable cumulative display-duration milliseconds to Question Attempt Student Work. Add an
authenticated cumulative checkpoint for a Question in the signed-in Student's open Attempt. Accept
only nonnegative cumulative milliseconds, make retries idempotent, and reject changes after
finalization. Keep duration within existing Student Work retention and add no raw event log.

**Exit:** Permanent tests cover ownership, idempotent cumulative writes, nonnegative values,
finalization, and retention. Attempts without a measured duration return no timing value. Regenerate
TypeScript types after Rust contracts are ready.

### M8 - Measure and show Question display duration

**Packages:** M8a Attempt-page measurement; M8b Response Stats duration aggregation and display.

Measure monotonic elapsed milliseconds while a Question is current and the document is visible.
Pause on Question change, hidden document, leaving the Attempt, and submission. Resume from the
saved cumulative value. Checkpoint periodically and at transitions and submission. Response Stats
reports average measured duration and sample count per Question, labelled approximate **time shown
with the Question**.

**Exit:** Browser evidence covers changes, hidden/visible transitions, resume, and submission.
Values and sample counts are plausible. No claim about attention, effort, or difficulty is made.

### M9 - Publish evidence and close out

**Packages:** M9a screenshot scenarios and publication; M9b contract/schema docs; M9c repository
gates and cleanup.

Capture populated Progress, Response Stats, a two-Course chooser, and Attempt History with 40
Attempts and a selected review. Retain representative question formats and meaningful response and
review states without building a full format/state/viewport matrix. Publish with the canonical
screenshot workflow and inspect scenario definitions, manifest, receipt, atlas, and native PNGs.

Update Design Decisions, API contracts, generated schema docs, Ribbon task model, design guide, and
`docs/CHANGELOG.md`. Keep Human Guidance unchanged. Regenerate disposable development/test schemas and generated
TypeScript contracts through repository workflows.

**Exit:** Screenshot artifacts are current and visually inspected; responsive layouts are checked;
temporary fixtures are removed; fast and full repository gates pass. If an external dependency
blocks a gate, report that gate as incomplete with the dependency and observed error.

## Integration rules

- API and UI packages have separate owners and narrow acceptance evidence. Integrate the UI against
  the settled API contract; generated TypeScript output has one integration owner.
- Shared RecordList changes follow `record_list_migration_plan.md`; shared Attempt-page changes follow
  `student_task_surface_plan.md`. Do not patch a shared component concurrently.
- Student identity comes from the authenticated session. Every Course read and write enforces active
  Course membership server-side and uses the repository's strict decoding, no-store, error, and
  authorization contracts.
- Edit the canonical base schema directly while PLE is pre-production. Do not add production
  migration machinery.
- Update the changelog after each bounded implementation patch. Run the narrow checks for that
  package before integration; run schema generation/style, TypeScript generation, screenshot
  publication, fast checks, and the full `all_test.sh` gate at their stated milestones.
- Keep durable tests for fixed navigation, authorization, disclosure, score/completion meaning,
  pagination, Due Soon/Completed rules, exact-revision aggregation, and duration persistence.
  Temporary stress fixtures, geometry probes, and screenshot inspection belong under
  `tests/_temp/` and are removed at closeout. Keep published corpus artifacts.

## Acceptance criteria

- Student Tier 2 labels and order depend only on Student Tier 1, including on Attempt and review
  routes. The selected Course is preserved; one-Course entry and two-Course selection work.
- A Progress Assessment with Attempts but no released/disclosed score remains visible as **Score not
  released** and is not treated as below 100%. Completion and score perfection remain distinct.
- APIs reveal only the signed-in Student's authorized Course data and apply score/feedback
  disclosure before returning any aggregate. No cohort analytics are exposed.
- Attempt History is newest-first and cursor-paginated without truncating older Attempts.
- Due Soon follows the existing seven-day convention. Completed means a submitted Attempt, not a
  perfect score. The views may overlap.
- Question duration is nullable, cumulative, idempotent, frozen at finalization, labelled
  approximate time shown with the Question, and never affects grades. Missing measurements are not
  inferred from Assessment elapsed time.
- Relevant API and schema docs, generated types, screenshot corpus artifacts, and repository gates
  reflect the implemented contracts.
