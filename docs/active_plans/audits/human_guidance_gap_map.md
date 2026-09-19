# Human Guidance implementation gap map

This map records each owning open item from the accepted section audits. Human Guidance is the
authority. A record is closed only by the named correction milestone or, for guidance that makes
no implemented-system claim, by the named audit-classification correction.

## Current ownership and expanded gaps

At the 2026-09-17 reconciliation, the ordered checklist has 1,018 bullets: 472 verified, 498 open,
and 48 N/A. Deferred product behavior remains authority but is excluded from current
implementation-compliance counting. The prior 998-bullet accounting is a historical snapshot. The
[implementation checklist](human_guidance_implementation_checklist.md) owns current
verbatim wording, occurrence status, evidence limitations, and duplicate first-owner pointers.
All existing identity/part/consistency gates pass at the snapshot recorded in the active plan.
Existing A/C identifiers below are historical correction provenance, not ordinal HG identities.
Unchanged accepted terminal/MATCH/scoring/timing/Blueprint evidence is retained.

Current public-ID authority uses seven random Crockford Base32 characters plus one final calculated
checksum, except for the distinct Question/Pool `XXXX-ZXXX` shape; implementation inventory remains open.
Internal non-user-facing objects use native UUID identifiers. Existing `R`, `W`, `D`, `M`, and `I`
short-reference concepts are implementation drift pending audit or removal, not approved public formats.

- Part 01 / Product vocabulary and glossary: new global and topical definitions remain open for
  independent model/behavior evidence. Course Instance creation can select a Blueprint source;
  creating a new Blueprint from existing Course Instance structure establishes Adoption with
  distinct source provenance, counts that Course Instance as the new Blueprint's first Adoption,
  and has accepted canonical HTTPS C420 closure. Non-closable KISS constraints remain binding
  audited N/A.
- Part 03 / Profile avatar interface: current random assignment/gallery consistency, Student
  collection/selection, and shared Instructor/Sysadmin upload/cropping requirements remain open
  absent complete current browser/authorization acceptance. The bounded creation-trigger proof
  does not close those workflows.
- Part 06 / Content classification; Part 07 / Library metadata; Part 08 / Course classification;
  Part 09 / Assessment classification: shared global taxonomy remains a cross-owner gap. The
  Discipline lifecycle decision is implemented in source with stable UUID create/rename/retire/
  restore, no delete, retained visible references, active-only new choice, exact inherited/copy
  retention, and concurrency locks. Final review and major-milestone SQL/browser proof remain open.
  Subjects and narrower classification, normalization, content attachments, hierarchical choices,
  and cross-content discovery retain their separate gaps.

- Part 03 / Role colors and themes: coordinated light/dark and specification cutover are missing;
  distinctness and actual-use accessibility need scoped rendered proof. Durable IDs have narrow
  source proof, not whole-palette acceptance.
- Part 03-05 / expanded interface guidance: five compact Attempt-navigation rows have bounded
  source/rendered acceptance, including the 250-Question harness and 200% enlargement. Full
  before-start/review density, intermediate-width Student layouts, narrow Ribbon, global feedback,
  progressive disclosure, and actual light/dark theme-state contrast remain open.
- Part 01 / installation: current-demo Public Genetics discovery and the ordinary retained-example
  publisher source are accepted context. Fresh default/opt-out installation, unchanged-Revision
  replay, and connected discovery/adoption remain pending, so shipped-example/default-installation
  requirements are open rather than inferred from the existing demo.
- Part 07 / Native Question response presentation: shared bank, partial/reset validation, exact
  four-choice Save/reload, and native mouse drag receipts are retained. Shared-bank desktop
  presentation and keyboard assigning/changing/clearing parity are verified; the latter uses
  normal Tab/Space/Enter swap/Clear/restore/Save. The remaining two rows stay open for adapted
  content/meaning/grading and full bank pointer/touch reachability.
- Part 07 / Draft publication, Published Question and Question Pool metadata, Library metadata:
  required shared metadata, Pool metadata/support, and Subject/Topic/Subtopic hierarchy are not
  established by Question-only nullable Subject/Topic and Title/Description fields.
  The implemented Discipline source lifecycle does not yet close the major-milestone proof.
  Instructor selection, editable Subjects, progressively allowed name lengths, and consistent
  trim/validation remain binding implementation requirements.
- Part 07 / Library stewardship: improvement-thread and impact-notice lifecycles are settled and
  implemented in source. Question owner/Sysadmin administration, Sysadmin-only Pool administration,
  vetted-Instructor text threads, retained resolved/cancelled records, and Sysadmin read-only
  Library mode await final review and major-milestone proof. Fresh PostgreSQL 17 actual-role proof
  closes four-event private Watch delivery as SQL; connected controls and browser notification
  presentation remain open.
- Part 07 / PLE support: accepted C910 general-feedback receipts remain partial context; independent
  Hints/Worked Solutions, disclosure, Pool attachment, coexistence, and Revision behavior remain open.
- Part 07 / Library object statistics and Bloom metadata: Pool statistics and application-owned
  classifier/provider orchestration remain open. Fresh PostgreSQL 17 actual-role proof closes
  protected Bloom preparation, receipt binding, and Question/Pool Library admission as SQL. Exact
  Question and Pool Revision reads now project each pair with its
  independent Edit Number. Typed complete-pair correction Stores and routes, exact-target editors,
  stale reload without automatic retry, draft retention, and completion focus are source-backed;
  connected two-Instructor, denied-role, and browser proof remains open. Library discovery has two
  independent exact filters that combine with existing predicates, retain both values and unchanged
  sort through saved searches, URL handoff, and normalized-query-bound cursors, and reports whole-
  matching-set six-plus-four guide-order counts including zeros and empty results. Connected multi-
  page, role, and browser proof remains open. Assessment source projects exact fixed, retained, and
  Assessment-owned Pool-fork pairs and stably orders them by cognitive process, knowledge dimension,
  then prior position; connected mixed-entry sort/save/reload and concurrent-save proof remains open.
- Part 08 / Blueprint forks: accepted exact-member-pin HTTP evidence is retained. Fresh actual-role
  proof closes Change Proposal persistence, canonical content scope, Entire/Selected acceptance,
  retained outcomes, stale targets, and unchanged daughters as SQL. Connected two-Instructor/API/UI
  review, comparison of changed Questions/Pools, and selected daughter incorporation remain open.
- Part 09 / Assessment Unrelease: current deletion/reset source is partial proof; re-audit explicit
  deletion scope, Student unavailability, teaching-definition retention, normal editing/revalidation,
  and empty later-release work. Preserve accepted unchanged scoring closures and their limitations.
- Part 09 / Attempt timing: finite duration, 250 delivered Questions, Pool selection count,
  calculated default, explicit override, and accommodation rows are reconciled to current Human
  Guidance. The accepted Pool import proof closes the 250 bound only; multi-selection Pool timing
  proof remains open. Accepted independent SQL reproduces the former role-order failure, then proves
  1/2/3/250 default arithmetic, empty/251 refusal, the 12-hour constraint, and ordinary
  Instructor-release/Student-start/resume with a 180-second snapshot. It is not browser, HTTP,
  renderer, real Instructor override-save, or broad delivery acceptance. Ratio/UI accommodations
  and the effective 24-hour cap remain incomplete and unverified. The working-speed rationale is
  audited N/A as pedagogical purpose, not a demonstrated learning effect. Cross-browser session
  resume remains open. Mixed expiry-worker SQL finalization is proven, but rendered unanswered
  presentation and actual Backend transport exclusion remain pending.

## A1: Development principles and product vocabulary

### A1-01

- HG bullet: "Every source file should stay below 1000 lines. Split complete capabilities into focused modules."
- Current evidence and mismatch: `src/style.css` has 1020 lines, and
  `tests/test_source_file_line_limit.py` `test_source_file_line_limit` fails against
  `LINE_LIMIT = 1000`.
- Owning source area: frontend shared styling, `src/style.css`.
- Dependencies: none.
- Closure owner: C1.
- Verification: run `source source_me.sh && python3 -m pytest tests/test_source_file_line_limit.py`;
  confirm every tracked source file remains at or below the limit.

### A1-01a: direct pre-production design correction

- HG bullets: "PLE is pre-production with no users. Fix the design directly rather than preserving legacy behavior." and "PLE is pre-production with no users or durable production data. Improve the design directly."
- Current evidence and mismatch: `crates/project-tools/src/database_coordinator.rs` `run` enforces direct base-schema correction, but that narrow guard is not a repository-wide direct-cutover proof. The obsolete Assessment route layer and tsgen retired-header migration are removed; live CI/A/U/BP client guards and receipt formats remain under audit. One Unrelease mutation path was found; no duplicate-current-path claim is made.
- Closure: audit actual live alternate readers, writers, routes, parsers, DTOs, clients, fallbacks, aliases, and migration paths; remove any obsolete path that lacks a current nonlegacy requirement. Historical documentation, rejection tests, and local variable words are not cleanup targets.
- Verification: record the focused audit result and re-audit both checklist occurrences. No new audit framework or permanent test is authorized.

### A1-02

- HG bullet: "Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details."
- Current evidence and mismatch: `tests/test_test_naming_conventions.py` checks test names only;
  no repository-wide source naming audit establishes the stated rule.
- Owning source area: repository development-conformance tooling in `devel/`.
- Dependencies: none.
- Closure owner: C2.
- Verification: add a focused source-name audit with an allowlist for externally required names;
  run it against tracked source paths and inspect its report for actionable violations.

### A1-03

- HG bullet: "Adaptability should be a focus so the software can evolve as requirements and insights change."
- Current evidence and mismatch: the statement is a development-value instruction, not a specific
  implemented PLE behavior; it has no behavior-level implementation criterion.
- Owning source area: Human Guidance audit classification.
- Dependencies: none.
- Closure owner: C4.
- Verification: re-audit the bullet as `N/A` with a concise reason that it is not separately
  closable. Review Course, Assessment, Question Backend, retention, and authorization changes
  against it as a binding design constraint; run checklist `--consistency` and the A1 gate.

### A1-04

- HG bullet: "Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes."
- Current evidence and mismatch: `tests/test_crate_boundaries.py`
  `test_registry_dependencies_use_open_latest_first_requirements` checks Cargo requirements only;
  PyPI freshness is not established and `pip_requirements.txt` pins `podman-compose==1.6.0`.
- Owning source area: dependency declaration and freshness-check tooling, `Cargo.toml`,
  `package.json`, `pip_requirements*.txt`, and `devel/`.
- Dependencies: none.
- Closure owner: C3.
- Verification: run a deterministic freshness audit that covers all three manifest families,
  records its registry snapshot date, and fails on a dependency known to be behind its supported
  security release; keep the focused dependency test green.

### A1-05

- HG bullet: "If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly."
- Current evidence and mismatch: no slow-interface measurement exists, so the conditional has not
  been triggered and does not require a Rust/WebAssembly implementation.
- Owning source area: Human Guidance audit classification.
- Dependencies: none.
- Closure owner: C4.
- Verification: re-audit the bullet as `N/A`, recording that it is conditional guidance with no
  measured slow interface; run checklist `--consistency` and the A1 gate.

### A1-06

- HG bullet: "Do not create or leave placeholder database tables, states, APIs, workers, or compatibility scaffolding before the feature has an approved product design."
- Current evidence and mismatch: no inventory or focused audit establishes the absence of these
  placeholder and compatibility-scaffolding categories.
- Owning source area: repository development-conformance tooling in `devel/` and approved-design
  references in `docs/`.
- Dependencies: none.
- Closure owner: C2.
- Verification: run the focused inventory against schema, server, and frontend declarations;
  each detected placeholder must point to an approved design or be removed. Inspect the report.

### A1-07

- HG bullet: "The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md)."
- Current evidence and mismatch: priority is a human project-management decision and cannot be
  verified as running-system behavior.
- Owning source area: Human Guidance audit classification.
- Dependencies: none.
- Closure owner: C5.
- Verification: re-audit the bullet as `N/A` with a concise reason that it expresses project
  priority rather than an implemented PLE behavior; run checklist `--consistency` and the A1 gate.

### A1-08

- HG bullet: "**Blueprint Course**: A reusable course used to create **Course Instances**. It has no enrolled **Students** or deadlines."
- Current evidence and mismatch: `schemas/base_schema/50_functions/blueprints.sql` establishes reusable
  Blueprint storage but does not, by itself, establish every stated absence and Course Instance
  creation behavior.
- Owning source area: Blueprint schema and operations, `schemas/base_schema/50_functions/blueprints.sql` and
  `schemas/base_schema/50_functions/course_blueprint_adoption.sql`.
- Dependencies: C7 for Course Instance creation and A8 course/Blueprint findings pending.
- Closure owner: C6.
- Verification: schema fixture proves that Blueprint records cannot carry roster or deadline data;
  service/E2E adoption creates a Course Instance from a Blueprint and leaves Blueprint content
  reusable.

### A1-09

- HG bullet: "**Course Instance**: A course used for teaching. It has **Students**, deadlines, releases, and other course settings. It may be created from a Blueprint Course or started empty."
- Current evidence and mismatch: `CourseInstanceCreationSource` and the strict browser DTO now
  accept `Empty | Adopted`; accepted private HTTP and actual-main browser proof created, persisted,
  and displayed an Empty Instructor Course without a Blueprint-list call, then selected an exact
  Public Blueprint Revision and created a daughter Course with an Unreleased Practice Assessment.
  Student Empty creation was denied. A follow-up actual-main/HTTP proof created an Unreleased
  direct-origin Practice Assessment in that newly browser-created Empty Course; the current
  `ple_data.assessment` origin constraint no longer requires a Blueprint triplet for direct rows.
  Shared Assessment `assessment_status` initial Unreleased state is verified for direct and adopted
  creation, with Template copying flowing through the same direct creator; this closes only the
  exact initial-state Assessment row.
  A second accepted actual-main/HTTP run added/saved one available exact Published Question
  Revision pin, set a valid Due/time limit, passed readiness, and released the direct Assessment.
  Accepted isolated actual-server/exact-main Student proof then imported and claimed a real roster
  invitation, showed the Released direct Assessment to its member, omitted an Unreleased sibling,
  and denied an outsider. The pre-start page displayed title, Type, one Question, one point, the
  one-hour limit, and the explicit no-previous-attempt state. A separate actual-server native
  fixed-Question proof published the checked-in PKU source via Draft API, saved/restored its
  selected opaque choice, whole-submitted a graded 1/1 Attempt, and started a distinct second
  unlimited Practice Attempt afterward. Pool copying, full Student browser Attempt interaction,
  other backend delivery/grading, and the complete started-empty lifecycle remain open.
- Owning source area: Course schema family, `schemas/base_schema/50_functions/course_core.sql`,
  `schemas/base_schema/20_tables/course_membership.sql`, `schemas/base_schema/50_functions/course_roster.sql`, and
  `schemas/base_schema/50_functions/course_operations.sql`.
- Dependencies: C6 contributes Blueprint adoption requirements. C49/C72 make only Public
  Blueprints eligible for the Adopted branch without blocking Empty-root/state work. C503's tagged
  Assessment origin is implemented and direct fixed-Question save/release observed; its remaining
  Pool content/full Student delivery work is required for C7's full started-empty closure.
- Closure owner: C7.
- Verification: accepted private HTTP/exact-main proof established Empty/Adopted root, one adopted
  Unreleased Practice Assessment origin, and one direct Practice Assessment through fixed-Question
  save, Due/time-limit policy save, readiness, and release. Accepted real roster import/claim and
  Student pre-start actual-main proof establishes membership and bounded Released visibility.
  Accepted native fixed-Question HTTP proof adds a saved/submitted graded Attempt and unlimited-
  after-perfect retry. Remaining focused proof must cover Pool content, full browser Attempt work,
  other backend delivery/grading, and
  complete independent settings; keep the course-instance E2E
  lane green.

### A1-10

- HG bullet: "**Published Question**: A validated question in the global **Question Library**, available to vetted **Instructors**."
- Current evidence and mismatch: `schemas/base_schema/50_functions/question_authoring_operations.sql`
  `ple_api.list_question_library_entries` lists published entries but does not establish both
  validation and vetted-Instructor availability.
- Owning source area: Question authoring schema and library operations,
  `schemas/base_schema/question_authoring_*.sql`.
- Dependencies: A2 instructor-vetting findings pending.
- Closure owner: C8.
- Verification: SQL/API fixture publishes a valid Question, rejects an invalid source, allows a
  vetted Instructor to discover it, and denies a non-vetted account; keep the Question Library E2E
  lane green.

### A1-11

- HG bullet: "**Draft Question**: A private question being developed by an **Instructor**. It must pass validation before publication."
- Current evidence and mismatch: `crates/server/src/question_publication.rs`
  `QuestionPublicationService::publish` verifies a stored Draft source record but does not
  establish content validation before publication or private Instructor-only persistence.
- Owning source area: Question publication service, `crates/server/src/question_publication.rs`.
- Dependencies: C8 supplies the Question authoring validation and discovery boundary; A2
  Instructor authorization findings are pending.
- Closure owner: C9.
- Verification: service tests prove only the author can retrieve/edit a Draft and that publication
  rejects invalid Draft content; an authorized valid publication reaches the global library.

### A1-12

- HG bullet: "**Question Library**: The global collection of Published Questions and published Question Pools available to vetted **Instructors**."
- Current evidence and mismatch: `schemas/base_schema/50_functions/question_authoring_operations.sql`
  `question_library_entries` and `src/api/question_library_repository.ts`
  `QuestionLibraryRepository` establish published Question entries and search, but not published
  Question Pool inclusion or vetted-Instructor-only availability.
- Owning source area: Question Library schema contract, `schemas/base_schema/question_authoring_*.sql`.
- Dependencies: A2 instructor-vetting findings pending; C8 also closes Published Question
  validation.
- Closure owner: C8.
- Verification: SQL/API fixture returns both published Questions and published Pools to a vetted
  Instructor, excludes Draft and unpublished Pool records, and denies an unvetted account.

### A1-13

- HG bullet: "**Sysadmin**: A PLE administrator who manages the system, approves **Instructors**, creates accounts, and helps manage courses."
- Current evidence and mismatch: `schemas/base_schema/50_functions/accounts.sql` `ple_api.create_instructor_account`
  proves Instructor-account creation but not system management, approval or identity vetting, or
  course-management help.
- Owning source area: Accounts authorization schema and API, `schemas/base_schema/50_functions/accounts.sql`.
- Dependencies: A2 Accounts and roles findings pending; C7 contributes Course Instance management
  capability.
- Closure owner: C10.
- Verification: role-boundary SQL/API fixture permits Sysadmin account and Instructor-approval
  operations, denies them to other roles, and proves the authorized course-management action.

### A1-14

- HG bullet: "**Instructor**: An approved user who teaches courses and can browse, reuse, create, fork, and publish Questions."
- Current evidence and mismatch: `crates/server/src/question_publication.rs` `publish_new_question`
  establishes only the publication authorization boundary; browse, reuse, create, and fork are
  not directly verified together.
- Owning source area: Instructor Question workflows in the frontend, `src/pages/question_*.tsx`
  and `src/api/question_*.ts`.
- Dependencies: A2 Instructor-approval findings pending; C8 and C9 provide the server Question
  contracts.
- Closure owner: C11.
- Verification: Playwright Instructor workflow covers library browse, reuse, create, fork, and
  publish using a vetted Instructor; unapproved accounts are denied by the API.

### A1-15

- HG bullet: "**Student**: A user enrolled in a **Course Instance** who completes Assessments and other course activities."
- Current evidence and mismatch: browser routes and DTOs still call the graded object `assignment`
  rather than the required Assessment terminology.
- Owning source area: frontend Assessment public boundary, `src/pages/assignment_workspace/`,
  `src/api/`, and browser routes.
- Dependencies: C7 supplies Course Instance enrollment; assessment findings in A9 are pending.
- Closure owner: C12.
- Verification: browser/API contract test enrolls a Student in a Course Instance and completes an
  Assessment through Assessment-named visible UI, route, and DTO surfaces; keep the Student E2E
  lane green.

### A1-16

- HG bullet: "**Assessment Question Editor**: The **Instructor** editor for selecting, adding, removing, and ordering Questions in an Assessment."
- Current evidence and mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx`
  and its routes use Assignment workspace naming rather than Assessment Question Editor.
- Owning source area: frontend Assessment editor, `src/pages/assignment_workspace/`.
- Dependencies: C12 establishes the shared Assessment terminology boundary; A9 Assessment
  interaction findings are pending.
- Closure owner: C12.
- Verification: Playwright Instructor flow finds the Assessment Question Editor, selects, adds,
  removes, and reorders Questions, and asserts Assessment-named route and visible labels.

### A1-17

- HG bullet: "**Assessment Properties Editor**: The **Instructor** editor for settings that apply to the whole Assessment, such as dates, scoring, attempts, late work, and what **Students** can see."
- Current evidence and mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx`
  presents the editor as `Policies`, not Assessment Properties Editor.
- Owning source area: frontend Assessment editor, `src/pages/assignment_workspace/`.
- Dependencies: C12 establishes the shared Assessment terminology boundary; A9 policy behavior
  findings are pending.
- Closure owner: C12.
- Verification: Playwright Instructor flow opens Assessment Properties Editor, changes each named
  whole-Assessment setting, and asserts Assessment-named UI and route contract.

## A2: Accounts and roles

### A2-01

- HG bullet: "Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access."
- Current evidence and mismatch: `crates/server/src/auth/live_demo.rs` defines seeded personas and
  `schemas/installation_data/prepublication_context.sql` inserts demo email addresses, but neither
  establishes that Live Demo email-code delivery is unavailable.
- Owning source area: Live Demo authentication contract, `crates/server/src/auth/live_demo.rs`.
- Dependencies: none.
- Closure owner: C13.
- Verification: focused handler contract test and Live Demo E2E prove seeded-role access works and
  an email-code request is unavailable or non-delivering.

### A2-02

- HG bullet: "**Students** are required to use their university or institutional (`.edu` in the USA) email accounts."
- Current evidence and mismatch: `crates/learning-data-access/src/course_roster.rs`
  `CourseRosterImportInput::validate` accepts a syntactically valid noninstitutional address.
- Owning source area: course-roster data access, `crates/learning-data-access/src/course_roster.rs`.
- Dependencies: none.
- Closure owner: C14.
- Verification: C801 validates a U.S. `.edu` address without accepting a lookalike suffix; C802
  proves lookup-or-create and rejected-input no-side-effect behavior across the complete seed,
  oracle, E2E, browser, and screenshot inventory. Non-U.S institutional-domain authorization is
  intentionally still a product question, not an invented validator rule.

### A2-03

- HG bullet: "**Sysadmin** accounts should require higher security than other accounts, like TOTP authentication"
- Current evidence and mismatch: `schemas/base_schema/20_tables/authentication.sql` provides passkey and
  email authentication but no Sysadmin-only stronger credential or ceremony.
- Owning source area: authentication schema, `schemas/base_schema/20_tables/authentication.sql`.
- Dependencies: none.
- Closure owner: C15.
- Verification: the recorded TOTP decision is implemented in the C803-C807 ceremony: database
  final gate, server pending-MFA path, same Live Demo ceremony, secure local provisioning, and
  connected outcome evidence. Student and Instructor mechanisms remain unchanged.

### A2-04

- HG bullet: "Instructor Accounts may be deactivated without deleting their authored content, Course relationships, or historical records."
- Current evidence and mismatch: `schemas/base_schema/50_functions/accounts.sql` and
  `tests/e2e/e2e_live_demo_instructor_accounts.sh` establish state transition, not preservation
  of every named relationship and history category. `docs/DESIGN_DECISIONS.md` states the intended
  non-destructive behavior.
- Owning source area: account lifecycle schema, `schemas/base_schema/50_functions/accounts.sql`.
- Dependencies: none.
- Closure owner: C16.
- Verification: a transactional fixture creates authored content, membership, and history,
  deactivates the Instructor, then proves their preservation while access is denied.

### A2-05

- HG bullet: "A **Sysadmin** vets an Instructor's real identity before creating the Instructor Account."
- Current evidence and mismatch: `crates/server/src/instructor_account.rs`
  `create_instructor_account` accepts an email without a recorded identity-vetting decision.
- Owning source area: Instructor-account creation service,
  `crates/server/src/instructor_account.rs`.
- Dependencies: C17 creates the durable vetting decision.
- Closure owner: C18.
- Verification: request/route tests reject an absent or invalid decision and accept a completed,
  authorized, audited vetting decision.

### A2-06

- HG bullet: "**Instructors** can browse the content of Public and Archived **Blueprint Courses**."
- Current evidence and mismatch: `crates/server/src/blueprint_course.rs` authorizes current
  Instructor sessions but has no Public-or-Archived Blueprint browse policy.
- Owning source area: Blueprint Course server routes, `crates/server/src/blueprint_course.rs`.
- Dependencies: C6 provides the existing reusable Blueprint boundary; later A8 lifecycle findings
  may add a prerequisite.
- Closure owner: C19.
- Verification: route and E2E cases prove that a vetted Instructor can read Public and Archived
  content without ownership, while Archived content remains non-adoptable.

### A2-07

- HG bullet: "An **Instructor** can reset Student login access and send a new signup code when needed."
- Current evidence and mismatch: `crates/server/src/course_roster.rs` exposes invitation claim and
  revocation routes but no Instructor reset or signup-code-delivery route.
- Owning source area: course-roster server routes, `crates/server/src/course_roster.rs`.
- Dependencies: C14 validates institutional addresses; C23 preserves the course relationship for
  safe restoration.
- Closure owner: C20.
- Verification: focused handler test and invitation-mailer integration prove an authorized reset
  invalidates the prior invitation and produces one new signup delivery.

### A2-08

- HG bullet: "**Student** data should be collected reluctantly, used deliberately, and purged predictably."
- Implementation: use the existing category and operation boundaries, not a field-policy engine.
  Roster import accepts only the institutional email required to resolve or create the global
  Student Account plus the Course-local roster ID required for teaching. `course_roster_profile`
  retains no duplicate email; ordinary Instructor roster projections return only roster ID and
  state, while task-scoped Sysadmin repair may read only one named roster record. The delivery
  email is read only by the exact, authorized,
  pending-invitation export. Student Work stays private Course data and the retention executor
  removes the defined identifiable Course records while preserving global Accounts and
  identity-free statistics.
- Owning source area: `schemas/base_schema/50_functions/course_roster.sql`,
  `schemas/base_schema/50_functions/course_operations.sql`, closed roster DTOs, and the existing
  Course-retention chain.
- Dependencies: C206/C208/C210/C215 establish retention authority and C851 verifies worker
  execution. They are implementation dependencies, not a product question.
- Closure: no unresolved product decision. The qualitative HG rule selects the simplest
  enforceable approach: minimize direct collection at the operation boundary, disclose it only
  when that operation requires it, and purge Course-scoped evidence under stored policy.
- Verification: roster/support E2Es require exact email-free projections; the invitation-export
  E2E proves email is available only to the authorized pending-delivery operation; controlled-clock
  retention proof verifies repeatable purge while Course metadata and global Accounts survive.

### A2-09

- HG bullet: "Student Course data falls under FERPA; treat it as radioactive."
- Current evidence and mismatch: `crates/server/src/support_capability.rs`
  `read_course_roster_entry_repair_support` establishes exact-record repair support, not
  repository-wide FERPA handling.
- Owning source area: support authorization enforcement,
  `schemas/base_schema/50_functions/course_operations.sql`.
- Dependencies: C24 defines platform administration distinct from Course records; C25 supplies
  resource-specific support capabilities.
- Closure owner: C26.
- Verification: authorization E2E proves ordinary Sysadmin administration cannot read FERPA data,
  while task-scoped audited support reads only authorized records.

### A2-10

- HG bullet: "Roster import uses institutional email to find an existing Student Account or create one when needed."
- Current evidence and mismatch: the same `CourseRosterImportInput::validate` permits
  noninstitutional email and the audit does not establish lookup-or-create under the required rule.
- Owning source area: course-roster data access, `crates/learning-data-access/src/course_roster.rs`.
- Dependencies: none.
- Closure owner: C14.
- Verification: C802 proves reuse of an existing Account, exactly-one creation for an accepted
  address, and no side effect for rejected input.

### A2-11

- HG bullet: "Student Work, Attempts, submissions, and grades follow Course retention independently of the Student Account."
- Current evidence and mismatch: no retention runtime or test evidence establishes this lifecycle.
- Owning source area: Student Work retention schema family.
- Dependencies: C21.
- Closure owner: C22.
- Verification: controlled-clock database/job integration proves retention acts on Course records
  without deleting the global Account.

### A2-12

- HG bullet: "Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work."
- Current evidence and mismatch: `schemas/base_schema/50_functions/course_operations.sql`
  `ple_api.revoke_course_roster_entry` records revocation, but does not prove preservation of all
  named records.
- Owning source area: roster relationship operations,
  `schemas/base_schema/50_functions/course_operations.sql`.
- Dependencies: C21.
- Closure owner: C23.
- Verification: transactional integration revokes enrollment, proves access denial, and retains
  Course record, Attempt, submission, and grade rows.

### A2-13

- HG bullet: "Student Work and grades remain subject to the normal Course retention policy after enrollment ends."
- Current evidence and mismatch: no retention runtime or test evidence establishes this behavior.
- Owning source area: Student Work retention schema family.
- Dependencies: C21 and C23.
- Closure owner: C22.
- Verification: a controlled-clock lifecycle test runs retention after enrollment ends and proves
  normal Course-policy disposition.

### A2-14

- HG bullet: "Deactivating Course access does not delete the Student Account or Student Work."
- Current evidence and mismatch: `ple_api.revoke_course_roster_entry` establishes revocation but
  does not prove non-deletion of both the global Account and Student Work.
- Owning source area: roster relationship operations,
  `schemas/base_schema/50_functions/course_operations.sql`.
- Dependencies: C21.
- Closure owner: C23.
- Verification: the revocation fixture proves both the global Account and Student Work remain.

### A2-15

- HG bullet: "An **Instructor** can restore the Student's Course access later."
- Current evidence and mismatch: `crates/server/src/course_roster.rs` exposes claim and revoke,
  not an Instructor restore route.
- Owning source area: course-roster server routes, `crates/server/src/course_roster.rs`.
- Dependencies: C23.
- Closure owner: C20.
- Verification: route and E2E tests demonstrate authorized restoration of original Course access
  without creating an Account or erasing records.

### A2-16

- HG bullet: "A **Sysadmin** has full administrative authority over PLE."
- Current evidence and mismatch: `crates/server/src/instructor_account.rs` supplies
  Instructor-account administration, not a demonstrated full-platform administrative surface.
- Owning source area: Sysadmin authorization schema,
  `schemas/base_schema/50_functions/authorization.sql`.
- Dependencies: none.
- Closure owner: C24.
- Verification: authorization integration establishes a durable Sysadmin platform-administration
  boundary separately from Course membership.

### A2-17

- HG bullet: "Sysadmins vet **Instructors** and create Instructor Accounts."
- Current evidence and mismatch: `crates/server/src/instructor_account.rs` proves Sysadmin-gated
  creation but records no real-identity vetting decision.
- Owning source area: Instructor-account creation service,
  `crates/server/src/instructor_account.rs`.
- Dependencies: C17.
- Closure owner: C18.
- Verification: the create-flow test proves both recorded vetting and Sysadmin authorization are
  required.

### A2-18

- HG bullet: "Sysadmins can help Instructors repair Courses, Students, and content."
- Current evidence and mismatch: `crates/server/src/support_capability.rs` and
  `tests/e2e/e2e_live_demo_support_capability.sh` establish scoped roster support, not repair
  authority for Courses, Students, and content.
- Owning source area: support-capability server routes,
  `crates/server/src/support_capability.rs`.
- Dependencies: C24.
- Closure owner: C25.
- Verification: capability API/E2E proves explicit, limited repair capabilities for every named
  resource class, with purpose and audit record.

### A2-19

- HG bullet: "**Sysadmins** have full platform-administration capability but do not automatically have access to FERPA Course records."
- Current evidence and mismatch: scoped roster support exists, but full platform administration
  is not demonstrated.
- Owning source area: support authorization enforcement,
  `schemas/base_schema/50_functions/course_operations.sql`.
- Dependencies: C24 and C25.
- Closure owner: C26.
- Verification: authorization matrix proves platform actions are allowed while Course-record reads
  are denied absent a support capability.

### A2-20

- HG bullet: "Sysadmin support does not make the Sysadmin an **Instructor** or Course member."
- Current evidence and mismatch: support issuance is separate from Course membership, but the
  audit has no direct proof that issuance cannot create Instructor or Course-member authority.
- Owning source area: support authorization enforcement,
  `schemas/base_schema/50_functions/course_operations.sql`.
- Dependencies: C24 and C25.
- Closure owner: C26.
- Verification: capability issuance asserts no role or membership row changes and verifies an
  Instructor-only operation remains denied.

## A3: Interface shell

The following records use the accepted `03_shell.md` evidence. Later duplicate HG occurrences
are not repeated here. `product decision still unclear` is a positive audit result: the quoted
question identifies the missing decision that HG and the investigated repository do not answer.
There are 36 owning open bullets: 34 have exactly one closure owner, A3-16 is the one recorded
product question, and A3-21 is the one N/A permission classification.

### A3-01

- HG bullet: "Instructor and **Sysadmin** workflows should work well in a 1280 by 800 desktop browser viewport."
- Evidence and mismatch: `tests/playwright/ribbon_responsive_evidence.mjs` covers the
  Instructor desktop profile only; it does not exercise Sysadmin Instructor Accounts or Scoped
  Support Roster at 1280 by 800.
- Owning source area: Ribbon browser evidence, `tests/playwright/ui_corpus_manifest.ts`.
- Dependencies: none. Closure owner: C27.
- Verification: `node tests/playwright/ribbon_responsive_evidence.mjs`; a temporary leased-stack
  probe visits `/sysadmin/instructor-accounts` and `/sysadmin/support-roster` at 1280 by 800;
  `bash tests/e2e/e2e_live_demo_instructor_accounts.sh`; `bash tests/e2e/e2e_live_demo_support_capability.sh`.

### A3-02 through A3-06

- HG bullets: "Design around what users need to find and do."; "Important information should stand out from supporting information."; "Related information should be visually grouped and aligned."; "Similar pages should place similar controls in consistent locations."; "Primary actions should be easy to find and appear near the content or workflow they affect."
- Evidence and mismatch: local hierarchy and controls exist, but no repository-wide behavior or
  usability evidence defines primary tasks, information priority, groups, comparable page sets,
  or primary actions.
- Owning source area: shared task hierarchy and page-action placement. Dependencies: C69.
  Closure owner: C69.
- Verification: the one-time HCI walkthrough ledger follows the task models and verifies the
  four hierarchy levels, local primary action, grouped controls, keyboard path, and axe result
  for each role screenshot; it is removed after review rather than made a brittle permanent test.

### A3-07, A3-25 through A3-27

- HG bullets: "Avoid scattering related actions across page headers, menus, navigation, and content areas."; "Clicking the Profile avatar opens the Profile menu."; "The Profile menu contains Profile settings, account settings, and Sign Out."; "Sign Out belongs in the Profile menu rather than the main top bar."
- Evidence and mismatch: `src/ribbon/app_ribbon.tsx` renders main-bar Sign Out; `src/ribbon/ribbon_contract.ts` gives Profile `href: "/profile"`; no menu exists.
- Owning source area: Ribbon presentation. C36/C818 close only avatar-opens-menu and Sign Out
  relocation. C819-C823 own the real Profile and recorded-scope Account Settings routes, then
  the final menu-contents/no-scattering closure; no fake or disabled controls stand in for them.
- Verification: C818 retains stable pointer/keyboard/focus/relocation behavior. C820 and C822
  add their real routes only after their prerequisites; C823 runs the final route/contract and
  leased M10 shell evidence.

### A3-08 through A3-11

- HG bullets: "Optimize large collections for scanning, searching, filtering, and comparison."; "Show enough useful information at once to support comparison without excessive scrolling."; "Search and filters should help users quickly narrow large collections."; "Dense pages should remain easy to scan."
- Evidence and mismatch: library controls exist but no all-collection inventory, required comparison
  fields, narrowing target, or scrolling threshold exists.
- Owning source area: collection presentation and query controls. Dependencies: C70. Closure
  owner: C70.
- Verification: the one-time HCI walkthrough ledger uses the Question, Course, and Assessment
  task models to verify scanning fields, visible query/filter state, comparison path, keyboard
  operation, axe, and role screenshots without turning visual judgement into a pixel metric.

### A3-12 through A3-15

- HG bullets: "Use spacing to separate meaningful groups rather than simply making pages spacious."; "Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers."; "Keep the visual design compact, flat, information dense, and consistent across PLE."; "Dream big on the UI. Choose one visual philosophy and carry it through the entire interface."
- Evidence and mismatch: current pages mix cards and borders; no named visual philosophy or
  measurable page-level acceptance criteria exists.
- Owning source area: shared precision-field-console styling. Dependencies: C71. Closure owner: C71.
- Verification: one-time HCI walkthrough ledger and role screenshots apply
  `docs/UI_DESIGN_GUIDE.md`'s precision-field-console, hierarchy, grouping, token, keyboard, and
  axe criteria; do not retain screenshots or subjective walkthrough checks as permanent tests.

### A3-16

- HG bullet: "Use drag-and-drop where it makes reordering faster and more natural."
- Evidence and mismatch: known reorder surfaces are Blueprint reusable Assignment entries
  (`src/features/blueprint_course/blueprint_assignment_content_editor.tsx`), Assignment Workspace
  entries (`src/pages/assignment_workspace/assignment_workspace_questions_page.tsx`), and native
  JSON single-choice/matching/ordering editors (`src/features/ple_question_json_authoring/`);
  none has drag-and-drop.
- Disposition: `Reason: product decision still unclear`.
- Both plausible behaviors: retain precise button/keyboard reordering where drag would not be
  faster, or add drag-and-drop to one or more named authoring lists. HG requires the former
  accessibility foundation but does not select the latter surfaces.
- Question: For which named surfaces does drag-and-drop make reordering faster and more natural
  than explicit Move earlier/Move later controls? This decision follows C28.

### A3-17

- HG bullet: "Reordering must also have a precise keyboard-accessible method."
- Evidence and mismatch: individual editor controls exist, but no all-surface inventory proves a
  keyboard method for every current reorderer.
- Owning source area: frontend reorder controls in the three A3-16 source families.
- Dependencies: none. Closure owner: C28.
- Verification: temporary reorder-inventory keyboard probe; focused model tests; `./check_codebase.sh`; fast checks.

### A3-18

- HG bullet: "Themes should use biome and habitat names."
- Closure receipt: C29 preserved stored theme IDs while presenting the closed registry with biome
  and habitat names, including Grassland.
- Evidence: `src/features/course_appearance/course_theme_registry.ts` `COURSE_THEME_REGISTRY`,
  `src/pages/course_appearance_page.tsx` `COURSE_THEME_OPTIONS`, and
  `tests/test_course_theme_scope.mjs` `Grassland uses the Roosevelt-inspired anchors and accessible derived actions`.

### A3-18a

- HG bullet: "Implement the themes as specified in `docs/BIOME_THEME_PALETTES.md`"
- Current evidence and mismatch: `src/features/course_appearance/course_theme_registry.ts` still
  implements the current 15-ID, three-light-anchor registry. `docs/BIOME_THEME_PALETTES.md`
  specifies a proposed 25-theme light/dark four-color registry and identifies six unresolved
  decisions that block its atomic runtime cutover.
- Owning source area: Course Theme registry, persistence/readers, display-mode behavior, semantic
  token projection, and rendered acceptance named by `docs/BIOME_THEME_PALETTES.md`.
- Dependencies: explicit product direction on all six unresolved decisions in
  `docs/BIOME_THEME_PALETTES.md`. Closure owner: not yet assigned.
- Verification: satisfy the specification's complete acceptance criteria after an atomic cutover;
  do not infer the unresolved choices during unrelated work.

### A3-19

- HG bullet: "UUIDs should never appear in visible content, navigation URLs, or copyable links."
- Evidence and mismatch: `tests/test_public_navigation.mjs` covers route references only, not all
  rendered text or copy/link producers.
- Owning source area: public-reference presentation (`src/navigation/public_route.ts`,
  `src/navigation/resolved_route.ts`, `src/route_contract.ts`) and inventory-discovered consumers.
- Dependencies: pending A4/A7/A8/A9 presentation inventories. Closure owner: C30.
- Verification: temporary inventory covers route templates/dynamic hrefs, rendered API-derived text,
  clipboard/copy/share/export URLs, and API-to-display seams; then public-navigation tests and fast checks.

### A3-20

- HG bullet: "Use Atkinson Hyperlegible Mono for code and other monospace text."
- Evidence and mismatch: no bundled Mono asset or `@font-face` use exists.
- Owning source area: `src/styles/browser_fonts.css`, `src/style.css`, and local font assets.
- Dependencies: none. Closure owner: C31.
- Verification: temporary computed-style/font-source probe; `./check_codebase.sh`; fast checks.

### A3-21

- HG bullet: "Question Backend-rendered content may use its own fonts when needed for correct display."
- Disposition: `N/A`. Reason: permission for a future backend rendering need, not a current PLE
  requirement; no audited backend currently requires a font exception.

### A3-22

- HG bullet: "Students should have no upload capabilities. Instructor-created content should use text boxes."
- Evidence and mismatch: absent Student UI alone cannot prove API denial; current Instructor pages
  include both textareas and profile/course-appearance file inputs, without an authoritative
  inventory distinguishing educational content from profile/media settings.
- Owning source area: upload authorization/API (C32) then frontend authoring controls (C33).
- Dependencies: A2 role/authorization inventory, C32. Closure owner: C33; C32 contributes.
- Verification: server role-denial matrix for every upload endpoint; temporary authoring-control
  inventory; focused Rust/Node gates and fast checks.

### A3-23

- HG bullet: "Each Product Role has its own home dashboard and navigation."
- Evidence and mismatch: role-specific Ribbon tabs exist, but `src/route_contract.ts` has one `/`
  course-list route and does not prove a distinct dashboard for all roles.
- Owning source area: frontend route/Ribbon contract. Dependencies: A2 role facts. Closure owner: C34.
- Verification: temporary three-role home-route matrix; Ribbon browser evidence; codebase and fast checks.

### A3-24 and A3-28

- HG bullets: "Profile appears at the far right as an icon-only avatar."; "The Profile avatar uses a generic user avatar until the user selects another avatar."
- Evidence and mismatch: Instructor-only `RibbonProfileAvatar` composition does not provide the shared all-role control/fallback.
- Owning source area: Ribbon presentation. Dependencies: C34. Closure owner: C35.
- Verification: temporary three-role Ribbon matrix; responsive evidence; codebase and fast checks.

### A3-29 through A3-32

- HG bullets: "**Students** select avatars from a PLE-provided collection and cannot upload Profile images."; "Student avatar selection should be visual and playful, similar to choosing a LEGO avatar."; "**Instructors** and **Sysadmins** may select a provided avatar or add their own Profile image."; "The current avatar appears consistently anywhere PLE represents that user."
- Evidence and mismatch: Instructor thumbnail storage is self-only and Instructor-specific; Student/Sysadmin
  selection, role-neutral persistence, and cross-surface projection do not exist.
- Architect-approved design: `profile_media.sql` owns role-neutral `account_avatar` with a closed
  `ProvidedAvatarId` catalog and a discriminated `provided`/`profile-image` shape. Authenticated
  self routes use `/api/profile/avatar*`; Student image-upload requests are denied; Instructor and
  Sysadmin image delivery remains self-only.
- Dependencies and closure: C832 catalog source -> C833 catalog facts -> C834 generated-registry
  conformance; `{C812, C833} -> C38 -> C39 -> C819 -> C820`; C832 -> C835 reusable visual
  components; `{C39, C820, C834, C835} -> C836` real Profile integration. C836 then supplies
  C40 Student acceptance (A3-29/A3-30) and C41 Instructor/Sysadmin acceptance (A3-31).
  C837 projects generic/provided static assets only; C42 (A3-32) remains blocked on its explicit
  cross-account private-image product question.
- Closure owners: A3-29: C40; A3-30: C40; A3-31: C41; A3-32: C42. C37-C39 and C832-C837 are
  contributors/recipients only; no new row changes checklist ownership or permits premature `[x]`.
- Verification: focused schema/Rust authorization tests, temporary three-role browser flows, and
  self-only image-delivery plus Student-upload-denial live cases. Retain the behavior-level
  role/IDOR E2E because this self-only public-avatar authorization boundary is intentionally
  stable, important, and plausibly regressive. Parser/normalization permutations are temporary
  and removed. A failure is a public-avatar authorization regression: repair route/store policy
  before HG closure.

### A3-33 through A3-36

- HG bullets: "All signed-in users have a permanent breadcrumb row below the top Ribbon."; "The breadcrumb row remains in the same location and keeps the same space as users navigate."; "Breadcrumbs show the path from the user's home dashboard to the current page."; "Each breadcrumb level links back to its corresponding page."
- Evidence and mismatch: `src/application_shell.tsx` conditionally renders the prelude and allows a
  breadcrumb span without `href`; routes do not establish each role-home-rooted path.
- Owning source area: shell/breadcrumb route model. Dependencies: C34 and pending route inventories.
  Closure owner: C43.
- Verification: temporary route-table and cross-role breadcrumb traversal matrix; `node tests/playwright/ribbon_shell_evidence.mjs`; codebase and fast checks.

## A4: Instructor interface

The A4 audit has 88 owning unchecked bullets. There are no exact duplicates and no HG-unlocked
or product-decision exemption. C49, C64, C66, C72, C73, and C74 contribute prerequisites but
flip no A4 bullet. The following are per-bullet records; each repeats its evidence,
source, dependencies, closure owner, and verification rather than inheriting those fields.

### A4-01 - Instructor navigation and empty states (frontend ribbon/shared collection UI; 8)

#### A4-01.1

- HG bullet: "The Instructor interface should make frequent teaching tasks fast and easy to find."
- Current evidence and concrete mismatch: current Assessment labels/routes exist, but required navigation, first actions, shared placement, task-centered layout, and broad dense-list acceptance remain incomplete.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/library_page.tsx.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Verification: route tests for zero-record collections and desktop Playwright navigation/empty-state screenshots.

#### A4-01.2

- HG bullet: "The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar."
- Current evidence and concrete mismatch: the catalog now names the Assessment area, but no accepted desktop evidence establishes the required dense top-bar workflow.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/library_page.tsx.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Verification: route tests for zero-record collections and desktop Playwright navigation/empty-state screenshots.

#### A4-01.3

- HG bullet: "All required ribbon choices remain visible even when their collection is empty."
- Current evidence: source, focused component evidence, and canonical connected HTTPS proof show
  that all required Course choices remain visible with zero Course rows. Implemented choices retain
  their exact links, and the empty collection presents its honest state and first action.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/app_ribbon.tsx, src/ribbon/app_ribbon.css, src/ribbon/ribbon_catalog.ts.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Closure receipt: focused Ribbon component tests and canonical desktop connected proof passed with
  an empty Course collection. The official screenshot corpus remains historical pending replay.

#### A4-01.4

- HG bullet: "All required Instructor ribbon choices remain visible even when their target page is not implemented or complete."
- Current evidence: focused component evidence and canonical connected HTTPS proof retain every
  required Instructor choice. Working choices navigate through exact links; unfinished destinations
  remain visible as labeled non-links with `aria-disabled="true"`.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/app_ribbon.tsx, src/ribbon/app_ribbon.css, src/ribbon/ribbon_catalog.ts.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Closure receipt: focused Ribbon component tests and canonical desktop connected proof passed for
  working and unfinished destinations. The official screenshot corpus remains historical pending replay.

#### A4-01.5

- HG bullet: "Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so."
- Current evidence and concrete mismatch: `src/pages/library_page.tsx` has no first-item action, and no accepted empty-collection action evidence covers the required pages.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/library_page.tsx.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Verification: route tests for zero-record collections and desktop Playwright navigation/empty-state screenshots.

#### A4-01.6

- HG bullet: "Similar pages should place similar actions in consistent locations."
- Current evidence and concrete mismatch: no cross-page placement contract or acceptance evidence establishes consistent actions.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/library_page.tsx.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Verification: route tests for zero-record collections and desktop Playwright navigation/empty-state screenshots.

#### A4-01.7

- HG bullet: "Instructor pages should be composed around the teaching task rather than collections of padded components."
- Current evidence and concrete mismatch: no rendered-layout or workflow evidence establishes task-centered Instructor pages.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/library_page.tsx.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Verification: route tests for zero-record collections and desktop Playwright navigation/empty-state screenshots.

#### A4-01.8

- HG bullet: "Instructor Course and Assessment lists should be dense and easy to scan, more like a spreadsheet than cards."
- Current evidence and concrete mismatch: Course and Assessment rows exist, but no accepted visual evidence establishes shared spreadsheet-like scanning.
- Owning source area: shared Instructor-navigation frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/library_page.tsx.
- Dependencies: C12, C46, C56, and C61.
- Closure owner: C44.
- Verification: route tests for zero-record collections and desktop Playwright navigation/empty-state screenshots.

### A4-02 - Student View read-only projection (preview route/server request contract; 1)

#### A4-02.1

- HG bullet: "Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades."
- Current evidence and concrete mismatch: `src/pages/assignment_preview_page.tsx` supplies Assignment preview text only; it does not establish answer-free Assessment projection or no writes.
- Owning source area: Student View frontend: src/pages/assignment_preview_page.tsx.
- Dependencies: C12, C74, and pending A9 no-write preview API.
- Closure owner: C45.
- Verification: seeded Student Work/Attempt fixture; browser preview; database/API assertion that no Attempt, submission, or grade is created and answer content is absent.

### A4-03 - Course destinations and active/inactive lists (frontend Course routes/list state; 5)

#### A4-03.1

- HG bullet: "The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses."
- Current evidence and concrete mismatch: Source-backed `myActiveCourses` and `myInactiveCourses`
  Ribbon targets now route to separate strict lifecycle projections and list pages. The real
  restricted-role transition and canonical browser journey remain unproved; Public Blueprint search
  and upcoming Assessment activity remain separate gaps.
- Owning source area: Course destinations frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/course_instance_page.tsx.
- Dependencies: C206/C208/C215/C851/C209 transition receipt; C47, C55.
- Closure owner: C46.
- Verification: route/list tests with zero, active, inactive fixtures; Playwright checks all four choices and separated lists.

#### A4-03.2

- HG bullet: "My Active Courses and My Inactive Courses should both be available from the Courses area."
- Current evidence and concrete mismatch: Source-backed Ribbon targets and separate Active/Inactive
  routes now exist. The real restricted-role transition and canonical browser journey remain
  unproved.
- Owning source area: Course destinations frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/course_instance_page.tsx.
- Dependencies: C206/C208/C215/C851/C209 transition receipt; C47, C55.
- Closure owner: C46.
- Verification: route/list tests with zero, active, inactive fixtures; Playwright checks all four choices and separated lists.

#### A4-03.3

- HG bullet: "**My Active Courses** should emphasize Course Instances the Instructor is currently teaching."
- Current evidence and concrete mismatch: The strict lifecycle projection and `/instructor` Active
  list are source-backed. The real restricted-role transition and canonical browser journey remain
  unproved.
- Owning source area: Course destinations frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/course_instance_page.tsx.
- Dependencies: C206/C208/C215/C851/C209 transition receipt; C47, C55.
- Closure owner: C46.
- Verification: route/list tests with zero, active, inactive fixtures; Playwright checks all four choices and separated lists.

#### A4-03.4

- HG bullet: "Active Course Instances should make upcoming Assessments and important course activity easy to find."
- Current evidence and concrete mismatch: The Active list is source-backed, but it has no upcoming
  Assessment or important-activity surface. The real restricted-role transition and canonical
  browser journey remain unproved.
- Owning source area: Course destinations frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/course_instance_page.tsx.
- Dependencies: C206/C208/C215/C851/C209 transition receipt; C47, C55.
- Closure owner: C46.
- Verification: route/list tests with zero, active, inactive fixtures; Playwright checks all four choices and separated lists.

#### A4-03.5

- HG bullet: "**My Inactive Courses** should keep past Course Instances available without competing with active Course Instances."
- Current evidence and concrete mismatch: The strict lifecycle projection and
  `/instructor/courses/inactive` list are source-backed. The real restricted-role transition and
  canonical browser journey remain unproved.
- Owning source area: Course destinations frontend: src/ribbon/ribbon_catalog.ts, src/pages/course_list_page.tsx, src/pages/course_instance_page.tsx.
- Dependencies: C206/C208/C215/C851/C209 transition receipt; C47, C55.
- Closure owner: C46.
- Verification: route/list tests with zero, active, inactive fixtures; Playwright checks all four choices and separated lists.

### A4-04 - Public Blueprint Course search (frontend public-Blueprint search route/query UI; 2)

#### A4-04.1

- HG bullet: "**Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind."
- Current evidence and concrete mismatch: `searchPublicBlueprintCourses` is `future` in `src/ribbon/ribbon_catalog.ts`; no public Blueprint route/query controls can narrow a large collection.
- Owning source area: Public Blueprint search frontend: new src/pages/blueprint_course_search_page.tsx and src/api/blueprint_course.ts.
- Dependencies: pending A8 public discovery API; C46.
- Closure owner: C47.
- Verification: focused query-state tests and Playwright seeded large-collection search.

#### A4-04.2

- HG bullet: "Public Blueprint Course search should support quickly narrowing a large collection."
- Current evidence and concrete mismatch: `searchPublicBlueprintCourses` is `future` in `src/ribbon/ribbon_catalog.ts`; no public Blueprint route/query controls can narrow a large collection.
- Owning source area: Public Blueprint search frontend: new src/pages/blueprint_course_search_page.tsx and src/api/blueprint_course.ts.
- Dependencies: pending A8 public discovery API; C46.
- Closure owner: C47.
- Verification: focused query-state tests and Playwright seeded large-collection search.

### A4-05 - Blueprint Assessment editor surface (Blueprint Course frontend editor; 5)

#### A4-05.1

- HG bullet: "Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor."
- Current evidence and concrete mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` uses `setSelectedAssignment` and a Blueprint Assignment editor; `blueprint_assignment_content_editor.tsx` has no separate Properties editor. Required Assessment vocabulary, scope, and two-editor structure are absent.
- Owning source area: Blueprint editor frontend: src/features/blueprint_course/blueprint_course_workspace.tsx and src/features/blueprint_course/blueprint_assignment_content_editor.tsx.
- Dependencies: pending Assessment terminology/A8 Blueprint model; C50.
- Closure owner: C48.
- Verification: component plus Playwright test selects one of two Blueprint Assessments, shows only its Questions, reorders them, then opens Properties.

#### A4-05.2

- HG bullet: "Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment."
- Current evidence and concrete mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` uses `setSelectedAssignment` and a Blueprint Assignment editor; `blueprint_assignment_content_editor.tsx` has no separate Properties editor. Required Assessment vocabulary, scope, and two-editor structure are absent.
- Owning source area: Blueprint editor frontend: src/features/blueprint_course/blueprint_course_workspace.tsx and src/features/blueprint_course/blueprint_assignment_content_editor.tsx.
- Dependencies: pending Assessment terminology/A8 Blueprint model; C50.
- Closure owner: C48.
- Verification: component plus Playwright test selects one of two Blueprint Assessments, shows only its Questions, reorders them, then opens Properties.

#### A4-05.3

- HG bullet: "Only the selected Blueprint Assessment's Questions should appear in its editor."
- Current evidence and concrete mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` uses `setSelectedAssignment` and a Blueprint Assignment editor; `blueprint_assignment_content_editor.tsx` has no separate Properties editor. Required Assessment vocabulary, scope, and two-editor structure are absent.
- Owning source area: Blueprint editor frontend: src/features/blueprint_course/blueprint_course_workspace.tsx and src/features/blueprint_course/blueprint_assignment_content_editor.tsx.
- Dependencies: pending Assessment terminology/A8 Blueprint model; C50.
- Closure owner: C48.
- Verification: component plus Playwright test selects one of two Blueprint Assessments, shows only its Questions, reorders them, then opens Properties.

#### A4-05.4

- HG bullet: "**Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment."
- Current evidence and concrete mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` uses `setSelectedAssignment` and a Blueprint Assignment editor; `blueprint_assignment_content_editor.tsx` has no separate Properties editor. Required Assessment vocabulary, scope, and two-editor structure are absent.
- Owning source area: Blueprint editor frontend: src/features/blueprint_course/blueprint_course_workspace.tsx and src/features/blueprint_course/blueprint_assignment_content_editor.tsx.
- Dependencies: pending Assessment terminology/A8 Blueprint model; C50.
- Closure owner: C48.
- Verification: component plus Playwright test selects one of two Blueprint Assessments, shows only its Questions, reorders them, then opens Properties.

#### A4-05.5

- HG bullet: "**Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see."
- Current evidence and concrete mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` uses `setSelectedAssignment` and a Blueprint Assignment editor; `blueprint_assignment_content_editor.tsx` has no separate Properties editor. Required Assessment vocabulary, scope, and two-editor structure are absent.
- Owning source area: Blueprint editor frontend: src/features/blueprint_course/blueprint_course_workspace.tsx and src/features/blueprint_course/blueprint_assignment_content_editor.tsx.
- Dependencies: pending Assessment terminology/A8 Blueprint model; C50.
- Closure owner: C48.
- Verification: component plus Playwright test selects one of two Blueprint Assessments, shows only its Questions, reorders them, then opens Properties.

### A4-08 - Blueprint adoption projection (A8 adoption service/serialization; 2)

#### A4-08.1

- HG bullet: "Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings."
- Current evidence and concrete mismatch: `tests/e2e/e2e_live_demo_course_instance.sh` adopts Blueprint Assignments, not the required Assessment-named contents/settings or unreleased/no-date initial state.
- Owning source area: Blueprint adoption schema/server: schemas/base_schema/50_functions/course_blueprint_adoption.sql and crates/server/src/course_instance.rs.
- Dependencies: C6, C7, C12, C48, C49, C72, and C73.
- Closure owner: C52.
- Verification: adoption E2E with Questions, pools, settings, release, and dates assertions.

#### A4-08.2

- HG bullet: "Assessments created from a Blueprint Course start unreleased with dates unset."
- Current evidence and concrete mismatch: `tests/e2e/e2e_live_demo_course_instance.sh` adopts Blueprint Assignments, not the required Assessment-named contents/settings or unreleased/no-date initial state.
- Owning source area: Blueprint adoption schema/server: schemas/base_schema/50_functions/course_blueprint_adoption.sql and crates/server/src/course_instance.rs.
- Dependencies: C6, C7, C12, C48, C49, C72, and C73.
- Closure owner: C52.
- Verification: adoption E2E with Questions, pools, settings, release, and dates assertions.

### A4-09 - Six-month Course Instance validation (A8 Course domain validation; 1)

#### A4-09.1

- HG bullet: "A Course Instance represents one teaching period and remains Active for at most six months from creation."
- Current evidence and concrete mismatch: `crates/question_model/src/course_term.rs` `CourseTerm::new` checks only date order, not active duration from creation.
- Owning source area: Course term domain validation: crates/question_model/src/course_term.rs.
- Dependencies: pending A8 Course create/update API.
- Closure owner: C53.
- Verification: deterministic-clock boundary tests reject over-six-month Active state and accept the valid boundary.

### A4-10 - Course banner presentation (course-appearance frontend CSS/guidance; 5)

#### A4-10.1

- HG bullet: "Course banners use a 5:1 aspect ratio."
- Current evidence and concrete mismatch: `COURSE_ENTRY_IDENTITY_STYLES` is 6:1; authoring guidance is 1200-by-200; existing Playwright evidence is one viewport and no high-resolution acceptance. Centered non-hero rendering is unproved.
- Owning source area: Course appearance frontend: src/features/course_appearance/course_entry_identity.tsx and src/pages/course_appearance_page.tsx.
- Dependencies: none.
- Closure owner: C54.
- Verification: C814 inventories all model/server/course-media/object/frontend/test consumers;
  C815 accepts valid 5:1 inputs within independent safety bounds and rejects non-5:1 without
  cropping; C813 proves centered responsive delivery. 1280x256 is recommended, not an exact or
  minimum normalization rule.

#### A4-10.2

- HG bullet: "1280 by 256 pixels is the recommended Course banner authoring size."
- Current evidence and concrete mismatch: `COURSE_ENTRY_IDENTITY_STYLES` is 6:1; authoring guidance is 1200-by-200; existing Playwright evidence is one viewport and no high-resolution acceptance. Centered non-hero rendering is unproved.
- Owning source area: Course appearance frontend: src/features/course_appearance/course_entry_identity.tsx and src/pages/course_appearance_page.tsx.
- Dependencies: none.
- Closure owner: C54.
- Verification: C815 accepts 1280x256 and higher-resolution 5:1 inputs; smaller valid 5:1 input
  is also permitted within safety bounds. Exact rendition choice remains an unlocked design.

#### A4-10.3

- HG bullet: "Higher-resolution 5:1 Course banner images are supported."
- Current evidence and concrete mismatch: `COURSE_ENTRY_IDENTITY_STYLES` is 6:1; authoring guidance is 1200-by-200; existing Playwright evidence is one viewport and no high-resolution acceptance. Centered non-hero rendering is unproved.
- Owning source area: Course appearance frontend: src/features/course_appearance/course_entry_identity.tsx and src/pages/course_appearance_page.tsx.
- Dependencies: none.
- Closure owner: C54.
- Verification: C815 uses small, 1280x256, and 2560-wide valid fixtures plus a non-5:1 rejection;
  C813 verifies responsive no-crop delivery.

#### A4-10.4

- HG bullet: "PLE responsively scales Course banners while preserving their aspect ratio."
- Current evidence and concrete mismatch: `COURSE_ENTRY_IDENTITY_STYLES` is 6:1; authoring guidance is 1200-by-200; existing Playwright evidence is one viewport and no high-resolution acceptance. Centered non-hero rendering is unproved.
- Owning source area: Course appearance frontend: src/features/course_appearance/course_entry_identity.tsx and src/pages/course_appearance_page.tsx.
- Dependencies: none.
- Closure owner: C54.
- Verification: C813 verifies 1280-by-800 and narrow viewports after C815 preserves 5:1/no crop.

#### A4-10.5

- HG bullet: "Course banners appear as small centered banners rather than full-width page heroes."
- Current evidence and concrete mismatch: `COURSE_ENTRY_IDENTITY_STYLES` is 6:1; authoring guidance is 1200-by-200; existing Playwright evidence is one viewport and no high-resolution acceptance. Centered non-hero rendering is unproved.
- Owning source area: Course appearance frontend: src/features/course_appearance/course_entry_identity.tsx and src/pages/course_appearance_page.tsx.
- Dependencies: none.
- Closure owner: C54.
- Verification: C813's temporary viewport evidence proves centered non-hero presentation.

### A4-11 - Course Instance Assessment editors (Course frontend workspace; 6)

#### A4-11.1

- HG bullet: "The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once."
- Current evidence and concrete mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor; `AssignmentWorkspaceQuestionsPage` and `AssignmentWorkspacePoliciesPage` are Assignment-named, rather than separate Assessment Question and Properties editors.
- Owning source area: Course/Assessment workspace frontend: src/pages/course_instance_page.tsx and src/pages/assignment_workspace/.
- Dependencies: pending terminology; C52.
- Closure owner: C55.
- Verification: route/component and Playwright test changes Questions and Properties separately then reloads.

#### A4-11.2

- HG bullet: "Selecting an Assessment in the Course Editor opens that Assessment for editing."
- Current evidence and concrete mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor; `AssignmentWorkspaceQuestionsPage` and `AssignmentWorkspacePoliciesPage` are Assignment-named, rather than separate Assessment Question and Properties editors.
- Owning source area: Course/Assessment workspace frontend: src/pages/course_instance_page.tsx and src/pages/assignment_workspace/.
- Dependencies: pending terminology; C52.
- Closure owner: C55.
- Verification: route/component and Playwright test changes Questions and Properties separately then reloads.

#### A4-11.3

- HG bullet: "Assessment content and Assessment properties should remain separate editing tasks."
- Current evidence and concrete mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor; `AssignmentWorkspaceQuestionsPage` and `AssignmentWorkspacePoliciesPage` are Assignment-named, rather than separate Assessment Question and Properties editors.
- Owning source area: Course/Assessment workspace frontend: src/pages/course_instance_page.tsx and src/pages/assignment_workspace/.
- Dependencies: pending terminology; C52.
- Closure owner: C55.
- Verification: route/component and Playwright test changes Questions and Properties separately then reloads.

#### A4-11.4

- HG bullet: "Course Instance Assessments have two editors:"
- Current evidence and concrete mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor; `AssignmentWorkspaceQuestionsPage` and `AssignmentWorkspacePoliciesPage` are Assignment-named, rather than separate Assessment Question and Properties editors.
- Owning source area: Course/Assessment workspace frontend: src/pages/course_instance_page.tsx and src/pages/assignment_workspace/.
- Dependencies: pending terminology; C52.
- Closure owner: C55.
- Verification: route/component and Playwright test changes Questions and Properties separately then reloads.

#### A4-11.5

- HG bullet: "**Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment."
- Current evidence and concrete mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor; `AssignmentWorkspaceQuestionsPage` and `AssignmentWorkspacePoliciesPage` are Assignment-named, rather than separate Assessment Question and Properties editors.
- Owning source area: Course/Assessment workspace frontend: src/pages/course_instance_page.tsx and src/pages/assignment_workspace/.
- Dependencies: pending terminology; C52.
- Closure owner: C55.
- Verification: route/component and Playwright test changes Questions and Properties separately then reloads.

#### A4-11.6

- HG bullet: "**Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see."
- Current evidence and concrete mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor; `AssignmentWorkspaceQuestionsPage` and `AssignmentWorkspacePoliciesPage` are Assignment-named, rather than separate Assessment Question and Properties editors.
- Owning source area: Course/Assessment workspace frontend: src/pages/course_instance_page.tsx and src/pages/assignment_workspace/.
- Dependencies: pending terminology; C52.
- Closure owner: C55.
- Verification: route/component and Playwright test changes Questions and Properties separately then reloads.

### A4-12 - Personal Question collections (Question frontend routes; 4)

#### A4-12.1

- HG bullet: "The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library."
- Current evidence and concrete mismatch: `myQuestions`, `starred`, and `watched` remain `future` in `src/ribbon/ribbon_catalog.ts`; no collection views exist.
- Owning source area: Question collection frontend: src/ribbon/ribbon_catalog.ts, src/pages/question_drafts_page.tsx, src/pages/library_page.tsx.
- Dependencies: C8, C9, and C11.
- Closure owner: C56.
- Verification: route/query tests and zero/nonzero collection Playwright fixtures.

#### A4-12.2

- HG bullet: "**My Questions** should make the Instructor's Published Questions easy to find and manage."
- Current evidence and concrete mismatch: `myQuestions`, `starred`, and `watched` remain `future` in `src/ribbon/ribbon_catalog.ts`; no collection views exist.
- Owning source area: Question collection frontend: src/ribbon/ribbon_catalog.ts, src/pages/question_drafts_page.tsx, src/pages/library_page.tsx.
- Dependencies: C8, C9, and C11.
- Closure owner: C56.
- Verification: route/query tests and zero/nonzero collection Playwright fixtures.

#### A4-12.3

- HG bullet: "**Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy."
- Current evidence and concrete mismatch: `myQuestions`, `starred`, and `watched` remain `future` in `src/ribbon/ribbon_catalog.ts`; no collection views exist.
- Owning source area: Question collection frontend: src/ribbon/ribbon_catalog.ts, src/pages/question_drafts_page.tsx, src/pages/library_page.tsx.
- Dependencies: C8, C9, and C11.
- Closure owner: C56.
- Verification: route/query tests and zero/nonzero collection Playwright fixtures.

#### A4-12.4

- HG bullet: "**Watched** should help Instructors follow Questions where changes or activity matter to them."
- Current evidence and concrete mismatch: `myQuestions`, `starred`, and `watched` remain `future` in `src/ribbon/ribbon_catalog.ts`; no collection views exist.
- Owning source area: Question collection frontend: src/ribbon/ribbon_catalog.ts, src/pages/question_drafts_page.tsx, src/pages/library_page.tsx.
- Dependencies: C8, C9, and C11.
- Closure owner: C56.
- Verification: route/query tests and zero/nonzero collection Playwright fixtures.

### A4-13 - Search landing and return state (Question Library frontend state; 3)

#### A4-13.1

- HG bullet: "Search should begin with a prominent search box, similar to Google Search."
- Current evidence: `src/pages/library_page.tsx` `LibraryPage` starts with its single Search entry and suppresses filters/results until input; accepted one-time compiled-browser evidence confirmed idle no-fetch and the rendered landing.
- Owning source area: Question Library search frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C57.
- Closure receipt: accepted one-time compiled-browser check and landing screenshot; its temporary harness remains outside the repository during review. C58 and C59 remain open.

#### A4-13.2

- HG bullet: "The initial Search page should stay simple and focus attention on entering a search."
- Current evidence: `src/pages/library_page.tsx` `LibraryPage` keeps its initial surface to the Search entry, then renders filters/results after query input; accepted one-time compiled-browser evidence confirmed the transition.
- Owning source area: Question Library search frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C57.
- Closure receipt: accepted one-time compiled-browser check and results screenshot; its temporary harness remains outside the repository during review. C58 and C59 remain open.

#### A4-13.3

- HG bullet: "Opening a result and returning should preserve the Instructor's search and position."
- Current evidence: `src/pages/library_page_model.ts` `saveQuestionLibraryReturnState` and `takeQuestionLibraryReturnState` preserve one session-bound in-document return snapshot, while `src/pages/library_page.tsx` `LibraryPage` restores it.
- Owning source area: Question Library search frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C57.
- Closure receipt: accepted one-time compiled-browser check restored query, filters, 80 rows, and virtual-list position through visible return and browser Back; changed-session isolation also passed. Its temporary harness remains outside the repository during review. C58 and C59 remain open.

### A4-14 - Advanced Question Library grammar (A7 query parser/API; 7)

The shared C58 receipt combines accepted source/parser coverage with a one-time private PostgreSQL
17 and actual-server HTTP proof. The connected production Store and route accepted ordinary AND
words, quoted phrases, minus exclusion, and all five PLE fields under an active vetted Instructor;
anonymous and Student requests remained concealed and every response was `no-store`. The bounded
temporary proof remains outside Git through documentation acceptance.

#### A4-14.1

- HG bullet: "Search should support Google-like syntax for more precise queries."
- Current evidence: `QuestionTextQuery::parse` handles ordinary words and exact-ID-plus-filter matching; see the shared C58 receipt above.
- Owning source area: Question Library server query boundary: crates/server/src/question_library.rs.
- Dependencies: none.
- Closure owner: C58.
- Closure receipt: accepted C58 source/parser evidence and actual-server HTTP projection described in the shared receipt.

#### A4-14.2

- HG bullet: "Quoted text should search for an exact phrase."
- Current evidence: `term_value` retains quoted phrases as one term; see the shared C58 receipt above.
- Owning source area: Question Library server query boundary: crates/server/src/question_library.rs.
- Dependencies: none.
- Closure owner: C58.
- Closure receipt: accepted C58 source/parser evidence and actual-server HTTP projection described in the shared receipt.

#### A4-14.3

- HG bullet: "A minus sign should exclude matching terms."
- Current evidence: `exclusion_prefix` records a leading minus exclusion; see the shared C58 receipt above.
- Owning source area: Question Library server query boundary: crates/server/src/question_library.rs.
- Dependencies: none.
- Closure owner: C58.
- Closure receipt: accepted C58 source/parser evidence and actual-server HTTP projection described in the shared receipt.

#### A4-14.4

- HG bullet: "Search should support PubMed-like field tags such as `topic:genetics`."
- Current evidence: `field_prefix` recognizes `topic`; see the shared C58 receipt above.
- Owning source area: Question Library server query boundary: crates/server/src/question_library.rs.
- Dependencies: none.
- Closure owner: C58.
- Closure receipt: accepted C58 source/parser evidence and actual-server HTTP projection described in the shared receipt.

#### A4-14.5

- HG bullet: "Field tags should use PLE concepts and vocabulary."
- Current evidence: `field_prefix` limits tags to PLE vocabulary; see the shared C58 receipt above.
- Owning source area: Question Library server query boundary: crates/server/src/question_library.rs.
- Dependencies: none.
- Closure owner: C58.
- Closure receipt: accepted C58 source/parser evidence and actual-server HTTP projection described in the shared receipt.

#### A4-14.6

- HG bullet: "Useful fields may include subject, topic, tags, Question Type, and author."
- Current evidence: `SearchField` supplies subject, topic, tags, type, and author; see the shared C58 receipt above.
- Owning source area: Question Library server query boundary: crates/server/src/question_library.rs.
- Dependencies: none.
- Closure owner: C58.
- Closure receipt: accepted C58 source/parser evidence and actual-server HTTP projection described in the shared receipt.

#### A4-14.7

- HG bullet: "Simple and advanced searches should use the same search box."
- Current evidence: `LibraryPage` has one visible Search input and C59's accepted component proof confirms normal-flow tips; `QuestionTextQuery::parse` receives that one text value through the connected C58 HTTP/API projection.
- Owning source area: Question Library server query boundary: crates/server/src/question_library.rs.
- Dependencies: none.
- Closure owner: C58.
- Closure receipt: accepted C58 source/parser evidence and actual-server HTTP projection described in the shared receipt.

### A4-15 - Advanced-query discovery (Question Library frontend help; 2)

#### A4-15.1

- HG bullet: "The interface should make useful search syntax discoverable when needed."
- Current evidence: `src/pages/library_page.tsx` `question-library-search-tips` provides a native disclosure beside ordinary Search with words, quotes, minus, PLE fields, and examples. Accepted corrected desktop component proof showed it without obscuring filters or bulk controls; `./check_codebase.sh` passed.
- Owning source area: Question Library search-help frontend: src/pages/library_page.tsx.
- Dependencies: C58.
- Closure owner: C59.
- Verification: Playwright checks discoverable help, ordinary search unchanged, and seeded expert narrowing.

#### A4-15.2

- HG bullet: "Search syntax should help expert users quickly narrow a very large Question Library."
- Current evidence and verification pending: Search tips provide the grammar and C58 now has connected actual-server projection evidence across a bounded 69-Question fixture, but this does not establish usability or performance for a very large production Question Library.
- Owning source area: Question Library search-help frontend: src/pages/library_page.tsx.
- Dependencies: C58.
- Closure owner: C59.
- Verification: Playwright checks discoverable help, ordinary search unchanged, and seeded expert narrowing.

### A4-16 - Browse Question Library (Question frontend Browse route/state; 8)

#### A4-16.1

- HG bullet: "**Browse Question Library** helps Instructors explore Questions without knowing what to search for."
- Current evidence: `libraryBrowse` is a distinct overview-first route with grouped subject-to-topic navigation and exact Search handoff.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: accepted private full-app browser evidence bundled exact `src/main.tsx`, used an actual authenticated server session, and exercised overview-first Browse through Biology, Enzymes, and focused Search with `no-store` actual search responses.

#### A4-16.2

- HG bullet: "Browse should help Instructors understand what the Question Library contains."
- Current evidence: `LibraryPage` explains the full-snapshot count scope and presents subject, topic, tag, and Question Type groups. Accepted component evidence rendered the overview, and actual-server HTTP evidence returned the authorized matched groups.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: accepted C60 component and actual-server grouping evidence; production layout and the broad connected workflow are not claimed.

#### A4-16.3

- HG bullet: "Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings."
- Current evidence: `LibraryPage` renders dedicated Subjects, Topics, Tags, and Question Types groups with explicit free-text truncation notices; actual-server evidence covered their bounded truthful projection.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: accepted C60 component and actual-server grouping evidence; production layout and the broad connected workflow are not claimed.

#### A4-16.4

- HG bullet: "Browse should make moving from broad subjects to narrower topics easy."
- Current evidence: selecting a subject clears the prior topic, applies exact filtering, and presents topics for that subject. Accepted routed component evidence selected Biochemistry then Enzymes and preserved both exact filters.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: accepted C60 routed component hierarchy evidence.

#### A4-16.5

- HG bullet: "Browse should show useful counts where they help Instructors choose where to explore."
- Current evidence: group choices render server-owned counts and disclose that they cover all authorized matches. Actual-server HTTP evidence used `page_size=1` while returning Biology topic counts over all three matching Questions.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: accepted C60 actual-server full-snapshot count evidence.

#### A4-16.6

- HG bullet: "Browse results should use the same dense Question presentation used by Search where practical."
- Current evidence: Search and Browse share the virtualized Question-row renderer.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: accepted private full-app browser evidence rendered the production-styled Browse path at 1280 by 800 with two shared dense Question rows; root manager visual review accepted the initial Browse, narrowed Browse, and Search handoff screenshots.

#### A4-16.7

- HG bullet: "Instructors should be able to move from browsing into a more focused search."
- Current evidence: `Search within results` serializes exact Browse filters into Search. Accepted routed component evidence preserved Biochemistry and Enzymes visibly and in repository requests through text editing and detail return even when selected facets disappeared from responses.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: accepted C60 routed component handoff and retained-filter evidence.

#### A4-16.8

- HG bullet: "Search and Browse are different paths into the same **Question Library**."
- Current evidence: `library` and `libraryBrowse` are distinct Instructor routes backed by the same repository and Question-row implementation; the Ribbon Browse task targets `libraryBrowse`.
- Owning source area: Question Library Browse frontend: src/pages/library_page.tsx and src/pages/library_page_model.ts.
- Dependencies: none.
- Closure owner: C60.
- Closure receipt: focused route, Ribbon, screenshot-manifest, and picker checks passed with the distinct Browse route.

### A4-17 - Assessments navigation/list/Templates (Assessment frontend routes; 6)

#### A4-17.1

- HG bullet: "The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates."
- Current evidence: `src/ribbon/ribbon_catalog.ts` admits `assessmentsDueSoon` and `assessmentTemplates` with the required labels. Accepted independent actual-Ribbon proof covered `AppRibbon`, `deriveRibbonModel`, and both routes.
- Owning source area: Assessment navigation frontend: src/ribbon/ribbon_catalog.ts and src/pages/assessment_templates_page.tsx.
- Dependencies: none for this label-and-route row.
- Closure owner: C61.
- Verification: route/list tests and Playwright fixtures covering upcoming, release, due, Course, and template records.

#### A4-17.2

- HG bullet: "**Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention."
- Current evidence: `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` states its Instructor across-Courses deadline purpose.
- Owning source area: Assessment navigation frontend: src/pages/assessments_due_soon_page.tsx.
- Dependencies: none.
- Closure owner: C61.
- Closure receipt: accepted private full-app actual-HTTP evidence visibly emphasized the seven-day Account-zone window, release state, and Due group for each upcoming Assessment.

#### A4-17.3

- HG bullet: "Assessment lists should make Course, release status, due date, and other important state easy to scan."
- Closure evidence: accepted private actual-server and exact-main browser proof rendered Due Soon with per-row Course, release state, and Account-zone Due. The first owned Course's three Assessment rows remained readable at 1280px and 720px in a Los Angeles browser with a Chicago Account zone; a second owned Course rendered three rows with mixed Released/Unreleased states and distinct Due values matched row-by-row to its actual 200/`no-store` Assessment-list response. The Course heading qualified every row. Root manager visual review and independent artifact review accepted the bounded scan behavior; fixture Released state was privileged projection setup, not release-workflow proof.
- Owning source area: Assessment navigation frontend: src/pages/assessments_due_soon_page.tsx and src/pages/course_instance_page.tsx.
- Dependencies: none for this bounded scan row.
- Closure owner: C61.
- Verification receipt: accepted actual-HTTP and browser proof with readable Due and release states across Due Soon and two owned Course Assessment lists.

#### A4-17.4

- HG bullet: "**My Assessment Templates** should emphasize reusable Assessment design rather than Course activity."
- Current evidence: `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` foregrounds reusable settings. Accepted independent actual-page proof verified its heading, lede, legend, and Ribbon route; it claims no HTTP, CRUD, or copy workflow acceptance.
- Owning source area: Assessment Template frontend: src/pages/assessment_templates_page.tsx.
- Dependencies: none for this reusable-design row.
- Closure owner: C61.
- Verification: route/list tests and Playwright fixtures covering upcoming, release, due, Course, and template records.

#### A4-17.5

- HG bullet: "**Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches."
- Current evidence: `src/pages/assessments_due_soon_page.tsx` calls `listAssessmentsDueSoon`.
- Owning source area: Assessment navigation frontend: src/pages/assessments_due_soon_page.tsx.
- Dependencies: none.
- Closure owner: C61.
- Closure receipt: accepted private actual-server evidence covered two owned Courses and one outsider Course; each Instructor saw only their own rows, and anonymous and Student responses were identically concealed.

#### A4-17.6

- HG bullet: "Assessments Due Soon shows the Course and due time for each Assessment."
- Current evidence: `src/pages/assessments_due_soon_page.tsx` `DueSoonAssessmentRow` renders Course and formatted due time.
- Owning source area: Assessment navigation frontend: src/pages/assessments_due_soon_page.tsx.
- Dependencies: none.
- Closure owner: C61.
- Closure receipt: accepted private full-app evidence showed both Course names and Due values, with visible values matched to actual HTTP instants formatted in the returned Account time zone.

### A4-18 - Assessment editor shell and distinct tasks (Assessment frontend workspace; 5)

#### A4-18.1

- HG bullet: "Assessment editing has two editors:"
- Current evidence: `src/route_contract.ts`, `src/ribbon/ribbon_catalog.ts`, and `src/ribbon/ribbon_contract.ts` expose distinct Question and Properties routes, tasks, and breadcrumb language. Accepted private exact-main evidence navigated between the two rendered editors.
- Owning source area: Assessment workspace frontend: src/pages/assessment_workspace/.
- Dependencies: none.
- Closure owner: C62.
- Closure receipt: accepted private actual-HTTP and exact-main browser evidence rendered and navigated between the two named editors; focused TypeScript and Ribbon checks passed.

#### A4-18.2

- HG bullet: "**Assessment Question Editor**: Selects, adds, removes, and orders Questions."
- Current evidence: `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` supplies Add, Move, Remove, and Save controls. Accepted private actual-HTTP and exact-main evidence exercised all four actions and reloaded the persisted two-Question order.
- Owning source area: Assessment workspace frontend: src/pages/assessment_workspace/.
- Dependencies: none.
- Closure owner: C62.
- Closure receipt: two exact Available Questions were added, moved, removed, re-added, saved, and reloaded in visible order against the actual server.

#### A4-18.3

- HG bullet: "**Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings."
- Current evidence: `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` provides the named Properties Editor for dates, instructions, Attempt/time limits, late work, order, disclosure, and focused fixed-Question point-value editing.
- Owning source area: Assessment workspace frontend: src/pages/assessment_workspace/.
- Dependencies: none.
- Closure owner: C62.
- Closure receipt: accepted private actual-HTTP and exact-main browser evidence saved and reloaded `2.5` and `1` for two ordered exact Questions without changing other Assessment fields; Cancel, Stay/Discard, a real stale-write refusal, retained draft, and explicit reload/discard recovery also passed.

#### A4-18.4

- HG bullet: "The two Assessment editors should remain clearly distinct."
- Current evidence: the two editor pages have separate routes, headings, Ribbon tasks, state models, and save operations. Accepted private actual-HTTP and exact-main evidence preserved Question order and Properties instructions through independent reloads.
- Owning source area: Assessment workspace frontend: src/pages/assessment_workspace/.
- Dependencies: none.
- Closure owner: C62.
- Closure receipt: independent source review and accepted browser evidence confirmed visibly distinct Question and Properties tasks without compatibility labels.

#### A4-18.5

- HG bullet: "Assessment Properties should group related settings so important settings are easy to find."
- Current evidence: `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` groups Assessment and delivery separately from Student feedback. Accepted private exact-main evidence and computed styles verified restored panel/control styling, two desktop columns, and one column at 720px.
- Owning source area: Assessment workspace frontend: src/pages/assessment_workspace/.
- Dependencies: none.
- Closure owner: C62.
- Closure receipt: accepted production-styled browser evidence showed the named groups and responsive owning layout; one instructions edit survived actual HTTP and reload.

### A4-19 - Assessment Question Editor interaction (Assessment frontend Question editor; 3)

#### A4-19.1

- HG bullet: "The Assessment Question Editor should make Question order easy to understand at a glance."
- Current evidence: `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` renders one numbered ordered list with adjacent Move and Remove controls. Accepted private actual-HTTP and exact-main browser evidence changed two visible Questions' order and reloaded the persisted result; root manager visual review and independent review accepted the rendered order.
- Owning source area: Assessment Question Editor frontend: src/pages/assessment_workspace/assessment_workspace_questions_page.tsx.
- Dependencies: none.
- Closure owner: C63.
- Closure receipt: accepted actual-server and production-styled browser evidence verified visible order, reorder, save, and reload without claiming the still-missing Search/Browse or inspect-before-add actions.

#### A4-19.2

- HG bullet: "Adding Questions should provide direct paths to Search and Browse Question Library."
- Current evidence: `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` now renders direct Search and Browse Question Library links inside Available published Questions.
- Owning source area: Assessment Question Editor frontend: src/pages/assessment_workspace/assessment_workspace_questions_page.tsx.
- Dependencies: none.
- Closure owner: C63.
- Closure receipt: accepted private actual-HTTP and exact-main browser evidence followed Search and Browse, returned through browser Back, preserved a dirty title on Stay, and navigated only after deliberate Discard.

#### A4-19.3

- HG bullet: "Instructors should be able to inspect a Question before adding it to an Assessment."
- Current evidence and concrete mismatch: `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` links each candidate's exact Revision through `getQuestionRevisionDetails`, with strict identity checks and the shared unsaved-changes guard. The authorized private source and checksum now reach the existing opaque WeBWorK adapter and hardened iframe. Private PostgreSQL 17/MinIO plus unchanged-renderer HTTP evidence returned preview 200 with hardened headers and concealed missing Revision, Student, and anonymous requests; exact-main browser evidence visibly rendered the prompt and five choices. This isolated preview path left all five Student Work counts at zero before and after: Assessment Attempts, Question Attempts, saved responses, submissions, and grading results. The renderer JavaScript then dereferences `window.frameElement.id` when the hardened sandbox has no same-origin frame element, preventing focus, popover, and parent telemetry; do not loosen the sandbox or rewrite sibling renderer HTML.
- Owning source area: Assessment Question Editor frontend: src/pages/assessment_workspace/assessment_workspace_questions_page.tsx.
- Dependencies: C62, C60.
- Closure owner: C63.
- Verification: retain the hardened embed boundary, obtain successful renderer behavior beyond the observed JavaScript error, and return without losing Assessment state.

### A4-20 - Assessment randomization semantics (A9 policy/API plus Properties binding; 2)

#### A4-20.1

- HG bullet: "Instructors can randomize Question order for an Assessment."
- Current evidence: `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `updateOrder` binds the current Assessment Properties checkbox to authored or shuffled Question order. Accepted actual-HTTP evidence persisted authored and shuffled rules, issued the complete fixed-and-Pool vector, retained immutable shuffled order on resume, and concealed unauthorized starts. Accepted exact-main browser evidence saved and reloaded the checkbox through actual HTTP.
- Owning source area: `schemas/base_schema/50_functions/assessment_attempt_operations.sql`, `crates/learning-data-access/src/postgres/assessment_delivery_start.rs`, and `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx`.
- Dependencies: none.
- Closure owner: C65.
- Closure receipt: focused compilation, private PostgreSQL 17 decision proof, actual authenticated Student HTTP start/resume evidence, exact-main browser save/reload evidence, and independent review passed.

#### A4-20.2

- HG bullet: "Answer-choice randomization belongs to the Question, not the Assessment."
- Current evidence: PLE Question authoring and its strict codec own `randomizeChoices`; the PLE source adapter compiles it to `NativeChoiceOrder`, and the Question presentation builder applies the nonce-derived choice permutation. The closed Assessment activity rules contain no answer-choice override. Accepted exact-main browser evidence preserved this explicit ownership copy while independently saving and reloading Assessment Question order.
- Owning source area: `src/features/ple_question_json_authoring`, `crates/adapters/ple/src/question_json/source_document.rs`, `crates/question_model/src/presentation/builder.rs`, and `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx`.
- Dependencies: none.
- Closure owner: C65.
- Closure receipt: focused Question-source and nonce-reproduction tests plus accepted Assessment-order actual-HTTP and browser evidence establish the ownership boundary. This does not claim a runtime matrix of every native choice permutation.

### A4-21 - Assessment Unrelease danger transaction (A9 release API/confirmation UI; 2)

#### A4-21.1

- HG bullet: "Assessment Unrelease should explain that Student work will be deleted."
- Current evidence and concrete mismatch: current Danger Zone says "Unrelease assignment"; confirmation and deletion explanation are Assignment-named, not the required Assessment behavior.
- Owning source area: Assessment release server/UI: crates/server/src/assignment_release.rs and src/pages/assignment_workspace/assignment_workspace_policies_page.tsx.
- Dependencies: C12, C62, C66, and pending A9 Student Work deletion contract.
- Closure owner: C67.
- Verification: seeded submitted-work transaction test proves exact-title confirmation and deletion only after confirmation; browser copy/action check.

#### A4-21.2

- HG bullet: "Assessment Unrelease should require typing the Assessment title before confirmation."
- Current evidence and concrete mismatch: current Danger Zone says "Unrelease assignment"; confirmation and deletion explanation are Assignment-named, not the required Assessment behavior.
- Owning source area: Assessment release server/UI: crates/server/src/assignment_release.rs and src/pages/assignment_workspace/assignment_workspace_policies_page.tsx.
- Dependencies: C12, C62, C66, and pending A9 Student Work deletion contract.
- Closure owner: C67.
- Verification: seeded submitted-work transaction test proves exact-title confirmation and deletion only after confirmation; browser copy/action check.

### A4-22 - Archive confirmation parity (Published Question/Blueprint frontend confirmations; 2)

#### A4-22.1

- HG bullet: "Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**."
- Current evidence and concrete mismatch: `blueprint_course_workspace.tsx` covers only Blueprint archive; Published Question archive and all-action shared-availability explanation/confirmation are unproved.
- Owning source area: shared high-consequence action frontend: new src/features/high_consequence_actions/availability_archive_confirmation.tsx.
- Dependencies: pending A7 Published Question archive semantics; C50.
- Closure owner: C68.
- Verification: browser tests for both archive actions verify effect text, explicit confirmation, and refusal before confirmation.

#### A4-22.2

- HG bullet: "Archive actions should explain the effect on shared availability and require a clear confirmation."
- Current evidence and concrete mismatch: `blueprint_course_workspace.tsx` covers only Blueprint archive; Published Question archive and all-action shared-availability explanation/confirmation are unproved.
- Owning source area: shared high-consequence action frontend: new src/features/high_consequence_actions/availability_archive_confirmation.tsx.
- Dependencies: pending A7 Published Question archive semantics; C50.
- Closure owner: C68.
- Verification: browser tests for both archive actions verify effect text, explicit confirmation, and refusal before confirmation.

### A4 closure and contribution crosswalk

| A4 record | Owning bullets | Closure owner | Contributor |
| --------- | -------------: | ------------- | ----------- |
| A4-01     |              7 | C44           | -           |
| A4-02     |              1 | C45           | C74         |
| A4-03     |              5 | C46           | -           |
| A4-04     |              2 | C47           | -           |
| A4-05     |              5 | C48           | -           |
| A4-06     |              8 | C50           | C49, C72    |
| A4-07     |              1 | C51           | -           |
| A4-08     |              2 | C52           | C73         |
| A4-09     |              1 | C53           | -           |
| A4-10     |              5 | C54           | -           |
| A4-11     |              6 | C55           | -           |
| A4-12     |              4 | C56           | -           |
| A4-13     |              3 | C57           | -           |
| A4-14     |              7 | C58           | -           |
| A4-15     |              2 | C59           | -           |
| A4-16     |              8 | C60           | -           |
| A4-17     |              6 | C61           | -           |
| A4-18     |              5 | C62           | -           |
| A4-19     |              3 | C63           | -           |
| A4-20     |              2 | C65           | C64         |
| A4-21     |              2 | C67           | C66         |
| A4-22     |              2 | C68           | -           |
| **Total** |         **87** |               |             |

C68 owns the combined Danger Zone bullet because that behavior necessarily includes both archive
actions; C67 owns the two Unrelease-specific bullets only.

## A5: Student and Sysadmin interface

The A5 audit has 27 raw accounted entries: 26 `[ ]` entries and one N/A. Of the `[ ]` entries,
24 are owning product-behavior findings and exactly two are HG-unlocked complete Ribbon-layout
details. A5-S04 is the sole N/A: a permission (`may provide filters`), not a required current
behavior. A5-Y10 is the sole owning product question. C75, C80, C81, C83, C85, and C88
contribute prerequisites but flip no A5 bullet.

### A5-S01

- HG bullet: "**Coursework** is the Student-facing collective term for Regular Assignments, Practice Question Assignments, Bonus Assignments, Quizzes, and Exams."
- Current evidence and concrete mismatch: `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` says `Assignments`; `LiveStudentAssignmentLandingSummary` has no Type field.
- Owning source area: Student landing frontend. Dependencies: C75 and A1/A9 terminology/type contracts. Closure owner: C76.
- Verification: seeded browser fixture asserts the Coursework heading and each supported Type name.

### A5-S02

- HG bullet: "Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment."
- Current evidence and concrete mismatch: `AssignmentCard` shows title/progress only and its DTO has no Type.
- Owning source area: Student landing frontend. Dependencies: C75 and A9 Type contract. Closure owner: C76.
- Verification: decoder and browser fixture assert a specific Type label and reject generic Assessment copy.

### A5-S03

- HG bullet: "The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment."
- Current evidence and concrete mismatch: `src/ribbon/ribbon_catalog.ts` `studentAssignments` is labelled `Assignments`.
- Owning source area: Student Ribbon. Dependencies: C76. Closure owner: C79.
- Verification: catalog plus Student browser navigation accessible-name assertion.

### A5-S04

- HG bullet: "Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**, **Bonus Assignments**, **Quizzes**, and **Exams**."
- Current evidence and classification: no type filter exists, but `may` grants permission rather than requiring an implementation.
- Owning source area: Human Guidance audit classification. Dependencies: none. Closure owner: audit classification correction.
- Verification: reclassify N/A with `Reason: permission, not a required current PLE behavior`; ensure no UI promises filters that do not exist.

### A5-S05

- HG bullet: "Each Coursework item should clearly show its Assessment Type using its label and Type icon."
- Current evidence and concrete mismatch: `AssignmentCard` has neither Type label nor icon.
- Owning source area: Student landing frontend. Dependencies: C75 and A9 icon/Type contract. Closure owner: C76.
- Verification: browser fixture asserts label and meaningful icon for each Type.

### A5-S06

- HG bullet: "The Student menu is simpler than the Instructor menu."
- Current evidence and concrete mismatch: no complete backed Student menu exists in `src/ribbon/ribbon_catalog.ts`.
- Owning source area: Student Ribbon. Dependencies: C76. Closure owner: C79.
- Verification: backed Student destinations cover active Courses and Coursework, expose no Instructor authoring/grading or Sysadmin task, and use learner language; do not use a control-count proxy.

### A5-S07

- HG bullet: "Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays."
- Current evidence and concrete mismatch: `tests/playwright/student_course_entry_m6_evidence.mjs` lacks a complete Student journey at all four required classes.
- Owning source area: Student browser journey. Dependencies: C76-C79; C80 and every concrete page-specific milestone created by C81. Closure owner: C82.
- Verification: named journey at `1280x800`, `768x1024`, `320x640`, and `768x768` has reachable next action/result and no horizontal page overflow.

### A5-S08

- HG bullet: "Every Student browser action should be usable with the keyboard alone."
- Current evidence and concrete mismatch: native controls exist but no complete keyboard-only journey is recorded.
- Owning source area: Student browser journey. Dependencies: C80 and every concrete page-specific milestone created by C81. Closure owner: C82.
- Verification: keyboard-only course-to-summary journey reaches/fires each task without pointer input; assert task result, not computed styles or handlers.

### A5-S09

- HG bullet: "Student navigation and pages should contain only Student interfaces and capabilities."
- Current evidence and concrete mismatch: `src/route_contract.ts` gates only `studentCourseLanding`; no complete server/API and browser role matrix exists.
- Owning source area: Student route admission. Dependencies: C83 and C24-C26 if discovery finds server failure. Closure owner: C84.
- Verification: Student, Instructor, Sysadmin, anonymous matrices independently cover every Student API/route and prove denial with no protected data.

### A5-S10

- HG bullet: "Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish."
- Current evidence and concrete mismatch: `progressLabel` has only completed, in-progress, and not-started.
- Owning source area: Student landing state presentation. Dependencies: C75 and A9 delivery/late-state contract. Closure owner: C77.
- Verification: deterministic date fixture exposes all four states with distinct visible labels.

### A5-S11

- HG bullet: "Coursework lists should make due dates, Type, and completion status easy to scan."
- Current evidence and concrete mismatch: `AssignmentCard` lacks due date and Type.
- Owning source area: Student landing state presentation. Dependencies: C75 and A9 Type/due semantics. Closure owner: C77.
- Verification: each card renders those three fields legibly at every S07 viewport.

### A5-S12

- HG bullet: "Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts."
- Current evidence and concrete mismatch: `AssessmentOverviewPage` and `StudentAssessmentStartFacts` expose all six fields. Accepted isolated actual-server/exact-main member proof showed a Released direct Practice Assessment's title, Type, one Question, one point, exact one-hour time limit, and explicit zero-previous-Attempts note before Start. No actual nonempty prior-Attempt history or Question rendering/Start was exercised.
- Owning source area: pre-start Student presentation. Dependencies: C75. Closure owner: C78.
- Verification: retain the accepted zero-prior actual-main proof; a later real completed-Attempt fixture must assert nonempty prior history before the start control. Do not mint fabricated grading history to close this row.

### A5-S13

- HG bullet: "The complete Student Ribbon task layout does not have a locked-in design yet."
- Classification: HG-unlocked detail, not an implementation gap. Reason: `HG: no locked-in design`.
- Verification: retain `[ ]` with that exact reason; C79 must not select or claim a complete layout.

### A5-S14

- HG bullet: "Course invitations should show the Course name and relevant Instructor and term information before the Student accepts the invitation."
- Current source and independent review: `StudentCourseInvitationsPage` and
  `StudentCourseInvitationPage` show Course name, verified assigned Instructor display name, and
  inclusive term before `Accept invitation`. `LiveStudentCourseInvitationSummary` is a closed,
  self-only projection. Loading, unavailable, missing, foreign, expired, and already-accepted
  states conceal invitation context. The independent review accepted the scoped source and
  actual-component browser evidence.
- Owning source area: Student invitation projection and presentation. Dependencies: actual-role
  connected PostgreSQL proof and authenticated canonical-browser acceptance. Closure owner: C91.
- Verification: actual-role PostgreSQL covers self versus foreign, expired, accepted, and
  already-enrolled invitations; canonical browser shows index/detail context before acceptance and
  no context in transient states. Keep the checklist row `[ ]` until both pass.

### A5-Y01

- HG bullet: "The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find."
- Current evidence and concrete mismatch: `src/ribbon/ribbon_catalog.ts` has only Instructor Accounts and Scoped Support; no Courses/settings destination.
- Owning source area: Sysadmin Ribbon. Dependencies: C87, C89, and A5-Y10 resolution. Closure owner: C90.
- Verification: Sysadmin navigation reaches Accounts/Instructors, Courses, and the separate settings area after the Y10 decision identifies its settings inventory.

### A5-Y02

- HG bullet: "Sysadmins should be able to find users quickly by name or email."
- Current evidence and concrete mismatch: `InstructorAccountsPage` and `src/api/instructor_account.ts` have no name/email search; email is deliberately omitted.
- Owning source area: Sysadmin account list frontend. Dependencies: C85/C86 and C24/C26 approved disclosure boundary. Closure owner: C86.
- Verification: role-gated server search and browser search find seeded rows by supported key without extra disclosure.

### A5-Y03

- HG bullet: "Account lists should support searching, filtering, and scanning large numbers of users."
- Current evidence and concrete mismatch: unbounded account list has no search, filter, or cursor contract.
- Owning source area: Sysadmin account list frontend. Dependencies: C85. Closure owner: C86.
- Verification: deterministic multipage fixture proves search, filter, pagination, result count, empty state.

### A5-Y04

- HG bullet: "User pages should clearly show role, account status, and other important administrative information."
- Current evidence and concrete mismatch: account rows show only reference/state/sign-in; no detail page exists.
- Owning source area: Sysadmin account detail frontend. Dependencies: C85-C86 and approved field inventory. Closure owner: C87.
- Verification: Sysadmin detail route shows permitted role/lifecycle/approval fields; non-Sysadmin access is denied.

### A5-Y05

- HG bullet: "Sysadmins approve Instructors before they receive Instructor capabilities."
- Current evidence and concrete mismatch: `createAccount` creates active Instructor accounts; `accounts.sql` has no vetting/approval state.
- Owning source area: Instructor account creation authorization. Dependencies: C17. Closure owner: C18.
- Verification: C18 service E2E proves missing/invalid/completed vetting outcomes and pre-approval capability denial.

### A5-Y06

- HG bullet: "Instructor approval status should be easy to find and change."
- Current evidence and concrete mismatch: lifecycle has active/deactivated/closed only; no approval status/action exists.
- Owning source area: Sysadmin account detail frontend. Dependencies: C17, C18, C85-C86. Closure owner: C87.
- Verification: browser detail changes C18-backed approval and list/search/detail reflect the result; server matrix proves transition.

### A5-Y07

- HG bullet: "Sysadmins should be able to find and inspect Courses across the installation."
- Current evidence and concrete mismatch: `SupportRosterPage` handles one capability-scoped roster, not installation Courses.
- Owning source area: Sysadmin Course frontend. Dependencies: C88 and A8 status projection. Closure owner: C89.
- Verification: Sysadmin finds/opens a seeded Course outside current Instructor scope with only permitted summary.

### A5-Y08

- HG bullet: "Course administration should show the Instructor and important Course status information."
- Current evidence and concrete mismatch: no Sysadmin Course status projection/page exists.
- Owning source area: Sysadmin Course frontend. Dependencies: C88 and A8 status contract. Closure owner: C89.
- Verification: Course detail shows assigned Instructor and defined status data using seeded dates.

### A5-Y09

- HG bullet: "Sysadmins should manage Courses through Sysadmin interfaces and capabilities."
- Current evidence and concrete mismatch: no Sysadmin Course-management route exists in `src/route_contract.ts`.
- Owning source area: Sysadmin Course frontend. Dependencies: C88 and A2 authorization model. Closure owner: C89.
- Verification: API/browser matrix proves permitted Sysadmin management and denies Instructor/Student use of Sysadmin capability.

### A5-Y10

- HG bullet: "System-wide settings should have their own area, separate from user and Course administration."
- Current evidence and mismatch: no settings inventory or source-of-truth setting boundary was found; `ribbon_catalog.ts` has no destination.
- Plausible readings: (1) the existing implemented installation-wide settings need a separate Sysadmin area; (2) no current setting belongs in a UI area, so creating an empty page would invent behavior HG does not define.
- Question: Which implemented installation-wide settings must Sysadmins view or change, and which source-of-truth boundary owns each?
- Closure owner: none until product decision. Verification after decision: one settings boundary and route satisfy the selected inventory; Y01 then depends on it.

### A5-Y11

- HG bullet: "High-consequence administrative actions should have a visually distinct area."
- Current evidence and concrete mismatch: `Deactivate Instructor Account` is an ordinary `quiet-action` with no high-consequence region.
- Owning source area: Sysadmin account detail frontend. Dependencies: C85-C86. Closure owner: C87.
- Verification: browser accessibility evidence finds labelled high-consequence region, visible warning, keyboard reachability; no CSS-value assertion.

### A5-Y12

- HG bullet: "Confirmation for destructive actions should clearly state what will happen."
- Current evidence and concrete mismatch: `deactivate` runs after reason entry without confirmation.
- Owning source area: Sysadmin account detail frontend. Dependencies: C85-C86. Closure owner: C87.
- Verification: confirmation names target/consequence; Cancel keeps displayed state, Confirm reports resulting state; no exact-call-count assertion.

### A5-Y13

- HG bullet: "The complete Sysadmin Ribbon task layout does not have a locked-in design yet."
- Classification: HG-unlocked detail, not an implementation gap. Reason: `HG: no locked-in design`.
- Verification: retain `[ ]` with that exact reason; C90 must not select or claim a complete layout.

### A5 closure and classification crosswalk

| A5 record             | Status / closure owner                                             |
| --------------------- | ------------------------------------------------------------------ |
| A5-S01, S02, S05      | C76                                                                |
| A5-S03, S06           | C79                                                                |
| A5-S04                | N/A audit classification: permission, no required current behavior |
| A5-S07, S08           | C82                                                                |
| A5-S09                | C84                                                                |
| A5-S10, S11           | C77                                                                |
| A5-S12                | C78                                                                |
| A5-S13                | HG-unlocked, retain `[ ]` with exact reason                        |
| A5-S14                | C91                                                                |
| A5-Y01                | C90                                                                |
| A5-Y02, Y03           | C86                                                                |
| A5-Y04, Y06, Y11, Y12 | C87                                                                |
| A5-Y05                | C18 after C17                                                      |
| A5-Y07, Y08, Y09      | C89                                                                |
| A5-Y10                | Product question, no closure until decision                        |
| A5-Y13                | HG-unlocked, retain `[ ]` with exact reason                        |

Accounting: 27 raw entries = 26 `[ ]` plus one N/A. The 26 `[ ]` entries split into 24 owning
product-behavior records and two HG-unlocked Ribbon details (A5-S13, A5-Y13). Of the 24 owning
records, 23 have an implementation closure owner and A5-Y10 is the sole product question. A5-S04
is the sole N/A permission classification.

## A6 Data and history - canonicalized 2026-09-14

Identifier allocation: C91 and later are reserved for A5's dynamic page-specific dispatches. A6
uses the disjoint explicit range **C200-C216**; no earlier or reserved identifier is reused.

The A6 part has 37 unchecked records. Two are later duplicates, not closure owners: HG 434
`**Student** data should be collected reluctantly, used deliberately, and purged predictably.`
points to the Accounts occurrence at HG 117; the second HG 471 `Course metadata, Assessment
definitions, Questions, settings, and other teaching material remain after Student data is
deleted.` points to the first A6 occurrence at HG 439. The 35 owning records are accounted for
once below. The later Questions occurrence of `Privacy-safe aggregate Question statistics remain
after the underlying Student records are deleted.` points to A6 D11.

| Record  | Owning HG bullet / current mismatch                                                                                                                                                                                                         | Closure owner                 |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| D01     | Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**. One history projection and expiry test do not prove every delivery path.                                                         | C214                          |
| D02     | Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data. One aggregate/private boundary is incomplete.                                                                                        | C214                          |
| D03     | Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them. One `assignment_title` projection is not a surface audit.                                                                         | C216                          |
| D04     | FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage. `src/log.ts` leaves redaction later.                                                                                          | C201                          |
| D05     | FERPA access should be scoped through exact Course membership and **Student** ownership. Predicate exists but denial/runtime evidence is absent.                                                                                            | C203                          |
| D06-D07 | Closed by C208: fresh PostgreSQL 17 actual-role proof deleted a populated submitted Student Work graph at expiry while retaining the Account, Course, Assessment, Question, immutable source, settings, and an unrelated Course membership. | C208                          |
| D08-D09 | **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance; Student Work is an umbrella term; the underlying records retain their own identities and purposes.                           | C204                          |
| D10-D11 | Closed by C208: the populated retention proof removed identifiable Student evidence while retaining the existing anonymous Question Revision aggregate. Aggregate disclosure safety remains the distinct D12 product question.              | C208                          |
| D12     | Aggregate Question statistics must not identify or allow reconstruction of individual Student activity. No disclosure rule exists.                                                                                                          | Product question Q-A6-STAT-01 |
| D13-D14 | Course retention follows Course Instance dates/six-month Active lifetime; latest Assessment deadline starts the FERPA retention clock.                                                                                                      | C206                          |
| D15     | Creating or extending a later Assessment deadline may move dates, but not beyond the six-month Active lifetime.                                                                                                                             | C207                          |
| D16-D17 | Starting the FERPA retention clock does not itself notify/archive/hide/delete; configured FERPA policy determines later transitions.                                                                                                        | C206                          |
| D18     | PLE warns Instructors before Inactive six months after creation.                                                                                                                                                                            | C209                          |
| D19     | Active limit prevents indefinite delay.                                                                                                                                                                                                     | C207                          |
| D20     | Course inactivity and FERPA deletion are separate transitions.                                                                                                                                                                              | C206                          |
| D21-D22 | Retention works for all academic calendars; Instructor is notified before archive.                                                                                                                                                          | C209                          |
| D23     | Archived Student data leaves normal interfaces but remains recoverable during retention.                                                                                                                                                    | C210                          |
| D24     | Closed by C208: premature deletion was refused, expiry deletion succeeded, repeat deletion returned false, and concurrent executors serialized to one transition.                                                                           | C208                          |
| D25     | FERPA retention intervals are operational configuration.                                                                                                                                                                                    | C206                          |
| D26-D30 | Worker finds passed deadlines, executes rather than defines policy, is late-run equivalent and repeat-safe.                                                                                                                                 | C209                          |
| D31     | Be conservative about creating revisions.                                                                                                                                                                                                   | C211                          |
| D32-D34 | Published Questions, Question Pools, and Blueprint Courses have immutable revisions; Revision Numbers start at 1/increase; a Revision Number identifies stored immutable Revision. Pool identity absent.                                    | C212                          |
| D35     | Student Work records exact Question Pool Revision and selected Published Question Revision.                                                                                                                                                 | C213                          |

### C205 / Q-A6-STAT-01

HG, `DATA_CLASSIFICATION.md`, `RETENTION_POLICY.md`, and `DESIGN_DECISIONS.md` require sufficient
disclosure protection and warn that small cells remain protected, but none supplies a numeric
minimum cohort, an intersection/composition rule, or a decision whether that rule is operational
configuration. Exact product question: **What shared-statistics release rule, including minimum
cohort and intersection behavior, makes a slice not reasonably identifying for PLE?** D12 remains
`[ ]` with `Reason: product decision still unclear` and this question. It is independent of the
physical aggregate-preservation behavior in C208, so it creates no retention cycle.

### A6 milestone dependency DAG

`C200 -> C214 -> C201`; C808 audits existing Course-core facts consumed by C206;
`{C206, C809} -> C207`; `C206 -> C208`; `C208 -> {C215, C210}`;
`C206 -> C847 -> {C848, C849} -> C850`;
`{C215, C848, C849, C850} -> C851 -> C209`; `A7 published-Pool lineage -> C212 -> C213 -> C204`;
`A7 backend-delivery evidence -> C200`;
`C202 -> C216`; C30, C46, C55, C76-C78, C86-C87, C89, and the explicit A8/A9 title-reference
inventory placeholders feed C216. C203 is the permanent real-session authorization oracle and
is independent. A7 prerequisites are specific: its backend delivery/server-owned grading bullets
feed C200; its published-Pool/public-ID/revision bullets feed C212; its new-Attempt fresh
selection bullet feeds C213; its Question-statistics disclosure bullets are inputs to
Q-A6-STAT-01.

### C200-C216 contracts

All new implementation proof begins in ignored `tests/_temp/hg_a6_*`; closeout removes it unless
it passes the `docs/PYTEST_STYLE.md` permanent-test checklist. C203 is the explicit exception: it
is the accepted permanent behavior-level BOLA/FERPA oracle. The other durable candidates are the
archive/delete preservation invariant (C208) and immutable Pool-revision pin (C213). A failure
leaves its listed bullets open and repairs the named boundary; it never establishes a new product
policy.

| Contract | Boundary and expected behavior                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Focused verification                                                                                                                                                                                                                                                                                                                                               |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C200     | `crates/browser-api-contract/src/student_assignment_decision.rs` only. Define the allowlisted Student response contract; it flips no bullet until C214 converts every server delivery path.                                                                                                                                                                                                                                                                                                                                | `source source_me.sh && python3 tests/_temp/hg_a6_c200_contract_probe.py`                                                                                                                                                                                                                                                                                          |
| C214     | `crates/server/src/assignment_delivery/` conversion only. Enforce C200 for native and WeBWorK delivery; closes D01-D02.                                                                                                                                                                                                                                                                                                                                                                                                    | `source source_me.sh && python3 tests/_temp/hg_a6_c214_delivery_probe.py`                                                                                                                                                                                                                                                                                          |
| C201     | `src/log.ts`. Redact/reject classified Student Work values and eliminate facade bypasses; do not create analytics/URL/storage product behavior.                                                                                                                                                                                                                                                                                                                                                                            | `node --test tests/_temp/hg_a6_c201_log_probe.mjs`; `npx playwright test tests/_temp/hg_a6_c201_browser.spec.ts`                                                                                                                                                                                                                                                   |
| C202     | Contributor only: `src/components/copyable_question_id.tsx`. Extend the discovered Question recognition/copy control so it shows title plus reference rather than opaque ID alone; it does not close global D03.                                                                                                                                                                                                                                                                                                           | `node --test tests/_temp/hg_a6_c202_labels.mjs`; `npx playwright test tests/_temp/hg_a6_c202_labels.spec.ts`                                                                                                                                                                                                                                                       |
| C216     | D03 closure. One temporary frontend conformance inventory consumes the authoritative route manifests `src/routes.ts` and `src/route_contract.ts` plus the checked-in candidate set `src/components/copyable_question_id.tsx`, `src/pages/assignment_editor_content_list.tsx`, `src/pages/assignment_editor_model.ts`, `src/pages/library_page.tsx`, `src/pages/library_page_model.ts`, and `src/pages/question_detail_page.tsx`, regenerated by `rg -l "CopyableQuestionId                                                 | reference_number                                                                                                                                                                                                                                                                                                                                                   | assignment_title | course_instance_id | question_id" src/routes.ts src/route_contract.ts src/pages src/components`. C202, C30, C46, C55, C76-C78, C86-C87, C89, A8-PENDING-title-reference-inventory, and A9-PENDING-title-reference-inventory must each be closed/resolved before dispatch. The ignored inventory fails on an unreviewed route/candidate or an opaque identifier where a person must recognize/copy/enter it; it is removed at closeout and never becomes a permanent DOM/file-inventory test. | `node tests/_temp/hg_a6_c216_human_identity_inventory.mjs --routes src/routes.ts --contract src/route_contract.ts` |
| C203     | `schemas/base_schema/50_functions/authorization.sql`; permanent `crates/learning-data-access/tests/assignment_access_postgres.rs` real-application-session BOLA/FERPA oracle proves owner allow and nonmember, same-Course other Student, and same-Account other-Course denial. This is stable, high-impact, plausibly regressive, and asserts outcomes rather than schema/call structure.                                                                                                                                              | Existing `bash tests/e2e/e2e_database_baseline.sh`. On failure keep D05 open and triage session, exact membership predicate, then ownership boundary.                                                                                                                                                                                                              |
| C204     | `crates/question_model/src/student_work.rs`; named Student Work aggregate composes distinct Attempt/response/issuance/submission/grading/receipt identities.                                                                                                                                                                                                                                                                                                                                                               | `source source_me.sh && python3 tests/_temp/hg_a6_c204_probe.py`                                                                                                                                                                                                                                                                                                   |
| C206     | `schemas/base_schema/50_functions/course_retention.sql`; consume C808's existing `course_core.sql` creation/lifecycle facts and add only operational policy schedule and due actions: `warn_inactive`, `notify_archive`, `archive`, and `delete`. It owns no duplicate Course-core fields or triggers and no generic recovery state/queue/snapshot service.                                                                                                                                                                             | `source source_me.sh && python3 tests/_temp/hg_a6_c206_schedule_probe.py`                                                                                                                                                                                                                                                                                          |
| C207     | `schemas/base_schema/assignments.sql`; after C206 and C809, Assignment save/release synchronizes latest due and atomically rejects a due date beyond the current `active_until_at`.                                                                                                                                                                                                                                                                                                                                        | Expanded `source source_me.sh && python3 tests/_temp/hg_a6_c207_deadline_probe.py` with deterministic concurrency; retain only if the behavior earns permanent status.                                                                                                                                                                                             |
| C208     | Complete. `course_retention.sql` archive/delete procedures retain protected Course records through archive and delete identifiable submitted Student Work at expiry while preserving Account, Course metadata, definitions, Questions, settings, unrelated Course membership, and existing aggregate rows. Fresh PostgreSQL 17 actual-role proof also covered premature, repeated, unauthorized, and concurrent deletion outcomes. The large schema-shaped fixture remains one-time evidence rather than a permanent test. | Accepted 2026-09-16 disposable actual-role receipt; `source source_me.sh && cargo check -p learning-data-access --no-default-features --features postgres --tests` passed.                                                                                                                                                                                         |
| C215     | `crates/learning-data-access/src/retention.rs` only. Add the least-privilege store interface/adapter that returns stored due actions and commits their transitions; it flips no bullet.                                                                                                                                                                                                                                                                                                                                    | `source source_me.sh && python3 tests/_temp/hg_a6_c215_store_probe.py`                                                                                                                                                                                                                                                                                             |
| C209     | `crates/server/src/worker.rs` only. Consume C215 due actions, invoke C851's retention-notification boundary, archive/delete even when the provider fails, and never calculate dates. Closes D18, D21-D22, D26, D28-D30.                                                                                                                                                                                                                                                                                                    | `source source_me.sh && python3 tests/_temp/hg_a6_c209_worker_probe.py`; lease-gated `bash tests/_temp/hg_a6_c209_retention_e2e.sh`                                                                                                                                                                                                                                |
| C210     | Complete. `assessment_attempt_history.sql`, `student_assessment_landing.sql`, `grading_access.sql`, `assessment_attempt_access.sql`, and the explicit `archived_student_work_recovery.sql` path keep archived Work out of normal readers while preserving bounded current-Instructor recovery before expiry/deletion.                                                                                                                                                                                                      | Fresh PostgreSQL 17.11 actual-role plus production-server proof covered active visibility, the real retention archive, every current ordinary Student Work route, Instructor Gradebook, current-Instructor-only recovery, expiry concealment, and physical deletion. Receipt: `/private/tmp/ple-c210-archived-visibility-20260916.md`. No permanent test retained. |
| C211     | `schemas/base_schema/50_functions/question_authoring_operations.sql`; turn the already implemented A7 Published Question distinction (source/answer/grading/feedback/assets revise; title/description/tags/subject/topic do not) into a connected invariant. It depends on no unclosed A7 milestone.                                                                                                                                                                                                                                    | `source source_me.sh && python3 tests/_temp/hg_a6_c211_revision_probe.py`                                                                                                                                                                                                                                                                                          |
| C212     | `schemas/base_schema/assignments.sql` Pool tables/edit functions, after A7 published-Pool lineage design. Immutable Pool Revision identity and sequential per-Pool number.                                                                                                                                                                                                                                                                                                                                                 | `source source_me.sh && python3 tests/_temp/hg_a6_c212_pool_probe.py`                                                                                                                                                                                                                                                                                              |
| C213     | `schemas/base_schema/attempts.sql` selection/issuance constraints persist exact Pool Revision with selected Published Question Revision.                                                                                                                                                                                                                                                                                                                                                                                   | `source source_me.sh && python3 tests/_temp/hg_a6_c213_pin_probe.py`                                                                                                                                                                                                                                                                                               |

### C800-C893: accepted cross-boundary corrections

These records reconcile the named closure owners without adding a second owner. New probes are
ignored and removed after use unless the behavior independently passes `docs/PYTEST_STYLE.md`.
C821's Account Settings scope decision is recorded; C822-C823 may implement only its self-only
time-zone behavior. Account Settings exposes no credential lifecycle. HG-required Student/Instructor
passwordless authentication and multiple Student passkeys remain Accounts-and-roles milestone-owned;
only self-service credential enumeration/revocation/re-authentication, identity-proofed
recovery/notification, and session termination remain unresolved pending a separate decision.

| ID   | Atomic owner and outcome                                                                                                                                                                                                                                                                                                                                                                                                                      | Boundary / dependency                                                                                                                                                              | Gate and lifetime                                                                                                                                                                                                                                                                                                                                                                |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C800 | C13 verification only: prove seeded-role entry and no email-code delivery; no product/UI edit.                                                                                                                                                                                                                                                                                                                                                | `tests/test_live_demo_auth_ui.mjs`; `auth/live_demo.rs` and live-demo UI.                                                                                                          | `node --import tsx --test tests/test_live_demo_auth_ui.mjs`; ignored service probe.                                                                                                                                                                                                                                                                                              |
| C801 | C14 U.S. `.edu` validator contributor; reject lookalike suffixes and create no Account.                                                                                                                                                                                                                                                                                                                                                       | `crates/learning-data-access/src/course_roster.rs`; C802.                                                                                                                          | `rg -n -i 'live-demo\.invalid                                                                                                                                                                                                                                                                                                                                                    | m17\.support | screenshot\.support | @[^[:space:],;]+\.edu | Email, roster ID' schemas/installation_data/live_demo.sql schemas/installation_data/live_demo_oracle.sql schemas/installation_data/prepublication_context.sql crates/learning-data-access/src/course_roster.rs crates/learning-data-access/src/postgres/course_roster.rs crates/server/src/course_roster.rs src/api/course_roster.ts src/api/decoders/course_roster.ts src/pages/course_roster_page.tsx src/pages/roster_import_template.ts tests/e2e/e2e_live_demo_roster.sh tests/e2e/e2e_live_demo_support_capability.sh tests/playwright/e2e_live_demo_roster_browser.mjs tests/playwright/e2e_live_demo_support_capability_browser.mjs tests/playwright/screenshot_corpus/scenarios_instructor.ts tests/playwright/screenshot_corpus/scenarios_student.ts tests/playwright/screenshot_corpus/scenarios_sysadmin.ts tests/playwright/capture_live_demo_screenshots.mjs tests/playwright/screenshot_corpus/cli.ts docs/screenshots/current_capture_manifest.json`; then temporary malformed-address probe. |
| C802 | C14 lookup-or-create owner: migrate `live_demo.sql`, `live_demo_oracle.sql`, `prepublication_context.sql`, roster/support E2E/browser expectations, scenarios, and captures; prove reuse/exactly-one create/no rejected-input effect.                                                                                                                                                                                                         | Includes `scenarios_sysadmin.ts` roster import, support `m17.support`, `e2e_live_demo_support_capability_browser.mjs`, and Instructor/Student/Sysadmin `screenshot.support`; C801. | `bash tests/e2e/e2e_live_demo_roster.sh --import`; `bash tests/e2e/e2e_live_demo_roster.sh --browser`; `bash tests/e2e/e2e_live_demo_support_capability.sh --issue`; `bash tests/e2e/e2e_live_demo_support_capability.sh --browser`; leased capture.                                                                                                                             |
| C803 | C15 database owner: encrypted-at-rest wrapped TOTP seed and role-derived atomic unused Account/browser-bound attestation consumption plus session creation; Store only typed calls.                                                                                                                                                                                                                                                           | `authentication.sql`, `authentication_ceremony.rs`, PostgreSQL adapter; recorded TOTP decision.                                                                                    | Fresh-schema fixture; `cargo test -p learning-data-access --features postgres --lib`; retain only stable final-gate/one-use behavior.                                                                                                                                                                                                                                            |
| C804 | C15 server owner: primary-to-pending MFA, 30-second TOTP verification/counter replay/rate limit, then C803 final gate.                                                                                                                                                                                                                                                                                                                        | `auth.rs`, `auth/browser_boundary.rs`, `auth/session_cookie.rs`; C803.                                                                                                             | Existing auth E2E and temporary controlled-secret probe; retain role outcome only.                                                                                                                                                                                                                                                                                               |
| C805 | C15 Live Demo owner: Morgan selection follows C804 ceremony and creates pending/no session, never demo-only auth.                                                                                                                                                                                                                                                                                                                             | `auth/live_demo.rs`, `live_demo_auth_model.ts`, `live_demo_auth.css`; C800/C804.                                                                                                   | `node --import tsx --test tests/test_live_demo_auth_ui.mjs`; leased browser evidence.                                                                                                                                                                                                                                                                                            |
| C806 | C15 controller owner: OS-CSPRNG seed, encrypted persistence, ignored mode-0600 path-only artifact, separate authenticator.                                                                                                                                                                                                                                                                                                                    | Local controller fixture; C803-C805.                                                                                                                                               | Ignored provisioning probe; never retains/logs seed or code.                                                                                                                                                                                                                                                                                                                     |
| C807 | C15 integration owner: pending/no session, wrong-code denial/rate limit, genuine session, replay denial, ordinary roles unchanged.                                                                                                                                                                                                                                                                                                            | `tests/playwright/e2e/auth_authorization.spec.ts`; C803-C806.                                                                                                                      | `npx playwright test tests/playwright/e2e/auth_authorization.spec.ts`; retain deterministic outcome only.                                                                                                                                                                                                                                                                        |
| C808 | C21 evidence-only audit of existing `course_core.sql` creation anchor/current fields; no new policy state and no closure.                                                                                                                                                                                                                                                                                                                     | `created_at`, `active_until_at`, `latest_assessment_due_at`, `retention_starts_at`, lifecycle fields; C206 consumes facts.                                                         | Read-only inventory and ignored fresh-schema probe.                                                                                                                                                                                                                                                                                                                              |
| C809 | C207 contributor: Assignment save/release atomically synchronizes latest due and rejects due beyond active limit.                                                                                                                                                                                                                                                                                                                             | `assignments.sql`, `ple_api.save_assignment*`, `postgres/assignment_release.rs`; C206/C207.                                                                                        | Deterministic-concurrency expansion of `hg_a6_c207_deadline_probe.py`; promote only if earned.                                                                                                                                                                                                                                                                                   |
| C810 | C24 command correction only.                                                                                                                                                                                                                                                                                                                                                                                                                  | PostgreSQL authorization contract.                                                                                                                                                 | `cargo test -p learning-data-access --features postgres --lib`.                                                                                                                                                                                                                                                                                                                  |
| C811 | C37 PostgreSQL proof: fresh labelled PostgreSQL 17 baseline, administrator `CREATE EXTENSION dblink`, fixture, trap cleanup; never Browser Suite DB.                                                                                                                                                                                                                                                                                          | `e2e_database_baseline.sh`, baseline compose, profile schema.                                                                                                                      | Ignored `hg_c37_profile_avatar_schema.sh --postgres17`: Student select/replay/upload denial, staff self image, other-account concealment, replacement cleanup, two-session race.                                                                                                                                                                                                 |
| C812 | C38 prerequisite: one typed ProfileImage address/data/storage bridge, remove second live ProfileThumbnail path.                                                                                                                                                                                                                                                                                                                               | `ProfileImageReference`, `ObjectAddress::ProfileImage`, ProfileImage data class, private content, `profiles/images/{image}/{object_id}`.                                           | Retain exactly typed private/signed/serde/path coverage because it prevents exposure/wrong physical address, plus legacy `profileThumbnail` JSON rejection because it prevents a second current-image path. Remove bridge proof after C38 acceptance; failure restores ProfileImage-only privacy/address/signing and repairs the owning migration, never legacy deserialization. |
| C813 | C54 frontend contributor: centered responsive 5:1 banner; no source validation or rendition decision.                                                                                                                                                                                                                                                                                                                                         | Course-appearance frontend/API; C815.                                                                                                                                              | Temporary 1280-by-800/narrow viewport proof.                                                                                                                                                                                                                                                                                                                                     |
| C814 | C54 inventory owner across model/server/data access/objects/`course_media.sql`/frontend/E2E/Playwright consumers.                                                                                                                                                                                                                                                                                                                             | `CourseBannerRendition`, `CourseBanner`, `course_banner`, `normalized_course_banner`.                                                                                              | HG requires 5:1 and recommends 1280x256; exact/minimum rendition remains unlocked.                                                                                                                                                                                                                                                                                               |
| C815 | C54 server owner: accept valid still 5:1 within safety bounds including smaller, 1280x256, and higher; reject non-5:1; no crop.                                                                                                                                                                                                                                                                                                               | Course-appearance validation/store/schema; C814.                                                                                                                                   | Temporary small/1280/2560/non-5:1 fixtures plus E2E/browser; 5:1/no-crop is durable candidate.                                                                                                                                                                                                                                                                                   |
| C816 | C28 command correction only.                                                                                                                                                                                                                                                                                                                                                                                                                  | Keyboard reorder tests.                                                                                                                                                            | `node --import tsx --test tests/test_blueprint_course_model.mjs tests/test_ple_question_json_editor_model.mjs`.                                                                                                                                                                                                                                                                  |
| C817 | C203 reconciliation: permanent real-session BOLA/FERPA outcome oracle replaces temporary probe.                                                                                                                                                                                                                                                                                                                                               | `authorization.sql`; `crates/learning-data-access/tests/assignment_access_postgres.rs`.                                                                                            | Existing `bash tests/e2e/e2e_database_baseline.sh`; failure retains D05 and repairs session/predicate/ownership.                                                                                                                                                                                                                                                                 |
| C818 | C36 narrow interaction owner: avatar opens menu and Sign Out relocates; contributes but does not close contents/no-scattering.                                                                                                                                                                                                                                                                                                                | Ribbon component/CSS/contract; C35.                                                                                                                                                | Ribbon contract plus `ribbon_shell_evidence.mjs`; retain pointer/keyboard/focus/relocation behavior.                                                                                                                                                                                                                                                                         |
| C819 | Role-neutral Profile Settings authorization: self-derived time zone and C39's self-only avatar capability only; no catalog selection or avatar closure.                                                                                                                                                                                                                                                                                       | Account time-zone store/account-profile server; C39.                                                                                                                               | PostgreSQL/lib/server self/other-denial probe; remove unless a stable authorization outcome earns promotion.                                                                                                                                                                                                                                                                     |
| C820 | Role-neutral `/profile` page/route, migrating rather than duplicating Instructor Profile; generic identity/time-zone only until C836.                                                                                                                                                                                                                                                                                                         | routes/route contract/profile page/API; C819.                                                                                                                                      | Ribbon route/contract and temporary role matrix.                                                                                                                                                                                                                                                                                                                                 |
| C821 | Recorded scope: every role gets only self-derived time-zone read/write at `/account-settings`; Account Settings exposes no credential lifecycle. Student/Instructor passwordless authentication and multiple Student passkeys remain Accounts-and-roles milestone-owned. Only self-service credential enumeration/revocation/re-authentication, identity-proofed recovery/notification, and session termination remain unresolved.            | `docs/DESIGN_DECISIONS.md` Account Settings decision; no credential-lifecycle implementation.                                                                                      | Decision satisfied; no HG closure or behavior evidence implied.                                                                                                                                                                                                                                                                                                                  |
| C822 | All-role `/account-settings` consumes C821's self-only time-zone behavior; invitations remain separate.                                                                                                                                                                                                                                                                                                                                       | routes/route contract/account-settings page/API; C821.                                                                                                                             | Closed-shape self-only behavior plus Ribbon route/contract tests; no fake controls.                                                                                                                                                                                                                                                                                              |
| C823 | Final menu composition is sole owner of real `/profile`, `/account-settings`, and Sign Out contents/no-scattering closure.                                                                                                                                                                                                                                                                                                                    | Ribbon/route contract; C818/C820/C822.                                                                                                                                             | Ribbon route/contract, M10 shell tests, leased capture.                                                                                                                                                                                                                                                                                                                          |
| C824 | C311 evidence owner: inventory every biologyproblems.org WeBWorK problem family, record provenance/license and its canonical algorithmic author source (official PG/PGML file or generator), and map generated static QTI/PG variants. The accepted inventory contains 42 canonical PGML sources (41 official biologyproblems-website sources plus HLA), but it creates no Question lineage, Pool, Blueprint, archive, or historical rewrite. | `docs/TODO.md`; `content/genetics/manifest.yaml`, `content/genetics/sources/`, `content/genetics/pg/`; hands C838.                                                                 | A temporary independent validator accepted all 42 manifest registrations, including source-format/path, local and upstream SHA, license, and legacy-bank mapping; its 42-source renderer/lint/whitelist and representative seed/grading evidence passed and was removed. A missing canonical source is an implementation gap to repair, not authority to retain static copies.   |

| C832 | Avatar-catalog contributor: canonical original safe SVG assets, `avatar_catalog` manifest and PROVENANCE, with one deterministic generator deriving Rust/TypeScript/SQL registry data. | Asset source/manifest/provenance/generator only; no selection route. | Ignored safe-SVG/provenance/determinism probe and SVG-skill renders at smallest, typical, largest picker tiles with dimensions and accessible-name/decorative evidence; repair then remove. |
| C833 | Avatar-catalog schema contributor: seed generated catalog facts in `profile_media.sql`; `is_selectable` controls new selection while retired assets still render for existing rows. | C37 and C832 generated SQL; fresh PG17. | Ignored PG17 seed/checksum, unknown-ID, selectable-only, and retired-render/no-new-select fixture; repair invariants, then remove. |
| C834 | Avatar-catalog server/domain contributor: generated-registry conformance, deny unknown/retired selection, allow selected retired rendering. | C832/C833; C836 is its sole route/UI consumer. | Ignored domain/server unknown-retired-render probe; no permanent generated snapshot/call-order test. |
| C835 | Avatar UI contributor: reusable `ProvidedAvatarPicker` plus `AvatarVisual` from C832's generated TypeScript registry, keyboard/name/text alternative/playful grid. The pure picker has only `currentAvatarId` and `onSelect(id)` props: no route, API call, C834 server shape, or closure. | C832 generated TypeScript registry only. | Ignored component accessibility/render probe and SVG-skill three-tile evidence; remove. Promote only independently justified stable accessible behavior. |
| C836 | Avatar UI integration contributor: use C835 only on real `/profile` after C819/C820; Student provided-only/no upload, staff provided plus C39 self-image. C40/C41 remain closure owners. | C39,C819,C820,C834,C835. | Ignored three-role route/browser plus Student-denial/staff-delivery proof; remove then use C40/C41 acceptance gates. |
| C837 | C42 contributor and terminal privacy question: project generic/provided static assets across representation surfaces; private staff Profile images remain C39 self-only, not cross-account. | C35,C39,C835,C836; C42 only after decision. | Ignored surface inventory/matrix; remove. **Question:** May private Instructor/Sysadmin Profile images be delivered cross-account, and to which authorized roles/surfaces? HG does not resolve it; static provided assets alone may render cross-account. |
| C838 | ACCEPTED C311 shipped-catalog input and publication owner: the canonical Genetics manifest contains the 42 C839-accepted canonical PGML sources (41 biologyproblems-website sources plus HLA), provenance/license, nine nonempty topics, and direct Fixed entries. Fresh PostgreSQL 17/MinIO evidence published 42 ordinary available WeBWorK Question lineages at Revision 1, with 42 Fixed entries and zero Pools; exact replay made no mutation and a same-short-name conflict made no mutation. | Canonical `content/genetics/pg/topicNN/` sources and minimal Genetics manifest; `question_authoring_operations.sql`, `question_publication.rs`; C824. | Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`. This is no parser or separate backend and has no retained-catalog migration, static-row equivalence, historical ID/revision/pin rewrite, or Pool-retirement claim. |
| C839 | ACCEPTED C311 canonical-source acceptance owner: the shipped fresh Genetics catalog contains exactly 42 accepted canonical PGML sources: 41 official biologyproblems-website sources plus HLA. Acceptance records source/provenance and the deterministic render/grading contract for those sources; it is not row-for-row equivalence to static expansions. The 43rd Chargaff source is unaccepted. | C838's manifest inputs; hands fresh install and, only for a retained database lineage, C840. | The 76 banks and 13,434 old generated rows are an explicit non-published import inventory, not the active install manifest. They remain unresolved and neither imported nor complete. Do not restore static expansions or claim their publication, migration, or retirement. |
| C840 | C311 retained-catalog migration owner, not a fresh-baseline requirement: only when an actual existing database retains Genetics lineages, Blueprint pins, or Student Work, reconcile through current ordinary Revision and availability operations with expected-current CAS. Preserve every historical pin and Student Work record; a stale CAS makes no publication mutation. | Existing retained database records; current Question, Blueprint Revision, availability, and CAS operations; C839. | The disposable fresh baseline has no existing catalog to reconcile and needs no Pool-retirement subsystem. Do not infer or create implicit Pools. A retained-lineage migration uses ordinary forward Revisions/availability, never deletion or historical rewrite. |
| C841 | C311 retained-catalog recovery owner, not a fresh-baseline requirement: after a successful C840 CAS operation, recover any retained static lineage only through ordinary availability and a later Revision while preserving Blueprint and Student Work pins. | Existing retained database records and C840; no fresh-install manifest change. | No persistent database reset is authorized by this plan edit. Any reset requires verified disposable state, confirmation that no durable data exists, and the existing operator command. Never silently drop pins or present a retained-catalog migration as completed without actual-database proof. |

Cross-boundary DAG: `C801 -> C802`; recorded TOTP decision -> `C803 -> C804 -> C805 -> C806 -> C807`;
C808 -> C206 and `{C206, C809} -> C207`; `C37 -> C811`; C812 and `C832 -> C833 -> C834`;
`{C812, C833} -> C38 -> C39 -> C819 -> C820`; `C832 -> C835`;
`{C39, C820, C834, C835} -> C836 -> C40/C41`; `{C35, C39, C835, C836} -> C837 -> C42` after its
terminal privacy decision; `C814 -> C815 -> C813`; `C821 -> C822`; `{C818, C820, C822} -> C823`;
`C311 -> C824 -> C838 -> C839` defines and proves the fresh canonical Genetics manifest,
publication, and idempotent install. `C840 -> C841` is a separate branch only for an actual
retained database with lineages, Blueprint pins, or Student Work; it uses ordinary Revision,
availability, and CAS rather than a fresh-baseline Pool-retirement subsystem.

For the unresolved C14 non-U.S. case, retain: `Reason: product decision still unclear` and
`Question: For a Student whose institution uses a non-US or non-.edu domain, what approved institutional-domain evidence or configuration authorizes roster import?`

## A7 Questions - canonical dispatch map

A7 owns exactly 76 unchecked product-behavior occurrences. The exact duplicate
**"Privacy-safe aggregate Question statistics remain after the underlying Student records are
deleted."** remains A6 C208, not an A7 closure. C321 owns the distinct A7 aggregate-retention
requirement. C322 alone waits for unresolved A6 C205's disclosure threshold.

The optional abandoned-Draft-cleanup sentence is an audited N/A, not a missing
product behavior: `Reason: Automated abandoned-Draft cleanup is an explicitly optional future
capability; HG sets no clock or durations.` C861 removes only prohibited placeholder seams under
HG's no-placeholder rule. The unresolved product question is: **Should PLE automate cleanup of
abandoned Draft Questions? If yes, what event starts inactivity; how long until warning; how long
is the recovery period after a successfully delivered warning; which save/edit/publication/ownership
events reset or cancel it; and what is the outcome when warning delivery fails?**

A closure row owns its listed occurrence count; a contributor row owns zero and exists only to
supply its listed successor. Thus each closure occurrence has exactly one owner. Each row has one
code boundary. Before running a temporary gate, its coder creates the named ignored probe; after
review, removes it unless the row explicitly permits promotion after every `docs/PYTEST_STYLE.md`
question is answered yes. Any failure leaves only that row's occurrence open and returns to its
named boundary. C317's 13,000-Question fixture is always temporary.

| ID   | Kind; occurrences; one boundary and outcome                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Prerequisites / handoff                                                                                            | Exact focused gate and test lifetime                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C300 | contributor; 0; evidence-only inventory of the former seed-bearing native path. It closes no HG occurrence and hands the approved seed-free vertical chain to C825.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Architect decision "Native PLE JSON attempt reproduction is seed-free"; no sentinel/fixed seed implementation.     | Ignored inventory records the removed seed/hash surfaces and does not become a permanent test.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| C301 | closure; 2; `crates/adapters/ple/src/question_json/source_document.rs`: record reviewable native external resources.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | -; hands C302,C305.                                                                                                | Create `tests/_temp/hg_a7_c301_resource_manifest_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c301_resource_manifest_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| C302 | closure; 2; `crates/adapters/ple/src/question_json/source_document.rs`: declare author script/RDKit without seed.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C301; hands C303,C857.                                                                                             | Create `tests/_temp/hg_a7_c302_author_script_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c302_author_script_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| C303 | contributor; 0; architectural handoff to the approved answer-free isolated-document boundary. It implements no adapter, route, frame, or proof and owns no HG occurrence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | C302; C857-C860 carry the implementation chain.                                                                    | Record the durable decision and inspect the C857-C860 handoff; no permanent test.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| C304 | closure; 3; `src/components/question_response_controls/hotspot.tsx`: PLE-owned HOTSPOT input/assets and server-owned grading.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | C831,C860.                                                                                                         | Create `tests/_temp/hg_a7_c304_hotspot.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c304_hotspot.spec.ts`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| C305 | contributor; 0; current `crates/adapters/ple/src/question_json/source_document.rs` validation of author-declared `cdnUrl`/`localPath` is not an approved inventory or delivery path. C899 removes it; no HG occurrence closes here.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C301; hands C899.                                                                                                  | Record the false-closure audit; C899's ignored source-shape migration matrix replaces the old probe. No permanent test.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| C306 | contributor; 0; `schemas/base_schema/attempt_presentation.sql`: discover current production Backend bindings only.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | PLE and WeBWorK are current; iMathAS and H5P are deferred and create no current handoff.                           | Create a temporary Backend-seam probe only for a current closure need; remove it.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| C307 | contributor; 0; `crates/question_model/src/question_library.rs`: common adapter type foundation.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | C306; hands C358,C325.                                                                                             | Create `tests/_temp/hg_a7_c307_contract_shape_probe.py`; run `source source_me.sh && cargo test -p question_model`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| C308 | contributor; 0; `crates/adapters/webwork/src/lib.rs`: opaque backend fixtures.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | C307; hands C331-C333,C361.                                                                                        | Create `tests/_temp/hg_a7_c308_backend_fixture.sh`; lease-gated run `bash tests/_temp/hg_a7_c308_backend_fixture.sh`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C309 | closure; 5; `crates/server/src/assignment_delivery/direct_finalization.rs`: synchronous immutable fraction and score derivation.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | C362,C333; hands C310,C324.                                                                                        | Create `tests/_temp/hg_a7_c309_result_pipeline_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c309_result_pipeline_probe.py`; retain only if PYTEST_STYLE approves immutable public outcome contract.                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| C310 | closure; 1; `crates/server/src/assignment_delivery/direct_finalization.rs`: reproject score when points change without backend call.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | C309.                                                                                                              | Create `tests/_temp/hg_a7_c310_rescore_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c310_rescore_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| C311 | contributor; 0; inventory every biologyproblems.org WeBWorK family against bundled Genetics content and identify its canonical algorithmic author source, closed `source_format` (`pg` or `pgml`), matching extension, and generated static variants. `pgml` requires fully PGML-compliant source; traditional or mixed source remains `pg`. This is metadata only, not a runtime parser or separate backend. Any current same-question Genetics folder containing more than one `.pg` file is an incorrect static import to clean up. It supplies evidence for C840/C841 and closes no HG occurrence itself.                                                                                                                                                        | `docs/TODO.md`; `content/genetics/manifest.yaml`, `content/genetics/sources/`, `content/genetics/pg/`; hands C824. | Ignored family ledger records source format/extension, source hash/provenance/license, canonical author source, generated coverage, and source count; remove after C824 handoff. No adapter heuristic, runtime parser, separate backend, automatic Pool substitution, or permanent catalog snapshot.                                                                                                                                                                                                                                                                                                                                                                                  |
| C312 | contributor; 0; `schemas/base_schema/50_functions/question_pools.sql`: reusable published Pool root plus later-independent immutable Pool Revision/member physical foundation. It has no Assessment-local `question_pool_item` substitute.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | -; hands C313,C354,C885.                                                                                           | Ignored fresh-schema root/member separation proof; remove. No table-inventory test is permanent.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| C313 | contributor; 0; `question_pools.sql`: immutable ordered Pool Revision members pin each member's exact Published Question ID and Revision, require a nonempty distinct member set, and retain an Instructor interchangeability attestation. Pool changes append a Revision under a Pool metadata ETag/CAS; they never mutate an earlier Revision. Membership is backend-neutral; C904 assigns selection count to the Assessment-owned fork, not the reusable Pool Revision.                                                                                                                                                                                                                                                                                           | C312; hands C354,C885,C905.                                                                                        | Ignored fresh-schema append/CAS/member-pin/backend-mix matrix; remove. Retain only a small immutable-revision or CAS outcome if every `PYTEST_STYLE.md` criterion passes.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| C314 | closure; 4; `schemas/base_schema/50_functions/question_pools.sql`: import/fork a Pool for an Assessment while retaining member IDs.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | C909,C362.                                                                                                         | Create `tests/_temp/hg_a7_c314_pool_fork_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c314_pool_fork_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| C315 | contributor; 0; `schemas/base_schema/attempt_access.sql`: persisted per-Attempt selection read/write.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | C353; hands C334 and A6 C213.                                                                                      | Create `tests/_temp/hg_a7_c315_selection_probe.py`; run `source source_me.sh && cargo test -p learning-data-access`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| C316 | contributor; 0; `src/route_contract.ts`: Pool-library and Student-route candidates.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C354; hands C364,C336.                                                                                             | Create `tests/_temp/hg_a7_c316_access_manifest.mjs`; run `node tests/_temp/hg_a7_c316_access_manifest.mjs`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| C317 | contributor; 0; `crates/server/src/question_library/paging.rs`: 13k load fixture.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C58; hands C338.                                                                                                   | Create `tests/_temp/hg_a7_c317_13k_fixture.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c317_13k_fixture.py`; remove, always temporary.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| C318 | contributor; 0; `crates/server/src/question_publication.rs`: identifier/collision seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | -; hands C369,C319.                                                                                                | Create `tests/_temp/hg_a7_c318_identifier_seam_probe.py`; run `source source_me.sh && cargo test -p server_core`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| C319 | contributor; 0; audit pointer: record that `question_lineages.sql` installs before authoring tables, so it cannot create a Draft or `draft_question_fork_source`. It owns no SQL source read/pin, Draft operation, or test; C876 is the one lineage implementation owner.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | C211,C846; hands C876.                                                                                             | Record the install-order receipt; no implementation test or permanent inventory.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| C320 | contributor; 0; `crates/question_model/src/question_stewardship.rs`: stewardship-event vocabulary.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | C359; hands C370,C372.                                                                                             | Create `tests/_temp/hg_a7_c320_stewardship_fixture.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c320_stewardship_fixture.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| C321 | closure; 3; `schemas/base_schema/20_tables/statistics.sql`: revision-scoped aggregate counts survive Student deletion.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | A6 C208,C211,C213; hands C322.                                                                                     | Create `tests/_temp/hg_a7_c321_aggregate_retention_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c321_aggregate_retention_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| C322 | closure; 5; `src/pages/question_statistics_panel.tsx`: labeled, course-sensitive non-identifying aggregate view.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | C321,A6 C205.                                                                                                      | Create `tests/_temp/hg_a7_c322_statistics_privacy.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c322_statistics_privacy.spec.ts`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| C323 | contributor; 0; `crates/question_model/src/presentation/choice_order.rs`: Question choice configuration foundation.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C831,C64; hands C348.                                                                                              | Create `tests/_temp/hg_a7_c323_choice_fixture.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c323_choice_fixture.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| C324 | contributor; 0; `src/components/student_feedback_panel.tsx`: optional-feedback state.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | C309; hands C350.                                                                                                  | Create `tests/_temp/hg_a7_c324_feedback_state.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c324_feedback_state.spec.ts`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| C325 | contributor; 0; `schemas/base_schema/50_functions/question_authoring_operations.sql`: Draft ownership predicate.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C307; hands C351.                                                                                                  | Create `tests/_temp/hg_a7_c325_draft_auth_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c325_draft_auth_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C328 | contributor; 0; `crates/question_model/src/question_library.rs`: split common-interface policy seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | C307; successor C358.                                                                                              | Create `tests/_temp/hg_a7_c328_backend_contract_seam.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c328_backend_contract_seam.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| C330 | contributor; 0; `crates/server/src/assignment_delivery.rs`: split backend-ownership seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | C359,C361; successor C362.                                                                                         | Create `tests/_temp/hg_a7_c330_backend_ownership_seam.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c330_backend_ownership_seam.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| C331 | closure; 1; `crates/adapters/webwork/src/lib.rs`: opaque WeBWorK PG/PGML ownership, including renderer-owned transient feedback.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | C308,C358,C831.                                                                                                    | Create `tests/_temp/hg_a7_c331_webwork_opaque.sh`; lease-gated run `bash tests/_temp/hg_a7_c331_webwork_opaque.sh`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| C332 | closure; 1; `crates/adapters/imathas/src/imathas_question_backend.rs`: opaque iMathAS ownership.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | C308,C358,C831.                                                                                                    | Create `tests/_temp/hg_a7_c332_imathas_opaque.sh`; lease-gated run `bash tests/_temp/hg_a7_c332_imathas_opaque.sh`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| C333 | closure; 1; `crates/question_model/src/question_library.rs`: delivered complex interaction for current production Backends remains adapter-owned.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | C331 and current WeBWorK ownership; deferred iMathAS/H5P do not block current closure.                            | Use a temporary complex-Backend probe only for current PLE/WeBWorK behavior; remove it.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| C334 | closure; 1; `schemas/base_schema/attempt_access.sql`: fresh Pool selection persists for reload.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | C315,C909; hands A6 C213.                                                                                          | Create `tests/_temp/hg_a7_c334_fresh_selection_probe.py`; run `source source_me.sh && cargo test -p learning-data-access`; retain only if PYTEST_STYLE approves attempt-selection persistence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| C335 | contributor; 0; `schemas/base_schema/50_functions/question_authoring_operations.sql`: split vetted-Pool projection seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | C885,C316; successor C363/C364.                                                                                    | Create `tests/_temp/hg_a7_c335_pool_access_seam.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c335_pool_access_seam.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C336 | closure; 1; `src/route_access_boundary.tsx`: Student has Coursework, not Question Library.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | C316,C359.                                                                                                         | Create `tests/_temp/hg_a7_c336_student_library_denial.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c336_student_library_denial.spec.ts`; retain only if PYTEST_STYLE approves stable authorization denial.                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| C337 | closure; 1; `src/pages/library_page.tsx`: 13k Questions remain practical through real canonical selection and bulk shared-metadata workflow, not primarily archival.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | C9,C338,C365,C367,C893,C366,C368.                                                                                  | Create `tests/_temp/hg_a7_c337_13k_archive_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c337_13k_archive_probe.py`; remove after this corrected plan gate, always temporary.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| C338 | contributor; 0; `src/api/question_library_repository.ts`: split bulk-operation request seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | C317; successor C365/C366.                                                                                         | Create `tests/_temp/hg_a7_c338_bulk_api_seam.mjs`; run `node --import tsx tests/_temp/hg_a7_c338_bulk_api_seam.mjs`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| C339 | contributor; 0; audit pointer: current repository shared-metadata request seam is not an implementation boundary. It creates no API, mutation, or test; C367 is the one typed LDA implementation owner.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | C365; hands C367.                                                                                                  | Record the former seam's replacement; no implementation test or permanent inventory.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| C340 | closure; 1; `src/pages/library_page.tsx`: search/filter/sort/bulk edit make import cleanup practical.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | C58,C368.                                                                                                          | Create `tests/_temp/hg_a7_c340_import_cleanup.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c340_import_cleanup.spec.ts`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| C341 | contributor; 0; `crates/server/src/question_publication.rs`: split Question-ID formatter seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | C318; successor C369.                                                                                              | Create `tests/_temp/hg_a7_c341_question_id_seam.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c341_question_id_seam.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C342 | closure; 1; `schemas/base_schema/50_functions/question_pools.sql`: published Questions and Pools project public Crockford IDs.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C354,C846,C887.                                                                                                    | Create `tests/_temp/hg_a7_c342_pool_id_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c342_pool_id_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| C343 | closure; 2; `crates/server/src/question_publication.rs`: public-ID format, retry, and uniqueness.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C846.                                                                                                              | Create `tests/_temp/hg_a7_c343_collision_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c343_collision_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| C344 | contributor; 0; `schemas/base_schema/50_functions/question_stewardship.sql`: split star-store seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | C320; successor C370/C371.                                                                                         | Create `tests/_temp/hg_a7_c344_star_store_seam.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c344_star_store_seam.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| C345 | contributor; 0; `schemas/base_schema/50_functions/question_stewardship.sql`: split watch-store seam.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | C320; successor C372/C373.                                                                                         | Create `tests/_temp/hg_a7_c345_watch_store_seam.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c345_watch_store_seam.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C346 | contributor; 0; Watch notifications use source-bound event kinds and no generic hook. Revision/fork events exist; improvement-thread/impact-notice lifecycles are settled and their source core is implemented, while four-event delivery remains in progress.                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | C373; hands C874/C875 after implemented C871-C873 source review and proof.                                        | Use temporary transaction/delivery probes; remove them. No permanent event-inventory or mock-orchestration test.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| C347 | closure; 2; `src/pages/question_detail_page.tsx`: published Question stewardship model exposes star/watch.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | C371,C373.                                                                                                         | Create `tests/_temp/hg_a7_c347_stewardship_closure.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c347_stewardship_closure.spec.ts`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| C348 | closure; 1; `crates/adapters/ple/src/question_json/source_document.rs`: native Question declares choice randomization.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | C323.                                                                                                              | Create `tests/_temp/hg_a7_c348_choice_declaration_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c348_choice_declaration_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C349 | closure; 1; `crates/question_model/src/presentation/choice_order.rs`: delivery obeys Question choice randomization, not Assessment policy.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | C348,C64.                                                                                                          | Create `tests/_temp/hg_a7_c349_choice_delivery_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c349_choice_delivery_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| C350 | closure; 1; `src/components/student_feedback_panel.tsx`: completion works with feedback withheld or shown.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | C324.                                                                                                              | Create `tests/_temp/hg_a7_c350_feedback_optional.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c350_feedback_optional.spec.ts`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| C351 | closure; 1; `schemas/base_schema/50_functions/question_authoring_operations.sql`: Instructor deletes own Draft, never another's.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C325.                                                                                                              | Verified: accepted isolated PostgreSQL 17/MinIO actual-server and focused browser proof covered owner cancel/confirm and reload persistence; valid-current-ETag collaborator, unrelated Instructor, Student, Sysadmin, and anonymous denials with unchanged owner source/Edit Number; 428 missing, 400 malformed, and 412 stale preconditions; preserved parsed Published Question lineage/Revision JSON; and 404 repeat DELETE/PUT after deletion. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`. This does not claim S3 erasure, automated cleanup, authentication acceptance, full Live Demo browser acceptance, or a healthy backend while the renderer is disabled. |
| C353 | contributor; 0; `crates/domain/src/question_pool_selection.rs`: backend-neutral deterministic selection from the exact Assessment-owned fork Pool Revision after C905-C906 carry the existing per-entry `selection_count` and provenance. It never changes backend-native randomization.                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | C313,C905,C906; hands C315,C907.                                                                                   | Ignored selection/reload/new-Attempt matrix; remove. No random-fixture or algorithm snapshot is permanent.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| C354 | contributor; 0; audit pointer: Pool ID/revision schema requires the C312/C313 content foundation and C846 issuer evidence. It owns no storage, allocator, operation, or test; C885 is the sole Pool schema/create-operation owner.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | C312,C313,C846; hands C885,A6 C212/C213,C314,C363,C342.                                                            | Record the corrected ownership; no implementation test or permanent inventory.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| C355 | contributor; 0; audit pointer for the five Pool behavior occurrences formerly claimed by a schema-only selection path. It closes none: C909 is their sole closure after C904's engineering decision and C905-C908's real selection chain.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | C313,C887; hands C909.                                                                                             | Record the corrected ownership; no implementation test or permanent inventory.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| C358 | closure; 1; `crates/question_model/src/question_library.rs`: common backend interface.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | C328; hands C359,C331,C332,C361.                                                                                   | Create `tests/_temp/hg_a7_c358_interface_probe.py`; run `source source_me.sh && cargo test -p question_model`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| C359 | closure; 1; `crates/server/src/assignment_delivery.rs`: architect-approved PLE authorization/ID/revision/persistence/lifecycle/outcome shell.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | C358; hands C362,C320,C336.                                                                                        | Create `tests/_temp/hg_a7_c359_ple_shell_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c359_ple_shell_probe.py`; retain only if PYTEST_STYLE approves public authorization/outcome contract.                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| C361 | closure; 1; `crates/adapters/webwork/src/lib.rs`: backend-specific adapter knowledge.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | C306,C308,C358; hands C362.                                                                                        | Create `tests/_temp/hg_a7_c361_adapter_knowledge.sh`; lease-gated run `bash tests/_temp/hg_a7_c361_adapter_knowledge.sh`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| C362 | closure; 1; `crates/server/src/assignment_delivery.rs`: backend owns rendering, response, grading, feedback and state.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | C330,C331,C332,C333,C359-C361; hands C309.                                                                         | Create `tests/_temp/hg_a7_c362_backend_boundary_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c362_backend_boundary_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| C910 | closure; 3; author-managed general feedback is immutable Revision metadata and is projected at the submitted-history boundary independently of backend-provided interaction feedback. PLE preserves recorded native feedback, treats WeBWorK feedback as transient, and does not extract or reconstruct it from PGML or renderer output. Question Feedback has no Assessment delayed-release state; the six remaining timing fields independently gate score, correctness, submitted response, answer, explanation, and class statistics.                                                                                                                                                                                                                            | C307; independent of C331/C362.                                                                                    | Accepted fresh-PG17 publication proof preserved two immutable general-feedback values with identical source provenance. Accepted actual Student HTTP and exact-main browser proof started a WeBWorK Attempt, stopped the renderer, submitted it, rendered exact-Revision General feedback with all six timings `Never`, withheld response/score/correctness/answer/explanation, returned a completion-only two-field submission acknowledgement, and concealed history from nonowners and staff. Source and focused domain tests establish provided native feedback independently of answer disclosure; the runtime WeBWorK fixture supplied no transient backend feedback.           |
| C363 | contributor; 0; `schemas/base_schema/50_functions/question_authoring_operations.sql`: vetted-Instructor Pool projection.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | C335,C885; hands C364.                                                                                             | Create `tests/_temp/hg_a7_c363_pool_projection_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c363_pool_projection_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| C364 | closure; 1; `src/pages/library_route_page.tsx`: vetted Instructor sees published Pools.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | C316,C363.                                                                                                         | Create `tests/_temp/hg_a7_c364_pool_library_access.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c364_pool_library_access.spec.ts`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| C365 | contributor; 0; accepted current shared-metadata foundation: `PublishedQuestionSharedMetadata` is a closed DTO with a generated 1,000-item collection bound. Current `tags`, `subject`, and `topic` carry positive metadata Edit Numbers; source review establishes once-only canonical `PLE authoring`/`Pilot` initial tags for native publication and an empty WebWork start. The PostgreSQL command locks canonical IDs, validates active vetted-Instructor authority and every target before a single all-or-none update, and returns one canonically ordered whole result. Unknown, unavailable, unauthorized, stale, oversized, invalid, or duplicate selection changes nothing without per-item disclosure; no digest, receipt, or exactly-once claim exists. | C338; hands C367,C893.                                                                                             | Fresh PostgreSQL 17 SQL/API proof and independent rerun passed read/write/read, stale all-or-none denial, clear, concealed unauthorized/unvetted/archived/missing/duplicate cases, unchanged Revision count, private-helper denial, and explicit-initial-tag/empty-successor/null-tag rejection facts. The temporary proofs were removed.                                                                                                                                                                                                                                                                                                                                             |
| C366 | closure; 1; implementation exists in `src/pages/library_page.tsx` and its repository client: a selected canonical set invokes the shared-metadata editor with exact current metadata/Edit Numbers, whole results, and stale or ambiguous refresh. It makes no per-item disclosure or exactly-once claim.                                                                                                                                                                                                                                                                                                                                                                                                                                                             | C338,C893; hands C368.                                                                                             | Accepted temporary compiled Chromium component and strict-client proof covered sorted selection/Edit Numbers, virtualization, busy/blank/ID/denial/filter states, stale/ambiguous refresh without an automatic second write, no page errors, and zero critical/serious axe findings. Connected actual-server/private exact-main proof now covers a bounded selected set, one successful replace, stale 412, and refresh without retry; thousands-Question practical workflow evidence remains pending, so keep open.                                                                                                                                                                  |
| C367 | contributor; 0; accepted typed LDA `BulkPublishedQuestionMetadataStore` has one closed request/result model for C365's all-or-none command and excludes source, Revision, ownership, availability, backend, answer, grading, asset, authorship, and arbitrary JSON fields. The fresh PostgreSQL SQL/API receipt covers the command's database behavior; `cargo check -p learning-data-access --no-default-features --features postgres --lib` passed.                                                                                                                                                                                                                                                                                                                | C365; hands C893.                                                                                                  | Full server compilation and HTTP execution remain unverified because of the known AWS Smithy dependency incompatibility. No Store call-order test is permanent.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| C368 | closure; 1; selected Published Questions intentionally replace or clear only tags, subject, and topic through C366/C893, show a whole result, and refresh current metadata/Edit Numbers after stale response without changing source or creating a Question Revision.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | C9,C366,C893.                                                                                                      | Verified: accepted isolated actual-server HTTP and private exact-main browser proof covered current read, subject replace/clear, browser tag replace, search projection, authorized nonowner access, stale two-Question 412 with no partial write, browser refresh without an automatic second write, role denial, and unchanged source/Revision/availability. The bounded fixture does not prove 13k practical cleanup or screenshot-corpus acceptance.                                                                                                                                                                                                                              |
| C369 | open correction; `crates/server/src/question_publication.rs`: server-only Question-ID mint/parse/validation boundary must emit and accept the sole canonical `XXXX-ZXXX` value unchanged at storage, serde, and browser boundaries. Its embedded `Z` is the public SHA-256 checksum from the ASCII bytes of seven identity characters `XXXXXXX`; calculation excludes only hyphen and checksum position, while the canonical value retains both. The server validates syntax and checksum before lookup. | C318,C341,C842,C843; hands C844,C846. No dual parser, legacy rewrite, or compatibility read. | Create `tests/_temp/hg_a7_c369_question_id_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c369_question_id_probe.py`; remove. |
| C370 | contributor; 0; `schemas/base_schema/50_functions/question_stewardship.sql`: star persistence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C344,C320; hands C371.                                                                                             | Create `tests/_temp/hg_a7_c370_star_store_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c370_star_store_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C371 | complete closure; 2; `src/pages/question_detail_page.tsx` and `src/components/question_star_control.tsx` deliver a Star as visible favorite/endorsement plus vetted-Instructor count and exact verified-name projection. The C853 permanent real-session test passed: only an active vetted Instructor receives the closed name-only projection; anonymous, Student, inactive-Instructor, and non-Published routes are concealed. The one-time C854 compiled Chromium render/accessibility proof also passed and was removed.                                                                                                                                                                                                                                        | C370,C855 complete; hands C347.                                                                                    | Retain `tests/e2e/e2e_question_star_name_privacy.sh`: it protects the deliberately stable authorization and identity-disclosure boundary, has a clear repair action (restore the predicate/closed projection), and satisfies `PYTEST_STYLE.md`. The browser fixture was temporary-only and removed.                                                                                                                                                                                                                                                                                                                                                                                   |
| C372 | contributor; 0; `schemas/base_schema/50_functions/question_stewardship.sql`: watch persistence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | C345,C320; hands C373.                                                                                             | Create `tests/_temp/hg_a7_c372_watch_store_probe.py`; run `source source_me.sh && python3 tests/_temp/hg_a7_c372_watch_store_probe.py`; remove.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| C373 | closure; 3; `src/pages/question_detail_page.tsx`: subscription, private owner list, and Student/anonymous denial.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C372; hands C346,C347.                                                                                             | Create `tests/_temp/hg_a7_c373_watch_access.spec.ts`; run `npx playwright test tests/_temp/hg_a7_c373_watch_access.spec.ts`; retain only if PYTEST_STYLE approves stable authorization denial.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

### C825-C831: seed-free Native PLE JSON reproduction correction

The recorded decision "Native PLE JSON attempt reproduction is seed-free" replaces C300's
server-only sentinel approach. Native JSON has no random seed or generated-parameter hash in its
public presentation, persisted native representation, or resume path. A nonce remains only for
PLE-controlled response/choice order. Renderer-backed WeBWorK and iMathAS keep their server-only
seeded reproduction facts. C306 has no H5P mapping and is not a native-seed substitute.

| ID   | Kind; occurrence ownership; one boundary and outcome                                                                                                                                                                                                                            | Prerequisites / handoff                                                                                        | Exact focused gate and test lifetime                                                                                                                                           |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C825 | contributor; 0; `crates/question_model`: tagged reproduction evidence is `Static` or `Seeded { question_seed, generated_parameter_sha256 }`; native public presentation omits seed, descriptor checksum is v4, and nonce remains only for PLE-controlled response/choice order. | `docs/CONTRACTS.md` "Student Work and assessment evidence"; C300; hands C826,C827.                             | Ignored model/presentation fixture proves native seed absence and seeded-backend evidence shape; remove unless a stable public-representation contract earns promotion.        |
| C826 | contributor; 0; `crates/adapters/ple`: `issue_question_json(&source)` has no seed or parameter hash.                                                                                                                                                                            | C825; hands C829. WeBWorK/iMathAS interfaces retain seeds.                                                     | Focused adapter test plus ignored migration probe; retain no implementation-coupled call-order test.                                                                           |
| C827 | contributor; 0; base schema `attempts`, operations, interaction, presentation, and finalization persist nullable seed/hash pair with CHECK: PLE has both NULL; WeBWorK/iMathAS have both populated; triggers/APIs enforce.                                                      | C825; hands C828. Preproduction uses direct base correction and reinitialization, with no compatibility views. | Ignored fresh PostgreSQL 17 fixture proves accepted/rejected pairs and trigger/API enforcement; always temporary schema-migration proof.                                       |
| C828 | contributor; 0; LDA tagged types/codecs/read/write/resume preserve C827's backend distinction.                                                                                                                                                                                  | C827; hands C829.                                                                                              | Focused LDA codec/read-resume test; retain only if it protects stable externally meaningful resume behavior.                                                                   |
| C829 | contributor; 0; server start mints a seed only for `Seeded` according to source backend; native delivery/finalization has no seed and accepts no browser seed input.                                                                                                            | C826,C828; hands C830.                                                                                         | Ignored server delivery/finalization probe; failure repairs backend classification, not a sentinel.                                                                            |
| C830 | contributor; 0; browser API and TypeScript decoders reject legacy native seed fields and expose no public seed.                                                                                                                                                                 | C829; hands C831.                                                                                              | Closed-shape decoder/API probe; retain only stable public rejection behavior.                                                                                                  |
| C831 | closure; 1; connected fresh-PG17/server proof establishes native rows, payloads, and resume have no seed/hash; seeded backends retain server-only seed; invalid PLE seed insertion is rejected.                                                                                 | C830; hands C304,C323,C331,C332,C309.                                                                          | Ignored connected fixture first. Promote only one small deterministic native no-seed outcome contract under `docs/PYTEST_STYLE.md`; remove all fixtures/diagnostics otherwise. |

### C842-C846: Question-ID direct-cutover correction

The architect-approved Question-ID correction is one fresh preproduction
cutover. `docs/QUESTION_ID_SPEC.md` remains the parser authority: `XXXX-ZXXX`
is the sole value at storage, display, serde, and browser boundaries; its
embedded `Z` checks the seven identity characters `AAAABBB`; and every other
form is invalid. Calculation excludes only the hyphen and check character, and
the canonical value retains both. The server validates the checksum before lookup. The
fresh-schema preflight requires zero published Question rows; any nonzero row
stops and escalates. No task may add a dual parser, legacy reader, rewrite, or
compatibility path. C842-C845 explicitly exclude the active/accepted C358
`crates/server/src/question_library.rs` boundary and its tests.

| ID   | Kind; occurrence ownership; one boundary and outcome                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Prerequisites / handoff                                                                                                                   | Exact focused gate and test lifetime                                                                                                                                                                    |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C842 | contributor; 0; `crates/question_model/src/question_library.rs` is the one Rust contract input. `devel/generate_question_id_contract.py` derives the browser-safe alphabet, lengths, canonical `XXXX-ZXXX` contract, and closed human-input normalization at `generated/api/QuestionIdSyntaxContract.ts`; `src/question_id.ts` consumes it. The required check command is `source source_me.sh && python3 devel/generate_question_id_contract.py --check`. No second hand-maintained alphabet, normalization map, regex, or syntax rule remains; browser output has no secret, HMAC, allocator, or resolver capability. | `docs/QUESTION_ID_SPEC.md`; hands C843,C369,C844. | Ignored model/generator/output conformance probe covers canonical output, accepted closed human-input normalization, and invalid-form rejection; run the generation check; remove after handoff. |
| C843 | contributor; 0; fresh base schema and Live Demo fixtures use exact canonical IDs. One append-only global public-ID registry reserves every full issued public ID, including the shared Question/Pool `XXXX-ZXXX` namespace; no compact storage, migration, rewrite, or dual reader remains. | C842; hands C369,C845. | Ignored fresh-PG17 fixture proves atomic reservation and collision refusal across Question, Pool, and prefixed public-ID types; remove. |
| C844 | contributor; 0; browser codecs consume C842's generated syntax contract, normalize only closed human-input forms before validation and lookup, and copy/display only exact canonical `XXXX-ZXXX`. | C842,C369; hands C845,C846; excludes C358's active/accepted `crates/server/src/question_library.rs` boundary and its tests. | Ignored browser codec matrix proves canonical output, accepted closed human-input normalization, and invalid-form failure before request; remove. No decoder snapshot is permanent. |
| C845 | contributor; 0; sweep only owned executable fixtures, seed data, docs examples used as executable inputs, and server/browser contract fixtures for canonical IDs.                                                                                                                                                                                                                                                                                                                                                                                                            | C843,C844,C358; hands C846; waits for and excludes C358's active/accepted `crates/server/src/question_library.rs` boundary and its tests. | Ignored bounded fixture inventory plus focused fixture load; remove. Textual absence is not a permanent test.                                                                                           |
| C846 | contributor; 0; integrated fresh-schema proof of mint, exact canonical `XXXX-ZXXX` storage/display/serde, closed human-input normalization before checksum validation and lookup, invalid-ID denial, shared Question/Pool namespace, and global registry reservation. | C369,C844,C845; hands C319,C342,C343,C354,C885. | Ignored fresh-PG17/server/browser proof; repair the named failed boundary and rerun. Retain only one stable public canonical-ID contract if it passes every `PYTEST_STYLE.md` criterion. |

### C847-C851: C209 retention-notification delivery correction

The notification boundary implements C209's existing warnings and retention
processing; it does not define Course dates or retention policy. Its one
identity is `(course_id, action_kind, due_at, recipient_account_id)`, with
`action_kind` exactly `warn_inactive` or `notify_archive`; archive/delete
never notice. The deduplicated recipient union is assigned Instructor plus
active Instructor memberships/accounts, and every claim derives the current
eligible recipient and current verified destination rather than storing an
address snapshot or exposing a generic lookup. A provider failure never
blocks archive/delete. Invitation export, Mail.app, generic outbound email,
and fake success are prohibited. Operational provider credentials remain
configuration; Live Demo `NotConfigured` is not delivery evidence, and the
boundary makes no provider exactly-once promise.

| ID   | Kind; occurrence ownership; one boundary and outcome                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Prerequisites / handoff                                                                                                                        | Exact focused gate and test lifetime                                                                                                                                                                                                                                                                                                                                  |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C847 | contributor; 0; late `schemas/base_schema/50_functions/course_retention_notifications.sql` owns receipt/lease state and a new NOLOGIN notifier capability. It permits only `warn_inactive`/`notify_archive`, enforces the identity, and on every claim derives the current eligible recipient and current verified Instructor destination from the deduplicated union. It stores no address snapshot and exposes no generic lookup. DB-owned `next_attempt_at` starts at `due_at`; one evaluated-at claim requires action still due, next attempt due, no acceptance, and absent/expired lease, orders `(due_at,id)`, increments attempts, and advances next attempt to lease expiry with `SKIP LOCKED`. The receipt persists an idempotency key before provider invocation and provider acceptance. | C206; current Course membership/assignment facts; hands C848,C849.                                                                             | One temporary fresh-PG17 proof covers current eligibility/address, deduplication, identity/order, lease/crash reuse, terminal acceptance/no resend, failure nonblocking, and exact least privilege; remove. No new permanent test.                                                                                                                                    |
| C848 | contributor; 0; typed Learning Data Access notification Store exposes only claim, provider acceptance, and unaccepted-attempt failure receipt operations through C847's notifier capability. Acceptance is terminal for sending. A successful failure clears its lease and sets `next_attempt_at = failed_at + min(3600 seconds, 60 seconds * 2^(attempt_count - 1))`; accepted receipts never resend and an action no longer due cannot retry.                                                                                                                                                                                                                                                                                                                                         | C847; hands C850,C851. No invitation, Account search, Student Work, session, object, renderer API, delivery callback, or inbox-delivery state. | The C847 temporary proof covers the typed Store and capability denial; remove.                                                                                                                                                                                                                                                                                        |
| C849 | contributor; 0; server-only provider-neutral `CourseRetentionNotificationDelivery` with disabled `NotConfigured` adapter sends only a fixed redacted sign-in-only message: no Course identifier/title, raw ID, FERPA data, capability, or recovery link. It records provider acceptance only. `NotConfigured` records non-send failure, sends nothing, and never reports fake success.                                                                                                                                                                                                                                                                                                                                                                                                  | C847; hands C850,C851. Provider credentials are operational configuration; Live Demo `NotConfigured` is not delivery evidence.                 | The C847 temporary proof covers redaction, disabled recorded failure/no-send, and terminal provider acceptance. The boundary has no callback or inbox-delivery guarantee; remove.                                                                                                                                                                                     |
| C850 | contributor; 0; one isolated retention process has exactly two independently attested, non-inheriting database profiles/pools: C215 retention executor and C848 notifier. It has no third database authority and no API listener, S3/object-store, session, or renderer authority.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | C847,C848,C849; hands C851.                                                                                                                    | Ignored failed-access/two-pool attestation proof; remove. Static configuration/file inventories are never permanent.                                                                                                                                                                                                                                                  |
| C851 | contributor; 0; C209 worker orchestration, after C215 plus C847-C850, attempts required earlier notices in C847 `(due_at,id)` order, then executes archive/delete after a successfully recorded pre-acceptance failure. It stops only the notice lane for an unknown typed Store state and always continues retention transitions. Crash before send waits for lease expiry; crash after provider invocation uses the durable idempotency key on eligible reclaim. It never calculates dates or policy.                                                                                                                                                                                                                                                                                 | C215,C848,C849,C850; hands C209.                                                                                                               | The C847 temporary proof covers evaluated-at claim, due order, lease/crash reclaim, acceptance no-resend, deterministic failure backoff through cap, action-no-longer-due exclusion, repeated-run idempotency, recorded-failure nonblocking transition, and unknown Store state notice-lane stop with continued retention transitions; remove. No new permanent test. |

### C852-C855: C371 Verified Instructor Display Name correction

Human Guidance requires the active vetted Instructor viewer to see which
vetted Instructors starred a Published Question. The approved name is a
bounded, server-controlled verification attribute, not Profile/directory data.
C371 remains open until the whole chain below delivers exact names; a self-Star
or count-only route is contributor evidence only.

| ID   | Kind; occurrence ownership; one boundary and outcome                                                                                                                                                                                                          | Prerequisites / handoff                  | Exact focused gate and test lifetime                                                                                                                                                                                                                                                                                               |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C852 | contributor; 0; real C17/C18 identity-vetting/Account-creation storage writes one bounded Verified Instructor Display Name. No self edit, Profile surface, directory, or general Account projection exists.                                                   | C17,C18; hands C853.                     | Ignored vetting/creation fixture proves only that write path and rejects other projections; remove. A stable write-authorization test is a candidate only after `PYTEST_STYLE.md` review.                                                                                                                                          |
| C853 | contributor; 0; Star SQL, typed LDA, and server projection disclose exact Verified Instructor Display Names only to an active Instructor viewing a Published Question Star list; no email, UUID, Account reference, avatar, Course, or substitute identifier. | C370,C852; hands C854.                   | Retain one small real-session authorization/privacy outcome test only if it passes every `PYTEST_STYLE.md` criterion: active Instructor gets exact names; Student, anonymous, inactive, and non-Published/non-Star-list paths do not. Failure repairs predicate/fields. All other multi-identity fixtures are ignored and removed. |
| C854 | contributor; 0; frontend renders only C853's exact server names in the Published Question Star list, with no client lookup, Profile link, or substitute identity.                                                                                             | C853; hands C855.                        | Ignored render/accessibility fixture; remove. Component snapshots are never permanent.                                                                                                                                                                                                                                             |
| C855 | contributor; 0; connected proof covers C17/C18 -> C852 -> C370/C853 -> C854 exact-name delivery before C371 can close.                                                                                                                                        | C17,C18,C370,C852,C853,C854; hands C371. | Ignored PostgreSQL/server/browser chain fixture; remove after review, retaining only C853's approved privacy outcome test.                                                                                                                                                                                                         |

### C856: Blueprint Star identity-projection correction

Human Guidance requires exact vetted Instructor names on the authorized Public/Archived
Blueprint Star list. C409 is deliberately state/count/Watch fan-out only; it does not disclose
who Starred. C856 owns that single identity occurrence and C423 remains the final browser
closure.

| ID        | Kind; occurrence ownership; one boundary and outcome                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Prerequisites / handoff                                                                                                                          | Exact focused gate and test lifetime                                                                                                                                                                                                                                                                                                                                                                  |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C856      | closure; 1; 102; after C408/C409/C852, Blueprint Star SQL, typed LDA, and server projection return exact Verified Instructor Display Names only to an active vetted Instructor viewing a Public or Archived Blueprint Star list. Return no email, UUID, Account reference, avatar, Course, substitute identifier, or Watch identity/state; no client lookup or Profile link.                                                                                                                                                                                                                                                                                    | C408,C409,C852; hands C423, whose browser result supplies final closure.                                                                         | Retain one small real-session authorization/privacy outcome test only if it passes every `PYTEST_STYLE.md` criterion: authorized active vetted Instructor gets exact names; all other roles, visibility states, and Watch paths do not. Use ignored multi-identity/browser fixtures for the full matrix, then remove them. Failure repairs the projection predicate/fields, never a fixture snapshot. |
| C857      | contributor; 0; adapter/model/persistence answer-free `AuthorContentPresentation` carries only validated author-script source and closed reviewed library IDs through issuance, reproduction, and checksum. It excludes Answer Key, feedback correctness, grading input/output, seed/generated-parameter hash, response bindings, session/capability, Account/Course/Attempt metadata, and arbitrary URL; raw source is never a generic browser DTO.                                                                                                                                                                                                            | C302; hands C858.                                                                                                                                | Ignored permitted/excluded-field adapter/model/persistence/issuance/reproduction/checksum matrix; remove. No serialization snapshot is permanent.                                                                                                                                                                                                                                                     |
| C858      | contributor; 0; dedicated authenticated `no-store` HTML document route authorizes exact Student/position ownership and reproduces current reviewed author content. Safely encode source, never concatenate; no raw-source browser API/save/submit/grading/object-store/general API. Its CSP permits only the C901 exact local WASM `connect-src`, current registry JS SRI hash, and server bootstrap nonce; no author URL or broad `'self'` source.                                                                                                                                                                                                             | C857,C901; hands C859.                                                                                                                           | Ignored route/header/authorization matrix proves every header/CSP exclusion, current SRI/`locateFile`, and denied route; remove. A small real-session access/isolation outcome is a candidate only after every `PYTEST_STYLE.md` question passes.                                                                                                                                                     |
| C859      | closure; 5; AuthorContentFrame consumes only typed optional frame reference/availability. Exact iframe is `sandbox="allow-scripts" referrerpolicy="no-referrer" allow=""`, with no same-origin/forms/popups/downloads/modals/top navigation/pointer lock/storage access/permissions. No parent init/message; optional outbound resize only is versioned `author-content.resize`, finite integer/clamped, and accepted only from `event.source === iframe.contentWindow`; no answer/URL/HTML/navigation/storage/API command.                                                                                                                                     | C858; hands C860.                                                                                                                                | Ignored security browser matrix proves frame/CSP confinement, no privileged parent channel, denied form/navigation/network/API paths, and resize predicate; remove. Retain only an independently justified stable real-session isolation outcome.                                                                                                                                                     |
| C860      | contributor; 0; connected authorized Student-position proof covers C859 rendering/interaction, isolation, grading independence, and C901 reviewed local RDKit manifest before C304.                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | C859,C901; hands C903.                                                                                                                           | Ignored connected server/browser/RDKit-manifest fixture; remove. No renderer orchestration or mock call-order test is permanent.                                                                                                                                                                                                                                                                      |
| C861      | abandoned-Draft cleanup removal owner: delete the prohibited baseline placeholder warning/recovery table, APIs, grants, Store, worker, generated seams, and test seams. Preserve manual C351 own-Draft deletion and publication; do not implement a cleanup feature.                                                                                                                                                                                                                                                                                                                                                                                            | The N/A audit classification and unresolved Draft-cleanup question; independent of C351.                                                         | Ignored bounded removal inventory proves every listed placeholder seam is absent while C351 manual delete/publication remain; remove. A textual absence test is never permanent.                                                                                                                                                                                                                      |
| C862      | contributor; 0; Blueprint lifecycle browser codec/client direct cutover: generated `BlueprintAvailability` is exactly `Private                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Public                                                                                                                                           | Archived`; update only `src/api/decoders/blueprint_course.ts`, client fixtures, and executable generated consumer to reject legacy `available                                                                                                                                                                                                                                                         | archived` aliases. No C50 workspace ownership expands. | C49,C72; hands C50,C19 browser gates. | Ignored canonical/legacy codec matrix; remove. Retain only a stable public lifecycle-decoder contract if every `PYTEST_STYLE.md` criterion supports it. |
| C863-C869 | deferred product work; 0; Human Guidance makes H5P desired but not a current implementation requirement. Detailed H5P runtime design is outside the current goal.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | No current A7 dependency edge while H5P remains in Deferred product behavior.                                     | No implementation or proof until Human Guidance moves H5P out of Deferred product behavior.                                                                                                                                                                                                                                                                                                                                                                                              |
| C870      | complete removal; 0; dormant H5P binding/schema/grant/policy, secondary branch/DTO/API/test, adapter/importer/crate/member, and workspace-import enum/state seams are absent. Retained generic bindings serve delivered backends; no compatibility remains.                                                                                                                                                                                                                                                                                                                                     | Independent of blocked future chain.                                                                                                             | Current non-doc source inventory is empty; Cargo metadata has no `adapter_h5p` workspace member; the archived [changelog](../../CHANGELOG-2026-09i.md) records the removal. It closes no-placeholder only.                                                                                                                                                                                                  |
| C876      | contributor; 0; after C319's install-order audit, `question_lineages.sql` supplies the one immutable exact Published Question Revision read/pin that a fork must attribute. It creates no Draft, authoring operation, client-selected ID, or attribution table in this early install phase.                                                                                                                                                                                                                                                                                                                                                                     | C211,C846,C319; hands C877.                                                                                                                      | Ignored exact source/revision read probe; remove. No source-table or call-order test is permanent.                                                                                                                                                                                                                                                                                                    |
| C877      | contributor; 0; later `question_authoring_operations.sql` supplies one atomic active-Instructor Draft-fork operation and immutable source attribution. Server-only inputs are the C369-allocated canonical ID, resolved source Revision, and actor-bound opaque idempotency key; an `(actor,key)` retry returns its same Draft only for that source and refuses a key/source mismatch.                                                                                                                                                                                                                                                                          | C9,C876; hands C878.                                                                                                                             | Ignored fresh-schema/RLS/concurrent-repeat matrix proves private owner, exact immutable pin, mismatch refusal, and one-Draft outcome; remove. Retain only a narrow stable authorization or idempotency outcome if every `PYTEST_STYLE.md` question supports it.                                                                                                                                       |
| C878      | contributor; 0; typed `QuestionForkStore` and server command resolve the authorized exact Published Revision from the canonical request path, mint the new ID only through C369's server-side random checksum allocator, and invoke C877 once. Request body accepts no Question ID, source, attribution, authorship, or Draft payload. | C369,C877; hands C879. | Ignored Store/server malformed-path, authorization, allocator-collision, retry, and concurrency matrix; remove. No mock call-order test is permanent. |
| C879      | complete closure; 2; active Instructor UI invokes C878 from a Published Question and opens only its returned distinct private Draft, with own authorship, exact immutable source attribution, and no Library admission before C9 publication validation. Publication derives the exact source Revision license. | C9,C878. | Accepted connected two-Instructor PostgreSQL/server/browser proof covered source pin/attribution, private cross-account denial, retry/concurrency, distinct server-issued ID, and prevalidation-publication denial. C843-C846 own the exact canonical-ID proof. Accepted 3-by-3 PostgreSQL proof preserved each compatible CC source license and rejected mismatches through the approved minimal SQL. Temporary probes were removed. |
| C885      | contributor; 0; `question_pools.sql` stores an exact canonical Pool ID reserved through C843's global registry, immutable sequential Pool Revisions, and their ordered exact Published Question Revision members. Its trusted create/append operations accept a server-issued typed ID only at Revision 1, no browser/client grant, and are neither allocator nor unused coordinator. | C312,C313,C846,C354; hands C886,C342,C905. | Ignored uniqueness/revision/member-pin/RLS matrix proves no direct client path; remove. No schema call-shape test is permanent. |
| C886      | contributor; 0; typed `QuestionPoolCreationStore` and active-Instructor server route mint Pool IDs only through C369's random checksum allocator and atomically create Revision 1 from an ordered nonempty distinct list of exact Published Question Revision references plus the Instructor's interchangeability attestation. Browser input supplies only that bounded Pool content/attestation, never an ID, owner, stored revision number, or backend behavior; a registry collision gets a newly issued ID and retries the one creation transaction. | C369,C885; hands C887. | Ignored Store/server authorization, member validation, client-ID refusal, backend-mix, forced collision/retry, and atomic-create matrix; remove. No mock allocator/call-order test is permanent. |
| C887      | closure; 1; authorized Instructor Pool workflow creates a Published reusable Pool with a new canonical `XXXX-ZXXX` ID, Revision 1, ordered pinned Published Question members, and interchangeability attestation through C886. C342 projects that ID; C905 owns Assessment-owned fork provenance and selection evidence. | C886; hands C342,C355,C905. | Ignored connected PostgreSQL/server/browser proof covers active-Instructor creation, non-Instructor/client-ID denial, registry collision retry, globally unique canonical ID, Revision 1, exact member pins, and C342 handoff. Retain only a narrow real-session authorization or issuance outcome if every `PYTEST_STYLE.md` criterion passes. |
| C893      | contributor; 0; implementation has two bounded routes. Current read is `POST /api/questions/bulk-metadata/current` with only `questionIds`; `question_library/shared_metadata.rs` `load_current_shared_metadata` returns no-store current metadata. Write accepts only selection, each metadata Edit Number, and the closed patch; `question_bulk_metadata.rs` `bulk_replace_metadata` owns whole CAS success, stale, invalid, and inaccessible outcomes. No idempotency key, digest, receipt, coordinator, queue, partial result, or arbitrary field patch exists.                                                                                             | C365,C367,C338; hands C366,C368.                                                                                                                 | Fresh PostgreSQL 17 SQL/API proof established database behavior. Accepted isolated actual-server HTTP and private exact-main browser proof now covered the current read, replace/clear, search projection, all-or-none stale 412, no automatic browser retry, authorized nonowner Public Published access, and anonymous/Student denial. C893 contributes to C368; C366's scale gate remains open.    |
| C904      | engineering decision; 0; use the existing positive `selection_count` on the Assessment-owned Pool entry/fork. The reusable immutable Pool Revision owns exact members; no Pool default or Assessment override mechanism is added. This is the simplest existing architecture consistent with HG, not a claim that HG mandates field placement.                                                                                                                                                                                                                                                                                                                  | `crates/question_model/src/assignment.rs`; `schemas/base_schema/50_functions/assessments.sql`; imported Pool fork belongs to its Assessment. Hands C905-C909. | Source audit records existing positive per-entry count; no new code or permanent test.                                                                                                                                                                                                                                                                                                                |
| C905      | contributor; 0; retain the Assessment-owned fork's exact reusable Pool ID and immutable Revision, and validate its positive `selection_count` is no greater than the exact fork member count. It adds no Pool default or override.                                                                                                                                                                                                                                                                                                                                                                                                                              | C313,C885,C887,C904; hands C906.                                                                                                                 | Ignored fresh-schema validation/provenance matrix; remove. No permanent inventory test.                                                                                                                                                                                                                                                                                                               |
| C906      | contributor; 0; carry the Assessment-owned fork's exact Pool ID/Revision and `selection_count` through typed selection inputs without client-selected revision, backend behavior, or arbitrary selection policy.                                                                                                                                                                                                                                                                                                                                                                                                                                                | C905; hands C353,C907.                                                                                                                           | Ignored typed authorization/provenance codec matrix; remove. No Store call-order test is permanent.                                                                                                                                                                                                                                                                                                   |
| C907      | contributor; 0; authorized Assessment operation writes the existing per-entry `selection_count`, resolves its exact owned fork Pool ID/Revision, calls C353's backend-neutral selection, and C315 persists that exact fork Pool ID/Revision with the resulting exact selected Question Revisions.                                                                                                                                                                                                                                                                                                                                                               | C353,C315,C906,C887; hands C908.                                                                                                                 | Ignored server/Assessment authorization, count-bound, fresh-Attempt, resume, and Pool/Question provenance persistence matrix; remove. No orchestration test is permanent.                                                                                                                                                                                                                             |
| C908      | contributor; 0; Instructor Assessment editor and typed browser client submit only the Assessment entry's positive `selection_count`; the server derives the exact owned fork Pool ID/Revision and returns a whole validation outcome. They cannot provide selected Question IDs, Pool ID issuance, backend behavior, or a default/override.                                                                                                                                                                                                                                                                                                                     | C907; hands C909.                                                                                                                                | Ignored connected client/editor boundary proof; remove. No component snapshot is permanent.                                                                                                                                                                                                                                                                                                           |
| C909      | closure; 5; after C908, connected Instructor/Student proof closes C355's five Pool behaviors: Assessment-owned count configuration, new-Attempt backend-neutral selection from exact fork Pool Revision, resume preservation, and exact Pool/Question Revision evidence.                                                                                                                                                                                                                                                                                                                                                                                        | C355,C907,C908; hands C314,C334.                                                                                                                 | Ignored fresh schema/server/browser proof; remove. Retain only a narrow stable selection-evidence or authorization outcome if every `PYTEST_STYLE.md` criterion passes.                                                                                                                                                                                                                               |
| C899      | contributor; 0; direct-cutover removal of author-supplied `externalDependencies[].{id,cdnUrl,localPath}` from the strict native source shape. Only C302's closed `libraries` enum may request a runtime; C305 is reclassified false-closure evidence.                                                                                                                                                                                                                                                                                                                                                                                                           | C301,C302; hands C900.                                                                                                                           | Ignored legacy-declaration rejection/closed-`rdkit` acceptance matrix; remove. No absence or serialization snapshot test is permanent.                                                                                                                                                                                                                                                                |
| C900      | contributor; 0; current official RDKit supply-chain manifest and deterministic vendoring command verify provenance, BSD-3-Clause, npm integrity, license hash, and exactly the current approved JS/WASM hashes; generated server registry consumes no resolver/CDN or historical catalog.                                                                                                                                                                                                                                                                                                                                                                       | C899; hands C901,C902.                                                                                                                           | Ignored current-release/tamper/extra-file/reproducibility matrix; remove. No manifest snapshot is permanent.                                                                                                                                                                                                                                                                                          |
| C901      | contributor; 0; current generated registry exposes only exact local JS/WASM GET/HEAD routes with MIME, nosniff, CORP, revalidating cache, SRI, `locateFile`, and the sole anonymous `Access-Control-Allow-Origin: *` exception required by the opaque sandbox WASM fetch. C858 permits `'unsafe-eval'` only in that opaque document for the current official RDKit loader; it is not a main-PLE CSP allowance. No caller path, credential grant, redirect, cookie-sensitive response, or object-store URL.                                                                                                                                                      | C900; hands C858,C860,C903.                                                                                                                      | Ignored route/header/method/traversal/mismatch/opaque-frame matrix; remove. Retain only an independently justified stable outcome.                                                                                                                                                                                                                                                                    |
| C902      | contributor; 0; reviewed current-dependency refresh resolves the latest official release after provenance/license/integrity/file-hash/browser review and frame proof, directly replacing the pre-production local runtime with no history or retirement workflow.                                                                                                                                                                                                                                                                                                                                                                                               | C900; hands C903.                                                                                                                                | Ignored current-refresh/rejection/browser matrix; remove. No release orchestration test is permanent.                                                                                                                                                                                                                                                                                                 |
| C903      | closure; 3; authorized Student delivery proves `rdkit` yields only reviewed current C901 local JS/WASM, isolated chemistry rendering, no CDN/API fallback, exact CSP/SRI/`locateFile` confinement, and unchanged server grading.                                                                                                                                                                                                                                                                                                                                                                                                                                | C858,C859,C860,C900,C901,C902; hands C304.                                                                                                       | Ignored connected offline/network-denial server/browser proof; remove. Promote only one small stable local-runtime/security outcome if every `PYTEST_STYLE.md` criterion approves it.                                                                                                                                                                                                                 |

### C871-C875: Question Watch thread and impact notices

C346 remains intentionally partial. The lifecycles are settled: active vetted Instructors create,
read, and reply to text-only improvement threads and edit only their own posts. Question owners and
Sysadmins administer Question thread state and impact notices; Pool administration is Sysadmin-only.
Threads record their creation Revision, impact notices may name an affected Revision, and resolved
or cancelled records remain visible. Sysadmins use a read-only Library mode outside these explicit
administration actions. C871-C873 source is implemented but awaits final review and major-milestone
browser proof. Fresh PostgreSQL 17 actual-role proof closes the C874/C875 SQL delivery boundary for
all four event kinds; connected multi-identity controls and notification presentation remain open.

| ID   | Kind; occurrences; one boundary and outcome                                                                                                                                                                                        | Prerequisites / handoff                       | Exact focused gate and test lifetime                                                                                                                                                        |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C871 | implemented source contributor; 0; retained text-only threads target stable Question/Pool lineages, record the creation Revision, accept vetted-Instructor participation, permit own-post edits, and retain open/resolved records. | Hands C873,C874; final review/proof pending. | Major-milestone fresh-schema/LDA authorization matrix; remove temporary proof. Retain no inventory test. |
| C872 | implemented source contributor; 0; retained impact notices target stable Question/Pool lineages, may name an affected Revision, and preserve active/cancelled current records under Question-owner/Sysadmin or Pool-Sysadmin administration. | Hands C873,C874; final review/proof pending. | Major-milestone fresh-schema/LDA lifecycle matrix; remove temporary proof. |
| C873 | implemented source contributor; 0; server/API/browser surfaces expose vetted-Instructor threads, explicit owner/Sysadmin administration, and Sysadmin read-only Library content controls. | C871,C872; hands C875; final review/proof pending. | Major-milestone authorization/browser workflow proof; remove temporary probe. No component snapshot. |
| C874 | closed SQL contributor; 0; source-bound outbox writers emit exactly revision, fork, improvement-thread, and impact-notice events with no generic notification hook. | C346,C871,C872; hands C875. | Fresh PostgreSQL 17 actual-role proof passed. Keep connected control and presentation proof separate. |
| C875 | open application closure; 1; private in-app Watch delivery covers all four event classes without exposing Watch identities to Students, anonymous users, or unauthorized Instructors. | C373,C873,C874. | SQL delivery is closed. Major-milestone connected multi-identity/browser proof remains required; remove temporary probe. Retain only an earned stable private-notification authorization outcome. |

### A7 coordination notes

Use the current table rows as the dispatch source. Each row names its owner, affected boundary,
prerequisites, and proof. Before an edit to a shared schema, route, or model boundary, the manager
confirms the named predecessor has handed off and assigns one writer for that boundary. This is
enough coordination to prevent conflicts; do not build or maintain a separate dependency parser,
count checker, or ownership ledger.

Important sequences are: the canonical author-content/RDKit path (C899 through C903 before C304),
the native reproduction path (C825 through C831), Question IDs before dependent publication,
fork, and Pool creation work, and Pool selection only after C904's engineering decision is recorded. The individual rows
remain the authority for their exact prerequisite and proof.

### C400-C425: Complete A8 Courses gaps

Scope: the open A8 bullets in `08_courses.md`. Audit lines retain verbatim HG and mismatch
evidence. Later duplicate references point to their owning behavior rather than creating work.

A contract-only A8 row flips **no** checklist bullet. Its named recipient is the sole writer of the shared implementation boundary, and only the recipient gate plus the A8 temporary evidence allows the listed lines to close.

## Atomic work and exact proof lifecycle

Every A8 temporary path below is ignored, run directly, and removed at that row's handoff. None is durable: each proves a one-time implementation integration; permanent coverage requires a separate PYTEST_STYLE decision.

| ID   | owning lines                                                                                                                                                                                                                                                                                                                                                                                                                                             | A8 ownership / exact source                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | recipient and close condition                                                                                                                                                                                                                                                                                    | temporary proof; focused recipient gate                                                                                                                                                                                                                                                                                                                            |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C400 | 3,5,7                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Contract only: distinct Blueprint/Instance aggregate, using course_core.sql ple_data.course_instance and blueprints.sql ple_data.blueprint_course as read evidence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | C6 writes Blueprint boundary; C7 writes Course boundary. No flip until both reports agree.                                                                                                                                                                                                                       | tests/_temp/hg_a8_c400_aggregate_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c400_aggregate_probe.py; recipients run source source_me.sh && cargo test -p server_core blueprint_course course_instance; remove after C7.                                                                                                                            |
| C401 | 11,13,17                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Contract only: equal co-Instructor authority, first membership existence only; evidence course_core.sql assigned_instructor_account_id and course_instance.rs CourseInstanceView.is_assigned_instructor.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C7 sole writer for membership/procedure; A2 supplies role rule. No flip until C7 authorization gate.                                                                                                                                                                                                             | tests/_temp/hg_a8_c401_membership_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c401_membership_probe.py; source source_me.sh && cargo test -p server_core course_instance; remove after C7.                                                                                                                                                          |
| C402 | 9                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Contract only: explicit adopted-or-empty input; evidence CreateCourseInstanceInput and ple_api.create_course_instance.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C7 sole writer for Course root/state; C503 supplies tagged direct-versus-adopted Assessment origin before C7 closes started-empty authoring/delivery. No flip until both gates.                                                                                                                                  | tests/_temp/hg_a8_c402_creation_shape_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c402_creation_shape_probe.py; source source_me.sh && cargo test -p server_core course_instance; remove after C7/C503.                                                                                                                                             |
| C403 | 39,41                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Contract only: Private/Public/Archived and default Private; evidence blueprints.sql availability.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | C49 sole lifecycle-schema writer. No flip until C49 SQL/service gate.                                                                                                                                                                                                                                            | tests/_temp/hg_a8_c403_lifecycle_contract_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c403_lifecycle_contract_probe.py; bash tests/e2e/e2e_live_demo_blueprint_course.sh --service; remove after C49.                                                                                                                                               |
| C404 | 43,45,52,60,62,64                                                                                                                                                                                                                                                                                                                                                                                                                                        | Contract only: Private owner/no-adopt, Archived read-only/no-adopt, restore Public, archived fork permission; evidence transition_availability/restore_blueprint.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | C72 sole server-transition writer after C49. No flip until C72 gate.                                                                                                                                                                                                                                             | tests/_temp/hg_a8_c404_transition_contract_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c404_transition_contract_probe.py; source source_me.sh && cargo test -p server_core blueprint_course; bash tests/e2e/e2e_live_demo_blueprint_course.sh --service; remove after C72.                                                                          |
| C405 | 30,50,54,56                                                                                                                                                                                                                                                                                                                                                                                                                                              | Contract only: vetted Public/Archived browse, normal archive exclusion, explicit inclusion, Public-only adoption.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | C19 sole browse-route writer and C47 sole search UI writer after C72. No flip until both gates.                                                                                                                                                                                                                  | tests/_temp/hg_a8_c405_discovery_contract_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c405_discovery_contract_probe.py; C19: bash tests/e2e/e2e_live_demo_blueprint_course.sh --service; C47 under lease: npx playwright test tests/_temp/hg_a8_c405_discovery_ui.spec.ts; remove after C47.                                                        |
| C406 | 32,165                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Contract only: one ordered published Question/Pool predicate; evidence ReplaceBlueprintCourseContentInput and Blueprint save validator.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | C73 sole adoption-default/persistence writer after C6. No flip until C73 gate.                                                                                                                                                                                                                                   | tests/_temp/hg_a8_c406_published_content_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c406_published_content_probe.py; bash tests/e2e/e2e_live_demo_course_instance.sh --authority; remove after C73.                                                                                                                                                |
| C407 | 188,217                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Contract only: adoption invokes predicate and copies ordered eligible content/settings; evidence creation_assignments and initialize_course_assignments.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C52 sole adoption server-projection writer after C73. No flip until C52 gate.                                                                                                                                                                                                                                    | tests/_temp/hg_a8_c407_adoption_contract_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c407_adoption_contract_probe.py; bash tests/e2e/e2e_live_demo_course_instance.sh --authority; remove after C52.                                                                                                                                                |
| C408 | 92,98,104                                                                                                                                                                                                                                                                                                                                                                                                                                                | A8 sole writer: new schemas/base_schema/50_functions/blueprint_stewardship.sql and learning-data-access BlueprintStewardshipStore. Course-identity records span Revisions; Watch is private.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | A8 store implementation; backend result is contributor only until C423 browser workflow succeeds.                                                                                                                                                                                                                | tests/_temp/hg_a8_c408_stewardship_schema_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c408_stewardship_schema_probe.py; source source_me.sh && cargo test -p learning-data-access --test blueprint_course_postgres; remove after C423.                                                                                                              |
| C409 | 94,96,100                                                                                                                                                                                                                                                                                                                                                                                                                                                | A8 sole writer: stewardship store service and new in-app projection for Star/unstar/count, private self Watch/unwatch, and Revision/publish/archive/restore Watch fan-out. It returns no Starred-by names; fork/adopt create no Star/Watch.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | C408; server/store result is contributor only until C423 browser workflow succeeds.                                                                                                                                                                                                                              | tests/_temp/hg_a8_c409_stewardship_service_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c409_stewardship_service_probe.py; source source_me.sh && cargo test -p server_core blueprint_course; remove after C423.                                                                                                                                     |
| C410 | 113,119,121,123,150                                                                                                                                                                                                                                                                                                                                                                                                                                      | Closed: one authorized current-parent Course summary derives adopted-Assessment changed, matching, removed-source, Type-mismatch, and automatically-added correspondences without a persisted offer, receipt, comparison baseline, new update table, or new Store. It excludes direct Assessments.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | C52 adoption projection. Per-Assessment Apply uses `expectedSourceRevision` plus daughter Edit CAS; trusted Pool materialization preserves dates, status, origin pins, and Student Work. No whole-Course lifecycle milestone closes.                                                                             | Accepted actual-server proof: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`; prior detail-only proof: `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`.                                                                                                                                                                                 |
| C411 | 115,117,227                                                                                                                                                                                                                                                                                                                                                                                                                                              | Closed: the lazy Course summary reuses the existing Assessment pages for explicit detail Review and Apply, with clear removed-source and Type-mismatch cannot-apply results. It adds no implicit deletion or type conversion.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | C410. Accepted desktop and narrow browser proof covers lazy open/reopen, detail Cancel, explicit Apply, and refresh to match. No whole-Course lifecycle milestone closes.                                                                                                                                        | Accepted compiled-main browser proof: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.                                                                                                                                                                                                                                                                |
| C412 | 128,130,132,134,138                                                                                                                                                                                                                                                                                                                                                                                                                                      | A8 sole writer: new schemas/base_schema/50_functions/blueprint_lineage.sql and BlueprintLineageStore contract for the fork's immutable source Course/Revision ancestry.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | C51 sole primary fork route/service writer. C880-C884 add comparison/review/selective-save behavior; C412 closes no UI behavior by itself.                                                                                                                                                                       | tests/_temp/hg_a8_c412_lineage_contract_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c412_lineage_contract_probe.py; source source_me.sh && cargo test -p server_core blueprint_course; remove after C884/C413.                                                                                                                                      |
| C413 | 136                                                                                                                                                                                                                                                                                                                                                                                                                                                      | UI only, new src/features/blueprint_forks/; after C882 and the ordinary-visibility known-forks reader (visible child Reference/current name/current Revision/verified owner display name), its read-only discovery/known-forks/current-comparison slice may proceed. Apply controls consume C883; complete closure still waits C884. It exposes an authorized fork action for Public and Archived Blueprints and deliberately applies an explicit selection that may contain several related changes.                                                                                                                                                                                                                                                                                                     | C47,C50,C884. The known-forks reader returns no Private child or owner identity; C412's five fork backend bullets close only after this leased workflow. Question-level hunk selection is optional and is not a prerequisite.                                                                                    | tests/_temp/hg_a8_c413_fork_ui_probe.py and tests/_temp/hg_a8_c413_fork_actions.spec.ts; source source_me.sh && python3 tests/_temp/hg_a8_c413_fork_ui_probe.py; lease: npx playwright test tests/_temp/hg_a8_c413_fork_actions.spec.ts; remove both after browser gate; PYTEST_STYLE: browser workflow is one-time evidence, not a permanent test.                |
| C414 | 155,161,175                                                                                                                                                                                                                                                                                                                                                                                                                                              | A8 sole writer: new question_model blueprint_course/canonical_exchange.rs export DTO; relational data stays primary.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | C73 supplies the implemented published-content predicate.                                                                                                                                                                                                                                                        | tests/_temp/hg_a8_c414_export_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c414_export_probe.py; source source_me.sh && cargo test -p question_model blueprint_course; remove after C415.                                                                                                                                                            |
| C415 | 157,159,169                                                                                                                                                                                                                                                                                                                                                                                                                                              | Closed SQL: canonical export/import uses the strict authority-free representation. Import makes a distinct actor-owned Private root at Revision 1, reproduces ordered content/structure, preserves no source identity/owner/Star/Watch, and never overwrites the source.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | C414, accepted C425 canonical HTTPS browser round trip, and retained PostgreSQL 17 actual-role semantic round trip.                                                                                                                                                                                              | `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` passed authorized export/import/re-export deep equality; fresh Blueprint/module/Assessment/Pool identities; exact fixed and ordered Pool-member pins; unchanged source; and unauthorized denial. |
| C416 | 140,142,144,146,148,171                                                                                                                                                                                                                                                                                                                                                                                                                                  | A8 sole writer: new schemas/base_schema/50_functions/blueprint_change_proposals.sql and ChangeProposalStore/API. Receiver accepts subset; current Revision plus ETag checks stale/conflict in one transaction; acceptance creates one receiver Revision then calls C410.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | C401,C410,C415. It cannot close until C421 browser success.                                                                                                                                                                                                                                                      | tests/_temp/hg_a8_c416_proposal_service_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c416_proposal_service_probe.py; source source_me.sh && cargo test -p learning-data-access --test blueprint_course_postgres; remove after C421.                                                                                                                  |
| C417 | 182,184,186                                                                                                                                                                                                                                                                                                                                                                                                                                              | Lines 182/184's Public Blueprint adoption and Empty Instructor creation are verified by accepted private actual HTTP/exact-main browser proof. Adopted Course read showed one Unreleased Practice Assessment, exact origin Revision, finite Attempt limit, fixed Question Revision/points, and dates unset. A follow-up actual-main Empty Course created a direct-origin Practice Assessment, added/saved one available fixed Question pin, set Due/time limit, passed readiness, and released it. Real Student roster import/claim showed Released visibility/pre-start facts, and a separate native fixed-Question HTTP proof saved/submitted a graded 1/1 Attempt then issued another unlimited Practice Attempt. Line 186's complete Student/delivery settings and Pool behavior remain under review. | C7 sole Course root/state writer; C503 tagged origin and one direct native fixed-Question path are observed, but remaining Pool/full Student browser delivery is required; C49/C72 require Public-only adoption; C52 supplies adoption projection. Two creation subbullets closed, not C417 completion.          | Focused strict Rust/Node wire tests and accepted private Empty/Adopted/direct-author/release, member-visibility, and native Attempt HTTP/browser artifacts; future Pool/full Student delivery/grading proof before remaining closure.                                                                                                                              |
| C418 | 192,194                                                                                                                                                                                                                                                                                                                                                                                                                                                  | MarkInactive source/worker path exists. Restricted-role, concurrency, retained-data, and canonical browser acceptance remain open.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | C206/C207/C208 remain the retention writers; C53 owns the term cap; C46 owns Instructor presentation. The source/worker path does not close the open acceptance.                                                                                                                                                    | `crates/learning-data-access/src/retention.rs`, `crates/learning-data-access/src/postgres/retention.rs`, `schemas/base_schema/50_functions/course_retention.sql`, `schemas/base_schema/50_functions/course_retention_transitions.sql`, and `crates/server/src/course_retention_worker.rs`. |
| C419 | 197                                                                                                                                                                                                                                                                                                                                                                                                                                                      | SQL PASS; backend contributor: `schemas/base_schema/50_functions/course_blueprint_publication.sql` `ple_api.create_blueprint_from_course_instance` and `CourseBlueprintPublicationStore` create a distinct actor-owned Private Revision 1 from the locked reusable Course structure, exclude Student/delivery state, fork Pools with exact ordered pins, and record immutable source provenance without mutating the Course. Empty Courses and empty Assessments retain their minimal exact shapes.                                                                                                                                                                                                                                                                                                    | C6 Blueprint boundary, C7 Instance read contract, C406 published predicate, C417 create/delivery. PostgreSQL 17 actual-role proof covers denial/no-write, stale rollback, replay, source preservation, first-Adoption/student counts, and lifecycle rollback. C420 browser UI is accepted. | `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` passed; focused Rust lower-bound and empty-shape tests also passed.                                                                                                                                                                                                                                                           |
| C420 | 34; complete                                                                                                                                                                                                                                                                                                                                                                                                                                             | `src/pages/course_instance_page.tsx` Course tools opens a compact metadata-only dialog, preserves an uncertain-retry idempotency key, and opens the strict new Private Revision-1 receipt without retyping or mutating the source Course.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | C48/C55 own editor shells. Accepted canonical HTTPS proof used one child-route POST to create `BPJD8H28` from `CI0QR41X`; it showed Private Revision 1 and one Adoption, counted source Students only in the statistic, copied no roster/delivery state, and left the source unchanged. | The authenticated temporary browser probe was removed after acceptance. Focused Node transport/decoder, TypeScript, ESLint, and Prettier passed.                                                                                                                                                                                                                                                                          |
| C421 | contributor only                                                                                                                                                                                                                                                                                                                                                                                                                                         | UI only, new src/features/blueprint_change_proposals/; C416 submit/accept/read API.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | C48 owns editor shell. C416's 6 bullet closure waits for this leased browser success.                                                                                                                                                                                                                            | tests/_temp/hg_a8_c421_proposal_ui_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c421_proposal_ui_probe.py; lease: npx playwright test tests/_temp/hg_a8_c421_proposal_ui.spec.ts; remove after C416 close.                                                                                                                                           |
| C422 | 245                                                                                                                                                                                                                                                                                                                                                                                                                                                      | UI-only compact-name advisory component, no hard schema limit.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | C30 human identity rule; A4 input owner.                                                                                                                                                                                                                                                                         | tests/_temp/hg_a8_c422_short_name_hint_probe.py; source source_me.sh && python3 tests/_temp/hg_a8_c422_short_name_hint_probe.py; lease: npx playwright test tests/_temp/hg_a8_c422_short_name_hint.spec.ts; remove after UI gate.                                                                                                                                  |
| C423 | contributor to C409,C856                                                                                                                                                                                                                                                                                                                                                                                                                                 | UI only, new src/features/blueprint_stewardship/; consumes C408/C409/C856 server results and C47/C48 shells. The active vetted Instructor can Star/Unstar, Watch/Unwatch, see Star count, exact C856 names on Public/Archived Blueprint Star lists, and only their own Watch state.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | C409,C856,C47,C48. Its browser success is required to close lines 94,96,100 through C409 and 102 through C856.                                                                                                                                                                                                   | tests/_temp/hg_a8_c423_stewardship_ui_probe.py and tests/_temp/hg_a8_c423_stewardship.spec.ts; source source_me.sh && python3 tests/_temp/hg_a8_c423_stewardship_ui_probe.py; lease: npx playwright test tests/_temp/hg_a8_c423_stewardship.spec.ts; remove both after C409/C856 close; PYTEST_STYLE: temporary workflow proof only; do not promote a UI snapshot. |
| C424 | contributor to C417                                                                                                                                                                                                                                                                                                                                                                                                                                      | `src/pages/course_list_page.tsx` offers Empty by default or exact Public Blueprint Revision adoption, loading the Blueprint list only for adoption. Accepted private exact-main/actual-server proof closed both line 184's Empty creation and line 182's Public Blueprint creation; the daughter Course displayed its Unreleased Practice Assessment.                                                                                                                                                                                                                                                                                                                                                                                                                                                     | C7, C503, C52, C49, C72, C46, C47, C55. Lines 182/184 are verified; line 186 and full C417 remain open until direct-delivery/complete settings gates.                                                                                                                                                            | Focused strict DTO/Rust tests and accepted private Empty/Adopted HTTP/browser artifacts; no Pool-copy, publishing-workflow, or release/delivery closure claimed.                                                                                                                                                                                                   |
| C425 | contributor to C415; closed                                                                                                                                                                                                                                                                                                                                                                                                                              | The Instructor UI exports the authorized current canonical JSON, strictly validates pasted JSON before import, preserves one idempotency key across uncertain retries, and opens the distinct actor-owned new Private Blueprint.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | C415, C47, C48. Accepted browser success closes lines 157,159,169 through C415.                                                                                                                                                                                                                                  | Accepted disposable canonical HTTPS browser proof covered malformed input, real download, owner/Private/root/Revision-1 import, redirect, ordered deep-equal re-export, and unchanged source; probe removed rather than promoted.                                                                                                                                  |
| C880 | contributor only; OPEN current-authority canonical comparison must relate Blueprint Assessments through shared Question IDs, not internal Assessment IDs, and show shared/added/removed Assessments and Question IDs plus determinable changes through renames, order and structure. Normally compare newest source/fork Revisions; recorded origin is provenance, not a required baseline.                                                              | C412,C414. Preserve prior direct-source/internal-ID projection evidence only for its actual scope. No artificial matching IDs, new persistence or public status enum is required.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Ignored focused Question-ID correspondence and rename/order/structure proof; remove. No permanent classification or fixture inventory.                                                                                                                                                                           |
| C881 | contributor only; OPEN typed ordinary-visibility reads must support any visible related Blueprint Courses in the same fork lineage, not only a direct parent/source pair. Private sides remain owner-only and unavailable sides nonenumerating; fork ownership controls Apply, not comparison.                                                                                                                                                           | C412,C51,C880. Prior exact-current-head/visibility evidence remains valid for direct-source reads only.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Ignored authorization and related-pair proof; remove. Retain only a narrow stable privacy outcome if it earns promotion.                                                                                                                                                                                         |
| C882 | contributor only; OPEN on-request canonical-JSON server review must consume corrected Question-ID Assessment relationships for authorized related pairs, with shared/added/removed content and useful renamed/reordered/restructured comparisons. Prior C882 GET-only/no-store/zero-write direct-source receipt remains bounded evidence.                                                                                                                | C880,C881. No comparison persistence, candidates, digests, receipts or workflow state is required.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Ignored server comparison and nonenumeration proof; remove. No mock call-order test.                                                                                                                                                                                                                             |
| C883 | contributor-verified; selective save accepts an explicit set of reviewed changes, may apply multiple related changes, constructs and validates one coherent complete fork tree, and uses ordinary Revision CAS to create one fork Revision. Selected names use ordinary metadata ETag; stale input changes nothing and no source change applies automatically.                                                                                           | C882; hands C884,C413.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Accepted actual HTTP/pure-domain proof at `/private/tmp/ple-fork-apply-http-artifacts.1e5gy8` covered stale/CAS/coherence/no-auto-overwrite behavior. No permanent test; C883 does not close C884 or C413.                                                                                                       |
| C884 | contributor only; OPEN. The canonical workflow baseline is complete (`/private/tmp/ple-fork-workflow-baseline.log`: PASS); remaining fresh PostgreSQL/server connected Apply and Instructor workflow proof covers Public/Archived review, Private and unauthorized nonenumeration, names, module labels, structure/order, complete Assessment changes, multi-change selection, one coherent Revision save, no automatic overwrite, and stale zero-write. | C883; hands C413.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Residual connected/UI proof remains open; remove any temporary proof after review. Retain only a narrow stable authorization or CAS outcome if all `PYTEST_STYLE.md` criteria approve it.                                                                                                                        |

Owner count: 3+3+1+2+6+4+2+2+3+4+5+3+5+1+3+3+6+3+2+1+1+0+1 = 63 for C400-C425; C856 owns the moved identity occurrence 102, restoring 64 total A8 owners. C400-C425 has 22 closure owners and exactly four contributors: C421, C423, C424, and C425.

## Mandatory handoffs, recipient order, and no shared writers

- C403 -> C49 -> C404 -> C72 -> C405 -> {C19, C47}. C49 alone changes lifecycle schema; C72 alone changes server predicates; C19/C47 implement browse/server and search/UI. C403/C404/C405 are contracts only.
- C406 -> C73 -> C407 -> C52. C73 alone changes adoption persistence; C52 alone changes server projection. C406/C407 are contracts only. C49/C72 are the Public-only eligibility prerequisites for the Adopted branch.
- C412 -> C51; `{C412,C414} -> C880`; `{C412,C51,C880} -> C881`; `{C880,C881} -> C882 -> C883 -> C884`; and `{C47,C50,C884} -> C413`. C412 owns immutable ancestry only; C880 owns the canonical Revision comparison projection; C881 owns authorized reads; C882 owns server review; C883 owns the coherent selective save; C884 owns connected proof; C413 alone owns the Instructor workflow.

Latest-authority identity reconciliation (2026-09-16): Assessment IDs are local to their Blueprint
Course, forks receive fresh Assessment IDs, and there is no persistent cross-Blueprint Assessment
lineage/history. Shared Question IDs alone infer relationships; disjoint Question sets remain
unmatched. C51 fork creation and C883 selective Apply must be audited/reconciled where prior code
or proof assumes cross-fork internal-ID equality. C883's previous contributor evidence is
historical for that assumption, not corrected-contract closure. C413 still owes history and
obvious newer-source/downstream indications in addition to connected corrected comparison/Apply.

- C6 receives C400/C403/C406/C419 reusable boundary contracts. C7 receives C400/C401/C402/C417/C418 Course lifecycle contracts, and C503 hands it tagged direct-versus-adopted Assessment origin before C7's full started-empty closure. C49/C72 gate only the Adopted branch's Public eligibility. Neither A8 contract row edits their shared schema files.
- C30 receives C422 and title/reference findings. C46 receives C411/C418/C424 presentation contract; C47 receives C405/C413/C423/C424/C425; C48 receives C420/C421/C423/C425; C49/C50 receive C403/C404/C413; C51 receives C412; C52 receives C407/C424; C53 receives C418; C72 receives C404; C73 receives C406/C407; C206 receives C418; C216 receives title/reference inventory; C55 receives C411/C420/C421/C424 and all Course/Blueprint editor identity findings.

## A8 title/reference temporary inventory

### A4-06 - current owner: Course specifications / Part 08; Blueprint Private/Public/Archived state machine (A8 schema/server lifecycle boundary; 8)

#### A4-06.1

- HG bullet: "Blueprint Courses follow the lifecycle **Private -> Public -> Archived**."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

#### A4-06.2

- HG bullet: "New and forked Blueprint Courses start **Private**."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

#### A4-06.3

- HG bullet: "Private Blueprint Courses are visible only to their owner."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

#### A4-06.4

- HG bullet: "Instructors may develop and use Private Blueprint Courses without publishing them."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

#### A4-06.5

- HG bullet: "Making a Blueprint Course **Public** adds it to the shared Blueprint Course collection."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

#### A4-06.6

- HG bullet: "A Public Blueprint Course with no adoptions may return to **Private**."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

#### A4-06.7

- HG bullet: "A Public Blueprint Course with one or more adoptions remains **Public**."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

#### A4-06.8

- HG bullet: "Blueprint Courses do not have a separate Draft state."
- Current evidence and concrete mismatch: current Blueprint UI has only available/archive states; it does not model Private/Public, owner-only visibility, shared discovery, adoption-conditioned return, or absence of Draft.
- Owning source area: Blueprint lifecycle: schemas/base_schema/50_functions/blueprints.sql, crates/server/src/blueprint_course.rs, and src/features/blueprint_course/blueprint_course_workspace.tsx.
- Dependencies: C6, C19, C49, and C72.
- Closure owner: C50.
- Verification: lifecycle and authorization tests for every transition, owner/non-owner read, adopted/non-adopted Public course, and rejection of Draft.

### A4-07 - current owner: Course specifications / Part 08; Public Blueprint fork transaction (A8 fork API; 1)

#### A4-07.1

- HG bullet: "Instructors may fork a Public Blueprint Course to continue development privately."
- Current evidence and concrete mismatch: no Public Blueprint fork action or Private fork lifecycle exists.
- Owning source area: Blueprint fork server route: crates/server/src/blueprint_course.rs.
- Dependencies: C72.
- Closure owner: C51.
- Verification: transaction/browser fixture proves source remains Public and editable fork starts Private.

A8-PENDING-title-reference-inventory feeds C216 and is not permanent. C216 is its sole final owner. A8 runs only this ignored probe:

    tests/_temp/hg_a8_title_reference_inventory.mjs
    node tests/_temp/hg_a8_title_reference_inventory.mjs --routes src/routes.ts --contract src/route_contract.ts

Candidate set: src/pages/course_list_page.tsx; src/pages/course_instance_page.tsx; src/pages/blueprint_course_search_page.tsx; src/features/blueprint_course/blueprint_course_workspace.tsx; src/pages/assignment_workspace/assignment_workspace_questions_page.tsx; src/pages/assignment_workspace/assignment_workspace_policies_page.tsx.

It reports an opaque Course/Blueprint/Assessment reference shown without its human title/reference for recognition, copy, or entry. Each finding is handed to C30/C46/C47/C48/C55 or an A8-owned feature; remove the inventory after C216 consumes it.

## A8 coordination notes

Keep lifecycle work ahead of dependent discovery and fork work: C49/C72 establish the Public
eligibility needed by adoption; C412/C414 precede fork comparison and C880-C884; C47/C50/C884
precede the fork-update UI. Contract rows hand off to their named recipient and do not change a
checklist bullet alone. The table and surrounding milestone text identify the exact order when a
shared boundary is involved; no separate DAG artifact is required.

### C500-C536: Complete A9 Assessments gaps

C500-C536 cover the A9 implementation work. C502 is the C216 contributor. `G24`/C525 is open
implementation work at the existing `assessment_submission` and current Student cohort boundaries.
`G35`/C536 has a fixed, seven-column point-only CSV/TSV contract. Its source implementation and
independent review are complete. It remains `[ ]` only for actual-role and canonical-browser
acceptance.

| ID        | A9 owner lines / single boundary                                                                                                                                                                                                                                                                                                              | ignored proof                                                                                                                                                                                                |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C500      | 3,5,7,11,13 one install-safe schema graph: `assignments.sql`, `attempts.sql`, and every post-C870 installed direct SQL consumer, grants, and install inclusion. It waits for C870.                                                                                                                                                            | Regenerate ignored direct-consumer manifest; `source source_me.sh && python3 tests/_temp/hg_a9_g01a_schema/probe.py`; fresh base-schema install; remove both proof artifacts.                                |
| C501      | 9,97,109,119,121,213,215 one compile-safe Rust graph: the three canonical model/domain modules and every post-C857/C870 workspace consumer of their generic symbols. It waits for C857 and C870.                                                                                                                                              | Regenerate ignored direct-consumer manifest; `source source_me.sh && python3 tests/_temp/hg_a9_g01b_domain/probe.py`; `source source_me.sh && cargo test --workspace --no-run`; remove both proof artifacts. |
| C502      | contributor only: A9 title/reference inventory for C216                                                                                                                                                                                                                                                                                       | `node tests/_temp/hg_a9_g01c_title_reference_inventory/inventory.mjs --routes src/routes.ts --contract src/route_contract.ts`                                                                                |
| C503-C527,C529-C536 | 19,23,107,302;21;25;30,32,34,38,102,123;36;40,42,44,46,48,50,61,65;52,57,59;63,67;72,74,76,78,80,82,84,86,88,90,92;114,127,129;111,134,136,138,142,150;140;144,146,148;125;263;164,166,169,171,173;158,160,162,177;175;179,181;183,185,187;54,189,191,193,195;198,200;217,273,275;219,221,223;245,258;252;265;290;295;297;319;322,324 | Exact one-boundary source, temp path, command, observation, failure action, cleanup, and PYTEST_STYLE retention decision are canonical in the active plan's C500-C536 A9 ledger. |
| C528 | 237,250 | **OPEN, decision-blocked.** `FillIn` and `MultiFillIn` define maximum length/count but no required, nonblank, or optional-text semantics. `BackendOwned` is opaque; WeBWorK accepts `[]` and empty values, with no adapter pre-save completeness verdict. Structural completeness is unambiguous for `Numeric`, `SingleChoice`, `MultipleAnswer`, `Matching`, `Ordering`, and `Hotspot`; not for the blocked text/opaque variants. Product must decide raw versus trimmed nonempty versus optional text entries, and whether successful opaque capture itself is complete or an adapter must return trusted completeness. | No C528 implementation, test, or checklist status changes. Defer the old temporary proof until both decisions exist; then prove only the chosen semantics and remove the temporary artifact. |

**DD-A9-01 / terminology chain.** C12 is contributor-only frontend evidence.
Before any terminology closure: `{C857,C870} -> {C500,C501} -> T-A9-1` fresh-schema/workspace-compile
integration barrier -> LDA/server canonical API -> TS
decoders/client -> route/link emitters -> direct caller/no-legacy verification -> fresh-install/E2E.
Canonical paths are `/courses/:courseInstanceId/assessments/:assessmentId`,
`/assessment-attempts/:assessmentAttemptId`, and Instructor `/assessments/.../properties`.
JSON: `assessment`, `assessmentReference`, `assessmentAttempt`, `assessmentEntry`,
`assessmentStatus`; Blueprint `assessments`, `blueprint_assessment_reference`. Legacy Assignment
routes, APIs, decoders, and tests are absent; callers use the canonical route directly. No
mixed-version deployment; rollback restores matching code and resettable base together.

**Dependencies and temporary-proof policy:** C500's full SQL consumer list and C501's full Rust
consumer list are in the active plan because each direct cutover must leave its whole layer
installable or compilable. C500 and C501 can be prepared in parallel only in isolated worktrees
after their handoffs. This shared checkout serializes their mutating bundles, runs no
install/compile gate until both are present, and then uses T-A9-1. All probes are temporary and
removed unless `PYTEST_STYLE.md` justifies one focused permanent replacement.

## Final closeout

For each row, run the exact temporary probe and recipient gate above, then:

    source source_me.sh && python3 devel/human_guidance_checklist.py --gate 09_assessments.md

After dependency-closed tranche, use only existing Browser Suite lease:

    source source_me.sh && ./launchers/all_test.sh

Then generator build/diff/consistency. Do not start, stop, replace, or clean up the shared Browser Suite.

Latest three-bullet follow-up: Sysadmin interface work priority is audited N/A; searchable
Blueprint Promoted and exclusive Sysadmin control are closed SQL rows with browser integration
remaining open. Prior 940-bullet evidence is retained. Submitted R-4 progress is reopened narrowly because
disabled navigation says `Question - of 4` and `0 saved`; the four current active-navigation closures
remain accepted. Actual expired summary shows retained correct MATCH Q2 `1 / 1` with all four
pairs and feedback, other three Questions incorrect `0 / 1`, total `1 / 4`. Their Closed/response-
unavailable wording is an observed visibly-unanswered mismatch, not unobserved behavior.

Latest Student response distinction rewrite: live HG requires visually distinct current Question,
saved-response status and keyboard focus, plus response-effect labels distinguishing Save/Clear/
change from whole Coursework submission. Both rows are open; earlier navigation styling receipts
are retained as partial proof, not blanket acceptance of native response controls/actions.
