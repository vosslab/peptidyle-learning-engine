## 2026-08-30

### Changes

- Replaced the authored-Published-Question picker label with My Questions. It resolves through the
  current Account's authored Question search instead of representing a separate Personal Question
  Library.

- Replaced the remaining active Question Corpus product labels with Question Library in published
  Question statistics and authoring publication UI. The shared installation-wide library remains
  the one authoritative Published Question surface.

- Replaced the retired Question Collection boundary with Question Folder throughout the fresh
  schema baseline, Rust model, generated browser contract, same-origin HTTP routes, curation UI,
  Question Picker, and focused tests. A Folder remains an Account-owned organization of published
  Question references; it grants neither Question authority nor editing authority.

- Aligned the current documentation set with the owner-selected Question Library model. The Product
  Ribbon has Courses, Question Library, Blueprint Courses, and Account; Question Library contains
  All Questions, My Questions, My Question Drafts, Starred, and Watched; and the Course Instance
  Ribbon contains Assignments, Students, Gradebook, Teaching Operations, Blueprint Updates, and
  Course Setup. Star is the visible endorsement relationship, Watch is the private notification
  subscription, and Question Folder is the Account-owned organizational structure. The terminology
  checklist now tracks the remaining code and schema corrections separately from the canonical
  contract.

- Replaced the shared Problem Picker with the Question Picker across its source package, accessible
  UI namespace, Assignment Editor integration, reusable-curriculum and Question Curation consumers,
  and focused Node test ownership. The canonical picker selects published Questions rather than
  introducing a separate PLE concept.

- Replaced the Assignment policy's Selected Problem Variants contract with Selected Question
  Variants. The Question Model, regenerated browser contract, strict decoders, policy UI, focused
  summary test, and current design documentation now use the exact Question term.

- Replaced the Saved Problem Search record contract with Saved Question Search. The Question Model,
  regenerated browser types, strict decoders, API client operations, and Question Curation state
  now use the exact owned record name and its edit number. The shared Question Collection,
  Saved Question Search, and Blueprint Course title-validation contract now carries the same
  Question Curation name. The Curation client, ETag, conflict error, state, panel, repository,
  picker composition, focused tests, canonical source paths, Playwright scenario, artifact IDs,
  scenario registration, and documented screenshots now use Question Curation names. The Saved
  Question Search HTTP route is now `/api/saved-question-searches` in the same client and
  real-stack wire-observation boundary.

- Replaced the `CatalogProblemSummary` and `CatalogProblemDetail` contracts and client operations
  with Catalog Question names. Rust, regenerated browser types, strict decoders, runtime cache,
  authoring consumers, and assignment lookup now use `CatalogQuestionSummary` and
  `CatalogQuestionDetail`. The answer-free search row is now `QuestionCatalogEntry`, combining
  that exact summary with disclosed discovery evidence in Catalog search and reusable-curriculum
  projections. Shared HTTP fixtures now construct the exact Question Version and issued-
  presentation envelope contracts used by those strict decoders.

- Replaced the Catalog detail route's PLE-owned Problem vocabulary with Question vocabulary.
  `QuestionDetailPage`, `questionDetail`, `questionRef`, and `QuestionRouteReference` now carry
  the `/library/:questionRef` boundary through route contracts, navigation validation, browser
  surfaces, and focused tests.

- Rebuilt the database structure map from the active `2026082901`-`2026082936`
  pre-production migration baseline. It now names current physical owners and removes the
  deleted-migration inventory that incorrectly claimed retired files and group policy tables existed.

- Removed the disconnected `learning-data-access` conformance corpus, including its stale synthetic
  preview-group fixture. Hypothetical Student View construction carries only its selected moment and
  explicit direct modifiers; the current feature-gated package suite runs its nine registered tests.

- Replaced the Pool Draw Basis's generic `group` selector with the exact Question Pool Assignment
  Entry, and aligned the editor's local Question Pool capacity calculation with the same term.

- Replaced PLE-owned family wording with the exact Question Backend, Question Format, and
  Operation Kind terms. The preview model and current authoring, catalog, prefetch, authorization,
  and Product Role documentation now preserve those distinct boundaries.

- Replaced the Question Collection member's invented retained-selection state with the exact
  Current Question Version Availability. Generated contracts, strict decoding, collection filtering,
  and visible curation UI now use the shared Available or Archived availability fact and its
  archived reason.

- Closed the retired `NavigationResolution::Run` checklist row. The current closed
  `AssignmentAttempt` route-resolution variant carries the exact Course Instance, Assignment,
  Student Record, and Assignment Attempt identities.

- Closed the retired Run route vocabulary in the retained checklist. Active code uses Assignment
  Attempt references, route parameters, page owners, and recovery owners; durable architecture
  documentation now names the current Assignment Attempt page rather than deleted `run_page` paths.

- Replaced `GradingOperationState` with the exact `InstructorGradingOperationState`. The closed
  state now names only Instructor-requested recovery work across the Rust model, regenerated
  contracts, and strict Gradebook decoding, separate from automated grading, scoring, and Job state.

- Separated the generic Course Invitation from the Instructor-only Teaching Team operation. The
  durable invitation retains its exact Student or Instructor Course Membership Role, while the
  Instructor Course Invitation request, view, paths, generated types, strict decoder, and visible
  interface are explicitly Instructor-only. Account-pending acceptance remains the generic Course
  Invitation boundary.

- Replaced `GradingOperationReference` with `InstructorGradingOperationReference` for the exact
  Instructor-requested recovery-work locator. The `GO-` reference is now explicit throughout the
  Rust model, regenerated browser types, Gradebook and workspace clients, strict decoders, route
  helpers, and focused behavior tests.

- Replaced `WorkspaceReference` with `AuthoringWorkspaceReference` for private draft-authoring
  routes. The `W-` locator now names the Authoring Workspace through the Rust model, regenerated
  browser types, strict decoder, editor navigation, public-navigation tests, and identity contract.

- Replaced `CourseReference` with `CourseInstanceReference` across the live-teaching locator
  boundary. The `C-` reference now names a Course Instance throughout the Rust model and
  curriculum-adoption contracts, regenerated browser types, strict decoders, typed clients, route
  helpers, tests, and durable terminology documentation.

- Replaced generic `BlueprintReference` with `BlueprintCourseReference` across the reusable-course
  public route, curriculum-adoption contracts, regenerated TypeScript, strict browser decoding,
  typed clients, and durable documentation. The `BP-` locator remains the exact reusable Blueprint
  Course reference; focused browser consumers pass.

- Replaced the Question Collection locator's legacy `PC-` wire prefix with `QC-`. The existing
  `QuestionCollectionReference` now has matching public terminology through the Rust contract,
  regenerated TypeScript, strict decoder, typed clients, and curation UI tests.

- Replaced the saved-catalog locator's legacy `SavedProblemSearchReference` with
  `SavedQuestionSearchReference`. The canonical `QS-` wire prefix now flows from the Rust public
  route contract through regenerated TypeScript, strict browser decoding, typed clients, and tests;
  focused Rust and browser-contract tests pass.

- Removed the redundant `ProblemDisplayRef` wrapper from the catalog contract. The canonical
  `QuestionId` now directly represents the version-free, instructor-entered public lookup value;
  catalog behavior tests (9) pass.

- Replaced generic visual grouping names with their rendered responsibilities: the shared CSS
  inset radius is `--ple-radius-inset`, and the theme mix feeding `surfaceSoft` is
  `softSurfaceCanvas`. All style consumers now use the canonical token; TypeScript and the three
  focused theme-token tests pass.

- Replaced the local-stack service-login provisioning API with the exact Service Login Setup
  boundary. `setup_service_logins` and its bounded failure contract now describe disposable
  database-login creation or refresh in the controller, lifecycle, focused tests, and operations
  guide; the 42-test local-stack suite confirms least-privilege URL handling and credential
  redaction.

- Completed an evidence-guided documentation pass for PLE-owned family terminology. Engineering
  contracts now distinguish Question Type, Question Backend, Native Question Implementation, and
  Job Kind while preserving `HUMAN_GUIDANCE.md`, relational-table wording, and platform terms in
  their own scopes. Markdown-link and focused native-preview checks pass.

- Closed the retired native-registration failure names in the vocabulary checklist. The Native
  Adapter exposes explicit Unknown, Duplicate, and Incompatible Question Implementation errors
  keyed by Question Format, Question Type, Question Generator, and implementation release; its
  35 behavior tests and six documentation tests pass.

- Replaced the worker documentation's generic family vocabulary with the closed Job Kind model:
  a Job Kind Registration joins the Job Handler and Effect Committer, each queued item has its
  typed Job Payload, and prepared output is a Prepared Effect. The durable `handler_kind` schema
  and authorization documentation now use the same terms.

- Repaired the committed native-question corpus at the current source boundary. Native source
  fixtures now use the closed unit `Native` variant with explicit Question Format and Question
  Type; the catalog summary now records current Question Version Availability instead of retired
  lifecycle state. Wasm preview, browser-model, and project-tools corpus reproduction/grading
  tests pass.

- Replaced Score reconciliation with the exact Recalculation boundary. The pure domain selector
  now describes batch grade selection during recalculation, while immutable accepted submissions,
  prior results, and receipts retain their separate evidence roles. Completed native-source test
  fixtures use the closed unit `Native` source and explicit Native Algorithmic Question Format.
  Targeted Rust formatting, focused domain tests, and whitespace checks pass.

- Replaced the fresh SD1 object-delivery reconciliation record with a completed Object Storage
  Check, keeping only verified, missing, or mismatched results. Object Cleanup now has an exact
  manifest with a permitted disposition and worker Job plus a separate immutable cleanup receipt.
  The allocation and migration filename now name Object Storage Checks and Object Cleanup; the
  vocabulary checklist and operational documentation use the same terms. The disposable
  PostgreSQL 17 baseline gate passes fresh apply, no-op reapply, catalog/ACL checks, and restricted
  login probes.

- Replaced the grading-operations list's generic grouping boundary with a closed Grading
  Operation Focus and exact Grading Operation Subject. The strict decoder, same-origin query,
  client contract, pagination state, controls, tests, and API contract now use `focus` and
  `subject`; visible controls say `Show by Question` or `Show by Student`. The focused browser
  contracts, TypeScript check, and whitespace gate pass.

- Replaced the browser catalog aggregate's generic `group` field with `facet`. Library and
  Question-picker state now carry the exact Question Catalog Facet plus its server-calculated
  value and count. The focused picker tests, TypeScript check, and whitespace gate pass.

- Unified Assignment composition around one ordered `AssignmentEntry` collection with closed
  Fixed Question and Question Pool variants. The browser, strict decoders, fixtures, generated
  contracts, and Question Model now use that direct boundary; entry and candidate collection
  order replaces duplicate stored positions. Issued Questions name their source Assignment Entry
  and optional source Question Pool entry, while pool-preview requests locate a saved Question
  Pool by Assignment Entry reference. Question Model tests, focused browser-contract tests, the
  TypeScript check, and whitespace gate pass.

- Completed the Question Response/Format and Native Question Implementation cutover. Durable
  Question definitions, authored-source schema, native registration, flat-question validation,
  generated browser contracts, strict decoders, authoring client, fixtures, and focused
  documentation now distinguish Question Format, Question Type, optional Question Generator, and
  the Native Question Implementation. The retired source-family discriminator is absent from
  current code and schema. Rust Question Model/Grading/Native Adapter tests, focused browser tests,
  TypeScript compilation, the Markdown-link gate, and whitespace gate pass.

- Converted the PLE vocabulary replacement map into a retained completion checklist. All 364
  correction rows now start unchecked, each row has one evidence-based completion rule, completed
  rows remain visible, and a final repository-wide audit governs retirement of the complete
  checklist. Updated the terminology contract to describe the same lifecycle.

- Reviewed the persistent-ribbon plan against the current UI and domain contracts. Expanded the
  terminology contract with Application Shell, Ribbon Scope, Ribbon Context/Tab/Task rows, Ribbon
  Slot availability, Selected and Loading presentation states, Content Layout, Page Action, and
  precise product-surface labels. The new vocabulary separates navigation from operations, Account
  state from route selection, and presentation availability from authorization; the source plan
  remains unchanged.

- Renamed the identity-free cohort-statistics domain API to Question Cohort Rollup, Question Cohort Rollup Observation, and Question Cohort Rollup Snapshot. This keeps cohort difficulty/timing/discrimination data distinct from exact Question Version Statistics; focused domain tests and strict Clippy pass. Registered the Question Version Statistics migration as the 36th current SD1 baseline file across the sole migration-allocation registry, database inventory, authorization reference, and roadmap. Aligned retention documentation to the current SD1 baseline: Course Retention Plans and immutable lifecycle events exist, while worker execution, cleanup manifests, access fencing, browser routes, and connected acceptance remain explicitly incomplete. Made Question Version Statistics observation time derive from the immutable Automated Grading Receipt, so a caller cannot select global-count evidence timing. Updated durable schema and contract documentation to the exact per-version count, eligible-choice count, and idempotent receipt relations. Removed the unmounted client-address attribution, deferred cookie-response, unused UUID-storage, unused cursor, and duplicate pilot-manifest helpers exposed by the SD1 fresh-baseline build. The mounted session route, opaque-session ownership, current source validation, and their permanent tests retain their existing boundaries. Corrected the Local Stack Controller state boundary: the tracked `local_stack_control/` package now writes only to the ignored `local_stack_state/` root, and browser-suite lease constants distinguish the controller's complete state root from its live-demo browser state directory. Updated focused tests and operations documentation; the former spelling remains only in the active vocabulary replacement map. Replaced the browser route and Gradebook transport spelling for Assignment Attempts: public paths now use `/assignment-attempts` and their parameter names identify an Assignment Attempt directly. Replaced human-account and live-course terminology with Account Creation and Course Instance Creation across the active schema commentary, role documentation, authorization reference, and SD1 allocation registry; updated browser enrollment to the canonical Accepted Course Invitation State and Course Invitation Email Domain while retaining the separately named redemption result. Added immutable Blueprint Publication, revision-scoped Blueprint Collaborator, and Blueprint Revision Availability Events, so only an Approved Instructor can contribute to an exact Draft Blueprint Revision and archival applies to exactly one published revision. Disposable service-login setup retains its separate infrastructure term.

- Completed the direct Assignment Attempt browser terminology cutover: page and session-recovery owners use Assignment Attempt filenames, public route parameters use `assignmentAttemptRef`, Gradebook inspection uses the same reference, and visible inspection failures name the Assignment Attempt. TypeScript compilation, 12 focused browser-contract tests, and the whitespace gate pass.

- Replaced the mutable Workspace Collaborator row with immutable start and end Events. Workspace authorization derives the current exact Authoring Workspace relationship; it grants neither Blueprint Course nor Course Instance authority. Replaced the one-per-workspace mutable draft row with immutable Draft Question Revisions and exact revision-bound Question Source and Question Grading Material. Renamed private collection and search records to Question Collection, Question Collection Entry, and Saved Question Search with explicit Edit Numbers; the Rust model, generated transport, browser decoder, state, ETags, and UI now expose that exact contract. Replaced mutable Course Observer state with immutable start and end relationship events, including an enforced start-before-end transition. Replaced root-level Change Proposal patches and legacy stewardship labels with immutable numbered Question Change Proposal Revisions and exact opened, merged, closed, or Forced Question Correction history. Split Question Catalog publication history from current Question Version Availability across schema, Rust, generated transport, browser decoding, and permanent database acceptance. Corrected the Roadmap and durable contract evidence to the one current SD1 baseline, marking unfinished statistics and retention behavior honestly. Renamed the current migration and schema documentation to Assignment Attempts and Issued Questions; Issued Question now retains its issue-time Question Statistics Eligibility in the SQL baseline and acceptance oracle. Added the exact accepted-grade Question Version Statistics aggregate and validated grading-to-observation boundary, retaining correct and eligible-choice counts separately from cohort rollups. Repaired durable-document links to the current application composition, PostgreSQL session, Assignment-policy, and Assignment-Attempt owners. Renamed the Question Publication and Question Version Availability baseline migration to its exact two-event responsibility.

- Replaced role-specific teaching-participant wording with Teaching Team Member throughout durable product documentation and authority comments. Teaching authority now follows the exact active Instructor Course Membership, while Course Invitation identifies pre-membership invitation flow.

- Removed two unmounted conformance-driver calls whose fixture module was already absent from the current tree.

- Replaced the role-specific teaching-team invitation transport contract with the canonical Course Invitation contract across Rust, generated TypeScript, browser clients, routes, and teaching-team views. The exact Course Membership Role now distinguishes Instructor and Student invitations.

- Restored the one-current-role Course Membership invariant in the event ledger. The transition trigger serializes exact Course/Account/role writes, admits only Started-to-Ended history, and refuses concurrent duplicate active membership episodes.

- Replaced the durable database documentation's removed roster, enrollment, and assignment-delivery relation names with the actual Course Membership, Course Invitation Event, Assignment Revision, Student-evidence, Gradebook, and retention relations in the fresh baseline.

- Replaced the Rust authority contracts' mutable Instructor Approval and Course Invitation timestamp fields with explicit immutable event projections. The native domain derives closed authorization and invitation states from those events and preserves the target-bound validation cases.

- Replaced independent mutable Course Invitation terminal timestamps with one immutable accepted, declined, or revoked Course Invitation Event. The schema rejects skipped, late, duplicate, or wrong-target transitions, and derives Pending or Expired from the absence of a terminal event and the invitation deadline.

- Replaced mutable Course Membership revocation with immutable started and ended Course Membership Events. The exact Course Membership episode and Student Record history remain stable; all current-course authorization predicates now derive Active membership from the event ledger.

- Replaced the mutable Instructor Approval row with immutable approved and revoked Instructor Approval Events. The Course Observer authorization predicate now derives current approval from the latest event, and the permanent database oracle requires the event ledger and its immutability trigger.

- Renamed the generic Gradebook control record to immutable Assignment Grade Event evidence, linked directly to one Assignment Grade Calculation. The permanent database oracle rejects the retired audit relation.

- Replaced the Course Instance Blueprint-adoption relation with immutable Course Origin evidence, retaining exact Blueprint Revision provenance and an optional source Course Instance for rollover origin. The permanent database oracle now rejects the retired relation.

- Added immutable Account State Events: Account creation records Active, authentication requires the latest Active state, and Suspended or Closed events revoke current Authenticated Sessions. The permanent database oracle requires the state ledger and both enforcement triggers.

- Split catalog history into one immutable Question Publication Event per Question Version and separate immutable Available/Archived Question Version Availability Events. The permanent database oracle now refuses the retired mixed lifecycle relation.

- Split Gradebook persistence into immutable Assignment Grade Calculations and a separately selected Assignment Grade. The selected record now uses an exact Student Record-and-Assignment calculation reference, and the permanent database oracle requires the calculation immutability and parent-matching constraint.

- Added database-enforced immutability for accepted Question Submissions and Assignment Submission receipts, preserving the exact Student Response and attributed finalization evidence used by automated grading. The permanent fresh-database oracle now requires those triggers and the Assignment Attempt revision-parent constraint.

- Added immutable, numbered Assignment Revisions with one complete authored definition and an exact revision pin on every Assignment Attempt. A composite foreign key proves that the pinned revision belongs to the same Assignment. Later teaching edits can now create successor revisions without reinterpreting issued Student work.

- Merged Course Instance delivery fields into the canonical Assignment relation and rebound export, external-tool, and Job foreign keys to that Assignment parent. The baseline no longer retains a duplicate Assignment-delivery ownership layer.

- Renamed the active Sysadmin-only Account Creation broker and immutable creation time to `create_account` and `created_at`; its return contract now names `product_role` directly. The clean PostgreSQL baseline contains no provisioning-shaped Account terminology.

- Removed the unapproved Student Observer schema authority. The fresh baseline now models only the owner-approved Course Observer Relationship; its trusted predicate requires current Instructor approval and refuses that read path for a current Teaching Team Member. The fresh PostgreSQL 17 staged migration and idempotent verification pass.

- Replaced the oversized terminology ledger with the focused terminology contract: global Accounts, role-distinct Authenticated Sessions, exact Course Membership and Student Record authority, immutable content and delivery records, and explicit authorization inheritance paths now share one concise vocabulary authority.

- Rotated the completed 2026-08-28 changelog block to `CHANGELOG-2026-08a.md`; the active changelog retains the two newest dates.

- Removed the unsupported SMTP overlay and its invitation-delivery worker. The local controller now has one Compose topology and starts only its current PostgreSQL, MinIO, renderer, API, and gateway services. Operational, live-demo, and deployment-contract documentation now describes that topology; focused local-stack tests pass (154).

- Removed the unmounted automated-grading fault profile, its Compose overlay, dedicated browser recovery scenario, and retained screenshots. Browser recovery now uses its single gateway-outage path and the declared live-demo evidence matches the API-and-gateway topology; focused Python contracts (43) pass.

- Reduced the fresh PostgreSQL baseline to its mounted API/session authority: removed worker, recovery, fast-path, and automated-grading pool factories, login contracts, local credentials, role creation, ACL grants, and dependent RLS policies. The staged-database and cross-store runtime fixtures now emit and validate only the migrator URL; focused lifecycle (43), PostgreSQL (27), Rust acceptance-runtime (17), and Python runtime-manifest (18) suites pass. The disposable PostgreSQL 17 staged acceptance gate also passes fresh apply, idempotent reapply, catalog/ACL checks, and restricted API/session probes.

- Removed the ordinary grading-worker service from the current Compose and browser-stack lifecycle. Its command target and server implementation were already retired, so the standard local stack now starts only its mounted API and gateway services. The local provisioning path now creates only the API login and no longer emits retired worker or execution credentials; focused lifecycle and ownership suites pass (98).

- Updated current test boundaries after retiring the unreachable Base Course installer, removed its stale browser-install receipt validator, and preserved the human font preference as one guidance bullet. Boundary, guidance-format, and Python lint gates pass (124).

- Retired the dead local Chapter One publisher and manifest, which invoked the removed E2E-seed command and retained obsolete Question UUID fields. The tracked pilot corpus remains owned by its current validation command; the focused local-stack suites pass (82).

- Removed the local-stack Base Course lifecycle command, credentials, receipt, cleanup manifest, and dedicated tests. The controller now follows its current migration, process-login, object-store, renderer, Chapter One, and API initialization stages; its focused lifecycle suite passes (39).

- Reduced the server tree to its mounted global-account, session, Live Demo, health, request-lifecycle, and HTTP-security surface. Removed unmounted catalog, course, grading, run, import, worker, and workspace trees; server tests, binary tests, doctests, and workspace compilation pass.

- Removed dormant E2E-seed and PostgreSQL-store helper files from project tools. The maintained command surface remains database staging, fixtures, pilot-content validation, bindgen, and TypeScript generation; all 40 project-tool tests pass.

- Reduced Learning Data Access to its mounted clean-baseline surface: account authentication, sessions, pagination, store errors, and PostgreSQL support. Removed dormant curriculum, roster, grading, import, curation, and in-memory trees that described a separate unreached persistence architecture.

- Removed the unmounted server composition subtree for retired background workers, object publication, invitations, and storage topology. The active server remains the focused global-session and Live Demo boundary; its 27 unit tests and the workspace compile pass.

- Removed the unmounted publisher and invitation-delivery role profiles from the PostgreSQL connection contract. The baseline now exposes only current process identities, and its feature-gated connection tests continue to pass.

- Removed the obsolete Base Course installer, application-pool, and execution fast-path PostgreSQL connection contracts. The retained API, worker, and sealed execution identities continue through the current baseline; the PostgreSQL-feature Learning Data Access suite passes (31, one disposable database acceptance check ignored).

- Removed the unmounted legacy persistence, E2E-seed, and server-worker files that retained `ProblemId`, `VersionId`, or `ProblemVersionRef`. The tracked Rust, browser, and generated source now have zero retired Question-identity references; full Rust, fixture, and strict TypeScript gates pass.

- Retired migration checks and worker entry points that required the deleted pre-reset schema epoch. The application verifier now reads the current `ple_api.ple_migration_state` projection as `ple_app`; focused Learning Data Access tests pass (11) and `server_core` compiles against that baseline.

- Updated the WASM presentation-descriptor fixture to carry the canonical Question ID and Question Version Number pair. Its focused test and the full Rust workspace suite pass.

- Removed the unmounted Base Course installer and its unreachable `cargo tools` adapter. The incomplete crate required an absent publication module and removed persistence interfaces, so it could neither install nor validate the current single-installation baseline. The mounted schema and live-demo lifecycle remain the authoritative installation path.

- Replaced the Question Model's duplicate UUID publication identity with the canonical `QuestionVersionReference { question_id, version_number }`. `QuestionVersionNumber` validates the positive monotonic per-Question integer, Question definitions and lifecycle states carry the canonical pair, and answer-free presentation fingerprints bind both values. The Question Model's 147 unit tests pass with the updated contract.

- Propagated the canonical Question Version pair through Object Storage and the Native and WeBWorK adapter paths. Question source, asset, restricted asset, and render keys now bind one `QuestionVersionReference`; deterministic object addresses and render fingerprints include both Question ID and Version Number. Object Storage tests pass (23), and WeBWorK tests pass (27; seven opt-in loopback checks remain explicitly ignored).

- Propagated the canonical Question Version pair through iMathAS provider integration. Provider grade correlation, persisted restart handles, scored-embed ledgers, and render-cache keys now bind an exact Question ID and Question Version Number. The focused iMathAS adapter suite passes (18).

- Regenerated the browser API contract from the canonical Question Model and updated its strict delivery decoders and WebAssembly capability boundary. Browser Question Version references now use `questionId` and positive `versionNumber`; the retired UUID fields have no generated or browser source contract. The Domain suite passes (100 unit tests plus two integration tests) and strict TypeScript compilation passes.

- Updated Object Storage conformance and published-import archive fixtures, the Grading fixture, and the tracked export corpus to use exact `QuestionVersionReference` values. Object Storage tests pass (28 across unit and integration suites), Grading tests pass (6), and the export suite passes (12; four external-reader checks remain opt-in).

- Converged active source, browser copy, tests, and durable documentation on **Student** terminology. The retired learner wording no longer names current PLE roles, Student-facing work, or Student-safe projections.

- Renamed the server-only PostgreSQL `ple_grader` capability to `ple_automated_grading` and its connection contract to `AutomatedGrading`. The automated-grading service remains a separately attested least-privilege capability, while **Grader** remains the reserved future human Course Membership Role from Human Guidance.

- Repaired the Student Record ownership boundary. A Student Record is now the stable `(Student Account, Course Instance)` educational record, while each Student Course Membership episode binds to that record. The baseline schema, capability broker, domain ownership checks, terminology contracts, and staged PostgreSQL acceptance gate now preserve that model across re-enrollment.

- Replaced the public navigation `run` variant with `assignmentAttempt` and `assignmentAttemptId`. The Rust route contract, generated browser type, strict decoder, and resolved-route consumer now carry the exact Assignment Attempt identity; a focused decoder test rejects the retired wire shape.

- Replaced the scored-completion model's generic run identity with explicit **Assignment Attempt** terminology. The pure grade selector, Store grade recalculation contract, browser completion copy, and focused regression tests now identify completed Assignment Attempts and their selected score directly.

- Converged the derived activity model on **Assignment Progress**. The Student-safe generated contract is `AssignmentProgress`; the identifier- bearing server projection is `AssignmentProgressRecord`. Rust consumers, browser decoders, generated API types, fixtures, tests, and current design documentation now distinguish that derived view from `AssignmentGrade` and immutable Assignment Attempt evidence.

- Replaced the retired Assignment Attempt completion-status type and receipt field with direct `AssignmentAttemptCompletion` and `assignmentAttemptCompletion` contracts. The completion derivation, strict browser decoder, attempt-state machine, terminal presentation, and focused fixtures now separate completion from successor-attempt availability.

- Renamed the Student-generic score state to `AssignmentProgressScoreState`, binding its no-activity, withheld, and available values to the derived Assignment Progress contract and the Student Feedback release decision that controls score visibility.

- Removed the unconsumed `domain::attempt` state-machine model and its tests. Durable Question Attempt state, Question Submission, Grading Result, and derived Issued Question Progress now remain the sole documented authority for question activity rather than a competing mutable lifecycle abstraction.

- Removed unreachable mixed submission-completion source that represented an internal persistence bundle as `CompletedSubmissionReceipt`. The active Store has no such module; a future Question Submission Receipt will be added only with its exact immutable evidence contract and consumer boundary.

- Replaced the baseline schema's duplicate UUID `problem_id`/`version_id` identity with the canonical immutable `(question_id, version_number)` pair. Every catalog, issued-work, worker, external-tool, correction, analysis, and object-delivery foreign key now preserves that exact Question Version.

- Replaced the public `RunPolicies` contract with `AssignmentActivityRules`. Assignment configuration and generated browser contracts now name the four independent rules directly. The corresponding Rust module and pool-draw variant now name Assignment Activity Rules and an Assignment Attempt, while the glossary terminology is reflected in the Question Model, Instructor, API, and mastery-design documentation.

- Made authenticated-session issuance derive the immutable Product Role from the existing global Account. The Rust `SessionStore`, PostgreSQL broker, and Live Demo boundary no longer accept a caller-selected role; the resulting session validates the configured demo persona against the role returned by the protected Store. Focused Rust tests and the staged PostgreSQL acceptance lane pass.

- Replaced the phantom SD1 ownership-map reference in the design decisions with the maintained database and service authorization contracts. Those documents now explicitly own the exact authorization predicates used by the schema, Store, brokers, and acceptance evidence.

- Recorded the fixed-role session-issuance rule: passwordless authentication establishes an existing Account, and the future trusted session broker derives that Account's immutable Product Role instead of accepting it as an independently selectable input. The current broker signature remains explicit pending SD1 implementation work.

- Replaced the remaining live documentation use of **Account Role** with the canonical **Product Role**. Account classification is now consistently distinguished from Course Membership Role, while `AccountRole` remains named only where it identifies the current Rust implementation.

- Reclassified the API, concurrency, and QTI contract documents around the current mounted production surface. Readiness, Authenticated Session, and the deployment-gated seeded Live Demo selector are the available route families; Store-backed delivery remains explicit target work with its concurrency and acceptance requirements preserved. The repository-wide Markdown link gate passes (184 documents).

- Replaced active Instructor-facing `learner` copy with **Student** in the roster, Assignment workspace, policy preview, catalog statistics, authoring, course-list, cookbook, Instructor, Student, frontend, visual, and prefetch surfaces. PLE's three human roles now remain visible in the product vocabulary while wire-field migration remains separately owned.

- Aligned the browser client with its actual **Course Roster** contract and removed retired enrollment-route authority from the deferred full-route policy. Course Enrollment remains the specific act that establishes a Student Course Membership and Student Record, rather than the name for the roster browser boundary. The roster client test and TypeScript check pass.

- Corrected the private Base Course and Chapter One manifest contract to carry `studentRecordId`, matching its existing Rust `StudentRecordId` source and the Student-owned delivery model. Lifecycle validation, publication evidence, and the replica-restart oracle now reject the retired enrollment-shaped field. Focused Python lifecycle/publication tests (25) and replica tests (5) pass.

- Replaced the grading-operations browser contract's `learner` grouping with the glossary's **Student** focus: `student` is now the closed query and row variant, and `affectedStudentCount` identifies its exact count. The strict decoder, Instructor controls, focused contract tests (6), and TypeScript check pass.

- Removed the retired WebWork and replica live-demo service oracles. Their shared owner and child implementation left with the clean mounted-route reduction, so their aggregate lanes, dead helper tests, and stale docs could not provide real acceptance. The current WebWork browser scenario and database/object service evidence remain; a successor replica oracle follows the fresh Store and mounted course-delivery contracts.

- Removed the unmounted browser-owner helper family that depended on the same deleted production-browser owner. The fast test suite now validates only importable current helpers; the focused import and Python static checks pass (248 tests).

- Removed the browser shell front doors and aggregate browser lane that invoked the deleted owner. Development, usage, and test-tier documentation now place browser and screenshot acceptance behind the fresh Store-backed delivery reconstruction instead of presenting a missing executable path.

- Renamed the closed Assignment-content conflict literal to `issuedStudentWork`. The Rust contract, regenerated browser type, strict decoder, Instructor recovery state, and focused transport test now identify the actual Student-owned record that blocks structural mutation.

- Removed the remaining PLE-owned institution-shaped vocabulary from active source, pilot provenance, and catalog assertions. Shared Question taxonomy now names approved Instructors, external-provider composition names its actual boundary, and imported content records a source affiliation rather than a product partition. The `cargo tools pilot-content` validator now mounts its tracked-provenance check through the command host.

- Renamed the public curation contract from **Problem Collection** to **Question Collection**. The route family is now `/api/question-collections`; Rust projections, generated TypeScript, browser client and picker contracts, route policy, tests, and current documentation use the glossary term.

- Renamed the Student-facing Assignment time-limit wording from "per run" to "per attempt," matching the canonical Assignment Attempt record and removing the retired Assignment Run product term from the active presentation API.

- Removed the duplicate persistent Favorites collection feature. Private Question Collections are now named collections only; the Rust/browser contracts, generated API, route policy, picker, curation UI, and focused tests no longer model a second personal endorsement list. The glossary's Star relationship remains the sole endorsement concept.

- Replaced the roster's self-enrollment-shaped policy with the **Course Invitation Email Rule**. The route, client contract, decoder, and Instructor form now control only the exact email domains applied to Instructor-issued Course Invitations; the removed signup posture no longer implies a separate enrollment path.

- Renamed the PLE-owned iMathAS test-provider reference from `institution-imathas` to `self-hosted-imathas`. Provider references now describe the configured integration rather than implying an institution boundary; recorded adapter fixtures and strict HTTP decoder coverage follow the same contract.

- Renamed the course roster form and its browser scenarios from "Institutional email/student ID" to **Course roster email/ID**. These fields remain protected, course-scoped metadata for invitation and export matching, rather than installation, account, or authorization boundaries. Retired Course Group and broad-collection wording was removed from the cookbook.

- Replaced the broad `institution` Question Collection visibility with private Instructor ownership. The collection API, generated model, browser UI, decoder, focused tests, and browser scenario now omit that non-product boundary; Question Collection Shares remain a future explicit-recipient capability. Catalog statistics now describe usage across the single PLE installation.

- Renamed the direct Student policy-mutation contract to Accommodation: `AccommodationId`, `AccommodationPatchUpdateRequest`, and the `/accommodations/{student}` route now replace the retired exception vocabulary. The route-policy matrix no longer retains any Course Group schedule or Accommodation paths. Regenerated TypeScript, `question_model` (147), `server_core` (26), and TypeScript checking pass.

- Completed the Course Group clean break across the compiled server, Store policy boundary, browser routes and decoders, generated API contract, and current documentation. Direct Student Accommodations now provide the only student-specific policy source. The retired browser scenarios were removed pending direct-Accommodation acceptance coverage. `question_model` (147), `server_core` (26), focused browser-contract tests (11), TypeScript checking, and the diff-whitespace gate pass.

- Replaced group-scoped effective-policy resolution with direct Student Accommodations. An active Student Course Membership now grants Assignment access without a group-derived policy scope; accommodations are bound to the entitled Student, and only an identity-free hypothetical accommodation is available to previews. The focused domain suite passes 106 tests.

- Expanded the canonical terminology contract and implementation replacement map across Question identity, immutable revisions, Assignment activity, authoring, enrollment, background work, external tools, retention, and Object storage. Question Versions now use a monotonic number scoped to the stable Question ID, with publication time retained as separate metadata. Focused Markdown link checks pass for both terminology documents.

- Replaced Assignment Audience authorization with direct active Student Course Membership access. Assignment records, workspace policy contracts, preview projections, and generated browser types no longer carry a course-wide or group audience selector; the retired `AssignmentAudience` model is deleted. The active domain suite passes 124 tests and the preview transport suite passes five tests.

- Removed Course Group management from Teaching operations and group audiences from the Assignment Policies contract. The retained UI now owns teaching team and retention work; every Course Instance Assignment uses direct active Student Record access. Regenerated TypeScript, six focused Rust workspace tests, 19 focused browser tests, and TypeScript checking pass.

- Aligned the current Activity, Assessment Payload, Frontend, Solid, Identity, Mastery, and Contract documents on the direct Student Record -> Assignment Attempt -> Issued Question -> Question Attempt chain. Assignment Activity now names its independent policy dimensions rather than a bundled run mode.

- Renamed the active pure activity-model vocabulary to `AssignmentActivityTransition`, `AssignmentActivityError`, and `AssignmentAttemptCompletionStatus`. Summary and learner-wire projections now expose `completedAssignmentAttemptCount` / `completed_assignment_attempt_count`; regenerated TypeScript, 127 domain tests, and 10 focused browser decoder/progress tests pass.

- Renamed the public activity reference type to `AssignmentAttemptReference` throughout Rust, generated TypeScript, browser navigation, Gradebook adapters, fixtures, and cursors. The compact `R-` wire format remains a stable opaque reference, while product-facing code now names the record it identifies.

- Rebased the browser API, runtime, learner activity screens, theme scopes, strict decoders, and HTTP routes on `StudentRecord`, `AssignmentAttempt`, `IssuedQuestion`, and `QuestionAttemptTiming`. The client now uses explicit Assignment Attempt operations and nested `assignment-attempts` routes; retired Enrollment and Assignment Run fields no longer cross that boundary. The TypeScript contract generator now emits manually serialized, single-value bounded wrappers while retaining private custom-wire records outside the browser declaration graph. TypeScript checking passes, as do 24 focused response, secrecy, and HTTP-client tests and 24 generator tests.

- Moved the browser's strict learner-activity boundary to Assignment Attempts and Issued Questions. Question Attempt responses now carry their exact Issued Question and `QuestionAttemptTiming`; attempt summaries, prefetch descriptors, and browser fixtures preserve direct activity relationships rather than enrollment, run mode, copied version, or copied position fields. The focused response-decoder and HTTP secrecy suite passes 24 checks.

- Removed the unmounted Memory Store's Enrollment-based statistics and retention implementation, including its test-only state snapshots. The learning-data-access crate now documents the direct PostgreSQL path instead of advertising that retired implementation as an executable Store.

- Finished the direct Student Record and Assignment terminology cutover in the activity, enrollment, contract, identity, and mastery documentation. The fixture corpus now carries the same direct records, Issued Questions, and derived `AssignmentActivitySummary` model validated by the focused Rust suites.

- Replaced the opaque Rust `RunId` identity with `AssignmentAttemptId` across question-model, domain scoring, data access, fixtures, base-course, and E2E seed consumers. The UUID remains opaque while the owning activity concept is now explicit. The durable Rust records are now `AssignmentAttempt` and `IssuedQuestion`, matching the fresh schema; focused question-model tests and learning-data-access checking pass.

- Made the Question Attempt model reference one durable Issued Question rather than copy a Question Version and Assignment position. Issued Question IDs deterministically bind an opaque Assignment Attempt to its frozen selection; the fixture corpus, selection projection, statistics, and submission paths now carry that relationship directly.

- Renamed the native activity timing value from `AttemptTimerRecord` to `QuestionAttemptTiming`. Its owning Question Attempt and pure timer verdict consumers now share the same direct term; focused `question_model` and `domain` test suites pass.

- Replaced the fresh database's per-Assignment enrollment and Assignment Run relations with direct Student Record and Assignment parents. Assignment Attempts now own ordered Issued Questions; Question Attempts reference that exact issued selection; Question and Assignment Submissions are distinct immutable acceptance events; and Assignment Grade replaces the legacy Gradebook snapshot parent. The disposable PostgreSQL baseline gate passes.

- Replaced the fresh schema's `course_student` relation with `student_record`. Capability brokers, RLS predicates, observer and Sysadmin-support grants, protected course-object metadata, and indexes now bind the exact Student Record. The disposable PostgreSQL baseline acceptance passes this cutover.

- Replaced the database and activity documentation's legacy enrollment/run spine with the canonical Student Record -> Assignment Attempt -> Issued Question -> Question Attempt -> Question Submission chain. Assignment Grade is now correctly described as the selected course result rather than the owner of Student activity.

- Aligned the enrollment contract with the course-level roster model. Course Enrollment now creates a Student Record; Assignment audience and that exact record authorize activity directly, without a separate per-Assignment enrollment aggregate.

- Aligned identity, question-backend, and database-authorization documentation with Issued Question ownership. A Question Attempt is now explicitly one try under an Issued Question, while retained activity relations name their direct Student Record and Assignment parents.

- Removed the retired Alpha Course product surface from the server route-policy inventory, browser route labels, scenario catalog, and obsolete end-to-end journey. PLE now names one reusable course type, Blueprint Course; retained `alpha` references are scientific content or generic decoder test data.

- Replaced position-based Alpha curriculum selection with an exact, revision-bound Blueprint Course assignment source. The problem picker and assignment editor now resolve one retained assignment identity, preserve its answer-free Question order, and refuse a stale Blueprint Course revision.

- Replaced the mounted reusable-curriculum workspace with one Blueprint Course list, inspection, create, and owner-replacement surface. Every approved Instructor can inspect answer-free reusable structure; an owner edits the complete retained tree and receives an immutable new revision on save.

- Rebuilt reusable-course drafting and creation around one complete Blueprint Course tree. The browser now uses generated snake_case assignment fields, labelled modules, and server-assigned retained identities; focused model and creation tests prove local validation before one create request.

- Removed the unimplementable Alpha-curriculum independent-copy panel and its obsolete Playwright journey. The reusable-course workspace no longer mounts a source type or route that the current generated contracts and server transport no longer provide.

- Replaced the browser curriculum-adoption client's retired per-route Alpha and Blueprint operations with one generated, closed Blueprint Course adoption envelope. Preview and apply now use the server-owned `/api/curriculum-adoption` boundary, exact operation tags, idempotency intent, no-store transport, and answer-free completed results.

- Replaced the browser reusable-curriculum transport's parallel legacy shapes with one strict Blueprint Course contract. Requests and responses now use the generated course, module, assignment, and snake_case persistence fields; the focused client proof covers answer-free views, ETags, and conflicts.

- Removed the obsolete publication-scope choice. A Published Question now enters the one global Question Corpus; private work remains a Draft Question. Browser publication requests carry only reviewed byline evidence, corpus usage counts are installation-wide, and Published Question assets retain authorized delivery rather than becoming CDN-public.

- Replaced direct runtime access to `ple_data` and `ple_private` with exact `ple_api` session and credential-operation brokers. Capability roles now receive only the API operations they require; schema-owner access remains internal and the disposable PostgreSQL acceptance probes enforce that split.

- Made the fresh-baseline migration runner execute canonical top-level SQL in transactional order while retaining SQLx-compatible checksum ledger records. This preserves each migration's intentional owner switch, and the baseline acceptance oracle now scopes membership checks to PLE principals while reporting any unexpected PLE edge precisely.

- Reduced the mounted browser sign-in surface to its executable seeded-demo Account selector. Retired email-completion and email-change pages/routes no longer promise missing endpoints; the rebuilt email-code and passkey adapters remain the explicit follow-on for the canonical Authenticated Session.

- Reconnected durable documentation and retained historical records after the clean schema and planning-corpus deletion. Current material now links to its implementation-status, release, schema, security, and payload authorities; historical entries name retired evidence without dead links.

- Replaced the browser's stale multi-role user session DTO with the server's exact Authenticated Session projection: one `account` with one immutable Product Role. The strict decoder rejects the retired user/role-list shape, and route guards now consume that single Account role.

- Removed the unreferenced account-presentation PostgreSQL oracle. It asserted deleted account, session, preference, and broker relations from the retired schema epoch; the fresh baseline has no corresponding product capability.

- Replaced the remaining `UserId` ownership types in personal question collections, saved searches, and Instructor pool previews with the exact global `AccountId`. These private curation relationships now name their actual owner rather than retaining generic-user vocabulary.

- Added `2026082934_sysadmin_account_provisioning_broker.sql`, establishing a current-session Sysadmin-only path to create one global Account with its immutable Product Role. Passwordless authentication remains unable to create Accounts or assign roles.

- Replaced retired account-authentication table and index names in the database map with the fresh private Account, Authentication Email, challenge, Passkey, and Authenticated Session relations. The authorization and role documents now include the `2026082933` atomic credential-completion capability.

- Corrected the student, install, usage, cookbook, security, and Live Demo documentation to distinguish the required email-code/passkey product design from the currently executable seeded-demo entry. The removed adapters are no longer represented as a completed live-demo reauthentication journey.

- Added `2026082933_authentication_ceremony_brokers.sql` to the fresh baseline. Its execute-only `ple_auth` brokers atomically consume browser-bound email-code or validated-passkey ceremonies and return only the existing Account and immutable Product Role for canonical session creation.

- Added the single-session passwordless credential contract in `learning-data-access`: bounded, browser-bound email challenges and private passkeys can return only an existing Account plus immutable Product Role. Raw credential proofs stay hashed and redacted; the route layer alone then creates the canonical Authenticated Session.

- Converged the mounted authentication route inventory on the one current Authenticated Session surface: session lookup, sign-out, and the deployment-gated seeded Live Demo entry. The route policy, composition test, API contract, and database-structure narrative no longer advertise retired passwordless/passkey adapters or their obsolete auxiliary cookies. The fresh email-code and passkey schema roots remain the explicit next reconstruction package.

- Removed the stale browser account-security/passkey/email-change surface and its obsolete tests. The mounted Live Demo now truthfully provides only the seeded Account selector; the navigation, client contract, browser evidence, and Instructor documentation no longer advertise authentication adapters that are not mounted by the current server.

- Aligned the central identity, role, database-authorization, and security contracts with the terminology authority: a global `AccountId` authenticates through one Authenticated Session, and exact relationships authorize protected work. The documents now point to current implementation ownership rather than deleted migration-era planning files.

- Replaced the curriculum-adoption receipt contract's remaining generic identity labels with `authorized_account`. Its immutable receipt authority now names the exact Account that authorized the operation.

- Updated live enrollment, security, authorization, lifecycle, storage, cache, and browser-contract documentation from the retired `UserId` vocabulary to `AccountId`. Repaired their deleted planning links with their current durable contract or implementation-status authority.

- Removed the detached `AccountSession*` account-identity and account-presentation persistence models. `learning-data-access` now exposes the single canonical `SessionStore` foundation for all future email-code and passkey routes.

- Retired the server's unreachable dual-session passwordless, passkey, and seeded-selector route implementation. The retained session route resolves and revokes only `SessionRecord`; the next authentication package will rebuild email-code, passkey, and seeded-demo entry directly on that contract.

- Added the replacement disposable Live Demo entry contract: five validated, display-safe persona keys map only to configured Accounts, then issue the ordinary host-only Authenticated Session. The database's immutable account-role foreign key rejects a mismatched role at session creation.

- Restored `AuthenticationEmail` as a focused private credential value object, with strict IDNA lookup normalization, delivery spelling, and redacted diagnostics. It no longer shares a module with obsolete account-session or authorization models.

- Made Account Creation explicit and separate from authentication. Email challenges now bind an existing Account and have only `sign_in` or `change_email` purposes; Account creation and Product Role assignment remain Sysadmin-owned lifecycle operations.

- Added the private one-to-one `account_authentication_email` relation to the fresh passwordless schema. A verified mutable Authentication Email now identifies an existing global Account for email-code sign-in without becoming Account identity, a role grant, course authority, or a browser DTO.

- Removed obsolete SQL line-length overrides for four migrations retired by the fresh schema baseline; the source-style registry now contains only live files.

- Kept the browser-session cookie helper focused on the canonical `__Host-ple_session` credential. Passwordless ceremony and passkey binding cookies now await their reconstructed single-session route, rather than retaining a second account-session cookie vocabulary.

- Replaced the database-structure document's deleted pre-SD1 migration ledger with the fresh `2026082901`-`2026082906` foundation and its actual Account, Authentication Email, session, approval, and passwordless ownership.

- Rebuilt the current-package registry's retired plan pointers around the release plan and changelog evidence. Its migration allocation and dependency queue now remain navigable in the fresh SD1 documentation set.

- Repointed the implementation and release plans to the active SD1 status and release authorities, removing deleted auxiliary planning documents from their current dependency, browser, grading, and QTI narratives.

- Replaced the Base Course accepted-submission seed identity with its exact `student_account`. The privileged deterministic installer now supplies the Student Account whose membership and enrollment chain owns the seeded work; ordinary browser submission continues to begin with an Authenticated Session. The disconnected legacy issue helper was removed with its obsolete generic identity shape.

- Recast the course-contract authority and membership types around `AccountId`. Course creation, Instructor edits, listing, and Student-record authorization now name their exact Account input, while `CourseMembershipRecord` exposes the owning `account` rather than a generic identity field.

- Aligned the live-demo Account selector with the global Account contract. Its configuration, selected value, and tests now use `AccountId` and preserve the closed five-Account deployment mapping.

- Rebuilt the PostgreSQL session adapter around the clean authenticated-session brokers and `SessionRecord { account, role, ... }`. It now reads and writes the global Account and immutable AccountRole fields defined by the new schema; `cargo check -p learning-data-access` passes.

- Recast the central server authentication contract around the resolved Authenticated Session. Session issuance and browser-safe identity now use an Account plus its immutable role, and sign-out revokes that one session record.

- Restored the first-party HTTPS browser boundary for cookie-authenticated routes. It normalizes host-only session cookies and enforces exact canonical Host and Origin checks for state-changing requests (ASVS V3.3 and V3.5).

- Retired disconnected iMathAS publication and issued-snapshot resolution methods that depended on removed LDA contracts. The adapter now compiles around its current immutable `SourceArtifact` input surface.

- Bound course-grade export audit records and leased queue claims to their exact course, requester, job, payload, and lease identities.

- Simplified grading-operation and accepted-submission worker identities to their course, assignment, attempt, submission, job, lease, and worker keys.

- Simplified course-invitation delivery rows to their exact course, invitation, delivery, lease, and lifecycle identities.

- Simplified Student-work inspection access and audit facts to their Account, course, membership, assignment, run, evidence, and scoring identities.

- Bound account-course projections and their opaque page cursor to the global course identity. The account, course, title, and membership role fully identify the relationship they present.

- Simplified protected-asset access audit events to the Account, delivery, object, bucket, optional course, and authorization time that identify the actual access decision.

- Recast the course-store contract around the server-derived authenticated Account. Course listing no longer accepts a caller-selected member scope; implementations and routes now converge on direct membership authorization.

- Removed the generic scope-equality helper from activity-policy validation. Store families now migrate to their resource-specific authorization predicates instead of treating one installation selector as ownership.

- Simplified course-group and effective-policy value records to their exact course, assignment, membership, and attempt identities. These records no longer carry a redundant installation-scope field.

- Removed the retired in-memory course-provisioning test module and its scope-shaped fault hook. Its assertions targeted the previous request boundary; the course-and-membership cutover now owns replacement coverage.

- Removed the obsolete black-box Memory course-creation integration test. It encoded the retired multi-scope account/session contract and cannot protect the global-account course boundary being established.

- Simplified flat-question image descriptors and their in-memory index to the exact workspace and logical asset identities. The private object-key check now matches that same canonical shape.

- Bound in-memory private accepted responses solely to their immutable question attempt. Replay, grading, inspection, retention, and test-support paths retain the same private-response integrity checks.

- Simplified protected asset-delivery records to exact course ownership for student records and banners. Registration, authorization, export, and retention paths, including their server-owned object keys, now take scope only from their trusted request or job boundary.

- Bound publication validation to the canonical workspace/import and published archive identities. QTI and flat-import checks no longer accept a redundant scope parameter or compare object-key fields that no longer exist.

- Simplified accepted-submission records to their exact course, assignment, attempt, submission, and Account identities. Execution paths now use the verified request or worker claim authority already at their boundary.

- Kept rollover and term-shift tests bound to their exact course references, with an explicit single-match assertion before reading the course witness.

- Kept curriculum-adoption reconciliation tests bound to their immutable assignment references, removing stale internal map scope selectors while preserving derived-projection repair coverage.

- Removed duplicate catalog search wrapper tests that depended on the retired session/context fixture. The maintained catalog search and snapshot suites remain the canonical coverage for pagination and deterministic results.

- Removed the duplicate in-memory flat-question test module wired to the retired scope interface. The maintained conformance suite remains the canonical behavior coverage for flat-question persistence.

- Updated flat-question object-key fixtures to use their canonical workspace and object identities without obsolete scope fields.

- Simplified feedback-release records to their exact attempt, releasing Account, and release time; authorization continues to derive through the assignment's course membership.

- Kept curriculum-adoption integrity fixtures bound to their exact immutable course and assignment references while removing stale internal map selectors.

- Removed obsolete scope fields from the course-contract selection and scoring fixtures. Their assertions continue to exercise global run, assignment, course, enrollment, and attempt identities.

- Updated isolated curriculum and item-analysis fixtures to use only exact assignment, course, and scoring identities, removing retired test-only scope fields without weakening their integrity assertions.

- Removed the obsolete direct-SQL live-demo oracle and its aggregate E2E invocation. The clean database baseline plus the production service-owner lanes remain the current durable live-stack evidence.

- Corrected project-tools command help so the Base Course and E2E seed examples name their actual account and lifecycle inputs only.

- Made authenticated Account identity complete and self-checking. The resolved session carries the immutable role, and the in-memory session guard requires the exact Account, session, and role before returning the resolved record. The focused test remains blocked by unrelated active cutover errors in the data-access crate; Rust 2024 formatting passes.

- Aligned PostgreSQL session persistence with the clean global-account schema. Primary sessions now use a server-owned create/revoke broker, opaque hashes, generated `SessionId`, and one immutable role; active resolution installs the corresponding Account identity. The adapter and migration are clear of the retired vocabulary, and the changed Rust source passes Rust 2024 formatting.

- Removed seven unreferenced legacy PostgreSQL oracles for course appearance, assignment teaching projection, worker filtering, catalog search and plan, flat-question assets, and QTI import. Each targeted the deleted schema epoch; separately referenced reusable-curriculum and WP-R2 acceptance roots remain.

- Removed the inactive teaching-authority PostgreSQL oracle and its invitation, public-reference, expiry, and candidate helpers. The cluster depended on the retired account/session and schema model and had no active clean-baseline runner.

- Removed the isolated flat-question PostgreSQL oracle. Its object, grading, and direct-SQL fixture setup targeted the retired schema epoch and had no active clean-baseline runner.

- Removed nine unreferenced legacy PostgreSQL oracle roots and their issued attempt and student-work helper modules. They covered effective policy, course groups, attempts, outbox, public references, replay, course term, teaching operations, and student-work inspection against the deleted schema epoch; no active non-document runner referenced them.

- Removed the orphaned problem-curation PostgreSQL oracle and its authority, behavior, fixture, and pagination modules. They asserted retired-schema scope isolation and had no active clean-baseline runner.

- Removed the standalone legacy item-analysis PostgreSQL oracle. Its worker, RLS, and direct-SQL fixture assertions targeted the retired schema model and were absent from the clean-baseline runner.

- Removed the obsolete flat-import provenance PostgreSQL oracle and its success fixture. Its direct SQL assertions targeted the deleted schema epoch and were not reachable from the clean-baseline runner.

- Removed the standalone legacy course-grade upgrade and retention PostgreSQL fixture. Its migration-copy and historical schema assertions had no active non-document runner after the clean-baseline transition.

- Removed the orphaned entitlement-membership PostgreSQL oracle and its legacy SQL security probes. The cluster had no active non-document runner and targeted the retired schema model.

- Removed the isolated catalog-detail PostgreSQL oracle, whose fixture SQL and authorization setup targeted the deleted schema epoch and had no active non-document runner.

- Removed the isolated legacy preview-plane PostgreSQL oracle, which had no active non-document runner and asserted the retired schema model.

- Removed the orphaned catalog-discovery evidence PostgreSQL oracle and its fixture modules. Its migration-copy and broker assertions were tied to the deleted schema epoch, and no active non-document runner referenced it.

- Removed the unreferenced automated-grading PostgreSQL oracle and its legacy broker, receipt, retry, recovery, scoring, and assignment support modules. They exercised the deleted schema epoch; automated grading remains the required behavior for the clean-baseline acceptance suite.

- Removed the inactive passwordless-and-enrollment PostgreSQL oracle and its legacy account, invitation, roster, and session fixtures. The cluster depended on the deleted schema and superseded account contract, with no active non-document runner after the clean-baseline transition.

- Removed the inactive course-provisioning PostgreSQL oracle and its helper modules. Its migration-version and direct-SQL assertions targeted the deleted schema epoch, and no active non-document runner referenced the cluster after the clean-baseline transition.

- Removed the unreferenced legacy assignment-mutation PostgreSQL oracle and its direct-DML fixture. Both asserted migration versions and schema columns absent from the clean baseline; no active non-document runner references the retired cluster.

- Removed the obsolete live course-grade PostgreSQL oracle and its two dependent SQL fixture modules. They targeted the deleted schema corpus and had no active runner after the clean-baseline transition. The remaining database acceptance path owns current schema evidence; no non-document reference reaches the removed test cluster.

- Removed the caller-supplied installation selector from the host-only E2E seed command and from the live browser and service seed launchers. Seed invocation now names only the actual account identities and required host storage coordinates; converting the dependent record writers remains within the active data-access cutover. Rust formatting and Python syntax checks pass, while the project-tools check reaches the known missing-type frontier.

- Switched the public database-baseline E2E entry point to the clean-schema oracle and removed the obsolete lifecycle owner and legacy SQL probes that exercised the deleted migration corpus. Shell syntax checks pass, and no remaining non-document reference reaches those retired test artifacts.

- Removed retired installation-prefix object permissions from API and worker IAM policies. The deployment policy test now describes the retained typed workspace asset read boundary; the deployment directory is clear of the retired vocabulary. No OpenTofu or Terraform formatter is installed in this environment, while the focused sweep and diff check are clean.

- Made gradebook-summary paging an authenticated Account and course-bound Store read. Memory and PostgreSQL each resolve the persisted course owner and require that Account's active Instructor membership before reading its maintained enrollment-summary projection. Gradebook fixtures now exercise that FERPA predicate directly. Rust 2024 formatting and the call-site sweep are clean; the focused crate check remains blocked at the earlier root context-contract cutover.

- Made anonymous question-statistics disclosure a global public-publication read. Its Store contract and adapters now accept only the exact version; private catalog grants cannot expose the aggregate. The statistics fixture now proves repeatable public disclosure at the configured anonymity floor. Rust 2024 formatting and the call-shape sweep are clean; the focused crate check remains blocked at the earlier root context-contract cutover.

- Made worker-job failure and inspection exact-resource operations. Failure accepts only the active job lease; inspection accepts only the job identity. The worker, server paths, in-memory and PostgreSQL Stores, fixtures, and conformance/live callers now follow that closed contract. Rust 2024 formatting and the JobStore call-shape sweep are clean; the focused crate check remains blocked at the earlier root context-contract cutover.

- Made course-record lifecycle access a `CourseId`-bound Store operation. Route callers no longer supply an installation context; Memory resolves the unique stored course and PostgreSQL performs the same bounded course lookup before testing lifecycle accessibility. Rust 2024 formatting and the call-site sweep are clean; the focused crate check remains blocked at the earlier root context-contract cutover.

- Removed the ambient context argument from assignment-scoring worker prepare and commit operations. The command already carries the exact job, lease, assignment, and generation; both Store backends derive their internal key only after validating that lease. Production workers, seed paths, and conformance/live callers now use the closed worker command directly. Rust 2024 formatting and the call-shape sweep are clean; the focused crate check remains blocked at the earlier root context-contract cutover.

- Removed the ambient context argument from the lease-bound auto-submit worker Store contract and its server committer. Both backends now validate the exact job lease before deriving their internal key, so a worker cannot name an installation scope. Rust 2024 formatting and the worker call-site sweep are clean; the focused crate check remains blocked at the earlier root context-contract cutover.

- Removed caller-supplied installation scope from Instructor grading-operation list, retry, and recalculation commands. Route callers and both Store implementations now derive their internal key from authenticated context; opaque list cursors bind only the course, assignment, grouping, and row identity. All known constructors were migrated, and the affected Rust files pass Rust 2024 formatting. The focused crate check remains blocked at the prior root context-contract cutover.

- Removed the retired scope field from the retention worker command and all server, Memory, PostgreSQL, conformance, and live-test callers. Both Store implementations now derive their internal scope key from the leased durable job before any retention lookup or mutation. The retention contract itself contains no remaining retired-scope vocabulary; its Rust 2024 formatting and focused compiler diagnostic scans are clear.

- Renamed the installation-wide retention policy contract and every internal consumer to remove the obsolete institution vocabulary. Rust 2024 formatting is clean and the old type name is absent; the focused compile remains behind the active missing-context contract cutover.

- Made the in-memory retention policy one installation-wide setting instead of a scope-keyed map. The scheduler test now proves a configured policy applies to a later course; formatting and focused compiler diagnostics are clear.

- Migrated in-memory retention authorization and fixtures to the immutable one-role session contract. Sysadmin-only policy and extension operations now test the exact account role, while Instructor authority remains a direct course-membership predicate. Session validation no longer expects a retired scope field. Both changed modules pass Rust 2024 formatting, and the focused learning-data-access compiler diagnostic scan is clear.

- Updated in-memory grading-operation authorization to require the exact immutable Instructor account role before checking its current course membership. The narrow formatter and compiler diagnostic checks are clear.

- Updated the in-memory teaching-authority Sysadmin session predicate to use the immutable account role directly. Rust formatting and the focused compiler diagnostic scan are clear.

- Converted in-memory course creation and group-policy authorization to exact immutable account roles. Its course fixtures now construct current session subjects directly; Rust formatting and the focused compiler diagnostic scan are clear.

- Converted in-memory catalog-search authorization to an exhaustive immutable role decision. Sysadmin accounts retain the explicit support path, approved Instructor accounts resolve catalog access, and Student accounts are refused. Rust formatting and the focused compiler diagnostic scan are clear.

- Converted in-memory course-roster support and read authorization to the immutable Sysadmin role. Existing course-membership, concealment, and audit behavior remains at the Store boundary; Rust formatting and the focused compiler diagnostic scan are clear.

- Converted reusable-curriculum approval authorization to the immutable Instructor account role. Rust formatting and the focused compiler diagnostic scan are clear.

- Converted the server course-roster support route to its immutable Sysadmin account role check. Rust formatting and the focused compiler diagnostic scan are clear.

- Converted server course-policy authorization and provisioning derivation to exhaustive immutable-role decisions. Rust formatting and the focused compiler diagnostic scan are clear.

- Converted the server grading-operations route to the immutable Instructor role before its existing membership and course-visibility checks. Rust formatting and the focused compiler diagnostic scan are clear.

- Converted server retention route authorization to the immutable Sysadmin role while retaining its exact course-membership predicate. Rust formatting and the focused compiler diagnostic scan are clear.

- Converted the server teaching-operations operator gate to the immutable Sysadmin role. Rust formatting and the focused compiler diagnostic scan are clear.

- Reworked in-memory problem-curation session authorization around one immutable account role. Approved Instructor sessions can mutate personal collections, Sysadmin sessions retain their shared-collection read path, and Student sessions are refused. The fixtures now construct that one-role contract directly; the removed foreign-installation test branch no longer encodes an invalid boundary. All changed curation modules pass `rustfmt`, and their targeted compiler diagnostic scan is clear.

- Removed the retired installation-scope identity from PostgreSQL assignment summary decoding, gradebook projection aliases, and the locked automated grading-completion summary witness. Summary records now reconstruct only their exact enrollment identity. The self-contained decoder modules pass `rustfmt`, and the focused compiler scan reports no diagnostics for this slice; the focused test remains behind 545 unrelated data-access errors.

- Rebased the deterministic Base Course question on the installation-wide published corpus and simplified its catalog pagination test to cover two shared published questions directly. The obsolete institution-only catalog branch and grant fixture are gone; both changed Rust files pass `rustfmt`.

- Aligned passwordless course invitation redemption and course selection with the single-role account contract. Each newly issued session now carries the exact Student or Instructor role established by the flow, without combining a course role with auxiliary platform roles; the direct auth fixture uses that same one-role constructor. Both changed Rust files pass `rustfmt`. The focused server auth test reaches the independently migrating data-access crate first (686 unresolved upstream errors), so it does not yet execute. The replica-restart E2E child now derives its exact resource identities from its seed manifest alone, passes `node --check`, and contains no stale scope marker.

- Rebased learner attempt-recovery fixtures on their exact run and attempt storage key. The full 27-case Node suite continues to cover recovery, idempotency, offline retention, hostile local data rejection, deadline behavior, and answer-free grading state.

- Corrected the run-page recovery fixtures to use their exact run and attempt identities. Session recovery, preserved idempotency, correction after a refused request, and response replay remain covered by the two passing Node cases.

- Aligned the frontend session-contract fixtures with global account sessions. The retained Node checks continue to protect stale-request rejection, browser-boundary cleanup, safe session state, answer-free generated types, and issued-attempt binding; all seven pass.

- Updated base-course activity fixtures to construct current run and attempt records from fixed single-installation baseline IDs, without obsolete scope fields. The activity module passes `rustfmt`; its focused crate test is blocked by the independently migrating data-access crate (473 unresolved upstream errors).

- Removed the redundant installation-scope argument from the Memory reusable-Blueprint snapshot helpers and every direct caller. Immutable source locators now resolve against their globally unique reference and revision keys, while Account and course authorization stays at the calling Store boundary. Reusable-Blueprint pagination likewise binds its stored continuation only to the authenticated Account. Formatting passes and the focused compiler diagnostic scan is clean.

- Corrected the Memory reusable-Blueprint aggregate and immutable-snapshot accessors to match their globally unique state-map keys. Reference, aggregate, and revision reads and writes now use their exact Blueprint parent identities; the repaired files pass `rustfmt`, and the focused compiler diagnostic scan no longer reports either module.

- Bound run-summary continuation cursors solely to their immutable `RunId`; their keyset tuple and integrity digest no longer carry a redundant installation-scope value. Memory and PostgreSQL call sites use the same contract, and the changed Rust files pass `rustfmt`. The focused Cargo test remains behind the existing data-access cutover frontier (578 unresolved upstream errors).

- Rewrote the concluded Rust/SQLx/PostgreSQL review around server-derived Account and exact-resource authorization. Its historical RLS, foreign-key, catalog, worker, and denial-matrix findings no longer preserve retired scope names or identifiers; the review is formatter-clean.

- Aligned the accepted course-appearance plan with global `CourseId` ownership. Appearance, candidate banners, revisions, RLS, object delivery, and non-enumeration now describe exact course authority rather than a redundant installation scope. The plan is formatter-clean.

- Reconciled the security architecture audit with the single-installation authorization model. Session-derived Accounts, exact course membership, Student ownership, and leased capabilities now define protected-resource access; its grading evidence describes automated evaluation and recalculation. The audit is formatter-clean.

- Aligned the typed project-tools published-problem fixture generator with the canonical browser-safe course, assignment, enrollment, run, attempt, summary, and Gradebook records. Its deterministic local identities no longer include a redundant installation-scope value, matching the committed corpus. The source formatter check passes. Its focused Cargo test reaches the separately migrating data-access crate first, where a missing legacy migration include and 720 unresolved cutover errors currently prevent compilation; this is upstream integration status, not a passing fixture-test result.

- Converted Chapter One's deterministic course, assignment, item, statistics learner, run, and attempt identifiers to a fixed domain-separated single-installation namespace. Resume-manifest selection and validation no longer accept a redundant scope input, so they protect the stable corpus shape directly. The affected Rust files pass formatter checks; their package test remains behind the same data-access compilation frontier.

- Corrected the private Memory CourseInstance-to-Blueprint application map to use its globally unique `CourseId` as the sole key. Mutation and read paths retain exact course-state checks before consuming the immutable parent application, so the map no longer embeds a redundant installation scope in its durable parentage relation. The focused formatter check passes.

- Aligned the August 10 executive status report with the current account, Course, and Student ownership model. Educational records now describe their exact FERPA-bearing parent, invitation claim describes course-bound Student identity, and ordinary session language names the authenticated session directly. The dated report is formatter-clean.

- Updated the historical scale review to retain its shared-catalog and stateless-service guidance while naming the exact global account, workspace, Course, Student, and capability boundaries. It no longer treats installation scope as a durable data owner and is formatter-clean.

- Reconciled the active implementation plan with the single-installation end state. The session cutover, database boundary, browser contracts, and future deployment decision procedure now describe retired global-scope seams and exact resource ownership rather than preserving obsolete compatibility vocabulary. The plan is formatter-clean.

- Aligned the August 9 status snapshot with the current Account, Course, Student, and exact-problem ownership model. Its historical retention, catalog, analysis, and upload statements now describe the protected resource they concern; the snapshot is formatter-clean.

- Corrected the accepted Instructor-to-Student walkthrough plan's local-roster and Question-ID language. Server-derived Account and Student identity, exact course authority, and Account-bound catalog resolution now define the visible journey's inputs and denied cases; the plan is formatter-clean.

- Rewrote the active single-installation authorization plan's historical inventory and closure language around exact domain ownership. It now records retired global-scope context, keys, RLS, routes, and contracts without retaining obsolete type or field spellings; the plan is formatter-clean.

- Updated the sole current-package registry's preparatory receipts and handoff order to describe retired global-scope seams without retaining obsolete vocabulary. The account, session, Course, Student, workspace, catalog, and capability boundaries remain explicit; the registry is formatter-clean.

- Reconciled the historical partial-status record with the current ownership model. Its retention, appearance, analytics, QTI, RLS, and publication receipts now name their exact Course, Student, Account, object, or resource boundary; the document is formatter-clean.

- Aligned the secure question-grading payload plan with exact attempt, Account, Course, and exact-record RLS boundaries. Browser disclosure, durable reservations, replay-state keys, and foreign-access refusal now describe their actual resource binding without obsolete scope fields; the plan is formatter-clean.

- Aligned the accepted QTI profile-mapping plan's provenance and RLS language with exact workspace, publisher Account, published record, and another Account's ownership. Its archive secrecy and profile-import guarantees remain unchanged; the plan is formatter-clean.

- Reconciled the schema-evolution plan with the active clean-baseline model. Its migration history, RLS, keys, placement, and command authority now describe exact resource ownership and obsolete global-scope evidence without retaining retired fields or types; the plan is formatter-clean.

- Replaced the wire-naming ledger's obsolete SQL-name table with its durable outcome: globally unique account, Student, Course, assignment, run, attempt, workspace, catalog, and capability identities, with exact-resource policy and function ownership. The ledger is formatter-clean.

- Converted host-only native and WeBWorK E2E replay identities to fixed, domain-separated single-installation constructors. Seed callers no longer contribute an installation scope to deterministic course, assignment, run, attempt, action, or provider baseline identities; their resolved Account records remain responsible for Store authorization.

- Removed obsolete global-scope wording from the local-roster backend review and historical learner-work broker names from the wire-naming migration plan. Both records now state the server-owned identity, transaction, and successor-function boundaries directly.

- Removed stale global-scope terminology from the account-identity contract and isolated server comments, test rejection markers, and logout-result naming. The remaining account-context field and Store calls stay allocated to the session/course-context migration because they carry live authorization behavior.

- Removed historical global-scope spellings from the single-installation ownership register while retaining its exact canonical mapping to Account, workspace, course, and leased-capability boundaries. The active `schemas/migrations/` baseline is independently clear of that vocabulary.

- Rewrote six release, authorization, pagination, provenance, and walkthrough planning records to name exact Account, course, and retained-evidence boundaries. Their stale installation-scope terminology and compatibility allowance language are now absent.

- Removed obsolete global-scope fields from the test-owned published-problem fixture and shared public corpus, and replaced neutral decoder-rejection markers across the frontend and local-stack tests. The strict feedback decoder suite now accepts the fixture's compact browser shape (7 passing cases), the focused combined Node gate passes 47 cases, and the WebWork child suite passes all 11 cases.

- Removed obsolete global-scope vocabulary from ten planning and audit records and a live PostgreSQL pagination assertion. Those records now name concrete ownership, Account, course, or resource boundaries, and the pagination assertion describes the cursor behavior it actually protects.

- Advanced the IMathAS portion of the single-installation cutover. Protected grade correlations and contracted launch sessions now bind the exact attempt, problem, version, and seed; the retired installation-wide scope value no longer participates in their payloads, MACs, cache keys, provider requests, receipts, or tests. The direct server launch consumer now constructs that same attempt-bound binding. The adapter source passes the focused Rust formatter check and contains no remaining retired-scope vocabulary. Its package test currently stops at the separately migrating `learning-data-access` frontier, which reports 221 unresolved legacy references before the adapter can compile; this is recorded as an upstream integration limitation, not a passing test result.

- Completed the local Base Course host-contract cutover. Its CLI, lifecycle controller, and direct test callers now accept only the five actual account identities; deterministic Base Course IDs use a fixed single-installation namespace; and Chapter One's host seed request likewise carries only its instructor and Student identities. The local controller source is clear of the retired vocabulary. Twenty-five focused Chapter One and Base Course lifecycle tests pass, both changed Python modules compile, and the changed Rust files pass formatting. The corresponding Rust installer and seed persistence work remains part of the wider data-access migration.

- Replaced the retired global-scope mismatch error with `OwnershipMismatch` across data access, workers, routes, and concealment responses. The affected branches retain their established fail-closed behavior; their diagnostic category now accurately describes disagreement with the authenticated owner rather than implying a removed installation boundary. The direct source sweep finds no residual old error variant or diagnostic label.

- Converted the curriculum-adoption and reusable-BlueprintCourse contract roots to `SessionRecord`. The shared in-memory session resolver now validates the resolved Account identity, preserving active-session and role checks without a separate installation-wide authorization value. Their implementation storage conversions remain in the ongoing data-access family.

- Converted the catalog, preview-plane, pool-preview, and entitlement persistence contracts to `SessionRecord`. Preview audit provenance now records the exact Account, course, assignment, and membership target rather than a redundant installation-wide scope. Implementations and database predicates remain explicitly allocated to the corresponding data-access family.

### Fixes and Maintenance

- Synchronized shared style guides, tests, and repository support files from the starter template.
