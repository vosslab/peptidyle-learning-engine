# Plan: Ribbon Application Shell for every Product Role

Archived repository home: `docs/archive/ribbon_application_shell.md` (snake_case, per
`docs/REPO_STYLE.md` archive rules). This completed plan supersedes
`plan-velvet-brewing-llama.md`, `plan-velvet-brewing-llama-updated.md`, and
`plan-velvet-brewing-llama-terminology-updates-needed.md`; all three historical plans are archived
under `docs/archive/`.

> **Completion status (2026-09-05): implementation and aggregate acceptance complete.** M0--M12
> implementation is complete, and the durable ledger, task model, integration checklist, UI language,
> ownership decision, and responsibility inventory are current. Fresh focused temporary Chromium
> fixture captures and visual inspection passed for their stated shell and content-boundary invariants.
> They do not regenerate or substitute for the retired production screenshot corpus: committed
> `docs/screenshots/` images remain historical reference only. The task-owned archive transition is
> coherent in the index and worktree, and `./all_test.sh` passed against the final tree.
> `./run_playwright_tests.sh` also remains deliberately unclaimed: it requires documented
> human-owned `PLE_*` real-stack inputs. Fresh production screenshot publication and visual acceptance
> await the restored human-input production-browser owner.

## Context

Clicking a course navigation control changes the whole interface and moves the clicked control. The
navigation lives inside the region destroyed on navigation, and its shape depends on data fetched
after the click. `src/features/course_appearance/course_theme_scope.tsx:104-140` still proves it: an
uncached `resolveCourseRoute` (`src/navigation/resolved_route.ts:23`) runs on every navigation, and
while it runs the `Show` fallback at `:107-110` replaces the entire page **including the course
navigation** with one loading line. `src/components/course_management_frame.tsx:55-66` then renders
the course title as `h1` on one route and `p` on another, so the content origin moves again.

### Authority and derived documents

`docs/HUMAN_GUIDANCE.md` and `docs/TERMINOLOGY_CONTRACT.md` are the only authorities this plan obeys.
`docs/TERMINOLOGY_CONTRACT.md:1188-1190` delegates interface-surface vocabulary to
`docs/INTERFACE_TERMINOLOGY.md`, so canonical Ribbon terms and visible destination names reach this
plan through that delegation. Everything else cited below --
`docs/INTERFACE_TERMINOLOGY.md` beyond its delegated vocabulary, `docs/UI_DESIGN_GUIDE.md`,
`docs/DESIGN_DECISIONS.md`, and the `docs/ux/` audits -- is the current record, not an authority.
This plan may correct any of them, and M12 does. Where a derived document disagrees with the two
authorities, the authorities win and the plan records the correction.

Constraints this plan takes directly from `docs/HUMAN_GUIDANCE.md`:

- Instructor and Sysadmin workflows target a 1280x800 desktop viewport; Student workflows target
  laptop, portrait tablet, narrow phone, and square displays.
- Every Student browser action is usable with the keyboard alone.
- Visual design is pushed harder: less bubbly, less padding, composed around the teaching task rather
  than a collection of individually padded components.
- No UUID appears in visible content, navigation URLs, or copyable links.
- Requirements avoid arbitrary numeric, timing, byte, or pixel equivalence gates.
- Pre-production with no users, so foundational contracts and ownership boundaries may be improved
  directly rather than preserved for compatibility.
- The polished Live Demo is the top priority, and email is not configured, so seeded-role entry is
  the demo path.

Two things changed since the earlier plan was written at commit `6c38dfc` (54 commits ago), and both
shrink the work:

- **The supporting documents already describe the design.** `docs/INTERFACE_TERMINOLOGY.md` now names every Ribbon
  term (Application Shell, Ribbon Schema, Ribbon Scope, Context/Tab/Task Rows, Ribbon Task Area,
  Ribbon Availability, Selected, Loading, No Selected Ribbon Tab, Page Action, Content Layout) and the
  canonical visible destination names, under the terminology contract's delegation.
  `docs/UI_DESIGN_GUIDE.md:70-118` records the per-role schemas, the Question Library Task Areas,
  Course Setup Tasks, the Assignment Attempt slot, and the universal-then-suffix ordering rule.
  `docs/DESIGN_DECISIONS.md:1040-1048` records the six-slot Course Instance Ribbon.
  `docs/UI_DESIGN_GUIDE.md:72` already names `src/ribbon/ribbon_contract.ts` as the planned
  executable owner. This plan adopts those descriptions where they agree with the two authorities and
  corrects them where they do not; no new domain vocabulary is required.
- **Much of the earlier plan's backend and audit work is already done or now out of scope.** The route
  contract is 24 rows with `requiredProductRoles`, and **no row declares `sysadmin`**, so the Sysadmin
  route audit is closed. Parameters are already `:assignmentAttemptRef`, `:questionRef`,
  `:blueprintCourseRef`. `VOCABULARY_REPLACEMENTS.md` is retired with all 417 rows complete.

The remaining constraint is the frontend-only mandate: this plan changes no server, schema, session,
or authorization behavior, and a destination reaches the Ribbon only when its backend capability
already exists. Production `crates/server/src/composition.rs:64-71` registers four HTTP routes today
(`/health`, `/api/auth/session`, `/api/auth/logout`, `/api/auth/live-demo/accounts`), which matches
`docs/INTERFACE_TERMINOLOGY.md:8` ("the current Live Demo stops at seeded Account session entry"). So
the Ribbon must be a designed surface that is complete and reviewable with almost no backend behind
it, and must gain destinations by a one-line registry edit when a handler lands.

## Objectives

- One shell-owned Ribbon instance with fixed geometry survives every navigation for Sysadmin,
  Instructor, and Student, so no visible control changes position after a click.
- Ribbon topology is a pure synchronous function of Ribbon Scope and Product Role, so no network
  response can change which Ribbon Slots exist or where they sit.
- Every visible Ribbon destination is backed by a capability that exists today, recorded as evidence
  a reader can check.
- A future backend capability becomes visible through one declared registry entry plus a documented
  frontend integration checklist, with no shell or geometry change.
- The Ribbon is designable, renderable, and reviewable from hand-written values with no application,
  session, or server behind it.

## Design philosophy

The Ribbon is a designed interface surface, not a generated menu. Application state fills predefined
Ribbon Slots, sets their Ribbon Availability, and selects one; it never decides how many Slots exist.
Eight principles govern every later decision. Each names its authority or its evidence, so a coder
facing an unlisted case can reason from the principle rather than guess.

**1. Position is the contract.** Users remember a toolbar by position, and controls that vanish or
shift turn a trusted surface into "a skittish, tentative idiom that scares new users and disorients
the more experienced" (`About_Face...-2014.md:4487`). This is the same conclusion
`docs/DESIGN_DECISIONS.md:1048` reaches from the product side. Every other principle serves this one.

**2. Topology is synchronous; data may only fill.** Ribbon Scope and immutable Product Role select
the schema on the first frame. Network results supply labels, color, and the third availability
input, never the slot list. Enforced by construction: the derivation takes no resource.

**3. A viewer's Ribbon is constant for the session.** This is the condition that makes omission safe
rather than a violation of principle 1. Cooper's warning is about controls appearing and disappearing
_during use_; PLE's omissions are decided by capability existence (static per build) and Product Role
(immutable per Account), so a viewer sees one set of controls in one order for an entire session and
never watches the row rearrange. Where a genuinely late fact could add a Slot, the append-only
ordering rule confines it to the end of the row. Under that condition, omitting beats disabling: dead
controls are the extra furniture `docs/HUMAN_GUIDANCE.md` argues against, and a Student should not
face six Instructor controls they cannot use.

**4. Reserve all three rows, and make them earn the space.** All three rows are always present,
including an empty Ribbon Task Row, so nothing below the Ribbon moves because of Ribbon navigation.
The cost is real and named in the literature: ribbons "reduce the screen space for the document,
which is a drawback for many users"
(`Designing_the_User_Interface...-2018.md:2329`). The plan pays that cost deliberately and offsets it
by density rather than by removing the row. The rejected alternative is the earlier "content origin
moves only on tab change" compromise, which keeps exactly the movement the user reported.

**5. The Ribbon is deliberately information-dense.** The word is chosen with care: _compact_ means
small, while _information-dense_ names the actual objective -- more useful information and direct
capability per unit of persistent Ribbon space, with organization and accessibility retained. Stated
in full:

> Design the Ribbon for deliberate information density. The Ribbon occupies persistent screen space,
> so use that area to expose useful context, navigation, and frequent destinations efficiently.
> Organize controls into clear task groups, use a small number of standardized presentation sizes,
> reserve larger controls for cases where size materially improves recognition or interaction, and use
> concise labels, state styling, tooltips, and accessible names to preserve discoverability. Density
> must increase direct access without sacrificing visual hierarchy, positional stability, keyboard
> access, or recognition.

Four sources combine into that, and no single one states it alone. Refactoring UI supplies the
_deliberate_ half: dense interfaces are appropriate where a lot must be visible at once, and that
density is a design decision rather than the accidental result of cramped spacing -- the failure mode
being elements given only "the minimum amount of breathing room necessary to not look actively bad"
(`Refactoring_UI-2018.md:474-476`, `:463-466`), with predefined spacing and sizing systems keeping
compactness coherent rather than arbitrary. Shneiderman supplies the population trade-off: dense
collections of small controls can burden novices while experienced users value their small footprint
and rapid access (`Designing_the_User_Interface...-2018.md:2329`), and denser displays outperform
sparser ones for repeat users (`:3463`). Cooper's sovereign posture both licenses the pixels and
demands the restraint that makes them usable (`About_Face...-2014.md:2452`, `:2458`). PLE resolves the
novice half of the trade-off by pairing an icon with its label rather than going icon-only.

**6. Importance is not size.** Prominence comes from position, order, and selection treatment.
"Varying just one of these properties does the trick," and when two elements compete, turn the lesser
down rather than the greater up (`About_Face...-2014.md:4053`). A Ribbon control never grows to
signal that its destination matters.

**7. Acknowledge the click at the clicked control.** Peripheral vision is poor enough that a distant
spinner or banner is routinely unseen, so the surface the user is looking at must respond first. The
selection change lands on the activated control immediately, before content resolves.

**8. User settings may change geometry; application state may never.** Text resized to 200% must not
lose content or functionality (WCAG 2.2 SC 1.4.4) and content must reflow rather than require
two-dimensional scrolling (SC 1.4.10)
(`accessibility/What_Every_Engineer_Should_Know_About_Digital_Accessibility-2024.md:1048`, `:1058`).
So the Ribbon's fixed block size is expressed in relative units and grows with the user's text size,
viewport, and pointer profile. It never changes because of a route, a role, a loading state, an
error, a title length, or a theme. A user action may reshape the Ribbon; the application may not.

Truthfulness is the frontend-only corollary of these: a destination reaches the Ribbon only when its
backing capability exists, so navigation never advertises what the product cannot do.

### The ambition

Aim past "the controls stopped jumping." The target is an application shell an Instructor would
recognize as professional software rather than a course website: one instrument panel that never
moves, dense enough that a teaching session's whole working set is visible at a glance, quiet enough
to stare at for hours, legible at 200% text, usable by keyboard alone, and identical in structure for
every Product Role. PLE's advantage over the comparison products is flat, always-visible primary
navigation instead of a dashboard dropdown hiding ten destinations; this plan should widen that gap,
not merely repair a defect. `docs/REPO_STYLE.md` calls this **dream big**: build the strongest durable
version, then turn it into practical next steps -- which is what the milestone list is.

### What does not bend

Ambition erodes at implementation time, one reasonable-sounding concession at a time, so the
concessions are ruled out here rather than argued later. These five do not bend:

1. One shell-owned Ribbon instance; no navigation ever rebuilt by a route change.
2. Topology synchronous from Ribbon Scope and Product Role; no network result changes the slot list.
3. Constant block size within a responsive profile; no application state moves the content origin.
4. Deliberate information density; reserved space earns its keep rather than being padded out.
5. Icon supports the label; the label is the accessible name.

**These principles are implementation authority, not aspirations.** When an existing component, a CSS
convention, a page's assumptions, or simple convenience conflicts with one of them, reshape the
implementation to fit the principle. Do not reinterpret the principle until the existing code
qualifies -- that is how a philosophy decays into a comment. Principle 5's operational target stays
live through M6 to M9b: _more useful information and direct capability per unit of persistent Ribbon
space._ Fitting the current content into 1280x800 is the floor, not the goal.

Each has a gate, and the gate is the contract. **A failing gate is a design signal, not a test to
loosen.** An implementer who cannot pass one stops and reports the conflict rather than relaxing the
assertion, widening a tolerance, marking a case skipped, or special-casing a route -- any of which
converts a structural guarantee into a convention that will decay. If a genuine conflict emerges
between two invariants, that is a decision for the manager against this document, not a silent trade
inside a patch.

The reviewer clause, to be pinned into every review and QA prompt for this workstream:

> Flag density, compactness, reserved rows, or the persistent Ribbon as a problem only when citing a
> specific failure: unreadable text, clipped or hidden content, broken layout, an unclear state, a
> keyboard or accessibility failure, a contrast failure, or visual ambiguity. "Feels cramped",
> "looks busy", "seems like a lot of chrome", and "could be simpler" are not findings against this
> design. Conversely, any movement of a visible control caused by application state **is** a finding,
> however small.

This applies "fix the design, not the symptom" and "long-term over short-term"
(`docs/REPO_STYLE.md` core philosophies): the fix moves chrome ownership out of route-data resolution
rather than making the loading state prettier.

- Evidence strategy for uncertain methods: geometry and topology stability are proved by behavioral
  oracles (element identity across transitions, slot-index stability across a deferred projection,
  computed block size across every fixture) rather than pixel or timing equivalence, per
  `docs/HUMAN_GUIDANCE.md` on avoiding arbitrary numeric gates. Where a design choice is uncertain
  (row density on the phone profile, cluster spacing ratio), the plan states a measurable rule and a
  milestone encodes it.

## Scope

- Add `src/ribbon/` owning the Ribbon contract, the pure model derivation, the capability registry,
  the scope context, and `AppRibbon` with its stylesheet.
- Extend `src/route_contract.ts` with a `ribbon` member on all 24 rows: Ribbon Scope, optional Ribbon
  Tab, optional Ribbon Task group, and Content Layout.
- Replace prefix sniffing in `src/features/course_appearance/course_theme_route.ts` with a declared
  parameter extractor and a scope key that yields an invalid state instead of falling back to Product
  scope.
- Cache reference-to-identity resolution beside the existing router queries in
  `src/api/application_api.tsx:55-108`.
- Restructure `src/app.tsx` so the keyed `Show` wraps only the content region and the Ribbon, theme
  variables, and scope provider sit above it.
- Retire `course_management_frame`, `course_management_nav`, `assignment_workspace_nav`, and
  `course_theme_route` once the Ribbon owns their destinations.
- Migrate the 12 `useCourseThemeRouteData()` consumers to the hoisted scope data hook.
- Apply the presentation and density contract below: two presentation classes, ordered discrete
  collapse, one boundary per Ribbon Task Area, consistent row height, and selection expressed by
  styling rather than size.
- Publish `docs/ux/` artifacts: the destination capability ledger, the Ribbon task model and heuristic
  or accessibility ledger, and the frontend integration checklist for a future backend capability.

## Non-goals

- Change any server, schema, session contract, decoder shape, or authorization behavior. The session
  keeps `AuthenticatedSession { authenticated, account { id, productRole } }`
  (`src/api/contracts.ts:157-163`); the earlier plan's `courseMemberships` session field is dropped.
- Create pages, routes, mock data, or Ribbon Slots for capabilities whose backend does not exist.
- Build Question Star, Question Watch, Blueprint Updates, Instructor Accounts, Course Appearance, or
  Account Profile surfaces. Their labels stay declared and omitted until a backend lands.
- Rename existing identifiers already accepted by the closed vocabulary ledger.
- Treat Ribbon visibility as authorization. `withRouteAccessBoundary` and the server stay the sole
  enforcement points.

## Implementation contract

Ten normative statements: the canonical checklist for implementers and reviewer agents, each pointing
at the section that argues it. Where a detailed section and this list appear to disagree, this list
states the rule and the section explains why.

1. **One shell instance.** The Ribbon has one DOM instance for the life of the authenticated shell;
   no route change rebuilds it. -- _Design philosophy, principle 1; identity oracle._
2. **Synchronous topology.** Ribbon Scope and immutable Product Role decide the slot list on the
   first frame; no network result changes which Slots exist. -- _Principle 2; three inputs to
   Availability._
3. **Suffix-only late admission.** A Slot is never presented and then withdrawn, and a
   relationship-dependent Slot may only appear after every already-visible Slot. -- _Three inputs;
   topology-stability oracle._
4. **Fixed geometry per profile.** No route, role, loading state, error, title, or theme changes the
   Ribbon's block size. Profile and user settings may, through the token. -- _Principles 4 and 8;
   geometry contract._
5. **Three rows, always.** Context, Tabs, Tasks are all present, including an empty Task Row, and
   they read as one composition rather than three navbars. -- _Principle 4; M9b design objective._
6. **Truthful admission.** A destination reaches the Ribbon only when its complete usable path
   exists. -- _Capability registry; WP-REGISTRY definition of backed._
7. **Rows navigate; pages act.** Ribbon Tabs and Ribbon Tasks navigate. Page Actions perform
   operations and live with their content. -- _Delegated vocabulary; density contract._
8. **Deliberate density.** Reserved space earns its keep through information and direct access, never
   through padding; importance is expressed by position and treatment, never by size. -- _Principles
   5 and 6; presentation and density contract._
9. **Accessibility is geometry, not garnish.** Icon supports the label, the label is the accessible
   name, 200% text and 320px reflow without loss, keyboard reaches everything in DOM order. --
   _Principle 8; icon rules; keyboard and text-resize oracles._
10. **Content adapts to the shell.** During integration, correct the content-side assumption rather
    than weakening the Ribbon. -- _M10 integration stance; "What does not bend"._

A failing gate is a design signal, not a test to loosen. An implementer who cannot satisfy one of
these stops and reports the conflict rather than relaxing it.

## Current state summary

Drift between the earlier plan and the tree, from a direct read:

| Earlier plan states                                 | Today                                                                                                                                |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 32 route rows, `requiredRoles`                      | 24 rows, `requiredProductRoles` (`src/route_contract.ts:34,38-184`)                                                                  |
| 13 course routes declare `sysadmin`; audit needed   | No row declares `sysadmin`; audit closed. Dead branch remains at `src/route_access_boundary.tsx:19-21,34,41-42`                      |
| `src/api/runtime.tsx` holds the queries             | `src/api/application_api.tsx:55-108`, 10 cached queries                                                                              |
| `AuthSession.roles` array; add `courseMemberships`  | `AuthenticatedSession.account.productRole` singular; decoder rejects extra fields (`src/api/decoders/assignment_attempt.ts:686-688`) |
| 16 `useCourseThemeRouteData()` consumers            | 12 (`src/features/course_appearance/course_theme_context.ts:37`)                                                                     |
| `runRef`, `problemRef`, `curriculumRef` params      | `assignmentAttemptRef`, `questionRef`, `blueprintCourseRef`                                                                          |
| Terminology contract owns Ribbon terms              | `docs/INTERFACE_TERMINOLOGY.md` owns them; `docs/TERMINOLOGY_CONTRACT.md:1188-1190` delegates                                        |
| Draft Questions slot conflict needs an owner ruling | Settled: `docs/INTERFACE_TERMINOLOGY.md:92-96` makes My Question Drafts a Question-area Ribbon Task                                  |
| `VOCABULARY_REPLACEMENTS.md` read at gate time      | Retired; successor `docs/archive/vocabulary_final_audit_candidate_2026-09-04.md`                                                     |

Unchanged and still true: the keyed `Show` at `src/app.tsx:200-227` destroys the subtree on every URL
change; the course frame mounts from inside route-data resolution; the loading fallback replaces
chrome; the course title alternates `h1` and `p`; the content column width is switched by the
`data-route-surface` block at `src/style.css:279-288`; `resolveNavigation` is uncached. The Product
Ribbon Tab Row already persists, because `header.site-header` (`src/app.tsx:148-191`) sits outside the
keyed `Show` - so Product scope needs a redesign into Ribbon rows, not a persistence fix.

Backend capability reality, which the registry encodes:

| Destination                                                                                | Route today                                                    | Backend handler today                            |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------------- | ------------------------------------------------ |
| Sign In, seeded demo entry, session, sign-out                                              | yes                                                            | yes (`auth.rs:174-175`, `auth/live_demo.rs:167`) |
| Courses, Course Instance surfaces, Question Library, Blueprint Courses, Assignment Attempt | yes                                                            | no handler registered in `composition.rs:64-71`  |
| My Question Drafts                                                                         | routed to a placeholder (`src/pages/contract_pages.tsx:39-43`) | no                                               |
| Instructor Accounts, Blueprint Updates, Course Setup, Starred, Watched, Account Profile    | no route                                                       | no                                               |

## Ribbon presentation and density contract

Fixed geometry answers _where_ controls sit. It says nothing about _how much_ useful navigation that
reserved space carries, and a persistent surface that is spacious rather than dense pays its cost on
every screen. This section is the presentation half of the contract. It borrows the principle behind
Ferrum's ribbon density guidance while adapting its control system: Ferrum can be icon-_only_ because
chemistry notation is itself the icon, whereas PLE's destinations ("Teaching Operations", "Blueprint
Updates", "Grade Settings") have no conventional glyph. PLE therefore pairs an icon with its label,
which is the combination the sources actually support -- icons carry the scanning and recall benefit,
labels carry the unambiguity that pure pictograms lose for new and intermittent users.

**Principle.** Use Ribbon space efficiently. The Ribbon is persistent application chrome, so each
reserved row carries useful context or navigation without excessive padding or oversized controls.
Prefer concise labels, compact spacing, lightweight selection treatment, and visually grouped
controls. Size controls according to recognition and interaction needs rather than importance alone.

Four rules follow, each with the check that holds it:

- **Importance is separate from physical size.** A primary destination does not get a larger control
  for being primary. Prominence comes from position, order, and selection treatment. _Check:_ within
  one row, control sizing varies only with label length and pointer profile, never with catalog
  priority.
- **Design the dense desktop first, then derive downward.** The 1280x800 Instructor and Sysadmin
  target is designed as a first-class dense professional surface, not as the widest breakpoint of a
  mobile-first Ribbon. Responsive work then derives tablet and phone through the ordered collapse
  below. This ordering matters because mobile-first responsive implementations converge on controls
  sized for touch everywhere, which would spend the desktop's density on a profile that is not the
  Instructor's. Principle 8 already grants the freedom that makes the split legitimate: pointer
  profile may change geometry, so desktop can be unapologetically dense while coarse-pointer profiles
  stay comfortably tappable. _Check:_ the coarse-pointer minimum is met without raising the
  fine-pointer control height.
- **Density moves in discrete states, not a continuous shrink.** Desktop, tablet, and phone are three
  known compact presentations; the Ribbon switches between them at behavior thresholds rather than
  scaling every control smoothly. This is the same statement as the per-profile geometry invariant in
  M6, seen from the presentation side. _Check:_ the enumerated profiles are the only block sizes the
  Ribbon takes.
- **Direct access comes before overflow.** Spend available horizontal space on destinations, not on
  padding around fewer of them. Overflow and row scrolling are the fallback for a genuinely narrow
  viewport, never the normal way to reach a destination. _Check:_ at the 1280x800 Instructor target,
  every Slot in the Instructor Course Instance schema is directly visible with no scrolling and no
  overflow control.
- **Chrome stays visually light.** Grouping and selection organize the Ribbon; they do not turn each
  destination into its own card. Whitespace and alignment do the grouping before any border, per
  `docs/UI_DESIGN_GUIDE.md` on grouping and surfaces. _Check:_ no Ribbon control carries a resting
  border or card background; group separation is spacing or a hairline, and the between-group gap
  exceeds the within-group gap.

### Three separate fields, borrowed from Ferrum's catalog

Ferrum's density work turned on splitting one overloaded concept into three, so that a central command
could still be a small control. PLE's catalog adopts the same split, with PLE meanings:

| Field          | Answers                                                  | Does not control             |
| -------------- | -------------------------------------------------------- | ---------------------------- |
| `role`         | How central is this destination to the task?             | Physical size                |
| `priority`     | How long does it stay directly visible as width shrinks? | Physical size                |
| `presentation` | How is it physically rendered?                           | Importance or overflow order |

So a primary destination may render compactly, and a supporting one may keep a label because its name
is what makes it recognizable. PLE's presentation vocabulary is smaller than Ferrum's: `standard` and
`compact`, with no `large` class at all, since a Ribbon control never grows to signal importance.

**The two classes are a catalog-declared preference, not a viewport reading**, or the distinction
collapses into responsive CSS and the field stops meaning anything:

- `presentation` declares a control's **preferred density class independent of viewport**. A catalog
  entry marked `compact` begins compact at every profile, because that entry's name is short or
  familiar enough not to need the room.
- **Responsive collapse may move `standard` to `compact`; it never moves `compact` to `standard`.**
  The collapse order is one-directional, so a control's floor is its declared class.
- Label length and pointer profile still affect the rendered box within a class. They select
  dimensions; they do not select the class.

### Icons: paired with labels, and how they earn space

Icons are wanted, and they pay for themselves in three ways: faster scanning of a learned row,
stronger recall of position, and a genuine space saving at the narrowest profile where a conventional
glyph can stand alone. The rules that keep them from costing more than they return:

- **Icon plus label is the default for icon-bearing entries.** A catalog entry declares whether it is
  icon-bearing. Where it is, the control shows both glyph and label -- the shape Cooper endorses for a
  surface used by a mix of new, intermittent, and expert users, and the one that keeps the label as
  the accessible name. An entry stays text-only when no glyph carries real meaning for it; a
  destination is never given a decorative glyph merely to fill a column. Text-only and icon-bearing
  entries share one control box so a mixed row still aligns.
- **The icon is decorative markup; the label is the name.** Icons render `aria-hidden="true"` beside a
  real text label, so no screen reader announces a glyph name and no accessible name depends on an
  image. A destination is never identified by icon alone.
- **Icon-only is permitted in exactly two places**, both declared in the catalog rather than improvised:
  Ribbon Context Controls whose glyph is genuinely conventional (account, sign out, back), and the
  narrowest responsive profile for entries flagged as safe to drop their label. A destination with no
  conventional glyph keeps its label at every profile.
- **Every icon-only control carries a tooltip and an accessible name.** Tooltip text equals the
  canonical visible destination name, so the vocabulary stays single-sourced.
- **The icon never carries state or meaning by itself.** Selection, availability, and pending state
  are expressed by the control's own styling, matching the shape-position-text-and-color rule that
  forbids color as a sole indicator.

**Delivery: a bundled SVG sprite, not an icon webfont and not a CDN.** Font Awesome Free is a good
starting set and its glyph vocabulary covers PLE's destinations. Take it as SVG and bundle only the
glyphs actually used -- roughly fifteen -- as a sprite compiled into the build. The reasons are
reliability and control rather than catastrophe: an icon webfont can announce stray characters to some
assistive technology and vanishes when font loading fails (labels survive, since labels are required
beside icons here, but the scanning benefit the icons were added for does not); a CDN adds a runtime
dependency the app does not otherwise have and cannot be relied on offline or in a locked-down
network; a subset sprite ships a fraction of the payload of the full family; and bundled assets sit
inside `./check_codebase.sh` and render deterministically in tests rather than living on a network
path nothing exercises.

Two housekeeping consequences, both small but easy to forget. The icon package is declared in
`package.json` like any other dependency under the repository's pin policy. And the attribution must
be written per redistributed asset type, because Font Awesome Free licenses its parts separately:
**the SVG icon artwork PLE extracts into its sprite is the CC BY 4.0 material, and that is what
`README.md` attributes.** The project's font files (SIL OFL 1.1) and its tooling code (MIT) carry
their own terms and are not redistributed by PLE at all when only SVG paths are bundled. Write the
README line so it names the icon artwork specifically rather than implying one blanket license over
everything in the dependency, and keep it beside the existing per-license mapping that
`docs/REPO_STYLE.md` already requires.

### A Ribbon control may carry state, not only a destination

Cooper's observation that a control can simultaneously offer a command and expose current application
state -- delivering more information for less user effort -- is the strongest lever PLE has for
earning the space it reserves. A navigation link that also answers "how many, how far, what state" is
the difference between reserved chrome and useful chrome.

PLE applies it under three constraints that keep it from breaking the structural contract:

- **State is a label, never topology.** A count or progress value may change what a Slot says; it may
  never change whether the Slot exists, where it sits, or how tall its row is.
- **Every state-bearing position reserves its width up front**, sized for its largest plausible value
  and set in tabular numerals. Assignment Attempt Progress ("Question 3 of 7") is the current case
  named by the delegated vocabulary; a late or growing number then fills reserved space instead of
  nudging the controls beside it.
- **State appears only where it answers a question the viewer is already asking** at that position.
  Decoration disguised as information is the failure mode; a number nobody acts on is noise that costs
  space and attention.

Assignment Attempt Progress is the first case, not the whole opportunity. M7 and M9b carry a standing
design question for **each persistent position**: what useful thing could this area tell the viewer
without adding a control or changing topology? Course identity and current teaching context are
obvious candidates; meaningful counts and status may be others where the viewer already wants them.
This is the difference between a Ribbon that packs navigation labels closer together and one that is
genuinely information-dense. The restraint above is the sufficient guard against dashboard clutter --
apply the question widely, and let that test reject the weak answers.

This is also the honest boundary of the density argument: the Ribbon earns its rows by carrying
context and navigation, not by acquiring operations. Ribbon Tabs and Ribbon Tasks navigate; Page
Actions perform operations, and that split is fixed by the delegated vocabulary.

### Responsive collapse is ordered and discrete

Ferrum's rule is that controls move between known presentation states rather than shrinking
continuously, so the surface stays visually stable. PLE's collapse order as width decreases:

1. Keep the preferred presentation.
2. Convert eligible `standard` controls to `compact`.
3. Drop labels on entries flagged icon-safe, keeping their tooltips and accessible names.
4. Tighten inter-group spacing within its defined floor.
5. Scroll the row, with the clipping cue, so `priority: normal` destinations remain reachable.
6. Collapse the Context Row to a single compact line.

Nothing in that order removes a destination, and no step resizes a control by an arbitrary fraction.
This is the presentation-side statement of the discrete-profiles rule above.

### Group treatment: one boundary, one height

Ferrum's recorded failure is worth avoiding by name: ribbon background, then colored group border,
then group container, then button border, then selected-button fill, is too much visual structure.
The PLE Ribbon Task Row uses at most one boundary per Ribbon Task Area, group height is consistent
across a row so Task Area labels align, and the Area label is one quiet treatment rather than a
heading. Selected state is styling, never size: a Selected Ribbon Tab is heavier and carries the
accent underline, and it occupies exactly the space it did unselected, so selection never reflows the
row.

The reserved empty Ribbon Task Row stays deliberately quiet rather than being filled to look busy. It
is a band of the Ribbon surface, and the space it costs is recovered by compactness in the other two
rows, not by inventing content for it.

Two constraints keep this honest rather than merely tight. Coarse-pointer profiles still reach the
touch-target minimum in `docs/HUMAN_GUIDANCE.md` through their own padding, so density never costs
tappability. And selection stays legible in at least two non-color channels (weight plus an accent
underline), because `docs/UI_DESIGN_GUIDE.md` requires shape, position, text, and color together and
color alone is not the indicator. Course theme color earns its place by communicating grouping and
state, which costs no space, rather than by adding decorative surface.

This reads directly onto `docs/HUMAN_GUIDANCE.md`: "less bubbly, reduce excessive padding" and
"composed around the teaching task, not a collection of individually padded components."

### Why density is the right target for this surface

The literature in `~/Documents/teaching/MARKDOWN_BOOKS/hci` supports density specifically for a
persistent professional surface, which is what separates the Instructor Ribbon from a marketing page:

- **Dense beats sparse for repeat users.** Shneiderman reports that eliminating unnecessary
  information, grouping related information, and emphasizing task-relevant information cut task times
  almost in half, and that "performance times are likely to be shorter with fewer but denser displays
  than with more numerous but sparse displays," with expert users often preferring dense displays
  because they initiate fewer actions
  (`Designing_the_User_Interface...-2018.md:3463`). An Instructor navigating one course repeatedly is
  exactly that user.
- **Sovereign posture licenses the pixels and demands restraint.** Cooper's sovereign-posture
  application is the one users keep open for long periods; it "has a defensible claim on the pixels"
  and should not be shy about the toolbars it needs (`About_Face...-2014.md:2452`), while its visual
  style stays conservative because the user stares at it for hours: "tiny dots or accents of color
  will have more effect in the long run than big splashes, and they enable you to pack controls and
  information more tightly," and "toolbars and their controls can be smaller than normal"
  (`:2458`). That single passage licenses both halves of this contract -- more destinations directly
  visible, and quieter chrome to carry them -- and it is the same reconciliation Ferrum reaches
  between a colorful ribbon and restrained chrome.
- **Emphasis by one property, downward.** "Varying just one of these properties does the trick," and
  when two elements compete, "turn down the less important one rather than turn up the more important"
  (`About_Face...-2014.md:4053`). This is the design argument behind separating importance from size.
- **Group by spacing before containers.** Practical UI groups by proximity and shows navigation
  simplified by deleting its containers, with spacing chosen by how closely related the elements are
  (`Practical_UI-2024.md:505`, `:547`, `:596`), and removes styles that convey no information
  (`:255`). That is the measurable form of "chrome stays visually light."

The Student Assignment Attempt surface is the deliberate exception: it is a single-task, focused
screen rather than a sovereign tool surface, so its one-Slot schema stays sparse by design. Density is
an Instructor and Sysadmin argument, applied to the surface those roles use all day.

## Architecture boundaries and ownership

| Layer                                                     | Owns                                                                                         | May never change     |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------- | -------------------- |
| Design system (`src/style.css`, `app_ribbon.css`)         | Geometry, spacing, hierarchy, overflow, interaction states                                   | -                    |
| Ribbon contract (`src/ribbon/ribbon_contract.ts`)         | Which Ribbon Slots and Ribbon Tasks exist per Ribbon Scope and Product Role, and their order | Geometry             |
| Capability registry (`src/ribbon/capability_registry.ts`) | Whether a declared destination is backed today                                               | Slot order, geometry |
| Route (`src/route_contract.ts`)                           | Selected Ribbon Tab, which Task group fills the row, Content Layout                          | Slot list, geometry  |
| Loaded data (`useRouteScopeData`)                         | Context labels and course theme color                                                        | Anything structural  |

One line for the gates to test against: **Ribbon topology comes from Ribbon Scope and Product Role.
Availability comes from the three inputs below. Routes supply selection. Data supplies labels and
color.**

### The three inputs to Ribbon Availability

Availability has exactly three inputs, and only the third may resolve asynchronously. Keeping them
separate is what lets topology stay synchronous while availability is allowed to settle late.

| Input                   | Question it answers                                                     | Timing                   | Source                                                      |
| ----------------------- | ----------------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------- |
| Capability existence    | Does the complete usable path for this destination exist in this build? | Static, compile-time     | `capability_registry.ts`                                    |
| Product Role permission | May this Account's immutable Product Role use that destination?         | Synchronous, first frame | `productRole` from the session; `productRoleMayAccessRoute` |
| Relationship facts      | Does the current scope relationship permit it for this viewer?          | May resolve late         | `useRouteScopeData` projection                              |

Resolution rule, in order: an entry whose capability does not exist is **Unavailable** and is omitted
from the shipped Ribbon, permanently and synchronously. An entry the Product Role may not use is
**Unavailable** and omitted, also synchronously. An entry whose remaining relationship facts have not
resolved is **Checking**, and this is where an earlier draft of this plan contradicted itself: it had
a Checking Slot render in place and then _disappear_ if the relationship excluded it. Watching a
control vanish is precisely the disorientation principle 1 exists to prevent, and it breaks
principle 3's promise that a viewer never watches the row rearrange. The append-only rule protects
the controls to its left, but not the control itself.

**Corrected rule: a Slot is never presented and then withdrawn, and a late-admitted Slot may only
appear at the end of the visible sequence.** Checking is an internal model state that _withholds_ a
relationship-dependent Slot from the row until its admission is settled, after which it appends after
every already-visible Slot and stays for the session. The schema reserves relationship-dependent
entries as a **suffix**: they are declared last, so admitting one cannot insert before a visible
control. A schema that declared a relationship-dependent Slot ahead of a synchronous one would move
controls on admission, so that ordering is invalid by construction rather than by convention. This is also what the delegated
guidance already says -- "resolve that suffix before displaying it"
(`docs/UI_DESIGN_GUIDE.md:114`) -- so the corrected rule agrees with the record rather than departing
from it. A Checking Slot therefore never renders as a dead or pending control in the shipped Ribbon.
Since the first two inputs are synchronous and the third only withholds, nothing on screen ever moves
or disappears because an answer arrived late.

Today the third input is unused: no current schema Slot depends on a relationship fact beyond Product
Role, so every Slot resolves synchronously. `Checking` exists for the future Course Observer, Student
Observer, and Grader relationships named in `docs/HUMAN_GUIDANCE.md`, which are not derivable from
Product Role. M4 asserts that unused-today property rather than assuming it.

### Mapping (milestones / workstreams -> components / patches)

| Milestone / Workstream | Component                                                                                     | Review boundary                            |
| ---------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------ |
| M0-M1 / WS-Foundation  | `tests/support/`, `src/navigation/route_params.ts`                                            | Pure functions and fixtures; no UI         |
| M2-M4 / WS-Contract    | `src/route_contract.ts`, `src/ribbon/ribbon_contract.ts`, `src/ribbon/capability_registry.ts` | Declarative data plus pure derivation      |
| M5 / WS-Scope          | `src/ribbon/route_scope_context.tsx`, `src/api/application_api.tsx`                           | Resource keying and caching                |
| M6-M9b / WS-Ribbon     | `src/ribbon/app_ribbon.tsx`, `app_ribbon.css`, design fixture                                 | Renders from a `RibbonModel` prop only     |
| M10-M11 / WS-Shell     | `src/app.tsx`, `src/route_access_boundary.tsx`, `course_theme_variables.tsx`, `src/style.css` | Shell restructure and component retirement |
| M12 / WS-Docs          | `docs/ux/`, `docs/UI_DESIGN_GUIDE.md`, `docs/DESIGN_DECISIONS.md`, `docs/CHANGELOG.md`        | Documentation only                         |

## Milestone plan

| M   | Title                                           | Summary                                                                                                                                    | Goal                                                    |
| --- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------- |
| M0  | Test harness                                    | Authority fixtures, counting fake client, transition driver, deferred-resolution fixture, `scrollIntoView` stub, injectable routing signal | Every later gate runs with no server                    |
| M1  | Route parameters and scope key                  | `routeParams`, `routeScopeKey` with an invalid state                                                                                       | Scope derived from declared paths, not prefixes         |
| M2  | Ribbon member on the route contract             | `ribbon` on all 24 rows; Content Layout replaces the width overrides                                                                       | One declarative source of Ribbon behavior               |
| M3  | Catalogs, schemas, and the capability registry  | `TAB_CATALOG`, `RIBBON_TASK_CATALOG`, `ribbonSchemaFor`, backed-capability table                                                           | Topology and truthful admission are data                |
| M4  | Pure model derivation                           | `deriveRibbonModel`, `buildRoutePath`, parameter-subset invariant                                                                          | Topology cannot await the network                       |
| M5  | Scope ownership and cached resolution           | `RouteScopeProvider`, `useRouteScopeIdentity`, `useRouteScopeData`, cached resolve queries                                                 | Navigating inside one course refetches nothing          |
| M6  | Ribbon structure and geometry contract          | Three always-present rows, `--ple-ribbon-*` tokens, shell grid                                                                             | Ribbon block size is a constant                         |
| M7  | Ribbon design fixture                           | Static page rendering every reachable `RibbonModel` state                                                                                  | Design and review with no application                   |
| M8  | Selection, pending, and scrolling behavior      | `aria-current`, `aria-busy`, Selected Tab scrolled into view                                                                               | Feedback lands at the clicked control                   |
| M9  | Overflow cue and phone reachability             | Partial clipping, edge fade, scrollable rows                                                                                               | Narrow viewports stay usable, not merely unwrapped      |
| M9a | Icon system                                     | Bundled SVG sprite subset, one glyph per destination, icon-plus-label rendering                                                            | Icons aid scanning without becoming the accessible name |
| M9b | Density and visual system                       | One Ribbon surface, row roles, presentation classes, proximity spacing, accent in three places                                             | Fixed geometry carries dense, quiet, useful navigation  |
| M10 | Mount the Ribbon, then narrow the swap boundary | Ribbon beside the existing frame, then keyed `Show` around content only                                                                    | Navigation is never absent mid-migration                |
| M11 | Retire superseded navigation and CSS            | Course frame, course nav, workspace nav, route sniffing, width overrides                                                                   | One navigation object in the DOM                        |
| M12 | UX evidence and documentation close-out         | Capability ledger, task model, heuristic ledger, integration checklist, doc updates                                                        | A future backend plugs in from written steps            |

### Milestone: M0 test harness

- Depends on: none
- Deliverables: Product Role fixtures (Student, Instructor, Sysadmin); a counting fake `ApiClient`
  injected through `createApplicationApi` (`src/api/application_api.tsx:55`); a synthetic transition
  driver that walks a pathname list through one mounted app; a deferred-resolution fixture the test
  releases on demand; a `scrollIntoView` stub; an injectable routing-in-flight signal; Playwright
  context options `forcedColors: "active"` and `reducedMotion: "reduce"`.
- Entry criteria: none
- Exit criteria: the harness mounts a trivial component under each Product Role fixture.
- Parallel-plan ready: no. Every later workstream consumes it, so it lands first and alone.

### Milestone: M1 route parameters and scope key

- Depends on: M0
- Deliverables: `src/navigation/route_params.ts` with `routeParams(route, pathname)` zipping the
  declared pattern against the live pathname, and `routeScopeKey(pathname)` returning Product,
  Course Instance, Assignment Attempt, or an invalid state that keeps its declared Ribbon Scope.
- Exit criteria: each declared path extracts its named parameters; a malformed reference on a scoped
  path yields the invalid state and never falls back to Product scope.
- Parallel-plan ready: yes (WS-Foundation, independent of WS-Contract until M2 consumes it).

### Milestone: M2 Ribbon member on the route contract

- Depends on: M1
- Deliverables: `RibbonScope`, `ContentLayout`, `RibbonTabId`, `RibbonTaskGroupId`, and a `ribbon`
  member on every row. Assignment workspace rows declare the assignment Task group; Context Control
  routes (`signIn`, `pendingCourseInvitations`) declare no Tab and render No Selected Ribbon Tab.
- Exit criteria: `npx tsc --noEmit` passes; an exhaustive test resolves every `RouteId` to a declared
  Ribbon Scope and Content Layout.
- Parallel-plan ready: no. One file, one owner, consumed by everything downstream.

### Milestone: M3 catalogs, schemas, and the capability registry

- Depends on: M2
- Deliverables: `TAB_CATALOG` and `RIBBON_TASK_CATALOG` carrying canonical labels from
  `docs/INTERFACE_TERMINOLOGY.md:86-114`; `ribbonSchemaFor(scope, productRole)` total over all nine
  pairs of the three Product Roles named in `docs/TERMINOLOGY_CONTRACT.md:110` and the three Ribbon
  Scopes, currently described by `docs/UI_DESIGN_GUIDE.md:78-82`, and shaped so a future Course
  Observer, Student Observer, or Grader relationship is a catalog edit rather than a layout change
  (`docs/HUMAN_GUIDANCE.md`); `capability_registry.ts` declaring, per
  destination, whether a backing capability exists today, with the evidence reference recorded beside
  it; `ribbonAvailability` taking the three inputs above in their stated order and returning
  Available, Checking, or Unavailable.
- Exit criteria: the append-only ordering rule holds structurally in every schema; every declared
  label equals its canonical visible name; no Ribbon Task declares an operation (Create Assignment
  stays a Page Action); every registry entry naming a live capability resolves to a route in
  `ROUTE_CONTRACT`; `ribbonAvailability` returns Unavailable synchronously for a missing capability
  and for a Product Role that may not use the destination, and returns Checking only when a
  relationship fact is genuinely outstanding; a Checking Slot is withheld from the rendered row rather
  than drawn, so no Slot can be presented and later withdrawn.
- Parallel-plan ready: yes (catalog, schema table, and registry are separable work packages).

### Milestone: M4 pure model derivation

- Depends on: M3
- Deliverables: the pure derivation and `buildRoutePath(routeId, params)`. **Its parameters are named
  and typed for exactly what they may contain**, because `authority` and `identity` were broad enough
  that an implementer could pass loaded scope data through the synchronous boundary without noticing:
  `deriveRibbonModel(routeState, viewerIdentity, contextLabels)`, where `routeState` carries the
  matched route contract plus extracted parameters, `viewerIdentity` carries the synchronous
  session-derived Product Role and nothing else, and `contextLabels` is a narrow record of
  already-resolved display strings. No parameter type admits a resource, promise, accessor, or
  projection, so the compiler refuses the mistake the prose was relying on discipline to prevent.
- Exit criteria: the function takes no resource and returns synchronously with the network fake set
  never to resolve; the slot list is identical before and after releasing every deferred fixture, for
  each Product Role; with the current schemas every Slot resolves to Available or Unavailable
  synchronously, so no Slot renders Checking today, and the test states that as the current property
  rather than a permanent prohibition; every catalog destination's declared parameters are a subset of
  the parameters its declaring routes provide.
- Parallel-plan ready: no. One pure module.

### Milestone: M5 scope ownership and cached resolution

- Depends on: M1, M4
- Deliverables: `resolveCourse` and `resolveAssignmentAttempt` router queries beside the ten in
  `src/api/application_api.tsx:61-107`; `RouteScopeProvider` keyed by the scope reference;
  `useRouteScopeIdentity` (synchronous) and `useRouteScopeData` preserving the existing
  `CourseThemeRouteData` shape from `course_theme_context.ts` so consumer migration stays mechanical.
- Exit criteria, stated as behavior: navigating between routes of the same Course Instance reuses the
  already-resolved scope and performs no redundant resolution; a scope change resolves the new
  reference and reuses valid previously resolved data for a scope returned to; the provider is never
  unmounted by a route change, including Course Instance to Assignment Attempt and back; no
  navigation renders a chrome-level loading state. Request counts taken with the counting fake
  (same-scope zero, one resolve plus one scope request per new leg, zero on return to a cached scope)
  are recorded as **migration evidence** that the keying is right, not as the durable contract, and
  they retire with the milestone.
- Parallel-plan ready: yes (query caching and the provider are separable, sharing only the key type).

### Milestone: M6 Ribbon structure and geometry contract

- Depends on: M4
- Deliverables: `AppRibbon` rendering three always-present rows from a `RibbonModel` prop alone;
  `--ple-ribbon-context-block-size`, `--ple-ribbon-tab-block-size`, `--ple-ribbon-task-block-size`
  summed into `--ple-ribbon-block-size`; the shell grid keyed to that token.
- Exit criteria: all three rows render including an empty Ribbon Task Row; the module graph of
  `app_ribbon.tsx` pulls in no session, router, or API module; **within one responsive profile**,
  computed ribbon block size is equal across every Ribbon Scope, every Product Role fixture, a
  Task-less tab, a very long Course Instance title, a loading state, and an error state.
- Accessibility exit criteria, from principle 8: at 200% text size every Ribbon label remains
  readable and every destination remains reachable, with the Ribbon growing rather than clipping;
  at a 320 CSS pixel width the shell reflows without two-dimensional scrolling. Row heights are
  expressed in relative units so they follow the user's text size.
- Semantics, fixed here so no coder invents them: Ribbon Tabs and Ribbon Tasks are **navigation links
  inside labelled `nav` landmarks**, not an ARIA `tablist`. They change the route rather than swapping
  an in-page panel, so `aria-current="page"` marks the Selected Ribbon Tab and normal link tabbing is
  the keyboard model. A `tablist` would promise arrow-key panel switching the Ribbon does not
  implement. Each row's landmark carries a distinct accessible name.
- Scope of the invariant, stated once so M9 cannot contradict it: the Ribbon has one stable block
  size _per responsive profile_, not one universal number. A profile change is a viewport change the
  user performs deliberately, and content does not move under a click; an application-state change
  must never alter the block size within a profile. The token stays one value; each profile may
  redefine it.
- Parallel-plan ready: yes (component structure and the token/grid contract are separable).

### Milestone: M7 Ribbon design fixture

- Depends on: M6
- Deliverables: a static page rendering every state the Ribbon can occupy from hand-written values:
  each schema, each Ribbon Availability, selected and unselected, empty and populated Task Rows,
  short and very long Course Instance titles, Assignment Attempt Progress at its widest, and each
  course theme.
- **This fixture is a design laboratory, not a screenshot of today's PLE.** It exists at the one
  moment when no legacy page CSS is pushing back, which makes it the cheapest opportunity the project
  will get to decide what PLE looks like. WS-Ribbon produces several credible complete treatments --
  pushing spacing denser, row hierarchy stronger, chrome quieter, course color better integrated, and
  controls more compact than the current application would normally tolerate -- and then selects the
  best coherent system rather than the most familiar one. A treatment that merely reproduces the
  current look has not done the milestone's work.
- Glyph review happens here and as a **set**: display every icon together and ask "can I tell these
  destinations apart at a glance?", not "is each icon individually defensible?". Five individually
  reasonable document, list, and folder glyphs make a poor vocabulary; distinctiveness across the set
  outranks literal semantic fit for any one entry.
- Exit criteria: every declared combination renders; the fixture imports no session, router, or
  client module; at least two complete alternative treatments were built and the selection is recorded
  with its reasoning. This fixture is the target for the M9b and M12 visual assertions, so visual
  review needs no running application.
- Parallel-plan ready: yes (alternative treatments are independent explorations against one fixture
  harness, converging on a single selection).

### Milestone: M8 selection, pending, and scrolling behavior

- Depends on: M6
- Deliverables: `aria-current="page"` on the Selected Ribbon Tab; a Ribbon-owned pending-destination
  signal plus the injected routing-in-flight signal driving `aria-busy`; the Selected Tab scrolled
  into view with `inline: "nearest"`, smooth only when reduced motion is not requested.
- Pending state has a named owner, because a boolean routing signal cannot say which control started
  the navigation. The Ribbon records the destination its own control activated, in Ribbon-owned
  state. The pending treatment renders only while that recorded destination is set **and** routing is
  in flight, and the record clears when the route settles or when the settled route is not the
  recorded destination. A navigation begun from page content, browser history, or a redirect leaves
  the record empty, so no Ribbon control wears a pending treatment it did not cause.
- Exit criteria: activating a Ribbon control puts the pending treatment on that control and nowhere
  else; a routing-in-flight signal with no Ribbon activation puts it on no control; the record clears
  on settle including a redirect to a different destination; the scroll stub is called on Tab change
  and not called when the Tab is already visible.
- Parallel-plan ready: yes (pending state and scroll behavior are independent work packages).

### Milestone: M9 overflow cue and phone reachability

- Depends on: M8
- Deliverables: horizontal overflow with partial clipping and a soft edge fade; coarse-pointer
  padding that reaches the touch-target minimum in `docs/HUMAN_GUIDANCE.md`; a compact single-line
  Ribbon Context Row on coarse pointers.
- Exit criteria: at the tablet and phone profiles in `tests/playwright/ui_corpus_manifest.ts`, each
  row renders as a single non-wrapping row; the Ribbon positions the row so the Selected Ribbon Tab
  lies within the row's visible box without user action; every other Tab is reachable by scrolling
  that row; the clipping cue element is present when the row overflows; and the computed hit box
  meets the minimum. Tab width and ordering are deliberately not constrained: the requirement is that
  the Ribbon brings the Selected Tab into view, not that every schema fits a phone unscrolled.
- Geometry note: the coarse-pointer padding and compact Context Row change the block size _between_
  responsive profiles only. Within each profile the M6 invariant holds unchanged.
- Parallel-plan ready: no. One CSS surface.

### Milestone: M9a icon system

- Depends on: M3, M7
- Deliverables: WP-ICONS -- the glyph map, the bundled sprite and its build step, icon-plus-label
  rendering in `AppRibbon`, the declared icon-only entries, and the dependency and attribution
  housekeeping.
- Exit criteria: the WP-ICONS acceptance criteria pass; the M7 fixture renders every glyph at every
  profile; `./check_codebase.sh` clean with the sprite in the bundle.
- Parallel-plan ready: yes (glyph selection and the sprite build step are separable from the rendering
  change, sharing the glyph map as their contract).

### Milestone: M9b density and visual system

- Depends on: M7, M9, M9a
- **Design objective, which the mechanical criteria below support but cannot prove:** the Ribbon reads
  as one coherent professional instrument panel whose hierarchy is apparent before any individual
  control is read. The three rows are a composition, not three stacked navbars with slightly different
  CSS -- an outcome that could satisfy every assertion below while wasting the architecture. Each row
  communicates its job immediately through typography, spacing, accent strength, and rhythm:
  **Context answers "where am I", Tabs answer "which destination", Tasks answer "which part of this
  work do I enter".** The Tasks phrasing is deliberately navigational: Ribbon Tasks navigate and Page
  Actions perform operations, so a phrasing like "what can I do here" would quietly invite operations
  into the navigation rows.
  Passing the gates with three interchangeable bars is a failed milestone; a reviewer states whether
  the hierarchy reads, and that judgment is part of the exit.
- Deliverables: the presentation contract applied to `app_ribbon.css` -- one Ribbon surface with a
  single bottom edge against the content region and hairline or spacing-only internal separation;
  three differentiated row roles (Context quietest, Tabs the strongest horizontal rhythm, Tasks a
  lighter strip with visible Task Area grouping); the `standard` and `compact` presentation classes;
  spacing drawn from the existing `--ple-space-*` scale (`src/style.css:19-25`) with a smaller step
  within a Task Area than between Areas; soft-flat control states (near-flat at rest, soft rounded
  hover and focus, retained stronger background when selected); the course accent used in exactly
  three places -- beside the Context Row course identity, the Selected Tab underline, and the selected
  Task background -- derived through the existing theme recipe rather than raw anchors.
- Exit criteria, all computed rather than judged: every Ribbon spacing value resolves to a
  `--ple-space-*` token; the between-Area gap exceeds the within-Area gap; no Ribbon control has a
  resting border or card background; the Selected Tab differs from an unselected Tab in at least two
  non-color channels and occupies the same box in both states; at 1280x800 every Instructor Course
  Instance Slot is directly visible with no overflow control; control sizing varies only with label
  length and profile, never with catalog `role` or `priority`; a contrast script passes every theme in
  the theme catalog against `docs/ux/COURSE_APPEARANCE_ACCESSIBILITY_AUDIT.md` thresholds; under
  `forcedColors: "active"` the Selected Tab, Task Area separation, and control state stay
  distinguishable when tint and shadow drop out; under `reducedMotion: "reduce"` no scroll animation
  runs.
- Entry criteria: M7's design fixture exists, so every assertion above runs against hand-written
  models with no application, session, or server.
- Parallel-plan ready: yes (row roles, control states, and the theme accent are separable work
  packages sharing one stylesheet owner for merge).

### Milestone: M10 mount the Ribbon, then narrow the swap boundary

- Depends on: M5, M9b (which itself depends on M9 and M9a). Integration waits for the _finished_
  visual system deliberately: the integration stance below asks the Ribbon to resist legacy page CSS,
  and it cannot do that before the design it is defending exists.
- Requirement: navigation is continuously present and usable at every commit during M10, and exactly
  one navigation surface exists at M10 completion.
- **Integration stance for M10 and M11, where the design is most at risk.** Once the isolated Ribbon
  succeeds, legacy page CSS, heading structures, fixed widths, and theme wrappers start pushing back.
  The rule is that **the shell owns the composition and migrated content adapts to it.** When
  integration exposes awkward spacing, duplicated hierarchy, or a page that assumed it owned the top
  of the screen, correct the content-side assumption rather than enlarging, padding, or weakening the
  Ribbon to accommodate it. `docs/HUMAN_GUIDANCE.md` is explicit that PLE is pre-production with no
  users and that foundational correction beats compatibility; this is the milestone that spends that
  freedom.
- Preferred migration strategy, not an acceptance state: patch A mounts `AppRibbon` in the shell while
  `CourseManagementFrame` stays, so navigation is briefly duplicated and never absent; patch B moves
  the keyed `Show` to wrap only the content region. A coder who finds a clean atomic restructuring
  that keeps navigation continuously present may take it instead; the requirement above is what the
  gate checks.
- Deliverables either way: the keyed `Show` wraps only the content region; `id="main-content"` and
  `tabindex="-1"` move from `<main>` onto the content region; `focusMainContent`
  (`src/app.tsx:118-141`) is retargeted; `.shell` becomes a grid; `RouteScopeProvider` and the
  theme-variable wrapper sit above the keyed boundary.
- Exit criteria: the identity oracle passes across route-to-route inside one Course Instance,
  Product-to-Product, Course Instance A to B, Course Instance to Assignment Attempt and back, and Tab
  changes; the skip link lands past the Ribbon; a page-level error keeps the Ribbon usable because the
  error boundary now sits inside the content region.
- Parallel-plan ready: no. One owner holds `src/app.tsx` through the restructure, whichever patch
  sequence that owner chooses.

### Milestone: M11 retire superseded navigation and CSS

- Depends on: M10
- Entry criterion, before any retirement patch: a responsibility inventory of the four retiring
  components, mapping every user-visible responsibility to its new owner. Cover at least the
  `h1`-versus-`p` course title and where the page heading now lives; theme variable emission and the
  appearance live-preview context; assignment workspace section navigation and its `aria-current`
  semantics; the course eyebrow and identity surface; the `New assignment` entry, which becomes a
  Page Action on Assignments rather than a Ribbon Slot; and any non-navigation behavior hidden in
  those files. A responsibility with no named new owner blocks the retirement patch.
- Deliverables: `course_theme_scope.tsx` becomes `course_theme_variables.tsx`, keeping the themed
  wrapper and `CourseThemePresentationContext` (the appearance preview at
  `course_appearance_page.tsx` needs it) and dropping `managementRoute()` and the frame branch; the 12
  `useCourseThemeRouteData()` consumers move to `useRouteScopeData`; `course_management_frame`,
  `course_management_nav`, `assignment_workspace_nav`, and `course_theme_route` retire; the
  `data-route-surface` width block at `src/style.css:279-288` is deleted while the attribute stays for
  Playwright selectors; the dead `sysadmin` branch in `route_access_boundary.tsx:19-21,34,41-42` and
  the `workspaceList` placeholder nav entry go with them.
- Exit criteria: every row of the responsibility inventory has a named new owner and a check that
  exercises it; exactly one course navigation in the DOM; Ribbon element identity unchanged across
  the retirement; `npx tsc --noEmit` with the old hook deleted enumerates every missed consumer;
  `./check_codebase.sh` clean; dead-export scan empty.
- Parallel-plan ready: yes, after the responsibility inventory lands. That inventory is one owner's
  serial work package; consumer migration, component retirement, and CSS cleanup then run in parallel
  once the compiler enumerates the sites.

### Milestone: M12 UX evidence and documentation close-out

- Depends on: M11
- Deliverables: `docs/ux/RIBBON_DESTINATION_LEDGER.md`, with one explicit ownership split. Its
  generated section is emitted from `capability_registry.ts` by a small `devel/` command and carries
  the machine-owned columns only: canonical label, route id, client method, backing handler evidence,
  and derived Ribbon Availability. A test regenerates it and fails when the committed file differs,
  so code and document cannot disagree. Its editorial section is hand-written prose per destination
  explaining what the surface is for and what a reader should know; no test asserts that prose.
  `docs/ux/RIBBON_TASK_MODEL.md` (per Product Role: trigger, goal, decision points,
  information needs, error and recovery, completion evidence, plus the heuristic and accessibility
  ledger with acceptance criteria); `docs/ux/FRONTEND_CAPABILITY_INTEGRATION.md` (the ordered
  checklist a future backend follows); corrections to `docs/UI_DESIGN_GUIDE.md:70-73` so it points at
  `docs/INTERFACE_TERMINOLOGY.md` rather than the terminology contract, and to
  `docs/DESIGN_DECISIONS.md:1040` so the entry reads as the shell-owned Ribbon for every Product Role;
  a `docs/CHANGELOG.md` entry after each milestone.
- **Generalize the philosophy into PLE's UI language, not AppRibbon trivia.** This workstream produces
  principles that should govern pages built long after it closes: restrained surfaces, proximity
  before containers, deliberate information density, stable spatial memory, contextual state that does
  not disturb geometry, position and treatment rather than size for prominence, immediate feedback at
  the point of interaction, discrete responsive states, and accessibility expressed in the geometry
  itself. `docs/UI_DESIGN_GUIDE.md` records them as PLE's general UI language with the Ribbon as their
  first and reference implementation, so a future page inherits the philosophy instead of
  reinventing a padded, page-oriented layout beside it.
- Exit criteria: `pytest tests/` markdown-link and hygiene gates pass; every ledger row cites a file
  a reader can open.
- Parallel-plan ready: yes (three documents, three owners).

## Workstream breakdown

### Workstream: WS-Foundation

- Goal: fixtures and pure route helpers every other workstream consumes.
- Work packages: WP-HARNESS, WP-PARAMS.
- Provides: authority fixtures, counting fake, transition driver, `routeParams`, `routeScopeKey`.

### Workstream: WS-Contract

- Goal: declarative Ribbon data and its pure derivation.
- Work packages: WP-ROUTE-RIBBON, WP-CATALOG, WP-REGISTRY, WP-DERIVE.
- Needs: WS-Foundation. Provides: `RibbonModel` for WS-Ribbon.

### Workstream: WS-Scope

- Goal: one scope owner keyed by reference, with cached resolution.
- Work packages: WP-QUERY-CACHE, WP-SCOPE-PROVIDER.
- Needs: WS-Foundation, WP-DERIVE. Provides: synchronous identity and cached projection.

### Workstream: WS-Ribbon

- Goal: the designed surface, reviewable with no application behind it.
- Work packages: WP-STRUCTURE, WP-GEOMETRY, WP-FIXTURE, WP-STATE, WP-OVERFLOW, WP-ICONS, WP-DENSITY.
- Needs: WS-Contract. Provides: `AppRibbon` for WS-Shell.

### Workstream: WS-Shell

- Goal: shell restructure and retirement of superseded navigation.
- Work packages: WP-MOUNT, WP-BOUNDARY, WP-CONSUMERS, WP-RETIRE.
- Needs: WS-Scope, WS-Ribbon. Review boundary: `src/app.tsx` has one owner throughout.

### Workstream: WS-Docs

- Goal: written evidence and the plug-in path for a future backend.
- Work packages: WP-LEDGER, WP-TASK-MODEL, WP-CHECKLIST.
- Needs: WS-Shell exit. Provides: closure evidence.

## Work packages

### Work package: WP-REGISTRY

- Touch points: `src/ribbon/capability_registry.ts`, its unit test.
- Depends on: WP-ROUTE-RIBBON.
- **Definition of backed, stated once so a partial path cannot satisfy the gate:** a destination is
  backed when the _complete usable path_ it needs exists -- a mounted route, a real page rather than a
  placeholder, the frontend client method it calls, and the registered server handler that method
  targets wherever it needs one. A client method pointing at an absent endpoint is not backed; that
  combination is exactly the "looks implemented, fails on click" case the truthfulness objective
  exists to prevent. A destination whose page genuinely needs no server call is backed once its route
  and page exist, and the entry records that as its evidence.
- Acceptance criteria: every canonical destination in `docs/INTERFACE_TERMINOLOGY.md:86-114` appears
  exactly once; each entry declares a backing state and an evidence reference; a destination declared
  backed satisfies the complete-path definition above, naming its `RouteId`, its client method, and
  its server handler evidence or an explicit no-server-call justification; `ribbonAvailability` omits unbacked
  Slots and preserves the relative order of the rest.
- Obvious follow-ons: WP-LEDGER renders the same table into `docs/ux/` so the document and the code
  cannot disagree.

### Work package: WP-ICONS

- Owner: WS-Ribbon.
- Touch points: `src/ribbon/ribbon_icons.ts` (the glyph map, one entry per catalog destination), the
  bundled sprite asset and its build step, `package.json`, `README.md` attribution, and the icon
  fixtures in the M7 design page.
- Depends on: WP-CATALOG (destinations exist to map), WP-STRUCTURE (a control to render into).
- Acceptance criteria: every catalog entry **declared icon-bearing** resolves to exactly one glyph,
  checked exhaustively so an icon-bearing entry with no declared glyph fails the suite, while a
  text-only entry passes without one -- the invariant is completeness of declared intent, never
  "an icon for every destination"; every rendered icon carries `aria-hidden="true"` and sits beside a
  text label, or is one of the declared icon-only entries with a tooltip and an accessible name equal
  to its canonical visible name; the sprite contains only glyphs the catalog references, asserted by
  comparing sprite ids against catalog usage in both directions; **the Ribbon's own icon rendering
  resolves entirely from bundled assets** -- asserted by scoping the check to Ribbon icon requests
  (sprite ids and the icon asset path) rather than to page imagery generally, so an unrelated future
  feature that legitimately loads an image does not trip this gate; the icon package is declared in
  `package.json` and the attribution required below appears in `README.md`.
- Evidence or review: the M7 design fixture renders the full glyph set at every profile, including the
  narrow profile where flagged labels drop, so icon choices are reviewable with no application
  running.
- Obvious follow-ons: WP-DENSITY consumes the finished icon metrics when it sets row heights, since a
  glyph's optical size participates in the compact profile's block size.

### Work package: WP-CONSUMERS

- Touch points: the 12 files listing `useCourseThemeRouteData()` (`course_entry_identity.tsx:36`,
  `gradebook_page.tsx:631`, `teaching_operations_page.tsx:15`, `student_work_inspection_page.tsx:438`,
  `assignment_preview_page.tsx:236`, `assignment_attempt_page.tsx:815`,
  `course_assignments_page.tsx:306`, `course_grade_settings_page.tsx:672`,
  `assignment_attempt_summary_page.tsx:19`, `course_roster_page.tsx:116`,
  `assignment_workspace_live_page.tsx:165`, `assignment_workspace_create_page.tsx:29`).
- Depends on: WP-SCOPE-PROVIDER, WP-BOUNDARY.
- Acceptance criteria: each site changes an import and a hook name only, because the context value
  shape is preserved; `npx tsc --noEmit` with the old hook deleted is the completeness check.

### Work package: WP-TASK-MODEL

- Touch points: `docs/ux/RIBBON_TASK_MODEL.md`.
- Depends on: WP-RETIRE.
- Acceptance criteria: one task model per Product Role covering re-orientation after content changes,
  moving between Course Instance Tabs, and entering and leaving an Assignment Attempt; each heuristic
  row states the guideline, its user-facing rationale, and the acceptance check that proves it,
  including WCAG 2.2 SC 3.2.3 Consistent Navigation and 3.2.4 Consistent Identification.

## Acceptance criteria and gates

- Per-patch gate: `./check_codebase.sh` (typecheck, wider typecheck, ESLint at zero warnings,
  Prettier, Node unit tests).
- Integration gate: `./run_playwright_tests.sh`, then the repository aggregate gate; record browser
  evidence that could not run rather than claiming it.
- Independent review gate: a reviewer agent checks each milestone's oracle against its stated
  invariant before the next milestone starts, since several gates assert absence (no remount, no
  refetch, no movement) and absence is easy to assert vacuously.

## Test and verification strategy

Behavioral oracles, not pixel or timing equivalence (`docs/HUMAN_GUIDANCE.md` on avoiding arbitrary
numeric gates):

- **Identity oracle.** Tag the Ribbon element on mount with a per-mount value; assert it is unchanged
  across route-to-route inside one Course Instance, Product-to-Product, Course Instance A to B,
  Course Instance to Assignment Attempt and back, and Tab changes. Any remount fails.
- **Topology-stability oracle.** With the projection deferred, capture the slot list; release it;
  capture again. Every slot present in the first capture holds the same index in the second; **no slot
  present in the first capture is absent from the second**; and **every newly admitted slot appears
  strictly after all slots from the first capture**, so admission is asserted as a suffix rather than
  left as an implication of index stability. The Ribbon block size is unchanged. Run it for the unresolved Course Instance, the invalid reference,
  and each Product Role.
- **Geometry contract.** Computed ribbon block size equals `--ple-ribbon-block-size` across every
  scope, Product Role, Task-less tab, long title, loading state, and error state. **The token is the
  authority, and it is itself a function of profile and user settings**, so the oracle compares
  rendered geometry against the token's _currently computed_ value rather than against a baseline
  captured at default text size. Growth under 200% text is the token changing, which is correct
  behavior under principle 8; a regression is rendered geometry disagreeing with the token it should
  equal. Stated explicitly so nobody reads accessibility growth as a geometry failure and "fixes" it
  by clamping the Ribbon.
- **Chrome-during-load oracle.** With the projection deferred, the Ribbon Tab Row renders its scope's
  full schema, backed Slots are clickable, and an unresolved or invalid reference uses the Course
  Instance schema rather than the Product schema.
- **Consistency oracle.** For each Product Role fixture, the Ribbon landmark has the same accessible
  structure on every route that role can reach.
- **Keyboard oracle.** Tab order reaches Context Row, Tab Row, Task Row, then content, on every route.
- **Row-count and reachability oracle.** At 1280x800 and the tablet and phone profiles, each row
  renders as one row and every Tab is reachable. Treat a failure here as a design signal, not a test
  to loosen.
- **Availability-never-exceeds-boundary.** For every authority fixture, each Available Slot's
  destination is also permitted by `productRoleMayAccessRoute`. The Ribbon may show strictly less
  than the boundary allows, never more.
- **Registry truthfulness.** Every Slot rendered in the shipped shell has a registry entry marked
  backed, and that entry names a route and a client method that exist.

These oracles overlap in the combinations they walk, so each one earns permanence by naming the
regression it **uniquely** catches. `docs/HUMAN_GUIDANCE.md` and `docs/PYTEST_STYLE.md` both prefer a
missing test to a redundant one, so a fixture matrix is shared and each oracle asserts only its own
property over it rather than re-walking roles, scopes, and profiles on its own.

| Oracle                 | Unique regression it catches                                                                  | Matrix it needs                                        |
| ---------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Identity               | The Ribbon element is rebuilt by a navigation                                                 | Transition list, one Product Role                      |
| Topology-stability     | The Ribbon survives but its contents reorder, or a presented Slot is withdrawn, as data lands | Deferred projection, each Product Role                 |
| Geometry contract      | An application state changes the content origin within a profile                              | State list, one profile per run                        |
| Chrome-during-load     | Geometry or schema waits on the network                                                       | Deferred projection, unresolved and invalid references |
| Consistency            | The Ribbon's accessible structure differs between routes for one role                         | Routes per Product Role                                |
| Text-resize and reflow | Fixed geometry clips content when the user enlarges text                                      | 200% text, 320px width                                 |
| Keyboard               | Reading order or focus order breaks                                                           | One route per scope                                    |
| Reachability           | A destination becomes unreachable at a narrow profile                                         | Tablet and phone profiles                              |

Consistency and keyboard share one traversal; geometry and topology-stability share the deferred
fixture. Cases already protected by an earlier row are not repeated in a later one.

Permanent versus one-time, classified against `docs/PYTEST_STYLE.md` and `docs/HUMAN_GUIDANCE.md`:

| Check                                                                                                                | Disposition | Reason                                                         |
| -------------------------------------------------------------------------------------------------------------------- | ----------- | -------------------------------------------------------------- |
| Identity, topology-stability, geometry, chrome-during-load, consistency, keyboard, reachability, text-resize oracles | Permanent   | Each catches a distinct regression named above                 |
| Availability-never-exceeds-boundary                                                                                  | Permanent   | Security-adjacent invariant                                    |
| Catalog parameter-subset test                                                                                        | Permanent   | Cheap and static; prevents an unconstructible destination      |
| Registry truthfulness and ledger regeneration                                                                        | Permanent   | Keeps navigation and its documentation honest as backends land |
| Network-request counts (M5)                                                                                          | One-time    | Proves the migration; would pin an implementation detail       |
| `npx tsc --noEmit` consumer sweep (M11)                                                                              | One-time    | The compiler is the check                                      |
| Per-milestone render scaffolding (M2-M4)                                                                             | One-time    | Superseded by the oracles above                                |

After the production-browser owner is restored, refresh the production screenshot corpus last, since
the Ribbon changes every screen. Until then, committed screenshots are historical reference only and
the production visual-acceptance gate remains unrun.

## Risk register

| Risk                                                                                     | Impact                                           | Trigger             | Owner       | Mitigation                                                                                                                                                                                                         |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------ | ------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Truthful admission empties the Ribbon, since only the entry surfaces have handlers today | Shell looks broken in the live stack             | M3 registry lands   | WS-Contract | Registry state is per destination and reversible in one line; the design fixture renders the full schema so design review never depends on backend coverage                                                        |
| Navigation briefly duplicated during M10                                                 | Two course navigations visible                   | M10 patch A         | WS-Shell    | Deliberate ordering: duplicated beats absent; patch B removes it in the same milestone                                                                                                                             |
| Reserved empty Task Row costs phone vertical space                                       | Cramped Student attempt view                     | M9 phone profile    | WS-Ribbon   | Compact Context Row on coarse pointers; recover space by density, not by removing the row                                                                                                                          |
| The 12-consumer migration misses a site                                                  | Runtime undefined context                        | M11                 | WS-Shell    | Delete the old hook so the compiler enumerates every site                                                                                                                                                          |
| Ribbon visibility mistaken for authorization                                             | Security regression                              | Any milestone       | WS-Contract | `withRouteAccessBoundary` untouched; the availability-never-exceeds-boundary gate is permanent                                                                                                                     |
| Density pursued past legibility or tappability                                           | Cramped labels, missed touch targets             | M9b                 | WS-Ribbon   | Coarse-pointer padding meets the touch-target minimum as an M9 exit criterion; contrast and forced-colors assertions run on every theme; the Student attempt surface is exempt from the density argument by design |
| Icon chosen for a destination with no conventional glyph misleads more than it helps     | Wrong mental model of a destination              | M9a glyph selection | WS-Ribbon   | Labels are always present beside icons except for declared conventional-glyph entries; the M7 fixture makes the whole set reviewable at once, where a weak glyph is obvious next to its neighbors                  |
| Icon webfont or CDN reintroduced for convenience                                         | Unlabelled controls when fonts or network fail   | M9a                 | WS-Ribbon   | Bundled sprite is the declared delivery; the no-network-request assertion fails a CDN reintroduction                                                                                                               |
| Density read as a licence to shrink type                                                 | Instructor labels below comfortable reading size | M9b                 | WS-Ribbon   | Compact class tightens padding and spacing first; type steps down at most one step and never below the shared scale's body-adjacent value                                                                          |

## Rollout and release checklist

- [x] `./check_codebase.sh` clean after every milestone.
- [ ] `./run_playwright_tests.sh` clean after M9, M9b, M10, and M11. This remains unclaimed because
      the wrapper requires documented human-owned `PLE_*` real-stack inputs; focused Chromium fixtures
      do not substitute for live-stack acceptance.
- [x] Final aggregate acceptance: the task-owned archive path is present and the superseded active
      path is absent in both the index and worktree; `./all_test.sh` passed on the final tree.
- [ ] Production screenshot corpus and visual acceptance: pending the restored human-input
      production-browser owner. The committed `docs/screenshots/` corpus is historical reference only;
      fresh temporary Chromium fixture captures and visual inspection do not publish a current corpus.
- [x] `docs/CHANGELOG.md` entry added per milestone under the current date heading.

## Documentation close-out requirements

- Active plan: the current copy is archived at `docs/archive/ribbon_application_shell.md`; its
  superseded path, `docs/active_plans/active/ribbon_application_shell.md`, is absent from both the
  index and worktree. No active file remains to move.
- `docs/CHANGELOG.md`: one entry per milestone, including the deferred or omitted destinations and
  why, so the record stays a learning log.
- Archive notes: the three `plan-velvet-brewing-llama*.md` historical plans are archived under
  `docs/archive/`.

## Resolved decisions

- **No session membership index.** The earlier plan's `courseMemberships` field on the session is
  dropped, not deferred. Ribbon topology derives from Ribbon Scope and immutable Product Role
  (`docs/DESIGN_DECISIONS.md:783-785`), so the index was only ever supplying the Course Instance
  title. The title is a label in a fixed-height slot, filled from the existing course projection when
  it resolves; a late label moves nothing.
- **Sysadmin route audit is closed.** No route declares `sysadmin`; only the dead branch in
  `route_access_boundary.tsx` remains, and M11 removes it.
- **My Question Drafts is a Ribbon Task, not a Product Slot**, per
  `docs/INTERFACE_TERMINOLOGY.md:92-96`. Its route is a placeholder today, so the registry marks it
  unbacked and M11 drops it from live navigation.
- **Unbacked destinations keep their routes; only literal placeholders lose them.** A real page stays
  mounted, access-gated, and unlinked, so no working code is deleted and a pasted URL still resolves.
  Route existence and Ribbon admission are separate concerns, which is what makes the registry the
  single visibility lever. `workspaceList` (`src/pages/contract_pages.tsx:39-43`) is the only literal
  placeholder today, and M11 drops it from navigation.
- **Instructor Accounts stays in the designed Sysadmin Product ordering and is omitted until backed.**
  It has no route and no handler, so the registry marks it unbacked and the shipped Ribbon omits it;
  the schema keeps its designed position so the capability appends without moving a visible control.
  M12 records that append-point so the guide and the delegated vocabulary agree.
- **Schema reorganization is expensive and deliberate; growth appends.** Shneiderman records that
  expert users struggled to adapt when the ribbon reorganized commands they already knew, "highlighting
  the challenge of versioning and menu reorganization in professional applications"
  (`Designing_the_User_Interface...-2018.md:2329`). PLE's schemas are therefore designed once, and a
  new capability takes a declared position that appends rather than reshuffling learned positions. A
  future reorganization is a product decision with a migration cost, not a refactor.
- **PLE borrows Ferrum's density principle and adapts its control system.** The three-field split
  (`role`, `priority`, `presentation`), the ordered discrete collapse, the single group boundary, and
  selection-by-styling transfer directly. Icon-_only_ presentation does not: Ferrum's compact controls
  work because chemistry notation is itself the icon, while PLE's destinations have no conventional
  glyph. PLE pairs icon with label, reserving icon-only for declared conventional-glyph Context
  Controls and for the narrowest profile. PLE has two presentation classes rather than three, and no
  `large` class at all.
- **Icons ship as a bundled SVG sprite subset of Font Awesome Free, not an icon webfont and not a
  CDN.** A webfont fails open into unlabelled controls and announces junk to some assistive
  technology; a CDN adds an untested runtime dependency. The icon is decorative, the label is the
  accessible name, and the sprite carries only the glyphs the catalog references. `README.md`
  attributes the redistributed SVG icon artwork under CC BY 4.0 specifically, rather than describing
  the whole dependency under one license.
- **Icon-bearing is a declared property of a catalog entry, not a requirement on every destination.**
  The gate proves declared intent is complete; it never forces a glyph onto a destination that reads
  better as text alone. This keeps the icon set meaningful as capabilities append.
- **Authority chain.** `docs/HUMAN_GUIDANCE.md` and `docs/TERMINOLOGY_CONTRACT.md` govern; interface
  vocabulary reaches the plan through the terminology contract's delegation at `:1188-1190`. The UI
  guide, design decisions, and interface-terminology structure statements are current record that
  this plan may correct, so no milestone is blocked waiting on a document edit.
- **The three earlier plan files are superseded.** This plan absorbs the ribbon architecture from
  `plan-velvet-brewing-llama.md`, the truthful-admission rule and `docs/ux/` artifacts from
  `plan-velvet-brewing-llama-updated.md`, and every correction in the terminology-updates file, which
  the current documents already satisfy upstream.

## Open questions and decisions needed

None block dispatch. The two questions the earlier draft carried are resolved above, since the
repository evidence already supported a direction.

- Non-blocking follow-up: the first relationship-dependent Slot arrives with Course Observer, Student
  Observer, or Grader. Its admission mechanism -- withholding until resolved, or resolving the suffix
  before presenting the scope's schema -- is an implementation choice at that time. The user-facing
  invariant is settled now and does not reopen: a Slot is never presented and then withdrawn.
