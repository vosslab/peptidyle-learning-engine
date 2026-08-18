# Codebase and interaction review: evidence register

Supporting register for
[codebase_and_interaction_review.md](codebase_and_interaction_review.md). That document carries the
decisions; this one carries every finding with its citations, evidence class, and disposition.

Review date: 2026-08-18. Tree identity at review:
`bfdbdd7f5d597adf0aa5c2108785ca9a22cfb7b3`.

## How to read this register

Evidence classes state what a finding rests on, and each class carries different weight.

| Class | Rests on | Carries | Defers |
| --- | --- | --- | --- |
| Source | Reading current source | Structure, wiring, route and role declarations | Runtime behavior |
| Corpus | The committed screenshot corpus | Rendered layout and copy at capture time | Anything the corpus is too old to show |
| Tooling | A command run during this review | What that command reported | Anything it did not execute |

Dispositions: **confirmed** in current source or by a command run here; **unresolved** with the
missing observation named; **retired** when evidence withdrew it.

The review started no application stack and ran no browser capture, so every corpus-dependent
finding is unresolved rather than confirmed. The tooling built during the review measured corpus
ownership and staleness, and those results are recorded as confirmed.

## Access and authorization

| ID | Finding | Class | Disposition |
| --- | --- | --- | --- |
| SEC-1 | Catalog read routes carry no role requirement | Source | Confirmed |
| SEC-2 | Student reach across routes has no single declaration | Source | Confirmed |

**SEC-1.** `crates/server/src/catalog/routes.rs:25-53` mounts `/api/problems`,
`/api/problems/search`, `/api/problems/by-id/{reference}`, `/api/problems/by-id/{reference}/detail`,
and `/api/taxonomy` with no role layer, and `crates/server/src/catalog/query.rs` contains no
`UserRole` check. Authoring, roster, appearance, export, QTI, and course policy all check
`Instructor | Sysadmin` (`crates/server/src/course/policy.rs:71`,
`crates/server/src/course_appearance.rs:573`, `crates/server/src/flat_question_publication.rs:854`).
In the browser, `src/app.tsx:176-178` renders the Library nav link with no guard while Workspace at
`:179` is guarded by `canUseAuthoringTools`, and `src/pages/library_route_page.tsx` adds no gate.
The owner states the Library serves manually vetted Instructor accounts only, so a Student session
reading the published catalog is outside the intended boundary. Answer keys stay server-side; the
exposure is prompts and catalog membership.

Missing observation: the runtime status codes a Student session actually receives. The review read
routes and handlers and started no stack.

**SEC-2.** `src/route_contract.ts:29-101` declares 19 routes. Nine are instructor-only by intent
(`library`, `problemDetail`, `workspaceList`, `workspaceEditor`, `assignmentCreate`,
`assignmentEditor`, `gradebook`, `courseAppearance`, `courseRoster`). Gating is applied per page:
Workspace uses `mayAuthorWorkspace` (`src/pages/editor_live_pages.tsx:28-32`), the assignment editor
uses a course gate, and Library uses none. No single place answers what a Student may reach.
`src/app.tsx:29-36` shows the same shape in the authentication direction, hard-coding four public
pathnames rather than deriving them from the route contract.

## Student interaction

| ID | Finding | Class | Disposition |
| --- | --- | --- | --- |
| STU-1 | No time limit shown before a timed run starts | Source | Confirmed |
| STU-2 | No learner mastery view | Source | Confirmed |
| STU-3 | Implementation vocabulary shown to students | Source | Confirmed |
| STU-4 | Three verbs enter the same practice loop | Source | Confirmed |
| STU-5 | Student assignment rows carry no state | Source | Confirmed |
| STU-6 | Completeness copy leaks pagination and breaks grammar | Source | Confirmed |

**STU-1.** `src/pages/assignment_overview_page.tsx:66-85` renders exactly three facts: questions per
run, variation, and feedback. The countdown appears on `run_page` after the attempt is issued. A
student commits to a timed run without being told its limit. WP-HG1.T made whole-run timing
course-owned and visible during the run; the decision point is one screen earlier.

**STU-2.** `src/pages/` holds no learner analogue of `gradebook_page`. The data exists:
`student_assignment_summary` supplies best, latest, completed count, and last activity to the
instructor gradebook. `docs/MASTERY_ASSIGNMENT_DESIGN.md:23` promises completion "does not erase the
opportunity to learn more", and `crates/domain/tests/run_31.rs` makes repeated post-completion
practice a permanent contract. The learner cannot see their own trajectory, so the product's central
promise is unobservable by the person it is made to.

**STU-3.** `src/pages/assignment_overview_page.tsx:79` renders "Each newly issued attempt receives a
fresh seed." as a bold fact value; `:83` renders "Released according to the assignment policy",
which does not tell a student whether they will see answers. `docs/HUMAN_GUIDANCE.md:507-509` asks
for plain capability names before implementation jargon.

**STU-4.** `src/pages/course_assignments_page.tsx:161` "Review assignment";
`src/pages/assignment_overview_page.tsx:109` "Start or resume practice";
`src/pages/run_page.tsx:439` "Start another practice run". "Review assignment" is the first label a
student meets and names a different action than starting practice. The same label serves the
instructor's course assignment list, where the intent is editing. The label is also written into
`docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md:91`, so changing it is a contract edit.

**STU-5.** The row shows title and question count only. Comparison evidence:
`OTHER_REPOS/adapt/resources/js/pages/students/assignments.index.vue:364-415` carries points
received, points possible, status, and submitted date, with filters by group and status. ADAPT also
carries a z-score, which is more than a mastery product needs. PLE currently carries less than a
student needs to choose what to work on.

**STU-6.** `src/pages/course_assignments_page.tsx:77`, `src/pages/course_list_page.tsx:94`, and
`src/pages/gradebook_page.tsx:109,125` render "All N ... are shown."; with one item this reads
"All 1 assignments are shown." `docs/UI_DESIGN_GUIDE.md:25-26` already asks that confirmations the
visible content proves be left out.

## Instructor interaction

| ID | Finding | Class | Disposition |
| --- | --- | --- | --- |
| INS-1 | Item analysis is implemented with no interface | Source | Confirmed |
| INS-2 | Gradebook is a flat cross-product list | Source | Confirmed |
| INS-3 | Four surfaces render a constant where a fact belongs | Source | Confirmed |
| INS-4 | Library cannot show a question's family | Source | Confirmed |
| INS-5 | Assignment editor has no direct position control | Source | Confirmed |
| INS-6 | Terminology forks four ways for people, two for content | Source | Confirmed |
| INS-7 | A development-only capability appears in shared copy | Source | Confirmed |

**INS-1.** `crates/server/src/item_analysis.rs`, its worker
(`crates/server/src/composition/worker.rs:15`), router mount
(`crates/server/src/composition/router.rs:208`), both Store backends, and live PostgreSQL tests
(`crates/learning-data-access/tests/postgres_item_analysis_live.rs`) all exist. No reference appears
under `src/` or `generated/`. The instructor cannot see which questions the class is missing.

**INS-2.** `src/pages/gradebook_page.tsx` renders one row per (assignment, learner) pair with cursor
pagination, no sort, no filter, and no export control on the page. Thirty learners across ten
assignments is 300 undifferentiated rows. Comparison evidence:
`OTHER_REPOS/adapt/resources/js/pages/instructors/gradebook.index.vue:940-963` uses a
learner-by-assignment matrix with sticky name and email columns and per-assignment aggregates, and
`:714` supplies CSV export. PLE's mastery cell content (best, latest, completed runs) is richer than
ADAPT's single score and is worth preserving inside a matrix shape.

**INS-3.** `src/pages/course_list_page.tsx:29` renders "Review the current assignment or resume an
in-progress practice run." under every instructor course card, in the student's voice. The library
row summary, the workspace draft subtitle, and the question-detail lead show the same pattern of a
constant occupying the slot where a distinguishing fact belongs.

**INS-4.** `src/pages/library_page.tsx:211-219` renders title, summary, taxonomy, ID, and a link.
With eight response families shipping under WP-RC4 and WP-RC5, an instructor cannot tell MC from
MATCH from HOTSPOT without opening each question. Filter labels at `:150,166` read "Evidence" and
"Capability", which are internal terms.

**INS-5.** `src/pages/assignment_editor_page.tsx:641-651` provides "Move earlier" and "Move later"
only. `docs/UI_DESIGN_GUIDE.md:90-92` asks for directional controls **and** a direct position
selector, and `docs/UI_DESIGN_REVIEW.md:19` states direct position controls are implemented. Moving
item twelve to position one costs eleven activations.

**INS-6.** People appear as "Students" in navigation and headings, "learners" in body copy, "course
members" in a section heading, and "roster ID" in data. Content appears as "question" in the newer
pages and as "problem" in `src/route_contract.ts:64-69` (`problemRef`, "Shared problem browser") and
in older surfaces.

**INS-7.** `src/pages/course_roster_page.tsx:487` ships "Review active students, add configured
local learners when available, and export grades." to instructors. "Configured local learners" is
the local-file development adapter appearing in shared copy.

## Evidence integrity

| ID | Finding | Class | Disposition |
| --- | --- | --- | --- |
| EVD-1 | The corpus had two owners and no shared declaration | Tooling | Confirmed and repaired |
| EVD-2 | One committed image is owned by neither pipeline | Tooling | Confirmed |
| EVD-3 | The entire corpus predates current browser sources | Tooling | Confirmed |
| EVD-4 | No tablet artifact exists for any student surface | Tooling | Confirmed |
| EVD-5 | README ships images showing replaced identity schemes | Corpus | Confirmed |
| EVD-6 | A design review cites an image for the opposite of what it shows | Corpus | Confirmed |
| EVD-7 | The palette audit measures colors its source has replaced | Source | Confirmed |
| EVD-8 | The two public guides name one assignment differently | Source | Confirmed |
| EVD-9 | A random run suffix appears in a course name in public evidence | Corpus | Confirmed |

**EVD-1.** `tests/playwright/capture_docs_screenshots.mjs:9-21` held 11 image names from the
real-stack walkthrough and `capture_instructor_page_visuals.mjs:9-23` held 13 from a mock-backed
spec, with nothing reconciling them. Three pages existed in both sets under different names.
Repaired during this review: `tests/playwright/ui_corpus_manifest.ts` is now the single declaration
and both scripts read it.

**EVD-2.** `docs/screenshots/peptide_bond_mastery_overview.png` is committed, produced by neither
capture script, and referenced by no Markdown file. `node tests/playwright/verify_ui_corpus.mjs`
reports it as committed without a manifest owner.

**EVD-3.** `node tests/playwright/verify_ui_corpus.mjs --bootstrap-provenance` reports every one of
the 24 declared artifacts as captured before the current browser sources: the 13 mock artifacts sit
one commit behind `src`, and the 11 live artifacts sit three commits behind. A spike run during this
review measured `src`, `src/style.css`, `src/pages`, `src/components`, and `src/features` and found
they share one last-change commit, because this repository lands large batched commits, so narrowing
the owning path adds no discrimination. Staleness is therefore reported as a commit count rather
than enforced as a gate.

This retires the reading that the mock set was current and only the live set was stale. Both are
behind; the live set is further behind.

**EVD-4.** `surfacesMissingExpectedViewports()` reports six student surfaces with no 800 by 1280
artifact. `docs/UI_DESIGN_GUIDE.md` named student pages at that viewport as canonical evidence
before this review adjusted the wording, so the tablet evidence has been absent throughout.

**EVD-5.** `README.md` embeds `instructor_assignment_settings.png`,
`instructor_assignment_created.png`, `student_timed_problem.png`, `student_fresh_practice.png`, and
`student_retake_fresh_problem.png`. All five come from the live set. `instructor_assignment_settings.png`
displays `P-2-v1` through `P-5-v1` under the heading "Selected published versions", and its README
alt text reads "four selected Genetics Chapter 1 immutable versions".
`docs/HUMAN_GUIDANCE.md:225-231` establishes the `AAA-BBBB` Question ID as the sole durable question
identity and removes sequential and version-chain identity; `crates/question_model/src/catalog.rs:636`
keeps `P-123456` and `P-12-v3` only as rejected inputs.
`instructor_gradebook_mastery_loop.png` shows a "LEARNER ID" column of raw UUIDs against
`docs/HUMAN_GUIDANCE.md:206-207`.

**EVD-6.** `docs/UI_DESIGN_REVIEW.md:24` cites `student_timed_problem.png` as evidence that the
student surface "keeps timer, prompt, figures, response, status, and actions in one visual
sequence". That image shows a two-column prompt and response split, a "Question content ready."
status line, and a slogan footer band. Current source shows a single centred column
(`src/style.css:617-627`, `.question-card` used at `src/pages/run_page.tsx:450`), and the status
string and footer element are absent from `src/`.

**EVD-7.** `docs/PALETTE_CONTRAST_AUDIT.md:12-23` attributes seven color literals to `src/style.css`.
None appear there now; only `#172033` survives, in
`src/features/course_appearance/theme_catalog.ts`. Contrast is a stated acceptance target
(`docs/UI_DESIGN_GUIDE.md:117-120`), so the audit is not currently evidence for it.

**EVD-8.** `docs/INSTRUCTOR_GUIDE.md:22,24` names "Genetics Chapter 1 Practice";
`docs/STUDENT_GUIDE.md:13,15` names "Genetics Chapter 1 Mastery", for the loop they cross-reference
at `docs/STUDENT_GUIDE.md:64-66`.

**EVD-9.** `student_assignment_list.png` and `instructor_course_overview.png` title the course "Fake
Genetics Course qorg6t".

## Documentation and contract drift

| ID | Finding | Class | Disposition |
| --- | --- | --- | --- |
| DOC-1 | Reserved migration versions are already overtaken | Source | Confirmed |
| DOC-2 | Retention authority is stated three ways | Source | Confirmed |
| DOC-3 | Eleven migrations appear in no schema document | Source | Confirmed |
| DOC-4 | Migration counts disagree four ways | Source | Confirmed |
| DOC-5 | A removed flat-question reader is documented as live | Source | Confirmed |
| DOC-6 | Accepted identity work is described as pending | Source | Confirmed |
| DOC-7 | Container queries are documented and unused | Source | Confirmed |

**DOC-1.** `docs/DATABASE_STRUCTURE.md:90-92` reserves `2026080910`, `2026080911`, and `2026080912`
as "no migration file yet", restated in
[release_completion_plan.md](../active/release_completion_plan.md). `schemas/migrations/` already
holds `2026080914` through `2026080935` plus `2026081401`, `2026081501`, and `2026081502`. Adding a
reserved lower version later places it beneath twenty-two applied higher versions. The cluster is
rebuilt pre-production so nothing breaks today, and the stated policy would produce an out-of-order
insert if followed.

**DOC-2.** `docs/API_CONTRACTS.md:91` states retention control is Instructor-only.
`docs/RETENTION_POLICY.md:38-40` states Instructor or Sysadmin, with extension Sysadmin-only.
`crates/server/src/retention/access.rs:27,40` implements Instructor or Sysadmin.
`docs/API_CONTRACTS.md` is the outlier and is the document consulted when auditing this boundary.

**DOC-3.** `2026080923` through `2026080928`, `2026081401`, `2026081501`, and `2026081502` are absent
from `docs/DATABASE_STRUCTURE.md:41-49,86-108` and `docs/DATABASE_TENANCY.md:300-317`.
`docs/USER_ROLES.md:124-127` names `2026080928_user_roles.sql` as the canonical role migration.

**DOC-4.** `schemas/migrations/` holds 34 files. `docs/ROADMAP.md:8` and `docs/TODO.md:7` say 28;
`docs/DATABASE_STRUCTURE.md:31` says 31; `docs/active_plans/implementation_status.md:71` says 32 and
`:560` says 18.

**DOC-5.** `docs/QUESTION_MODEL.md:315-317` describes the v1 `singleChoice` reader as the immutable
compatibility source. `docs/HUMAN_GUIDANCE.md:340` asks that it not be retained,
`docs/active_plans/implementation_status.md:588-589` records its removal, and
`crates/adapters/native/src/flat_question/` holds only `imported.rs`, `tests.rs`, and `v2.rs`.
Residue also appears at `docs/QTI-JSON_OBJECT_FORMAT.md:264,276-277`.

**DOC-6.** `docs/QUESTION_ID_SPEC.md:181-193` instructs a manager to identify everything still
treating sequential or versioned IDs as human-facing. WP-R2 completed that removal.

**DOC-7.** `docs/HUMAN_GUIDANCE.md:28-29` and `docs/UI_DESIGN_GUIDE.md:57-60` state that layout
adapts with media and container queries. `@container`, `container-type`, and `container-name` return
no matches across `src/`.

## Engineering structure

| ID | Finding | Class | Disposition |
| --- | --- | --- | --- |
| ENG-1 | No SQL is compile-time checked | Source | Confirmed |
| ENG-2 | `Store` is a delegating facade over segregated traits | Source | Confirmed |
| ENG-3 | One adapter reaches persistence for a single type | Source | Confirmed |
| ENG-4 | Two types are duplicated across crate boundaries | Source | Confirmed |
| ENG-5 | The line cap reshaped the module system | Source | Confirmed |
| ENG-6 | The mock API client ships in the production bundle | Source | Confirmed |
| ENG-7 | A second HTTP transport bypasses the shared client | Source | Confirmed |
| ENG-8 | CSS-in-TS carries colors outside the token system | Source | Confirmed |
| ENG-9 | Increased contrast overrides inline theme styles | Source | Confirmed |
| ENG-10 | A pre-existing type error keeps a repository gate red | Tooling | Confirmed |

**ENG-1.** No `sqlx::query!` use appears workspace-wide although the `macros` feature is enabled
(`Cargo.toml:100-109`). Roughly 450 queries are inline strings with hand row decoding through
`crates/learning-data-access/src/postgres/row_decode.rs`; `postgres/jobs.rs` holds 38,
`submission.rs` 28, `assignment_records.rs` 28. Schema drift surfaces only in the ignored live
suites, which `check_rust.sh:25-26` states it does not run.

**ENG-2.** `crates/learning-data-access/src/contracts/store.rs:151-164` aggregates eleven capability
traits and re-declares each method as a default forwarding to an `_impl` twin, 82 `async fn` in one
945-line file. The segregated traits in `contracts/store_capabilities.rs:7-627` are the stronger
design.

**ENG-3.** `crates/adapters/imathas/Cargo.toml:21` depends on `learning-data-access` so
`crates/adapters/imathas/src/lib.rs:53` can name `PublishedSourceArtifact`. No other adapter reaches
persistence.

**ENG-4.** `WebworkReplayMappingV1` exists in both `adapter_webwork::renderer_contract` and
`learning_data_access`, with variant-by-variant conversion at
`crates/server/src/webwork_backend/replay_mapping.rs:17-96`. `PersistedCorrelation` is `(String)` at
`crates/adapters/imathas/src/lib/grade.rs:79` and `(Vec<u8>)` at
`crates/learning-data-access/src/external_tool.rs:81`.

**ENG-5.** 87 Rust files exceed 700 lines and 41 sit between 900 and 999. The workspace carries 137
`#[path]` attributes and 146 files opening `use super::*;`. Test modules are included at the bottom
of implementation files, which required `#[allow(clippy::items_after_test_module)]`
(`crates/server/src/qti_import.rs:387`). On the browser side six files sit within 2% of the cap,
including `src/style.css` at 980 and
`src/features/flat_question_authoring/flat_question_editor_page.tsx` at 983.

**ENG-6.** `src/main.tsx:10` statically imports `createMockApiClient` and `:31-32` selects it when
`window.__PLE_USE_MOCK_API__` is true. A live branch on a static import is not removed by bundling,
so the 898-line fixture client and its fixtures link into the shipped bundle.
`pipeline/build.mjs:63-67` already demonstrates the build-time alias swap used for local development
authentication.

**ENG-7.** `src/features/flat_question_authoring/flat_question_client.ts:78`,
`flat_question_asset_client.ts:78`, and
`src/features/qti_profile_import/qti_profile_import_client.ts:80` call `globalThis.fetch` directly
with their own error classes and size caps. The base-path validation, no-store policy, ETag
handling, and response bounds in `src/api/http_client/request.ts:77-138` cover the rest of the
application.

**ENG-8.** `src/components/question_renderer_styles.ts:5,11,14,15` defines `#344054`, `#101828`,
`#98a2b3`, and `#b42318` beside `var(--ple-radius-control, ...)`, and
`src/pages/assignment_editor_styles.ts:235` hard-codes `#237447`, the value of `--ple-success`.
These cannot respond to increased contrast or forced colors.

**ENG-9.** `src/features/course_appearance/course_theme_scope.tsx:100-107` writes the palette into an
inline `style` attribute, so `src/style.css:860-866` re-declares five tokens with `!important`
inside `.course-theme-scope` to win the cascade. It is the only `!important` used for color.

**ENG-10.** `npx tsc --noEmit -p tsconfig.lint.json` fails at
`tests/playwright/roster_ui_accessibility.spec.ts(137,31)` with
`Type '"revoked"' is not assignable to type '"active"'`. The file is unmodified from HEAD
(`git show HEAD:` confirms the same source), so `check_codebase.sh` step 2 is red on the committed
tree independently of this review.

## Boundaries that are working

Recorded so the recommendation is read against an accurate picture of the system.

- The answer-key boundary is a compile-time control. `tests/test_crate_boundaries.py:220-224` asserts
  the Wasm closure is exactly `{wasm_bridge, domain, question_model}`; `grading` appears in neither
  `crates/wasm/Cargo.toml` nor `crates/export/Cargo.toml`; `:228-257` proves `FeedbackContent` lacks
  `Debug`, `Serialize`, and `Deserialize` and is absent from four browser-facing files.
  `src/wasm/index.ts:219-221` adds a browser-side denylist and
  `src/pages/editor_page_typecheck.ts:12-14` a compile-time proof.
- Publication immutability is enforced in PostgreSQL. Six catalog tables carry triggers raising
  `published catalog content is immutable`
  (`schemas/migrations/2026080802_catalog_authoring.sql:1322-1327,1547-1569`).
- `TenantContext` has one production constructor and no `Default`
  (`crates/learning-data-access/src/rls.rs:9-27`); transactions set `ple_app` then the tenant GUC
  (`crates/learning-data-access/src/postgres.rs:330-350`); eleven roles are `NOBYPASSRLS` with three
  narrow brokers excepted (`schemas/migrations/2026080801_principals.sql:3-50`).
- The `AAA-BBBB` identity is implemented as specified, including the HMAC check character taken from
  the high five bits of digest byte zero
  (`crates/learning-data-access/src/question_id.rs:76-80`), with the capacity cap metered at
  `schemas/migrations/2026080931_question_id.sql:22-26`.
- Browser route references are branded with a `unique symbol` and resolve to internal identifiers at
  exactly four functions (`src/navigation/public_route.ts:25-48`,
  `src/navigation/resolved_route.ts:23-58`).
- API decoding is closed-world: 179 `requireOnlyFields` sites
  (`src/api/decoders/shared.ts:92-102`), null-prototype dictionaries rejecting prototype keys
  (`src/api/decoder.ts:128-145`), and length-bounded arrays.
- Memory and PostgreSQL share one conformance suite
  (`crates/learning-data-access/tests/conformance/`, 30 modules reused by the live suites).
- Accessibility descriptions are enforced by raising rather than by convention
  (`src/components/question_renderer.tsx:208-218`).
- TypeScript discipline across roughly 35,700 lines: no `any` annotations, no `createStore`, two
  non-null assertions, two deliberate `@ts-expect-error` proofs, one `eslint-disable`, under
  `noUncheckedIndexedAccess`.
- Rust carries no TODO, FIXME, or HACK markers, no `unwrap` or `expect` on a production path, and no
  `anyhow` in a library crate.

## Findings awaiting fresh visual evidence

These were observed in the committed corpus. EVD-3 establishes that the whole corpus predates
current browser sources, so each is **unresolved** until a regenerated corpus is inspected. The
missing observation is the same for all of them: a capture of the current interface at the viewport
named.

| ID | Observation in the corpus | Contract it would breach |
| --- | --- | --- |
| VIS-1 | A dark ring surrounds the whole page content region in three images | `docs/HUMAN_GUIDANCE.md:59-61` |
| VIS-2 | The theme chooser shows three small swatches per theme | `docs/UI_DESIGN_GUIDE.md:121-123` |
| VIS-3 | An empty state is a dashed placeholder box | `docs/UI_DESIGN_GUIDE.md:102-104` |
| VIS-4 | Page titles wrap to two lines on three surfaces | `docs/UI_DESIGN_GUIDE.md:22` |
| VIS-5 | Instructor pages leave 40 to 60 percent of the canvas unused | `docs/HUMAN_GUIDANCE.md:44-47` |
| VIS-6 | Course home renders unthemed while sibling course pages are themed | `docs/UI_DESIGN_GUIDE.md:106-115` |
| VIS-7 | The workspace policy chip clips and reads "immediateFull feedback" | Presentation defect |
| VIS-8 | The published question detail renders `{{residue}}` unsubstituted | `docs/HUMAN_GUIDANCE.md:507-509` |
| VIS-9 | Create-assignment shows two phrasings of one requirement | `docs/UI_DESIGN_GUIDE.md:25-26` |
| VIS-10 | Run policies double-label four fieldsets | `docs/UI_DESIGN_GUIDE.md:96-98` |
| VIS-11 | Single-line inputs stretch to roughly 1220 pixels | `docs/HUMAN_GUIDANCE.md:44-47` |

VIS-8 also has a source component: the template placeholder reaches a rendered instructor surface,
which is visible in the corpus and worth confirming against current source during recapture.

## Tooling produced during this review

- `tests/playwright/ui_corpus_manifest.ts` declares 24 surfaces with role, pipeline, live-capture
  reason, and evidence purpose, and both capture scripts now read it.
- `tests/playwright/ui_corpus_provenance.mjs` records the capture commit per artifact and reports
  ownership gaps and staleness.
- `tests/playwright/verify_ui_corpus.mjs` runs the reconciliation and prints the summary quoted in
  EVD-2, EVD-3, and EVD-4.

## Work not performed

Named so no reader mistakes absence for a clean result.

- No application stack was started, so SEC-1 has no runtime confirmation.
- No browser capture was run, so every VIS finding is unresolved and the corpus remains stale.
- `./check_rust.sh`, `./check_codebase.sh`, `pytest tests/`, and
  `local_stack.py acceptance` were not run to completion; only the document, ASCII, size, naming,
  TypeScript, ESLint, and Prettier checks touching the new files were executed.
- No screen-reader or human accessibility session was held.
  `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md:281-286` already requires VoiceOver and NVDA walkthroughs
  before accessibility is claimed for the Fall pilot, and `:277-279` names the families still resting
  on component evidence.
