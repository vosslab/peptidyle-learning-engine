# Plan: Professor capability architecture and Alpha course roadmap

## Context

Peptidyle already proves a focused teaching loop: create a course, manage a roster, assemble an
assignment, complete repeated student runs, and inspect the gradebook. The Playwright suite also
covers catalog scale, question authoring, assignment reuse, course appearance, imports, pagination,
accessibility, and recovery.

The broader professor cycle is less complete:

```text
discover -> inspect -> curate -> assemble -> teach -> intervene
    ^                                                   |
    |                                                   v
reuse <- revise <- analyze <------------------------- grade
```

ADAPT demonstrates useful discovery scopes, favorites, folders, assignment templates, course
copying, Alpha/Beta reuse, sections, co-instructors, accommodations, manual grading, and analysis.
It also demonstrates designs Peptidyle should avoid: a monolithic question-bank interface, live
Alpha/Beta synchronization, proliferating human roles, mutable duplicated content, and large webs
of synchronization exceptions.

This plan uses two ledgers:

- Repair existing release-contract gaps before RC12.
- Implement the broader professor system through a separate post-RC12 plan while Peptidyle remains
  pre-production and before the first durable-data baseline.

## Objectives

- Support the entire professor cycle from open-ended discovery through course reuse and
  evidence-led revision.
- Introduce public Alpha courses as creator-owned, non-enrollable curriculum blueprints that any
  approved Instructor may clone.
- Give collections, reusable assignments, Alpha curricula, and teaching courses distinct ownership
  and mutability contracts.
- Turn existing timing, accommodations, manual grading, item analysis, and retention foundations
  into complete professor-facing workflows.
- Preserve Peptidyle's stronger security, reproducibility, accessibility, and student-record
  boundaries.

## Design philosophy

Apply **Fix the design, not the symptom**, **Design for adaptability**, **Long-term over
short-term**, and **Dream big**.

Use this ownership tree:

```text
Question ID
  -> personal collection
  -> personal assignment blueprint
  -> public Alpha course assignment
  -> teaching-course assignment
  -> issued student run
```

Each level adds context without changing the layer beneath it. Shared questions remain immutable
publications; reusable curriculum remains answer-free current state; teaching assignments remain
mutable current state governed by issued-run evidence; student runs retain exact immutable
snapshots.

Optimize across four rates:

- Minutes: find, inspect, save, and add a problem.
- Weeks: schedule, accommodate, grade, intervene, and revise.
- Terms: clone courses, shift dates, and improve an Alpha curriculum.
- Years: preserve public attribution, aggregate learning evidence, and reuse curricula across
  instructors.

## Scope

- Repair PostgreSQL search ranking, trigram matching, catalog-statistics presentation, and live
  discovery evidence.
- Add personal favorites, named collections, saved searches, bulk selection, and course-usage
  context.
- Reuse one capable problem-picker surface in Library, assignment, blueprint, and Alpha-course
  workflows.
- Add private personal assignment blueprints.
- Add public creator-owned Alpha courses containing ordered reusable assignments and relative
  teaching schedules.
- Add Alpha forks, teaching-course instantiation, whole-course rollover, provenance, fast-forward
  updates, and selected copying.
- Activate assignment lifecycle, instructions, visibility, schedule, late-work, and accommodation
  workflows already represented in the schema and server foundations.
- Add teaching-course co-instructors, sections/groups, assignment audiences, retention controls,
  and archive/rollover interfaces.
- Add manual-grading queues, reusable feedback snippets, explicit assignment-grade overrides,
  item-analysis pages, and revision/replacement actions.
- Add complete professor-centered Playwright and live PostgreSQL journeys.
- Update Human Guidance, active plans, architecture documentation, usage guidance, and the
  changelog as each package closes.

## Non-goals

- Do not implement ADAPT-style live Alpha/Beta tethering or a general three-way merge engine.
- Do not implement in-product pull requests or contribution proposals; retain fork lineage and
  comparison data without adding that workflow.
- Do not add learning trees, discussions, clickers, LMS roster synchronization, research exports,
  generated content, or a generic dashboard.
- Do not add Manager, Publisher, Grader, Tester, or teaching-assistant human roles. Co-instructors
  remain ordinary approved Instructors.
- Do not expose answer keys, grading implementations, or author source to non-authors.
- Do not create course-assignment history or instructor-facing assignment versions.
- Do not introduce a dedicated search service while indexed PostgreSQL remains sufficient.
- Do not add compatibility readers or legacy migration paths for nonexistent production data.

## Current state summary

### Present and represented in Playwright

- Course creation, roster invitation/import/export, assignment creation, exact Question ID
  recovery, assignment/checklist reuse, ordering, policies, and run timing.
- Faceted virtualized catalog browsing and safe problem details.
- Private question authoring, protected author preview, publication, QTI import, and all eight
  native families.
- Repeated learner runs, feedback, prefetch, recovery, keyboard operation, summaries, gradebook
  history, and appearance.
- The live instructor walkthrough builds the Genetics assignment from four known Question IDs.

### Incomplete existing contracts

- The active plan promises full-text plus trigram search, but PostgreSQL currently uses only
  `websearch_to_tsquery`.
- Search results are ordered by internal identity rather than relevance.
- The detail page renders no metrics when anonymous statistics are available.
- Live browser acceptance proves known-ID retrieval, not broad discovery, refinement, detail
  review, and selection.
- Assignment lifecycle, instructions, visibility, availability, due, close, and late-policy fields
  exist in the database design but are not coherently projected through the current browser
  contract.

### Backend capability without professor capability

- Assignment schedule and per-student/group accommodations.
- Manual item grading and mixed automatic/manual recalculation.
- Course-local item analysis.
- Retention archive, extension, and deletion operations.
- Course groups and multiple Instructor memberships.

### Missing shared abstractions

- Personal collections and saved searches.
- Reusable assignment blueprints.
- Public curriculum ownership and Alpha-course lineage.
- Cross-course and term-level reuse.
- A common discovery-to-selection component.
- Public instructor bylines suitable for author search and Alpha attribution.
- A connected gradebook-to-intervention-to-revision workflow.

### Intentional Peptidyle advantages over ADAPT

- Shared immutable questions rather than course-local copies.
- Server-only grading and answer-bearing material.
- Human-readable Question IDs with hidden reproducibility snapshots.
- Current-state teaching assignments plus immutable issued runs.
- No live tether that can silently alter active teaching courses.
- Three human roles, tenant-scoped FERPA records, and privacy-first retention.
- Capability-owned modular-monolith components instead of monolithic Vue/controller files.

## Architecture boundaries and ownership

### Public instructor presentation

Add a safe `PublicInstructorByline` containing a typed public reference and approved display name.
It carries no UUID, email, roster identity, or private institution record. Catalog publications and
Alpha courses use it for attribution and author filtering.

### Collections and discovery

- `ProblemCollection` is private, user-owned, revision-checked current state.
- Every Instructor receives one built-in Favorites collection and may create flat named
  collections.
- `SavedProblemSearch` stores a normalized query, not a frozen result list.
- Collections may outlive one institution context, but every question is reauthorized when used in
  a tenant-owned teaching course.
- Extend catalog search with author, response family, backend, tag, taxonomy, license, capability,
  statistics, and safe course-usage filters.
- Rank exact Question ID first, then full-text relevance, then trigram similarity, with a stable
  opaque relevance cursor.
- Add safe prompt excerpts, public bylines, family, tags, human taxonomy labels, license, and
  disclosed statistics to result cards.
- Keep source-specific repositories behind one `ProblemPicker`: public catalog, my published
  questions, collection, retained course definitions, or Alpha curriculum.

### Assignment blueprints

Use one answer-free `AssignmentBlueprintDefinition` value shape for title, instructions, ordered
Question IDs, points/scoring modes, and default run policies.

- `PersonalAssignmentBlueprint` is private user-owned current state protected by a strong revision.
- An Alpha assignment owns the same definition shape under its Alpha course.
- Teaching instantiation resolves each Question ID to the current assignable immutable version.
- Reusable definitions do not contain absolute dates, rosters, accommodations, runs, grades, or
  answer material.
- Do not add blueprint history. Clone/import manifests retain only the source revision and
  materialized baseline needed for provenance and safe comparison.

### Alpha courses

Model Alpha courses as a separate shared-curriculum aggregate, not as a `CourseKind` field on the
FERPA-bearing course table.

- An Alpha course is public to approved Instructors and receives a human route such as `AC-123`.
- Only its creator may edit it; every approved Instructor may inspect, fork, or instantiate it.
- Students cannot join, receive assignments, start runs, or generate grades because Alpha records
  have no relationship to teaching-course membership or activity tables.
- Alpha assignments may reference only public, currently assignable Question IDs.
- The Alpha course stores an ordered curriculum with module/week labels and calendar-relative
  availability, due, and close defaults.
- Relative times use calendar days and local wall-clock values, not elapsed seconds, so term cloning
  remains correct across daylight-saving changes.
- Alpha creation uses an explicit choice card: "Public reusable Alpha course - no students" versus
  "Teaching course - students, dates, and grades."

### Teaching courses and reuse

Keep the current course aggregate exclusively for teaching.

- Teaching courses remain tenant-owned and may have multiple approved Instructor members.
- A teaching course may originate from an Alpha course, another teaching course, or no source.
- Cloning copies definitions, policies, theme defaults, and reviewed schedule offsets; it never
  copies students, invitations, groups containing students, accommodations, runs, responses,
  grades, retention state, or co-instructors.
- The clone wizard requires a course start date and time zone, previews all resolved dates, and
  blocks ambiguous or nonexistent local times until corrected.
- Each imported assignment records its source and normalized baseline manifest.
- If its reusable-definition fields still match that baseline and no student run has been issued,
  it may fast-forward to the current Alpha definition.
- If the teaching assignment diverged, the UI offers side-by-side selected copying. It never
  attempts an automatic merge.
- Delivery dates and accommodations are teaching-owned and are never overwritten by Alpha updates.
- After a run is issued, preserve the existing policy: adding or replacing questions is blocked;
  reordering affects future runs; points and policies remain editable; removal uses Delete and
  Regrade.

### Teaching operations

- Activate `draft`, `published`, `closed`, and `archived` assignment lifecycle states.
- Remove the redundant `visible` boolean; learner availability derives from lifecycle plus
  availability/close policy.
- Project instructions, availability, due, close, late-work policy, run timing, and lifecycle
  through typed Store/API/browser contracts.
- Extend course groups with a closed purpose such as section or accommodation group.
- Add `AssignmentAudience` for all active students or selected course groups.
- Add co-instructor invitations only for globally approved Instructor accounts.
- Add course-local pages for sections, accommodations, schedule, retention, and archive.
- Keep Alpha courses out of all these components by type and route ownership.

### Grading and analysis

- Add a paginated manual-grading queue whose list contains no raw response; fetch protected
  prompt/response detail only when an Instructor opens one item.
- Submit only normalized manual credit and bounded feedback through the existing
  server-authoritative route.
- Add private user-owned feedback snippets that insert text but never calculate correctness.
- Add a separate current `AssignmentGradeOverride` with reason, actor, strong revision, explicit
  clear action, and visible distinction from the computed score.
- Preserve pending manual items independently of assignment-level overrides.
- Render current item-analysis aggregates with links to the problem, assignment, author workspace
  when owned, fork flow when not owned, and future-assignment replacement controls.
- Never join professor analytics pages directly to raw learner responses except through an
  explicitly authorized manual-grading detail.

## Milestone plan

```text
M0  Release truth          Close existing discovery/statistics promises before RC12.
M1  Shared foundations     Record decisions and establish public authorship, ownership, refs, and schemas.
M2  Professor foundations  Build discovery/curation and teaching operations in parallel.
M3  Reusable curriculum    Add personal blueprints, Alpha courses, cloning, rollover, and bounded updates.
M4  Intervention loop      Add manual grading, overrides, item analysis, and revision pathways.
M5  Connected acceptance   Prove the complete professor cycle on the final material tree.
```

### Milestone M0: Release truth

- Depends on: current active release package order.
- Workstreams: indexed catalog search; browser presentation/evidence.
- Deliverables: trigram/relevance search, relevance-bound cursors, available-statistics rendering,
  and a separate live discovery journey.
- Exit criteria: exact-ID behavior remains intact; broad and misspelled searches return intended
  fixtures; facets and pages remain snapshot-consistent; representative query plans use indexes.
- Parallel-plan ready: yes, after the search response contract is frozen; maximum two
  implementation lanes.

### Milestone M1: Shared foundations

- Depends on: M0 and RC12 acceptance.
- Deliverables: durable Human Guidance, saved active plan, public byline, typed public references,
  reusable-definition value type, Alpha/collection/blueprint ownership contracts, migration
  allocation, and RLS design.
- Exit criteria: Alpha records cannot participate in any enrollment/activity relationship;
  creator-only mutation and cross-tenant Instructor reads are proven in Memory and PostgreSQL.
- Parallel-plan ready: no; shared types, migrations, generated contracts, and ownership rules must
  land serially.

### Milestone M2: Professor foundations

- Depends on: M1.
- Workstreams:
  - Discovery and curation: search metadata, collections, saved searches, common picker, bulk
    actions.
  - Teaching operations: assignment lifecycle/schedule, co-instructors, groups/audiences,
    accommodations, retention UI.
- Exit criteria: both lanes pass focused behavior gates and integrate without editing each other's
  capability modules.
- Parallel-plan ready: yes; maximum two implementation lanes plus one independent reviewer.

### Milestone M3: Reusable curriculum

- Depends on: both M2 workstreams.
- Workstreams: personal blueprints and Alpha authoring; clone/rollover/date resolution;
  provenance/fast-forward/selected-copy UI.
- Exit criteria: two fictional Instructors in different tenants can discover and clone one Alpha;
  only its creator can edit it; the derived teaching course contains no copied student records.
- Parallel-plan ready: yes after blueprint and clone-manifest contracts land; maximum three lanes.

### Milestone M4: Intervention loop

- Depends on: teaching operations from M2 and reusable definitions from M3.
- Workstreams: manual grading and overrides; item-analysis workflow; gradebook integration.
- Exit criteria: a mixed assignment moves from pending manual work to a current score, analysis
  identifies an item needing improvement, and the Instructor can revise/fork and place the
  correction into a future assignment without altering issued evidence.
- Parallel-plan ready: yes after the gradebook extension contract is frozen; maximum three lanes.

### Milestone M5: Connected acceptance

- Depends on: M3 and M4.
- Deliverables: integrated browser journeys, live PostgreSQL/RLS evidence, visual review,
  documentation, baseline migration closeout, and full Validation suite.
- Exit criteria: every required gate passes on the final material tree with no required skip and
  independent review reports no unresolved P0/P1 finding.
- Parallel-plan ready: yes for browser, database/security, documentation, and visual review; final
  integration remains serial.

## Work packages

- `WP-R0` - Catalog owner: implement ranked full-text/trigram discovery and same-snapshot facets;
  depends on none.
- `WP-R1` - UI/browser owner: render disclosed statistics and prove live broad discovery without
  changing the canonical known-ID walkthrough; depends on WP-R0.
- `WP-F1` - Architect: record Alpha, blueprint, collection, authorship, and teaching-course
  boundaries in Human Guidance and the active plan; depends on RC12.
- `WP-F2` - Expert coder: add shared domain contracts, short references, migration allocation, RLS
  policy, and generated projections; depends on WP-F1.
- `WP-D1` - Expert coder: implement public bylines, expanded search metadata, course-usage
  projections, and source-owned discovery APIs; depends on WP-F2.
- `WP-D2` - Coder: implement Favorites, collections, saved searches, bulk actions, and the reusable
  ProblemPicker; depends on WP-D1.
- `WP-T1` - Expert coder: project assignment lifecycle, instructions, schedule, late policy, and
  timing; remove redundant visibility state; depends on WP-F2.
- `WP-T2` - Expert coder: implement co-instructors, group purposes, assignment audiences,
  accommodations, retention, and archive UI; depends on WP-T1.
- `WP-PROF-B1` - Expert coder: implement personal blueprints and public creator-owned Alpha aggregates;
  depends on WP-D2 and WP-F2.
- `WP-PROF-B2` - Expert coder: implement Alpha fork, teaching instantiation, rollover, relative-date
  preview, manifests, fast-forward, and selected copy; depends on WP-PROF-B1 and WP-T1.
- `WP-G1` - Expert coder: implement manual-grading queue, feedback snippets, and separate grade
  overrides; depends on WP-T2.
- `WP-G2` - Coder: expose item analysis and connect it to problem inspection, revision/fork, and
  future assignment replacement; depends on WP-G1 and WP-B2.
- `WP-E1` - Playwright operator/tester: add behavior-named professor journeys and live-stack
  evidence; depends on all behavior packages.
- `WP-E2` - Integrator/reviewer: run final gates, visual review, documentation closure, changelog
  update, and first-data baseline procedure; depends on WP-E1.

Each package owns its capability modules and one allocated migration. Shared route registration and
migration ordering belong to the integrator so parallel lanes do not edit the same composition
files.

## Acceptance criteria and gates

- A professor can search broadly, tolerate a simple typo, filter, inspect safe details, favorite a
  problem, place it in a collection, and add it to an assignment without copying an ID manually.
- The assignment editor and Library use the same selection behavior and metadata vocabulary.
- A professor can save an assignment as a personal blueprint and instantiate it in another course.
- An Alpha course is visibly and programmatically non-enrollable, public to approved Instructors,
  creator-editable, and cloneable by any approved Instructor.
- Alpha clones preserve attribution and source manifests but remain independently editable.
- Fast-forward updates only untouched reusable-definition fields before the first issued run;
  divergence and issued work produce safe selected-copy or new-assignment recovery.
- Course rollover shifts schedules without copying any educational record or sensitive group
  membership.
- Co-instructors use the existing Instructor role; Students and Sysadmins without direct membership
  cannot access course records.
- Scheduling, accommodations, manual grading, overrides, item analysis, archive, and retention are
  operable through accessible professor pages.
- No public, learner, non-author, collection, blueprint, or Alpha response contains answer keys,
  grading implementations, private source, email, UUID, or FERPA data.
- All professor pages remain compact and keyboard-complete at 1280 by 800; student pages retain
  tablet and narrow-phone guards.

## Test and verification strategy

- Domain tests cover ownership, relative-calendar scheduling, daylight-saving refusal, normalized
  clone manifests, fast-forward eligibility, and issued-run structural locks.
- Memory/PostgreSQL conformance covers collection ownership, public Alpha reads, creator-only
  writes, cross-tenant cloning, teaching instantiation, rollover exclusions, lifecycle, groups,
  accommodations, overrides, and retention.
- Server tests cover authentication, role checks, non-enumeration, strict request decoding, strong
  revisions, idempotency, cache policy, and absence of secret fields.
- TypeScript/Node tests cover strict decoders, short route references, query/cursor recovery, and
  local state preservation.
- Playwright permanently covers:
  - broad discovery -> collection -> assignment;
  - Alpha creation -> cross-instructor clone -> teaching course;
  - untouched fast-forward and divergent selected copy;
  - rollover with schedule preview;
  - section/accommodation setup;
  - mixed manual grading and explicit override;
  - item analysis -> revision/fork -> future assignment;
  - keyboard, recovery, and canonical viewport behavior.
- Keep the canonical pilot walkthrough focused on its existing known-ID teaching loop; add
  professor-discovery and Alpha journeys as separate visible evidence.
- One-time evidence uses representative PostgreSQL scale for ranking/facets, cross-tenant RLS,
  schedule/DST cases, and complete live professor journeys.
- Every package runs focused gates first, then the applicable generated-contract, Rust, TypeScript,
  Playwright, and repository gates. Final acceptance requires the entire Validation suite from
  `docs/TEST_EVIDENCE_MODEL.md` green on the final material tree with no required skip.

## Migration and compatibility policy

- Preserve the active migration ledger until RC12 and this plan's accepted schema packages are
  complete.
- Because no durable production data exists, change foundational schemas directly and carry no
  compatibility readers.
- After all professor packages pass, execute the reviewed clean-cluster baseline replacement before
  first production data.
- If durable pilot data exists before baseline closure, stop consolidation and use forward-only
  migrations from that point.
- Keep Alpha/shared-curriculum tables outside FERPA course-record ownership while retaining creator
  authorization and public Instructor visibility.
- Keep PostgreSQL search behind the existing repository boundary so a measured future
  search-service replacement changes no UI contract.

## Risk register

- Alpha synchronization complexity: triggered by requests for background updates or merges;
  mitigate with current-source comparison, untouched fast-forward, and selected copy only.
- Reusable-template history growth: triggered by pressure to browse template versions; mitigate
  with mutable current definitions plus bounded clone manifests, not history tables.
- Cross-tenant leakage: triggered by Alpha or collection queries joining tenant records; mitigate
  with separate shared-curriculum stores, public-question validation, forced RLS, and clone-time
  reauthorization.
- Answer leakage: triggered by richer previews or grading aids; mitigate by retaining author-only
  protected preview and answer-free non-author discovery.
- Search-scale regression: triggered by sequential scans or unstable relevance pages; mitigate with
  representative plans, indexed full-text/trigram candidates, stable cursor ties, and measured
  relevance fixtures.
- Schedule surprises: triggered by daylight-saving or holiday shifts; mitigate with
  calendar-relative rules, local-time validation, and a complete clone preview.
- UI sprawl: triggered by duplicating search and selection controls; mitigate with capability-owned
  pages and one shared ProblemPicker.
- Scope collapse: triggered by implementing every lane simultaneously; mitigate with
  dependency-ordered milestones, one-owner packages, focused gates, and independent review.

## Documentation close-out requirements

- Preserve the settled Alpha-course and professor-workflow decisions in `docs/HUMAN_GUIDANCE.md`.
- Save this roadmap under `docs/active_plans/active/` and link it from the implementation status
  without replacing the current release plan.
- Update architecture, file structure, contracts, database structure, install/usage, instructor
  guidance, test evidence, and troubleshooting only when their owning capability changes.
- Add one categorized `docs/CHANGELOG.md` entry after each accepted package.
- Archive the plan only after M5 and the full final Validation suite pass.

## Assumptions

- "Public Alpha course" means visible and cloneable by every approved Instructor, not anonymously
  runnable by Students or the public internet.
- Alpha courses and teaching courses are separate aggregates rather than convertible kinds.
- Alpha assignments and personal blueprints are mutable current state; only published problems and
  issued student evidence retain durable immutable versions.
- Fork lineage is retained, but contribution proposals are outside this plan.
- Relative Alpha schedules use calendar offsets and require clone-time review.
- Existing answer-secrecy, user-role, first-issued-run, retention, human-reference, and
  no-generic-dashboard decisions remain authoritative.
