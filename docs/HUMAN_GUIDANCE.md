# Human guidance

This file records durable project guidance from the repository owner. Apply it
alongside [AGENTS.md](../AGENTS.md) and the active implementation plan.

- Keep this file close to the owner's actual words. Put engineering
  interpretations, inferred requirements, and implementation details in the
  active plan or workstream documents instead.
- When the owner corrects an interpretation, update this file to preserve the
  correction.

- Keep every source file below 1000 lines by moving complete capabilities into
  focused modules before a parent file becomes an implementation warehouse.

## Device and viewport priorities

- Design desktop-first around a canonical 1280 by 800 laptop browser window, and make the interface
  look its best there. Instructors will usually work on laptops. A supported laptop browser may
  occupy only part of a larger physical display; do not assume full-screen desktop use.
- Treat an 800 by 1280 portrait browser as a representative secondary student/tablet target. Student
  devices will be more variable than instructor devices, but that variability must not make the
  canonical laptop interface padded, stacked, or visually compromised.
- Keep ordinary narrow phones genuinely usable: prevent horizontal overflow, preserve accessible
  controls and navigation, and reorganize individual components when available space requires it.
  A phone layout may be less elegant and need not preserve the desktop information density or visual
  composition. Do not redesign or stack the entire product merely to optimize for a
  320-pixel-wide viewport; narrow-phone checks are compatibility guards, not the design canvas.
- Adapt layout with CSS Grid and Flexbox plus complementary media and container queries. Use a small
  accessible disclosure menu when the visible navigation actually runs out of space. SolidJS manages
  interface state but does not replace CSS layout. Do not add an npm responsive-menu dependency
  unless the current navigation has a demonstrated interaction need that native HTML and CSS cannot
  meet cleanly.
- Distinguish physical panel resolution from the browser's CSS-pixel viewport when recording
  evidence. Center visual acceptance on the canonical 1280 by 800 laptop window, include the
  representative 800 by 1280 student/tablet path, and keep one narrow-phone compatibility guard.
  Do not expand the viewport matrix without a demonstrated product problem.

## Interface composition and accessibility

- Compose pages around the teaching task, not around individually padded components. Review
  typography, spacing, alignment, borders, controls, navigation, content width, and information
  grouping together. Prefer shared shell and design-system corrections when the same problem appears
  on several pages.
- Optimize instructor workflows first for a 1280 by 800 CSS-pixel laptop viewport. Assignment
  authoring, problem selection and organization, gradebook, roster, course management, workspace,
  and library pages should use most of the useful width when it improves scanning or editing. Four
  selected problems, their policies, and the save action should fit comfortably in that workspace.
- Keep student workflows responsive across the representative tablet and narrow-phone guards. A
  wide screen should not force the learner's attention across an arbitrary prompt/response split;
  keep each question and its response controls in one readable composition.
- Establish hierarchy with composition and spacing before adding borders, weight, or color. Use
  compact page headings, quiet secondary actions, restrained dividers, and visible grouping. Do not
  make every control look equally important.
- Keep accessible names, semantics, keyboard operation, readable text, non-color status cues, and
  visible focus in every presentation. Increased contrast is a user-selectable presentation mode,
  not the visual default for every user. Standard presentation should meet the accepted 5.5:1
  ordinary-text target while preserving the selected course palette; increased contrast may use
  stronger text, boundaries, and focus treatment without changing content or behavior.
- Use a precise, element-sized focus indicator. Do not place thick dark rings around an entire page
  or content region when the focused control can be identified directly.
- Do not reserve a persistent footer band for a slogan on teaching workspaces where vertical space
  is scarce.
- Treat Courses as the instructor home workspace instead of copying ADAPT's generic Dashboard
  dropdown. Keep Courses, Library, Workspace, and Account directly visible as distinct global
  destinations; after opening a course, use course-local navigation for assignments, learners,
  gradebook, and appearance. Add a separate dashboard only for a demonstrated cross-course task that
  the object-centered workspaces cannot serve.

## Plan status

- Treat `docs/active_plans/implementation_plan.md` as the source of truth for
  implementation order, architecture, contracts, security, tests, and gates.
- Use `docs/active_plans/active/release_completion_plan.md` for the decision-complete remaining
  package sequence and binary version 1 scope.
- `docs/active_plans/m0-results.md` is concluded M0 evidence. Read it when M0
  history matters; do not treat it as an active task or reopen M0 without new
  evidence.
- Finish and validate one work package before advancing to its dependency-order
  successor.

## Retention defaults

- Ship the privacy-first course lifecycle defaults: notify after 30 days,
  archive student records after 100 days, and permanently delete them after
  365 days. Institutions may later configure their own ordered policy.
- Retain tenant-owned assignment definitions by default when student records
  archive or delete. A later archive workflow may offer an explicit owner
  choice without following references into shared published content.

## Backup and recovery

- Name SQLx migrations with a compact integer version prefix in the form
  `YYYYMMDDNN_description.sql`, where `NN` is the two-digit sequence for that
  day. Keep the prefix contiguous because SQLx parses everything before the
  first underscore as one integer migration version.
- Back up PostgreSQL role attributes and memberships together with the database,
  but omit password hashes. Rehydrate and rotate login credentials from the
  deployment secret manager after restoration.
- Encrypt backup artifacts before they reach persistent storage. Restore into a
  clean cluster and verify the migration ledger, logical data fingerprint,
  owners, grants, forced RLS, tenant isolation, application writes, and broker
  calls before calling the database recovered.
- Treat a local logical restore rehearsal as recovery evidence, not as proof
  that managed point-in-time recovery, production key management, a numerical
  recovery objective, or the disclosed backup-retention window is deployed.

## Agent-specific guidance

- Codex follows `AGENTS.md` and the repository style documents.
- A goal is not complete until the entire plan-scoped Validation test suite defined in
  [TEST_EVIDENCE_MODEL.md](TEST_EVIDENCE_MODEL.md#validation-test-suite) is green on the final
  material tree. Required skips and unrun gates are not green; later material changes invalidate the
  affected evidence. Keep the goal active until those gates and required independent reviews pass.
- `docs/CLAUDE_HOOK_USAGE_GUIDE.md` is specific to Claude tooling and does not
  govern Codex commands or file-search behavior.
- Choose the robust, clean methodology and keep pushing forward while the next
  safe task is clear.
- Prioritize positive prompting and focus on important issues.
- Be efficient with time. Subagents and tokens are cheap; wall time is not.
- Break hard problems into the smallest independently completable tasks. Give
  each task one owner, one clear outcome, and one verification step. Run
  independent tasks in parallel when doing so safely shortens wall time.
- Use the hang check to distinguish a genuinely stuck agent from a healthy
  agent that is still working.
- Use GPT-5.3-Codex-Spark subagents for simple, bounded tasks that can run independently. Keep
  architecture, difficult cross-cutting decisions, coordination, and final integration with the
  manager. Follow [docs/CODEX_SPARK_SUBAGENTS.md](CODEX_SPARK_SUBAGENTS.md) for the delegation
  contract.

## Plan and test discipline

- Avoid overly strict requirements. Ground plan gates and requirements in real behavior and
  actual risk. Use representative qualitative evidence and behavior contracts; a calibrated
  performance budget needs evidence and user value.
- Do not require byte-equivalent or pixel-equivalent results when the planned change is an
  improvement and semantic or visual behavior is the real contract.
- Do not add uncalibrated numeric-equivalence gates, including arbitrary targets such as
  `<400 ms`, exact counts, scores, query plans, timings, or backend numeric equivalence.
- Compare every test plan with [docs/REPO_STYLE.md](REPO_STYLE.md),
  [docs/PYTEST_STYLE.md](PYTEST_STYLE.md), [tests/TESTS_README.md](../tests/TESTS_README.md),
  [devel/DEVEL_README.md](../devel/DEVEL_README.md), and the relevant style documents.
- Classify one-time implementation checks separately from permanent tests. Keep regular tests
  offline, deterministic, behavior-focused, and in the repository's documented test location.
- Name permanent test files and cases for the product behavior they protect, not for plan,
  milestone, work-package, or journey labels. Stable journey identifiers may remain only where a
  serialized report contract or its executable journey mapping requires them.
- Use the permanent-test checklist in [docs/PYTEST_STYLE.md](PYTEST_STYLE.md). Avoid unnecessary
  fixtures, networked regular tests, and tests of tunable constants or incidental structure. When in
  doubt, remove the permanent test.

## Local services

- Podman is normally running on the owner's machine.
- Use `source source_me.sh && python3 local_stack.py` as the normal local-stack controller. Its typed
  reusable layer is also the common lifecycle contract for Codex, aggregate Playwright acceptance,
  and canonical walkthrough disposable ownership; do not recreate provider selection, env-file
  sanitization, label discovery, readiness, or generic cleanup in each shell/test runner.
- Keep `local_stack_control/launch.sh` as the private build, bootstrap, migration, seed, renderer-probe, and
  readiness owner behind that controller. The controller must delegate that work rather than create a
  second local-stack initialization path.
- Local-stack lifecycle tools may inspect any Compose project by label, but mutating commands should
  affect only the default pre-production `containers` project unless a typed disposable-owner contract
  explicitly supplies its private environment, compose target, owner evidence, and exact cleanup scope.
- Label-derived containers, volumes, and networks are the source of truth for lifecycle scope; generated
  resource names are not authority. The default controller never accepts arbitrary destructive project,
  image, network, volume, force, or prune behavior.
- A destructive local reset must show the exact project and resources, require the visible project
  confirmation `--confirm-project containers`, and avoid global Podman prune/remove behavior.
- Use the local containers when the active work package reaches a documented
  PostgreSQL, MinIO, health, tenancy, or other container-dependent gate.
- Keep offline contract work on memory backends when its work-package gate does
  not require containers.
- Start live or full-stack Playwright only after every required long-running
  Podman service is active, each declared health check passes, and every
  one-shot setup service exits successfully. Keep mock-backed browser tests
  clearly separate because they do not claim local-stack acceptance.

## Teaching and product priorities

- PLE is pre-production: it has no users and no current durable data. Improve
  the current design directly. Do not add compatibility shims, legacy readers,
  migration or adoption paths, or duplicate retained fixtures for hypothetical
  users. Keep explicit versioning where it defines the current external or
  artifact protocol and supports future evolution; ordinary disposable local
  data may be rebuilt.
- Do not squash the current PostgreSQL migration ledger during active feature
  acceptance. Before the first production deployment, replace the unreleased
  history with one reviewed clean-cluster baseline. After that baseline ships,
  preserve each new forward migration as the durable upgrade ledger.
- Push harder on the visual design. Make the interface less bubbly, especially by reducing overly
  padded content.
- Use human-readable identifiers for workflows where people need to read, recognize, communicate,
  copy, or enter an identifier. Size identifiers according to the uniqueness actually required by
  their scope. Keep opaque globally unique identifiers for internal boundaries where that level of
  uniqueness is useful.
- Never present a UUID in visible or announced page content, application navigation URLs, or
  user-copyable links. Browser routes use short typed human references such as `C-123`, `A-456`,
  `R-789`, and `W-42`; questions use the Crockford Base32 identity defined in
  `docs/QUESTION_ID_SPEC.md`. Resolve every reference under the current
  tenant, role, membership, and ownership boundary. A public reference is a locator, never authority.
  Background API and asset requests and hidden form values may retain internal UUIDs when needed.
- Use human-readable problem titles and one copyable, non-sequential Crockford Base32 Question ID in
  instructor workflows and documentation. Display the ID as `AAA-BBBB`; accept forgiving Crockford
  input; and never expose a question's internal UUID, historical snapshot UUID, sequential database
  value, or version number as its identity. An instructor may use the ID for occasional direct
  lookup, but assignment copy/import and a checklist from an existing assignment are the preferred
  ways to reuse a group of questions. Preserve entered input and the unchanged assignment when an ID
  is malformed, unavailable, unauthorized, or already selected.
- Treat one Question ID as one current question. The original owner may publish a bug correction
  that propagates forward to uses of that question. A substantive independently editable change is
  a fork with a new Question ID and retained provenance. Keep immutable snapshots and exact hidden
  `(ProblemId, VersionId)` attempt evidence only where grading reproducibility, audit, or provenance
  requires it; do not turn that history into instructor-facing versions.
- The product supports learning through repeated algorithmic practice. A first
  completion or a 100 percent score must not end continued practice when policy
  permits another run.
- Fresh variation is more important pedagogically than seed replay. Give every
  newly issued parameterized question instance a fresh server-owned seed;
  preserve an existing attempt's seed only for resume, re-render, audit, and
  debugging of that same instance.
- Preserve server-only grading and answer secrecy. The browser may validate
  response format but must not receive answer keys or grading implementations.
- Keep student and course records tenant-owned while published educational
  content remains shared and immutable.
- Favor behavior-focused evidence that reflects what instructors and students
  actually do over implementation-detail tests.

## User roles and student records

- The only human user roles are Sysadmin, Instructor, and Student. There are no
  Manager, Administrator, or Publisher users. The repository owner is the
  current sysadmin and is also an instructor.
- Approve every Instructor manually after real-person validation. Do not add a
  self-service promotion path.
- Student users fall under FERPA. Treat their course-linked data, not their
  account merely for existing, as radioactive.
- A sysadmin does not receive general access to FERPA course records. Require
  direct Instructor membership in the exact course when the sysadmin needs to
  act as its instructor. Permit roster troubleshooting through the separate
  audited Sysadmin roster-support capability; do not widen it to grades,
  responses, attempts, exports, item analysis, or general course browsing.
- Keep the dedicated public-asset publisher as a service identity, never a
  human role.

## Student keyboard accessibility

- Make every student browser action usable with the keyboard alone.
- Make the browser platform contract the primary path: Tab and Shift+Tab move focus, Space performs
  native selection or activates a focused button, and native links retain Enter activation. The
  complete student journey must work through visible controls without a widget shortcut.
- Treat Enter-to-submit from a response input, Arrow keys, visible-choice digits 1-9, and Escape as
  widget-specific extensions. Scope and test each extension separately; never let it replace the
  visible platform path or override text editing, input-method composition, or native dialogs.
- Apply the exact journey, response-family, recovery, and evidence rules in
  `docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md`. New question families satisfy that contract in their
  owning package rather than waiting for a later generic audit.

## Assignment walkthrough acceptance

- Make this start-to-finish teaching loop usable for the Fall 2026 pilot, which
  begins in approximately two weeks. Prioritize this pilot path over unrelated
  release breadth.
- Make the end-to-end walkthrough demonstrate the teaching workflow: an instructor creates a
  course, adds an active student to its roster, creates an assignment from problems in the
  published corpus, and then observes the student's scored work.
- Make the student take, submit, complete, and repeat that assignment through visible controls.
  The complete student assignment path must satisfy the keyboard accessibility contract above.
- Include one representative four-question Chapter 1 assignment in the canonical walkthrough.
  Keep the complete Genetics-plus-Biochemistry eight-question learner sweep as a separate release
  gate so the walkthrough remains focused without substituting a synthetic one-question story for
  release content.
- Have the instructor visibly copy or import the Genetics Chapter 1 assignment, then confirm its
  four recognizable question titles and Crockford Base32 Question IDs before creating the target
  assignment. The workflow must not reconstruct a teaching set from ranges, sequential numbers,
  version suffixes, or UUIDs.
- Keep the public pilot guides visually complete. Show the instructor's course, roster, published
  problem selection, and assignment settings. Show the student's assignment list, live timed
  problem, score, and visible option to start another practice run.
- Use the unmistakably fictional, deterministic labels `Dr. Fake Professor`, `Mary Fake Student`,
  and `Jack Fake Student` for local pilot identities and screenshot data. No screenshot should
  imply that a real Roosevelt student or instructor participated.
- Treat the approved fake-user walkthrough screenshots as required acceptance evidence. Do not use
  student privacy as a reason to omit them. Continue excluding credentials, answer material,
  traces, and raw child output because those are security and test-integrity boundaries.
- Keep email outside this walkthrough. The local instructor and student identities are the intended
  actors; no agent needs an email account, mailbox, delivered link, SMTP provider, passwordless
  challenge, invitation-delivery proof, or canonical-account acceptance for this walkthrough.
- Do not report missing email infrastructure as a walkthrough blocker. Canonical email identity and
  production onboarding remain separately owned release concerns.
- No SMTP provider or email-activation path is configured today. Fastmail is the intended future
  external provider, but that intent is not acceptance evidence: keep email-dependent controls and
  claims unavailable until operator credentials, an authorized sender, live delivery, and browser
  sign-in have each been verified.

## Flat question source

- Use versioned PLE flat-question JSON as the canonical machine format for
  simple static questions. Treat QTI as an import/export adapter and archival
  interchange format, not as the internal source model.
- Keep the first version small and closed. Add an explicit format version for
  new question families or incompatible semantics instead of accumulating
  QTI-style expression trees and vendor extensions in one flexible document.
- Compile answer-bearing author input into separate checksummed public question
  content and grader-only key/feedback material. Neither authored nor published
  source objects may receive signed delivery URLs.
- Stable semantic choice, blank, prompt, item, and region IDs own answer meaning inside PLE; display
  labels such as A, B, and C do not. An import adapter derives those IDs deterministically when an
  external source uses text or position instead of authored identifiers.
- PLE flat-question JSON must support, at a minimum, multiple choice (MC), multiple answer
  (MA), fill-in-the-blank (FIB), multiple fill-in-the-blank (MULTI-FIB),
  numerical entry (NUM), matching (MATCH), ordered list (ORDER), and image hot
  spot (HOTSPOT) questions.
- Use closed PLE flat-question JSON version 2 as the internal source contract for all eight families.
  Base MC, MA, MATCH, NUM, FIB, MULTI-FIB, and ORDER semantics on the reviewed QTI Package Maker item
  model while retaining accepted answers and grading data that print-oriented writers omit. HOTSPOT
  is a bounded PLE extension because the reviewed item model does not define it. Do not retain a
  version 1 `singleChoice` reader, source bytes, or compatibility behavior.
- Use QTI Package Maker's HTML self-test as the learner-interaction reference for those seven native
  families. Preserve its useful task clarity: one compact question surface, an obvious submit action,
  visible selected or entered state, exact per-part completion where a response has several parts,
  plain-language outcome and progress feedback, a clear way to change or reset an unsubmitted
  response, and an unmistakable completed state. Apply the same interaction vocabulary across
  families so students learn one practice workflow rather than seven unrelated widgets.
- Preserve PLE's stronger trust and accessibility boundaries while adapting that model. Grading and
  answer keys remain server-only; feedback comes from the immutable submission receipt. Do not copy
  the self-test's client-side grading data, exact result-string protocol, drag-only MATCH or ORDER
  interactions, color-only feedback, or large padded card treatment. Native labeled controls,
  Tab/Shift+Tab plus Enter or Space, visible text status, preserved input, and recoverable submission
  errors are the primary path; pointer and arrow-key interactions are optional enhancements.
- Treat a future external QTI-JSONL format as an adapter/interchange concern, not a prerequisite for
  native family support. Keep any accepted external interpretation in one versioned adapter and map
  it into PLE's answer-free public model plus grader-only private material; never spread external
  fields across storage, routes, UI, and grading.
- For image and other binary references, keep bytes, checksums, media types, lifecycle, and
  authorization in PLE object storage rather than embedding bytes in JSON or database rows. Port
  engine code to Rust only when a concrete integration needs it.
- Treat `feedback_correct` and `feedback_incorrect` as optional sidecars shared
  by question types, following QTI Package Maker's `BaseItem`. Authors are often
  incomplete, so feedback is not required and missing feedback does not make a
  question invalid.
- YAML may later be a human-editing input, but it must compile once into the
  canonical JSON contract. Prefer JSON for deterministic cross-language
  validation and checksums, not because parser speed materially affects the
  student request path.
- Keep flat-question private material behind the dedicated grader capability.
  Persistence may validate its opaque typed integrity and bind it to the public
  model. Outside the narrow protected author-source route below,
  browser-facing stores, routes, generated contracts, and the Wasm closure
  must neither construct nor read its canonical bytes.
- The authenticated, author-role-only flat-question source `GET`/`PUT` route is
  the deliberately narrow browser exception for an instructor's own canonical
  source. It must use `Cache-Control: no-store` and a strong ETag and never
  broaden ordinary browser contracts. An author-owned immutable asset picker may
  return the server-verified content checksum required to bind that exact asset;
  the browser may echo but never assert that value, and the server must re-resolve
  it on every save and publication. Never expose signed object URLs, object keys,
  buckets, paths, or storage-record identities.
  Learner/student preview, Wasm, public publication DTOs, and all non-author
  routes remain answer-free.

## First content release

- Populate a Chapter 1 assignment for genetics and a Chapter 1 assignment for
  biochemistry from the biology-problems-website content.
- Use a combination of first-class algorithmic WeBWorK questions and
  second-class static PLE flat-JSON questions. QTI remains an import/export adapter rather than the
  internal source format.
- Four questions per chapter is enough for the first release: one WeBWorK MC,
  one WeBWorK MATCH, one static PLE flat-JSON MC, and one static PLE flat-JSON
  MATCH.
- Use the existing PGML versions of the Chapter 1 questions where they are
  available.

## WeBWorK project boundary

- PLE owns the only course, roster, assignment, attempt, feedback, and gradebook
  system. Do not deploy a parallel WeBWorK2 assignment distribution.
- Use `webwork-pg-renderer` as the private HTTP integration layer and
  `openwebwork/pg` as its PG/PGML render-and-grade engine. Neither layer owns
  PLE educational records.
- Treat `openwebwork/webwork2` as implementation history and reference material
  after the standalone-renderer cutover. Its course database and MariaDB are not
  part of the intended PLE runtime.
- Treat every path under `OTHER_REPOS/` as read-only comparison evidence. Build,
  test, and runtime code must use pinned maintained upstream revisions or
  declared artifacts instead of importing, mounting, or building those copies.

## Course appearance

- Do something similar to Blackboard Original course themes.
- Allow an instructor to upload a small banner image per course and select one
  of the preconfigured themes. Center the banner image at the top of the course
  entry page.
- Normalize each banner to one fixed 1200 by 328 pixel wide image, using the
  proportions of the centered YouTube banner safe region as the guide. Crop
  once on the server without stretching; scale that same image down in the
  browser without changing its aspect ratio.
- Apply a three-color theme to all of the course pages.
- Use `grass` as the default course theme. Its Roosevelt-inspired palette uses the official
  `#73C167` and `#008852` greens plus the pale `#BDDEB1` fill observed in the public university
  logo. Treat the pale fill as logo-derived inspiration, not as an official brand-guide swatch, and
  do not present PLE as official Roosevelt branding.
- Keep raw palette anchors decorative when they do not meet the product's contrast target. Derive
  readable action, link, text, focus, and boundary colors without expanding the three stored theme
  choices. In standard presentation, keep those derived colors visually related to the palette and
  avoid pushing ordinary text toward near-black-on-white when the 5.5:1 target is already met.
- Treat each stored course palette as a meaningful visual system, not three decorative accent
  swatches on an otherwise identical white page. Standard presentation should use subtle
  palette-derived canvas, navigation, grouping, and active-state surfaces. Increased contrast keeps
  the course's hues and identity while strengthening the rendered pairs that benefit from it.
- Keep the stored three-color anchors stable when the weakness is in presentation rather than the
  palette. Tune the centralized palette-to-role recipe and shared course-scope controls first so the
  full canvas, tinted work surfaces, readable cards, active navigation, and accent treatment can
  evolve together. Theme selection and review artifacts should preview those applied roles rather
  than letting an optional banner or tiny swatches stand in for the theme.
- Use biome and habitat theme names that give a sense of color on their own:
  tundra, forest, desert, grass, arctic, ocean, tropical, woodland, coral reef,
  swamp, underground, salt marsh, wetland, sea floor, magma, and beach.
- Purge some theme names when they would look substantially the same.

## Performance choices

- When measured behavior is slow, consider implementing the hot path in Rust
  or WebAssembly.
- Keep the security boundary intact when optimizing: deterministic generation,
  response-format validation, timer display, and state transitions may run in
  WebAssembly; answers, keys, and correctness decisions remain server-only.

## Score precision and display

- Use `f64` for scoring calculations and `AttemptResult` across Rust, WebAssembly,
  and browser projections. Do not replace ordinary score arithmetic with `f32`
  or a scaled-integer points model.
- Round computed current points explicitly to at most four decimal places before
  persistence. Keep PostgreSQL `NUMERIC` as the rounded storage boundary without
  forcing general Rust scoring code to use fixed-point arithmetic.
- Exact decimal command boundaries, such as a manually entered credit fraction,
  may retain up to 12 decimal places. They do not require the rest of the score
  model to use decimal arithmetic.
- Display scores and percentages with at most two decimal places and trim
  trailing zeroes. Show `8 / 10`, `8.5 / 10`, or `8.33 / 10`, never a binary
  floating-point artifact such as `8.0000000000006 / 10`.
- Round to nearest with exact midpoint ties away from zero for both four-place
  persistence and two-place display. Cover the same positive, negative, and
  midpoint boundary examples in Rust and TypeScript so server and browser
  output cannot disagree.

## Software design

- Focus software design on adaptability, allowing systems to evolve with
  changing requirements and insights over time.
- Use adaptability to maintain functionality and relevance in a dynamic usage
  environment.
- Prefer a modular monolith with capability-owned components over either large
  catch-all files or operationally separate microservices. Crate boundaries
  continue to enforce security and deployment rules; modules inside a crate
  should make one capability understandable in isolation.
- Give each component one narrow contract, its backend implementations, and
  focused behavior or conformance tests. A contributor should be able to work
  from those files without reading the complete learning data-access or server
  backend.
- Keep crate roots and large parent modules as facades and composition points:
  declare modules, re-export stable public paths, and wire shared dependencies.
  Put domain behavior, SQL, route handlers, codecs, and tests in the owning
  capability module.
- Split by ownership and behavior rather than arbitrary line ranges. Preserve
  public paths and typed contracts during structural extraction so module work
  does not force unrelated callers to change.
- Keep walkthrough entry scripts thin. Put reusable Python orchestration in a
  dedicated importable package while preserving the documented shell command;
  keep browser journeys under `tests/playwright/` as separate visible evidence.
- Give Python programs a small explicit interface: put choices that an operator
  makes between runs in documented `argparse` arguments or a selected
  configuration file. Reserve the process environment for operating-system and
  ecosystem configuration, not hidden walkthrough modes, ports, paths, or
  credentials. When fixed child processes need a handoff, pass one explicit,
  versioned private input file by argument; validate its schema, ownership,
  path, and permissions at each boundary.
- A production worker may claim a job family only when that registry entry has
  both a real handler and its atomic committer. Derive the queue-family filter
  from those complete entries; leave reserved work queued instead of adding a
  placeholder or allowing a partial worker to consume it. Scale worker
  processes for concurrency and keep one bounded job between shutdown checks.
- Use plain capability names before implementation jargon in code comments,
  documentation, commands, and contributor handoffs. `learning-data-access`
  owns persistence contracts and backends, `in_memory` names its database-free
  backend module, and `project-tools` contains repository-only automation.
- Use hyphens for Cargo package and crate-directory names, such as
  `learning-data-access` and `project-tools`. Use underscores for Rust module
  and import names, such as `in_memory` and `learning_data_access`.
- Invoke repository automation through `cargo tools`. Do not retain the opaque
  `cargo xtask` compatibility alias after the atomic naming migration.

## Authentication storage and compliance

- PLE accounts are institution-independent. Use an opaque `UserId` and passwordless
  email authentication as the canonical registration and sign-in path. Passkeys are
  optional convenience credentials for the same account. SSO integration may also
  exist, but it is not required for independent use.
- Invite students at a verified email address. The primary instructor handoff may
  be a one-time copyable invitation link shared through an existing trusted LMS;
  configured SMTP is an optional delivery channel, not an enrollment dependency.
  Use an established Rust email library and the operator-selected external SMTP provider for
  email authentication and optional delivery. Do not build or maintain a PLE mail
  server, queue, templating engine, or deliverability system. A learner keeps one
  PLE account across courses and institutions; course membership limits each
  instructor's access.
- Do not create a separate account-recovery mode while verified email authentication
  remains the canonical path. Losing a passkey returns the learner to email sign-in.
  A verified email change is an ordinary signed-in account operation. Instructors may
  revoke and re-invite a learner at a new address, but version 1 does not merge accounts
  or transfer educational records between account identities.
- Keep the institutional email, institutional student ID, and useful display
  name as protected course roster data when they make enrollment and manual
  LMS grade export practical. Permit course email-domain rules for signups.
- Balance FERPA and data minimization with instructor convenience: collect
  reluctantly, use deliberately, and purge predictably under the course
  retention policy.

- Store the opaque authentication credential in one host-only HttpOnly cookie,
  not in `localStorage`. JavaScript must never be able to read the bearer
  credential.
- Use the cookie only for authentication, session security, expiration, and
  revocation needed to provide the signed-in service. Do not attach analytics,
  advertising, cross-site tracking, or unrelated preference data to it.
- Treat `localStorage` and similar browser mechanisms as storage/access
  technologies too; changing the browser API is not a way around European
  storage-consent rules.
- Classify the authentication cookie as strictly necessary only while it is
  essential to the service explicitly requested by the user and has no
  secondary purpose. Clearly disclose its name, purpose, deployment context,
  and lifetime even when prior opt-in consent is not required.
- Make ordinary authentication a browser-session cookie by default while the
  server retains an authoritative, bounded expiration. Do not assume that a
  persistent login cookie is exempt: any `remember me` behavior requires an
  explicit user choice and a jurisdiction-specific consent and legal review
  before implementation.
- Require separate consent handling before adding any nonessential browser
  storage or tracking. Recheck the deployed behavior against the target
  jurisdiction; this engineering rule is not a substitute for legal review.
- Keep the technical controls narrow: a `__Host-` cookie with `Secure`,
  `HttpOnly`, `SameSite=Lax`, `Path=/`, and no `Domain` for production HTTPS;
  explicit insecure mode only for local HTTP development; and immediate
  server-side revocation on sign-out. Do not add a production
  `SameSite=None`/embedded-LTI mode. A future LTI integration needs a separate
  reviewed browser and session design.

## Security design decisions

- Encrypt PostgreSQL, object storage, backups, and deployment volumes at rest
  with managed encryption and scoped KMS keys. Do not add blanket application
  encryption to immutable public published content: its delivery needs CDN
  access and its integrity is enforced by immutable publication evidence and
  SHA-256 bindings. Use application AEAD selectively for stored secrets such
  as external-tool launch state.
- Security concealment and accessible teaching guidance are complementary:
  revoked or unauthorized learners receive the same generic unavailable/not-
  found outcome needed to avoid record disclosure, while the UI gives clear,
  keyboard-accessible recovery guidance (for example, contact the instructor
  for an indeterminate external tool). Do not reveal membership, provider, or
  record details merely to make an error more specific.

The durable regulatory references for this decision are Article 5(3) of the
[consolidated EU ePrivacy Directive](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:02002L0058-20091219), the
[Article 29 Working Party Opinion 04/2012](https://ec.europa.eu/justice/article-29/documentation/opinion-recommendation/files/2012/wp194_en.pdf),
and current [ICO guidance on strictly necessary storage/access](https://ico.org.uk/for-organisations/direct-marketing-and-privacy-and-electronic-communications/guidance-on-the-use-of-storage-and-access-technologies/what-are-the-exceptions/).

## Dependency versions

- Focus on the latest versions of all code because many security bugs are being
  fixed.
- For each direct registry dependency, use either `version = "*"` or an audited
  open minimum such as `version = ">=0.29.0"`, where the minimum is the latest
  stable version reviewed at refresh time. Both forms leave newer releases
  eligible; an open minimum also records the known-safe floor in the manifest.
- Do not use caret, exact, tilde, or upper-bound requirements for direct registry
  dependencies unless this file records a repository-specific exception with its
  reason and removal condition. `Cargo.lock` remains the reviewed exact
  resolution between refreshes; refresh it deliberately and review advisory
  results before accepting the updated graph.

## Generated artifacts

- Put reproducible generated content under the repository-root `generated/`
  directory and keep that directory out of Git.
- Regenerate required artifacts through their tracked owning generator before
  builds and validation; ignored output must not become an unverified input.
- Link documentation to the tracked generator or authoritative source rather
  than to files under `generated/`, which do not exist in a clean checkout.
- Track small, deliberately reviewed golden baselines when they define a
  compatibility contract or record work evidence. These are authoritative test
  inputs rather than disposable generated build output.
- Treat `tests/fixtures/published_problem/` as reviewed cross-layer test
  evidence. Keep its fully derivative TypeScript projection under ignored
  `generated/fixtures/` and regenerate it before TypeScript validation.
