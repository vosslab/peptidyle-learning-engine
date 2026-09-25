# RecordList caller investigation: shared standards and expressive content

Date: 2026-09-25. Status: source investigation complete; implementation pending.

This follow-up revises the architectural recommendation in the
[original RecordList/PageFrame audit](record_list_page_frame_standardization_audit_2026-09-25.md).
The [implementation plan](../active/record_list_page_frame_standardization_plan.md) and
[execution ledger](../workstreams/record_list_page_frame_standardization_ledger.md) carry the changes.
The table below includes every direct RecordList/RecordSequence consumer, not a sample.

## Follow-up: bounded data and performance

The [data-flow investigation](record_list_data_flow_investigation_2026-09-25.md) traces all major
catalog-discovery paths. RecordList stays Solid/TypeScript; PostgreSQL/API reduce broad collections.
It found Question source resolution before server paging, accumulating browser results, partial
Blueprint sorting and a full-catalog Assessment picker. The required bounded data workstream corrects
those paths, keeping the 50 default and raising the discovery ceiling for the human's 250-record scanning need,
with shared 50/100/250 choices.
It supersedes earlier advice to keep My Blueprint's local comparator or retain windowing unconditionally.

## Conclusion

The earlier three-field-only proposal was too restrictive. It removed real distinctions between
supporting prose, short facts, links, media and action state, while leaving Sequence callers to render
whole bodies. That could recreate custom presentation outside the standard.

Use shared semantic content, a bounded domain-body area and controlled collection behaviors. Pages
describe the record and its available operations; the shared family renders them. RecordList owns
padding, typography, media size/fit, spacing, actions, selection, state notices and responsive layout.
Sequence adds meaningful order and shared movement controls. Sort controls compose with real filters,
while query, comparator, draft and persistence policy stay with the domain owner.

This is a design correction, not a reason to restore arbitrary region widths, priority rules or
page-owned row CSS. A useful description, image, linked Course or extra action belongs inside the
ordinary framework. Tables, full reviews, hierarchies and primary editing tasks retain their siblings.

## Evidence and limits

The initial inventory inspected all 38 direct consumer files and their helpers. This deeper pass
revisited media, descriptions, links, pressed commands, inline editors, ordering, collection states,
Library presentation/windowing and backend preview boundaries. A fresh source inventory confirmed
37 RecordList sites in 32 files and 11 RecordSequence sites in 11 files: 38 distinct consumer files.
Multiple sites and mixed families stay together in the table and in whole-file execution leases.

Targeted Graphify was refreshed against the September 25 08:13 CDT map (19,199 nodes). Conclusions
were checked in current source. Existing native and WeBWorK detail screenshots were inspected; they
show real renderer output but are historical artifacts, not a fresh UI acceptance run. No production
code changed and no product tests ran during this planning investigation.

Primary authority remains [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), including content-sized
surfaces, compact density, Assessment Type identity and visual avatar selection. The user's latest
clarification establishes the content/presentation ownership boundary. Existing
[DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md) supplies backend opacity and domain save-policy context;
its earlier compact-row wording must be reconciled at implementation closeout.

## Findings that change the plan

| Finding and current evidence | Consequence for the shared framework | Implementation owner / proof |
| --- | --- | --- |
| Question browse/picker, Pool discovery/picker, Drafts and avatars contain useful descriptions | Give supporting prose its own optional description field; preserve short facts separately | CORE WP-C1; LIBRARY/BLUEPRINT/AVATAR populated caller receipts |
| Due Soon has both an Assessment destination and a Course destination | Shared labeled link fact preserves the Course link; removing it is behavior loss | CORE WP-C1, INSTRUCTOR caller proof |
| Template buttons use aria-pressed for the loaded editor | Shared command supports pressed and disabled state; keep dirty replacement policy local | CORE WP-C1, INSTRUCTOR Template proof |
| Avatar List and Gallery independently implement images, labels and radios | Optional media belongs in the common content model; Gallery changes shared presentation of the same data | CORE WP-C1/C3/C5, AVATAR native radio and image-failure proof |
| Library Preview only rearranges metadata and adds Copy ID; it does not render a Question and omits the retired-Discipline notice | Consolidate into one complete scan with useful description, reference/copy and all decision facts | CORE WP-C4, LIBRARY browse/return/window proof |
| Course Instance puts conditional editors in a second loop after the list | Associate the existing editor with its record through one bounded body area owned by the shared frame | CORE WP-C1, INSTRUCTOR save/cancel/conflict/focus proof |
| All 11 Sequence sites duplicate identity/action content; six allow movement | Sequence shares the record header/facts/actions; real fields use a bounded body, shared controls own movement | CORE WP-C6/C7, six caller movement receipts |
| Assessment Pool replacement is asynchronous; Blueprint and picker changes are local drafts | Movement mechanics can be shared; announcements follow actual order changes and persistence stays local | CORE WP-C7 with local, successful delayed and rejected replacement cases |
| Library maps loaded loading/error states to ready and builds notices outside the family | Shared state rendering retains rows and selection while showing loading/error/Retry | CORE WP-C1 and LIBRARY removal of duplicate notices |
| Library server sort and My Blueprint Courses local sort repeat a labeled select | One shared sort control; page query choices stay local, full-query order belongs to PostgreSQL/API | CORE WP-C8; WP-P4 Blueprint global order; caller receipts |

## Follow-up: native behavior and stable ownership

A second boundary pass checked whether the richer schema would still force local workarounds:

| Concrete caller evidence | Required correction | Owner / success evidence |
| --- | --- | --- |
| [blueprint_fork_review.tsx](../../../src/features/blueprint_forks/blueprint_fork_review.tsx):455 marks Compare as expanded and identifies its comparison region | Native command descriptors carry expanded state and controls ID in addition to Template pressed state | CORE WP-C1, BLUEPRINT caller: open/close relationship and focus survive |
| [course_instance_page.tsx](../../../src/pages/course_instance_page.tsx):268-307 stores action refs; Edit unmounts while editing | Keep a narrow action element ref for focus return to the recreated Edit or read-only link; an event callback alone is insufficient | CORE WP-C1, INSTRUCTOR caller: close/save/conflict return focus to the actual current control |
| Invitation Accept and Draft Open have different task emphasis from Decline/Delete | Supply semantic primary-action intent; shared code selects fixed control styling | CORE WP-C1; caller preserves useful action hierarchy without CSS variants |
| List/Sequence iterate row objects; Course rows at line 563 are recreated around stable models | Mount by record ID and read current data reactively; key actions by stable action ID; invoke bodies with reactive row access | CORE WP-C1/C6: same-ID refresh retains a typed draft and focused input, while metadata/disabled state updates |
| Library notices belong to the logical loaded collection, not its virtual slice | Compose the existing shared state boundary outside the scrolling/spacer subtree; inner rows remain ordinary shared records | LIBRARY: loading/error/retry retains selection and useful window position without distorting measurements |

One layering issue also affects implementation elegance:
[record_list_reorder.tsx](../../../src/components/record_list/record_list_reorder.tsx):3 imports its
pure array operation from the Blueprint fork feature model. WP-C7 moves that operation into shared
ownership and the Blueprint caller consumes it. This is a small dependency inversion, with domain
validation left in the feature; the final cutover removes any temporary re-export. Shared components
should not acquire a dependency on a page feature merely to move an array element.

The installed Solid implementation compares `<For>` items by item identity. The source establishes a
remount risk when callers rebuild row objects; this investigation did not run a browser reproduction.
The execution proof covers the actual draft/focus behavior. It does not require DOM byte equivalence
or another state-management layer. Useful native semantics belong in the contract; arbitrary native
attribute bags and style callbacks add no demonstrated value here.

## All direct RecordList and RecordSequence callers

Each row states the actual content and interaction that the shared family must support. "Scan" means
the common content model, including descriptions, media, facts and actions when applicable. It does
not mean that every record must fill every field. A body area contains domain controls, not another
record header or a page-specific row surface. Source line numbers are investigation locators.

| Caller and current sites | Proposed family | Required content, behavior and disposition |
| --- | --- | --- |
| [course_student_work_recovery.tsx](../../../src/components/course_student_work_recovery.tsx); RecordList:228 | Selectable scan | Selectable scan: Assessment/Attempt title; roster and retained Attempt IDs; started/submitted/deletion-cutoff facts. Keep evidence Detail, selected-ID reset and focus return. |
| [question_pool_create_dialog.tsx](../../../src/components/question_pool_create_dialog.tsx); RecordSequence:185 | Sequence | Sequence: selected Question title and exact ID/revision in authored order; retain pool-creation form behavior. |
| [question_star_control.tsx](../../../src/components/question_star_control.tsx); RecordList:37 | Scan | Base: authorized Instructor display name only; keep surrounding disclosure. |
| [proposal_workspace.tsx](../../../src/features/blueprint_change_proposal/proposal_workspace.tsx); RecordList:143 | Scan | Base: source-to-target title; source/target short names and revisions, proposal state and created date; Open proposal. Keep paging. |
| [blueprint_assessment_content_editor.tsx](../../../src/features/blueprint_course/blueprint_assessment_content_editor.tsx); RecordSequence:292 | Sequence | Sequence: shared Question/Pool identity, revision/count/points facts and move/remove controls; bounded body for points/draw inputs and Pool editor; preserve deferred Blueprint Save. |
| [blueprint_courses_workspace.tsx](../../../src/features/blueprint_course/blueprint_courses_workspace.tsx); RecordList:259 | Scan | Scan: long/short names, revision, adoption/student counts, classification/access and Open. Shared sort select sends the choice to SQL/API; bounded pages use shared 50/100/250 choice and Previous/Next. Collapse only duplicate destinations. |
| [blueprint_history.tsx](../../../src/features/blueprint_course/blueprint_history.tsx); RecordList:347, RecordSequence:441 | Scan + Sequence | Base revision/name scan with saved/recorded date, availability/classification and Inspect command. Keep nested Module/Assessment Outline and historical-entry Sequence. |
| [blueprint_pool_members_editor.tsx](../../../src/features/blueprint_course/blueprint_pool_members_editor.tsx); RecordSequence:166 | Sequence | Sequence: exact Question revision facts, shared move/remove and disabled endpoints; preserve attestation and deferred Blueprint Save. |
| [blueprint_stewardship.tsx](../../../src/features/blueprint_course/blueprint_stewardship.tsx); RecordList:169, RecordList:181 | Scan | Two base scans: starred Instructor names; Watch event title and timestamp. Keep chronology, disclosures and outside-row controls. |
| [question_pool_picker.tsx](../../../src/features/blueprint_course/question_pool_picker.tsx); RecordList:188, RecordSequence:257 | Selectable scan + sequence | Selectable Pool scan: title, description, exact ID/edit and member count. Keep radio/stale-request behavior; ordered members use shared read-only Sequence, full inspection remains task UI. |
| [blueprint_fork_apply.tsx](../../../src/features/blueprint_forks/blueprint_fork_apply.tsx); RecordSequence:469 | Sequence | Sequence within Outline: shared content and move/remove controls, including Module movement; bounded body retains cross-Module destination select, restore and draft behavior. |
| [blueprint_fork_review.tsx](../../../src/features/blueprint_forks/blueprint_fork_review.tsx); RecordList:511 | Scan | Action scan: fork long name; short name, availability, source/fork revisions, owner/change facts; Open and Compare. Keep paired Detail review, expanded/controls semantics and trigger focus restoration. |
| [provided_avatar_picker.tsx](../../../src/features/profile_avatar/provided_avatar_picker.tsx); RecordList:211, RecordList:227 | Image scan / Gallery | Image scan/Gallery: one adapter supplies catalog asset URL, name and description; shared media owns size/fit/fallback, controlled radio and switcher own interaction and layout. |
| [question_picker.tsx](../../../src/features/question_picker/question_picker.tsx); RecordList:369, RecordSequence:434 | Selectable scan + sequence | Selectable Question scan: title, description and shared exact ID/reference. Selected tray uses shared Sequence movement/removal; preserve selection modes, limits and exact returned order. |
| [account_pending_invitations_page.tsx](../../../src/pages/account_pending_invitations_page.tsx); RecordList:245, RecordList:259 | Scan | Action scan: Course title, invitation state and expiry; Accept/Decline. Keep confirmation, busy state, reload and trigger/heading focus. |
| [assessment_overview_page.tsx](../../../src/pages/assessment_overview_page.tsx); RecordList:232 | Scan | Base: Attempt number, submitted/closed state and score/unavailable score; Review Attempt. |
| [assessment_templates_page.tsx](../../../src/pages/assessment_templates_page.tsx); RecordList:300 | Scan | Scan: Template name and shared Type fact; Edit command retains pressed/disabled state for the adjacent loaded editor, dirty replacement guard and focus. |
| [assessment_blueprint_update_review.tsx](../../../src/pages/assessment_workspace/assessment_blueprint_update_review.tsx); RecordSequence:167 | Sequence | Sequence: ordered exact Question/Pool identity and complete facts; retain separate current/proposed labels. |
| [assessment_fixed_question_points_editor.tsx](../../../src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx); RecordList:160 | Detail form | Existing Detail form: exact Question revision and editable points; preserve aggregate Save/Cancel, validation, conflicts and draft retention. |
| [assessment_pool_entry_editor.tsx](../../../src/pages/assessment_workspace/assessment_pool_entry_editor.tsx); RecordSequence:188 | Sequence | Sequence: exact revision/title plus shared move/remove. Keep attestation, count restrictions, asynchronous replacement and error recovery; announce only completed movement. |
| [assessment_workspace_questions_view.tsx](../../../src/pages/assessment_workspace/assessment_workspace_questions_view.tsx); RecordSequence:397, RecordList:454 | Scan + Sequence | Selected Sequence uses shared movement/removal and bounded Pool editor. WP-P5 replaces the full-catalog available scan with the existing paged Question/Pool pickers; retain exact revisions, Bloom-sort draft command, capacity and Save/conflict policy. |
| [assessments_due_soon_page.tsx](../../../src/pages/assessments_due_soon_page.tsx); RecordList:123 | Scan | Scan: Assessment title, shared Type, lifecycle/due time, linked Course context and Open Assessment. Preserve the separate real Course destination. |
| [blueprint_course_search_page.tsx](../../../src/pages/blueprint_course_search_page.tsx); RecordList:361 | Scan | Base: long name; short name, revision and adoption/student counts; Open Blueprint. Collapse duplicate title/Open links; preserve native link, return token and scroll/focus behavior. |
| [course_blueprint_update_review.tsx](../../../src/pages/course_blueprint_update_review.tsx); RecordList:144 | Scan | Base: Assessment title; update state, shared Type fact and ID; Review Assessment. Keep confirmation/disclosure/loading workflow. |
| [course_instance_page.tsx](../../../src/pages/course_instance_page.tsx); RecordList:643 | Scan | Scan: position/title, Type, lifecycle/due facts and Edit commands. Existing conditional inline form/conflict messages live in the bounded record body; retain row models, dirty state and native action refs for focus after Edit remounts. Same-ID refresh preserves the body owner. |
| [course_list_page.tsx](../../../src/pages/course_list_page.tsx); RecordList:522 | Scan | Base: Course long name, classification and term; Open Course. Remove repeated Course Instance kind, theme name and accent decoration. |
| [instructor_accounts_page.tsx](../../../src/pages/instructor_accounts_page.tsx); RecordList:290 | Detail form | Existing Detail lifecycle form: Account ID, state, last sign-in, reason and permitted action; preserve validation and per-account busy state. |
| [library_browse_rows.tsx](../../../src/pages/library_browse_rows.tsx); RecordList:475, RecordList:509 | Selectable scan | One selectable Question scan: title, description, authors, classification/Bloom/format/retired notice, exact ID/copy and Open. Consolidate metadata modes and loaded-state notices; retain return tokens, measurement and focused row; place the shared state boundary outside window spacers. |
| [library_pool_discovery.tsx](../../../src/pages/library_pool_discovery.tsx); RecordList:410, RecordSequence:539 | Scan + Sequence | Pool scan: title, description, classification/Bloom/exact identity/count and Inspect. Preserve heading/trigger/scroll return; immutable members use shared read-only Sequence. |
| [library_watch_notifications_page.tsx](../../../src/pages/library_watch_notifications_page.tsx); RecordList:144 | Scan | Base: event title, target identity, revision/fork/activity facts and timestamp; optional Open exact activity. Every detail remains visible. |
| [question_drafts_page.tsx](../../../src/pages/question_drafts_page.tsx); RecordList:288 | Scan | Scan: Draft title, useful description and edit number; Open/Edit and Delete with confirmation. Keep content hierarchy and collapse only duplicate navigation. |
| [question_statistics_panel.tsx](../../../src/pages/question_statistics_panel.tsx); RecordList:142, RecordList:190 | Scan | Two base scans: Revision number plus mean credit/observation count; Course title plus Assessment count and Open Course. Preserve denominators. |
| [student_course_attempt_history_page.tsx](../../../src/pages/student_course_attempt_history_page.tsx); RecordList:143 | Scan | Base pilot: Assessment title; Attempt number/state, started/submitted times and score state; Open/Review. Keep Course grouping and independent pagination. |
| [student_course_grades_page.tsx](../../../src/pages/student_course_grades_page.tsx); RecordList:69 | Scan | Base: Assessment title, shared Type fact and released score; retain Course grouping. |
| [student_course_invitations_page.tsx](../../../src/pages/student_course_invitations_page.tsx); RecordList:83 | Scan | Base: Course title, Instructor and term; Review invitation. |
| [student_course_landing_page.tsx](../../../src/pages/student_course_landing_page.tsx); RecordList:150 | Scan | Base: Assessment title; shared Type icon/label, access/reason, due time and completion; Start/Open/Review. Shared code chooses Type icon/color/placement from its enum. |
| [student_course_progress_page.tsx](../../../src/pages/student_course_progress_page.tsx); RecordList:170 | Scan | Base pilot: Assessment title and shared Type fact; Attempt/submission counts, latest activity, score/activity state and explanation; Open Assessment. Keep Course grouping. |
| [student_courses_page.tsx](../../../src/pages/student_courses_page.tsx); RecordList:64 | Scan | Base: Course long name and Open Course. Move invitations to page actions in WP-F1. |

### Adjacent collection controls

| Caller | Required behavior | Shared treatment / owner |
| --- | --- | --- |
| [library_page.tsx](../../../src/pages/library_page.tsx):716 | Title/recent-publication server order, editor-busy disabling, query reset and return-state restoration | RecordSortControl; WP-C8 supplies it, WP-L1.library_page migrates the owner |
| [blueprint_courses_workspace.tsx](../../../src/features/blueprint_course/blueprint_courses_workspace.tsx):243 | Name/adoption/student sort over loaded Course records | Same control; WP-P4 replaces the partial local comparator with query-wide API/SQL ordering |
| [assessment_workspace_questions_page.tsx](../../../src/pages/assessment_workspace/assessment_workspace_questions_page.tsx):214 | Sort draft entries by Bloom, mark dirty and save explicitly | Existing domain command through shared action styling in the view; retain this model's policy |

Public Blueprint search currently has no sort selector in its TSX owner. Human Guidance describes
future useful sort fields; Public Search keeps its existing name order. WP-P4 implements only the existing My Blueprint sort
choices across the full query; shared page controls serve both surfaces.

### Existing siblings retained

| Existing consumer | Confirmed task and disposition | Execution owner |
| --- | --- | --- |
| [gradebook_page.tsx](../../../src/pages/gradebook_page.tsx) and [course_roster_page.tsx](../../../src/pages/course_roster_page.tsx) | Table: explicit column comparisons and row headers | FR-SHELL for frame; INTEGRATOR for existing Table behavior |
| [blueprint_course_detail_workspace.tsx](../../../src/features/blueprint_course/blueprint_course_detail_workspace.tsx) | Outline: nested Modules and Assessments, with existing selected editor | NAVIGATION then FR-WORKSPACE |
| [library_discussion_panel.tsx](../../../src/components/library_discussion_panel.tsx) | Detail: Impact notices and threaded discussion bodies/actions | INTEGRATOR preserves existing behavior |
| [question_bulk_metadata_editor.tsx](../../../src/components/question_bulk_metadata_editor.tsx) | Detail: current values of selected Questions inside review disclosure | INTEGRATOR preserves existing behavior |
| [assessment_attempt_summary_page.tsx](../../../src/pages/assessment_attempt_summary_page.tsx) | Detail: recorded answers, feedback and backend-rendered Question content | FR-STUDENT for frame; INTEGRATOR for existing review behavior |
| [content_disciplines_page.tsx](../../../src/pages/content_disciplines_page.tsx) | Detail: per-Discipline edit/lifecycle form | FR-SHELL for frame |
| [student_course_response_stats_page.tsx](../../../src/pages/student_course_response_stats_page.tsx) | Detail: disclosed Question-level outcome review with exact revision identity | FR-STUDENT for frame |

The separate [PageFrame table](../workstreams/record_list_page_frame_standardization_ledger.md#pageframe-consumer-sweep)
accounts for every one of its 45 direct consumer files and 62 sites, including loading/error branches.
Its scope and per-file navigation handoffs remain intact.

## Question snapshot investigation

### What exists

- [library_page_model.ts](../../../src/pages/library_page_model.ts):41 and
  [QuestionSearchResult.ts](../../../generated/api/QuestionSearchResult.ts) expose title, description,
  metadata and exact revision, with no thumbnail or prompt payload. RecordList cannot derive a
  faithful image from those rows alone.
- [question_detail_page.tsx](../../../src/pages/question_detail_page.tsx):512 uses
  [QuestionPromptRenderer](../../../src/components/question_renderer.tsx) and
  [QuestionResponsePreviewControl](../../../src/components/question_response_preview.tsx) for native,
  answer-free inspection. This is an existing useful visual path, not a snapshot service.
- [OpaqueWebworkPreviewFrame](../../../src/components/opaque_webwork_preview_frame.tsx) displays an
  authorized backend document using a script-only opaque sandbox. The bridge reports size; its
  similarly named response-capture path is not a screenshot API.
- [question_library.rs](../../../crates/server/src/question_library.rs):408 authorizes an exact revision
  before creating its WeBWorK preview document. `preview_question_seed` at line 593 chooses a fresh
  random seed. Repeated previews are not currently a stable representative image.
- Existing [native detail](../../screenshots/instructor/published_question_detail.png) and
  [WeBWorK HLA detail](../../screenshots/instructor/webwork_hla_genotype.png) captures show that the
  browser renders both paths. Shrinking a long text-heavy Question would provide a recognition cue,
  not a legible substitute for inspection. Keep title, description, exact reference and Open/Inspect.

### Decision for this implementation

Implement shared optional media now, demonstrated by avatar selection. Keep current Question scans
useful without an image. A later supplied image URL can use exactly that same contract. Current
Library metadata Preview is not evidence for an additional rendered-Question mode; consolidate it.

A production Question snapshot needs a separate backend-owned artifact capability. The source
investigation establishes a plausible route: capture an authorized answer-free native or backend
preview for an exact Question revision, choose a stable representative example, and deliver an
image with the same authorization as its Question. RecordList consumes the image and never reads an
opaque frame's DOM or owns rendering credentials. This repository has no reusable product snapshot
producer, delivery contract or lifecycle today; documentation screenshots do not supply those pieces.

If that capability is commissioned, its concrete owner is the Question Backend/Library service,
with a frontend consumer in the existing Question record adapter. Its acceptance must cover a native
Question and a backend-generated Question, useful recognition, exact revision/example identity,
authorization and absence of answers or Student responses. Start with those two real cases before
choosing storage or asynchronous processing. This is a recorded feasibility conclusion, not an
unfinished requirement or blocking investigation hidden inside the current redesign.

## Revised execution consequences

- WP-C1 proves a range of actual record content, including media and bounded editors. The Student
  pilots prove the audited defects; they alone do not freeze the entire family API.
- WP-C6 now follows WP-C1 because Sequence reuses that renderer. WP-C7 separately supplies shared
  movement to six editable callers; five read-only Sequence sites need only WP-C6.
- WP-C8 provides the sort select; WP-C9 provides paging/size controls. Per-caller DATA handoffs replace partial sorting and accumulating discovery; the data-flow audit names their prerequisites.
- WP-C4 validates the complete Question discovery composition. WP-C5 supplies generic image Gallery,
  using the same ordinary media data. Callers wait for just the capabilities they consume.
- Two Detail conversions and navigation remain independent after the execution baseline. Frame work
  retains its actual file handoffs. Every required implementation has an owner and proof in the plan.
- Final cutover removes regions, caller geometry, duplicate Library modes/notices, local movement
  markup, obsolete CSS and staging. The deliverable is a complete shared framework with useful content.
