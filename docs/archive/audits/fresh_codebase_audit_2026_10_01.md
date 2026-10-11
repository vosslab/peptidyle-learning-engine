# Fresh codebase audit - October 1, 2026

Status: complete static audit of the captured current worktree; coverage reconciled.

This is a fresh review of the current codebase, including unchanged source, rather than another
review of the September 28 diff. The captured worktree has HEAD
`57f180c26975b54be367becb036e64767b95e8ec` and includes uncommitted work. Snapshot time:
2026-10-02 01:08:58 UTC (October 1 in Chicago). It accounts for 2,654 repository paths.

Human Guidance supplies direction, not automatic authorization to implement every sentence.
This review prioritizes teaching behavior, data integrity, clear ownership, and maintainable
contracts. Pre-production state permits direct foundational corrections without legacy support.
Explicitly deferred work, unfinished email/login readiness, and temporary build/wiring failures
are outside acceptance scope. Static source only: no tests, builds, services, browser sessions,
or repository compliance gates were run. Audit bookkeeping scripts only read source and produce
review artifacts.

The foundations have several sound ownership boundaries worth keeping. The highest-priority
corrections are the nonadvancing Pool concurrency token and native text-answer invariants. The
clearest unnecessary machinery is the no-effect Watch materializer and the expanding collection
of literal source/pixel compliance gates. These findings call for specific owner-level corrections,
not a broad rewrite or another layer of recovery infrastructure.

## Findings

### F1 High: Pool metadata concurrency checks do not protect the state being saved

Both [Pool support](../../../schemas/base_schema/50_functions/question_pool_support.sql) and
[bulk Pool search metadata](../../../schemas/base_schema/50_functions/question_pool_search_metadata.sql)
lock the Pool and compare an expected Edit Number, then change fields without advancing that
number. The [Pool update trigger](../../../schemas/base_schema/50_functions/question_pools.sql)
explicitly prevents the number from advancing when those fields change. Provenance replacement
has no expected version at all.

Two editors loading N can both save with N; serialization by the row lock does not detect that
the second editor is stale. The second replacement silently overwrites the first. This is an
intrinsic schema/command defect, not missing browser wiring.

Define the version scope once and enforce it at the owning command. One current-state Pool token
is simplest if all edits share that lifecycle. If the existing number deliberately identifies only
membership, retain that meaning and give mutable metadata a real advancing token. Every command
must compare and advance the token for the state it replaces. Do not add client retries, history
tables, or compatibility readers to conceal the mismatch.

### F2 Medium: shared statistics use issue counts as a proxy for the disclosed cohort

[QuestionUsageTotals::into_shared_statistics](../../../crates/question_model/src/question_library_statistics.rs)
checks only `issued_count` before exposing answer counts, outcome counts, credit sums, and rates.
The [SQL producer](../../../schemas/base_schema/50_functions/statistics.sql) increments per Issued
Question at submission; [issuance eligibility](../../../schemas/base_schema/50_functions/assessment_attempt_start.sql)
depends on the scoring rule, not distinct Student participation. The
[Library adapter](../../../crates/learning-data-access/src/postgres/question_library.rs) returns
these fields to the live server projection.

Five eligible observations with four blanks and one answered response pass the threshold. The
returned credit sum and mean then expose the one answered response's credit. Repeated Attempts
can also supply multiple observations from the same Student. Thus the guard does not implement
its own claim to suppress small contributing cohorts. This is a disclosure-contract defect;
the review does not claim identification of a particular person or offer a legal FERPA conclusion.

Define eligibility for each disclosed metric from its actual contributing cohort, and suppress
dependent totals/rates together when they reveal a withheld cell. Establish the count before
private records are purged; do not retain identities indefinitely to repair an aggregate later.
Keep the centralized projection, but make it the only public shared-statistics constructor and
give it the facts necessary to enforce its policy.

### F3 Medium: a Watch worker marks already-delivered events without producing a product result

[question_watch_notifications.sql](../../../schemas/base_schema/50_functions/question_watch_notifications.sql)
creates recipient rows synchronously when an event is inserted. Inbox reads use those rows and
do not depend on `processed_at`. The so-called materializer merely counts existing recipients
and sets `processed_at`. [worker.rs](../../../crates/server/src/worker.rs) runs it after each
Assessment expiry iteration, adding a store capability, grants, polling, and error handling.

The current in-app inbox works independently of that background lifecycle. Remove the no-effect
materialization path and its exclusive state if no other consumer exists. Preserve the synchronous
recipient snapshot and private inbox. This finding concerns demonstrably unnecessary coupling in
an existing path, not a demand to finish or delete deferred email infrastructure in general.

### F4 Medium: the compliance workflow rewards literal HG implementation and brittle gates

[human_guidance_checklist.py](../../../devel/human_guidance_checklist.py) transcribes every selected
HG bullet into a status-bearing checklist and gates exact bullet identity and prescribed evidence.
N/A is supported and useful; the mechanism is not inherently a mandate to write code. The problem
is using checklist closure as the implementation backlog and completion criterion, then writing
tests that preserve a particular interpretation rather than a product outcome.

Concrete examples include:

- [ribbon_profile_menu_contract.mjs](../../../tests/playwright/ribbon_profile_menu_contract.mjs):
  recursive CSS/source inventories and exact radii/borders on constructed examples.
- `tests/playwright/fast_ui_route_composition.mjs` (removed by the current audit repairs): exact
  alignment/inset comparisons derived from broad consistency guidance.
- [run_fast_ui_checks.sh](../../../launchers/run_fast_ui_checks.sh): many separate permanent
  visual probes for spacing, screen space, cards, density, consistency, and teaching-task layout.
- [test_ribbon_route_contract.mjs](../../../tests/test_ribbon_route_contract.mjs): an exact
  source-file inventory for upload controls rather than their role/behavior boundary.
- `tests/test_course_retention_ferpa.py` (removed by the current audit repairs) and
  `tests/test_blueprint_instructor_browse.py` (removed by the current audit repairs): SQL
  substrings and statement order offered as evidence for behavioral policies.

Retain meaningful keyboard, focus, decoding, authorization, persistence, and transaction checks.
Remove arbitrary source/pixel constraints from permanent gates. Treat broad design guidance as
review criteria; create work only for a concrete user need or independently established defect.
A smaller decision/evidence ledger is sufficient. No replacement compliance framework is needed.

### F5 Medium: Discipline-request fulfillment has two independent commit points

[createDisciplineFromRequest](../../../src/components/discipline_request.ts) creates a Discipline,
then separately resolves the request. If the latter fails, the Discipline exists, the request is
open, and retry tries creation again. The page cannot give this partial result one reliable meaning.

Keep the proportionate in-app request handoff. Put fulfillment in one Sysadmin-authorized server
command and database transaction, reusing existing normalization and creation logic. Do not add
browser compensation or a background reconciliation workflow.

### F6 Medium: capped collection contracts lack continuation

[Starred Questions](../../../schemas/base_schema/50_functions/question_stewardship.sql) returns
250 records and a truncation flag but no continuation input/token. The
[store contract](../../../crates/learning-data-access/src/question_star.rs) therefore cannot load
older Stars. The newer [Sysadmin Course list](../../../schemas/base_schema/50_functions/sysadmin_course_inspection.sql)
similarly stops at 251 rows without a continuation contract. Its search can still retrieve a
known target, so that case is incomplete browsing rather than permanent record inaccessibility.

Reuse the existing cursor-page design; do not raise the limits or add another discovery subsystem.
My Questions now has working source-level cursor propagation and shared page controls, and should
be preserved as the local pattern. No browser execution is claimed.

### F7 Medium: ordinary navigation still contains internal UUIDs

The [Review Attempt link](../../../src/pages/assessment_overview_page.tsx) and Attempt navigation
place the Attempt UUID in browser paths. The [Watch inbox](../../../src/pages/library_watch_notifications_page.tsx)
puts an activity UUID after `#library-activity-`. Blueprint search also creates a UUID return-state
token in visible query URLs. Fragments and query strings are visible and copyable, just like paths.
These are browser navigation, so they do not meet the user's clarified rule. Hidden API UUIDs
remain appropriate; the temporary return-state token can simply stay in browser state.

Resolve internal selection behind a route scoped by existing public Course/Assessment/Question
identity, or link to the public parent and select the record within the application. Do not invent
secondary persistent IDs merely to disguise UUIDs, and do not retain legacy redirects before
production. Authorization must remain on every internal read regardless of navigation shape.

### F8 High: native text-answer contracts permit impossible Questions and an editor hang

The [browser decoder](../../../src/features/ple_question_json_authoring/question_json_codec.ts)
and [Rust source validator](../../../crates/adapters/ple/src/question_json/source_compile.rs)
validate accepted answers and `maxLength` independently. An exact-match Question can accept only
`AB` while limiting Student input to one character. Both fill-in and multi-fill use this validator;
the Student controls enforce the shorter limit. Enforce answer/response compatibility at source
validation, including the backend boundary, using the same length semantics as the response contract.

Separately, [nextAcceptedAnswer](../../../src/features/ple_question_json_authoring/question_json_multi_fill_in_editor_model.ts)
repeatedly slices `Alternative N` to the maximum length. With `maxLength = 1` and `A` already
accepted, every candidate is `A`, so Add Accepted Answer loops indefinitely on the browser thread.
Use a terminating candidate strategy or let the Instructor enter the next answer directly. This
needs a local correction, not a generalized answer-generation service.

### F9 Withdrawn: membership-helper caller ownership was misidentified

The deeper follow-up traced `SET LOCAL ROLE` through the entire SQL files. The cited calls in
Student landing (389/488), history (190), and response statistics (158) belong to API functions
created after switching to `ple_api_owner`, not to the preceding private-owner functions. That
role has the event-table SELECT privilege and its RLS policy. The alleged permission failure
does not follow from these paths. Do not change the helper to SECURITY DEFINER based on this
finding. Extra unused EXECUTE grants, if any, are a separate narrow cleanup question.

### F10 Medium: active Genetics source misstates Angelman causation

Both [matching](../../../content/genetics/pg/topic01/genetic_disorders-matching.pgml) and
[which-one](../../../content/genetics/pg/topic01/genetic_disorders-which_one.pgml) teach UBE3A
duplication as the cause of Angelman syndrome. Maternal UBE3A loss of function is the relevant
mechanism; see [MedlinePlus Genetics](https://medlineplus.gov/genetics/condition/angelman-syndrome/).
Correct the canonical Question facts and their upstream owner together where applicable. This is
a content defect found during whole-source review, not a reason to add importer exceptions.

## Refinements

- **Installation-data gate:** [install.sql](../../../schemas/installation_data/install.sql) always
  invokes the 471-line Live Demo oracle, including exact mutable identities and lifecycle state.
  The accompanying README describes ordinary product data and convergent re-application. Keep
  stable installation invariants in apply; run exact fixture acceptance only when explicitly
  provisioning/checking that fixture. Normal lifecycle changes should not become deployment failures.
- **Scoring drafts:** invalid Points text leaves the prior model unchanged while the input can
  retain the new text. Preserve a local draft/error and prevent Save from silently keeping a score
  different from the displayed edit. This is a small form-state correction.
- **Statistics teaching prose:** chi-square term Questions equate inability to support the null
  with support for the alternative and contain `faill to support`. Use reject/fail-to-reject
  decisions and the conditional definition of a p-value; edit the source prose directly.
- **Fixture integrity:** Blueprint PostgreSQL test support inserts three different intended
  Accounts with the same primary key. Correct those fixture IDs before relying on the connected
  checks. This is a small current integration note, not an executed test failure or design verdict.

- **Curriculum publication replay:**
  [publication.rs](../../../crates/project-tools/src/curriculum_content/publication.rs) and
  [parameterized_publication.rs](../../../crates/project-tools/src/curriculum_content/parameterized_publication.rs)
  still require current title/description to equal manifest text before reusing an identical source
  checksum. A legitimate Library metadata edit can therefore make an otherwise idempotent import
  report a provenance conflict. Compare immutable source identity separately from mutable metadata;
  any intentional reset to manifest wording should be an explicit operation.

- **Pool provenance storage:** the single license and source rows share one save/load lifecycle.
  Consolidate those one-to-one tables while retaining ordered authorship. Decide who may replace
  attribution independently from who can discover a global Pool.
- **Legacy duration state:** the editor still blocks unrelated saves through
  `legacyTimeLimitSeconds`. Choose whether base overrides support whole minutes or seconds, align
  schema/domain/editor, and rebuild development data. Effective accommodation timing is a separate
  concern. Do not preserve a development-data repair workflow as permanent UX.
  The publication adapter also retains a legacy primary-key-error fallback beside its canonical
  `QP001` collision result. Verify the current SQL producer and remove that old interpretation if
  it has no remaining producer; no runtime legacy schema needs support.
- **Unused StudentWork aggregate:** `StudentWork::collect` has no production caller and validates
  only part of the relationships it claims to collect. Do not add loaders or integrity machinery
  solely to instantiate a glossary term. Keep or remove it based on a concrete consuming operation;
  incomplete wiring alone is not proof of a product defect.
- **Author JavaScript dependencies:** the existing closed library enum/server registry is a clear
  authority. The added external-Script syntax and duplicated Rust/TS CDN/dependency inventories
  create another future authority despite an empty CDN allowlist. Simplify this latent surface;
  no active arbitrary external-script path was established.
- **Warm-build recovery:** [build.mjs](../../../pipeline/build.mjs) clears `dist/` before bundling
  and copying finish, while the live gateway bind-mounts it. A later failure can leave the running
  demo incomplete. Prepare completed output before publication. Account for directory bind-mount
  semantics; renaming the mounted directory alone is not a safe publication design.
- **Small client-contract fixes:** represent unavailable timestamps explicitly after settings-load
  failure; show local Star mutation errors; reuse the point-value domain bound; keep identity types
  across the Rust boundary. These are bounded corrections, not reasons for new state frameworks.
- **Maintenance:** split the saved-response mega-oracle by independent behavior if retained;
  eliminate duplicated schema COMMENT blocks; update current screenshot captions at their manifest
  owner. The unquoted `find` output in `tools/rmpycache.sh` also needs a small safe argument fix.
  Two Genetics manifest descriptions describe different topics from their selected Questions;
  repair those source values without rebuilding the content-import architecture.

## Designs worth preserving

- SQL owns authorization and transactional mutations. Canonical lock order and advancing metadata
  tokens make bulk Question metadata editing substantially stronger than browser write loops.
- Native Question summary construction now overlays mutable title/description from current
  metadata while retaining immutable content validation. The previous metadata-read defect is fixed.
- Immutable Question/Blueprint pins remain separate from current recognition titles. Title
  enrichment does not rewrite historical content identity.
- PLE-managed support and backend-generated feedback can coexist. HG explicitly distinguishes
  them; conditional correct/incorrect feedback is not identical to general feedback. Do not delete
  one merely because the editor contains both. Clearer native Hint labels are a small refinement.
- The narrow audited support-capability design has a real task-scoped access requirement. Keep its
  authorization/revocation boundary; a generalized repair platform is unnecessary.
- The Discipline request table is a small, useful handoff while email is deferred. Its transaction
  needs correction; the workflow does not need removal.
- Shared rendering/response controllers keep answer keys and grading on the server, reject stale
  asynchronous client validation results, and reuse answer-free inspection.
- My Questions pagination, keyboard alternatives to reordering, Account-scoped appearance results,
  and direct terminology cutover are sound current-source improvements.
- Asset ownership is explicit: Rust generates browser contracts, a manifest owns provided avatars,
  the schema catalog owns table documentation, and the lockfile-installed package owns RDKit bytes.

## Judgments not promoted to defects

The Student WeBWorK frame deliberately has same-origin script privileges for its bridge. Its live
CSP allows same-origin scripts, blocks inline/foreign scripts, and blocks form actions; previews use
an opaque response sandbox. This is a privileged renderer trust boundary that should remain explicit,
but this review did not establish an attacker-controlled executable path. The initial "sandbox escape"
candidate was withdrawn. Removing one iframe attribute would break the bridge without completing a
coherent isolation change.

SVG conversion remains a localized upload concern, not a foundational headline. Deferred SMTP,
retention automation, iMathAS/H5P/AI, and future deployment are not completion requirements. A
container-context/COPY mismatch is recorded as transient integration evidence, not an architecture
failure. Module counts alone do not establish overengineering. Explicit file-selection tools and
global maintenance commands are not defects merely because their documented scope is broad.

## Coverage and limits

All 2,654 captured paths have one assessment and a recorded review depth in the
[complete CSV ledger](fresh_codebase_audit_2026_10_01_coverage.csv). It includes snapshot and final
source hashes. The final comparison found no drift in those inputs before this audit's Changelog
receipt. Audit outputs and that receipt are the only repository edits made by this review.

| Area ledger | Paths | Scope |
|---|---:|---|
| [Plan](fresh_codebase_audit_2026_10_01_plan.md) | 397 | Schema, grants, policies, data-access contracts and implementations |
| [Test](fresh_codebase_audit_2026_10_01_test.md) | 472 | Full test/helper/fixture source reading; no execution |
| [Style](fresh_codebase_audit_2026_10_01_style.md) | 196 | Pages, feature models, authoring, navigation and styles |
| [Comment](fresh_codebase_audit_2026_10_01_comment.md) | 305 | Browser API, decoders, shared components and ownership claims |
| [Legacy](fresh_codebase_audit_2026_10_01_legacy.md) | 505 | Rust models, domain, server, adapters, objects, WASM and content |
| [Docs](fresh_codebase_audit_2026_10_01_docs.md) | 589 | Current authorities, interpretation, generated docs, history and assets |
| [Coordinator](fresh_codebase_audit_2026_10_01_coordinator.md) | 190 | Root configuration, build, tools, operations and deployment boundaries |

Depth totals are 2,031 full-source reads, 148 generated/derived-owner reviews, 257 asset-metadata
reviews, and 218 historical-reference reviews. Generated content banks were assessed through their
owner/provenance; selected canonical PGML source was read in full. Binary screenshots were not
visually re-audited. Historical plans were not converted into current requirements.

Six independent review passes plus bounded supporting readers were used. Reconciliation caught
partial-source receipts in several initial passes; fresh full-source reads replaced those before
closure, including unchanged modules and all 472 test-area files. Coordinator adjudication removed
unsupported findings about deferred backends, intentional feedback ownership, iframe isolation,
scoped support access, broad maintenance commands, and module counts. File-level concerns overlap;
the numbered findings above are not a count of every flagged file. F9 was subsequently withdrawn
after the deeper follow-up traced function ownership across intra-file role switches.

The inventory includes tracked and nonignored untracked files present at capture, including existing
uncommitted work. Ignored external checkouts, installed dependencies, build outputs and Git internals
are excluded. Two tracked-but-absent paths were accounted for as deletions, not missing code defects:
`src/pages/assessment_workspace/assessment_workspace_selected_entry.tsx` and
`tests/test_student_assessment_presentation.mjs`.

Complete static coverage is not runtime acceptance or a claim that no defects remain. No tests,
builds, services, browser sessions, compliance gates, dependency-advisory scan, or production
deployment were performed. Deferred work and temporary integration gaps remain outside acceptance
scope. Findings describe the captured source and should be rechecked if their owning code changes.

## Goal completion reconciliation

The reference is `e67eacab4ceeafe655d5b2b7ca051b7acda7c8df`. A fresh name-status comparison
against the current worktree identifies 515 tracked changed paths: 513 are in the current-source
ledger and two are the deletions listed above. The
[reference-change ledger](fresh_codebase_audit_2026_10_01_reference_changes.csv) accounts for each.
Nonignored untracked source was independently included in the whole-worktree inventory.

Both deleted files were additionally read in full from the reference commit. The removed entry
presenter still has an import in the in-progress workspace view; this is explicitly excluded
temporary wiring, not a demand to preserve its old implementation. The removed Student presentation
test contained entry-count, duration and viewer-zone checks. Its absence does not justify restoring
legacy-duration expectations or claiming a runtime regression without execution.

| Objective | Completion evidence |
|---|---|
| Account for every reference change and inspect current surrounding code | 515-row reference ledger plus 2,654-path current-source ledger; both deletions read from baseline |
| Identify good, bad and refinable design in each area | Seven adjudicated area ledgers; consolidated findings, refinements and preserved designs above |
| Scrutinize interpretations and disproportionate machinery | F3 Watch materializer and F4 checklist/gates trace actual producers, consumers and planning mechanisms |
| Prefer foundational fixes and remove legacy obligations | F1 version ownership, F5 atomic fulfillment, F8 source invariants, duration and provenance refinements |
| Respect deferred and in-progress scope | Explicit exclusions and withdrawn candidates; no implementation, tests, builds or service execution |
| Verify evidence is current | HEAD remains `57f180c26975b54be367becb036e64767b95e8ec`; all captured hashes still match except this audit's Changelog receipt |

The preceding review turn completed the audit and produced evidence; it was progress. This closure
pass verified the reference-change boundary and source freshness rather than treating the report's
completion statement as proof. Completion here is completion of the requested audit, not remediation
of its findings or production readiness.
