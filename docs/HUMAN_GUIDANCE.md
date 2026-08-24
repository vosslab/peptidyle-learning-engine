# Human guidance

This file records durable project guidance from the repository owner. Apply it
alongside [AGENTS.md](../AGENTS.md) and the active implementation plan.

> Maintenance note: Prior manager runs expanded concise human specifications
> into additional inferred requirements. Keep this file tight to the human's
> specification. Put engineering interpretation and implementation detail in
> the active plan or a focused technical document.

- Keep every source file below 1000 lines by moving complete capabilities into
  focused modules before a parent file becomes an implementation warehouse.

## Device and viewport priorities

- Design desktop-first around a canonical 1280 by 800 laptop browser window, and make the interface
  look its best there. This canonical laptop viewport applies to both instructors and students.
  Instructors are expected to use laptops predominantly, and students are expected to use laptops
  most often as well. A supported laptop browser may occupy only part of a larger physical display;
  do not assume full-screen desktop use.
- For planning, professor surfaces target at least 1280 by 800 CSS pixels. Use this student planning
  mix: 1280 by 800 laptop 40%, 800 by 1280 portrait tablet 30%, iPhone Pro aspect 20%, and square
  aspect 10%. These are planning weights, not test quotas or telemetry targets.
- Permanent visual evidence is organized under `docs/screenshots/` by role and by student/access
  boundary. The exact CSS-pixel matrix is 1280 by 800 (16:10), 800 by 1280 (10:16), 393 by 852
  (iPhone Pro aspect), and 800 by 800 (square), with planning weights of 40%, 30%, 20%, and 10%.
- A project **demo** uses actual PostgreSQL with the real migrations, schema, RLS, and persistent
  seeded data through the ordinary browser and server stack. It shows that the site works.
- Every fresh installation includes the demo data as a persistent base course with the students
  already created. Returning to the demo is like checking in on a live course.
- Keep public evidence separate from private evidence. Do not put answer keys, grading
  implementations, private source, email, UUIDs, or FERPA records in a public or learner corpus.

## Interface composition and accessibility

- Compose pages around the teaching task, not around individually padded components. Review
  typography, spacing, alignment, borders, controls, navigation, content width, and information
  grouping together.
- Optimize instructor workflows first for a 1280 by 800 CSS-pixel laptop viewport. Assignment
  authoring, problem selection and organization, gradebook, roster, course management, workspace,
  and library pages should use most of the useful width when it improves scanning or editing. Four
  selected problems, their policies, and the save action should fit comfortably in that workspace.
- Compose student workflows for the canonical 1280 by 800 laptop and the high-priority 800 by 1280
  tablet as design targets, and keep them responsive across the narrow-phone compatibility guard. A
  wide screen should not force the learner's attention across an arbitrary prompt/response split;
  keep each question and its response controls in one readable composition.
- Do not reserve a persistent footer band for a slogan on teaching workspaces where vertical space
  is scarce.

## Retention defaults

- Ship the privacy-first course lifecycle defaults: notify after 30 days,
  archive student records after 100 days, and permanently delete them after
  365 days. Institutions may later configure their own ordered policy.
- Retain tenant-owned assignment definitions by default when student records
  archive or delete. A later archive workflow may offer an explicit owner
  choice without following references into shared published content.
- Project teaching-operation retention pages from the existing retention engine. Browser pages
  display server-owned state and actions instead of reconstructing a second retention lifecycle.

## Agent-specific guidance

- Codex follows `AGENTS.md` and the repository style documents.
- `docs/CLAUDE_HOOK_USAGE_GUIDE.md` is specific to Claude tooling and does not
  govern Codex commands or file-search behavior.
- Choose the robust, clean methodology and keep pushing forward while the next
  safe task is clear.
- Prioritize positive prompting, especially for small models. State the intended action directly
  and omit unwanted tools and actions unless a concrete safety boundary requires naming them.
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
- Use `source source_me.sh && python3 local_stack.py` as the normal local-stack controller.
- Treat all local Podman images and project-named simulated-data volumes as disposable acceptance
  infrastructure. Image pruning is pre-approved. Project-named simulated-data volumes may be removed
  when their named acceptance target is verified; retain the typed target, label, and explicit-resource
  safeguards so cleanup remains scoped and inspectable.
- Before an ordinary local stack replacement, remove all containers and orphans in the exact labelled
  project while retaining its named simulated-data volumes; then recreate the complete designed suite.
  After semantic readiness, prune every image not used by a current container. All images in the
  owner's local Podman machine belong to this project and are disposable; the active full-suite
  containers protect their current images from pruning, while obsolete application, renderer,
  gateway, base, and intermediate builds must not accumulate.
- Start live or full-stack Playwright only after every required long-running
  Podman service is active, each declared health check passes, and every
  one-shot setup service exits successfully. Keep test-double-backed browser tests
  clearly separate because they do not claim local-stack acceptance.

## Teaching and product priorities

- PLE is pre-production: it has no users and no current durable data. Improve
  the current design directly.
- Preview and rehearsal operate on the same ordinary live courses, assignments, published
  questions, deterministic graders, and production routes as the rest of PLE. Baseline seeding
  creates ordinary live records. Rehearsal-specific persistence contains only the immutable
  execution and audit evidence needed to keep Instructor inspection out of learner records; it is
  never a second content source, question bank, assignment model, or demo application.
- Before the first production deployment, replace the unreleased
  history with one reviewed clean-cluster baseline. After that baseline ships,
  preserve each new forward migration as the durable upgrade ledger.
- Push harder on the visual design. Make the interface less bubbly, especially by reducing overly
  padded content.
- Use human-readable identifiers for workflows where people need to read, recognize, communicate,
  copy, or enter an identifier. Size identifiers according to the uniqueness actually required by
  their scope. Keep opaque globally unique identifiers for internal boundaries where that level of
  uniqueness is useful.
- Never present a UUID in visible or announced page content, application navigation URLs, or
  user-copyable links. questions use the Crockford Base32 identity defined in
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
- No SMTP provider or email-activation path is configured today. Fastmail is the intended future
  external provider, but that intent is not acceptance evidence: keep email-dependent controls and
  claims unavailable until operator credentials, an authorized sender, live delivery, and browser
  sign-in have each been verified.

## Flat question source

- Use versioned PLE flat-question JSON as the canonical machine format for
  simple static questions. Treat QTI as an import/export adapter and archival
  interchange format, not as the internal source model.
- PLE flat-question JSON must support, at a minimum, multiple choice (MC), multiple answer
  (MA), fill-in-the-blank (FIB), multiple fill-in-the-blank (MULTI-FIB),
  numerical entry (NUM), matching (MATCH), ordered list (ORDER), and image hot
  spot (HOTSPOT) questions.
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

## Automated grading

- Keep PLE's question workflows strictly automated grading. A supported question has a
  deterministic server-owned grader and an answer-free browser contract; a manual or otherwise
  non-deterministically gradeable question fails closed before delivery and creates no manual
  grading queue, receipt state, or compatibility path.

## Performance choices

- When measured behavior is slow, consider implementing the hot path in Rust/WebAssembly.
- Keep the security boundary intact when optimizing: deterministic generation,
  response-format validation, timer display, and state transitions may run in
  WebAssembly; answers, keys, and correctness decisions remain server-only.

## Score precision and display

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
- Use plain capability names before implementation jargon in code comments,
  documentation, commands, and contributor handoffs. `learning-data-access`
  owns persistence contracts and backends, `in_memory` names its database-free
  backend module, and `project-tools` contains repository-only automation.

## Authentication storage and compliance

- PLE accounts are institution-independent. Use an opaque `UserId` and passwordless
  email authentication as the canonical registration and sign-in path. Passkeys are
  optional convenience credentials for the same account. SSO integration may also
  exist, but it is not required for independent use.
- Invite students at a verified email address. The primary instructor handoff may
  be a one-time copyable invitation link shared through an existing trusted LMS;
  configured SMTP is an optional delivery channel, not an enrollment dependency.
  Use an established Rust email library and the operator-selected external SMTP provider for
  email authentication and optional delivery. PLE owns the bounded course-invitation delivery
  outbox described in [ENROLLMENT_DESIGN.md](ENROLLMENT_DESIGN.md); it records provider submission,
  leases, retry state, and cancellation without claiming mailbox delivery. Do not build or maintain
  a PLE mail server, generic mail queue, templating engine, or deliverability system. A learner keeps one
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

- Store the opaque authentication credential in one host-only HttpOnly cookie.
  JavaScript must never be able to read the bearer credential.
- Use the cookie only for authentication, session security, expiration, and
  revocation needed to provide the signed-in service.
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
  with managed encryption and scoped KMS keys.
  Use application AEAD selectively for stored secrets such
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
