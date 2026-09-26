# Plan: Shared record and page presentation framework

## Context

The September 25 audit confirms that RecordList shares markup but delegates row design to callers.
Pages supply arbitrary tracks, alignment, visibility priorities, JSX, and overriding CSS. Progress
and Attempt History hide scores on phones; Attempt History rounds adjacent boxes with no gap.
PageFrame standardizes outer geometry but leaves ordinary content spacing to pages. Breadcrumbs
omit useful parents and sometimes link the Course ancestor to the wrong destination.

The user's clarified direction is a strong shared base with enough expressive power for real tasks.
Pages describe what records contain and what they can do; the shared family decides how those parts
are presented. Shared defaults own padding, spacing, typography, responsive behavior, states,
selection, actions and ordering mechanics. The same semantic content flows through every width.
A small API is useful only when it completes the task without pushing legitimate content into local
wrappers. This is a technical redesign and complete caller migration delivered through bounded
packages tracked in the workstream ledger.

The deeper caller investigation is complete. Its
[full caller table and findings](../audits/record_list_caller_investigation_2026-09-25.md)
cover every current RecordList/Sequence caller, adjacent sort controls and semantic siblings. It
supersedes the earlier three-field-only hypothesis: descriptions, linked context, thumbnails,
pressed actions and bounded editor bodies have concrete consumers. Execution status and acceptance
evidence are maintained in the workstream ledger. Source inspection and existing captures establish
design evidence, not acceptance of new rendering.

The additional [data-flow investigation](../audits/record_list_data_flow_investigation_2026-09-25.md)
traced database queries, API limits and browser retention. Its required
[bounded data workstream](../workstreams/record_list_bounded_data_workstream.md) specifies the SQL/API
corrections, page controls and picker reuse. Keep RecordList in Solid/TypeScript; reduce broad
collections before presentation. Execution status and acceptance evidence are maintained in the
workstream ledger.

The copy-ready manager prompt is
[record_list_page_frame_standardization_plan_goal.md](record_list_page_frame_standardization_plan_goal.md).

Primary authorities are [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md),
[FALL_2026_PILOT.md](../../FALL_2026_PILOT.md), and the user's clarification above. Reuse the
operative repository rules receipt only while its sources remain current. Supporting authorities are
[DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md), [REPO_STYLE.md](../../REPO_STYLE.md),
[TYPESCRIPT_STYLE.md](../../TYPESCRIPT_STYLE.md), [MARKDOWN_STYLE.md](../../MARKDOWN_STYLE.md),
and [PLAYWRIGHT_TEST_STYLE.md](../../PLAYWRIGHT_TEST_STYLE.md).

Evidence and coordination inputs:

- [record_list_page_frame_standardization_audit_2026-09-25.md](../audits/record_list_page_frame_standardization_audit_2026-09-25.md)
  contains findings, measurements, source locations, and validation limits.
- [record_list_migration_plan.md](../../archive/record_list_migration_plan.md) records the completed
  semantic-family migration. Preserve its useful family boundaries; its old region API is superseded
  by this plan. Its historical completion record stays intact.
- [fixed_tier_1_to_tier_2_ribbon_contract.md](fixed_tier_1_to_tier_2_ribbon_contract.md) retains
  ownership of fixed Ribbon destinations. This plan owns breadcrumb ancestry, not Tier 2 choices.
- [student_task_surface_plan.md](student_task_surface_plan.md) owns Question-response and Attempt
  surface work. Coordinate shared files; preserve its response controls and terminology boundaries.

## Objectives

- Express every ordinary record through shared content and behavior capabilities that complete its task.
- Bound broad discovery in PostgreSQL/API and keep query-wide sorting correct across pages.
- Make one shared scan treatment reflow the same content naturally from wide to narrow containers.
- Keep identity, Course context, decision status, and actions readable and reachable at narrow widths.
- Give lists and page sections consistent typography, insets, dividers, and meaningful spacing.
- Show truthful, navigable ancestry and actual loaded names in breadcrumbs and workspace context.
- Preserve existing domain behavior, semantics, focus, selection, draft edits, and persistence.
- Finish every caller disposition and remove superseded configuration, styling, and migration code.

## Design philosophy

Apply **Fix the design, not the symptom**, **Ground requirements in actual needs**, and
**Atomic task decomposition** from [REPO_STYLE.md](../../REPO_STYLE.md), with aggressive KISS.
The shared component makes routine presentation decisions once. A content adapter describes domain
records; shared rendering presents them without erasing information needed for the task.

Replace the permissive scan contract while retaining working collection states, identities,
useful windowing, and native family semantics. Keeping raw JSX regions and adding defaults would preserve
the root cause. A universal component that models every form and review would add unnecessary
complexity. The boundary is shared presentation with semantic content, controlled behaviors and a bounded body
for genuine domain controls. API size is a cost to weigh, not a field-count target.

For an uncertain presentation, compare the smallest shared candidates on existing populated data,
long labels, narrow containers, and keyboard tasks. Select the candidate that preserves information
and behavior with fewer public options and less page CSS. Correct the shared component when it fails.
Use existing rendering, controls, state and harnesses first. Add an abstraction, policy, state variable,
or permanent test only for a recorded current requirement or reproduced failure. Natural wrapping and
ordinary source/behavior checks suffice where the product has no stronger invariant. Evidence does
not require byte equivalence, pixel equivalence, arbitrary thresholds, or exhaustive viewport matrices.

Keep configuration simple. Add configuration like options, parameters, modes, overrides, and extension
points only when they solve a demonstrated need. Prefer sensible fixed behavior for internal
implementation decisions. Each additional choice increases code, testing, documentation, maintenance,
and the number of combinations the system must support. When callers need different behavior, first
determine whether the shared design should handle it automatically or whether the tasks are genuinely
different. Let demonstrated needs drive new configuration. When in doubt, use the simpler shared design.

Review existing options in touched presentation code and remove those made unnecessary by the new
shared design. Demonstrated exceptions improve the shared family rather than restore local customization.

## Scope

- Replace the ordinary RecordList region API and migrate every current caller in the companion ledger.
- Centralize record markup, description/media/fact styling, actions, selection, ordering controls and reflow.
- Delete caller-owned phone markup, responsive visibility/priority rules, and alternate action placement.
- Consolidate Library metadata Scan/Preview into one complete scan; retain visual avatar List/Gallery.
- Share image media, bounded editor bodies, loaded collection notices, sort controls and Sequence movement.
- Replace ignored Sequence layout inputs with shared content plus genuine domain editor bodies.
- Reclassify embedded forms that currently masquerade as scans using the destinations below.
- Standardize PageFrame content stacking and shared section boundaries across all current consumers.
- Correct breadcrumb parents, current labels, loaded object names, and Blueprint selection context.
- Complete the bounded data workstream: Question SQL pages/facets, Blueprint global sort, current-page browsing and Assessment picker reuse.
- Update focused behavioral checks, rendered evidence, durable design documentation, and the changelog.

## Non-goals

- Port presentation to Rust/WASM, introduce a client database, or redesign Question Backends/grading.
- Add speculative indexes, caches, workers, arbitrary page-size menus or virtualization to ordinary lists.
- Change fixed Ribbon choices/order, introduce new destinations, or add a persistent Course filter.
- Rebuild the theme, avatar catalog, Question renderer, or entire form system.
- Build a production Question snapshot service as a hidden dependency of record standardization.
  Its feasibility and concrete backend boundary are recorded in the caller investigation; current
  image support works now with catalog media and can display a future authorized snapshot URL.
- Flatten tables, outlines, sequences, expanded reviews, or task forms into ordinary scan rows.
- Add configurable density, user layout preferences, page-named variants, or a generic UI schema engine.
- Introduce phone modes, caller breakpoint APIs, or separate wide/narrow record content models.
- Publish a release, deploy, push, or commit as part of this handoff; repository rules reserve commits
  for the human.

## Current state summary

The completed September 25 caller investigation re-counted 37 RecordList and 11 RecordSequence sites
across 38 files, with every consumer represented in the ledger. It inspected those callers and their
content/action helpers, plus the existing semantic siblings. The current tree also has two Table sites,
five Outline sites, nine Detail sites, and 62 PageFrame sites across 45 files. These are scope evidence,
not required final counts. The earlier audit recorded 95 literal region configurations, 28 width
strings and 24 PageFrame `contentClass` uses.

The deeper investigation found descriptions in Question/Pool/Draft selection, a meaningful Course
link in Due Soon, pressed Template commands, avatar image content, inline Course editing and six
editable Sequence callers. Five other Sequence sites are read-only ordered records. It also found
repeated loaded-result notices and sort controls outside RecordList. These are shared capabilities,
not reasons to flatten content or maintain page-owned presentation.

Question Library's existing Preview is a second metadata layout, not a rendered Question preview.
One scan can preserve its description, authors, classification, exact reference and copy action,
including the retired-Discipline notice currently absent from Preview. Avatar Gallery/List is a
real visual-browsing choice over the same image/name/description and selection. See the audit table
for each caller's content and disposition; the ledger adds exact owners and readiness.

Targeted Graphify was refreshed against the September 25 08:13 CDT map (19,199 nodes), then checked
against current source. The earlier map was stale. Source remains authoritative for caller coverage.
Existing native and WeBWorK Question-detail PNGs were inspected for snapshot feasibility; they are
historical rendering evidence. No production code or product tests changed during this investigation.

Measured defects include hidden status at 393 px and touching 8 px rounded Attempt History rows.
A deliberately narrowed 400 px Library container at a 1280 px viewport had 628 px of scrollable
content; ordinary full-width Library fixtures fit. Student fixtures omitted shell padding. Use
actual-shell captures before judging outer page gutters.

The current worktree contains the audit, its changelog entry, and unrelated `gui-buffer.txt`.
Preserve unrelated work. No service restart or database rebuild is implied by this frontend plan.

## Architecture boundaries and ownership

### Performance ownership

The current Library requests 50 records, with an API ceiling of 100. Broad discovery offers the
user-requested shared choice of 50 (default), 100 or 250 per page; keep that choice in query/return state,
restart at page one on change, and preserve selection. Small task lists need no size control.
Its server currently loads/resolves the catalog before paging, and the browser accumulates pages.
My Blueprint sorts only loaded results; Assessment available Questions return the entire catalog.
WP-P0-WP-P5 and WP-C9 correct these demonstrated boundaries as specified in the bounded data workstream.
PostgreSQL filters/orders broad queries and computes full-query facets; API transfers a bounded page.
Pages own query/cursor/selection state; shared controls own presentation. Sort or page-size changes restart the
query, and Previous/Next replaces the current page while preserving independent task selection.
Keep small complete task-local arrays and saved sequences local. WP-P6 compares plain bounded-page
rendering with existing Library windowing and retains only useful machinery. Future measured client
computation can justify a separate data-processing optimization; the current design adds no WASM.

### Shared record content and behavior

Keep the existing collection inputs `rows`, `recordId`, `state`, `ariaLabel` and `emptyState`.
Replace `regions` with `content(row)` returning semantic content. Properties are readonly; domain
records and callbacks retain their actual types. Optional content is omitted when absent.

| Content | Shared treatment | Demonstrated need |
| --- | --- | --- |
| `title: string` | One clear identity with consistent typography | Every ordinary record |
| `description?: string` | Supporting prose separated from short facts; wraps naturally | Question browse/picker, Pool picker/discovery, Drafts and avatar descriptions |
| `details: readonly RecordFact[]` | Consistent short facts and references in semantic order | Scores, lifecycle, Course context, dates, counts, exact revisions and classifications |
| `media?: { src: string; alt: string }` | Optional image; shared size, fit, placement, inset and failure treatment | Avatar selection now; any future supplied Question snapshot uses the same image contract |
| `actions: readonly RecordAction[]` | Native links/commands with shared emphasis, wrapping and record context | Open, Inspect, Edit, Accept/Decline, Add/Remove, Compare and Delete |

`RecordFact` supports plain text and three demonstrated semantic forms: a labeled link, an Assessment
Type enum, and an exact Question ID. Link facts preserve Due Soon's Course destination; they do not
turn it into an unrelated command. Assessment Type uses existing `assessmentTypePresentation` and
`RibbonIcon` for the required icon, label and theme color. Question ID uses the existing
`CopyableQuestionId` control where the record exposes its ID. One shared reference treatment replaces
the old copy-only-in-Preview rule. Other identities and score states stay explicit text. Derive types
from existing domain contracts; ordinary content is neither HTML nor a general UI schema.

Actions have stable IDs within their record. Links supply label, href and an optional native follow
callback. Commands supply label, native button callback and existing disabled state. Template editor
selection supplies pressed state; Known Forks supplies expanded state and the existing comparison
region ID. A primary-action indication preserves the actual task emphasis, such as Accept beside
Decline and Open beside Delete; shared code fixes the corresponding styles. These semantic fields
replace neither the native control nor its accessible behavior.

Allow a narrow native element ref on an action for the existing Course editor's focus return. Its
Edit button disappears during editing and is recreated on close; the original click target alone
cannot restore focus. Link refs also support its read-only fallback. Shared code owns control markup;
this ref serves existing focus behavior rather than row styling. Keep useful distinct destinations and
commands; collapse a duplicated title/Open destination into one action. Preserve disabled explanations
and confirmation flows. An ordered array handles zero, one or several actions without per-page
placement or priority settings.

Assessment/Coursework callers with an existing Type value use the shared Type fact. Attempt History
and recovery keep their existing Attempt projections and Course context; those records omit Type.
This presentation work adds no guessed Type, extra request or backend payload change. Scores and
"Score not released" remain directly visible. Description, reference and media support represent
real content, not opportunities for caller width, color, truncation or breakpoint configuration.

WP-C1 implements the common content, including media and references. Its existing harness uses real
avatar catalog data, a descriptive Question result with copyable ID, a linked Course fact, paired
commands, a pressed Template command and an expanded Fork command alongside Student score rows. These are bounded examples of
supported composition, not a combinatorial matrix. WP-C2P/WP-C2H prove the two audited page defects;
they are not the only evidence for contract completeness. Accept interfaces per capability as its
proof passes; later work may improve a shared boundary when a demonstrated task exposes a gap.

### Bounded rich content within the shared frame

Add one optional `renderBody` boundary to RecordList and RecordSequence for an actual domain
editor or rich reading body. Invoke it in the mounted record owner with a reactive accessor for the
current row, so its native inputs can retain their owner while server data changes. The shared component places and spaces it below the standard header,
media, facts and actions. The caller returns only the domain body; shared code continues to own the
record surface, title, action area, padding, dividers and responsive arrangement.

Course Instance is the immediate scan consumer: its existing conditional title/due-date editor and
conflict messages remain associated with the relevant Assessment, inside the shared record body.
Keep its existing row model, dirty guards and controlled visibility. Sequence bodies contain real
points/count inputs, nested Pool editors and fork destination controls. This avoids both a form-schema
engine and the earlier proposal to let every Sequence render its whole row independently. An existing
Question renderer can occupy the same bounded body when a real inspection task supplies its data;
RecordList itself performs no Question fetch or backend rendering. Current full reviews stay Detail.

Use shared form/control primitives inside bodies. Record identity, facts and record commands use
`content(row)`; form-local Save/Cancel and destination controls stay with their inputs. Each body's
existing caller receipt names its domain task and confirms shared ownership of the surrounding
presentation. CORE resolves any duplicated header, spacing or reflow through the shared boundary.
Extract a shared editor capability when repeated domain content demonstrates the need.

### Media, selection and presentation

Media is ordinary optional content. The adapter supplies an available image URL and meaningful alt
text, or an empty alt when nearby name/description already describes it. Shared rendering uses a
consistent thumbnail frame, contains the whole supplied image, and owns loading/error fallback.
A failed image leaves text, selection and actions usable. Rows without images need no fake image.
Size, crop, radius, fit and breakpoint settings remain fixed shared implementation choices.

WP-C3 adds controlled radio/checkbox selection: pages supply selected IDs, disabled decisions and
change callbacks. The family owns native grouping, labels, focus treatment and placement; it keeps
links and commands outside the selection label. Template pressed commands remain commands.

WP-C4 proves the complete Question-discovery composition over the shared content and selection.
One normal scan replaces Library Scan/Preview, retaining summary, authors, classification, retired
status, available format, exact ID/copy and Open. Shared copy behavior also serves IDs explicitly
shown in Question picker rows. Keep exact revisions, modified-click navigation, return tokens and
focused-row retention. Remove the old presentation switcher and its height-cache branch. WP-P6 decides whether the bounded
page still benefits from ordinary window measurements or can remove that machinery entirely. Actual Question rendering remains in the
existing detail/inspection task, clearly distinguished from metadata discovery.

WP-C5 adds the shared List/Gallery choice for image browsing, using the same content and selection.
The avatar caller supplies catalog assets/name/description and controlled selection once. Shared
code owns the switcher, thumbnail/tile sizing and gaps. Gallery is user-selected for visual browsing;
width changes only reflow it. A new catalog does not need a new row renderer or page-named mode.

Question snapshot generation is not part of RecordList. The investigation traces native answer-free
prompt/response rendering and authorized opaque WeBWorK preview documents; no browse thumbnail field
or reusable snapshot service exists. WeBWorK preview seeds are currently random. A future backend-owned
snapshot must identify an exact revision and representative example, preserve authorization and show
answer-free content. The shared media input already accepts its eventual URL; missing snapshots leave
a useful text scan and existing Open/Inspect path. This plan delivers media now without inventing a
new render farm, cache policy or browser screenshot bridge.

### Collection states, sorting and saved order

The shared state view owns initial loading/empty/error and nonblocking loading/error notices when
records are already loaded. Use existing rows/isEmpty plus loading/error state to choose the treatment;
retain the records, selection and focus while displaying the shared notice and Retry action. Library
currently maps these states to ready and builds extra notices itself. WP-C1 repairs the shared state
behavior; WP-L1 removes those duplicate wrappers. While Library uses windowing, compose the existing
shared state view around the scrolling region using the total loaded set, and render the mounted
RecordList slice as ready content inside it. Shared notices then sit outside spacer calculations and
remain reachable. This is shared component composition, not a second page-owned state renderer.
Fetching, pagination and retry callbacks remain local. A query/access owner still clears stale rows
when their scope changes; the presentation layer does not decide which records remain authorized.

Sorting and saved order are different behaviors with shared controls:

- WP-C8 implements `RecordSortControl`, a labeled native select with typed options, current value,
  disabled state and change callback. Library and My Blueprint Courses already supply actual choices.
  Their query choices, pagination reset and return-state policy remain local; PostgreSQL orders the full match. The control composes
  beside existing filters; a toolbar schema or client sort of one server page is unnecessary.
- WP-C6 gives RecordSequence the same `content(row)` and bounded `renderBody` under native
  `ol/li`. Read-only sequences use just the content. It depends on WP-C1's shared renderer.
- WP-C7 integrates existing reorder mechanics into Sequence: the caller supplies a controlled move
  callback and its existing disabled policy; shared code derives position/boundaries, renders move
  and drag controls, announces completed moves and restores focus by record ID. Preserve page-owned
  immediate versus deferred saves, conflicts and error handling. Announce movement after the supplied
  order actually changes, including the Assessment Pool's asynchronous replacement. A failed request
  keeps the existing order and message. Reuse the controls for the co-located fork Module outline;
  its destination selection and cross-Module transfer remain domain controls in the body. Assessment
  "Sort by Bloom Classification" remains a domain command over the local draft using shared action
  styling; it is not a browse-sort preference.

Keep dependencies pointing toward the shared layer. The existing `record_list_reorder.tsx` imports
its array movement from `blueprint_fork_apply_model.ts`. WP-C7 moves that small pure operation into
the shared family; BLUEPRINT consumes it when its caller migrates. CORE leases the feature-model
extraction and affected existing helper checks before releasing that file. WP-X1 removes any short
cutover re-export. Domain validation remains in the Blueprint model. Student Ordering response
controls stay with the Question runtime; they are not another collection migration.

This changes the earlier dependency claim: Sequence shares the standard and therefore follows WP-C1.
Its five read-only consumers need WP-C6; six editable consumers additionally need WP-C7. Independent
Detail and navigation work still starts after WP-B1. Sort controls depend only on WP-C1 and release
their two consumers; My Blueprint also needs WP-P4 for correct query-wide ordering.

### Shared rendering and semantic siblings

Make `recordId` the actual mounted identity. Current List/Sequence use Solid's `<For>` on row objects;
Course Instance and several Sequence callers reconstruct those objects. Merely writing a
`data-record-id` attribute does not preserve their controls when refreshed objects arrive. WP-C1
iterates stable IDs and supplies the current row through reactive access; WP-C6 reuses that boundary.
Keep this mapping local to the mounted collection, with no separate domain store. Recompute content
reactively, key action controls by their stable IDs and preserve a mounted domain-body owner while
its record remains. Normal body closing, record removal and window unmounting still dispose content.
WP-C1's existing browser harness replaces row objects with fresh objects carrying the same IDs:
metadata/action state updates while a focused body input retains its unsaved value and focus.
The Course Instance migration repeats this with its real editor and Save/Cancel/conflict behavior.
Assert useful behavior; pages continue to own unsaved values and failures.

Native callbacks carry the actual anchor/button event. Library retains unmodified-click return tokens;
Inspect/confirmation commands capture `event.currentTarget` for focus return. Shared rows retain
`data-record-id` and their measurement boundary. Windowing owns mounted range and spacers, not content
or presentation. Preserve hrefs, modified clicks, stable identities and reactive updates.

The same content and task state flow through every width. Shared code owns wrapping, stacking,
retention and action placement. Remove caller phone summaries, visibility priorities, row classes and
track widths. Use one compact scan with an inset, one divider, normal supporting text and a clear title.
Images and rounded controls sit inside that inset. Gallery objects receive a shared gap; expanded
bodies receive shared separation without making ordinary rows into padded islands. Use intrinsic
layout, with a small private container rule only when the rendered task needs it.

| Existing case | Required shared treatment |
| --- | --- |
| Fixed Question point editor | Detail form entries; editing every points field is the main task, with aggregate Save/Cancel and conflict recovery |
| Instructor Accounts | Detail lifecycle forms; preserve Account-ID-only identity, reason validation and busy state |
| Course Instance inline editing | Standard scan content/actions plus bounded existing editor body, attached to the relevant Assessment |
| Recovery and Question/Pool pickers | Same record content with controlled native selection; evidence/full inspection stays in the existing review surface |
| Ordered Questions/Pool members | Sequence shares content, actions and reorder; actual nested editors use the bounded body |
| Gradebook and roster | Table headers, row identity and internal scrolling remain meaningful |
| Blueprint hierarchy and expanded reviews | Outline/Detail retain their real hierarchy, comparisons and domain renderers |

Family choice follows the user's task. A rich description, image, copyable reference or extra action
fits the ordinary standard. A form, table, hierarchy or full review retains the sibling that expresses
its semantics. Domain state, permissions, persistence and backend rendering keep their existing owners.

### Frame and breadcrumb boundaries

PageFrame owns its root, heading/actions, route-selected width, content origin, and ordinary section
spacing. Introduce a shared PageSection with heading, optional helper/actions, and body. Review each
`contentClass` usage by responsibility. Migrate ordinary spacing into the shared primitives. An editor
may retain a documented inner-layout hook when it controls only task-specific arrangement inside the
standard frame and sections. The manager chooses whether to retain, narrow, relocate, or remove the
prop from those actual uses; deleting one named prop is not the acceptance condition. The shared
frame retains exclusive ownership of outer geometry and ordinary spacing in every case.

Breadcrumb hierarchy stays in the existing central resolver. Correct its concrete route cases and
factor repeated ancestor construction only where it simplifies those cases. Reuse route links and
parameter parsers. Extract a focused module if responsibility or the source-file limit warrants it;
keep one authority and ordinary functions rather than introducing another route graph or registry.

Required trails use these real destinations:

- Student Course: Home > Courses > Course name. Progress adds > Progress; Assessment content descends
  from the Course landing as the Course parent.
- Student Grades detail: Home > Grades > Attempt History or Response Stats. Grades links to the
  existing `/student/grades` landing; its current page label remains Scores.
- Instructor Assessment sections: Home > Course name > Assessment title > section; preserve existing
  valid paths. Home already represents My Active Courses, so avoid duplicate same-destination levels.
- Blueprint: Home > appropriate collection > loaded Blueprint name. Use the already-loaded
  `BlueprintCourseView.read_access`: `blueprint_course_owner` selects My Blueprint Courses;
  `active_instructor` selects public search. Direct loads follow the same rule; browser visit history
  is not ancestry. Reconcile any future relationship enum change with the actual authorized projection.
- Question and Draft: Home > their existing collection > actual loaded title. Keep filtered-search
  return behavior separate from canonical parent relationships.

Publish display names from existing authorized loads using the route-scoped mechanism already used
for Assessment titles. WP-N1 adapts the existing publisher in `assessment_workspace_live_page.tsx`
to the corrected publication boundary and releases that file. WP-N2 owns `question_detail_page.tsx`,
`question_draft_editor_page.tsx`, `question_json_editor_page.tsx` and Blueprint detail. The JSON editor
publishes its existing saved source title after load/save/reload; local unsaved title edits retain their
existing draft semantics. Its general-feedback-only response contains no title, so that branch uses
the truthful generic Draft Question label and complete collection ancestry. This uses available data
without adding a metadata request or changing a server contract. Extend the pure model with strings and a closed parent choice, not resources,
callbacks, raw response objects, or new fetching. Guard publication against stale route generations;
clear names on identity/session changes, failure, and disposal. Preserve generic loading labels while
data resolves and prevent old names appearing after rapid navigation or access denial.

Blueprint Module/Assessment selection currently lives inside one route. Present a workspace trail
to the existing workspace showing the Module and selected Assessment names, with real return-to-outline
buttons and appropriate focus restoration. Preserve unsaved edits. These local buttons act on existing
selection state; global breadcrumb links point to actual pages and show the loaded Blueprint name.
FRAME later places this trail inside the common section rhythm. The combined visible context must
identify every actual level of the task.
Reuse existing navigation/section primitives for that presentation. Introduce a small shared primitive
only if the existing ones cannot express this concrete local-return task; retain the existing selection
state and return handler.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component | Review boundary |
| --- | --- | --- |
| M1 / MANAGER | Execution baseline and leases from the completed investigation | One manager reconciles source drift and opens dispatch |
| M2-M4 / CORE, PROGRESS, HISTORY | Shared content renderer and separate real-page pilots | Base component followed by two bounded page proofs |
| M5-M10 / CORE | Selection, discovery, Gallery, Sequence, reorder and sort packages | Concrete task contracts stay with CORE; each capability has its actual prerequisite |
| M11-M16 / five caller lanes, then CORE | Exact ledger files and final cleanup | Per-file dispatch followed by one complete public API cutover |
| M17-M18 / FRAME | Frame primitives and bounded consumer tasks | Shared ordinary layout; documented inner editor arrangements |
| M19-M21 / NAVIGATION | Ancestry, safe name publishers, then local Blueprint context | Starts after M1 with disjoint file leases |
| M25-M32 / DATA, CORE, INSTRUCTOR, INTEGRATOR | Bounded query/page work in the linked workstream | Disjoint data work starts after M1; WP-P6 precedes final acceptance |
| M22-M24 / INTEGRATOR, then MANAGER | Integrated checks, agent evidence review, then closure | One owner for each aggregate run, capture and documentation operation |

Use [record_list_page_frame_standardization_ledger.md](../workstreams/record_list_page_frame_standardization_ledger.md)
as the whole-file assignment and status tracker. Workers share the codebase with other owners: preserve
their edits, keep each dispatch within its file lease, and send shared-contract pressure to CORE. MANAGER owns the ledger/changelog and serializes shared edits.

### Autonomous completion

The executing manager and subagents complete all in-scope design decisions, implementation, review,
verification and documentation. Their decision procedure and acceptance evidence replace human
checkpoints. Agent reviewers inspect captures and assess code; populated fixtures and synthetic
transitions exercise selection, dirty-state returns, authorization states and deferred loading.

Use current local runtime and seeded fixtures through repository entry points. Reuse existing harnesses
for isolated behavior; add only a bounded fixture for a demonstrated missing case. Choose implementation
details with the KISS/evidence procedure and record the outcome. Continue independent work while a
package is corrected. A genuine environment or permission failure is recorded accurately, never
converted into a product-approval question or a false pass.

Completion is the verified local implementation and reconciled documentation. Human visual approval,
manual clicking, credential entry, committing, publishing and deployment are outside its critical path.
Repository-required archive handling follows available permissions; an administrative Git restriction
is reported separately from completed implementation. No milestone waits for a human to wake up.

## Milestone plan

| M | Title | Summary | Goal |
| --- | --- | --- | --- |
| M1 | Open execution from investigated scope | MANAGER: WP-B1 | Current baseline and leases use the completed caller dispositions and contract |
| M2 | Implement the flat scan | CORE: WP-C1 | Existing harness proves descriptive/media records, action states, retained results and one shared reflow |
| M3 | Prove Course Progress | PROGRESS: WP-C2P | Populated released and withheld scores remain visible wide and narrow |
| M4 | Prove Attempt History | HISTORY: WP-C2H | Populated score states, Course grouping, pagination and shared boundaries work |
| M5 | Support demonstrated selection | CORE: WP-C3 | Controlled native radio/checkbox selection uses the shared scan presentation |
| M6 | Resolve Library presentation | CORE: WP-C4 | One complete Question scan retains descriptive information, copy, selection and navigation |
| M7 | Resolve avatar presentation | CORE: WP-C5 | One avatar adapter works in List/Gallery with image fallback and native radio behavior |
| M8 | Share Sequence content | CORE: WP-C6 | Ordered content and bounded editor bodies use the same shared presentation |
| M9 | Share reorder controls | CORE: WP-C7 | Six editable Sequence callers use shared movement, boundaries, announcements and focus |
| M10 | Share result sort controls | CORE: WP-C8 | Library and Blueprint sorting use one control while preserving query/order policy |
| M11 | Migrate Student callers | STUDENT: WP-S1 | Each file has its own completed task and Course/score/Attempt behavior receipt |
| M12 | Migrate Instructor callers | INSTRUCTOR: WP-I1 | Per-file evidence preserves revision identities, editing and lifecycle behavior |
| M13 | Migrate Library callers | LIBRARY: WP-L1 | Per-file evidence preserves browse/return, copy, selection, focus and bounded paging |
| M14 | Migrate Blueprint callers | BLUEPRINT: WP-BP1 | Per-file evidence preserves history, chronology, order, forks and drafts |
| M15 | Migrate the avatar picker | AVATAR: WP-A1 | Existing avatar browser check and selected/disabled radio behavior pass |
| M16 | Complete the API cutover | CORE: WP-X1 | Whole-tree compiler/fast UI pass and every collection disposition is reconciled |
| M17 | Establish frame ownership | FRAME: WP-F1 | Frame owns geometry/ordinary spacing; a genuine inner editor still functions |
| M18 | Complete frame consumer review | FRAME: WP-F2 | Every site uses shared ordinary spacing and each retained hook has a task-specific reason |
| M19 | Correct central ancestry | NAVIGATION: WP-N1 | Real parent routes, loading names, session clearing and stale-result rejection work |
| M20 | Publish actual object names | NAVIGATION: WP-N2 | Direct loads, actual parent clicks, denied routes and rapid navigation preserve truthful names |
| M21 | Complete Blueprint local context | NAVIGATION: WP-N3 | Captured synthetic edits and return transitions preserve draft values and restore focus |
| M22 | Verify the integrated tree | INTEGRATOR: WP-V1 | Receipts identify the final tree and all in-scope behavior checks pass |
| M23 | Review rendered and code evidence | INTEGRATOR: WP-V2 | Agent reviewers resolve findings through owners and confirm final-tree evidence |
| M24 | Close implementation records | MANAGER: WP-V3 | All required tasks complete; probes/staging removed; local deliverables ready independently of human review |
| M25 | Bound discovery at 250 | DISCOVERY-CONTRACTS: WP-P0 | 50/100/250 works through API/SQL/decoders while ordinary limits stay scoped |
| M26 | Query bounded Question pages | QUESTION-DATA: WP-P1 | SQL applies predicates, global order and continuation before transfer |
| M27 | Aggregate full-query facets | QUESTION-DATA: WP-P2 | Bounded facet groups cover the authorized match independently of page position |
| M28 | Bound Question source resolution | QUESTION-DATA: WP-P3 | Existing response shape resolves only returned native Questions |
| M29 | Correct Blueprint global sorting | BLUEPRINT-DATA: WP-P4 | Existing name/adoptions/students choices order the full query in SQL |
| M30 | Share paging controls | CORE: WP-C9 | Shared Previous/Next and 50/100/250 size choice serve broad discovery |
| M31 | Reuse bounded Assessment pickers | INSTRUCTOR: WP-P5 | Later-page Questions/Pools are reachable; catalog-dump API is retired |
| M32 | Prove bounded work and simplify rendering | INTEGRATOR: WP-P6 | Data/return/selection behavior and useful rendering machinery are evidenced |

M25-M32's owners, exact scope, dependencies, done checks, validation and parallel readiness are in the
[bounded data workstream](../workstreams/record_list_bounded_data_workstream.md#packages).
These eight additional milestones are required before closure; WP-V1 now follows WP-P6.

Milestone IDs are labels, not a serial schedule. Navigation starts after M1, alongside component work.
For a single-package milestone, entry means its listed prerequisites are accepted and its file lease
is available; exit means its deliverable is present and its stated verification passes. For the caller
and frame lanes, dispatch each child as soon as its own ledger prerequisites and lease are ready. The
parent milestone closes when every child has passed its own verification. Its later children can wait
for a capability while ready children deliver independently.

The base proof means both WP-C2P and WP-C2H are accepted. Ordinary callers can then migrate while CORE
finishes selection/Gallery capabilities. Existing Detail conversions can start after WP-B1; read-only
Sequence callers follow WP-C6 and editable ones WP-C7. Sort consumers also need WP-C8. Mixed files
wait for each capability they actually use.
Each capability releases its own consumers. CORE serializes shared type,
renderer, CSS and harness changes; caller owners work against the accepted interfaces they use.
The presentation maximum is seven owners: CORE, NAVIGATION and five disjoint caller/pilot owners.
QUESTION-DATA and BLUEPRINT-DATA can add two independent owners, for an overall maximum of nine. Before
base proof those are the two pilots and ready Instructor/Library/Blueprint tasks; afterward they
are the five caller lanes. The ledger, readiness and shared-resource leases determine actual dispatch.
During the frame sweep, three frame lanes, NAVIGATION and WP-P6 can proceed, for a maximum of five.
One runtime owner serializes database, generation and capture operations regardless of source concurrency.
MANAGER dispatches only ready work and keeps shared edits and shared-harness runs under one lease.

Concrete dispatch order: WP-B1 opens CORE, NAVIGATION, DISCOVERY-CONTRACTS and the existing Detail conversions; WP-P0 releases both DATA owners. CORE
implements WP-C1; both Student pilots can then run while CORE builds Sequence and sorting under its
exclusive family/harness lease. Read-only Sequence consumers follow WP-C6; movable ones follow WP-C7.
The base proof releases ordinary scans. WP-C3 releases selectable consumers; WP-C4 proves Question
discovery and WP-C5 releases Gallery. Only the two sort-control consumers wait for WP-C8. The ledger's
per-file prerequisites define dispatch. WP-C9 releases paging consumers; only Question discovery waits
for WP-P3, and only My Blueprint waits for WP-P4. WP-P5 follows the reusable picker handoffs.

### Milestone: open execution from investigated scope

- ID: M1; package: WP-B1; owner: MANAGER.
- Depends on: none.
- Deliverable: Fresh execution baseline receipts and file leases using the completed field inventory
  and per-child prerequisites; reconcile any source changes since the investigation.
- Done check / exit: Every ready task has its actual current owner/input boundary and baseline failures
  are distinguished from implementation results.
- Parallel-plan ready: no. One manager establishes authority and dispatch boundaries.

### Milestone: implement the flat scan

- ID: M2; package: WP-C1; owner: CORE.
- Depends on: WP-B1.
- Deliverable: Typed content, media, reference, action/body rendering and retained-result collection states using existing primitives.
- Done check / exit: Existing browser harness proves shared content/reflow, retained results and
  same-ID refresh with reactive metadata/action state, retained input focus and a live unsaved draft.
- Parallel-plan ready: yes. CORE, NAVIGATION and the two Detail conversions, maximum four owners with disjoint files.

### Milestone: prove Course Progress

- ID: M3; package: WP-C2P; owner: PROGRESS.
- Depends on: WP-C1.
- Deliverable: Progress adapter and removal of its row/phone overrides.
- Done check / exit: Populated released and withheld scores remain visible wide and narrow.
- Parallel-plan ready: yes. PROGRESS, HISTORY, CORE, NAVIGATION and ready Detail/Sequence callers, maximum seven owners; accepted interfaces stay stable.

### Milestone: prove Attempt History

- ID: M4; package: WP-C2H; owner: HISTORY.
- Depends on: WP-C1.
- Deliverable: History adapter and removal of touching rounded row boxes.
- Done check / exit: Populated score states, Course grouping, pagination and shared boundaries work.
- Parallel-plan ready: yes. HISTORY, PROGRESS, CORE, NAVIGATION and ready Detail/Sequence callers, maximum seven owners; accepted interfaces stay stable.

### Milestone: support demonstrated selection

- ID: M5; package: WP-C3; owner: CORE.
- Depends on: WP-C2P, WP-C2H.
- Deliverable: Controlled native radio/checkbox selection over the shared scan.
- Done check / exit: Exact selected identities, disabled state and native keyboard behavior work through shared rendering.
- Parallel-plan ready: yes. CORE, ready caller lanes and NAVIGATION, maximum seven owners; CORE serializes shared API/harness edits.

### Milestone: resolve Library presentation

- ID: M6; package: WP-C4; owner: CORE.
- Depends on: WP-C3.
- Deliverable: Question discovery composition over shared descriptions, references, selection and loaded states.
- Done check / exit: One complete Question scan retains descriptive information, copy, selection and navigation.
- Parallel-plan ready: yes. CORE, ready caller lanes and NAVIGATION, maximum seven owners; CORE serializes shared capability edits.

### Milestone: resolve avatar presentation

- ID: M7; package: WP-C5; owner: CORE.
- Depends on: WP-C3.
- Deliverable: Shared Gallery/List composition over the ordinary media/content and selection model.
- Done check / exit: One avatar adapter works in List/Gallery with image fallback and native radio behavior.
- Parallel-plan ready: yes. CORE, ready caller lanes and NAVIGATION, maximum seven owners; CORE serializes shared capability edits.

### Milestone: share Sequence content

- ID: M8; package: WP-C6; owner: CORE.
- Depends on: WP-C1; Sequence reuses the shared content and body renderer.
- Deliverable: Sequence uses shared content and a bounded domain body under native ordered items.
- Done check / exit: Ordered content and bounded editor bodies use the same shared presentation.
- Parallel-plan ready: yes. CORE, ready caller lanes and NAVIGATION, maximum seven owners; shared harness ownership remains serial.

### Milestone: share reorder controls

- ID: M9; package: WP-C7; owner: CORE.
- Depends on: WP-C6.
- Deliverable: Sequence-controlled movement and reusable fork Outline controls, using existing mechanics.
- Done check / exit: Local and asynchronous moves preserve exact order, disabled policy and focus; failed saves retain the existing order and page error.
- Parallel-plan ready: yes. CORE, ready caller lanes and NAVIGATION, maximum seven owners; one family/harness lease.

### Milestone: share result sort controls

- ID: M10; package: WP-C8; owner: CORE.
- Depends on: WP-C1.
- Deliverable: Shared typed native sort select for Library and My Blueprint Courses.
- Done check / exit: Existing choices, disabled state and callbacks work; query choice stays with each caller and server ordering remains authoritative.
- Parallel-plan ready: yes. CORE, ready caller lanes and NAVIGATION, maximum seven owners; one family/harness lease.

### Milestone: migrate Student callers

- ID: M11; package: WP-S1; owner: STUDENT.
- Depends on: WP-C2P, WP-C2H for ordinary rows; WP-C3 only for recovery selection.
- Deliverable: Individually dispatched STUDENT ledger rows.
- Done check / exit: Each file has its own completed task and Course/score/Attempt behavior receipt.
- Parallel-plan ready: yes. Ready migration lanes, CORE and NAVIGATION, maximum seven owners with disjoint file leases.

### Milestone: migrate Instructor callers

- ID: M12; package: WP-I1; owner: INSTRUCTOR.
- Depends on: WP-B1 for existing Detail forms; WP-C2P, WP-C2H for scans; WP-C6 for read-only Sequence; WP-C7 for editable Sequence; WP-P5 completes the Questions workspace page/view.
- Deliverable: Individually dispatched INSTRUCTOR rows, including genuine form dispositions.
- Done check / exit: Per-file evidence preserves revision identities, editing and lifecycle behavior.
- Parallel-plan ready: yes. Ready migration lanes, CORE and NAVIGATION, maximum seven owners with disjoint file leases.

### Milestone: migrate Library callers

- ID: M13; package: WP-L1; owner: LIBRARY.
- Depends on: WP-C2P, WP-C2H for scans; WP-C6 for read-only Sequence; WP-C4 for browse; WP-C3/WP-C7 for the picker; WP-C8 for the adjacent sort owner; WP-C9 and WP-P3 only for named data consumers.
- Deliverable: Individually dispatched LIBRARY rows.
- Done check / exit: Per-file evidence preserves browse/return, copy, selection, focus and bounded paging.
- Parallel-plan ready: yes. Ready migration lanes, CORE and NAVIGATION, maximum seven owners with disjoint file leases.

### Milestone: migrate Blueprint callers

- ID: M14; package: WP-BP1; owner: BLUEPRINT.
- Depends on: WP-C2P, WP-C2H for scans; WP-C6 for read-only Sequence; WP-C7 for editable Sequence; base plus WP-C6 for history; WP-C3/WP-C6 for the Pool picker; WP-C8/WP-P4 for the Course list; WP-C9 only for paged discovery.
- Deliverable: Individually dispatched BLUEPRINT rows.
- Done check / exit: Per-file evidence preserves history, chronology, order, forks and drafts.
- Parallel-plan ready: yes. Ready migration lanes, CORE and NAVIGATION, maximum seven owners with disjoint file leases.

### Milestone: migrate the avatar picker

- ID: M15; package: WP-A1; owner: AVATAR.
- Depends on: WP-C5; its prerequisites already include selection and the base proof.
- Deliverable: Picker consumes shared presentation; local gallery geometry removed.
- Done check / exit: Existing avatar browser check and selected/disabled radio behavior pass.
- Parallel-plan ready: yes. Ready migration lanes, CORE and NAVIGATION, maximum seven owners with disjoint file leases.

### Milestone: complete the API cutover

- ID: M16; package: WP-X1; owner: CORE.
- Depends on: WP-S1, WP-I1, WP-L1, WP-BP1, WP-A1.
- Deliverable: Single strict public entry; old region API, row overrides and staging deleted.
- Done check / exit: Whole-tree compiler/fast UI pass and every collection disposition is reconciled.
- Parallel-plan ready: no. One owner integrates shared imports, API and stylesheet cleanup after caller convergence.

### Milestone: establish frame ownership

- ID: M17; package: WP-F1; owner: FRAME.
- Depends on: WP-X1.
- Deliverable: Shared stack/sections with representative ordinary, editor and failure consumers.
- Done check / exit: Frame owns geometry/ordinary spacing; a genuine inner editor still functions.
- Parallel-plan ready: yes. FRAME and NAVIGATION, maximum two owners; choose pilots outside navigation leases.

### Milestone: complete frame consumer review

- ID: M18; package: WP-F2; owner: FRAME.
- Depends on: WP-F1 for every child; WP-N1, WP-N2 or WP-N3 only for the overlapping files named in the ledger.
- Deliverable: Individually dispatched frame ledger tasks, including inner-hook dispositions.
- Done check / exit: Every site uses shared ordinary spacing and each retained hook has a task-specific reason.
- Parallel-plan ready: yes. FR-STUDENT, FR-WORKSPACE, FR-SHELL and NAVIGATION, maximum four owners with per-file handoffs.

### Milestone: correct central ancestry

- ID: M19; package: WP-N1; owner: NAVIGATION.
- Depends on: WP-B1.
- Deliverable: Existing breadcrumb resolver and safe publication boundary corrected.
- Done check / exit: Real parent routes, loading names, session clearing and stale-result rejection work.
- Parallel-plan ready: yes. NAVIGATION starts alongside CORE after M1 and later ready caller lanes, maximum seven total owners.

### Milestone: publish actual object names

- ID: M20; package: WP-N2; owner: NAVIGATION.
- Depends on: WP-N1.
- Deliverable: Question/Draft/Blueprint title publishers and loaded Blueprint parent choice.
- Done check / exit: Direct loads, actual parent clicks, denied routes and rapid navigation preserve truthful names.
- Parallel-plan ready: yes. NAVIGATION and ready record/frame tasks, within the seven-owner record or four-owner frame maximum.

### Milestone: complete Blueprint local context

- ID: M21; package: WP-N3; owner: NAVIGATION.
- Depends on: WP-N2.
- Deliverable: Module/Assessment context uses existing selection and real return behavior.
- Done check / exit: Captured synthetic edits and return transitions preserve draft values and restore focus.
- Parallel-plan ready: yes. NAVIGATION and ready record/frame tasks, within the same seven/four-owner limits; only Blueprint detail awaits this handoff.

### Milestone: verify the integrated tree

- ID: M22; package: WP-V1; owner: INTEGRATOR.
- Depends on: WP-X1, WP-F2, WP-N3, WP-P6.
- Deliverable: Existing focused, fast and complete repository gates run on integrated changes.
- Done check / exit: Receipts identify the final tree and all in-scope behavior checks pass.
- Parallel-plan ready: no. One integrator controls the service runtime and aggregate acceptance runs.

### Milestone: review rendered and code evidence

- ID: M23; package: WP-V2; owner: INTEGRATOR.
- Depends on: WP-V1.
- Deliverable: Fresh relevant captures and independent agent code/visual/doc review.
- Done check / exit: Agent reviewers resolve findings through owners and confirm final-tree evidence.
- Parallel-plan ready: yes. REVIEW and DOC-RECONCILIATION, maximum two read-only reviewers; capture runtime stays single-owner.

### Milestone: close implementation records

- ID: M24; package: WP-V3; owner: MANAGER.
- Depends on: WP-V2.
- Deliverable: Durable docs, audit resolutions, ledger, changelog and completion receipt updated.
- Done check / exit: All required tasks complete; probes/staging removed; local deliverables ready independently of human review.
- Parallel-plan ready: no. One manager reconciles shared documentation and records closure.

WP-F1 can proceed while NAVIGATION completes publisher files. WP-F2 starts after WP-F1: FR-STUDENT
and the ready files in FR-SHELL/FR-WORKSPACE proceed immediately. App/shell and Assessment publisher tasks wait for WP-N1,
Question/Draft route and JSON editor tasks wait for WP-N2, and Blueprint detail waits for WP-N3. The ledger names these
exact handoffs. The manager leases shared types, harnesses, stylesheets and generated artifacts to
one owner at a time.

## Workstream breakdown

| Stream / owner | Goal | Needs | Provides / review boundary |
| --- | --- | --- | --- |
| CORE / component engineer | Expressive shared record presentation and controlled behaviors | Completed investigation and WP-B1 leases | Shared API, primitives, pilots, harness changes and final cutover |
| STUDENT / Student engineer | Preserve Course, score and Attempt decisions | Accepted contract used by each ledger row | Seven file migrations with focused evidence |
| INSTRUCTOR / Instructor engineer | Standardize scans/forms and Assessment discovery | Per-file contract and WP-P5 picker handoffs | Existing callers plus Questions page; preserve editing and revisions |
| LIBRARY / Library engineer | Standardize discovery and selection | Accepted contract used by each ledger row | Eight direct caller migrations plus the adjacent Library sort owner |
| BLUEPRINT / Blueprint engineer | Standardize Blueprint scans/configuration | Accepted contract used by each ledger row | Ten file migrations preserving history, order and drafts |
| DISCOVERY-CONTRACTS / contract engineer | Scope the 250 discovery bound | WP-B1 | Typed bounds, SQL/API/decoder changes and generated handoff |
| QUESTION-DATA / database/API engineer | Bound Question query work | WP-P0, then WP-P1/WP-P2 | SQL pages/facets and bounded summary hydration |
| BLUEPRINT-DATA / database/API engineer | Make existing sort choices global | WP-P0 | Typed full-query order/cursor contract for BLUEPRINT |
| AVATAR / selection engineer | Apply shared catalog presentations | Accepted contract used by each ledger row | One picker migration preserving radio behavior |
| FRAME / layout engineer | Enforce ordinary geometry and spacing | WP-X1; per-file navigation handoffs | Shared primitives and every frame-site disposition |
| NAVIGATION / navigation engineer | Complete ancestry and loaded context | WP-B1 | Central parents, safe publishers and local Blueprint return |
| REVIEW / independent reviewer | Evaluate final behavior and design | Final tree and receipts | Findings routed through INTEGRATOR to responsible owners |
| DOC-RECONCILIATION / documentation reviewer | Compare docs/ledger with implementation | Final tree and receipts | Read-only discrepancy list for MANAGER |

FR-STUDENT, FR-WORKSPACE and FR-SHELL are the three FRAME consumer lanes with exact file assignments
in the ledger. They consume the accepted WP-F1 layout; only overlapping files wait for their named
navigation handoff. FRAME retains ownership of common CSS. PROGRESS and HISTORY own their named
pilot files before the ordinary sweep.

## Work packages

Assign a fresh owner to each bounded dispatch. The migration lane is a coordination group, not a
single ten-file assignment: dispatch one whole file or a small inseparable capability, with all
co-located sites and CSS. Keep the accepted shared contract in every prompt. When a caller pressures that
contract, send its complete task content and evidence to CORE; continue unrelated leased work.

Include these instructions in every bounded worker prompt, together with its exact files,
prerequisites, accepted interface, outcome and focused validation:

- Ordinary scans provide typed content, native action semantics and bounded domain bodies. RecordList owns presentation and reflows the
  same content at every width. Use Detail for genuine forms/reviews and Sequence for ordered tasks.
- Name the actual editor or rich-content task in a body receipt. Keep record identity/facts/actions
  in shared content, form-local controls with their inputs, and record geometry/reflow in shared code.
- Preserve useful descriptions, media, links and action states through the shared capabilities. Remove
  genuine repetition. Send an unsupported task to CORE with a populated example and improve the shared
  boundary; continue independent leased work.
- Keep RecordList in Solid; use bounded data from its owner. Broad-query sorting runs before server paging; local draft ordering remains local.
- Use existing permanent checks first. Add a durable behavior case only for an uncovered stable
  requirement or reproduced failure. Reuse the existing harness; keep inventories, measurements
  and presentation comparisons temporary and remove them after recording results.
- Use representative populated wide/narrow evidence for shared reflow and the known constrained
  container failure where relevant. Additional widths answer a reproduced layout question.
- Preserve other owners' edits. Complete the assigned file and its obsolete local presentation
  cleanup, then return the focused evidence and handoff receipt to MANAGER.

| ID / owner | Entry prerequisites | Concrete outcome and touch points | Acceptance / follow-on |
| --- | --- | --- | --- |
| WP-B1 / MANAGER | None | Adopt completed caller/content inventory, reconcile source drift, establish fresh baseline receipts and file leases | Concrete contract and per-child prerequisites ready; release CORE, NAVIGATION and existing Detail conversions |
| WP-C1 / CORE | WP-B1 | Implement shared title/description/facts/media/actions, bounded body and retained-result state rendering; adapt existing harness | Representative content fits one standard; browser same-ID refresh updates metadata/actions and retains input focus/draft; release pilots, Sequence and sort work |
| WP-C2P / PROGRESS | WP-C1 | Migrate Progress and remove its row/phone styling | Populated wide/narrow Coursework retains released/withheld scores and the shared skin |
| WP-C2H / HISTORY | WP-C1 | Migrate Attempt History and remove its row/phone styling | Populated wide/narrow history retains scores, pagination and flat shared boundaries |
| WP-C3 / CORE | WP-C2P, WP-C2H | Implement controlled native selection using existing controls and the same scan content | Radio/checkbox identity, disabled state and keyboard behavior work; release selection consumers |
| WP-C4 / CORE | WP-C3 | Prove Question discovery with description, exact ID/copy, classification, retired notice and selection in one scan | Preserve all useful Scan/Preview content, return/focus/window behavior; release Library consolidation |
| WP-C5 / CORE | WP-C3 | Implement shared Gallery/List over the ordinary media/content model and controlled selection | Catalog image/name/description supplied once; image failure and native radio behavior work |
| WP-C6 / CORE | WP-C1 | Replace Sequence regions with shared content and bounded editor body under native ordered items | Read-only ordered records and real editor content fit the shared frame; release read-only Sequence consumers |
| WP-C7 / CORE | WP-C6 | Share Sequence move/drag, boundary disabling, completed-move announcement and focus; expose same control for fork Module Outline | Local and asynchronous ordering retains caller save/error policy; release six editable callers |
| WP-C8 / CORE | WP-C1 | Share native sort select with typed choices/current value/callback | Library and Blueprint query choices use the control; WP-P4 makes Blueprint ordering global |
| WP-C9 / CORE | WP-C1, WP-P0 | Shared Previous/Next and 50/100/250 choice; full package in bounded data workstream | Controlled navigation works; release named discovery consumers |
| WP-P0 / DISCOVERY-CONTRACTS | WP-B1 | Scope 250 discovery bound across model, SQL, API and decoders | 250 accepted; excess rejected; unrelated limits preserved |
| WP-P1 / QUESTION-DATA | WP-P0 | Store-only typed predicates/cursor-position contract, SQL filters, global sort and bounded keyset page | Correct page across ties and metadata filters; WP-P3 adapts the existing parsed server query/token once |
| WP-P2 / QUESTION-DATA | WP-P1 | Full-query authorized facet aggregates | Beyond-page counts and bounded groups preserved |
| WP-P3 / QUESTION-DATA | WP-P2 | Adapt the existing normalized server query and opaque cursor to the WP-P1 store contract; hydrate only page items; remove full-catalog pipeline | Existing page shape with bounded source resolution; query digest and token encoding remain server-owned |
| WP-P4 / BLUEPRINT-DATA | WP-P0 | Existing sort choices become API/SQL order with bound cursors | Cross-page rank correct; release My Blueprint caller |
| WP-P5 / INSTRUCTOR | WP-L1.question_picker, WP-BP1.question_pool_picker, WP-C7, WP-P3 | Complete Assessment page/view using reusable pickers; remove dump endpoint | Later-page selection and draft Save remain usable |
| WP-P6 / INTEGRATOR | WP-X1, WP-P3, WP-P4, WP-P5 | Measure bounded work; keep useful windowing only; remove obsolete machinery | Required data evidence accepted before WP-V1 |
| WP-S1 / STUDENT | Base proof; WP-C3 only for recovery | Migrate STUDENT ledger files in bounded dispatches; keep Course context, collection states, scores, recovery selection and pagination | Relevant populated tasks and existing Student entry evidence; release files to FRAME |
| WP-I1 / INSTRUCTOR | WP-B1 Detail; base/WP-C6/WP-C7 per child; WP-P5 Questions workspace | Migrate scans and Detail forms; keep Course inline editor inside bounded record body; adopt shared movement | Course link, Template pressed state, exact revisions, Inspect/Add, reorder/focus, saves/conflicts and Account validation retained |
| WP-L1 / LIBRARY | Base/WP-C6; WP-C4 browse; WP-C3/WP-C7 picker; WP-C8 sort; WP-C9/WP-P3 per discovery child | Migrate eight direct callers plus library_page sort control; consolidate metadata modes and loading/error notices | Description, exact references, copy, selection, return/focus, global sorting and bounded paging remain usable |
| WP-BP1 / BLUEPRINT | Base/WP-C6/WP-C7 per child; WP-C3/WP-C6 Pool picker; WP-C9 discovery; WP-C8/WP-P4 Course sort | Migrate BLUEPRINT content/actions, shared movement and sort controls; preserve real domain bodies | History, Pool selection, chronology, fork destinations and deferred saves retained |
| WP-A1 / AVATAR | WP-C5 | Migrate picker/CSS to the shared task presentation and remove wrapper geometry | Existing avatar presentation check and native radio selection pass |
| WP-X1 / CORE | WP-S1, WP-I1, WP-L1, WP-BP1, WP-A1 | Final public API cutover; delete old region/grid assembly, presentation wrappers, row/phone overrides and staging files | Whole-tree compiler/fast UI pass and ledger reconciled; release FRAME |
| WP-F1 / FRAME | WP-X1 | Implement shared stack/PageSection; migrate Student Courses, Grades, `assessment_workspace_create_page.tsx`, and `route_access_boundary.tsx` as pilots | Shared geometry/spacing and legitimate inner layout coexist; these pilots are outside NAVIGATION leases |
| WP-F2 / FRAME | WP-F1; navigation handoff per child | Sweep all frame sites and hooks in bounded groups; migrate repeated ordinary spacing and record retained editor hooks | Every frame site has disposition/evidence and shared outer layout ownership |
| WP-N1 / NAVIGATION | WP-B1 | Correct/extract breadcrumb model; extend route-scoped labels and shell plumbing in route context, shell and app; adapt the existing Assessment publisher | Parent cases, loading labels, stale-generation rejection and session clearing pass; release app/shell and Assessment publisher files; start new name publishers |
| WP-N2 / NAVIGATION | WP-N1 | Publish Question/Draft/Blueprint names from existing loads and choose the real Blueprint parent | Actual parent clicks, direct loads, denied routes and rapid A-B-A navigation retain truthful names; release Question/Draft route and JSON editor files; carry Blueprint detail into WP-N3 |
| WP-N3 / NAVIGATION | WP-N2 | Present Module/Assessment context using existing selection and return behavior in Blueprint detail | Fixture edits survive local return and focus is restored; release Blueprint detail to FRAME |
| WP-V1 / INTEGRATOR | WP-X1, WP-F2, WP-N3, WP-P6 | Run integrated focused, fast and complete existing gates | Final-tree receipts identify passing in-scope behavior and any exact external limitation |
| WP-V2 / INTEGRATOR | WP-V1 | Capture relevant actual-shell states; independent agents review code, visuals and document consistency | Findings corrected by owning package and affected evidence rerun; final evidence accepted by agents |
| WP-V3 / MANAGER | WP-V2 | Update durable docs, audit resolutions, ledger and changelog; remove staging and temporary evidence; record local completion | Every required task closed, shared redesign complete and no human-dependent milestone remains |

WP-P0-WP-P6 and WP-C9 use the bounded data workstream's package definitions and ledger handoffs.
The ledger's per-child prerequisites control dispatch. Base-only callers follow the Student pilots;
selection follows WP-C3; Question browse WP-C4; Gallery WP-C5; read-only Sequence WP-C6; editable
Sequence WP-C7; sort controls WP-C8. Existing Detail conversions start after WP-B1. A mixed file waits
for the capabilities it actually consumes. A capability is accepted on its demonstrated examples;
record unsupported task content before declaring that boundary complete.
CORE keeps accepted interfaces stable as later compositions arrive. If evidence requires a refinement,
CORE updates it once, records the affected consumers and coordinates just those file handoffs before
owners resume. Each lane keeps its exact file set; shared harness changes go through CORE and route
harness changes through NAVIGATION. The complete WP-X1 cutover still waits for every caller task.

## Acceptance criteria and gates

- Pages express useful title, description, semantic facts, media and native actions through the shared renderer.
  Actual editors use its bounded body; the shared frame retains ownership of record presentation.
  The shared family owns markup, geometry, title style, responsive
  reflow and row CSS. All current callers have a completed disposition and evidence.
- Each page supplies the same typed content at all widths. A viewport change only reflows the shared
  scan while preserving the selected task presentation, essential information and semantic order.
- The standard retains Course context, exact required revision identities, decision status, and
  primary actions in representative wide and narrow renders, plus the constrained desktop-container
  case that exposed the audited defect. Check an actual shared layout transition if one exists.
- Scores and "Score not released" remain visible directly in the scan. Required Assessment Type
  icons, labels and theme colors come from shared semantic rendering.
- Rows use shared insets and dividers; separate rounded objects have a visible gap and parent inset.
  Shared edges have one divider. Compactness and reachable focus remain.
- One complete Library scan replaces the duplicate metadata layouts; image List/Gallery, copyable IDs,
  pressed actions, linked Course context, retained-result notices and shared sort/reorder controls work.
  Bounded paging, inline editors and native table/outline semantics retain their tasks; windowing earns retention in WP-P6.
- Question predicates/order/facets are database-owned; only returned native Questions require source resolution.
  Discovery keeps one result page, preserves independent selection, and sorts the whole match.
  Assessment pickers reach later pages; superseded catalog-dump and accumulation paths are removed.
- PageFrame owns outer geometry and standard content rhythm; every use has been reviewed, including
  shared loading, denied, and failure surfaces. Retained editor wrappers have a specific workflow reason.
- Breadcrumbs show actual parent pages and current authorized loaded names while preserving Course
  context. Blueprint selection remains identifiable and locally navigable.
- Old region inputs, caller row overrides, transitional entry points, and meaningless Sequence
  fields are removed, with completion recorded in the deletion receipt.

Per-patch gate: the affected compiler/build boundary, existing lint/format checks on touched source,
focused behavior checks, and a populated render when layout changed. Documentation changes use the
existing Markdown-link check and a read of changed prose for structure and whitespace. Report API
cutover progress separately from a complete-tree pass. Integration gate: fast checks, fast UI, full
repository acceptance, fresh captures, documentation validation, and independent code/visual review
of the final tree.

## Test and verification strategy

Use existing permanent coverage first. Update the stable contracts affected by this redesign;
retain a small number of new behavior cases only where existing coverage misses the demonstrated
failure. Source-grep inventories, variant comparisons, and exact pixel measurements are one-time
evidence. Keep permanent protection centered on shared user behavior rather than migration rows or CSS selectors.

Verify shared reflow with the same populated records while resizing from wide to narrow. Use the
repo's 1280 px laptop and the audit's 393 px phone as initial observations, plus the demonstrated
400 px container within a wide viewport. Probe any actual shared layout transition and use enlarged
text when diagnosing crowding or truncation. Other widths are diagnostic follow-ups to an observed
failure. Judge whether the one shared layout works; these samples define neither separate layouts
nor an exhaustive matrix. Keep durable reflow coverage concentrated at the shared component and
use representative caller captures for composition regressions.

Commands are run from the repository root with `source ./source_me.sh &&`:

| Scope | Command / evidence |
| --- | --- |
| Compiler | `npx tsc --noEmit -p tsconfig.json` |
| Touched TypeScript/JavaScript | `npx eslint --max-warnings 0 <touched files>` and `npx prettier --check --ignore-path .gitignore --ignore-path .prettierignore --ignore-path .prettierignore.local <touched files>` |
| Shared family | `node --import tsx tests/playwright/record_list_contracts.mjs` |
| Avatar | `node --import tsx tests/playwright/provided_avatar_picker_presentation.mjs` |
| Route model | `node --import tsx --test tests/test_ribbon_contract.mjs tests/test_ribbon_route_contract.mjs tests/test_student_ribbon_navigation.mjs` |
| Integrated offline | `./launchers/run_fast_checks.sh` |
| Current-source browser | `./launchers/run_fast_ui_checks.sh` |
| Full acceptance | `./launchers/all_test.sh` |
| Fresh actual-shell corpus | `./devel/capture_screenshots.sh --fresh`; inspect `docs/SCREENSHOT_ATLAS.md` and relevant PNGs |
| Documentation | `python3 -m pytest -q tests/test_markdown_links.py`; review changed prose for heading/table structure and whitespace |

CORE updates the existing family harness to assert retained status/actions, native selection and
reference/media/body controls, native action state, stable-ID refresh, loaded-result retention, family
semantics, reorder focus and any retained window behavior. The bounded data workstream assigns SQL/API and paging checks. NAVIGATION updates existing
route/shell cases for ancestry and stale publication. Populated page probes cover domain composition
the isolated harness cannot establish. Keep generated reports/screenshots with their normal owner;
temporary probes belong under ignored test scratch paths and are removed after their results are recorded.

Visual review includes actual-shell Student Courses, Progress, Attempt History, Course Assessments,
Library discovery, avatar List/Gallery, form entries, Blueprint selection, Gradebook, and denied/loading
surfaces. Compare the same populated task before/after, inspect native PNGs, and exercise keyboard
actions. State which palette actually rendered; OS color-scheme emulation alone does not prove a
dark theme. Observe readable grouping and boundary separation rather than inventing pixel gates.

A failing focused behavior blocks its package: correct the responsible shared component or adapter
and rerun that case. Reproduce unrelated baseline failures separately and continue independent work;
report them without relabeling them as passing. Full completion requires a passing integrated tree.
Browser sandbox startup failures require the repository-supported outside-sandbox path, not a skipped
visual gate. One integrator leases the Live Demo/capture runtime; use its stale-bundle rebuild path.

## Migration and compatibility policy

This pre-production change permits direct API replacement. Implement the strict API and remove the
old one as one bounded migration; preserve user behavior rather than the former caller contract.
The deliverable is the completed redesign. Once the shared contract is proven, migrate every caller
and delete the superseded implementation. Migration scaffolding exists only for the bounded cutover;
the finished architecture uses the shared family directly.

For a compiler-safe staged cutover, CORE may expose the strict List and Sequence through one temporary
internal entry while unchanged callers use their current entries. WP-C1 establishes that entry and WP-C6 adds the shared Sequence export to it. The two pilots and subsequent callers use the relevant
strict export. WP-X1 moves both finished implementations to their existing public entries, normalizes
imports, and deletes old implementations and the temporary entry. Keep styles distinguishable during
this short interval so migrated callers receive the new shared skin. Record exact paths and WP-X1's
removal obligation in the ledger. The finished List and Sequence APIs share semantic content and bounded bodies; Sequence adds native
order and controlled movement.
If an atomic cutover is practical, use the public entries directly.

WP-X1 owns a concrete deletion receipt: old RecordRegion contracts and grid
assembly; superseded presentation factories/wrapper CSS; Library metadata-mode and duplicate
loaded-state notice code; caller movement/sort markup replaced by shared controls; page row overrides
and duplicate phone summaries; ignored Sequence inputs; old imports, exports and compatibility branches; and every staged
entry/module. Keep a helper only when the finished shared family actually uses it. The compiler,
focused behavior checks and one-time source inventory verify this receipt.

WP-V3 removes migration-only fixtures, synthetic probe scripts, comparison renders, measurements and
temporary reports after durable checks and normal published screenshots carry the needed evidence.
Record results and limitations concisely in the ledger; retain useful behavioral protection in the
existing tests. The final tree contains the finished API and durable product evidence.

The manager may resolve implementation details and documented exceptions using evidence without
further human approval. Keep scope and API choices in the ledger. If evidence exposes a new product
workflow or a permission/persistence change, preserve the existing behavior and record the separate
issue. Complete this plan within its presentation and navigation scope.

## Risk register

| Risk | Impact | Trigger | Owner | Mitigation |
| --- | --- | --- | --- | --- |
| Restrictive contract pushes content outside the family | Missing information or new local wrappers | Description, media, action state or real editor has no shared home | CORE | Complete the demonstrated semantic capability or bounded body; review caller task completion, not field count |
| Arbitrary JSX returns under a new name | Standardization fails again | Bounded body recreates title/actions/row skin or a page requests geometry knobs | CORE | Use semantic content and shared controls; reserve bounded bodies for real domain content and siblings for distinct tasks |
| Responsive API expands during migration | Every caller designs its phone row again | New breakpoint props, mobile modes, alternate summaries or action placement | CORE | Use one width-independent content model and shared reflow; complete the caller migration and its obsolete-code removal |
| Fake sibling reclassification | Custom scan survives as Detail | Ordinary title/status/action row moved merely to retain CSS | MANAGER | Use Detail for genuine forms/reviews and the typed standard for ordinary scans; record the task evidence |
| Information hidden to make rows fit | Student or Instructor decisions become unreliable | Narrow case loses score, Course, revision, or action | CORE | Shared stacking and disclosure for optional details; rerun affected task |
| Record identity reduced to an attribute | Refresh remounts editors or loses focus | Same-ID row objects or action objects are recreated | CORE | Key mounted owners by record/action IDs, pass reactive row access to bodies and prove draft/focus retention on refresh |
| Solid reactivity captured as a snapshot | Busy/selection/status stops updating | Adapter caches values outside reactive rendering | CORE and caller owner | Keep mapping reactive; exercise update-in-place cases with stable IDs |
| Data remains unbounded behind paged UI | Catalog-sized work and misleading sort | Full fetch/compile, appended history or browser page sorting | DATA and caller owners | Complete WP-P0-WP-P6; verify global order and one-page retention |
| Layout reset remounts controls | Lost drafts/focus/window position | Layout, Gallery switch or editor expansion changes identities | Caller owner | Preserve keys and state ownership; verify focus and unsaved edits |
| Global stack creates empty grid items | Unwanted gaps or broken editor sizing | Hidden announcements/dialog hosts become stack children | FRAME | Inspect actual children, group content deliberately, keep overlays out of layout flow |
| Stale names or false breadcrumb parents | Misleading context or unauthorized label exposure | Deferred load resolves after navigation/session change | NAVIGATION | Route-generation publication guard, clear on disposal/failure, direct-load cases |
| Concurrent ownership drifts | Reverted work or inconsistent shared types | Worker edits CORE, shared CSS, or generated evidence | MANAGER | Whole-file lease, change requests, serial shared integration |
| Green historical evidence reused | Incomplete implementation is declared done | Capture predates final CSS/API or uses only component fixtures | INTEGRATOR | Final-tree receipts and actual-shell captures with explicit limits |

## Documentation close-out requirements

MANAGER owns documentation and closes it using INTEGRATOR's final evidence.

- Preserve the human's shared-base and expressive-content requirement in [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) as
  a short, direct rule; record implementation choices in [DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md).
- Update [CODE_ARCHITECTURE.md](../../CODE_ARCHITECTURE.md) and
  [FILE_STRUCTURE.md](../../FILE_STRUCTURE.md) for the resulting types, components, ownership, and
  breadcrumb publication. Change [CONTRACTS.md](../../CONTRACTS.md) only where its existing UI boundary
  descriptions become inaccurate. DATA owners update changed store/API/SQL contracts and regenerate
  affected bindings through existing tooling; MANAGER serializes generator ownership.
- Keep the full caller audit table and companion ledger current: pending/active/complete, final disposition, exception reason,
  verification receipt, and removed CSS/configuration. Add evidence to the audit as resolution notes.
- Update [CHANGELOG.md](../../CHANGELOG.md) after each bounded accepted task. Distinguish implementation,
  isolated browser proof, actual-shell evidence, and full-service acceptance.
- Refresh the screenshot corpus/atlas through the existing capture workflow. Shared source/API files
  are not generated artifacts. Refresh affected database documentation/bindings for the data workstream;
  the redesign requires no Wasm or avatar catalog regeneration.
- Reconcile overlapping plans with narrow status links, preserving their unrelated ownership and
  historical claims. Remove temporary probes and staged APIs. Archive completed planning artifacts
  using `git mv` only when the index is writable and unlocked; repair inbound links. If that operation
  is permission-blocked, record implementation complete and archival pending, rather than copying files
  around the restriction. Leave committing to the human.

## Open questions and decisions needed

No execution-blocking product question remains. Use this procedure for a requested extension:

1. The caller owner records the complete task, content, actions and source/example, including what
   would be lost if it were reduced to the current schema.
2. CORE uses the existing semantic content, media, controlled behavior and bounded-body capabilities.
   Remove duplicate presentation without discarding useful descriptions, context or interactions.
3. Use a semantic sibling when the task is a form, full review, saved sequence, table or hierarchy.
   When a shared capability is missing, compare a focused addition with the local code it would
   otherwise force. Prefer the design that completes the task with clear ownership and less total code.
4. MANAGER records the chosen shared owner, affected consumers, success condition, validation and
   removal of superseded markup. Refine the capability once and coordinate the affected handoffs.
5. Prove it on the populated task and a narrow container, then release its callers. A capability may
   be justified by one concrete consumer; configuration needs observed task evidence, not speculation.

The proposed content boundary now covers the investigated callers. Rendering details remain subject
to the assigned proofs. Question snapshot generation has a documented backend boundary and is not
misrepresented as an implemented image capability. Exact tokens and module splits remain engineering
decisions under the same evidence and existing style authorities.
