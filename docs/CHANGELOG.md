# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was
> changed and believed at the time. They are not product authority. Current
> intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old
> Assignment, Blueprint, lifecycle, grading, role, retention, and UI models.

## 2026-09-15

### Additions and New Features

- Added the canonical no-write Instructor Student View server boundary. The manifest projects
  current answer-free Assessment policy and transient exact Pool selections with a strong saved
  Edit Number ETag; separate reauthorized reads deliver native PLE presentations and sandboxed
  pre-submission WeBWorK documents without creating Student Work, Attempts, submissions, grades,
  stored objects, or renderer cache entries. The prior metadata-only Assessment `/preview` API is
  no longer registered. Production iMathAS Student View rendering remains explicitly unavailable
  until its existing configured backend is composed into this server path.

- Added locally vendored IBM Plex Sans Condensed only for long Citation URL input values, including
  slashed-zero numerals, same-origin production delivery, and a one-time Chromium font receipt.
  Atkinson Hyperlegible Next remains the main PLE font; this adds no broad anchor styling.

- Added active-vetted-Instructor published Question Pool reads: bounded opaque-cursor global
  discovery, server-HMAC-validated current Revision detail, and Course-Instructor-owned exact
  Assessment fork detail reuse the answer-free exact Question Revision projection. Every published
  Pool, including child forks, is reusable; global responses disclose no source provenance,
  Course/Assessment association, or Student facts. A fresh PostgreSQL 17 proof passed global
  paging, current ordered pins, child-of-child reuse, and Student denial. A separate fresh
  PostgreSQL 17 Course-adoption proof confirmed that a Blueprint may pin a child Pool and the
  adopted Assessment fork records that immediate child Revision as its source without flattening
  provenance to the root Pool.

- Cut over the retained Student Assessment Attempt Node contracts to canonical Assessment test
  filenames, source modules, API fields, routes, and opaque Course Instance and Assessment
  references. The 34 focused tests continue to protect disclosure, response persistence,
  request/acknowledgement matching, BackendOwned capture, answer-free navigation, supplied-time-zone
  rendering, and withheld-score behavior. This is a bounded test integration repair; the broader
  Assessment cutover remains open.

- Updated retained transport, decoder, and connected-E2E contracts to use canonical opaque
  `CI`, `A`, and `BP` references and canonical Assessment routes. Removed the retired
  Assignment-access helper test and an internal bundle-export test; their durable behaviors are
  covered by the current policy and route-scope tests. The focused Node contracts, gateway pytest,
  shell syntax checks, formatting, and independent review passed. Broader Assessment test cutover
  remains open.

- Simplified the live browser-suite origin receipt to one direct envelope.
  Aggregate-only journeys write `contexts: null`; journeys that observe each
  BrowserContext write the named context evidence they captured. The oracle
  rejects the retired two-field receipt rather than treating its missing
  distinction as compatible. Existing focused origin-security tests passed;
  the one-time retired-shape probe was removed.

- Registered the accepted canonical PGML Chargaff source as a mapped migration input for the
  Genetics static `chargaff-dna-percent-5-choices` bank. Its record pins the flat bundled path,
  immutable upstream generator, and separate content/source-code licenses. This is one mapped
  family, not catalog publication or completion of the remaining Genetics migration.

- Tightened TypeScript generator output ownership to its current exact header.
  Retired historical generator headers now remain protected as unowned files
  rather than being silently replaced; current generated contracts already use
  the canonical marker.

- Repaired the retained Pool-selection unit fixtures to use canonical 4-4
  Question IDs. Removed the transient-entropy replay test: it constrained the
  random-selection implementation rather than a durable product contract.
  The retained tests protect available pinned-item selection/order and refusal
  when too few available items remain; C353 reusable Pool provenance remains
  open.

- Reclassified Pool selection count from an artificial product question to an
  engineering decision: the existing positive Assessment-entry count belongs to
  the Assessment-owned Pool fork, while the reusable immutable Pool Revision
  owns its members. C905-C909 remain open for exact fork provenance,
  count-bound validation, delivery, and proof; this does not claim those
  behaviors are implemented.

- Blueprint Course adoption now resolves each reusable Pool to one exact root
  Pool Revision, creates an Assessment-owned child Pool lineage at Revision 1,
  and retains original source provenance and member pins atomically with fresh
  Assessments. The Course-side HMAC issuer supplies fork public IDs; a scoped,
  non-public Sysadmin adoption capability preserves the source attestation
  without exposing a standalone Sysadmin Pool operation.

- Made `cargo tsgen` a source-only `project-tools` binary.  The regular
  `cargo tools` host retains its runtime commands and dependencies, while the
  TypeScript contract generator now builds without the database, object-store,
  server, or AWS dependency graph.  Its optional output-directory argument and
  generated-contract semantics are unchanged.

- Consolidated Human Guidance's durable algorithmic-Question, Question-Pool,
  backend-feedback, human-reference-ID, and WeBWorK PG/PGML source-format
  rules. BiologyProblems.org import and migration requirements apply per
  relevant family, not to one example. This records requirements and checklist
  audit scope, not product closure or a completed catalog migration.

- Added Human Guidance requirements for opaque human-facing references: `BP`, `CI`, `A`, and
  Sysadmin-only `U` prefixes use one common random Crockford Base32 format without a separator;
  public Question and Question Pool `AAAA-ZBBB` IDs remain separate. This records required
  behavior only. Current sequential references remain noncompliant pending implementation.

- Simplified Human Guidance compliance coordination. The checklist remains the
  verbatim, evidence-backed audit record, while the plan and gap map now use
  short owner and handoff notes instead of dependency-parser, count, and
  ownership-ledger requirements. Adaptability remains a binding review
  constraint despite its N/A classification. Student-data minimization now has
  an active owner applying the simplest category-and-operation boundary review,
  rather than waiting for an invented field-allowlist decision. The checklist
  splice operation now replaces one complete manifest part through its next
  part boundary, preventing repeated splices from duplicating a generated
  section.

- Corrected C3's dependency-freshness gate to follow the latest-first policy.
  PyPI requirements now audit as one `>=` floor rather than exact pins, and the
  snapshot no longer permits an AWS downgrade exception. The recorded current
  `aws-sdk-s3` 1.147.0 resolution is audited without freezing the dependency.
  The audit does not duplicate the separate workspace build gate; its upstream
  Smithy failure remains an open build report rather than an excuse to weaken
  the latest-first declaration policy.

- Completed C365's atomic Bulk Published Question metadata database boundary.
  An active vetted Instructor can replace only `tags`, `subject`, and `topic`
  for a bounded distinct selection carrying every current metadata Edit Number.
  The security-definer command locks canonical Question-ID order, validates all
  targets before its first write, advances each metadata Edit Number together,
  and stores an actor-bound opaque idempotency receipt keyed to a canonical
  request digest. Stale, invalid, unavailable, unauthorized, duplicate, or
  oversized requests make no partial change and expose no per-target outcome.
  The ignored fresh PostgreSQL 17 proof covered unvetted denial, replay,
  same-key mismatch, stale/unknown/invalid rollback, and final zero-write
  state; it passed independent review and was removed. C367/C893 own the
  typed Store and HTTP outcomes.

- Completed C877's atomic Question-fork authoring boundary. An active
  Instructor can create one distinct private Draft from an exact Available
  Published Question Revision, with immutable source attribution and an
  actor-scoped opaque idempotency receipt. Concurrent same-key requests return
  the same Draft; a reused key for another source is refused. The stored
  server-allocated compact Question ID is required if that Draft is later
  published. The ignored PostgreSQL 17/RLS proof covered private ownership,
  source pinning, actor scope, mismatch refusal, concurrent retry, and
  fork-versus-archive ordering; it passed independent review and was removed.
  C878-C879 still own the typed server command and Instructor workflow.

- C331's isolated opaque WeBWorK render/pair/grade proof passed. The adapter
  boundary is intact, but required WeBWorK feedback is discarded after the
  renderer score; the Human Guidance bullet remains open pending
  cross-boundary outcome, persistence, and disclosure work.

- Corrected the earlier C900 entry in this section. Its frozen 2025 RDKit
  tarball and versioned immutable-route design conflicts with Human Guidance's
  latest-dependency rule and is historical attempted work, not current intent.
  The current correction uses `@rdkit/rdkit >=2026.3.6` through the npm lock,
  one reviewed local JS/WASM pair, unversioned revalidating routes, and no
  package-version/digest persistence or historical catalog. The current
  generator check (`node devel/sync_author_content_dependency.mjs --check`),
  Rust formatting, and diff check passed. A temporary Caddy-routed Chromium
  probe loaded the exact SRI JS/WASM pair in the opaque sandbox and denied
  parent, storage, cookie, and private-fetch access; it was removed after use.
  The server crate's focused test is blocked by an unrelated incompatible AWS
  Smithy resolution, and the generated checklist currently has an unrelated
  duplicate-status inconsistency. Independent review accepted the two exact
  API-origin CloudFront routes: they forward only canonical `Host`, never
  cookies, query strings, authorization, or other viewer input, while retaining
  the two-file anonymous CORS/CORP exception. This correction does not claim
  C901 or C903 complete.

- Corrected the Human Guidance plan and contracts from a code audit.
  `attempt_presentation.sql` remains the live native and WeBWorK
  issued-presentation/reproduction boundary and is now explicitly a C500
  Assessment-rename consumer; C870 removes only dormant H5P seams. The backend
  contract records H5P as blocked, not supported. The corrected RDKit chain
  uses a current local runtime only; it has no immutable-identity handoff,
  version catalog, or retirement workflow. These are scope and dependency
  corrections, not completed implementation claims.

- The WebWork audit found that the renderer path currently normalizes only
  `problem_result.score`, while Student history supplies default feedback.
  C910 is now the atomic contributor for typed, protected, policy-gated opaque
  backend feedback; C331 and C362 cannot close first. This corrects the
  dependency graph without inventing native parsing or a feedback completion
  claim.

- Completed C371's Published Question Star closure. A Star is now a visible
  favorite/endorsement, and an active vetted Instructor can see its count and
  the exact vetted display names of its endorsers. The permanent isolated
  PostgreSQL/server test passed with the closed name-only response and
  concealment for anonymous, Student, inactive-Instructor, and non-Published
  requests. A one-time compiled Chromium check confirmed the accessible plain
  text name list has no profile link, control, or avatar; it was removed after
  review. C347 may now use C371 as its completed Star prerequisite.

- Historical C900 record, superseded by the latest-first correction above:
  the former frozen `@rdkit/rdkit@2025.3.4-1.0.0` manifest and versioned
  runtime catalog were removed because they conflict with Human Guidance.
  Current C900/C902 use the npm lockfile's reviewed current release and one
  unversioned local `RDKit_minimal.js`/`.wasm` pair. The generator still
  rejects tampered, extra, or nonregular runtime files and generated-registry
  drift; it never permits an npm/CDN/author URL at runtime. The earlier
  clean-cache/reproducibility matrix remains historical evidence, not a
  claim that the frozen artifact is current authority.

- Added C414's canonical model-layer Blueprint export projection. It serializes only reusable
  short/long names, authored module and Blueprint Assessment order, reusable Assessment settings,
  and exact Published Question Revision pins and published Question Pools. It carries no owner,
  source or revision identity, visibility, Star/Watch, Course, Student, delivery, or private
  operational state; relational storage remains primary. C415 still owns import, comparison-store,
  and exchange workflow behavior. The ignored behavioral fixture proved deterministic bytes,
  authored order, complete fixed/pool settings, and identity omission, then was removed after
  independent review. No permanent test was warranted under `docs/PYTEST_STYLE.md`.

- Completed C876's Question-fork source-pin boundary. An active Instructor's
  server command can resolve only one exact immutable Revision of an Available
  Published Question; missing, archived, wrong-revision, Student, and
  unauthenticated requests receive no source fact. This schema phase creates no
  Draft, attribution, authoring operation, or client-facing identity path. The
  ignored fresh PostgreSQL 17 proof passed and remains temporary under the
  plan's test-liability policy.

- Completed C856's Blueprint Star verified-name projection. Only an active
  vetted Instructor viewing a Public or Archived Blueprint can receive active
  vetted endorsers' exact immutable display names; the separately closed,
  no-store response contains no email, UUID, Account/Profile link, avatar,
  Course, substitute identifier, Star aggregate, or Watch fact. An independent
  fresh-PostgreSQL multi-identity review passed. Its ignored disposable-stack
  matrix remains temporary because it does not earn a permanent test.

- Completed C862's remaining lifecycle-consumer cutover. Blueprint Course
  adoption and reusable-assignment source selection now accept only Public
  Blueprints; Private courses remain owner-only and Archived courses remain
  browseable history rather than new selection sources. The TypeScript check,
  lifecycle-model contract, lint, formatting, and narrow diff checks pass. The
  existing decoder-client test is blocked before execution by a concurrent
  missing generated API constant, so it supplies no result for this change.

- Completed C885's trusted Pool schema boundary. Its create and append
  procedures accept a canonical compact ID only at Revision 1, preserve the
  database collision authority, and retain each Revision's nonempty ordered
  distinct exact Published Question Revision pins plus its active Instructor
  interchangeability attestation. Appends require and replace a metadata ETag
  atomically; member sets cannot be changed after commit. Membership remains
  backend-neutral and stores no selected-count setting. There is no
  `ple_app`/browser mutation grant or public coordinator; C886-C887 own the
  allocator, route, retry, and workflow. The ignored PostgreSQL 17
  uniqueness/revision/member-pin/RLS matrix passed and was removed after
  review. No permanent test was warranted.

- Implemented C342's narrow schema projection for public content references.
  An active Instructor can receive only canonical `AAAA-ZBBB` Question and
  Pool references with their current Revision Numbers; it exposes no UUID,
  Pool contents, selection rule, UI, or lineage detail. The fresh PostgreSQL
  17 proof passed and was temporary-only. C342 remains dependent on C887's
  actual Pool-creation closure and this entry does not claim that workflow.

- Completed C343's bounded Question-ID collision retry. New-lineage
  publication now receives a typed identity-collision result only when the
  PostgreSQL adapter confirms SQLSTATE `23505` on `published_question_pkey`.
  It deletes and retries only that unregistered object; every other Store or
  object outcome retains potentially committed evidence. The permanent tests
  protect retry cleanup, capped exhaustion, fail-closed cleanup, and ambiguous
  outcome retention; the ignored one-time probe was removed after review.

- Completed C854's Published Question Star display-name surface. The Question
  detail page renders only the exact verified Instructor display names in the
  server's closed Star projection, as plain accessible list text. It performs
  no identity lookup and introduces no Profile link, avatar, email, UUID,
  Account reference, Course, substitute identity, or Watch disclosure. An
  ignored SSR rendered-name/accessibility probe passed and remains temporary;
  it is not a permanent component snapshot.

- Completed C354's Question Pool public-identity and immutable-revision
  persistence seam. Pools now retain a server-issued compact Crockford public
  ID with database collision authority, begin at Revision 1, and append only
  the expected next immutable Revision under a lineage lock. Direct table
  mutation remains unavailable to the application role; active Instructor
  capability is required for the narrow creation and CAS append procedures.
  A disposable PostgreSQL 17 proof passed shape/collision, authorization,
  immutability, stale-CAS, and two-session race checks. It remains ignored
  temporary evidence because its container-backed implementation matrix does
  not earn a permanent test.

- Completed C853's Question Star verified-name projection. Only an active
  Instructor viewing a Published Question can receive its active vetted
  endorsers' exact immutable display names; the closed response has no Account
  identity, email, avatar, Course, substitute identifier, or Watch state. An
  ignored fresh-PostgreSQL multi-identity proof passed and remains temporary
  because its setup-heavy matrix does not earn a permanent test. Invalid,
  revoked, expired, or deactivated sessions and HMAC-valid non-Published
  Questions now take the same concealed 404 disclosure path; genuine session
  storage failures remain unavailable errors.

- Completed C361's WebWork test-liability cleanup. The durable tests retain
  opaque issued-presentation and grading outcomes, stateless lifecycle
  rejection, and the boundary that rejects native PLE responses before the
  renderer. They no longer constrain renderer call counts or call order; the
  native-response test now fails immediately if the renderer is reached.

- Completed C57's bounded Question Library return path. Opening a Question now
  retains the active search, selected filters, loaded server-validated browse
  pages, and Library scroll position for one immediate in-document return;
  unrelated Library visits still begin with Search. The implementation proof is
  ignored temporary verification, not a new permanent test.

- Completed C880's Blueprint fork-sync schema foundation. Each C412 fork now
  receives private, append-only immutable baselines for its short name, long
  name, every stable-reference whole Blueprint Assessment, and its ordered
  Assessment list. The schema has no application-facing source read path; it
  preserves origin-linked source snapshots for the later authorized C881-C884
  comparison and selected-application chain. A disposable PostgreSQL 17 probe
  verified source linkage, private nonenumeration, immutability, and append-only
  unit history; the ignored temporary proof was removed after acceptance.

- Completed C881's canonical Blueprint fork comparator. It classifies only
  short name, long name, each stable-reference whole Blueprint Assessment, and
  the ordered Assessment list as safe, already applied, conflict, or not
  applicable from immutable base/source/fork values. Canonical Assessment
  equality includes reusable settings, Published Question Revision pins, and
  Question Pool content; ownership, visibility, Star, Watch, adoption, Course,
  Student, and other operational state cannot enter the comparison type. The
  ignored deterministic matrix covered every status, Assessment addition and
  removal, and exact-once stable ordering, then was removed after independent
  review. No permanent fixture inventory was warranted.

- Completed C83's one-time Student endpoint authorization discovery. The
  ignored proof traced 25 endpoints across Student-only, shared-self,
  authenticated-membership, authenticated-asset, public-renderer-asset, and
  Student-denied classes through their server-side session and Store/SQL
  authority boundaries, independently of frontend route admission. It found no
  server authorization flaw and deliberately creates no permanent
  endpoint-inventory test.

- Implemented C202's bounded Question-recognition improvement: every existing
  shared Question-reference control now receives and shows its already-present
  Question title alongside the canonical copyable reference. The planned
  disposable-stack browser proof remains pending its suite owner's input, and
  this contributor does not claim the wider C216 recognition/copy-surface
  inventory.

- Completed the C6 reusable Blueprint-content schema boundary. The closed
  content model now carries the bounded immutable facts required by Human
  Guidance; the service and browser vertical remain pending and are not
  claimed by this record.

- Completed C17's immutable, auditable Instructor-vetting store foundation.
  It records the durable vetting decision without claiming the separate C18
  Account-creation workflow.

- Completed C803's Sysadmin TOTP foundation: encrypted and zeroized
  database-bound seed handling plus account/browser-bound attestations,
  replay protection, and rate limiting. Session completion and the remaining
  C804-C807 ceremony are still pending.

- Completed the C37/C38/C812 Profile-image storage foundation. Role-neutral
  exact private `ProfileImage` schema, Object Address, and Learning Data
  Access support replace the legacy thumbnail representation; C39 and the
  browser-facing Profile work remain pending.

- Completed C815's Course Banner rendition pipeline foundation. It produces
  one complete, oriented, exact-5:1 no-crop rendition. Focused acceptance
  passed; the lease-owned end-to-end browser lane remains pending.

- Completed C824's local evidence mapping for 12 candidate parameterized
  Genetics sources, including hashes, provenance, and license pins. It is a
  source-selection contributor only; C838-C841 still own publication,
  equivalence, archive, historical-preservation, and recovery behavior.

- Completed C832/C833/C835's Profile-image catalog foundation: generated
  original safe SVG assets, catalog schema, and picker contribution. C834,
  C836, and the browser route work remain pending, so this does not claim
  selectable Profile-image delivery.

- Completed C338's Question Library bulk-selection request seam. It accepts
  only a nonempty, duplicate-free list of canonical Published Question IDs and
  carries no operation, metadata, route, or simulated mutation. C365 owns the
  real bulk request boundary and C366 its UI. The ignored temporary proof was
  run with the repository's TypeScript loader and is not a permanent test.

### Fixes and Maintenance

- Repaired the Question Asset Publication claim to use the canonical Job attempt fields and
  unambiguous Job-qualified CAS predicates. A fresh PostgreSQL 17 proof passed exact claim,
  single-attempt increment, active-lease exclusion, and atomic Pending-to-Ready activation.

- Repaired the deferred Question Asset Publication-to-Job invariant. The trigger now reads the
  referenced current Job row and null-safely checks its exact kind, target, worker, Question, and
  Revision instead of reading nonexistent publication fields; a fresh PostgreSQL 17 proof passed
  valid Pending/Ready commits and rejected wrong bindings without disabling triggers or RLS.

- Split accepted publication receipt and Question Library test owners into focused modules, and
  modularized Question-authoring and Blueprint SQL without changing their statements or behavior.
  Focused formatting and line-limit gates plus a fresh PostgreSQL schema install passed; the full
  Cargo compile remains blocked by the unrelated AWS Smithy dependency incompatibility.

- Set the local combined compiled-artifact budget to under 10 GB across workspace and temporary
  build targets. The development guide now preserves useful compatible caches, requires exact
  target, owner, and active-process inspection before an explicit cleanup decision, and does not
  treat the budget as an entire-checkout limit or recurring test threshold.

- Repaired durable contract and archived release-readiness links after the Assessment source
  cutover, and restored the required rationale fields for three existing design decisions.

- Synchronized shared style guides, tests, and repository support files from the starter template.

### Decisions and Failures

- Recorded blocked future H5P C863-C869 architecture and C870 immediate dormant
  placeholder removal. H5P has no delivered runtime until its exact content,
  library, terminal xAPI, and scoreless-activity question is answered; no
  compatibility path is authorized. This is planning, not completion.

- Recorded C862's Blueprint lifecycle browser-codec cutover. Generated
  `BlueprintAvailability` accepts only `Private|Public|Archived`; the decoder,
  client fixtures, and executable generated consumer reject legacy aliases.
  C49/C72 hand this bounded client work to C50/C19 without expanding C50's
  workspace ownership. This is approved planning, not completion.

- Reclassified the optional abandoned-Draft-cleanup sentence as an audited N/A:
  it supplies no clock or durations. C326/C352/C374/C375 are removed from
  dispatch; C861 will remove prohibited placeholder cleanup seams while
  preserving manual Draft deletion and publication. The unanswered cleanup
  policy question remains explicit. This is approved planning, not completion.

- Recorded the C303/C857-C860 author-JavaScript correction. C303 is now an
  architectural handoff, while the answer-free descriptor, authenticated
  no-store isolated document, locked-down frame, and connected proof own the
  implementation. C859 alone owns the five isolation behaviors; C304 waits for
  C860. The document permits only reviewed local libraries and a bootstrap
  nonce, never author URLs or PLE authority. This is approved planning, not
  completion.

- Recorded the C856 Blueprint Star identity split. C409 owns Star/unstar/count,
  private self Watch state, and lifecycle Watch fan-out without names; C856 may
  disclose exact vetted names only to an active vetted Instructor on a
  Public/Archived Blueprint Star list, never Watch identities/state or other
  account/Course identifiers. C423 is the final browser closure. This is
  approved planning, not completion.

- Recorded the C371 Verified Instructor Display Name boundary. The bounded
  server-controlled name is created only by C17/C18 vetting/Account creation,
  never self-edited or surfaced through Profile/directory projections. C852-C855
  carry it through authorized Published-Question Star SQL/LDA/server/frontend
  delivery; exact names, not a self/count-only route, are required before C371
  closes. The retained-test candidate is the stable real-session disclosure
  authorization/privacy outcome. This is approved planning, not completion.

- Recorded the C209 retention-notification delivery boundary. C847-C851 use
  only `warn_inactive`/`notify_archive` receipts, one verified Instructor
  destination per claim, terminal provider acceptance, callback updates to the
  same idempotency key, a disabled recorded-failure adapter, two isolated
  non-inheriting worker database pools, and due-time-order nonblocking
  transitions. Invitation export, Mail.app, fake success, and Live Demo
  delivery evidence are excluded. This is approved implementation planning,
  not a completion claim.

- Recorded the architect-approved direct preproduction Question-ID cutover.
  Question storage is compact `AAAAZBBB`; display and serde are `AAAA-ZBBB`;
  the middle compact character is the HMAC high-five-bit Crockford check over
  the other seven identity characters. The server validates before lookup, the
  existing specification owns accepted normalization, and a nonempty published
  Question table stops the fresh-schema cutover for escalation. C842-C846 now
  provide its atomic foundation, schema/fixture, browser, sweep, and integrated
  proof handoffs before C319/C342/C343/C354. No dual parser, legacy rewrite, or
  compatibility reader is authorized. This is an implementation plan decision,
  not a completion claim.

- Accepted the C832-C837 Avatar planning boundary: Profile images will use a
  PLE-provided, first-party generated-SVG catalog with stable selectable and
  retired IDs, SVG provenance, and a safe grammar. There is no avatar-list
  API. C40 waits for C819/C820, and cross-Account staff Profile-image delivery
  remains an explicit product question. This is accepted atomic planning only;
  it does not claim implementation.

- Corrected the Genetics parameterized-replacement planning boundary from 11
  to 12 sources. C824 supplies the evidence, and C838-C841 cover 12 new
  lineages, new Blueprint CAS, per-bank all-199 equivalence, conditional
  archive, historical preservation, and forward recovery. This is accepted
  atomic lifecycle planning only; it does not claim implementation.

- Accepted the A9 Human Guidance Milestone G plan and checklist map. Canonical
  C500-C536 is a 37-row atomic authoritative ledger with 104 owning behaviors,
  five duplicate pointers, and an acyclic dependency graph; C502 is
  contributor-only. C525 (cohort/completion) and C536 (export
  identity/privacy) remain terminal Human-Guidance-unresolved product
  questions. The plan includes DD-A9-01 and C12's complete Assessment
  terminology-cutover chain. Its temporary-proof, remove-by-default test
  policy is accepted. This is planning and audit evidence only; it does not
  claim product implementation.

- Accepted the A8 Human Guidance Milestone G plan and checklist map. Canonical
  C400-C425 accounts for 64 owning behaviors through 22 closure and four
  contributor milestones, with six duplicate pointers and an acyclic
  dependency graph. Its temporary-proof-first policy and independent planning
  review are accepted. This is planning and audit evidence only; it does not
  claim product implementation.

- Recorded DD-A9-01, the durable Assessment terminology decision. Assessment
  is the only generic object; Assignment remains only one of the three Type
  display names. Preproduction uses a direct cutover to the exact canonical
  routes and JSON shape, with no legacy API or runtime-import compatibility.
  One announced release may provide only a finite, safe browser GET/HEAD
  redirect before it is removed. This records the approved implementation
  boundary, not completion of that cutover.

- Recorded the Course Banner rendition decision. A banner is one complete,
  oriented, exact-5:1, no-crop rendition; 1280 by 256 is guidance and the
  generated output, not an input minimum. Preproduction uses a direct cutover.
  This records the approved implementation boundary, not completion of it.

- Recorded the architect-approved `Static`/`Seeded` attempt-reproduction
  decision. Native `pleQuestionJson` is `Static` and has neither a
  `QuestionSeed` nor generated-parameter hash; a PLE presentation nonce only
  binds response-item and authored choice ordering. Descriptor evidence
  checksum v2 binds the tag and its applicable facts. WeBWorK and seed-using
  iMathAS delivery remain backend-owned `Seeded` cases; C306 must make an H5P
  binding choose a tag before H5P delivery. The preproduction cutover replaces
  the generic seed shape directly, reinitializes a fresh database, and forbids
  sentinels, null ambiguity, and compatibility shims. One public no-seed
  contract test is justified; its vertical database proof is temporary and is
  removed after use. This is a durable design and contract decision, not a
  claim that the current implementation has completed the cutover.

- Recorded the approved self-only Account Settings boundary. Every signed-in
  Product Role uses `/account-settings` and `GET` / `PUT /api/account/settings`
  for the one closed exact-IANA time-zone preference; callers select no Account,
  Course, or role, and PostgreSQL derives the active Account in the atomic
  update. The change affects display and later Instructor wall-clock entry, not
  stored instants. Profile Settings retains avatar work, while Instructor
  Profile displays and links to the preference. Account Settings does not
  expose credential controls; the required Student and Instructor passwordless
  behavior and multiple Student passkeys remain Accounts-and-roles work. Only
  self-service credential enumeration, revocation, re-authentication,
  identity-proofed recovery, notification, and session-termination semantics
  await a separate decision. C15's Sysadmin TOTP session boundary is unchanged.

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
