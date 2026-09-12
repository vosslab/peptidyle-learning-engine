# Plan: Database baseline and revision model reset

Status: approved for implementation by the human owner on 2026-09-11; implementation is in
progress. LLM review is advisory only.

## Context

The current repository has 101 SQLx migration files comprising 20,724 SQL lines, including 293
`ALTER TABLE` operations and repeated function replacement. Graphify attributes roughly 654 nodes
to migration files. This history now obscures the intended schema and preserves several retired
Revision concepts.

The older Interface Cleanup plan is historical evidence, not implementation authority. The current
[backend_terminology_reconciliation_2026_09.md](../../archive/backend_terminology_reconciliation_2026_09.md) plan
has the right terminology direction but incorrectly treats the accumulated SQLx history as the
future baseline. The newer [curried-watching-clarke.md](../../archive/curried-watching-clarke.md) draft
identifies much of the schema debt but still conflates initial creation with forward migrations and
includes unsupported or unrelated capability work.

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and
[TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md) govern this replacement:

- Only Question Revision and Blueprint Revision are product Revision concepts.
- Mutable configuration uses current state and qualified Edit Numbers only where concurrent writers
  exist.
- Student Work retains exact evidence independently of mutable teaching configuration.
- This pre-production cutover preserves no existing development database or migration ledger.

This is a database-lifecycle reset. Before the first production deployment, the base schema remains
editable source and every structural correction is folded back into it. The first human-approved
production deployment makes that source immutable; only then do SQLx forward migrations begin.

## Objectives

- Replace the accumulated development migration history with one readable PostgreSQL-native
  database build.
- Make installation one canonical workflow: create the complete database structure first, then
  apply the required data-only seed phase.
- Fold every structural correction made before production into the database build and reserve SQLx
  migrations for immutable forward changes after the first production deployment.
- Remove Course Schedule Revision, Assignment Revision, Question Change Proposal Revision, and
  Course Retention Plan Revision from schema, code, APIs, and tests.
- Preserve exact Question Revision and Blueprint Revision provenance.
- Make released Assignment edits, future Attempt creation, Unrelease, published-content
  availability, and Blueprint publication conform to the current terminology contract.
- Retain only tests that protect stable behavior, authorization, evidence integrity, or the schema
  lifecycle.

## Design philosophy

- Model current domain truth directly instead of translating the old schema object-for-object.
- Keep PostgreSQL authoritative for constraints, transactions, RLS, concurrency, and
  destructive-operation closure.
- Keep runtime services verify-only with respect to schema.
- Prefer normalized, qualified evidence over generic snapshot tables or JSON fallback records.
- Include only complete capabilities with a domain, Store, Server, authorization, and user workflow;
  unfinished schema scaffolding leaves the new database build.
- Rebuild disposable databases from the edited base throughout pre-production. After the first
  production deployment makes that base immutable, represent every structural change as a forward
  migration.

## Scope

In scope are the base schema, required reference data, the installation seed-data boundary, schema
administration tooling, revision-related domain/store/server/browser contracts, Assignment evidence,
Unrelease, Published Question and Blueprint availability, Blueprint Draft publication, affected
tests, and durable documentation.

## Non-goals

- Import or upgrade existing development database contents.
- Build a complete Question Change Proposal or Course Retention capability.
- Change inactive Course cleanup, Student Account Time Zone UI, unrelated ribbon work, or the
  screenshot corpus.
- Add a compatibility reader for retired Revision fields or endpoints.
- Add a schema-diff framework, declarative-schema dependency, or maintained `pg_dump` source.
- Perform the production deployment itself.

## Final architecture decisions

### One canonical database build

`schemas/base_schema/install.sql` is a short structural installation manifest, not the combined
schema body. It contains only comments, transaction-safe `psql` settings, and ordered `\ir` includes
and should remain well under 100 lines. The roughly 20,000 lines of schema live in descriptively
named domain-family files such as `accounts.sql`, `questions.sql`, `blueprints.sql`, `courses.sql`,
`assignments.sql`, `student_work.sql`, and `grading.sql`. Each authored SQL file stays below the
repository's 1,000-line limit; a family splits again at a clear ownership boundary when needed.
Each base-schema module is the canonical source for that domain's current tables, constraints,
indexes, functions, RLS policies, and grants. Pre-production changes update the owning module
directly rather than append corrective SQL elsewhere. The structural manifest contains DDL only;
reference rows and teaching content belong to the separate data-only phase.

The existing database-administration job runs the manifest using PostgreSQL 17
`psql -X --set=ON_ERROR_STOP=1 --single-transaction`. These options isolate client configuration,
stop on the first error, and make the complete base installation atomic, as documented by
[PostgreSQL 17 psql](https://www.postgresql.org/docs/17/app-psql.html). PostgreSQL parses and executes
the SQL directly; the current custom Rust SQL statement splitter leaves the administration path.

Run a representative proof before reorganizing the full schema. The structural proof includes one
table, cross-file foreign keys, a function, a role change, grants, and forced RLS. It must
demonstrate successful atomic installation, complete rollback after an intentional late failure,
and execution inside the existing administration container. A second data-only proof loads required
reference rows after the structure exists and demonstrates convergent replay. If `\ir` cannot
satisfy the structural proof, record the failure and choose the smallest PostgreSQL-native manifest
alternative before the schema rewrite begins.

Database initialization has two explicit phases under one installation owner:

1. **Structure:** install the complete DDL-only base atomically, then apply any SQLx forward
   migrations created after production cutover.
2. **Seed data:** load universal database-owned reference rows and the approved starter teaching
   content through data-only seed/provisioning boundaries. This phase may run after the API and
   object store are available because teaching content uses normal product contracts and may own
   external objects.

The seed inventory classifies every current seeded row or object as universal database reference
data, production starter teaching content, or disposable Live Demo data. Production initialization
receives the approved starter teaching content. Fictional demo Accounts, sessions, Student Work,
and acceptance-only observations stay in the disposable Live Demo profile. The data-only seed is
convergent and never gains DDL authority.

### Pre-production and production boundary

Throughout pre-production:

- `schemas/base_schema/` is the editable source of the intended database.
- The required reference-data seed is editable installation source alongside that structure.
- Structural corrections update that source directly and validation rebuilds a disposable database
  from empty.
- `schemas/migrations/` contains no production forward migration.
- SQL files that merely correct the evolving base are folded into the appropriate domain-family
  file rather than appended as another layer.

The base becomes immutable only when a human approves the first production deployment and that
exact CalVer release tag and Git commit are recorded in `schemas/base_schema/README.md`,
[RELEASE_HISTORY.md](../../RELEASE_HISTORY.md), and [CHANGELOG.md](../../CHANGELOG.md). Before that recorded event the README says
`Status: editable pre-production source`; afterward it says `Status: immutable production source` and
names the deployment record. This explicit repository state tells future maintainers which
lifecycle applies.

After that event:

- The immutable files under `schemas/base_schema/` retain their exact bytes.
- The required reference-data seed used by fresh installations also retains its exact bytes;
  subsequent reference-data changes travel in forward migrations.
- `schemas/migrations/` becomes the sole SQLx forward-migration directory.
- `sqlx.toml` pins that directory and keeps SQLx's forward ledger in the restricted
  `ple_migration` schema rather than `public`.
- Each forward file uses a timestamp version, one bounded transactional responsibility, immutable
  bytes and filename, and no down migration.
- A fresh database installs the immutable base, applies every forward migration in order, and then
  runs the data-only installation seed.
- An existing database applies only pending forward migrations.
- Restore uses `pg_restore` followed by the same migrate and verify path; a rehearsal precedes the
  first production deployment.

The base defines one application-readable compatibility projection containing the base's release
identity and the read-only SQLx forward state. Before production it carries an explicit development
identity and disposable databases are rebuilt with the current source. At production cutover it is
set to the approved CalVer release tag and remains part of the immutable base. The runtime compares
that identity and the forward ledger with its embedded expectations.

The release tag, Git commit, atomic install, restricted compatibility projection, and SQLx's own
forward-migration ledger provide the retained integrity evidence. The initial design therefore has
no separately maintained base digest, digest loader, numbered baseline table, or detailed repair
classifier. The early proof may add one of those only after recording a concrete failure that the
simpler evidence cannot detect.

### Administration and role boundaries

Keep the existing database-migrator container and schema advisory lock. They already solve two
demonstrated repository needs: administration on the private Compose network and serialization of
schema changes against runtime verification. Extend that existing image with the PostgreSQL 17
client and the checked-in base files; create no second administration image.

Use three direct operations:

- `cargo tools database initialize` owns the fresh build. It accepts a database with no PLE schemas
  and no SQLx ledger, runs the base manifest, applies post-cutover forward migrations when present,
  verifies compatibility, and hands off to the data-only seed phase. A repeat against the exact
  compatible installation performs verification and convergent seeding without schema mutation.
- `cargo tools database migrate` applies post-cutover SQLx forward migrations to an initialized
  database and then verifies compatibility. Before the production cutover, the migration directory
  is empty.
- Application startup and `cargo tools database verify` share one read-only compatibility check
  through the existing restricted application-safe projection. The application role has neither
  DDL nor migration-ledger write privileges.

Initialization recognizes only two successful states: empty and exactly compatible. Any PLE schema
or legacy SQLx ledger that does not satisfy one of those states is refused as an unsupported
database and requires an explicit operator reset or restore decision. The tool reports observed
facts without implementing a repair/adoption framework.

The superuser-only platform bootstrap continues to create the database owner, migrator login, and
restricted capability roles that the migrator does not control. The base creates ordinary PLE
schema-owner and service roles. Credentials pass to `psql` through a redacted child environment,
without shell interpolation, command-line secrets, or diagnostic output.

### Domain revision model

Preserve only:

- Question Revision: immutable published Question content.
- Blueprint Revision: immutable published Blueprint content.

Replace the retired families as follows:

- Course Instance stores its current Course Term dates directly. Add no Course Instance Edit Number
  until an actual concurrent mutation route requires one.
- Assignment is one stable current aggregate with Assignment Edit Number, Assignment Status,
  authored policy, normalized Assignment Entries, and Question Pool Items.
- Every Assignment Question or pool item pins an exact Question Revision when selected. Newer
  Question Revisions never advance an Assignment silently.
- `BlueprintAssignmentRevisionReference` becomes `BlueprintAssignmentSource`: an exact Blueprint
  Revision Reference plus the stable Blueprint Assignment Reference. It is provenance, not another
  Revision family.
- Remove dormant Course/Assignment snapshot and successor-revision contracts that have no live
  Store or Server consumer.

### Assignment and Student Work evidence

Remove `assignment_revision`, its entry/pool snapshot tables, `released_assignment_revision_id`,
and all runtime Revision-shaped Assignment contracts.

The following list is the starting evidence inventory, not a frozen column list. Before the schema
owner finalizes it, trace the resume, response-save, submission, grading, history, disclosure,
statistics, presentation-reproduction, and future-Attempt consumers. Retain each effective fact
whose later interpretation must survive an Assignment edit and record the consumer that justifies
it:

- Assignment title and instructions.
- Availability, due, and close timestamps.
- Whole-Attempt time limit.
- Late-work, completion, feedback-release, reuse, variation, and question-order policy.
- Effective per-Student policy values and their qualified sources where accommodations or schedule
  adjustments affected them.

Attempt-start eligibility and future-attempt calculations use current state unless that consumer
trace demonstrates a later interpretation dependency.

Each Issued Question records:

- The Assignment Entry identity and issue position.
- Exact Question Revision.
- Question Seed and reproduction/presentation binding.
- Point value, scoring rule, statistics eligibility, and pool-selection source.

Attempt interpretation reads its retained Attempt and Issued Question evidence. Missing required
evidence fails closed as unavailable or inconsistent; mutable Assignment state is never substituted
for historical evidence.

Released Assignment saves remain allowed when the resulting current Assignment passes Release
Validation. Existing Attempts retain their evidence; later Attempts use the newly accepted current
state.

### Published content and Blueprint Drafts

Move Question and Blueprint availability from individual revisions to their stable lineages:

- Stable lineage rows contain current `Available` or `Archived` state and a qualified Edit Number.
- Append-only availability events record actor, transition, target, and timestamp.
- Archived content disappears from ordinary browsing and new selection.
- Existing exact Question Revision and Blueprint Revision references continue to resolve.
- Existing Assignment entries may retain and release archived Question pins; archived content
  cannot be newly added.
- Restore is an ordinary availability transition. Archive remains a Danger Zone operation.
- Publishing a new Revision never resets lineage availability.

The proposed creation transaction follows the durable Draft-to-publication model in
[TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md) and
[DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md): a new Blueprint Course creates its stable lineage
and one private mutable Blueprint Draft, but no Blueprint Revision. Draft saves require Blueprint
Draft Edit Number and increment it only on change. Human approval of this plan confirms that exact
create transaction because the authorities define the concepts and publication boundary but do not
spell out the initial transaction explicitly.

Each deliberate successful publication validates the complete Draft and copies it into exactly one
new immutable Blueprint Revision, even when its content checksum matches the latest Revision. This
follows the authority's definition of a Blueprint Publication Event as creation of a new Revision;
content equivalence remains evidence for comparison, not implicit publication deduplication. A
transport replay of the same already-accepted publication command converges on its existing receipt
and Revision. The owner's Draft remains the working copy for later edits.

The current revision-keyed Blueprint collaborator schema is incomplete scaffolding and leaves the
baseline. No Store, Server, authorization, or browser workflow makes it a product capability. A
future bounded collaboration capability starts from the Blueprint Draft, defines that complete
vertical and its publication transition, and then closes its active Draft relationships as part of
that approved publication transaction.

### Assignment Unrelease

Add one database-owned Unrelease operation and expose it through the Store and Server. It must:

1. Lock the Assignment first using the same ordering required of Attempt start, response save,
   submission, and grading commit.
2. Verify current Teaching Team authority, Released status, exact Assignment Edit Number, and exact
   title confirmation.
3. Count affected Assignment Attempts, submissions, and grades for confirmation and receipt
   reporting.
4. Change status to Unreleased and increment the Assignment Edit Number.
5. Delete the complete Student Work closure rooted at that Assignment's Attempts.
6. Rebuild statistics for affected Question Revisions from surviving observation receipts.
7. Insert one redacted Unrelease audit event containing actor, Assignment, aggregate deletion
   counts, outcome, and timestamp, with no Student identities, responses, or grades.
8. Commit everything atomically.

The rebuilt schema, rather than this draft's table inventory, defines the exact deletion closure.
After the base relations and foreign keys are designed, traverse `pg_catalog` dependencies from an
Assignment's Attempts and supplement that mechanical graph with indirect semantic owners such as
Assignment-scoped Gradebook calculations. Review one ledger that classifies every dependent as:

- Delete with Student Work.
- Recompute or revoke from surviving evidence.
- Preserve as shared content, non-Student domain state, or redacted audit evidence.

The candidate graph to reconcile includes Issued Questions, Question Attempts, saved responses,
presentation/replay and asset bindings, pool selections, submissions, grading jobs/results/receipts,
backend exchanges, statistics observations, and correction-target links. Shared Question Revisions,
shared assets, the Assignment, current Assignment Entries, Course membership, and the redacted
Unrelease event are expected survivors, subject to the same reviewed ledger.

Design root-oriented `ON DELETE CASCADE` relationships where the reviewed ownership graph proves
exclusive Student Work ownership. The guarded Unrelease routine explicitly handles recompute,
revoke, or indirect dependencies. Immutable Student Work tables deny ordinary update/delete
privileges; the dedicated no-login Unrelease executor is the sole deletion authority. In-flight
workers fail closed if Unrelease wins the Assignment lock.

### Schema-only capabilities excluded from the baseline

Remove Question Change Proposal, Course Retention Plan, and Blueprint collaborator persistence
rather than carrying incomplete scaffolding into the baseline:

- Remove proposal Revision tables and proposal-specific event/FK branches. Preserve implemented
  publication and Forced Question Correction evidence.
- Remove retention plan revisions, retention-specific job targets, and retention events. Preserve
  generic jobs and technical object cleanup.
- Remove `blueprint_collaborator_event` and its revision-keyed functions, triggers, policies, and
  foreign-key branches. The owner's private Draft lifecycle remains; a future collaboration
  capability begins as a bounded Draft-keyed vertical.
- Keep their intended current-state terminology in durable documentation and route full vertical
  capabilities through [TODO.md](../../TODO.md).
- Add proposal, retention, or collaboration tables only through a bounded plan that includes their
  Store, Server, authorization, and user workflow.

## Public interface changes

- Assignment Release becomes a state transition: `POST .../release` requires `If-Match`, returns
  `200 OK` with the current Assignment and new ETag, and no longer returns an Assignment Revision
  Reference.
- Released Assignment `PUT` continues to require `If-Match`; accepted changes affect future Attempts
  only.
- Assignment properties include a Released-only Unrelease impact projection with aggregate Attempt,
  submission, and grade counts.
- Add `POST .../unrelease` with `If-Match` and `{ "confirmationTitle": "..." }`. Success returns the
  Unreleased Assignment, new ETag, and aggregate deletion counts.
- Archive endpoints require exact lineage Edit Number and explicit confirmation; restore endpoints
  require the Edit Number but not Danger Zone confirmation.
- Blueprint Course creation returns the private Draft. Draft `PUT` uses its ETag. `POST .../publish`
  creates and returns the immutable Blueprint Revision.
- Remove Assignment/Course/Proposal/Retention Revision fields and decoders. Keep exact
  `QuestionRevisionReference`, `BlueprintRevisionReference`, and renamed
  `BlueprintAssignmentSource`.

Concurrency conflicts return `412 Precondition Failed`; invalid resulting content or confirmation
returns `422 Unprocessable Entity`; wrong lifecycle state returns `409 Conflict`; unauthorized or
non-resolvable targets retain the repository's non-enumerating response policy.

## Workstreams and sequencing

### Working evidence and durable decisions

Investigation notes, reviewer responses, experiment outputs, and the one-time object disposition
ledger are temporary working evidence held outside the repository. They support implementation and
review; they are not a second documentation system. Before retaining any artifact, determine
whether it duplicates or supersedes existing evidence and retain it only when it protects a durable
operational, security, or product need. As each decision is accepted, fold its conclusion into this
active plan while work is underway, then into the applicable canonical authority documentation,
implemented behavior, behavior-focused tests, and final changelog evidence. The temporary working
evidence can then be discarded.

### Authority and cutover ledger

- Replace the current backend plan with this plan and archive the superseded plan, candidate, and
  incorporated concern note.
- Produce a one-time, non-durable working-evidence object disposition ledger: every current table,
  function, trigger, policy, role, index, and API projection is marked Keep, Rewrite, or Drop.
  Fold accepted dispositions into the active plan and the canonical code, documentation, and test
  contracts; do not publish the ledger as a parallel repository document.
- Classify every current seed row and object as universal reference data, production starter
  teaching content, or disposable Live Demo data, with one named initialization owner for each.
- Record human approval of the proposed Blueprint Course creation transaction before schema work
  treats it as settled product behavior.
- Recheck worktree provenance before dispatch; it was clean during this review.
- Gate: no unresolved schema object, seed owner, or terminology decision.

### Schema lifecycle

- Prove the short `psql` manifest against representative roles, grants, RLS, functions, reference
  data, and intentional rollback before reorganizing the complete schema.
- Implement the domain-family base modules, simple empty-or-compatible eligibility check, existing
  restricted compatibility projection, SQLx forward directory, and initialize/migrate/verify
  operations in the existing administration image.
- Connect the data-only production seed and disposable Live Demo seed to the canonical
  initialization workflow after their inventories are classified.
- Retire the custom migration-acceptance splitter and the accumulated public-ledger assumptions.
- Gate: atomic fresh build, late-failure rollback, compatible replay, convergent data-only seed,
  application-role verification, and refusal of unsupported nonempty states.

### Current Assignment and evidence model

- Author final Assignment, Attempt, Issued Question, submission, grading, statistics, and
  API-function families directly in the base schema.
- Trace every Attempt interpretation consumer and use that evidence to finalize the retained Attempt
  and Issued Question columns.
- Update Rust Store/domain types and TypeScript decoders without compatibility aliases.
- Coordinate [wp_i3_assignment_creation.md](../workstreams/wp_i3_assignment_creation.md) so
  Assignment creation retains exact Question Revision pins instead of selecting only stable
  Question IDs.
- Gate: existing Attempt evidence survives released edits; later Attempts use current state.

### Published content and Blueprint model

- Implement lineage availability, archive/restore transitions, Blueprint Draft/Edit Number, and
  explicit publication.
- Rename Blueprint assignment provenance and remove retired revision and collaborator scaffolding.
- Gate: archive preserves exact references; Blueprint creation produces the human-approved private
  Draft state; each accepted deliberate publication produces exactly one new immutable Revision;
  replay of the same accepted command returns that Revision.

### Unrelease and destructive-operation security

- Derive and review the Delete/Recompute/Preserve ledger from the rebuilt foreign-key graph and
  semantic owners, then implement the privileged transaction, statistics rebuild, redacted audit
  event, Store/API surface, and impact counts.
- Independently review role membership, RLS, function ownership, fixed `search_path`, concurrency,
  and FERPA deletion behavior.
- Gate: accepted Unrelease deletes all and only Student Work; every failed precondition leaves
  status, evidence, statistics, and audit state unchanged.

### Cutover, test triage, and documentation

- Delete the 101 development migrations only after the new base passes connected acceptance.
- Reset disposable databases; provide no automatic legacy upgrade.
- Update database, authorization, contract, API, roadmap, file-structure, and test-evidence
  documentation; remove obsolete migration-history decisions.
- Fully regenerate Graphify so removed migration nodes cannot survive incremental indexing.
- Run repository, connected PostgreSQL, service, and relevant browser acceptance; record completed
  and skipped gates in the changelog.

The schema/lifecycle owner integrates every change under `schemas/`, the short base manifest, SQLx
configuration, and migration tooling. After the disposition ledger publishes stable table and
procedure contracts, Assignment/evidence, Blueprint/published-content, API/decoder cleanup, and test
triage may proceed in parallel in their separate Rust/TypeScript/test modules. Each lane proposes
needed schema changes to the schema owner, who integrates them in the owning domain module.

## Test plan

### Permanent tests to keep or rewrite

- Domain validation and qualified Edit Number behavior.
- Exact Question and Blueprint provenance.
- Assignment save/release/start behavior, including released edits and unchanged-save semantics.
- Attempt, submission, grading, history, disclosure, asset, and statistics behavior expressed
  without Assignment Revision vocabulary.
- Course-membership, ownership, RLS, worker-capability, and non-enumeration boundaries.
- Connected fresh initialization, compatible replay, intentional late-failure rollback, and
  application-role compatibility verification through the short base manifest.
- The data-only seed's stable classification, convergent replay, and absence of DDL authority.
- Unrelease authorization, title/ETag/status preconditions, deletion closure, statistics correction,
  audit redaction, and worker race behavior.
- Archive/restore and Blueprint Draft publication contracts.

### Tests to remove

- `assignment_revision_entry_snapshot_catalog.sql`.
- Unit or decoder tests whose only purpose is preserving retired Revision types.
- Tests that preserve revision-keyed Blueprint collaborator catalog details without a complete
  collaboration vertical.
- Exact migration counts, filenames, historical checksums, or full catalog inventories.
- Custom SQL splitter tests after the splitter is removed.
- Schema-layout tests that pin the number or names of base-family files; the existing repository
  line-limit gate remains sufficient for module size.
- Stale source-line override entries associated with removed migrations.

Trim the existing catalog oracle to stable security contracts: schema ownership, forced RLS, closed
grants, capability-role membership, restricted schema state, and runtime absence of DDL authority.

### One-time evidence, not permanent tests

- Compare the final base catalog and live API behavior against the Keep/Rewrite/Drop ledger.
- Run the representative `psql` manifest experiment before the broad rewrite and retain its result
  as temporary working evidence rather than as a permanent fixture suite or repository report.
- Exercise one temporary forward migration in a disposable checkout/database, including apply,
  no-op replay and checksum mutation; remove the temporary migration afterward. Add a permanent
  case only for a stable behavior not already protected by SQLx.
- Compare the final Unrelease Delete/Recompute/Preserve ledger with a populated database's actual
  dependency graph and record every indirect exception.
- Record Graphify before/after migration-node counts without turning the count into a gate.
- Rebuild and provision the real live-demo stack, then replay released-edit, existing/new Attempt,
  archive, Blueprint publish, and Unrelease journeys.
- Before production launch, rehearse backup restore followed by migrate and verify.

## Risks and assumptions

- The reset is justified because the system is pre-production and has no durable production data.
  Any environment requiring preservation is out of scope and must block cutover.
- PostgreSQL 17 and SQLx 0.9 remain the selected versions.
- A generated dump may be used as a one-time comparison oracle, never as maintained source.
- Missing behavior hidden in late migrations is mitigated by the object disposition ledger and
  real-stack replay, not by retaining migration history.
- The unrelated ribbon and screenshot active plans remain independent. Student Account Time Zone
  editing and full retention/proposal capabilities receive separate future plans.
- No global "zero Revision matches" grep is an acceptance gate. Final semantic review categorizes
  remaining occurrences as Question Revision, Blueprint Revision, historical documentation, or an
  actual defect.
