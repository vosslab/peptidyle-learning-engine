# RecordList and PageFrame audit

Date: 2026-09-25. Status: audit complete; recommendations are not implemented.

## Follow-up: data ownership and discovery performance

The [data-flow investigation](record_list_data_flow_investigation_2026-09-25.md) records actual
API/SQL limits, full-catalog Question work, partial Blueprint sorting and browser accumulation.
Its required plan workstream keeps RecordList in Solid, bounds broad discovery at PostgreSQL/API,
and supports the human's 50/100/250 page-size choice with default 50. Earlier findings remain
historical evidence; the linked active plan owns the current implementation direction.

## Follow-up: expressive shared content

The [deeper caller investigation and full caller table](record_list_caller_investigation_2026-09-25.md)
cover every RecordList/Sequence caller and revise the recommendation below. The shared family owns
presentation while supporting real descriptions, linked facts, media, action state, bounded editor
bodies and common selection/sort/reorder behavior. The earlier minimal schema is superseded; the
measured defects below remain valid historical findings. The implementation plan and ledger reflect
the revised content boundary.

## Required direction

The user's clarification establishes the acceptance criterion: RecordList enforces the standard.
Pages supply record content and behavior. A page earns a presentation exception by demonstrating
a task that the shared standard cannot express; the exception is implemented in the shared family.

The current implementation centralizes list markup and mechanics, but leaves too much of the
display design to each caller. It is a useful foundation with an overly permissive public contract.
PageFrame has a stronger outer boundary, but does not yet standardize ordinary content spacing.

Authorities: [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md), especially compact scan rows,
consistent rounding, permanent breadcrumb ancestry, and visible unreleased-score states;
[DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md), especially PageFrame ownership and the semantic
record family. The earlier migration's completion does not establish conformity to this stricter
standardization requirement.

## Findings

### 1. Pages still design rows

**P2: correct the public presentation contract.**

[region_spec.ts](../../../src/components/record_list/region_spec.ts), lines 15-40, requires pages
to choose arbitrary CSS widths, alignment, responsive priority, and arbitrary JSX for every region.
[record_list.tsx](../../../src/components/record_list/record_list.tsx), lines 23-45 and 83-104,
turns those choices into tracks and wrappers. Region order and count are caller-defined; the type
does not require exactly one identity region.

A current-source AST inventory finds 37 RecordList uses and 11 RecordSequence uses. Their region
configuration contains 95 literal region objects and 28 distinct width strings. These counts include
Sequence configuration, some of which is ignored by that component.

Concrete display decisions remain in
[library_browse_rows.tsx](../../../src/pages/library_browse_rows.tsx), lines 139-196:
title markup, summary markup, selection placement, classification layout, and track widths.
[library_browse_record_list.css](../../../src/pages/library_browse_record_list.css), lines 28-60,
then supplies row padding, title sizing, metadata typography, and internal spacing.

**Recommendation:** replace the ordinary scan's region array with a small, typed content contract.
Require an identity/title; admit a summary, decision status, one relevant value/date, and actions.
The component renders their markup and fixes their order, typography, geometry, and wrapping.
Represent link versus command actions as a discriminated union and render shared controls.
Pages map domain data into those fields and retain fetching, permissions, mutations, and persistence.
Returning an arbitrary row or identity JSX tree would leave the same display loophole open.

Keep selection and richer Question previews only where current use demonstrates the need, with
their presentation owned by shared components. Ordinary scans expose no width, alignment, priority,
row class, row style, or general-purpose row renderer. Avoid accumulating page-named variants.

### 2. Phone layout hides scores

**P2: retain decision status at every supported width.**

[student_course_progress_page.tsx](../../../src/pages/student_course_progress_page.tsx),
lines 68-79, marks score and completion status as `high` priority.
[student_course_attempt_history_page.tsx](../../../src/pages/student_course_attempt_history_page.tsx),
lines 76-89, does the same for scores and "Score not released".
[record_list.css](../../../src/components/record_list/record_list.css), lines 88-96, hides that
priority entirely below 32rem.

Current-source Chromium fixtures confirm that both status regions compute to `display: none` at
393 px. Their records and actions remain, so the page appears usable while withholding the very
information the Student is visiting to see. The Progress requirement explicitly preserves this state.

The component also responds to viewport width rather than its available container width. A controlled
400 px Library list container at a 1280 px viewport has 628 px of scrollable content. This is a
demonstrated composability limitation; the ordinary full-width Library fixture fits at all four
examined viewport widths.

**Recommendation:** preserve title, decision status, and primary action; stack them when needed.
Use the shared component's available width to choose its layout. Additional metadata can move into
a shared disclosure or the record detail. A page should not choose which essential content vanishes.
[MDN's container-query documentation](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_queries)
describes the relevant container-based sizing mechanism.

### 3. Page CSS defeats consistency

**P2: remove row decoration overrides during migration.**

[student_course_attempt_history_page.css](../../../src/pages/student_course_attempt_history_page.css),
lines 9-14, changes shared rows into individually bordered, rounded boxes without adding row spacing.
Chromium measures an 8 px radius and a 0 px gap between adjacent boxes. Screenshots confirm the
touching corners and doubled divider. This directly reproduces the reported visual concern.

[student_course_progress_page.css](../../../src/pages/student_course_progress_page.css),
lines 26-31, separately changes padding, border, and background. Its `--ple-radius-md` token is not
defined in the current CSS; the rendered radius is 0 px.
[course_instance_page.css](../../../src/pages/course_instance_page.css), lines 180-184,
adds a 6.5rem minimum row height and accent border to descendant lists.

**Recommendation:** use one compact, flat scan treatment with shared title/metadata styles, consistent
inset padding, and one divider between records. Reserve modest rounding for the collection boundary
when it actually needs a surrounding surface. Keep buttons and controls rounded inside that inset.
This removes the touching-box problem without introducing large decorative gaps.

For genuinely separate cards, use a shared card presentation with a real gap and parent padding.
One component owns each visible boundary; avoid an outer panel border and another touching inner
card border. Use the existing spacing/radius tokens. Delete superseded page selectors as each caller
migrates, rather than leaving overrides in the cascade.

### 4. Breadcrumb ancestry is inconsistent

**P2: fix the central breadcrumb model and loaded names.**

This belongs to [ribbon_contract.ts](../../../src/ribbon/ribbon_contract.ts), not PageFrame.
At line 554, the Student Course ancestor is built from `studentCourseProgress`, so the apparent
parent of All Coursework leads to Progress. Lines 644-661 omit Courses from Course trails and omit
the Progress label from the Progress page. Both the pure model and browser shell reproduce this.

At lines 662-686, Question and Blueprint detail trails use generic current labels.
`RibbonContextLabels`, lines 81-86, admits Course and Assessment names but has no Blueprint or
Question title. The Blueprint editor also changes selected Module/Assessment content inside one
route without reflecting that selection in the global trail; see
[blueprint_course_detail_workspace.tsx](../../../src/features/blueprint_course/blueprint_course_detail_workspace.tsx),
lines 724-734 and 791 onward.

Examples of the proposed hierarchy:

| Surface | Current trail | Proposed trail |
| --- | --- | --- |
| Student Course landing | Home > Course name > All Coursework; Course links to Progress | Home > Courses > Course name |
| Course Progress | Home > Course name | Home > Courses > Course name > Progress |
| Attempt History | Home > Attempt History | Home > Grades > Attempt History, with Grades using its existing Scores landing |
| Blueprint detail | Home > My Blueprint Courses > Blueprint Course | Home > relevant Blueprint collection > actual Blueprint name |
| Question detail | Home > Question Library > Question | Home > Question Library > actual Question title |

Keep the existing Instructor Assessment hierarchy, which already includes Course, Assessment, and
section. Instructor Home already serves My Active Courses; inserting another link to the identical
destination adds no useful ancestry.

**Recommendation:** declare real parents centrally and compose the complete trail from that model.
Publish already-loaded object names through the existing route-scoped context mechanism. Keep the
fixed breadcrumb row and accessible current-page semantics. The
[W3C breadcrumb pattern](https://www.w3.org/WAI/ARIA/apg/patterns/breadcrumb/)
supports parent links in hierarchical order.

For Blueprint's local selection, use a shared workspace trail that shows Blueprint, Module, and
selected Assessment, with a working return-to-outline action that preserves draft edits. Module and
Assessment are currently local selections, so adding fictitious route links would misrepresent the
application. A later routed workspace can promote those levels into the global breadcrumb. Public
Blueprint records need a valid collection parent rather than an unconditional "My" collection.

### 5. PageFrame stops before content rhythm

**P2: centralize ordinary section spacing.**

[page_frame.tsx](../../../src/components/page_frame.tsx) correctly centralizes the title,
optional introductory slots, actions, and route-owned content width. Its root does not accept caller
classes. However, [page_frame.css](../../../src/components/page_frame.css), lines 34-36, gives
the content region only `min-inline-size: 0`; children have no shared stack or gap.

Twenty-four of 62 PageFrame uses supply `contentClass`. Those classes are not all mistakes: editors
need internal composition. The gap is the lack of an ordinary shared content layout. For example,
[student_courses_page.tsx](../../../src/pages/student_courses_page.tsx), lines 61-71, places a
Course invitations link immediately before RecordList in default block flow.

**Recommendation:** let PageFrame provide the standard vertical content stack, and provide a small
shared section primitive for heading, helper text, local actions, and body. Put the invitations link
in the admitted page-action slot. Keep task-specific editors inside the common frame and section
spacing. Replace repeated grid/gap rules with those primitives; retain domain-specific inner layout
only where the workflow requires it. Inspect direct children before applying a grid globally.

## Smaller contract issues

- [record_sequence.tsx](../../../src/components/record_list/record_sequence.tsx), lines 10-16
  and 31-41, requires `RecordRegion` but ignores width, alignment, priority, and header. Remove these
  meaningless required inputs. Share content types only where they have the same meaning.
- [record_list.tsx](../../../src/components/record_list/record_list.tsx), lines 98-101, maps
  the documented horizontal `align` setting onto both axes. An end-aligned action is also bottom
  aligned. Shared row policy should choose these axes independently.
- [blueprint_courses_workspace.tsx](../../../src/features/blueprint_course/blueprint_courses_workspace.tsx),
  lines 225-265, wraps the collection in custom loading/error/empty branches and gives RecordList a
  ready state. Use the shared collection-state view for that collection, while preserving distinct
  parent-resource failures and incremental-loading behavior.

## Preserve useful boundaries

- Keep Sequence, Table, Outline, and Detail presentations. Saved order, named table columns,
  hierarchy, and expanded reviews have different semantics. Their shared presentation should also
  be enforced, but forcing all of them into a scan row would lose useful structure.
- Keep typed rows, readonly public inputs, stable record identities, shared accessible states, and
  focus-aware windowing. Table column proportions are a justified task-specific choice already
  admitted by the design decision.
- Keep persistence timing and failure recovery with the workflow. Standardizing the control's
  appearance does not make deferred Blueprint saves and immediate membership mutations equivalent.
- These defects have no PostgreSQL or Rust/Wasm owning boundary. Resolve them in the TypeScript
  presentation contract, shared CSS, and navigation model.

## Bounded implementation sequence

| Step | Owner and concrete work | Success condition and verification |
| --- | --- | --- |
| Shared scan contract | Record family owner: replace layout regions with typed content and standard controls; migrate Progress and Attempt History | Both pages use shared markup and skin; score/status and actions remain visible at laptop and phone widths; inspect rendered pages |
| Scan caller migration | Page owners: map every current scan to the contract; remove corresponding row CSS; record any justified shared extension | Every scan has a disposition; ordinary pages specify no row geometry or typography; inspect representative long titles, actions, selection, and preview |
| Frame rhythm | PageFrame owner: standard stack and section primitive; migrate repeated spacing and page-action placement | No touching nested boundaries in affected pages; editors retain their necessary internal layout; inspect actual parent containers |
| Breadcrumb hierarchy | Navigation owner: central parent relationships, Course landing links, loaded object names, Blueprint workspace selection trail | Parent links reach the named destination; current labels match the visible task; returning preserves unsaved Blueprint edits |

Keep durable checks on observable contracts: retained status/actions, semantic table/order/nesting,
real parent destinations, and focus during windowing/reorder. Use temporary measurements for visual
diagnosis. Do not add a permanent source-grep test for each removed CSS override.

## Evidence and limits

- Used targeted Graphify before source review. Its older RecordList caller count was stale; the
  counts above come from the current TypeScript AST, not the graph snapshot.
- `source source_me.sh && npx tsc --noEmit -p tsconfig.json` passed.
- Documentation link validation passed (326 checks); `git diff --check` passed.
- `source source_me.sh && node --import tsx tests/playwright/record_list_contracts.mjs` passed
  outside the sandbox. The initial sandbox attempt failed during Chromium startup. Existing
  contracts verify mechanics; they do not establish that caller-selected hidden content is optional.
- Temporary Chromium evidence uses actual Library page composition and actual Progress/Attempt
  History components with controlled API data at 1280, 800, 600, and 393 px. No browser errors were
  recorded. Normal tested layouts had no horizontal page overflow; the deliberately narrowed
  Library container had internal overflow. Student fixtures omit the application shell, so their
  outer viewport edge is not evidence of production shell padding.
- Visually inspected laptop and phone Attempt History captures and the Library capture. Status
  disappearance and adjacent-row geometry were also measured directly. OS color-scheme emulation
  did not change the current palette; this is not a dark-theme acceptance claim.
- Captures, fixture source, AST inventory, breadcrumb output, and measured JSON are disposable
  evidence under `/private/tmp/ple_record_frame_audit_2026_09_25/`.
- No production code changed. No fresh full-service run or complete screenshot corpus was produced
  for this audit. Earlier changelog acceptance runs remain historical evidence.
