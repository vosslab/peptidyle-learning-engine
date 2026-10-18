# DataSheet spatial baseline

**Milestone:** M03, baseline evidence ready for independent review
**Capture date:** 2026-10-16
**Runtime:** disposable `ple-live-demo-browser` Live Demo, seeded Instructor/Student/Admin identities
**Scope:** first content and action placement on representative collections; one-time measurement, not a pixel-threshold test

## Findings

The Instructor Question Library has the largest measured pre-record stack. On the 1280 x 800
laptop capture, breadcrumbs end at y=115, the page action begins at y=136, the search field begins
at y=211, shared search controls end at y=472, and the first result row begins at y=776. Fifty
records are loaded, but only the first row and a sliver of the next intersect the viewport. On the
393 x 852 phone capture, the first result begins at y=1085, below the viewport; none of the 50
loaded rows is visible without scrolling. The Instructor account rendered Light mode even when the
browser context requested Dark mode.

Student Courses and Sysadmin Find Courses have shorter control stacks. Their first course rows
appear at y=481 and y=442 on laptop, and y=603 and y=495 on phone, respectively. Each has one
seeded matching Course and its direct Open Course or Inspect action is visible. Student and
Sysadmin phone captures rendered Dark mode. Their narrow table rows extend horizontally beyond the
viewport while their direct action buttons remain visible.

These measurements identify shared collection spacing, control frequency, and duplicate status
content for later milestones. They do not establish an arbitrary acceptable pixel limit.

## Coverage and measurements

Coordinates are CSS pixels from the viewport origin. "First row" is the first `tbody tr[data-record-id]`,
including rows below the viewport. Visible counts use viewport intersection; an edge-clipped row
still intersects. "Pre-record overhead" is first-row top minus breadcrumb bottom.

| Role and surface | Viewport and rendered mode | Breadcrumb bottom | First useful page control | Search input | Shared search controls bottom | First row top | Pre-record overhead | Loaded / intersecting rows |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Instructor, filtered Question Library | 1280 x 800, Light | 115 | Create Question Pool, 136 | 211 | 472 | 776 | 661 px (83% of height) | 50 / 2, both edge-clipped |
| Instructor, filtered Question Library | 393 x 852, Light* | 130 | Create Question Pool, 152 | 236 | 749 | 1085 | 955 px (112% of height) | 50 / 0 |
| Student, Courses | 1280 x 800, Light | 115 | Course invitations, 136 | 219 | 257 | 481 | 366 px (46% of height) | 1 / 1 |
| Student, Courses | 393 x 852, Dark | 130 | Course invitations, 152 | 244 | 355 | 603 | 473 px (56% of height) | 1 / 1 |
| Sysadmin, Find Courses | 1280 x 800, Light | 115 | Course-name search, 149 | 149 | 201 | 442 | 327 px (41% of height) | 1 / 1 |
| Sysadmin, Find Courses | 393 x 852, Dark | 130 | Course-name search, 189 | 189 | 248 | 495 | 365 px (43% of height) | 1 / 1 |

\* The Instructor's actual rendered mode stayed Light under a requested Dark browser scheme. The
probe recorded the DOM's rendered mode and did not change saved account appearance settings.

Student canonical captures also cover the 800 x 1280 tablet and 800 x 800 square profiles, with
both Course list and Course landing states. The Instructor canonical Course list and Question
Library workflow and the Sysadmin home capture use the 1280 x 800 laptop profile. Additional
phone captures fill the Instructor and Sysadmin narrow-view gaps; Sysadmin Find Courses provides
the populated administrative collection view.

## Content-before-record rubric

This rubric records the purpose and current cost of each visible layer. Ownership follows the
current component boundaries in the M01-approved plan.

| Visible layer | Purpose and necessity | Frequency and redundancy | Measured space / task effect | Current owner and precedence |
| --- | --- | --- | --- | --- |
| Ribbon and task navigation | Establish role, destination, and available tasks; necessary on every page. | Always present; navigation labels are not repeated in the collection. | The main content begins below the shared Ribbon. Narrow Ribbon tabs horizontally scroll. | Ribbon shell; keep before page identity and content. |
| Breadcrumbs | Show the route within the role's work; useful on every sampled surface. | One trail per page. | Bottom is y=115 on laptop and y=130 on phone. | Ribbon breadcrumbs; follow shared navigation. |
| Page action | Create a Question Pool from the Library; open Student course invitations. Useful, distinct actions. | One prominent action/link on the sampled Instructor and Student pages. | Starts y=136 laptop / y=152 phone. Each takes one activation before its destination. | Page/domain command; place before search when it is a common task. |
| Search text and submit | Find a Course or Question by visible text; necessary for collection discovery. | Common. One text field and one Search action. | Search starts y=211 on Instructor laptop and y=236 on Instructor phone; y=219 / 244 for Student; y=149 / 189 for Sysadmin. Search takes text entry plus submit, then a direct record action to open/inspect. | SearchBlock shared controls with the page's query definition; before result controls and records. |
| Search count and clear action | State the current page size and allow query reset. | Useful after a query. The Instructor also repeats the query in an "Applied search" line and a filter chip. | Search controls end at y=472 on Instructor laptop and y=749 on phone. Count text says 50 shown; later selection text repeats 50 loaded results. | SearchBlock / SearchControls; retain query status and clear behavior near the query. |
| Question membership and Owner account ID | Restrict Library content by membership and owner. | Membership is common; Owner is less frequent. Both fields are expanded in the default filtered view, including an empty Owner value. | Two full-width rows appear before Search tips, display modes, sorting, filtering, and records. | Library page supplies these criteria; consider their frequency when composing SearchBlock controls. |
| Search tips | Explain search syntax. | Occasional help; collapsed in the captures. | Still occupies a separate disclosure row before collection controls. | SearchBlock/page guidance; keep subordinate to common tasks. |
| Display modes | Change how records are scanned. | Repeated collection operation. | A separate row precedes Sort. The baseline exposes Compact, List, and Visual boxes; the M01 contract expects Compact, List, Cards, and Visual Boxes. | Shared DataSheetControls; follow query criteria and precede records. |
| Sort and Filters | Order or narrow the result collection. | Common but less frequent than search and direct record opening. | Each is a separate collapsed row. On Instructor phone, the Sort disclosure only begins at the bottom of the screenshot. | Shared DataSheetControls, with page-owned filter definitions. Keep keyboard/touch access and query context. |
| Selection and bulk controls | Select records for a meaningful bulk action. | Occasional. The default Library state has no selection. | "0 selected from 50 loaded results" duplicates the search count, while "Select loaded results" and "Clear selection" remain visible at zero selected. These controls sit before Columns and the record header. | Shared selection/DataSheetControls; optional bulk UI should follow whether bulk action is meaningful. |
| Columns | Configure table fields. | Occasional. | A separate disclosure row appears after selection and before the table header. | Shared DataSheetControls/HeaderRow; place after more frequent search, sort, and filter operations. |
| Table heading and first record | Identify the collection fields and the useful data. Essential primary content. | Always needed. | First row begins at y=776 (Instructor laptop), y=1085 (Instructor phone), y=481 / 603 (Student), and y=442 / 495 (Sysadmin). | DataSheetBlock/HeaderRow/RecordItem; should follow the controls needed for the active task. |
| Direct record action | Open a Course or inspect it. Primary task for Student and Sysadmin. | Common; one action after the row is visible. | Open Course and Inspect remain visible on phone despite row overflow to the right. Instructor Open is below the phone viewport until scrolling. | RecordItem/page route; keep directly reachable without selecting the row. |
| Pagination and page size | Navigate beyond the loaded page and choose a page size. | Low immediate need for the one-row Student/Admin results; necessary as collections grow. | Previous/Next and a 50-row selector are displayed below the one visible row, with navigation disabled for the singleton result. | Shared DataSheetControls; after records so it does not displace the first useful result. |

## Common task cost observed

- **Search the Question Library:** activate the labeled search field, enter a title, submit, then
  activate Open on a matching record. This is two query actions plus one record action. On phone,
  the first record starts 233 px below the viewport bottom, so the person must scroll before Open.
- **Open a Student Course:** activate Open Course on the visible Course row. From a non-filtered
  list, searching adds text entry and submit; the direct action does not require selection.
- **Inspect a Course as Sysadmin:** from the Sysadmin home, activate Find Courses, optionally enter
  a Course name/ID/Instructor query and submit, then activate Inspect. From Find Courses itself,
  the query plus Inspect takes three actions.
- **Change collection presentation or ordering:** activate the mode link, or open Sort/Filters and
  choose an option. These remain keyboard/touch-visible on phone; opening each disclosure currently
  requires scrolling past the page's search criteria.

## Evidence and provenance

Canonical selective capture images were copied to the ignored evidence directory:

`tests/_temp/datasheet/m03_spatial_baseline/canonical_initial_stage/`

The directory contains 20 WebP captures: 11 Instructor screenshots emitted before the Library
scenario stopped, 8 Student Course list/landing screenshots over laptop/tablet/phone/square, and
one Sysadmin home screenshot. The six direct baseline PNGs and machine geometry are under:

`tests/_temp/datasheet/m03_spatial_baseline/probe/`

The six probe contexts reached the actual app over the trusted local HTTPS gateway through
`choosePersona`, `enterInstructor`, `enterSysadmin`, and visible Student/Admin navigation helpers.
They recorded zero page errors. `geometry.json` stores the exact viewport, rendered theme, landmark
rectangles, first useful control, search-control geometry, first row position, loaded/intersecting
counts, and navigation landmarks. `evidence_receipt.json` records runtime provenance, exits, and
SHA-256 hashes for all 20 WebP and six PNG captures plus the geometry file.

The canonical entry command was:

```bash
source source_me.sh && ./devel/capture_screenshots.sh --only instructor_library,instructor_seeded,student_courses_navigation,sysadmin_courses
```

Before starting, the scoped status probe reported that the private `workspace/env.local` did not
exist; the browser lock was not held and no developer-control receipt existed. The canonical
capture wrapper then created the clean disposable stack. After the Library scenario stopped, the
remaining manifest scenarios were run against that same live stack with:

```text
node --import tsx tests/playwright/capture_live_demo_screenshots.mjs --only <receipt-origin>/sign-in student_courses_navigation,sysadmin_courses
```

`<receipt-origin>` was read from the private `developer-control.json` receipt; the run exited 0.
The temporary geometry command was:

```text
node --import tsx tests/_temp/datasheet/m03_spatial_baseline/spatial_probe.mjs
```

It used the same private receipt-derived gateway CA and local authenticator path; it exited 0
after six role/viewport states.

It cold-started the disposable stack in 1467.6 seconds. The Instructor seeded scenario completed;
the Library scenario wrote its default, My Pools, filtered, and Browse images, then exited 1 when
the existing helper timed out at `scenarios_instructor.ts:204` waiting for a `Multiple choice`
button after Browse. No product code or test helper was edited during capture recovery. Student
and Sysadmin were then captured directly with the existing manifest runner against the same
running stack; both scenarios
completed with exit 0 and their ordinary privacy checks passed. A direct probe invocation without
the approved elevation failed at macOS Chromium `bootstrap_check_in` with permission denied; the
same local probe completed after using the task-authorized escalation path.

Runtime provenance captured before source-only work was released:

- `src/` plus `assets/`: 595 files; deterministic tree SHA-256
  `51df2f8a909c812d409dc811bcf16ec2f7d94603e8a70649b53c82df98a340f8`.
- Built `dist/`: 118 files; deterministic tree SHA-256
  `cd0b1ae6e2154b18a61345112f437ff8e3d86bf83d3b5c25887bf47220c230ad`.
- `dist/index.html` SHA-256
  `2b78cb19b70779475a0a6a47d11cf9ecd79cd498ed919f3bd862e95f32b73087`.
- API application image ID: `2dc9c1c99f566effdce2daf65177c1f8a8cf28469d397637034dfe66b66ac545`,
  `localhost/peptidyle-learning-engine:local`.
- Database migrator image ID after its first-use build:
  `9fdc75134d9a32b49001fb5cb167f107ea726fbc060c002c7ff9891d34320a40`,
  `localhost/peptidyle-learning-engine:local-database-migrator`. The pre-start tag pointed to an
  older image and was replaced by the canonical first-use build before schema migration completed.
- The `dist/` tree hash was unchanged at probe completion. Live Demo status reported `ready`, with
  10 healthy required services; the stack was left running for the next tasks.

## Deviations and handoff

The first read-only inspection accidentally included `git status --short --ignored` and reported
only that `test-results/` is ignored. No further Git command was run. No tracked screenshot,
manifest, receipt, atlas, plan, status, changelog, source, or permanent test was edited.

The direct probe's requested Dark scheme did not override the Instructor's actual Light rendering;
Student and Sysadmin narrow captures did render Dark. Treat this as observed theme behavior, not a
pixel comparison gate. The official browser-suite lock remains held by the Live Demo supervisor
while the stack stays up; M03 browser use is complete and no further runtime, browser, start, or
reset work is in progress.
