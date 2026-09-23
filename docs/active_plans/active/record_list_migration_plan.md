# Plan: RecordList migration

## Context

The current `RecordList` proof has seven distinct consumers from WP-E1 through WP-E7. The
independent 2026-09-22 source pass first identified 55 repeated-render candidates: seven converted
proof pages and 48 apparent remaining sites. Source re-review excludes
`proposal_review.tsx`: its literal `source`/`target` pair renders the two frozen sides of one
comparison, with nested Blueprint/module/Assessment content belonging to those fixed sides. It is
not a homogeneous set of independently actionable records. The reconciled inventory is therefore
54 user-facing record sites: seven converted proof pages and 47 remaining sites. The earlier
48 = 7 + 41 count omitted six requested sites; it must not be combined with the subsequently
excluded comparison pair.

Stability assessment: the newest changelog records focused TypeScript, Rust, Node, formatting, and
diff checks passing, while one clean Live Demo replay remains pending. There is no current evidence
in this plan that `run_fast_checks.sh` is red, so the earlier red-fast-gate assertion is removed.
This is a migration plan, not a stabilization or refactor plan: each implementation package still
requires a fresh green fast-gate run before it starts, and this plan makes no full-suite-green
claim.

## Objectives

- Move each of the 47 remaining inventoried record sites to `RecordList` after its package has a
  fresh green fast-gate baseline.
- Preserve the proven WP-E patterns without adding page-specific component escape hatches.
- Record the current `instructor_data_tables.css` evidence and defer its cleanup decision until a
  follow-up has performed a fresh last-user check.

## Design philosophy

Use the smallest shared component that already proved its boundaries: regions own alignment and
responsive retention; presentation, reorder, and windowing compose outside it. A broad markup
rewrite is rejected because a repeated loop is not automatically a scan list. This follows KISS in
[docs/REPO_STYLE.md](../../REPO_STYLE.md): migrate only user-facing repeated records with a real
list/table task.

## Scope

- Convert the 47 sites in the inventory below.
- Declare a pattern, supported presentation variants, and semantic region roles for every site.
- Remove local repeated-row markup and CSS only after the replacement renders and its last user is gone.
- Keep the existing API and task behavior of each page.

## Non-goals

- Do not change question-authoring choice editors, option selectors, navigation ranges, or static tables.
- Do not add global presentation preferences or a generic table framework.
- Do not delete `src/pages/instructor_data_tables.css`; this plan has no evidenced current stylesheet
  user or deletion dependency to act on.
- Do not begin an implementation package without a fresh green fast-gate baseline.

## Current state summary

`RecordList` currently supplies shared loading, empty, error, region, subgrid, and narrow-screen
behavior. The seven proof pages establish simple scan, dense table, variants plus windowing,
reorder, action-bearing course rows, student action adjacency, and gallery/list presentations.

| Inventory result             |       Sites |
| ---------------------------- | ----------: |
| Fresh inventory total        |          54 |
| Converted WP-E1--WP-E7       |           7 |
| Remaining below              |          47 |
| Check: converted + remaining | 7 + 47 = 54 |

Region shorthand: I = identity, M = metadata, S = status, A = actions. `--` means one fixed
presentation, not an absent design decision.

## Data inventory

| Remaining site | Page or component                                        | Pattern                                                | Variants | Region roles |
| -------------: | -------------------------------------------------------- | ------------------------------------------------------ | -------- | ------------ |
|              1 | `student_courses_page.tsx`                               | student course scan                                    | --       | I, S, M, A   |
|              2 | `course_list_page.tsx` Blueprint Courses                 | course comparison rows                                 | --       | I, M, S, A   |
|              3 | `course_list_page.tsx` Course Instances                  | course activity rows                                   | --       | I, S, M, A   |
|              4 | `course_roster_page.tsx`                                 | dense roster table                                     | --       | I, M, S, A   |
|              5 | `assessments_due_soon_page.tsx`                          | deadline scan                                          | --       | I, S, M, A   |
|              6 | `assessment_templates_page.tsx`                          | template scan                                          | --       | I, M, S, A   |
|              7 | `assessment_overview_page.tsx` previous Attempts         | compact history                                        | --       | I, S, M, A   |
|              8 | `assessment_attempt_summary_page.tsx`                    | submitted Question review units; expanded review cases | --       | I, S, M      |
|              9 | `student_course_grades_page.tsx`                         | student grade rows                                     | --       | I, S, M      |
|             10 | `student_course_invitations_page.tsx`                    | invitation scan                                        | --       | I, M, S, A   |
|             11 | `account_pending_invitations_page.tsx`                   | account invitation scan                                | --       | I, M, S, A   |
|             12 | `question_statistics_panel.tsx` revisions                | revision history                                       | --       | I, M, S, A   |
|             13 | `question_statistics_panel.tsx` Course use               | compact usage rows                                     | --       | I, M, S      |
|             14 | `library_discussion_panel.tsx` threads                   | discussion scan                                        | --       | I, M, S, A   |
|             15 | `library_discussion_panel.tsx` posts                     | chronological discussion                               | --       | I, M, S, A   |
|             16 | `course_student_work_recovery.tsx` Attempts              | recovery history                                       | --       | I, S, M, A   |
|             17 | `course_student_work_recovery.tsx` Questions             | recovery evidence rows                                 | --       | I, S, M      |
|             18 | `library_pool_discovery.tsx` results                     | searchable pool scan                                   | --       | I, M, S, A   |
|             19 | `library_pool_discovery.tsx` pool members                | member rows                                            | --       | I, M, A      |
|             20 | `question_picker.tsx` search results                     | selectable library scan                                | --       | I, M, S, A   |
|             21 | `question_picker.tsx` selected Questions                 | selection tray rows                                    | --       | I, M, A      |
|             22 | `assessment_student_time_accommodations.tsx`             | accommodation table                                    | --       | I, M, S, A   |
|             23 | `assessment_fixed_question_points_editor.tsx`            | editable points rows                                   | --       | I, M, A      |
|             24 | `assessment_pool_entry_editor.tsx` members               | pool-member editor                                     | --       | I, M, A      |
|             25 | `assessment_pool_entry_editor.tsx` candidates            | addable Question scan                                  | --       | I, M, S, A   |
|             26 | `assessment_workspace_student_view_page.tsx`             | answer-free Question preview rows                      | --       | I, M, S      |
|             27 | `course_blueprint_update_review.tsx`                     | update Assessment comparison                           | --       | I, M, S, A   |
|             28 | `blueprint_course_search_page.tsx`                       | public Blueprint result scan                           | --       | I, M, S, A   |
|             29 | `blueprint_courses_workspace.tsx`                        | Blueprint Course scan                                  | --       | I, M, S, A   |
|             30 | `blueprint_course_detail_workspace.tsx`                  | Course structure tree rows                             | --       | I, M, S, A   |
|             31 | `blueprint_pool_members_editor.tsx`                      | editable pool members                                  | --       | I, M, A      |
|             32 | `question_pool_picker.tsx` results                       | pool discovery scan                                    | --       | I, M, S, A   |
|             33 | `question_pool_picker.tsx` members                       | selected pool members                                  | --       | I, M, A      |
|             34 | `blueprint_history.tsx` revisions                        | revision history                                       | --       | I, M, S, A   |
|             35 | `blueprint_history.tsx` Assessment entries               | immutable content rows                                 | --       | I, M, S      |
|             36 | `blueprint_stewardship.tsx` activity                     | stewardship activity                                   | --       | I, M, S      |
|             37 | `blueprint_fork_review.tsx` differences                  | side-by-side comparison rows                           | --       | I, M, S      |
|             38 | `blueprint_fork_apply.tsx` layout                        | ordered module/Assessment rows                         | --       | I, M, S, A   |
|             39 | `instructor_accounts_page.tsx`                           | Instructor Account administration                      | --       | I, S, M, A   |
|             40 | `library_watch_notifications_page.tsx`                   | chronological Watch activity                           | --       | I, S, M, A   |
|             41 | `content_disciplines_page.tsx`                           | Discipline management                                  | --       | I, S, A      |
|             42 | `library_discussion_panel.tsx` Impact notices            | managed Impact-notice scan                             | --       | I, M, S, A   |
|             43 | `question_pool_create_dialog.tsx` selected Questions     | selected-Question confirmation                         | --       | I, M         |
|             44 | `question_bulk_metadata_editor.tsx` current-values cards | selected-Question metadata cards                       | --       | I, M, S      |
|             45 | `blueprint_assessment_content_editor.tsx` entries        | editable Assessment-content entries                    | --       | I, M, S, A   |
|             46 | `blueprint_fork_review.tsx` known forks                  | known-fork comparison scan                             | --       | I, M, S, A   |
|             47 | `proposal_workspace.tsx` Change Proposals                | paged Change-Proposal scan                             | --       | I, M, S, A   |

Converted evidence retained for pattern selection: WP-E1 `question_drafts_page.tsx` (simple scan),
WP-E2 `gradebook_page.tsx` (dense full-width table), WP-E3 `library_browse_rows.tsx` (scan/preview
with windowing), WP-E4 `assessment_workspace_questions_view.tsx` (keyboard and drag reorder),
WP-E5 `course_instance_page.tsx` (action-bearing dense rows), WP-E6
`student_course_landing_page.tsx` (student action next to identity), and WP-E7
`provided_avatar_picker.tsx` (genuinely different gallery/list shapes).

## Excluded repeated-render candidates

The source pass inspected user-facing Solid `For` and array-render loops, then excluded the
following candidates because they do not present independently actionable records. They remain out
of the total unless their task changes.

| Candidate group                                                                                                                                                                       | Exclusion reason                                                                                                                                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `application_shell.tsx`, `app_ribbon.tsx`, `student_assessment_attempt_navigation.tsx`                                                                                                | Navigation controls and numbered/ranged navigation are excluded.                                                                                                                         |
| `bloom_classification.tsx`, `library_bloom_discovery.tsx`, `content_classification_select.tsx`, `library_page.tsx`, Blueprint/Assessment create and settings editors                  | These loops render form choices, facets, or fixed field definitions, not records.                                                                                                        |
| `question_response_controls/`, `question_json_*/`, `question_response_preview.tsx`, `question_star_control.tsx`                                                                       | These are answer-authoring controls, response choices, or per-Question controls, not a repeated-record task.                                                                             |
| `question_renderer.tsx`, `student_feedback_panel.tsx`                                                                                                                                 | These render authored Question/feedback blocks and static content tables, which are explicitly excluded.                                                                                 |
| `assessment_workspace_policies_page.tsx` validation issues and `question_detail_page.tsx` authors                                                                                     | These are fixed diagnostic strings or an inline value, not independently managed rows.                                                                                                   |
| `course_appearance_page.tsx` and `provided_avatar_picker.tsx` option loops                                                                                                            | Theme choices are form options; the avatar picker is already WP-E7 converted proof evidence.                                                                                             |
| `assessment_workspace_questions_view.tsx`, `course_instance_page.tsx`, `gradebook_page.tsx`, `library_browse_rows.tsx`, `question_drafts_page.tsx`, `student_course_landing_page.tsx` | These are the seven converted proof pages (with `provided_avatar_picker.tsx` above), not remaining migration work.                                                                       |
| `proposal_review.tsx` source/target pair                                                                                                                                              | The literal two-side loop renders one fixed frozen source/target comparison. Its nested Blueprint, module, and Assessment content serves that pair, not independent homogeneous records. |
| Nested Question/Pool entry lines inside comparison snapshots                                                                                                                          | These show fixed content belonging to their parent Assessment comparison; migrating them separately would split one user task into speculative nested lists.                             |

## Approach

1. Run the fast gate, record its fresh result, and freeze the inventory against the merge base for
   the package. A failure stops that package for stabilization outside this plan.
2. Migrate small whole-file packages rather than numeric batches; one owner holds every file in a
   package. `library_discussion_panel.tsx` is one package containing its threads (site 14), posts
   (site 15), and Impact notices (site 42), so its three coupled surfaces never receive different
   owners. Other packages may contain every remaining site in one page or component, but no file
   may cross package ownership.
3. For every site, define regions from the table before moving markup. Use WP-E2 for genuine
   tables, WP-E4 only where reorder semantics match, and WP-E3 only where result-windowing is needed.
4. After each group, rerun the inventory. A site is complete only when it renders through
   `RecordList`; a deleted loop without a replacement is an inventory failure.
5. Record, but do not act on, the stylesheet evidence: the 2026-09-22 check found no active
   `instructor_data_tables.css` import. It found `roster-table*` markup classes in
   `course_roster_page.tsx`, but that alone does not establish a current stylesheet user; the roster
   also has its sizing token in `src/style.css`. The check for a later deliberate cleanup decision
   excludes the candidate stylesheet itself: `rg -n -g '!instructor_data_tables.css'
'instructor_data_tables|gradebook-(table|calculation-status|course-total|assignment-cell|cell-score)|roster-table' src`.
   Per WP-H1, retain the sheet pending a fresh last-user check. A follow-up must make the cleanup
   decision deliberately; this migration plan neither identifies a current user nor authorizes
   deletion.

## Files to modify

- The 47 remaining sites and their owning page/component files, plus page-local CSS where the last
  selector moves. `src/pages/instructor_data_tables.css` is retained and is outside this plan's
  source modifications.
- `docs/active_plans/active/record_list_migration_plan.md` only to record an inventory reconciliation.

## Acceptance criteria and gates

- The reconciliation remains exactly `7 converted + 47 remaining = 54 inventoried sites` until a
  source addition/removal is recorded with its reason.
- `proposal_review.tsx` remains excluded unless its fixed source/target comparison task changes
  into independently actionable homogeneous records.
- Every migrated site declares only the region roles and presentation variants its task uses.
- No new `RecordList` prop exists solely for one caller.
- The three `library_discussion_panel.tsx` sites have one whole-file owner.
- Site 8 keeps its compact review units while checking the M6 `PageFrame` outer rail with long
  feedback, multipart responses, and several reviewed Questions; a short current capture is not
  sufficient acceptance evidence.
- Student sites 1 and 10 keep Coursework and invitation objects compact: retain identity, state,
  essential status, and the next action as the primary scan group; place fuller timing or
  explanatory detail in a compact aligned group or disclosure. This is the SUI-08 object-density
  acceptance lane, distinct from the active-Attempt vertical budget. Its rendered wide and narrow
  acceptance remains deferred until the relevant migration packages have a fresh green baseline.
- `instructor_data_tables.css` remains unchanged until a follow-up completes a fresh last-user check
  and records a deliberate cleanup decision; no present import or markup class is asserted to be a
  stylesheet user.
- The repository fast gate must have a fresh green result before and after each package; a failure
  blocks migration and is handled by a separate stabilization plan.

## Risk register

| Risk                                        | Impact                              | Trigger                                                       | Owner           | Mitigation                                                             |
| ------------------------------------------- | ----------------------------------- | ------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------- |
| A nested editor is mistaken for a scan list | Incorrect shared abstraction        | The site needs editor-local interaction state in every region | implementer     | Keep it out of this inventory and record why before changing the total |
| Shared CSS is deleted without evidence      | Lost residual roster styling        | A cleanup is proposed before a fresh last-user check          | follow-up owner | Retain `instructor_data_tables.css`; decide cleanup only in follow-up  |
| Concurrent edits change the inventory       | A site is skipped or double-counted | Merge base differs from fresh sweep                           | manager         | Re-run the inventory before each group and reconcile the table         |

## Verification

Run `source ./source_me.sh && ./launchers/run_fast_checks.sh` for the fresh baseline and after each
package; run the affected focused browser evidence where it exists. Review the rendered narrow and
wide rows for identity and primary-action retention. `git diff --check` and a fresh inventory
reconciliation are required before closing the final package.
