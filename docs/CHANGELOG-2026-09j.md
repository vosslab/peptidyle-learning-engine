## 2026-09-14

### Additions and New Features

- Added a generated, verbatim Human Guidance implementation checklist and its narrow generator.
  The baseline accounts for all 761 HG bullets: 244 verified `[x]`, 480 findings `[ ]`, and 37
  positive-audit `N/A` entries. `[ ]` records an unverified behavior or implementation mismatch;
  it is not completion. Generator tests remain ignored one-time proof under `tests/_temp`; no
  permanent test was added.
- Added the planned Gmail API Email Delivery Backend specification. It keeps institutional email
  as the PLE Account identity and a dedicated Gmail account as delivery transport, defines the
  provider-neutral adapter, email-code ceremony, abuse controls, operator CLI, OAuth flow,
  host-protected credential file, API-only container mount, failure behavior, recovery, and
  attended delivery acceptance. Enrollment documentation and the future-capability backlog now
  point to the specification. No Gmail route, credential, or runtime behavior is claimed.
- Every installation now publishes the complete free and open-source Biology Problems Website
  Genetics Blueprint as its example course. The default installation also creates the optional
  Live Demo. `--without-live-demo` still publishes Genetics and omits the Live Demo Accounts,
  Course, and activity. Repeating provision does not duplicate the Blueprint. The exact portable
  content validation and fresh default, replay, and opt-out installation checks passed. The full
  `source source_me.sh && ./launchers/all_test.sh` gate passed in session 62894.
- Question Library now returns additional pages through its existing cursor API, so bundled
  Questions no longer cause searches to fail above one page. Server and frontend checks passed.

### Fixes and Maintenance

- Removed the unreachable legacy Assessment Editor page cluster, including its obsolete
  Question-Pool item model and retained-Assessment picker. The routed live Assessment Workspace
  remains the sole Instructor editing surface.
- Repaired four repository-hygiene findings without changing runtime behavior: escaped the
  Question Library Unicode test literal, normalized two Python indentation sites, and wrapped one
  long Assessment-attempt SQL query. The four focused hygiene nodes and Python compilation passed.
- Excluded vendored `.pgml` Question sources from the source-file line-limit hygiene scan without
  changing its exclusive 1,000-line threshold or other hygiene scans.
- Qualified relation references in the immutable Assessment-owned Pool import
  and append functions, preventing their `RETURNS TABLE` output names from
  ambiguously binding in PostgreSQL. Structural diff and ambiguity scans
  passed; independent PostgreSQL proof covered import, append, CAS,
  authorization, and historical Student Work.
- Recorded two accepted narrow closures. C203/C817 now verifies the FERPA Student-Work ownership
  boundary through a permanent real-session BOLA oracle: the owner is allowed and the four
  cross-Student, nonmember, cross-Course, and Sysadmin cases are denied. C36/C818 now verifies
  only that Profile opens its small menu and Sign Out is inside it; Profile/account menu contents
  and the broader no-scattering behavior remain open for C819-C823.
- Completed C301's native external-resource inventory. Native Question JSON now records reviewed
  absolute HTTPS external URLs in the closed link, image, script, stylesheet, and other categories,
  rejecting malformed and duplicate URLs. This is source metadata only: it does not claim upload,
  fetch, execution, CDN approval, or local serving. The temporary parser proof will be removed.
- Accepted the A7 Milestone G planning completion: 78 unchecked records yield 77 owning behaviors
  plus one A6 C208 duplicate. Canonical C300-C375 contains 76 rows (44 closure and 32 contributor)
  for 77 closure occurrences, with an 81-node, 100-edge acyclic DAG. Architect gates C303, C355,
  C359, and C360, the temporary-only 13k fixture, and the test-liability review are accepted.
  This records planning and audit work only; it does not claim implementation.
- Completed C29's habitat-theme naming verification. Stored theme ID `grass` retains its reviewed
  palette while every visible registry consumer presents `Grassland`; Forest, Ocean, Desert, and
  the remaining closed theme labels are likewise biome or habitat names. The density browser
  evidence could not launch Chromium in this sandbox, so it is not claimed as verification.
- Completed C28's keyboard-reordering audit. Blueprint Assignment, Assignment Workspace, and all
  native JSON reorderers expose labelled Move earlier/Move later buttons backed by identity-safe
  reorder models. The temporary seven-surface inventory was removed; this does not choose any
  drag-and-drop surface, which remains the separate Human Guidance product question.
- Completed C1's source-file split verification. Every tracked authored source passes the
  exclusive 1000-line limit; `src/style.css` is now 994 lines, and the shared role-color selectors
  live in the separately loaded and production-copied `src/styles/product_role.css`. The isolated
  production-style artifact check passed.
- Completed C211's conservative Question-revision boundary. Metadata-only title and description
  changes stay on the current lineage; unchanged source is rejected before successor writes, while
  changed source creates exactly one successor under concurrent publication. The PostgreSQL 17
  metadata/no-op/new-source/race probe is temporary evidence and does not invent behavior for the
  not-yet-persisted tags, subject, or topic fields.
- Completed C35's shared Profile-control evidence: Student, Instructor, and Sysadmin now have
  one far-right, accessible, icon-only generic Profile affordance, with the selected-avatar
  rendering seam retained where applicable. The one-time 1280/320 coarse-pointer and
  thumbnail-request-isolation probe was removed; this entry does not claim Profile-menu or
  avatar-persistence behavior.
- Completed C27's staff desktop evidence: the permanent responsive check covers Instructor and
  Sysadmin 1280-by-800 Ribbon behavior, including visible Sysadmin Instructor Accounts and Scoped
  Support controls. The one-time real-shell page/keyboard probe was removed. A fresh local rerun
  could not launch Chromium because the sandbox denied its macOS Mach rendezvous port; that is an
  environment limitation, not a product failure.
- Completed C2's development-conformance audit. It inventories current tracked and untracked
  source safely, enforces readable snake_case names, and rejects unsupported placeholder or
  compatibility scaffolding through exact durable-authority exceptions. The adversarial proof was
  removed rather than retained as a permanent implementation-coupled test.
- Verified C13's pre-existing Live Demo entry: the visible seeded-role selector creates ordinary
  sessions while its router deliberately omits email-code delivery. The one-time five-persona
  runtime proof was removed rather than retained as a permanent URL-specific test. Live Demo email
  remains a future capability unlocked by "yet"; this did not add product code.
- Recorded the architect-approved Sysadmin TOTP session decision in the durable
  design, contract, and Live Demo documents. Primary authentication now has a
  specified pending-MFA-to-session boundary for Sysadmins, while Student and
  Instructor sessions remain unchanged. The record requires a one-use,
  Account- and browser-bound, short-lived TOTP attestation; server-side
  counter/replay/rate protections; encrypted seed storage; and a genuine local
  operator artifact that is consumed by a separate authenticator and logged by
  path only, without a browser or fixed-secret bypass. It records C15
  implementation scope only; no production authentication behavior is claimed.
- Restored the supported default optional-sqlx build by adding the missing `#[cfg(feature =
"postgres")]` guard to the `course_blueprint_adoption` module declaration in
  `crates/learning-data-access/src/postgres.rs`. No product behavior or permanent test changed.
  The default `--lib course_roster` gate passed 2/2, the postgres `--lib connection_contract`
  gate passed 3/3, and format and clippy passed.
- Completed C3's dated direct-dependency freshness audit across Cargo, Node, and PyPI manifests.
  The snapshot covers all 43 direct dependencies; only the documented aws-sdk-s3 security-release
  and TypeScript compatibility exceptions remain, with their narrow blockers recorded.
- Accepted the A6 Milestone G planning completion: 37 unchecked records include two later
  duplicate pointers and 35 owning behaviors. Sixteen atomic implementation/evidence milestones
  use the safe C200-C216 range; C205 is the sole product question for the cohort/intersection
  statistic-release rule. The DAG and temporary-test-first policy received independent review.
  This records planning and audit work only; it does not claim implementation.
- Accepted the A5 Milestone G planning completion: 26 raw records (25 open and one N/A) yield
  23 owning product-behavior records with 22 closure milestones. Y10 is the sole product
  question; two Ribbon details remain HG-unlocked. Atomicity and temporary-test-first gates
  received independent review. This records planning and audit work only; it does not claim
  implementation.
- Corrected the A6 Student-data duplicate ownership pointer to its first Accounts-and-roles
  occurrence. The unverified status and finding remain unchanged.
- Corrected the A3 Product Role identity evidence locator to the shared Ribbon identity plate.
  Its verified status and behavior claim are unchanged.
- Completed C31's Atkinson Hyperlegible Mono delivery: code and monospace elements use the
  locally bundled normal and italic family, and the production build copies and verifies its
  assets. The one-time computed-style proof was removed rather than promoted as a permanent test.
- Corrected four A3 role-color checklist evidence locators after the shared CSS moved to
  `src/styles/product_role.css`. The verified statuses and product behavior are unchanged.
- Completed C34's role-home route contract: Instructor, Student, and Sysadmin now have explicit
  home routes, and the selected Courses navigation targets the signed-in role's route. The stable
  role-route contract test passed. The screenshot manifest deliberately records all three new
  role-home captures as deferred, so no visual-capture evidence is claimed yet.
- Accepted the A1 Milestone G correction map: 17 owning gaps map exactly once to C1-C12;
  C4 and C5 are `N/A` audit corrections under the Human Guidance classification rule. The
  cycle-free dependencies and executable temporary-first gates received independent review.
  This records planning and audit work only; it does not claim implementation.
- Accepted the A2 Milestone G2 correction map: 20 owning Accounts-and-roles gaps map exactly
  once to C13-C26. Reviewed owned boundaries span Live Demo authentication, roster identity and
  access, authentication, account lifecycle and Instructor vetting, Blueprint browse, Course
  retention, and Sysadmin/support authorization. The reviewed gates are the exact focused
  `cargo test` boundaries (`learning-data-access` authentication, roster, and connection
  contracts; `server_core` Instructor-account, Blueprint, roster, worker, and support-capability
  behavior), the named Live Demo E2E lanes, `npx playwright test
tests/playwright/e2e/auth_authorization.spec.ts`, and `./launchers/run_fast_checks.sh`.
  A2-08 remains one genuine product question: Human Guidance requires reluctant collection and
  deliberate use of Student data but does not choose between per-field/per-purpose allowlisting
  and qualitative category/operation-boundary enforcement; C22 plans only predictable purge.
  Proof begins in `tests/_temp/` and is promoted only when it meets `docs/PYTEST_STYLE.md`.
  This records planning and audit work only; it does not claim implementation.
- Accepted the A3 Milestone G3 correction map: all 36 Interface-shell findings are accounted for.
  Thirty-four have one closure owner in C27-C43 or C69-C71; the remaining exact drag-and-drop
  question stays a product decision because Human Guidance does not identify which named
  reordering surfaces would be faster and more natural with drag-and-drop. The backend-font
  permission is an audited `N/A`, not a missing implementation requirement. HCI acceptance is
  grounded in the precision-field-console and role task-model criteria, the role-neutral avatar
  decision is durable, and proof starts with temporary checks. This records planning and audit
  work only; it does not claim implementation.
- Accepted the A4 Milestone G4 correction map: all 87 individual Instructor-interface gaps map
  exactly once, with 22 closure owners and six one-boundary contributors across C44-C68 and
  C72-C74. There are no product questions. Danger Zone/archive behavior has one owner, and every
  gate is executable and temporary-test-first. This records planning and audit work only; it does
  not claim implementation.
- Accepted the A1 Development and Product vocabulary audit baseline. It records current
  implementation evidence and findings against the verbatim checklist; unresolved `[ ]` entries
  remain correction work, not a compliance claim.
- Accepted the A2 Accounts and roles audit baseline. It records current implementation evidence
  and findings against the verbatim checklist; unresolved `[ ]` entries remain correction work,
  not a compliance claim.
- Accepted the A3 shared interface shell audit baseline. It records current implementation
  evidence and findings against the verbatim checklist; unresolved `[ ]` entries remain
  correction work, not a compliance claim.
- Accepted the A4 Instructor interface audit baseline. It records current implementation evidence
  and findings against the verbatim checklist; unresolved `[ ]` entries remain correction work,
  not a compliance claim.
- Accepted the A5 Student and Sysadmin interface audit baseline. It records current
  implementation evidence and findings against the verbatim checklist; unresolved `[ ]` entries
  remain correction work, not a compliance claim.
- Accepted the A6 Data and history audit baseline. It records current implementation evidence and
  findings against the verbatim checklist; unresolved `[ ]` entries remain correction work, not a
  compliance claim.
- Accepted the A7 Questions audit baseline. It records current implementation evidence and
  findings against the verbatim checklist; unresolved `[ ]` entries remain correction work, not a
  compliance claim.
- Accepted the A8 Courses audit baseline. It records current implementation evidence and findings
  against the verbatim checklist; unresolved `[ ]` entries remain correction work, not a
  compliance claim.
- Accepted the A9 Assessments audit baseline. It records current implementation evidence and
  findings against the verbatim checklist; unresolved `[ ]` entries remain correction work, not a
  compliance claim.
- Reconciled the complete `docs/` corpus against Human Guidance as current product authority.
  Current specifications, contracts, guides, and active plans now use the same Assessment,
  Blueprint, Question, role, authorization, retention, grading, and interface models. Temporary
  reports inventory all 295 documentation files, preserve compatible detail and dated evidence,
  and separate the two unlocked UI designs from source-owned generated-artifact refreshes. The
  submission language now names only the whole Assessment Attempt as the submission target; saved
  Question responses are finalized together by that action. Assessment is the generic object with
  five Types, while Assignment is not an object or parent category. Practice uses the ordinary
  whole-Attempt boundary, Question Feedback is independent of correct-answer disclosure, unanswered
  Questions receive zero without backend evaluation, and the highest submitted Attempt score is
  used. Scoring uses Question points directly; pilot export is CSV or TSV without a Course-grade
  model. The KISS reconciliation also removed invented rollover, deferred grading, and permanent
  Account-closure models; connected the six-month maximum Active Course lifetime to FERPA retention
  by capping deadline movement while keeping inactivity and deletion as separate transitions;
  clarified that the latest Assessment deadline only starts the FERPA clock while configured policy
  controls later record transitions; and preserved bulk roster import for adding Students while
  limiting removal to one Student at a time. Course banners now use one responsive
  5:1 geometry, recommend 1280 by 256 pixels for authoring, and do not preserve the former 6:1 hero,
  5:2 card crop, or dual-rendition subsystem. Six fresh independent Plan, Test, Style,
  Documentation, Legacy, and Comment reviews
  corrected additional stale authentication, submission, scoring, adapter, and report claims. The
  reports retain their current naming/placement style issue and the limits of structural inventory
  as semantic proof. This was a documentation-only change; source code, schemas, tests,
  configuration, and migrations were not modified.
- Reorganized Human Guidance under the approved Development, vocabulary, Accounts and roles,
  Interface, Data and history, Questions, Courses, and Assessments hierarchy. A temporary exact
  comparison preserved all 744 bullet blocks without wording changes, and links to renamed current
  headings were updated. The focused Human Guidance format gate passed two tests.
- Cleaned only same-subsection Human Guidance duplication and the Course Instance ownership
  contradiction. Course Instances retain equal co-Instructors with no privileged first Instructor;
  Private Blueprint ownership and intentional cross-section reinforcement remain unchanged. A
  read-only follow-up found no remaining same-subsection duplication or move-created contradiction
  requiring correction, and the focused Human Guidance format gate passed two tests.
- Course Instance creation now adopts every Blueprint Assignment atomically, preserving exact
  Question Revision pins, fixed Questions, pools, points, instructions, and policies with fresh
  teaching identities. Adopted Assignments start Unreleased with dates unset. Removed Blueprint
  relative schedules from the model, API, seed content, and editor. Blueprint discovery now sorts
  by total adoptions or Students ever enrolled, counting each Student once per Course Instance
  even after leaving and rejoining. The fresh-schema isolated PostgreSQL Store probe passed
  adoption of fixed and pooled Questions, initial state, lifetime counts, and concurrent Revision
  checks. The complete offline Rust gate (including WebAssembly), frontend gate (373 tests),
  materialization unit test, and 276 focused Python checks passed. C-3 was not repaired.
- New Live Demo launches choose their HTTPS gateway port from 8000-8399 instead of
  55000-55399. Existing running demos retain their current URL. The focused target and
  developer-controller checks passed 30 tests.
- Clarified demo role choices with Instructor Dr., Student, and Sysadmin labels, sharing teal,
  lavender, and tomato role colors with the Ribbon. Blueprint overviews now list assignments;
  owners enter Course Editor and select one assignment before editing. Names and availability
  are expandable. A direct Create Course Instance action preselects the current Blueprint.
  Removed repeated unavailable-statistics copy and reduced desktop Question Library rows from
  164 to 112 pixels. The demo controller prints and flushes its URL before any browser opener.
  Fixed Blueprint input focus loss caused by remounting the editor on each local change.
  The frontend gate passed 374 Node tests, and focused controller/documentation checks passed
  255 tests. The existing live Blueprint browser journey passed Save, conflict, metadata,
  archive/restore, and unsaved-navigation checks. A temporary real-browser probe created a
  Course Instance from BP-2, verified one selected assignment editor, and found no axe violations
  on that editor or horizontal overflow at 320, 480, 768, and 1920 pixels. Captures are in
  `test-results/interface_polish/`; this is focused evidence, not full aggregate acceptance.
- M4 and Phase 2 are complete as of 2026-09-14. The fresh canonical runtime at
  `https://localhost:55390` published `BP-2`, Revision 1, in session 53170 with the r11 hashes:
  11 topics, 119 banks, and 20,579 rows. The reviewed r5 browser proof reloaded that exact
  Blueprint, created throwaway Course/Assignment records, visibly restored a saved radio selection
  after reload and a new authenticated Mary session, navigated directly to the same Attempt, and
  used the UI to submit wrong then correct responses with Student-history/Gradebook agreement.
  Root inspected the fresh Topic 11 screenshot as legible. The r4 outage proof recorded an actual
  submission `503` and plain copy, restoration before and after renderer recovery, accepted UI
  submission, and a healthy renderer. GET receipt R6 recorded only public API observations and
  reviewed SQL evidence of no Student Work/outcome writes; it does not claim a private database
  snapshot. This remains intentionally disposable, makes no production-deployment claim, and adds
  no target/deployment gate. The earlier r3/r4 installation was canonically removed before the
  aggregate; this is the current fresh publication receipt. Authorized closeout then deleted only
  `tests/_temp/genetics_bank_port` (208,331 files in 1,490 directories, about 3.59 GiB), with no
  archive or permanent fixture. The original 11-topic Biology Problems Website Genetics source,
  runtime/volumes/`BP-2`, and Git index were untouched.
- Corrected the vendored naming gate after aggregate session 68687 stopped following 6,003 passing
  pytest tests. It had applied the permanent Playwright-placement rule to `tests/_temp`, despite
  `PYTEST_STYLE.md` requiring temporary probes there. The narrow generic correction excludes only
  the `tests/_temp` subtree and keeps permanent checks unchanged; it adds no framework, fixture,
  or import workaround. Full pytest passed 6,004 tests and the final-path boundary gate passed
  five checks; independent review approved. The exact rerun
  `source source_me.sh && ./launchers/all_test.sh` passed in session 78713. Vendored propagation
  may overwrite this local correction; upstream modification is out of scope.
- Corrected the backend-owned Question iframe sizing demonstrated by the Genetics screenshot: its
  304 by 154 frame clipped rendered content. The existing iMathAS width/minimum-height rule now
  also applies to `backend-owned-document__frame`. This is CSS-only; it adds no Question Backend
  protocol, parser, or permanent test. Independent review, Prettier, TypeScript, and diff checks
  passed. Fresh desktop and narrow browser captures after the frontend rebuild verify full-width
  desktop rendering and reachable narrow-layout document scrolling. The later exact aggregate
  passed in session 78713, and the fresh r5 integration proof completed the normal saved/graded
  delivery evidence.
- The Phase 2 audit corrected durable guidance that still described public grading status,
  Instructor attention, retry/requeue behavior, and grading Jobs. Current guidance records one
  immutable backend credit fraction, read-time current-point scoring, ordinary expiry
  finalization, and only hidden backend-specific completion. It keeps public-asset Jobs separate.
  The aggregate `source source_me.sh && ./launchers/all_test.sh` exited 0 in session 36718, but
  the audit found a focused expiry Cargo filter that matched zero tests and obsolete
  grading-job/failure residue. Follow-up removed the stale private-table inventory catalog check;
  restored the Assignment-root update authority required by the expiry lock; and found that the
  corrected expiry test executed once, while its stale global-empty assertion conflicted with two
  intentionally retained expired Attempts from the preceding race oracle. The redundant
  `attempt_expiry_store_postgres` test and runner were removed under the test policy, with no
  replacement. Existing security-catalog, expiry-SQL, race, and no-browser journey evidence
  remain. The fresh exact aggregate `source source_me.sh && ./launchers/all_test.sh` passed in
  session 38324: 6,004 pytest tests, database-baseline E2E, installation-data provision and
  opt-out, Course Appearance PostgreSQL/MinIO, and complete live acceptance. Canonical cleanup
  was empty. Approved audit reviews cover the native/iMathAS unused-job cleanup, permissions and
  identity boundary, documentation, and stale-test removal. M2.5 is complete. M4 behavior and
  authorized temporary-corpus closeout are complete by the later fresh-runtime receipt.
- The Genetics preparation receipt now records the verified r11 conversion path: all six observed
  BBQ response shapes compile to opaque static PG, and private renderer proof passed all 20,579
  generated documents. Ordinary r11 publication then succeeded on the disposable runtime as
  Blueprint `BP-2`, Revision 1, with 11 topics, 119 banks, and 20,579 rows; its retained receipt
  was regenerated. The current fresh-runtime r5 proof established visible saved-control restoration
  after reload and a new Mary session, direct same-Attempt navigation, UI wrong/correct submission,
  and Student-history/Gradebook agreement. Fresh outage and GET receipts separately establish
  recovery and read-only document behavior. Session 78713's exact aggregate passed before fresh
  M4 startup with corrected PG baseline, installation provision/opt-out, Course Appearance
  PostgreSQL/MinIO, complete live acceptance, and empty owned inventory. Authorized closeout
  removed the temporary corpus/probe tree without creating a fixture or recurring inventory gate.
- Corrected the backend-owned resume boundary that the r4 evidence exposed. The existing
  authorized WeBWorK document read now carries the private saved opaque input and exact
  Question Attempt source pins to create an ephemeral resumed document; the original issued HTML
  remains immutable. It adds no API/table, expiry policy, GET write, outcome persistence, control
  parser, queue, or submission/grade action. The renderer transport still rejects answer/preview,
  process, and source-URL overrides. Independent review and focused gates pass, including
  temporary backend-only rendering of real r11 MC, MA, FIB, NUM, and duplicate-MA cases with
  restored visible controls and no feedback. The later fresh r5 PLE proof completed reload through
  a new Mary session and UI Submit. Session 78713's exact aggregate remains a separate receipt
  that predates fresh M4 startup.
- Corrected the Genetics walkthrough's retired automatic-grading Job wording. Its Attempt authority
  section now preserves saved responses across backend failure, permits Student submission within
  the existing time limit, and keeps expired Attempts closed to edits until ordinary finalization.
- M2.5 implementation and connected/live acceptance passed. The full registered Live Demo Attempt
  journey passed in session 26042 on the owned rebuilt stack: start authorization, exact released
  revision boundary, native immediate submission with immutable credit/current-point scoring,
  no-browser expiry, opaque WeBWorK capture/save/reload, outage preservation and recovery,
  immediate score, and Gradebook agreement. The database baseline passed earlier; latest Rust and
  pytest checks also passed. The exact aggregate `source source_me.sh && ./launchers/all_test.sh`
  passed in session 98225, including Rust, frontend, 6,039 Python tests, database baseline,
  installation-data provision and opt-out, Course Appearance/MinIO, and banner/thumbnail saga
  lanes. Genetics/M4 corpus and publication acceptance remain pending, so Phase 2 is not complete.
- Question Library search now decodes the browser's repeated query filter keys, including a
  one-value `backends=ple` filter, through the maintained multi-value Axum query extractor.
  Typed enum validation, unknown-field refusal, scalar duplicate-key refusal, normalization,
  and page bounds remain unchanged.
- The connected database baseline E2E passed in session 51122, including the direct-finalization
  late-save and commit/expiry races, immutable-credit/current-point/expiry-Gradebook SQL proofs,
  and the Unrelease fixture path. The canonical owned-runtime inventory was empty after cleanup.
  An independent final `./check_rust.sh` passed full all-feature Clippy, tests, and Wasm; the full
  `./check_codebase.sh` passed both TypeScript checks, lint, format, and 374 Node tests. Full
  aggregate, live-browser, and M4 acceptance remain pending.
- Corrected current Phase 2 fixtures and improved redacted diagnostics while preserving their
  bounded evidence boundaries. The refreshed offline gate `source source_me.sh &&
./launchers/run_fast_checks.sh` passed in session 37638: 6,039 pytest tests in 10.84 seconds,
  372 Node tests, and the Rust, TypeScript, lint, and formatting checks. `git diff --check` is
  clean. Connected and live acceptance remain pending; M4 acceptance is not yet approved.
- Raised only the Blueprint client response budget to 16 Mi characters through an optional internal
  constant after the real R8 response-shape estimate measured 15,526,628 ASCII characters. All
  other API clients retain the shared 4 Mi character default. This is a capacity correction, not
  paging or projection work, and adds no Genetics fixture. Seven generic protocol tests plus
  TypeScript, lint, format, and diff checks pass; real-browser 15 Mi character evidence remains
  pending. The two-connection harness uses a test-local pool of two with `CONNECTION LIMIT 2` for
  its holder observation, without a production-role change.
