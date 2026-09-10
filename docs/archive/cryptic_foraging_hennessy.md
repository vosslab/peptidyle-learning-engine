# Plan: Restore Course Appearance end to end

## Context

**Course Appearance is one capability with two properties: theme and banner.** That is how
`docs/HUMAN_GUIDANCE.md:170` states it -- "An **Instructor** can upload a small centered course banner
and select a three-color theme" -- and how `docs/TERMINOLOGY_CONTRACT.md:1175-1177` defines it.
Neither property works today.

The two store differently: a theme is a scalar on a row, a banner is an asset with an upload
lifecycle. They are two workstreams running concurrently, sharing one page, one read model, one
authorization predicate, one Ribbon entry, and one end-to-end scenario. The storage difference stays
internal.

**Authority order**, per the owner: `docs/HUMAN_GUIDANCE.md` first, `docs/TERMINOLOGY_CONTRACT.md`
second. `docs/CONTRACTS.md` is **not authoritative** -- the owner has never read it, its appearance
section is agent-authored, and working code wins wherever they disagree.

Why this is not "put the picker back": the editor was deleted on 2026-08-31
(`docs/CHANGELOG-2026-08d.md:20`, commit `bb227b10`) because nothing persisted behind it. Restoring
the interface without the store recreates the state that was removed.

Confirmed gaps:

- **The read path is not backed.** `src/api/http_client/response.ts:150-167` fetches
  `/api/courses/{id}/appearance`. `grep -rn -i appearance crates/server/src/` returns **zero hits**.
- **Neither property persists.** No theme column in `schemas/migrations/`; no banner record. The
  object-storage plumbing does exist -- `ObjectDataClass::CourseAppearance`
  (`crates/objects/src/bucket.rs:58,165-180`), the `ple_data.course_banner` and
  `course_banner_delivery` tables
  (`schemas/migrations/2026082928_object_delivery_storage_checks_and_cleanup.sql:21,38,90-99`), and
  the `course_appearance_cross_store` E2E owner. What is missing is everything above them.
- **The write decoders have no callers** (`src/api/decoders/course_appearance.ts:90-147`).
- **The 15 theme IDs are hand-maintained in four places**:
  `crates/question_model/src/course_appearance.rs:23-55`, `generated/api/CourseTheme.ts`,
  `src/api/decoders/course_appearance.ts:20-36`, `tests/test_course_theme_scope.mjs`.
- **A 299-line stylesheet is orphaned** (`src/features/course_appearance/course_appearance_styles.ts`).

Reused, not rebuilt: the `CourseTheme` union; the 15-palette registry
(`src/features/course_appearance/course_theme_registry.ts`); DOM application via
`CourseThemeVariables` (`src/application_shell.tsx:141`); the preserved live-preview hook
`useCourseThemePresentation()`, kept with no caller
(`docs/ux/RIBBON_RETIREMENT_RESPONSIBILITY_INVENTORY.md:32`); the contrast oracle
`tests/test_course_theme_scope.mjs`; the route and role-gate shapes in
`crates/server/src/assignment_release.rs`; the object-storage primitives above; and the Ribbon catalog
entry plus `palette` glyph at `src/ribbon/ribbon_catalog.ts:366-377` and
`src/ribbon/ribbon_icons.ts:25,51`.

## Objectives

- Let an Instructor select a course theme and upload, replace, or remove a course banner.
- Make the appearance read path truthful by serving the route the browser already calls.
- Persist both properties so choices survive reload and reach every course member.
- Correct the documentation whose claims this implementation falsifies.

## Design philosophy

**Close the gap at the persistence layer first; add the interface last.** The visible half is the
small half. This is _fix the design, not the symptom_: the symptom is a missing page; the fault is a
capability with no store.

**One plan, two workstreams -- not two plans.** An earlier draft split banner into a companion plan.
The parallelism that appears to buy is already supplied by workstreams; what a split adds is a second
dispatch cycle, a second closure, and every shared surface opened twice. It also ships a capability
named Course Appearance whose banner half is knowingly absent -- the same transitional half-surface
that got the original editor deleted. **Theme still reaches users early**, because the theme lane
never depends on the banner lane, under one rule: _the page never renders a control that does
nothing._ An unfinished Banner section is absent, not disabled.

**Theme persistence is one column, with no revision.** Earlier drafts carried a separate appearance
relation, a current-revision pointer, and compare-and-swap. All three were removed on evidence: no
current-pointer pattern exists in this repository (it appears only in `CONTRACTS.md`), and Course
Instance has no revision to reuse (`crates/learning-data-access/src/course_instance.rs:51-61`;
`blueprint_revision` at `:23` is unrelated). The domain concept is the course's _current_ appearance
-- no history, each save replaces the previous value, so there is no prior value to protect and
last-write-wins is sufficient. Theme is therefore `course_theme NOT NULL DEFAULT 'grass'` on the
Course Instance row: no backfill, no creation-path insert, no missing-row handling, no invariant to
police. If concurrency ever becomes a real requirement, a revision column can be added behind the
same route pair, copying `assignment_release.rs`.

**The appearance read model is the aggregation boundary.** `/api/courses/{id}/appearance` answers one
question -- what does this course look like -- and no consumer learns which store each half came from.

**Theme and banner save independently.** Reading is aggregated; writing is not. Each property has its
own mutation path against its own store, and each section of the page saves on its own. A single
combined save would have to span a database column and an object store atomically -- reintroducing
exactly the coupling this design avoids -- to buy nothing an Instructor asked for. The consequence
that matters: **the Theme section is complete and shippable before the Banner section exists**, which
is what lets themes reach users without waiting on banner work. Only the theme preview has an
abandonment invariant, because only theme has a live preview overriding the rendered page; a pending
banner file is just an unsaved form value.

**The existing update body must narrow.** `CourseAppearanceUpdate`
(`crates/question_model/src/course_appearance.rs:346-353`) bundles theme with a mandatory
`banner: CourseAppearanceUpdateAction` under `deny_unknown_fields`, forcing every theme change to
restate a banner action. It was designed for an architecture never built. Derive the smallest API
matching the real operations instead of preserving that shape by reflex.

**No human is on the critical path.** Every milestone completes with manager and subagents alone.

**Documentation correction is restoration work.** Agents build from these documents; ones describing
a deleted editor as accepted, or a server contract never implemented, keep misleading the next reader.

- Evidence strategy for uncertain methods: banner sizing, crop, upload shape, and API shape are
  settled by measurement in M3, with decision rules stated there.

## Scope

- Generate the theme vocabulary from one source before any code validates against it.
- Add `course_theme NOT NULL DEFAULT 'grass'` to the Course Instance row.
- Settle banner storage, sizing, and crop strategy from existing repository patterns.
- Serve one appearance read model plus theme and banner mutation paths, under one predicate.
- Build the Appearance page with Theme and Banner sections, live preview, and safe save behavior.
- Prove accessibility and cross-member propagation with automated browser evidence.
- Admit `appearance` through the full `docs/ux/FRONTEND_CAPABILITY_INTEGRATION.md` path.
- Correct the documentation this implementation falsifies.

## Non-goals

- Add appearance revisions, ETag preconditions, or conflict reconciliation. Removed on evidence.
- Retain appearance history. Current state only.
- Apply the lookalike-removal rule at `docs/HUMAN_GUIDANCE.md:65`. All 15 palettes retained.
- Repair `--ple-theme-primary` or the inert scope-style block. Theme-system debt, but neither blocks
  this capability and repairing the token activates dead styles. Recorded as a follow-up.
- Build a per-user theme preference, dark mode, or the "Increased contrast" account option.
- Add theme categories, rarity tiers, unlocks, currency, or purchases.
- Back any of the other 14 unbacked Ribbon destinations.
- Change Ribbon geometry, size tokens, or slot positions.
- Expose any appearance operation as a Wasm export.
- Accept Student uploads (`docs/HUMAN_GUIDANCE.md:68`).
- Restructure `docs/CONTRACTS.md` beyond its appearance section.

## Reference evidence: how Blackboard Ultra does it

Four screenshots of the owner's own live courses -- the closest thing to a requirements document this
feature has, because they show the workflow he already uses.

- **Full-bleed and wide**, roughly 5.5:1, spanning the content width. This contradicts the
  agent-authored "centered, normalized to 1200 by 328" at `docs/DESIGN_DECISIONS.md:1138-1140`; a
  fixed centered box letterboxed into that space would read as broken. **Aspect ratio and responsive
  fit are the contract, not a pixel size.**
- **One asset, several renditions.** The same upload serves a ~2.5:1 course-card thumbnail and the
  wide page hero. The banner is the course's identity wherever the course appears, so one uploaded
  source must serve every surface.
- **A dark scrim is composited along the bottom** so overlaid white text stays legible -- and it
  **damages instructor content**. Every one of these banners bakes the course title, number, and term
  into the artwork; in Biology and Ethics the baked "BCHM 483 -- Summer 2026" is half-swallowed, in
  Biotechnology "BIOL 480 -- Fall 2026" is dimmed.
- **Editing is in place**, via a pencil on the banner itself.

Three principles follow, carried into M3 and M7:

- **Preserve the instructor's artwork; present course identification separately.** Placing the
  identifier beneath the image removes the need for a scrim: no gradient, no damaged artwork, no
  contrast obligation against an unpredictable photograph.
- **Shorter, not narrower.** The owner's instinct that the banner should be smaller is supported, but
  the axis is height -- these heroes consume ~300px above the fold, the same budget
  `docs/active_plans/virtual-doodling-honey.md` is reclaiming from the Ribbon.
- **Show the renditions during editing.** A preview of how the upload crops at each surface is worth
  more than automatic image analysis, and lets the Instructor judge acceptability.

## Initial pre-implementation state summary

| Piece                                             | Status                                                  |
| ------------------------------------------------- | ------------------------------------------------------- |
| `CourseTheme` union, 15-palette registry          | Exist, contrast-gated at 5.5:1, reusable as-is          |
| `CourseAppearanceUpdate` / `CourseAppearanceView` | Mandate a banner action / a revision; both narrow in M2 |
| Course Instance record                            | No revision or edit number exists                       |
| Theme applied to the DOM                          | Wired, via `application_shell.tsx:141`                  |
| Live-preview hook                                 | Provider mounted, no production consumer                |
| Object storage for banners                        | Buckets, tables, cross-store E2E owner exist            |
| Appearance read over HTTP                         | Client half only; **no server handler**                 |
| Theme and banner persistence                      | Do not exist                                            |
| Appearance editor page                            | Deleted in `bb227b10`                                   |
| Ribbon Appearance task                            | Declared, `unbacked`, never rendered                    |

## Architecture boundaries and ownership

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                                                            | Review boundary                |
| ---------------------- | -------------------------------------------------------------------- | ------------------------------ |
| M1-M2, M5 / WS-T       | Generation pipeline, `schemas/migrations/`, `crates/question_model/` | Vocabulary and theme storage   |
| M3, M6 / WS-B          | `crates/objects/`, banner storage and routes                         | Asset lifecycle and validation |
| M4 / WS-S              | `crates/server/src/`                                                 | Read model and authorization   |
| M7-M8 / WS-S           | `src/api/`, `src/features/course_appearance/`, `src/routes.ts`       | Page behavior                  |
| M9-M11 / WS-S          | `tests/playwright/`, `src/ribbon/capability_registry.ts`             | Evidence and Ribbon admission  |

Shared-resource ownership: the theme migration is owned by M2, any banner schema by M6. The theme
vocabulary is owned by M1 and consumed unchanged. The read model, page, predicate, Ribbon entry, and
scenario are owned by WS-S; both property workstreams supply into them and neither edits them. M7 may
narrowly repair the existing `CourseThemeVariables` presentation setter to bind preview and release to
`CourseId` and preserve an unsaved theme across independent same-course banner cache writes. No
milestone touches `course_theme_scope_styles.ts`, theme tokens, selectors, or shell layout, so this
plan has **no dependency on `docs/active_plans/virtual-doodling-honey.md`**.

## Milestone plan

| M   | Title                                   | Summary                                                 | Goal                                        |
| --- | --------------------------------------- | ------------------------------------------------------- | ------------------------------------------- |
| M1  | Theme vocabulary single source          | Generate IDs from the Rust owner                        | Validation rests on one list, not four      |
| M2  | Course theme column and domain shape    | One column; narrow the view and update types            | A course owns its theme                     |
| M3  | Banner storage and sizing decision      | Investigate existing patterns; decide shape             | Banner rests on evidence, not invention     |
| M4  | Appearance read model and authorization | `GET` returning both; one course-scoped predicate       | One truthful answer, one access rule        |
| M5  | Theme write path                        | Route plus browser client method                        | An Instructor can save a theme              |
| M6  | Banner backend                          | Upload, validation, storage, set/replace/remove         | An Instructor's banner is stored and served |
| M7  | Appearance page and Theme section       | Page shell, theme selection, preview, save, abandonment | Themes reach users                          |
| M8  | Banner section                          | Upload, replace, remove, rendition previews, save       | Banners reach users                         |
| M9  | Automated accessibility evidence        | What this page introduces, not global invariants        | The gate passes without a human             |
| M10 | Ribbon admission                        | 14-step path, registry flip, ledger                     | The task renders and navigates              |
| M11 | Cross-member propagation evidence       | Instructor saves both, Student receives both            | The owner requirement is proven             |

Two lanes run from day one. The **theme lane** is M1, M2, M4, M5, M7. The **banner lane** is M3, M6,
M8. They converge at M4, which both need, and again at M9. M7 depends on M5 only -- **not on M6** --
so the Appearance page ships with a working Theme section while banner work continues. M8 adds the
Banner section to the existing page.

### Milestone: M1 theme vocabulary single source

- Depends on: none.
- First act: **use the repository's existing cross-language generation mechanism** -- the one already
  producing `generated/api/` -- if the theme vocabulary fits it. If eliminating the duplication would
  require building a new generation subsystem, stop and record that. The invariant worth having is one
  authoritative vocabulary for 15 stable identifiers; the implementation stays proportionate.
- Deliverables: theme IDs generated from `crates/question_model/src/course_appearance.rs:23-55`;
  `src/api/decoders/course_appearance.ts:20-36` and `tests/test_course_theme_scope.mjs` derive from it.
- Done check: adding a theme locally propagates from one edit, then reverted.
- Exit criteria: the palette oracle passes unchanged for all 15; no duplicate list remains.
- Parallel-plan ready: yes. Gates only M5.

### Milestone: M2 course theme column and domain shape

- Depends on: none.
- Deliverables: forward migration adding `course_theme NOT NULL DEFAULT 'grass'` to the Course
  Instance row; `CourseAppearanceView` narrowed to drop `revision`; the theme update contract derived
  as the smallest shape expressing "change the theme"; store read and write;
  `docs/DATABASE_STRUCTURE.md` updated.
- Done check: after migration every existing Course Instance reports theme `grass`; a newly created
  course does too, with no application code inserting it.
- Exit criteria: the invariant is owned by the column default, not application code; no revision
  column; `CourseAppearanceRevision` deleted if nothing else references it; the `database_baseline`
  oracle covers the column.
- Parallel-plan ready: yes.

### Milestone: M3 banner storage and sizing decision

- Depends on: none. Dispatches day one so banner research never delays the theme lane.
- Method: **investigate existing repository patterns first.** Named starting points:
  `crates/objects/src/bucket.rs:58,165-180`; the `ple_data.course_banner` and
  `course_banner_delivery` tables and their RLS at
  `schemas/migrations/2026082928_object_delivery_storage_checks_and_cleanup.sql:21,38,90-99`; the
  `course_appearance_cross_store` E2E owner; any existing upload or object-promotion path. Reuse
  them; do not invent a banner subsystem.
- Deliverables: a decision record under `docs/active_plans/decisions/` settling storage shape, aspect
  ratio and height, accepted formats, validation, crop strategy, and mutation API shape.
- Decision rules:
  - **Sizing** is measured, not inherited: render candidate heights at the laptop, tablet, phone, and
    square profiles the screenshot atlas already uses; recommend the shortest height that still reads
    as course identity. The 1200 by 328 figure is an unapproved agent proposal, not a constraint.
  - **Crop** is judged by whether the meaningful center region survives at every rendition; prefer
    showing the Instructor a preview per rendition over automatic image analysis.
  - **Upload shape** is whatever the existing object primitives support atomically. Two-phase only if
    they require it; otherwise one authenticated request.
  - **API shape** derives from the real operations -- set or replace, and remove.
- Done check: M6 and M8 are dispatchable from the record without further research.
- Exit criteria: **every implementation-blocking choice above is closed here**, from repository
  evidence and rendered measurement, with no open question left for a person. The no-human-critical-path
  rule applies to this milestone like every other: M3 picks the height, ratio, formats, validation,
  crop strategy, upload shape, and API shape, and records the evidence for each. The only thing
  deferred is **optional visual refinement** -- the owner may later prefer a different height, and
  changing one number is cheap -- which is explicitly not a gate on M6, M8, or closure.
- Parallel-plan ready: yes.

### Milestone: M4 appearance read model and authorization

- Depends on: M2 and M3, so the response shape is right the first time.
- First act: **identify the existing course-scoped membership oracle** and name it in the record.
  `assignment_release.rs:289-294` establishes only `ProductRole::Instructor`, which is not a
  course-membership rule. Named starting points: `crates/question_model/src/teaching_authority.rs`,
  `crates/learning-data-access/src/course_instance.rs`,
  `crates/learning-data-access/src/live_student_course_landing.rs:78`,
  `docs/DATABASE_AUTHORIZATION.md`. Adopt what those already use; do not invent one.
- Deliverables: `GET /api/courses/{id}/appearance` returning theme and current banner as one response
  wrapped in `no_store`; the predicate applied to every appearance operation, read and write; the
  browser client's ETag-equals-revision assertion at `response.ts:145-149` removed with the revision
  it checked.
- Two predicates, stated explicitly so they cannot be conflated:
  - **Read**: any member of the course, Instructor or enrolled Student. Students must read appearance
    -- the theme renders for them, which is the whole point of the capability.
  - **Write**: Instructor course membership only.
  - **Concealed for both**: anonymous callers and anyone with no membership in that course, including
    a foreign Instructor. Concealment is identical across those cases so membership is not probeable.
- Done check: the browser read path succeeds against the running stack for an Instructor _and_ for an
  enrolled Student; a course with no banner returns the absent-banner case cleanly; an anonymous
  caller and a foreign Instructor are concealed identically; a Student write is refused.
- Exit criteria: consumers cannot tell the two halves come from different stores; no ETag, because no
  revision exists to derive one from.
- Parallel-plan ready: no. One response, one predicate.
- Review: independent `reviewer` pass before M7 begins.

### Milestone: M5 theme write path

- Depends on: M4 and M1.
- Deliverables: the theme mutation route paired per the house convention at
  `assignment_release.rs:48`; an `ApiClient` method decoding through the existing
  `decodeCourseAppearanceUpdate`.
- Done check: an authorized Instructor changes the stored theme; unknown theme IDs are refused rather
  than defaulted.
- Exit criteria: the `src/api/decoders/question_library.ts:86` re-export has a real caller; no Wasm
  export.
- Parallel-plan ready: yes. Concurrent with M6.

### Milestone: M6 banner backend

- Depends on: M3 and M4.
- Deliverables: banner persistence per M3's decision; upload bound to its exact Course Instance and
  Account; decoded-image validation; set, replace, and remove operations; delivery authorized only
  through the persisted record; the browser client method; alternative text handling; its own
  migration if M3 determined one is needed.
- Done check: an Instructor sets a banner, replaces it, and removes it, with the read model reflecting
  each, and it is served to authorized readers only.
- Exit criteria: validation decodes the image rather than trusting extension or declared type; no
  caller-supplied object path is honored; a failed write leaves no partial state and no orphaned
  object; Students cannot upload; alternative-text defaults reflect that instructor artwork usually
  carries meaning.
- Parallel-plan ready: yes. Concurrent with M5.
- Review: independent `reviewer` pass on the promotion path; a partial promotion corrupts a course's
  visible identity.

### Milestone: M7 Appearance page and Theme section

- Depends on: M5. **Not on M6** -- this is the milestone that puts themes in users' hands.
- Deliverables: the page shell, composed around Theme and Banner sections so M8 fills a reserved place
  rather than renegotiating the layout; `src/routes.ts` and `src/route_contract.ts` entries with
  `ribbon` metadata declaring scope `courseInstance` and task group `courseSetup`; a native radio
  group over all 15 themes; live preview through the preserved `useCourseThemePresentation()` setter;
  theme save and its abandonment invariant; a decision on `course_appearance_styles.ts` -- imported by
  the revived page or deleted, never left orphaned.
- Done check: selecting a theme changes the rendered canvas with no network write; saving persists it;
  two automated transitions assert the rendered theme after save success and after unmount
  mid-preview.
- Exit criteria:
  - Each theme option shows its name beside its swatch, never color alone; the preview shows applied
    palette roles rather than three tiny swatches (`docs/UI_DESIGN_GUIDE.md:238-239`).
  - After a **successful save**, the theme is persisted and displayed, and the preview override is
    released so the page renders from stored data.
  - After **abandonment** -- navigation away, sign-out, error boundary, or any unmount without an
    explicit cancel -- the override is released and the saved theme is authoritative. A stranded
    preview must not outlive the page that owns it.
  - After a **failed save**, the displayed theme is the stored theme, never a preview of a selection
    that did not persist.
  - **The Banner section is absent until M8, not disabled or empty** -- the page never renders a
    control that does nothing.
  - No orphaned appearance module remains.
- Parallel-plan ready: no. One coherent surface and one state machine.

### Milestone: M8 Banner section

- Depends on: M6 and M7.
- Deliverables: banner upload, replace, and remove controls added to the section M7 reserved;
  rendition previews per M3's crop decision; alternative text input; banner save, independent of
  theme save.
- Done check: uploading shows how the image crops at each rendition before saving; saving persists it;
  replacing and removing behave as M6 defined.
- Exit criteria: course identification is presented outside the banner image, not composited over it;
  a pending upload that is abandoned persists nothing; saving a banner does not disturb an unsaved
  theme selection or vice versa.
- Parallel-plan ready: no.

### Milestone: M9 automated accessibility evidence

- Depends on: M8.
- First act: **distinguish what this page introduces from application-wide invariants.** Keep
  permanent tests for keyboard theme selection, non-color identification of the selected theme,
  keyboard banner upload, and usable layout at the supported profiles. Reuse existing global evidence
  for generic invariants such as forced colors and reduced motion where the repository already covers
  them.
- Done check: the suite passes headless against the built bundle over HTTP.
- Exit criteria: axe reports no serious or critical issues on the page; the behaviors above are
  covered; no test exists merely because something could be measured.
- Parallel-plan ready: yes.

### Milestone: M10 Ribbon admission

- Depends on: M9 (`FRONTEND_CAPABILITY_INTEGRATION.md:38-44` requires a real route and page).
- Deliverables: the 14-step walk recorded; the `capability_registry.ts:377-381` flip to `backed`
  naming its client method and server evidence; catalog destination `{kind:"future"}` to
  `{kind:"route"}`; `courseAppearance` removed from `FutureRibbonDestinationId`; regenerated
  `docs/ux/RIBBON_DESTINATION_LEDGER.md`.
- Done check: the task renders for an authorized Instructor and is absent for every other role.
- Exit criteria: no existing control changes position, and the Ribbon layout invariant holds across
  all 15 themes -- same row sizes, same control positions, all controls reachable, no clipping.
  **Reuse the existing all-theme Ribbon oracle from Ribbon M7 and M9b** if it already expresses that
  invariant; a color-only change does not warrant a pixel identity contract.
- Parallel-plan ready: no.

### Milestone: M11 cross-member propagation evidence

- Depends on: M10.
- Deliverables: one registered scenario under `tests/playwright/e2e/`.
- Done check: an Instructor in Course A opens Appearance, selects theme X, uploads banner Y, and
  saves; a Student enrolled in Course A then loads the application normally and receives both,
  asserted from the `data-course-theme` attribute, a computed token value, and the rendered banner.
- Exit criteria: the scenario runs against the disposable stack through its existing owner; a second
  course in the same run is unaffected, proving course scoping. **Course and membership setup follows
  the repository's existing E2E house pattern**; only the appearance behavior under test is driven
  through visible controls.
- Parallel-plan ready: no.

## Acceptance criteria and gates

- Per-patch gate: `./check_codebase.sh` for browser changes; `cargo fmt --check`, `cargo check`,
  `cargo test`, `cargo clippy -- -D warnings` for Rust, per `docs/RUST_STYLE.md`;
  `source source_me.sh && pytest tests/` for the Python lane.
- Integration gate: `tests/test_course_theme_scope.mjs` passes for all 15 palettes at 5.5:1 text and
  3:1 focus and boundary; the appearance routes exercised against the disposable stack; the Ribbon
  layout invariant holds across every theme.
- Independent review gate: a `reviewer` pass on M4 before M7 starts, and on M6 before it closes.
  Authorization defects and partial promotions are both invisible in ordinary use and expensive later.

Failure semantics: a failed M4 review blocks M7. A failed M6 review blocks M8, not M7 -- the theme
lane is unaffected. A failed M9 gate blocks M10. A failed M11 scenario blocks closure.

## Test and verification strategy

- Rust unit tests cover the narrowed theme update, refusal of unknown theme IDs, and banner validation
  including a malformed image and a mismatched declared type.
- Route tests cover authorized Instructor, enrolled Student, anonymous, and foreign Instructor against
  every appearance route, asserting concealment rather than a distinguishable refusal.
- `tests/test_course_theme_scope.mjs` remains the durable palette oracle, unchanged in coverage.
- Browser evidence follows `docs/PLAYWRIGHT_TEST_STYLE.md`: accessible selectors, web-first waits, no
  sleeps, built output over HTTP. The M11 journey belongs in `tests/playwright/e2e/`; focused
  structural checks belong in `tests/playwright/ribbon_*.mjs`.
- No new pytest asserts on collection sizes, required-key lists, or hardcoded palette constants;
  `docs/PYTEST_STYLE.md` treats those as fragile.
- No test is written for concurrent theme editing -- the hypothetical case this plan does not
  mechanize.
- Every gate runs unattended.

## Migration and compatibility policy

- Forward migration only. The theme column default gives every existing course theme `grass`, matching
  the existing Rust default, so no course changes appearance when this ships.
- All 15 theme IDs retained; no stored value can be orphaned.
- Banner schema, if M3 determines one is needed beyond the existing tables, is its own forward
  migration owned by M6.
- Narrowing `CourseAppearanceUpdate` is safe because it has no callers. Narrowing
  `CourseAppearanceView` and the client's ETag assertion is the one deliberate edit to the browser
  read path, made because both mandated a revision this design does not create.

## Risk register

| Risk                                             | Impact                                                               | Trigger                                            | Owner   | Mitigation                                                                                                                                      |
| ------------------------------------------------ | -------------------------------------------------------------------- | -------------------------------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| A section ships before its store                 | Recreates the 2026-08-31 state that was deleted                      | Pressure to show visible progress early            | Manager | M7 depends on M5 and M8 on M6, by construction                                                                                                  |
| Banner research delays themes                    | The working half waits on the harder half                            | M3 or M6 treated as a blocker for the theme lane   | Manager | M3 dispatches day one; M7 depends on M5 only, so the Theme section ships without banner                                                         |
| The membership predicate is guessed              | A course-scoped write is authorized too broadly or narrowly          | M4 dispatched without the identification step      | WS-S    | M4's named first act, covered by the reviewer gate                                                                                              |
| Partial banner promotion                         | A course shows a broken visual identity                              | Interrupted write between object and record        | WS-B    | Persisted saga stages work, compensates confirmed failures, and records uncertain work for repair; fault-injected failure is an acceptance case |
| A stranded preview outlives its page             | Displayed appearance disagrees with the persisted one                | Unmount during preview                             | WS-S    | M8's abandonment invariant with its own automated transition                                                                                    |
| M1 grows into a generation subsystem             | Cleanup outgrows the feature it serves                               | The existing mechanism does not fit 15 identifiers | WS-T    | M1's first act tests that fit and stops if it does not                                                                                          |
| Implementers follow agent-authored contract text | Work is built to `CONTRACTS.md` where it conflicts with working code | Any server package                                 | Manager | Authority order in `## Context`; `assignment_release.rs` is the named precedent                                                                 |

## Rollout and release checklist

- [x] One theme-ID source; a local add propagates from one edit.
- [x] Migration applied; every Course Instance reports a theme by column default.
- [x] Banner decision record complete; M6 dispatchable from it.
- [x] One appearance response carrying theme and banner, under one predicate.
- [x] Theme saved by an authorized Instructor, refused for everyone else.
- [x] Banner uploaded, replaced, and removed, with no orphaned objects.
- [x] Theme section: preview, save, and release on unmount; Banner section absent until it works.
- [x] Banner section added to the reserved place; theme and banner save independently.
- [x] Accessibility suite green, unattended, without duplicating global invariants.
- [x] Ribbon Appearance task rendering for an authorized Instructor, absent for all other roles.
- [x] Cross-member scenario green: Instructor saves both, Student receives both, second course
      unaffected.
- [x] Full gate green: `./check_codebase.sh`, the Rust baseline, and `pytest tests/`.

## Documentation close-out requirements

Each of these currently states something this implementation makes false, and agents build from them.

- `docs/ux/COURSE_APPEARANCE_ACCESSIBILITY_AUDIT.md:3` -- claims a 2026-08-09 acceptance for a form
  deleted three weeks later. Correct it to describe the restored page and its automated evidence.
- `docs/DESIGN_DECISIONS.md:1136-1144` -- its "centered, normalized to 1200 by 328" claim is
  superseded by M3's measured decision. Update it and give it an `Owner` field.
- `docs/ux/RIBBON_DESTINATION_LEDGER.md` -- regenerated by M10.
- `docs/DATABASE_STRUCTURE.md` -- the new column, and any banner schema from M6.
- `docs/CONTRACTS.md:88-131` -- describes a Course Appearance Store, a current-pointer relation,
  `If-Match` compare-and-swap, and appearance revisions. **None was implemented, and none is built
  here.** Correct the section to describe what exists and give it an `Owner` field per
  `docs/REPO_STYLE.md`. Do not restructure the rest of the file.
- `docs/CHANGELOG.md` -- one entry per milestone under the canonical subsection headings. Record under
  `### Decisions and Failures` the read-path gap and the finding that the appearance contract language
  described nothing the repository implemented.
- `docs/ROADMAP.md` and `docs/TODO.md` -- add the delivered capability plus deferred items.
- Active plan: file at `docs/active_plans/active/` with a snake_case filename per
  `docs/REPO_STYLE.md`; `git mv` to `docs/archive/` at closure.

## Open questions and decisions needed

- Manager/subagent decision procedure -- banner sizing, crop, upload shape, and API shape: owned and
  **closed** by M3, from repository patterns and rendered measurement. No human input required and
  none awaited. M3 also produces rendered specimens, so the owner can later ask for a different height
  as a one-number refinement; that possibility gates nothing.
- Non-blocking follow-up -- `--ple-theme-primary`, consumed at `src/style.css:537`,
  `src/pages/assignment_workspace/assignment_workspace_operations.css:34,88`, and
  `assignment_workspace_authoring.css:182,251` but never emitted, so those `color-mix()` declarations
  are dropped today. Repairing it **activates dead styles**, making it a behavior fix needing its own
  review, not cleanup.
- Non-blocking follow-up -- the inert scope-style block neutralized by
  `course_theme_variables.tsx:111`; belongs with `docs/active_plans/virtual-doodling-honey.md`, which
  already owns those files.
- Visual and accessibility acceptance is closed by the M3 rendered specimen and M8/M9 automated
  evidence. Any later owner preference is outside this plan and non-gating.
- Non-blocking follow-up -- the rest of `docs/CONTRACTS.md`. The owner has never read it and has
  suggested deleting it; this plan corrects only its appearance section.
- Non-blocking follow-up -- lookalike removal, owed by `docs/HUMAN_GUIDANCE.md:65`. M1 makes it a
  one-edit change when it happens.
- Non-blocking follow-up -- the other 14 unbacked Ribbon destinations.
