# Persistent ribbon shell for every user type

## Context

Clicking a navigation control changes the whole interface, and the clicked control itself moves.
This is a design defect, not a styling defect: **the navigation chrome lives inside the region that
is destroyed on navigation, and its shape depends on data fetched after the click.**

The repository already settled the intended design; the code contradicts it:

- `docs/DESIGN_DECISIONS.md:762-770` - "one spatial owner ... A persistent navigation landmark
  preserves spatial memory and makes a tab change feel like changing tasks inside one course."
- `docs/UI_DESIGN_GUIDE.md:63-68` - "Global navigation is quiet and persistent."
- `src/components/course_management_frame.tsx:50` claims it "Keeps course context and the Instructor
  ribbon stationary while route content changes below it." It does not.

Outcome: one Office-style ribbon (Ribbon Context Row, Ribbon Tab Row, Ribbon Task Row) created once, owned by the
shell, with identical geometry for Sysadmin, Instructor, and Student, and adaptable to the planned
Course Observer, Student Observer, and Grader roles (`docs/HUMAN_GUIDANCE.md:55`, `:185-187`).

Pre-production, so the route contract, the session contract, chrome ownership, and the CSS shell are
all in scope (`docs/HUMAN_GUIDANCE.md:26`).

## Reference stability

Line numbers in this plan were read at commit `6c38dfc` and the working tree is moving under active
refactoring - several shifted while this plan was being written (`app.tsx` keyed `Show` 205 to 200,
`AuthSessionResponse` 126 to 113, the `RunReference` ledger row 204 to 257). **Treat the symbol name
as authoritative and the line number as a hint**: locate `AuthSessionResponse`, not `auth.rs:113`.
Every milestone below is stated against a symbol or file, so drift costs a grep rather than a wrong
edit.

## Root cause chain

Six independent causes stack into the observed jump. All six must go.

| # | Cause | Evidence |
| --- | --- | --- |
| 1 | Every URL change tears down the entire `<main>` subtree, including chrome a page rendered | `src/app.tsx:205` |
| 2 | The course ribbon is mounted from inside route-data resolution, so chrome depends on a fetch | `course_theme_scope.tsx:115-121` |
| 3 | While that fetch runs the whole page **including the ribbon** is replaced by a one-line block | `course_theme_scope.tsx:85-92` |
| 4 | Course identity above the ribbon is variable height: `h1` on course home, `p` elsewhere, wrapping at `52ch` | `course_management_frame.tsx:60-71`, `course_management_frame.css:7-11` |
| 5 | The ribbon shrink-wraps its items and reflows into 3/2/1 grid columns by breakpoint | `course_management_nav.css:3-14`, `:37-59`; `assignment_workspace.css:8-20` |
| 6 | The content column width changes per route via seven `data-route-surface` overrides | `src/style.css:274-289` |

Compounding, and worth stating precisely because it changes the fix: `route_access_boundary.tsx:75-80`
re-instantiates `CourseThemeScope` per route, so moving between two routes **in the same course**
re-runs the pair at `course_theme_scope.tsx:48-82`. Of that pair, `courseScope` is already
router-cached (`src/api/runtime.tsx:78-87`, `query(...)` keyed `"course-scope"`), but
`resolveCourseRoute` is a **plain uncached call** to `client.resolveNavigation`
(`src/navigation/resolved_route.ts:23-30`). The cached query cannot even be requested until that
uncached round-trip returns, so the loading fallback renders on every single navigation. Caching the
reference-to-identity resolution is therefore part of the fix, not an optimization.

## Target architecture

### The shell contract

The governing idea, and the one that makes everything else fall out:

> **The ribbon is a designed interface surface, not a generated menu. Application navigation plugs
> into predefined ribbon slots. Routes and authority change the state and contents of those slots,
> but do not determine the ribbon's structure.**

The inversion matters. The failing model is:

```
route + authority + fetched data  ->  construct some navigation  ->  hope the layout holds
```

The model this plan builds is:

```
Ribbon design (authored, fixed)          Application state
+-- Identity row                                 |
+-- Tab row                                      | fills slots,
|   +-- slot 1  slot 2  slot 3 ...   <-----------+ sets slot state,
+-- Command row                                    selects one
    +-- cluster 1
    +-- cluster 2
```

The frame exists independently of whatever function happens to be active - the property that makes a
desktop application toolbar feel like part of the application rather than part of the page.

That yields a clean split of ownership, and each layer may affect only its own column:

| Layer | Owns | May never change |
| --- | --- | --- |
| Design system | Geometry, spacing, hierarchy, overflow, interaction states, visual grouping | - |
| Ribbon contract | Which Ribbon Slots exist for each Ribbon Scope | Geometry |
| Authority | Whether a slot is available or pending | Which slots exist, or their order |
| Route | Which slot is selected, which command cluster fills the row | Slot list, geometry |
| Network data | Labels and theme color | Anything structural |

Condensed to one line for the milestones to test against:

> **Ribbon topology comes from scope. Authority supplies slot state. Routes supply selection.
> Data supplies labels and color.**

Deriving topology from all four is what allows controls to move. Separating them is what makes
stability provable rather than likely.

A practical consequence worth stating, because it changes how the work is built and verified: **the
ribbon can be designed, rendered, and reviewed with no application behind it.** `AppRibbon` takes a
`RibbonModel` prop and nothing else, so every state it can ever occupy is reachable from hand-written
values. That is why milestones 14-17 land before the shell integration in 18-19, and it is what makes
the visual gates automatable.

The full invariant this plan commits to:

> The ribbon has one shell-owned DOM instance and one fixed geometry for the lifetime of the
> authenticated application shell. A scope change may replace the declared slot model. A route change
> may change selection and command contents. Asynchronous data may change labels, enabled state, and
> appearance. **None of them may change ribbon row geometry or the content origin.** And no control
> present at one moment changes position at the next.

Three consequences, each of which is a change from the previous draft:

1. **All three rows are always reserved**, including the Ribbon Task Row on tabs that declare no
   commands. This reverses the earlier "the content origin moves only on tab change" compromise. The
   cost is a band of vertical space on command-less tabs; the gain is the clean invariant that
   *nothing below the ribbon ever moves because of ribbon navigation*. Given that the reported defect
   is controls moving after a click, that trade is worth making.
2. **Topology is synchronous and network-independent.** A projection response can never be required
   to decide how many slots exist or where they sit.
3. **Reconciliation updates slot state, never the slot list.** Late-arriving authority changes a
   slot's state; it does not rebuild the row.

This forces one contract change: the session carries the viewer's own course memberships, so course
title and course role are known without a fetch.

### The session carries the membership index

`AuthSession` (`src/api/contracts.ts:165`) gains:

```ts
readonly courseMemberships: ReadonlyArray<{
  readonly reference: CourseReference;   // public "C-12" form, never a UUID
  readonly title: string;
  readonly role: CourseMembershipRole;   // "student" | "instructor"
}>;
```

It is the viewer's own membership list - the same data the course list already renders - so it adds
no FERPA exposure beyond what `/` already shows.

Server side this is a narrow, contained change: `AuthSessionResponse`
(`crates/server/src/auth.rs:126-131`) gains the field, and `session_response`
(`crates/server/src/auth.rs:341-343`) fills it. Note the one real cost: `session_response` is
currently a pure projection of a `SessionRecord`, so it must gain a membership lookup and its caller
`session_handler` (`crates/server/src/auth.rs:299`) becomes the query site. That is one query per
session bootstrap, not per navigation - the trade that removes a per-route fetch from the chrome.

Watch the field-name mismatch while wiring: the server projection carries singular
`role: UserRole` (`auth.rs:142`) while the browser contract exposes
`roles: ReadonlyArray<UserRole>` (`src/api/contracts.ts:165`). Follow the existing decoder rather
than assuming either shape.

Carry `reference`, `title`, and `role` only. Do **not** put `CourseId` in the membership index: it is
an internal identity, and keeping it out preserves the no-UUID rule
(`docs/HUMAN_GUIDANCE.md:67`, `docs/DESIGN_DECISIONS.md:889`).

**Bound the set explicitly.** "The same data the course list renders" is not yet a definition:
`listCourses` returns a `CursorPage<CourseSummary>` (`src/api/client.ts:415`), so the course list is
paginated and an instructor's memberships can grow without limit across terms. Reuse the existing
current-membership query that backs page one rather than inventing a second notion of "my courses",
and treat the index as bounded to current memberships.

This degrades gracefully by design: a course outside the index is simply the unknown-course state
defined below - zero tabs, then filled in when the projection resolves. So a large or archived
history costs a small first-paint difference on rarely visited courses, never a wrong tab set.

#### Authority and staleness of the membership index

State the rule plainly, because the whole design rests on it:

- The membership index is authoritative for **the Ribbon only**, and only until the session refreshes.
  It never authorizes anything. `withRouteAccessBoundary` and the server remain the sole enforcement
  points, exactly as today.
- The loaded projection is authoritative and **does carry the reconciling value**: `CourseSummary`
  already includes `role: CourseMembershipRole`, documented as "Signed-in user's authority for this
  course" (`crates/question_model/src/course.rs:40`), and the frontend decoder already requires it
  (`src/api/decoders/catalog_course.ts:523`, `:529`). The reconciliation mechanism therefore exists
  rather than being wished for.
- Reconciliation rule: `RouteScopeIdentity.courseRole` reads the projection's `summary.role` when
  `useRouteScopeData()` has resolved for the active reference, and falls back to the session
  membership entry until then. Title follows the same precedence. A renamed course updates a
  fixed-height label in place, so nothing moves; a changed role updates the tab set, where
  correctness outweighs stability.
- **Snapshot consistency, stated honestly.** Because both legs are router-cached, chrome and course
  metadata are a snapshot for the query-cache lifetime, not a live view. A to B back to A serves
  cached A, so a rename made elsewhere in that window is not reflected. That is acceptable precisely
  because the Ribbon authorizes nothing: the server re-decides on every real request, and a revalidated
  query or `session.retry()` (`session_context.tsx:41`) refreshes the snapshot. The plan does not
  claim the UI is fresher than it is.
- There is no background membership refresh and no push channel; both are out of scope.

#### A course absent from the index is its own presentation state

A valid URL may reference a Course Instance the index does not list: a stale bootstrap, a
just-granted enrollment, or one the viewer genuinely cannot access. Chrome must not guess, and it
must not restructure itself later.

The slot model gives this a clean answer - an **unresolved Course Instance ribbon state**, not an
empty one:

- `scope` stays `"courseInstance"`, so the Course Instance slot schema is used. It never falls back
  to the global schema.
- Slots whose authority is not yet known carry `state: "pendingAuthority"`, rendered in place with
  their labels and their geometry, not omitted.
- The Ribbon Context Row shows its fixed-height label slot with the product label; `contextLabel` fills in
  when the projection resolves.
- Content is unaffected: the access boundary and the server produce the real answer - the page, a
  denial, or a not-found.
- When the projection resolves, slots move from `pendingAuthority` to `available`, or disappear from
  the end of the row under the append-only ordering rule. Controls already usable do not move.

This keeps presentation and enforcement separate - chrome still authorizes nothing - while replacing
"zero tabs, then seven tabs" with one stable row whose slots change state. An invalid reference
(`kind: "invalid"`) renders the same unresolved state for the scope it names.

This is the single change that makes the ribbon data-independent. Without it, course tabs cannot be
chosen until a fetch resolves, and any later-arriving tab set is a visible change.

### Ribbon behavior is declared on the route contract

Extend the frozen table in `src/route_contract.ts` rather than adding a parallel list.

```ts
export type RibbonScope = "product" | "courseInstance" | "assignmentAttempt";
export type ContentLayout = "reading" | "fullWidth";

export type TabId =
  // Product Ribbon Scope. Draft Questions is a Task under Question Library, not a slot.
  // Account and profile live in the Context Row corner, so there is no account tab.
  | "coursesTab" | "questionLibraryTab" | "blueprintCoursesTab" | "instructorApprovalsTab"
  // Course Instance Ribbon Scope
  | "courseAssignmentsTab" | "courseStudentsTab" | "courseGradebookTab"
  | "teachingOperationsTab" | "blueprintUpdatesTab" | "courseGradeSettingsTab"
  | "courseAppearanceTab"
  // Assignment Attempt Ribbon Scope
  | "assignmentAttemptTab";

export type RibbonTaskGroupId = "assignmentWorkspaceCommands";

export interface RouteRibbon {
  readonly scope: RibbonScope;
  /** Absent for routes reached from the Context Row account menu; the Tab Row shows no selection. */
  readonly tab?: TabId;
  /** Absent means this tab shows no Ribbon Task Row. */
  readonly taskGroup?: RibbonTaskGroupId;
  readonly contentLayout: ContentLayout;
}

export interface RouteContract {
  readonly id: RouteId;
  readonly path: string;
  readonly surface: string;
  readonly requiredRoles: ReadonlyArray<UserRole>;
  readonly ribbon: RouteRibbon;          // new
}
```

Every one of the 32 rows gains a `chrome` value. Representative rows:

```ts
{ id: "courses", path: "/", ...,
  ribbon: { scope: "product", tab: "coursesTab", contentLayout: "reading" } },

{ id: "courseAssignments", path: "/courses/:courseRef", ...,
  ribbon: { scope: "courseInstance", tab: "courseAssignmentsTab", contentLayout: "reading" } },

{ id: "assignmentWorkspaceQuestions",
  path: "/instructor/courses/:courseRef/assignments/:assignmentRef/questions", ...,
  ribbon: { scope: "courseInstance", tab: "courseAssignmentsTab",
            taskGroup: "assignmentWorkspaceCommands", contentLayout: "fullWidth" } },

{ id: "gradebook", path: "/instructor/courses/:courseRef/gradebook", ...,
  ribbon: { scope: "courseInstance", tab: "courseGradebookTab", contentLayout: "fullWidth" } },

{ id: "signIn", path: "/sign-in", ...,
  ribbon: { scope: "product", tab: "entryTab", contentLayout: "reading" } },
```

`contentLayout` replaces the seven ad-hoc `data-route-surface` width overrides at `src/style.css:279-289`.
Keep the `data-route-surface` **attribute** - Playwright specs and screenshot selectors use it - and
delete only the width rules keyed on it.

`courseManagementSectionForRoute` (`course_management_frame.tsx:17-48`) and
`assignment_workspace_paths.ts` are already pure route-to-section maps. They are absorbed by
`chrome.tab` and `chrome.taskGroup`, and their files retire.

### One parameter extractor, derived from the declared path

`course_theme_route.ts:18-32` sniffs path prefixes and indexes `segments[1]` / `segments[2]` by hand.
Replace with a pure extractor that zips the declared pattern against the pathname, in a new
`src/navigation/route_params.ts`:

```ts
/** Zips a matched route's declared pattern with the live pathname. */
export function routeParams(route: RouteContract, pathname: string): Readonly<Record<string, string>>;

export type RouteScopeKey =
  | { readonly kind: "product" }
  | { readonly kind: "course"; readonly courseReference: CourseRouteReference }
  | { readonly kind: "assignmentAttempt"; readonly attemptReference: RunRouteReference }
  /** A scoped route whose reference did not parse. Never global, by construction. */
  | { readonly kind: "invalid"; readonly scope: "courseInstance" | "assignmentAttempt" };

/** Uses the declared chrome scope plus the declared parameter names; no prefix sniffing. */
export function routeScopeKey(pathname: string): RouteScopeKey;
```

`routeScopeKey` reads `route.chrome.scope`, pulls `courseRef` or `runRef` from `routeParams`, and
validates it with the existing branded parsers `parseCourseReference` / `parseRunReference`
(`src/navigation/public_route.ts:36`, `:50`).

A malformed reference on a scoped path yields `"invalid"`, **not** `"global"`. Today's collapse to
global (`course_theme_route.ts:20`, `:27`, `:31`) was harmless when "global" only meant "skip the
theme", but in this architecture `"global"` also selects the global tab set - so a malformed
`/courses/nonsense` URL would briefly wear account-level chrome. `"invalid"` renders the same
zero-tab course chrome as an unknown course, while the router's `*unmatched` route and the access
boundary continue to own the content answer. `course_theme_route.ts` then retires.

### Chrome derivation

New `src/ribbon/ribbon_contract.ts` holds the catalogs and the pure derivation. No JSX, no fetching.

```ts
export interface TabDefinition {
  readonly id: TabId;
  readonly scope: RibbonScope;
  readonly label: string;
  /** The route this tab navigates to; its requiredRoles are the tab's global authority. */
  readonly destination: RouteId;
  /** Course-scope tabs additionally require one of these membership roles. */
  readonly courseRoles?: ReadonlyArray<CourseMembershipRole>;
  readonly order: number;
}

/** A Ribbon Task: a navigation link to one task-specific destination. Never an operation. */
export interface RibbonTaskDefinition {
  readonly id: string;
  readonly group: RibbonTaskGroupId;
  /** Ribbon Task Area: a presentation-only heading for adjacent Tasks sharing one purpose. */
  readonly area: RibbonTaskAreaId;
  readonly label: string;
  readonly destination: RouteId;
  readonly order: number;
}

export type RibbonTaskAreaId =
  | "assignment" | "delivery"          // Assignment workspace
  | "questionSets" | "questionRelationships"  // Question Library
  | "courseConfiguration";             // Course setup
export const RIBBON_TASK_AREA_LABELS: Readonly<Record<RibbonTaskAreaId, string>>;

export const TAB_CATALOG: ReadonlyArray<TabDefinition>;
export const RIBBON_TASK_CATALOG: ReadonlyArray<CommandDefinition>;

export interface RibbonAuthority {
  /**
   * The Account's one immutable Product Role. Session storage carries one role, never a
   * collection (DESIGN_DECISIONS.md), and Course Membership Role must match it - so this
   * single synchronous value selects the schema in every Ribbon Scope.
   */
  readonly productRole: UserRole;
}

/** Fills a declared path template from resolved scope references. */
export function buildRoutePath(routeId: RouteId, params: Readonly<Record<string, string>>): string;
```

#### The ribbon model: topology from scope, state from authority

This is the piece that upgrades the ribbon from "a component that survives navigation" to a stable
application shell. `AppRibbon` never reads the catalogs, the session, or a resource directly. It
renders exactly one synchronous value:

```ts
/** TERMINOLOGY_CONTRACT.md, Ribbon Availability. Distinct from Selected and Loading. */
export type RibbonAvailability =
  | "available"    // presentation facts make the destination appropriate as a live link
  | "checking"     // the relationship facts needed for presentation are still loading
  | "unavailable"; // the known relationship excludes this destination

export interface RibbonTabSlot {
  readonly tab: TabId;
  readonly label: string;
  readonly href: string;
  readonly availability: RibbonAvailability;
  /** Destination matches the current route. Never called "active" - that is a domain-state term. */
  readonly selected: boolean;
  /** A navigation to this destination is still in progress. */
  readonly loading: boolean;
}

export interface RibbonIdentity {
  readonly productLabel: string;              // "Peptidyle"
  readonly contextLabel: string | undefined;  // Course Instance title once known
}

export interface RibbonModel {
  readonly scope: RibbonScope;
  readonly selectedTab: TabId;
  readonly tabs: ReadonlyArray<RibbonTabSlot>;
  readonly commands: ReadonlyArray<RibbonCommandSlot>;  // empty is legal; the row still exists
  readonly identity: RibbonIdentity;
}

/** Pure. URL plus session snapshot in, topology out. Never awaits anything. */
export function deriveRibbonModel(
  pathname: string,
  authority: RibbonAuthority,
  identity: RibbonIdentity,
): RibbonModel;
```

The derivation pipeline, with each input allowed to affect exactly one thing:

```
scope        (from URL)      -> which slot schema, therefore the slot list and its order
authority    (from session)  -> each slot's state
route        (from URL)      -> selectedTab, selected, and which command group fills the row
loaded data  (from network)  -> identity.contextLabel and theme color ONLY
```

Because `deriveRibbonModel` takes no resource and returns no promise, topology cannot depend on the
network by construction - a stronger guarantee than a convention that says it should not.

#### Each Product Role gets its own ribbon, and that makes topology fully synchronous

Instructor, Student, and Sysadmin use the same Application Shell, the same three rows, the same slot
model, and the same geometry contract - and **completely distinct menus**. The schema is selected by
scope *and* Product Role:

```ts
/** Pure lookup. Both inputs are synchronous, so topology never awaits anything. */
export function ribbonSchemaFor(scope: RibbonScope, productRole: UserRole): ReadonlyArray<TabId>;
```

This is a stronger guarantee than the earlier draft achieved, because of two facts already settled in
the repository:

- **Product Role is immutable and lives in the session.** "Account creation assigns one immutable
  Product Role" (`docs/TERMINOLOGY_CONTRACT.md`), and session storage "carries one role, never a
  collection" (`docs/DESIGN_DECISIONS.md:570-571`). So it is known on the first frame, always.
- **Course Membership Role must match Product Role** (`docs/DESIGN_DECISIONS.md:572`). An Instructor
  Account is Instructor in every Course Instance it belongs to; a Student Account is Student. So the
  Course Instance ribbon shape is *derivable from Product Role* and never needs a membership lookup.

Together: **ribbon topology is a pure function of two synchronous inputs.** No network response can
affect which slots exist, in any scope, for any role - not by convention, but because neither input
can arrive late.

What this retires from the earlier design:

- The `Checking` availability state is not needed for Product Role narrowing, since Product Role is
  synchronous. It stays in the model for any genuinely async slot fact.
- The ordering rule is now in the contract, and in a stronger form than I proposed: universal slots
  first, role-narrowed slots as **one suffix**, and *"resolve availability for the complete
  role-narrowed suffix before rendering any of its controls, then render its Available slots together
  in their predefined order"*. Batching the whole suffix is better than my append-only phrasing -
  it rules out a partially-filled suffix, not just reordering. Keep it: with Product Role synchronous
  it costs nothing today, and it protects the future Course Observer, Student Observer, and Grader
  relationships, which are not derivable from Product Role.
- The session membership index narrows in purpose. It no longer supplies role, only the Course
  Instance **title** for the Ribbon Context Row label. That is a label, not geometry, so it may
  resolve late without moving anything. Milestone 1's "current membership" definition still matters,
  but a stale or missing entry now costs only a late title.

The one remaining asynchronous question - whether this Account has a membership in *this* Course
Instance - is authorization, answered by `withRouteAccessBoundary` and the server in the content
region. It never changes which slots the ribbon shows.

#### Slot order is fixed within a schema

Within one scope the slot schema is a fixed ordered list. Authority does not rebuild it; it marks
slots. The schemas:

Labels are the canonical visible surface names from `docs/TERMINOLOGY_CONTRACT.md`, not the older
`docs/UI_DESIGN_GUIDE.md` wording:

**One Ribbon Schema per (Ribbon Scope, Product Role) pair** - the contract's term for the predefined
ordered set of Ribbon Slots and Ribbon Tasks. Same Application Shell, same three rows, same slot
model, same geometry contract - completely distinct menus.

| Ribbon Scope | Instructor | Student | Sysadmin |
| --- | --- | --- | --- |
| Product | Courses, Question Library, Blueprint Courses | Courses | Courses, Instructor Approvals |
| Course Instance | Assignments, Students, Gradebook, Teaching Operations, Blueprint Updates, Course Setup | Assignments | Teaching Operations |
| Assignment Attempt | - | Attempt | - |

**Account and Profile destinations are Ribbon Context Controls**, in the upper corner of the Ribbon
Context Row, for every Product Role. They are not Ribbon Slots. Their routes carry **No Selected
Ribbon Tab**: the Ribbon Schema stays present with nothing selected, rather than a phantom or hidden
tab.

Three problems disappear at once, which is the sign the model is right:

- **Instructor Approvals stops being awkward.** It is simply a slot in the Sysadmin schema, not a
  shared-list slot that must be Unavailable for everyone else.
- **The Account ordering conflict vanishes.** There is no longer a universal Account slot fighting the
  "universal slots first" rule for last position in the row.
- **The role-narrowed suffix mostly empties out.** Each role's schema already contains only its own
  slots, so there is little left to narrow. The suffix rule stays in force for the future Course
  Observer, Student Observer, and Grader relationships, which are not derivable from Product Role.

A wording consequence for the contract: "one stable structure for each Ribbon Scope" should read
**one stable structure for each Ribbon Scope and Product Role**. Stability is preserved either way -
Product Role is immutable, so a given Account still sees exactly one structure per scope for its
entire session - but the key is a pair, not a single value.

Routes reached through a Ribbon Context Control - account security, co-instructor invitations,
sign-in - declare **no tab**. `RouteRibbon.tab` is therefore optional, and those routes render as
**No Selected Ribbon Tab**.

Grounding for each column:

- **Instructor** is the contract's Product Ribbon list plus the six-slot Course Instance row.
- **Student** is deliberately small. Students cannot reach the Question Library at all
  (`docs/HUMAN_GUIDANCE.md`: Students do not see the shared corpus), and Blueprint Courses is an
  Instructor reuse surface, so the Product row is two slots. In a Course Instance a Student's routes
  are Assignments and their own attempts; Gradebook, roster, and settings are Instructor surfaces.
  Assignment Attempt scope is Student-only, since it is one Student's exact attempt.
- **Sysadmin** follows "god-level account: Instructor vetting and account creation, help for non-tech
  Instructors fixing their courses" with "no ambient FERPA browsing"
  (`docs/HUMAN_GUIDANCE.md`, `docs/DESIGN_DECISIONS.md`). A Sysadmin cannot hold Course Membership,
  so its Course Instance row is the support surface only. Question Library is **excluded** on the
  narrow reading of ambient browsing: a Sysadmin supports Instructors rather than browsing Question
  content.

  That exclusion extends the milestone 7 route audit beyond the 12 course routes: `library`,
  `problemDetail`, `workspaceList`, and `workspaceEditor` currently declare
  `requiredRoles: ["instructor", "sysadmin"]`, and the `sysadmin` entry is removed from each so the
  route contract and the ribbon agree. Publishing content is an Instructor action
  (`docs/DESIGN_DECISIONS.md`), so no Sysadmin capability is lost.

**A misplacement this change corrects.** Instructor approval is a global Sysadmin duty, but today it
renders inside one Course Instance's Teaching Operations page and is gated on the retention
permission (`src/pages/teaching_operations_page.tsx:66-68`,
`teaching_operations/sysadmin_instructor_approval_panel.tsx`). Vetting an Instructor has nothing to do
with a particular course. Distinct ribbons give it its proper home: an **Instructor Approvals** slot
in the Sysadmin Product row. Moving the panel is part of milestone 20.

Because Product Role is immutable, a given Account sees exactly one column for its whole session. The
columns never mix, and no viewer ever watches slots appear or disappear.

#### Which document owns what

The terminology contract owns **terms**; it does not own **structure**. That split decides where each
decision above lives, and it means this plan waits on no further terminology edit.

| Concern | Owner |
| --- | --- |
| What a thing is called - Ribbon, Ribbon Scope, Ribbon Slot, Ribbon Availability, Selected, Loading, Page Action, Content Layout | `docs/TERMINOLOGY_CONTRACT.md` |
| Canonical visible names - Question Library, Blueprint Courses, Blueprint Updates, Course Setup, My Question Drafts, Starred, Watched | `docs/TERMINOLOGY_CONTRACT.md` |
| Which slots exist, in what order, for which Product Role | `src/ribbon/ribbon_contract.ts`, documented in `docs/UI_DESIGN_GUIDE.md` |
| How the Ribbon behaves and looks - geometry, grouping, states, overflow | `docs/UI_DESIGN_GUIDE.md` |

The terminology contract currently carries some structure that sits on the UI side of that line:
"Product Ribbon Scope has four ordered Ribbon Slots", "Course Instance Ribbon Scope has six ordered
Ribbon Slots", the Question Library task list, and the universal-then-suffix ordering rule. Those
statements are correct and useful; they simply belong in the UI document, and this plan treats the UI
document plus the executable contract as their home.

Two consequences:

- **Nothing here is blocked on a terminology edit.** Milestone 9 encodes the per-role schemas and the
  Context Row account controls directly. Milestone 11 checks *names* against the terminology
  contract - a slot labelled "Library" or "Curriculum" still fails - while *structure* is checked
  against the schemas in this plan.
- **One ledger row should shed its structural clause.** The row directing "Library -> Question
  Library" adds "Use the complete object name in the four-slot Product Ribbon: Courses, Question
  Library, Blueprint Courses, and Account." The naming instruction is right and stays; the four-slot
  enumeration is structure, is now superseded by the per-role schemas, and would otherwise teach an
  Account tab that this design deliberately removes. Worth trimming to the naming rule alone.

The Account-ordering conflict raised earlier is resolved by removal rather than by an exemption, and
needs no contract text at all.

#### Assignment Attempt: progress is context, question navigation is content

The Attempt schema is one slot, and its Ribbon Task Row carries one Task - **Back to Assignments** -
which is a real route and therefore a legitimate Task, giving a keyboard-reachable exit instead of a
browser-back guess.

**Questions are deliberately not numbered tabs.** Four reasons, the first decisive:

1. Slot count would depend on assignment length - 7 here, 25 there, unbounded in Endless mode - and
   would only be known after a fetch. That is topology derived from data, the exact defect this
   design removes, and it would overflow the row on any long assignment.
2. Availability would churn mid-attempt. The Assignment Navigation Rule is Forward Only or Free
   Navigation, so under Forward Only most numbered tabs would be dead and would flip to live as the
   Student advances - state churn in the surface meant to hold still.
3. A Ribbon Task is "a navigation link to one task-specific destination". Questions inside an attempt
   are not routes; there is one route with internal state. Numbered tabs would mean inventing a route
   per question.
4. `docs/UI_DESIGN_GUIDE.md:35` places it: "Student questions: keep prompt, response, feedback,
   navigation, and timer in one visual sequence." Question-to-question movement, and the timer, stay
   with the question, where they can also respect the per-assignment navigation policy.

What the ribbon does carry is **Assignment Attempt Progress** - the contract's term for the current
Question position within the Assignment Attempt - in the Ribbon Context Row, beside the Course
Instance and assignment labels. `docs/UI_DESIGN_GUIDE.md:65-68` treats assignment progress as its own
layer that "communicates sequence and state", separate from global and course navigation, and this is
the layer that benefits from persistence: it answers "where am I" without competing with the question.

Give that indicator a **fixed-width slot with tabular numerals**, sized for the largest expected
count. It is a label, so data may change it; a reserved width keeps it from nudging the Account
controls beside it as the number grows.

#### Course setup consolidates the two configuration surfaces

Grade settings and Appearance become Ribbon Tasks inside a **Course setup** slot rather than two
slots of their own, taking the Course Instance row from seven slots to six.

This follows ADAPT, which reaches only four in-course destinations - `assignments`, `gradebook`,
`analytics`, `properties` (`OTHER_REPOS/adapt/resources/js/router/routes.js`) - by folding
configuration into one Properties page. The consolidation stops there deliberately: Students and
Teaching Operations stay first-class, because roster work and course lifecycle are recurring teaching
tasks, not setup. ADAPT's tighter four-slot shape would push roster work behind an extra click, which
trades daily cost for a rarely-felt gain.

The mechanism is already in the contract - this is what the Ribbon Task Row is for. `courseSetupTab`
declares a task group whose Ribbon Tasks are Grade settings and Appearance, so both remain one click
from anywhere in the Course Instance while occupying one slot. Six slots also gives the phone profile
real headroom against the reachability oracle, where seven was the tightest case.

#### The Question Library Task Row: a GitHub-shaped semantic model

Question destinations divide on three orthogonal axes, which is what makes them a Task Row rather
than five competing top-level names:

| Task | Ribbon Task Area | GitHub analogue | Axis it expresses |
| --- | --- | --- | --- |
| All Questions | Library Views | the repository ecosystem | Discoverability - every Published Question |
| My Questions | Library Views | your repositories | **Ownership** |
| My Question Drafts | Library Views | private, unpublished repositories | **Publication state** |
| Starred | Question Relationships | starred repositories | **Question Star** - visible endorsement |
| Watched | Question Relationships | watched repositories | **Question Watch** - private subscription |

The semantic rule: *Library means discoverable; My means ownership; Draft means publication state;
Starred and Followed mean relationships to Questions.* Folders, tags, classifications, and search
facets are then organizational mechanisms inside these sets, never competing names for sets. This
also rules out "Personal Question Library", which names a location where the real distinction is
ownership.

So the Product Tab Row keeps four slots and the Question Library tab carries the five Tasks:

```
Tab Row      | Courses | Question Library | Blueprint Courses | Account |
Task Row     | All Questions  My Questions  My Question Drafts | Starred  Followed |
               <------ ownership and state ------>              <-- relationships -->
```

The two Ribbon Task Areas fall out of the axes themselves - sets defined by ownership and publication
state, then sets defined by relationship - which is exactly the "presentation-only heading for
adjacent Ribbon Tasks with one shared purpose" the contract describes.

Two items this raises that are **not** ribbon decisions and should be settled in the owning
documents:

- **Starred and Followed do not exist yet.** `src/` has no `starred` or `watching` state; they appear
  only as intent in `docs/HUMAN_GUIDANCE.md`. The ribbon can declare the slots, but the Tasks stay
  `unavailable` until the underlying relationships are built. This plan does not build them.
- **"Followed" diverges from both GitHub and the current guidance.** GitHub's repository term is
  *Watch*; *Follow* applies to people. `docs/HUMAN_GUIDANCE.md` currently defines "Watch =
  subscription". Adopting Followed is defensible as plainer English, but it needs a ledger row
  (`Watch -> Followed`) so the rename is tracked rather than drifting.
- **Question Collections and Saved Question Searches are already built** (`question_curation_model.ts`
  `QuestionCollectionReplacement`, `SavedQuestionSearchReplacement`, and their deletion variants).
  Treating Favorites and Saved Questions as unnecessary implies retiring or reframing implemented
  curation features - a product decision with real code behind it, outside this workstream's scope.
  Recorded here so the ribbon does not silently orphan those surfaces.

#### Why Draft Questions is not a Product slot

The Product row is organized by **object type**, with the mine-versus-shared distinction living
*inside* a tab rather than beside it. Draft Questions are Questions, so they belong within Question
Library - reached as a segment of it, alongside the Published corpus - not as a fifth top-level
destination.

Comparison evidence supports the axis while warning against the packaging. LibreTexts ADAPT's
instructor menu (`OTHER_REPOS/adapt/resources/js/components/Navbar.vue:310-327`) repeats the
mine/shared pair per object type - "My Questions" beside "Search Questions", "My Courses" beside
"Public Courses" and "Commons". The axis is real and recurring. But ADAPT surfaces each half as its
own entry, which is how that menu reaches ten items, and it then hides all ten behind a "dashboards"
dropdown - the pattern `docs/UI_DESIGN_GUIDE.md` explicitly refuses ("Do not add a generic Dashboard
dropdown that merely duplicates those destinations or hides them behind another navigation step").

ADAPT's Learning Tree entries are excluded from this comparison: PLE has no Learning Tree concept and
is not adopting one, so those rows carry no design weight here.

Folding mine into shared per object type keeps the Product row at four slots, which is comfortably
inside the count a top row can carry, and preserves PLE's advantage of flat, always-visible primary
navigation.

Route mapping consequence: `workspaceList` and `workspaceEditor` declare
`tab: "questionLibraryTab"`, so editing a Draft Question keeps the Question Library slot selected
rather than dropping the viewer into an unrelated top-level context. `library` and `problemDetail`
declare the same tab, which is what makes the two halves feel like one destination.

**This conflicts with two documents and needs the owner's ruling before milestone 9 encodes it.**
`docs/TERMINOLOGY_CONTRACT.md` lists **Draft Questions** as its own visible surface name, and
`docs/UI_DESIGN_GUIDE.md` says "Workspace owns drafts". Adopting the four-slot row is a contract
change - the Draft Questions surface becomes a segment of Question Library rather than a peer of it -
not merely a plan choice.

Note what is **absent** from the Course Instance row: today's "New assignment" nav entry
(`course_management_nav.tsx`) does not become a slot. The contract separates navigation from
operations - "Ribbon Tabs and Ribbon Tasks navigate; Page Actions perform operations" - so Create
Assignment is a Page Action on the Assignments page, not a Ribbon Slot. Retiring that nav entry is
part of milestone 20.

The ordering rule that makes late availability safe: **slots available to every Course Membership
Role come first, and role-narrowed slots follow.** In the Course Instance schema, Assignments serves
both Student and Instructor Course Membership Roles and sits first; the six Instructor-only slots
follow.

This rule is **not** in `docs/TERMINOLOGY_CONTRACT.md`, and it is the piece that makes its Ribbon
Availability states safe in practice. The contract defines a Ribbon Slot as "one stable ordered
position" and defines Unavailable, but does not say whether an Unavailable slot holds its position or
is omitted. Both readings satisfy the words; only one keeps controls still:

- Rendering Unavailable slots in place holds every position, but shows a Student six dead Instructor
  controls - the "too subtle / unusable controls" failure Practical UI warns about, and extra
  furniture the owner's padding guidance argues against.
- Omitting them is the right presentation, but then a `Checking -> Available` transition can insert
  slots, which moves controls unless ordering forbids it.

This plan omits Unavailable slots **and** enforces the append-only ordering rule, so late
availability can only extend the row to the right. Milestone 9c asserts it. Recommend recording the
ordering rule in the contract beside Ribbon Availability, since without it the contract permits a
conforming implementation that still moves controls.

The consequence is the guarantee the plan needs: when authority resolves late and adds slots, they
**append to the right of controls already on screen**, so no visible control changes position. A
Student never sees six controls they cannot use, and an Instructor's row is complete the moment
authority is known - which, for any indexed Course Instance, is the first frame.

A Sysadmin in Course Instance scope gets the same schema with only Operations available, per the
route audit below - again by slot state, not by a different row.

The eight course tabs carry `courseRoles: ["instructor"]` except `courseAssignmentsTab`, which
carries `["instructor", "student"]` - that is how one Ribbon Tab Row serves both roles in a course.

#### Sysadmin inside course scope: settle the route contract, then derive the tabs

The domain settles the membership question. `CourseMembershipRole` is `Student | Instructor` and
documents that **"Sysadmin is never a membership value"** (`crates/question_model/src/course.rs:14-25`).
`docs/DESIGN_DECISIONS.md:551-571` is stronger still: a Sysadmin provisions a course for an assigned
Instructor and "receives none" of the membership, has "no ambient FERPA browsing", and - stated
outright - **"Sysadmin accounts cannot hold course membership."** The owner's guidance agrees
(`docs/HUMAN_GUIDANCE.md:143`, `:153`).

Against that, 13 course routes currently declare `requiredRoles: ["instructor", "sysadmin"]`
(`src/route_contract.ts:153`, `:159`, and others). A Sysadmin therefore passes the route gate and
then meets a course-level denial, because course code requires `summary.role === "instructor"`
(`course_theme_scope.tsx:96`, `assignment_workspace_live_page.tsx:196`). That is dead permission
contradicting a settled decision, and this workstream is the right place to fix it, since it is
already editing every row of the contract and is the moment the contract becomes the single source
of chrome.

Audit each of the 13 rows against one rule:

> A course-scoped route keeps `sysadmin` only when it serves an administrative capability that is
> explicitly defined **without** course membership. Otherwise the entry is removed.

Applying that rule to the evidence already in the code decides every row without further input:

- **`teachingOperations` keeps `sysadmin`.** The page carries a sysadmin-gated capability today -
  retention extension at `teaching_operations_page.tsx:86-89` - and the sysadmin instructor-approval
  surface lives in the same area (`sysadmin_instructor_approval_panel.tsx:37`). Administrative
  lifecycle work is not FERPA content browsing, so it sits consistently beside
  `docs/DESIGN_DECISIONS.md:559`.
- **The other 12 course routes drop `sysadmin`.** Each one requires `summary.role === "instructor"`
  downstream, so the permission can never be exercised; removing it deletes an unreachable branch
  rather than changing behavior. `assignmentWorkspaceGradingOperations` (`:184`) and
  `studentWorkInspection` (`:208`) already declare instructor-only, which is the shape the rest
  converge on.

The chrome then falls out of the corrected contract: a Sysadmin has no `courseRole`, so `visibleTabs`
yields exactly one course tab, Operations, and no others. The global Ribbon Tab Row serves Sysadmins as it
does today. No chrome special case either way.

This is machine-checkable rather than a matter of judgment: a test asserts that every course-scoped
route declaring `sysadmin` has a corresponding sysadmin-gated capability recorded in the audit table,
and that the remaining course routes declare `["instructor"]`. If a future route adds `sysadmin`
without that entry, the suite fails.

#### Every destination must be constructible

`buildRoutePath` is a runtime failure waiting to happen unless parameter availability is pinned.
The repo declares six parameter names (`src/route_contract.ts`): `courseRef`, `assignmentRef`,
`runRef`, `problemRef`, `curriculumRef`, `workspaceRef`, plus `membershipRef` on the deepest
gradebook route (`:206`).

The rule: **a tab or command may only declare a destination whose parameters are all available from
the active scope key plus the active route's own parameters.**

| Catalog entry | Destination needs | Available from |
| --- | --- | --- |
| Product tabs (`coursesTab`, `questionLibraryTab`, `blueprintCoursesTab`, `accountTab`) | none | - |
| All eight course tabs | `courseRef` | scope key |
| `assignmentWorkspaceCommands` (5 entries) | `courseRef` + `assignmentRef` | scope key + current route params |

That third row is the constrained one: `assignmentRef` is not in the scope key, so the command group
may only be declared on routes that themselves carry `assignmentRef`. Today that is the five
`assignmentWorkspace*` routes plus `assignmentAccess` (`:188`) and `assignmentPreview` (`:194`) - all
of which should declare the group so the Ribbon Task Row survives a hop into Access or Delivery check.

Routes with no constructible destination simply declare no `taskGroup`; `studentWorkInspection`
(`:206`) is the clear case, since `membershipRef` and `runRef` exist nowhere else.

Make this checkable rather than a convention: a unit test asserts that for every catalog entry, the
parameter names in the destination's declared path are a subset of the parameters its declaring
routes provide. A missing parameter then fails the suite, not the browser.

**Adding a role is a catalog edit.** Course Observer and Student Observer become new
`CourseMembershipRole` values plus `courseRoles` entries; Grader likewise. No layout tree changes.

### Chrome visibility never grants access

Two different jobs, deliberately kept apart:

- `withRouteAccessBoundary` (`route_access_boundary.tsx:49-84`) stays exactly as it is. It is the
  fail-closed enforcement point and continues to re-derive from the pathname.
- `visibleTabs` is presentation only: it decides what to draw.

Both read `rolesMayAccessRoute`, so they cannot drift. A unit test pins the invariant: for every
authority fixture, every tab returned by `visibleTabs` has a destination that
`rolesMayAccessRoute` also permits. Chrome may show strictly less than the boundary allows, never
more.

### Route scope owner, keyed by reference

`useCourseThemeRouteData()` has **16 page-level consumers**, and they include the Assignment Attempt
pages (today's `run_page.tsx`, `run_summary_page.tsx`), not only Course Instance pages. So the
hoisted owner must cover Course Instance **and** Assignment Attempt scope, exactly as
`courseThemeRouteRequest` does today.

To keep that migration mechanical, **preserve the context value shape**. `CourseThemeRouteData`
(`course_theme_context.ts:8-11`) and the `courseRouteData(...)` normalizer (`:18-27`) stay as they
are; only ownership, keying, and the hook's home change. The 16 consumers then change an import and
a hook name, not their logic.

New `src/ribbon/route_scope_context.tsx`:

```ts
/** Geometry-bearing facts, known synchronously from the URL plus the session. */
export interface RouteScopeIdentity {
  readonly key: RouteScopeKey;      // global | courseInstance | assignmentAttempt | invalid
  readonly title: string | undefined;               // course title for course scope
  readonly courseRole: CourseMembershipRole | undefined;
}

export function useRouteScopeIdentity(): Accessor<RouteScopeIdentity>;

/** The existing loaded projection, unchanged in shape; drives color and page content. */
export function useRouteScopeData(): Accessor<CourseThemeRouteData | undefined>;
```

Resolution order:

1. `routeScopeKey(location.pathname)` gives the reference. The same reference across routes yields
   the same scope value, so **nothing recomputes while navigating inside one course.**
2. For course scope, title and `courseRole` come from `session.courseMemberships` by reference
   lookup - synchronous, so the Ribbon Context Row and the tab set are correct on the first frame. A
   reference absent from the membership list leaves them `undefined`; `withRouteAccessBoundary` and
   the server still own the actual denial.
3. The projection behind `useRouteScopeData` is fetched with `createResource` keyed on the
   **reference**, not the pathname, so it fires once per Course Instance or Assignment Attempt.
   Until it lands, theme
   variables fall back to the default `grass` anchors (`docs/DESIGN_DECISIONS.md:775`). Colors
   settle; geometry never moves.

Assignment Attempt scope keeps its own single-slot schema (`assignmentAttemptTab`), so a Student
working an attempt sees stable chrome rather than the current full-page swap.

There is no geometry-bearing "resolving" state: the ribbon always has a title slot and a tab set.

#### Why the provider stays mounted, and what caching actually guarantees

Two separate mechanisms, worth not conflating:

- **Mount persistence** is structural. `RouteScopeProvider` sits above the keyed `Show` in the
  "after" tree, so the router never tears it down; only the content region is keyed. This is what
  keeps the ribbon element alive, and it is what the identity oracle proves.
- **Refetch avoidance** is caching. `courseScope` is already a router `query` keyed `"course-scope"`
  (`src/api/runtime.tsx:78-87`), so repeat requests for the same course are served from cache.
  `resolveCourseRoute` is **not** cached (`resolved_route.ts:23-30`), which is precisely why the
  current UI blinks on every navigation.

So the plan adds a cached resolution query alongside the existing ones:

```ts
readonly resolveCourse: QueryFunction<[CourseRouteReference], CourseId>;   // key "course-resolve"
readonly resolveAssignmentAttempt: QueryFunction<[RunRouteReference], RunId>;  // "attempt-resolve"
```

With both legs cached and the provider mounted, the transitions behave as follows, and the gate
checks all of them rather than only same-course:

| Transition | Expected |
| --- | --- |
| Same course, route to route | Zero requests; zero chrome change |
| Course A to course B | One resolve + one scope request; identity and tabs change, rows do not move |
| A to B back to A | Zero requests for A, served from the query cache |
| Course Instance to Assignment Attempt and back | One resolve + one screen request per scope; provider never unmounts |

If any of these disagrees with the table, the resource keying is wrong and should be fixed before
downstream tasks assume it.

### Three rows, fixed geometry

```
+--------------------------------------------------------------+  Ribbon Context Row (fixed height)
| P Peptidyle | BIOL 101 - Cell Biology            [Account v]  |
+--------------------------------------------------------------+  Ribbon Tab Row (fixed height)
| Assignments | Students | Gradebook | Curriculum | Appearance   |
+--------------------------------------------------------------+  Ribbon Task Row (fixed height, per tab)
| Overview  Questions  Policies  Grading operations  Student view|
+--------------------------------------------------------------+
|   content region - the ONLY thing that swaps                  |
```

Layout rules:

1. Row heights are tokens: `--ple-ribbon-identity-block-size`, `--ple-ribbon-tab-block-size`,
   `--ple-ribbon-command-block-size`, summed into `--ple-chrome-block-size`.
2. Rows never wrap: one row plus horizontal overflow scroll replaces `width: fit-content` +
   `flex-wrap: wrap` + the breakpoint grids. Row count is constant, so height is constant at 1280x800
   and at the student phone profile alike (`docs/HUMAN_GUIDANCE.md:161`, `:172`).
3. **All three rows are always present, including an empty Ribbon Task Row.** The ribbon's block size is
   a constant for the lifetime of the shell, so the content origin never moves for any reason.

   This reverses an earlier draft that let the Ribbon Task Row appear and disappear per tab. That version
   bought a little vertical space and gave up the invariant; since the defect being fixed is
   *controls moving after a click*, the invariant is worth more. Keep the reserved empty row visually
   quiet - it is a band of the ribbon surface, not a bordered container - and recover the space by
   making the row compact rather than by removing it, which respects
   `docs/HUMAN_GUIDANCE.md` on padding without reintroducing movement.

   Expressed as a CSS contract rather than as component behavior:

   ```css
   :root {
     --ple-ribbon-identity-block-size: ...;
     --ple-ribbon-tab-block-size: ...;
     --ple-ribbon-command-block-size: ...;
     --ple-ribbon-block-size: calc(
       var(--ple-ribbon-identity-block-size) +
       var(--ple-ribbon-tab-block-size) +
       var(--ple-ribbon-command-block-size)
     );
   }
   .app-shell { display: grid; grid-template-rows: var(--ple-ribbon-block-size) minmax(0, 1fr); }
   ```

   No route, Product Role, loading state, error state, title length, theme, or page component may
   alter that block size. Because the shell grid is defined in terms of the token, a page cannot
   change the content origin even by accident.
4. The course title is a fixed-height, single-line, ellipsized label in the Ribbon Context Row. The page
   keeps its own `h1` for the task, which also removes today's duplicate-heading problem where the
   title is `h1` on one route and `p` on another.
5. `.shell` becomes `display: grid; grid-template-rows: auto 1fr`. The content region keeps one
   column width; `contentLayout: "fullWidth"` routes use a full-bleed inner region rather than resizing the
   column.
6. Loading and error states render **inside** the content region, never in place of chrome.

## Vocabulary

`docs/TERMINOLOGY_CONTRACT.md` owns canonical meanings and
`docs/VOCABULARY_REPLACEMENTS.md` is the live correction ledger. Because this workstream creates a
new contract surface, every identifier it introduces uses canonical vocabulary from the start -
adding new code in deprecated wording would enlarge the ledger instead of shrinking it.

Applied to the names above:

| Deprecated wording | Canonical target | Where it lands here |
| --- | --- | --- |
| Run, Assignment Run, `RunReference` (ledger rows for `RunReference` and Assignment Run) | Assignment Attempt, Assignment Attempt Reference | `RibbonScope` uses `"assignmentAttempt"`; tab `assignmentAttemptTab`; scope key `{kind:"assignmentAttempt", attemptReference}`; query `resolveAssignmentAttempt` |
| "global role", "user role" | **Product Role** (`TERMINOLOGY_CONTRACT.md`, Product Role and Course Membership Role) | `RibbonAuthority.productRoles` |
| bare "course role" | **Course Membership Role** | `RibbonAuthority.courseMembershipRole` |
| bare "Course" for a delivered course | **Course Instance** | `RibbonScope` uses `"courseInstance"`; prose says Course Instance |
| Problem (ledger: "Problem \| PLE-authored assessment content \| Question") | **Question**, Question Version, Question Catalog Entry | No new identifier here uses "problem". The existing `problemDetail` route id, `problemRef` parameter, and `ProblemRouteReference` are pre-existing ledger rows, left to their own correction |
| Learner | **Student** | Prose uses Student throughout |

### Visible labels now come from the contract

An earlier draft of this plan kept the owner's older `docs/UI_DESIGN_GUIDE.md` labels ("Library",
"Workspace", "Curriculum") on the grounds that the contract named no visible surfaces. It now does,
so that reasoning expires: `docs/TERMINOLOGY_CONTRACT.md` lists **Courses**, **Question Library**,
**Blueprint Courses**, **Draft Questions**, **Account**, plus **Teaching Operations** and **Blueprint
Updates** inside a Course Instance. The plan uses those.

Both identifiers and visible labels therefore follow the contract. `docs/UI_DESIGN_GUIDE.md` still
owns how navigation *behaves and looks*; it no longer owns what the surfaces are called, and
milestone 26 updates its wording so the two documents agree.

### Terminology the contract now owns

`docs/TERMINOLOGY_CONTRACT.md` gained an "Interface surfaces and ribbon navigation" section that
supersedes the working names this plan used earlier. The plan adopts it wholesale:

| Earlier working name here | Contract term |
| --- | --- |
| `RibbonScope`, `"global"` | **Ribbon Scope**, **Product Ribbon Scope** |
| Ribbon Context Row | **Ribbon Context Row** |
| Ribbon Tab Row / tabs | **Ribbon Tab Row** / **Ribbon Tabs** |
| Ribbon Task Row / commands | **Ribbon Task Row** / **Ribbon Tasks** |
| command cluster | **Ribbon Task Area** (presentation-only heading) |
| "Selected Ribbon Tab" | **Selected Ribbon Tab** - `Active` stays a domain-state term for Accounts and Course Memberships |
| slot state `pendingAuthority` | **Ribbon Availability**: Available, Checking, Unavailable |
| `contentLayout: "reading" \| "wide"` | **Content Layout**: Reading Layout, Full-width Layout |
| shell | **Application Shell** |

Two consequences beyond renaming:

- **Ribbon rows carry navigation only.** "Ribbon Tabs and Ribbon Tasks navigate; Page Actions perform
  operations." So `RibbonTaskDefinition` has no `kind: "action"` variant, and Create Assignment leaves
  the navigation entirely.
- **Availability, Selection, and Loading are three separate facts**, not one enum. A slot can be
  Available and Selected and Loading at once, and the ribbon renders each independently.

### Two collisions the contract exposes

**"Curriculum" meant two different things**, and the contract has now settled it better than this
plan had. The Product scope slot is **Blueprint Courses** (reusable, answer-free, no Students or
deadlines); the Course Instance slot is **Blueprint Updates** (reviewed changes from the parent
Blueprint Course). Both name the thing rather than describing it, which the earlier pairing of
"Curriculum" and "Curriculum changes" did not. The ledger's `Reusable Curriculum -> Blueprint Course`
and `Curriculum Adoption -> Blueprint Adoption` rows point the same way.

**"Course" alone is ambiguous** between Blueprint Course and Course Instance. In this plan, bare
"course" appears only in three legitimate places: existing code identifiers (`courseScope`,
`courseRef`, `CourseSummary`, `course_theme_scope.tsx`), compound adjectives on those identifiers
("course-scoped route"), and quoted prior wording. Every normative statement - slot schemas,
invariants, milestones, oracles - says **Course Instance**, because the ribbon is scoped to live
teaching, never to a Blueprint Course.

### Audit gate

Milestone 11 turns this from intention into a check: a script asserts that no identifier introduced
by this workstream matches the ledger's "current wording" column, and that the plan's own new names
(`assignmentAttempt`, `courseInstance`, `productRoles`, `courseMembershipRole`) appear in place of
their deprecated equivalents. Because the ledger is a live document that shrinks as corrections land,
the script reads it at run time rather than hard-coding a word list.

Scope is deliberately bounded: this plan does **not** rename the existing `runAttempt` /
`runSummary` route ids, `RunRouteReference`, or the server's `RunId`. Those are a separate ledger
entry spanning the server, and folding them in would couple a navigation fix to a cross-stack rename.
The rule is narrower and checkable: **new identifiers use canonical terms; existing ones are left for
their own correction.** A milestone verifies that no newly added identifier reintroduces a term from
the ledger's "current wording" column.

## Visual design

Target, in one line: **a modern Office ribbon crossed with macOS restraint** - one persistent
instrument panel on a neutral surface, with strong typography, grouping by proximity, and the course
theme used as a restrained accent.

This is the appearance half of the same decision the architecture makes structurally. It also serves
`docs/HUMAN_GUIDANCE.md:65` ("less bubbly, reduce excessive padding") and stays inside
`docs/UI_DESIGN_GUIDE.md:77` ("shape, position, text, and color together; color alone is not the
indicator").

### The ribbon is one object, not three stacked bars

Draw one outer ribbon surface with a single bottom edge against the content region. Internal row
separators stay extremely subtle - a hairline at low contrast, or none where spacing already
separates. People perceive wholes, so the ribbon should register as one panel above changing
content, which is exactly the mental model the architecture builds.

### Each row gets a distinct visual role

Consistency across the three rows would flatten the hierarchy. Differentiate them:

| Row | Visual role |
| --- | --- |
| Identity | Quietest. Slightly offset neutral; smallest type; the course is the strongest text here but well below an `h1` |
| Tabs | The strongest horizontal rhythm; clean navigation strip; unmistakable active state |
| Commands | Lighter workspace strip with visible clustering |

### Tabs: text, not icons, with an unambiguous active state

Use text labels. Icons are ambiguous to new and intermittent users, and small icons end up needing
labels anyway - and PLE's tab names ("Grade settings", "Teaching operations") have no conventional
glyph.

Avoid the near-invisible 2px underline. Combine a small number of reinforcing indicators - heavier
type weight, a 3px course-accent underline, and a very faint tinted background behind the Selected Ribbon Tab
- and keep `aria-current="page"`, which the existing navs already set
(`course_management_nav.tsx:25-27`). One strong indicator beats several loud ones.

### Commands: cluster by proximity, label the clusters

This is where the Office influence should read. Replace the current undifferentiated strip with
labelled semantic clusters:

```
  ASSIGNMENT                              DELIVERY
  Overview   Questions   Policies    |    Grading operations   Student view
```

Whitespace does the grouping work; a faint vertical rule is optional. Small subdued cluster labels
make the row look designed rather than like a second nav bar. This is why `CommandDefinition` gains
`cluster` - the grouping is declared data, not a styling accident.

### Whitespace is the primary separator, on the existing spacing scale

Do not box everything. A larger gap between clusters, and generous horizontal spacing within them,
carries the structure. This is the single biggest lever between "mature application chrome" and "a
row of bordered buttons", and it directly serves the reduce-padding-and-bubbliness guidance.

Make the proximity ratio explicit rather than eyeballed, using the existing `--ple-space-*` scale
(`src/style.css:19-25`): a small step between commands **inside** a cluster, and a large step
**between** clusters - roughly a 1:3 ratio, in the spirit of Practical UI's "closely related" versus
"not related" spacing pairs (`Practical_UI:606`). Proximity then does the grouping with no group box
at all; Johnson calls the redundant group box around already-adjacent controls a common design
blooper (`Designing_with_the_Mind_in_Mind:411-414`), and Practical UI shows the same navigation
simplified by deleting its containers (`:547`).

Where a boundary really is needed, prefer a tint or a soft shadow over a 1px border - a shadow
outlines an element as a border would, without the same distraction (`Refactoring_UI:1647-1652`).

Two typographic constraints from the same sources: keep UI font weights at 400 or above, and
de-emphasize with color or size rather than a lighter weight (`Refactoring_UI:298`); and treat the
small cluster labels as labels, not headings - supportive content that should be small and quiet,
with the commands as the focus (`Refactoring_UI:377-391`).

### Generous targets, because the ribbon is the most-clicked surface

Fitts's Law gives movement time as `MT = a + b log2(D/W + 1)`, where `D` is distance travelled and
`W` is target width (`Designing_the_User_Interface:2905-2911`). The ribbon is where a teaching
session's repeated navigation happens, so both terms are worth spending on: a fixed position keeps
`D` predictable and learnable, and comfortable horizontal padding buys `W` cheaply. This is a second,
independent reason the padding inside ribbon controls should be generous even while the overall
design gets less bubbly - and it is consistent with the 56px minimum touch target already required
by `docs/HUMAN_GUIDANCE.md`.

### Soft-flat affordance

Fully flat controls make it hard to tell what is clickable; heavy chrome looks dated. Use:

- Rest: nearly flat, no shadow, no permanent border.
- Hover and focus: soft rounded background, roughly 6-8px radius.
- Active or selected: a retained stronger background.
- Focus: a visible keyboard focus ring in every state, honoring the existing
  `src/styles/accessibility.css` `forced-colors` and `prefers-reduced-motion` blocks.

### Course color is an identity system, not decoration

Use the theme in about three places, never as a ribbon-wide flood:

1. A small accent beside or behind the course identity in the Ribbon Context Row.
2. The Selected Ribbon Tab underline.
3. The active command's retained background.

The asynchronous theme design makes this safe: the ribbon is fully legible in neutral tones before
the palette arrives, and the accent settles in without moving anything. Derive the accent through the
existing `THEME_MIX` recipe in `theme_catalog.ts` rather than using raw anchors ad hoc, and keep the
active-section treatment on the secondary anchor as `docs/UI_DESIGN_GUIDE.md:105-111` already
specifies. Contrast is checked against `docs/ux/COURSE_APPEARANCE_ACCESSIBILITY_AUDIT.md`.

### Feedback belongs at the control that was clicked

This is the one requirement the books add that the visual direction did not, and it matters here more
than in most interfaces.

Peripheral vision is roughly 20/200 - legally blind outside the fovea, which covers only one to two
centimetres of screen at normal viewing distance
(`Designing_with_the_Mind_in_Mind:810`, `:875`). When someone clicks a tab, **their fovea is on that
tab**. Anything that changes far away - a spinner in the content region, an error banner, a status
line - lands in the periphery and is routinely not noticed; Johnson's worked example is a login error
message that users genuinely failed to see (`:861-877`).

So the ribbon must acknowledge the click **at the clicked control**:

- The active-state change lands on the tab itself, immediately, before content resolves. The chrome
  contract makes this possible, since the tab selection is synchronous.
- If content is still loading, show that pending state on the clicked control too, not only in the
  content region.
- Never rely on a distant region as the only feedback that a navigation happened.

**Source of the pending state.** Do not build a navigation-state subsystem for this. `@solidjs/router`
already exposes `useIsRouting()`, an accessor that is true while a route transition is in flight, and
the repo's dependency floor is `">=1.0.0"` (`package.json:19`), so it is available. `AppRibbon`
combines it with the destination it just selected: the tab matching the pending pathname renders
`aria-busy="true"` and its pending treatment while `useIsRouting()` is true. Keep the treatment
non-motion-based, or gate motion behind the existing `prefers-reduced-motion` block in
`src/styles/accessibility.css`.

The same chapter explains why a weight change is a good active indicator: font weight "pops" in
peripheral vision, whereas shape does not (`:967`). A heavier Selected Ribbon Tab therefore stays findable
when the user's eyes are down in the content region - which is precisely the re-orientation moment
the persistent ribbon exists to serve. It also means the active indicator must not be color alone,
independently matching `docs/UI_DESIGN_GUIDE.md:77`.

### Overflow needs a visible cue

Horizontal scrolling is the phone-width answer, but a clipped row must *look* clipped. Use partial
clipping - let the next label be visibly cut rather than ending on a clean boundary - plus a soft
edge fade. Partial clipping is a strong, control-free signal that more exists beyond the edge, and it
pairs with the reachability oracle so the eight-tab instructor row stays navigable rather than merely
un-wrapped.

Two independent sources agree, which is why this is a requirement and not a nicety: About Face
recommends partial clipping as a continuation cue, and Practical UI states the same rule as "expose
the edge of cards that are off screen so that people know they're there", listing invisible overflow
alongside unlabelled controls and too-subtle selected states as the failure modes of minimal
interfaces (`Practical_UI:223`, `:225`).

**Keeping the Selected Ribbon Tab in view is behavior, not styling**, so assign it explicitly: on every change
of Selected Ribbon Tab, `AppRibbon` scrolls that tab into the visible portion of its row with
`scrollIntoView({ inline: "nearest", block: "nearest" })`, using `behavior: "smooth"` only when
`prefers-reduced-motion` is not set. `inline: "nearest"` is the right choice because it moves the row
the minimum distance and leaves it alone when the tab is already visible, so a desktop row never
scrolls gratuitously. This is what makes the reachability oracle a test of implemented behavior
rather than of luck.

Practical UI also confirms the label treatment chosen above: navigation menus and tabs carry enough
other interactive cues that they do not need underlined-link styling (`:213`) - so the tabs can stay
clean text, provided the selected state is unmistakable.

## Before and after control flow

Before:

```
App
 |- header.site-header                      (persists)
 `- main.shell
     `- Show keyed location.pathname        <-- destroys everything below, every click
         `- ErrorBoundary                   (its fallback also replaces course chrome)
             `- div[data-current-path]
                 `- RouteContent -> SessionContent -> PresentationContrastProvider
                     `- <router outlet> -> RouteAccessBoundary
                         `- Show keyed allowedRoute
                             `- CourseThemeScope        (async; replaces page while loading)
                                 `- CourseManagementFrame (ribbon lives here)
                                     `- page
```

After:

```
App
 `- PresentationContrastProvider
     |- SessionGate                          (same states as today's SessionContent)
     `- RouteScopeProvider                   (keyed by scope reference; sync title + membership role)
         `- CourseThemeVariables             (emits --ple-course-theme-*; wraps chrome AND content)
             |- AppRibbon                    (identity / tab / Ribbon Task Rows - created once)
             `- main.shell > div#main-content.content-region
                 `- Show keyed location.pathname   <-- now wraps ONLY content
                     `- ErrorBoundary
                         `- <router outlet> -> RouteAccessBoundary -> page
```

Consequences worth stating:

- A page-level error now keeps the ribbon usable, because the `ErrorBoundary` fallback sits inside
  the content region instead of above the chrome.
- Skip link and focus: `id="main-content"` and `tabindex="-1"` move from `<main>` to the content
  region, so "Skip to learning content" lands past the chrome. `focusMainContent` and the
  path-change effect (`src/app.tsx:120-143`) keep their behavior and retarget that element.
- Tab order follows DOM order: identity, tabs, commands, content - on every route.

## Files

Create:

- `src/navigation/route_params.ts` - `routeParams`, `routeScopeKey`.
- `src/ribbon/ribbon_contract.ts` - `TAB_CATALOG`, `RIBBON_TASK_CATALOG`, `visibleTabs`,
  `visibleCommands`, `buildRoutePath`.
- `src/ribbon/route_scope_context.tsx` - `RouteScopeProvider`, `useRouteScopeIdentity`,
  `useRouteScopeData`.
- `src/ribbon/app_ribbon.tsx`, `src/ribbon/app_ribbon.css`.

Modify:

- `src/api/contracts.ts` and `crates/server/src/auth.rs:126`, `:341` - session membership index.
- `src/api/runtime.tsx` - add `resolveCourse` / `resolveRun` cached queries beside the existing ones.
- `src/navigation/resolved_route.ts` - callers use the cached queries; the strict kind checks stay.
- `src/route_contract.ts` - `chrome` on all 32 rows; chrome types.
- `src/app.tsx` - shell restructure per the tree above.
- `src/route_access_boundary.tsx` - drop the `CourseThemeScope` wrap at `:77`; keep the gate.
- `src/features/course_appearance/course_theme_scope.tsx` -> `course_theme_variables.tsx`: keep the
  `<style>` + themed wrapper and `CourseThemePresentationContext` (the appearance page needs live
  preview at `course_appearance_page.tsx:114`); delete `managementRoute()` (`:95-102`) and the
  `CourseManagementFrame` branch (`:115-121`); read from `useRouteScopeData` instead of owning its
  own `createAsync` pair.
- `src/features/course_appearance/course_entry_identity.tsx` - keep it. The ribbon Ribbon Context Row owns
  a fixed-height course **label**; this component keeps the course-home `h1` and the entry banner
  (`docs/HUMAN_GUIDANCE.md` course banner). Label and page heading are different things, so this is
  not the duplicate-heading case being fixed.
- `src/routes.ts:56-65` - the five `assignmentWorkspace*` entries map to `AssignmentWorkspaceLivePage`
  with no `section` prop.
- `src/pages/assignment_workspace/assignment_workspace_live_page.tsx` - see task 11.
- `src/style.css` - grid shell, ribbon tokens, remove the `data-route-surface` width block
  (`:279-289`) and the `nav` / `.nav-action` rules superseded by ribbon CSS (`:202-254`).
- The 16 consumers of `useCourseThemeRouteData()` switch to `useRouteScopeData()` - import and hook
  name only, since the value shape is preserved: `course_entry_identity.tsx:40`,
  `course_appearance_page.tsx:113`, `run_page.tsx:752`, `run_summary_page.tsx:16`,
  `gradebook_page.tsx:625`, `teaching_operations_page.tsx:51`,
  `student_work_inspection_page.tsx:427`, `curriculum_adoption_live_page.tsx:19`,
  `assignment_access_live_page.tsx:66`, `assignment_preview_page.tsx:225`,
  `course_assignments_page.tsx:312`, `course_grade_settings_page.tsx:665`,
  `course_roster_page.tsx:116`, `assignment_workspace_live_page.tsx:165`,
  `assignment_workspace_create_page.tsx:29`, and `route_access_boundary.tsx:77`.

Retire: `course_management_frame.tsx` + `.css`, `course_management_nav.tsx` + `.css`,
`assignment_workspace_nav.tsx`, `course_theme_route.ts`.

Docs: `docs/DESIGN_DECISIONS.md` (restate `:762-770` as the shell-owned ribbon for all roles),
`docs/UI_DESIGN_GUIDE.md:63-78`, `docs/CHANGELOG.md` after each task, and a workstream doc under
`docs/active_plans/workstreams/` in snake_case.

## Autonomy requirements

Every milestone below completes without a person in the loop. Two consequences shaped the list:

- **No gate is a human judgment.** "Review the screenshots", "judge the proportions on a phone", and
  "confirm with the owner" are replaced by computed assertions, fixture comparisons, and decisions
  already made in this plan from repository evidence.
- **No decision is deferred.** Where a choice was previously left open - the Sysadmin route rows, the
  membership definition, the touch-target relationship - the plan now states the rule and a milestone
  encodes it as a test.

### Shared harness, built first

Milestone 0 builds the fixtures every later gate uses, so no milestone needs a live human, a live
course, or a seeded database it cannot create.

- **Authority fixtures** (pure data): Student, Instructor, Sysadmin, plus Instructor-in-course-A /
  Student-in-course-B, and an empty membership index. These drive every `visibleTabs` test with no
  browser at all.
- **Counting fake `ApiClient`.** `createApiRuntime` already takes an injected client
  (`src/api/runtime.tsx:49`), so a fake that counts calls per method gives request-count gates as
  fast unit tests instead of browser network logs.
- **Synthetic transition driver.** A helper that drives a list of pathnames through the router in one
  mounted app, for the identity oracle - no server needed.
- **Deferred-resolution fixture.** A promise the test releases on demand, for the
  chrome-during-load gate. Replaces "throttle the network and look".
- **Stub for `scrollIntoView`** and an injectable `useIsRouting` signal, so scroll and pending
  behavior are asserted directly rather than inferred from pixels.
- **Playwright context options** `forcedColors: "active"` and `reducedMotion: "reduce"` for the
  accessibility-mode gates.

## Milestones

Twenty-six small milestones, each independently completable with one automated gate
(`docs/REPO_STYLE.md`, atomic task decomposition). Milestones 1-11 change no UI.

**Foundation**

0. **Test harness.** Build the fixtures listed above. *Gate:* the harness's own unit test mounts a
   trivial component with each authority fixture.
1. **Name the current-membership rule.** Identify the existing query that decides which Course
   Memberships back the course list, and export it as one named function both the list and the
   session bootstrap call. *Gate:* a test asserting both call sites resolve through that one function,
   so "my courses" cannot fork into two definitions.
2. **Session response carries memberships.** Extend `AuthSessionResponse` (`auth.rs:126`) and
   `session_response` (`:341`); `session_handler` (`:299`) becomes the query site. *Gate:* `cargo test`
   on the response shape plus a serde snapshot including the empty-membership case.
3. **Browser session contract decodes memberships.** *Gate:* decoder unit tests for populated and
   empty lists, and for the singular-`role` / plural-`roles` mapping.
4. **Route parameter extractor.** Add `routeParams` in `route_params.ts`. *Gate:* one assertion per
   declared path that named parameters are extracted.
5. **Scope key with `invalid`.** Add `routeScopeKey`; retire `course_theme_route.ts`. *Gate:* a
   malformed reference on a scoped path yields `{kind:"invalid", scope}` and never `"global"`.
6. **Cached reference resolution.** Add `resolveCourse` / `resolveAssignmentAttempt` router queries.
   *Gate:* with the counting fake, repeated resolution of one reference issues one call. Ships a real
   improvement to today's UI before any shell work.
7. **Sysadmin route audit.** Apply the decided rule: `teachingOperations` keeps `sysadmin`; the other
   12 course routes become `["instructor"]`. *Gate:* a test asserting each course route's
   `requiredRoles` matches the audit table, failing on any future unaudited `sysadmin`.
8. **Chrome types and contract column.** Add chrome types; give all 32 rows a `chrome` value.
   *Gate:* `npx tsc --noEmit` plus an exhaustive test that every `RouteId` resolves to a tab.
9. **Catalogs and slot schemas.** `TAB_CATALOG`, `RIBBON_TASK_CATALOG`, and the fixed ordered slot schema
   per scope. *Gate:* a test asserting the append-only ordering rule holds structurally - within each
   schema, every slot available to all Course Membership Roles precedes every role-narrowed slot, so
   late authority can only append.
9b. **`deriveRibbonModel`.** The pure topology function. *Gate:* authority-fixture tests - Student,
    Instructor, Sysadmin, and `pendingAuthority` all produce the **same slot list in the same order**,
    differing only in slot state; the function's signature takes no resource, and a test asserts it
    returns synchronously with the network fake configured to never resolve.
9c. **Topology is a pure function of two synchronous inputs.** *Gate:* assert `ribbonSchemaFor` is
    total over all nine (scope, Product Role) pairs; assert `deriveRibbonModel` returns the identical
    slot list before and after releasing every deferred fixture, for each Product Role - the
    machine-checkable form of "no network response can change which slots exist".
10. **`buildRoutePath` and the parameter invariant.** *Gate:* the subset test - every catalog
    destination's declared parameters are provided by its declaring routes.
11. **Vocabulary check.** Checks names, not structure - the terminology contract owns the former and
    this plan owns the latter. *Gate:* a script that reads `docs/VOCABULARY_REPLACEMENTS.md` at run
    time and asserts no identifier added by this workstream matches its "current wording" column; a
    test that every Ribbon Slot and Ribbon Task label equals its canonical visible surface name from
    `docs/TERMINOLOGY_CONTRACT.md`, so a regression to "Library" or "Curriculum" fails; and a test
    that no Ribbon Task declares an operation, keeping Page Actions out of the navigation rows.

**Scope ownership**

12. **Scope identity, synchronous.** `RouteScopeProvider` + `useRouteScopeIdentity`. *Gate:* fixture
    tests over indexed course, unindexed course, invalid reference, and Assignment Attempt scope.
13. **Scope data, keyed by reference.** `useRouteScopeData` with resources keyed on the reference.
    *Gate:* the four-transition table asserted with the counting fake - same course zero calls,
    A to B one call each leg, A to B to A zero for A, course to attempt and back no unmount.

**Ribbon, testable before it is mounted**

14. **Ribbon structure.** Three always-present rows driven solely by a `RibbonModel` prop, plus the
    `--ple-ribbon-*` token contract and the shell grid. *Gate:* component test - all three rows
    present including an empty Ribbon Task Row, correct `aria-label`, and `AppRibbon` renders identically
    for a given model regardless of session or resource state, since it reads neither.
14a. **Ribbon design fixture.** A static page that renders `AppRibbon` from hand-written
     `RibbonModel` values covering every state it can occupy: each scope's schema, every slot state,
     selected and unselected, empty and populated Ribbon Task Rows, a short and a very long Course
     Instance title, and each course theme. It imports no session, router, or client - proof by
     construction that the ribbon is a designed surface rather than a generated menu.
     *Gate:* the fixture renders every declared combination, and a test asserts the module graph of
     `app_ribbon.tsx` pulls in no session, router, or API module. This fixture is also the target for
     the milestone-25 visual assertions, so visual work needs no running application.
14b. **Geometry contract.** *Gate:* computed-style test that the ribbon's block size equals
     `--ple-ribbon-block-size` across every scope, every authority fixture, a command-less tab, a
     long Course Instance title, a loading state, and an error state - the exhaustive form of "no
     input may change ribbon height".
15. **Active and pending state.** `aria-current="page"`; `aria-busy` driven by the injected
    `useIsRouting`. *Gate:* component test toggling the injected signal.
16. **Active-tab scrolling.** *Gate:* stubbed `scrollIntoView` asserted to be called with
    `inline: "nearest"` on tab change, and not called when the tab is already visible.
17. **Overflow cue.** *Gate:* Playwright at the phone profile - `scrollWidth > clientWidth`, the
    Selected Ribbon Tab's box lies within the row's visible box, and the clipping cue element is present.

**Shell integration, no milestone regressing navigation**

18. **Mount the ribbon alongside the existing frame.** `AppRibbon` goes into the shell while
    `CourseManagementFrame` stays. Navigation is briefly duplicated, never absent - this is the
    ordering fix for the regression the previous draft accepted. *Gate:* both navigations present;
    identity oracle passes on global routes.
19. **Narrow the swap boundary.** Keyed `Show` wraps only the content region; `#main-content` and
    focus retargeted; `.shell` becomes a grid. *Gate:* identity oracle across all five transition
    classes; skip-link test.
20. **Retire the course frame.** `course_theme_scope.tsx` becomes `course_theme_variables.tsx`,
    dropping `managementRoute()` and the frame branch. *Gate:* exactly one course navigation in the
    DOM; ribbon element identity unchanged across the change.
21. **Hoist theme variables; drop the boundary wrap.** *Gate:* no nested duplicate theme element;
    palette present on course routes; the appearance page's live preview still updates through
    `CourseThemePresentationContext`.
22. **Page-consumer migration.** The 14 consumers move to `useRouteScopeData`. *Gate:*
    `npx tsc --noEmit` with the old hook deleted, so the compiler enumerates every missed site.
23. **Assignment workspace.** Section from the ribbon contract; `createResource` keyed on the
    Assignment Attempt reference; own nav removed; course object from `useRouteScopeData` via
    `courseRouteData(...)`, since it needs `course.id` (`:204`, `:208`). Keep `WorkspaceState`, the
    authority checks (`:185-201`), and the retry-button focus (`:247`). *Gate:* section changes issue
    zero assignment requests; denied and error states render and still focus the retry button.

**Finish**

24. **Retire superseded components and CSS**, including the `data-route-surface` width block while
    keeping the attribute. *Gate:* `./check_codebase.sh` clean; dead-export scan empty.
25. **Apply the visual system.** One ribbon surface, differentiated row roles, cluster labels, the
    proximity ratio, soft-flat states, the course accent in its three places, clipping cue.
    *Gate, fully automated:* computed-style assertions that ribbon spacing resolves to `--ple-space-*`
    tokens and that the between-cluster gap exceeds the within-cluster gap; a contrast script over
    every theme in `theme_catalog.ts` against the audit thresholds; `forcedColors: "active"`
    assertions that Selected Ribbon Tab, cluster separation, and command state stay distinguishable when tint
    and shadow drop out; `reducedMotion: "reduce"` assertion that no scroll animation runs; and
    `tests/playwright/verify_ui_corpus.mjs` over the regenerated corpus.
26. **Docs and changelog.** Record in `docs/UI_DESIGN_GUIDE.md`: the per-role slot schemas, the
    Account controls in the Ribbon Context Row, the Assignment Attempt structure, the three-row
    geometry contract, and the visual system - this document, not the terminology contract, is where
    ribbon structure lives. Update `docs/DESIGN_DECISIONS.md` (the "one spatial owner" entry) to the
    all-role shell-owned ribbon, and note the completed `sysadmin` route correction. *Gate:*
    `pytest tests/` markdown-link and hygiene gates.

### Two rules the milestones encode rather than defer

- **Touch target versus row height.** Ribbon controls reach the 56px minimum through their own
  padding under `@media (pointer: coarse)`, and each row's height is the larger of its content box
  and that minimum. The Ribbon Context Row collapses to a single compact line on coarse pointers so the
  three rows stay a reasonable share of a phone viewport. Milestone 25 asserts the computed hit box
  on the phone profile rather than leaving the trade to improvisation.
- **Command clusters are presentation grouping.** `cluster` lives in `ribbon_contract.ts` beside the
  catalog, not in the domain model, and `"assignment" | "delivery"` describe the Ribbon Task Row only.
  They earn promotion to domain vocabulary only if server or domain code ever needs them; until then
  they stay adjacent to the ribbon so regrouping costs one file.

## Verification

Repo gates first: `./check_codebase.sh` (typecheck, lint, format, unit tests), then
`./run_playwright_tests.sh`. The real-stack check runs the existing automated real-stack specs
against `./run_live_demo.sh` rather than a person looking at it, so the whole sequence is executable
unattended.

Chrome persistence is proved **behaviorally, not by pixel comparison**, since the owner asks to avoid
arbitrary pixel and timing equivalence gates (`docs/HUMAN_GUIDANCE.md:25`):

- **Identity oracle.** Tag the ribbon element on mount with a value that changes per mount, then
  assert the tag is unchanged across every transition where persistence is intended: route-to-route
  inside one Course Instance, global-to-global, Course Instance A to B, Course Instance to Assignment
  Attempt and back, and tab switches within a Course Instance. Any remount fails. This is the central
  architectural invariant, and it proves causes 1-3 without asserting a coordinate. The synthetic
  transition driver runs the whole list in one mounted app, so it needs no seeded server.
- **Consistency oracle.** For student, instructor, and sysadmin fixtures, assert the ribbon landmark
  has the same accessible structure on every route that role can reach - WCAG 2.2 SC 3.2.3 Consistent
  Navigation and 3.2.4 Consistent Identification.
- **Keyboard oracle.** Tab order reaches identity, Ribbon Tab Row, Ribbon Task Row, then content, on every route
  (`docs/HUMAN_GUIDANCE.md:173`).
- **Topology-stability oracle.** The strongest of the set, and the one that proves the shell contract
  rather than mere persistence. With the projection deferred, capture the ribbon's slot list; release
  the projection; capture it again. Assert that every slot present in the first capture holds the same
  index in the second, and that the ribbon's block size is unchanged. Run it for the unresolved
  Course Instance, the invalid reference, and a Student-to-Instructor authority change. Component
  identity alone would pass even if the interface inside the ribbon rearranged; this is what closes
  that gap.
- **Chrome-during-load oracle.** With the projection deferred, assert the Ribbon Tab Row is present with its
  scope's full slot schema, that available slots are clickable, and that an unresolved or invalid
  reference uses the Course Instance schema rather than the global one.
- **Row-count and reachability oracle.** At 1280x800 and the student tablet and phone profiles in
  `tests/playwright/ui_corpus_manifest.ts`, assert each ribbon row renders as one row - and, on the
  phone profile, that navigation is not merely un-wrapped but still **usable**: the Selected Ribbon Tab is
  visible without scrolling the row, and every other tab is reachable by scrolling that row. Not
  wrapping fixes the geometry defect but can hide destinations, so the same test guards both. On the
  eight-tab instructor course row this is the case most likely to need a scroll affordance or an
  overflow control; treat a failure here as a design signal, not a test to loosen.

Refresh the screenshot corpus (`capture_screenshots.sh`) last, since chrome changes on every screen.

### Permanent versus one-time

Classified against `docs/PYTEST_STYLE.md` and `docs/HUMAN_GUIDANCE.md:28`, since a fragile permanent
test is worse than none:

| Check | Disposition | Reason |
| --- | --- | --- |
| Identity oracle | Permanent | Proves the ribbon instance is never rebuilt |
| Topology-stability oracle | Permanent | Proves controls inside the ribbon never move; the shell contract itself |
| Geometry contract test | Permanent | Proves no input can change the content origin |
| Position-stability property | Permanent | Cheap pure-function guard on the append-only ordering rule |
| Consistency oracle | Permanent | A durable WCAG-backed user-facing contract |
| Keyboard oracle | Permanent | Durable accessibility contract (`docs/HUMAN_GUIDANCE.md:173`) |
| Chrome-during-load oracle | Permanent | Guards the "geometry never waits on data" rule against future async work |
| Row-count and reachability oracle | Permanent | Behavioral, not a pixel gate: counts rows and checks reachability |
| Catalog parameter-subset test | Permanent | Cheap, static, prevents an unconstructible destination |
| Tab-visibility never exceeds boundary | Permanent | A security-adjacent invariant |
| Network-request counts (tasks 4, 5, 11) | One-time | Proves the migration; would pin an implementation detail if kept |
| `npx tsc --noEmit` consumer sweep (task 10) | One-time | The compiler is the check; nothing to retain |
| Per-task render gates (tasks 6-9) | One-time | Scaffolding for the rebuild, superseded by the oracles above |

The identity oracle and the fixed identity/tab-row rule are the two things not to trade away under
implementation pressure. Users learn control positions and keep acting on the learned position after
a control moves, and a persistent navigation object works as an orientation landmark - which is the
same conclusion `docs/DESIGN_DECISIONS.md:768` already reached from the product side.
