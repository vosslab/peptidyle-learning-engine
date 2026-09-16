# Human Guidance implementation compliance summary

## Current heading reconciliation

Current Human Guidance SHA256 is
`9e92c864a019d89ef9952cfb6e3b9c05a4f45d6055b6c3f49520c400f4dc7055`.
All nine existing generator part gates, identity diff, and consistency passed. The generator and
checklist agree on 1,038 occurrences: 441 verified, 547 open, and 50 N/A. The first-owner
inventory is 432 verified, 533 open, and 50 N/A across 1,015 distinct identities; 23 are later
duplicates. Sixty-nine new or changed requirements remain pending independent audit. This is a
current inventory receipt, not a new Human Guidance closure or a product-acceptance result.

The prior 998-bullet snapshot (449 verified, 499 open, 490 owning-open, and 50 N/A at
`e81d5bb0a63cfb7d3ca5f4f34287e5155dc9d20b0b91cb856fdbefa1ef6fa82b`) remains historical
provenance only. Earlier scoring, timing, Blueprint, terminal-Attempt, and bounded MATCH receipts
retain their stated boundaries.

Product work remains in progress. The Course-classification foundation and Store HTTP slice use
mandatory Discipline with optional hierarchy and tags, independent Course Instances; support work
must retain the exact issuer of Course authorization; and compact Student details UI work is
independent. Ownership remains at the classification storage/Store HTTP boundary, Course
authorization boundary, and Student-details UI boundary respectively. The only shared dependency
is authorized Course identity where a surface needs it; no inheritance, Instance synchronization,
or cross-slice completion follows.

The completed repair is limited to required-classification fixtures in
`tests/test_ple_question_json_authoring.mjs`, which passed 26 tests. See
`/private/tmp/ple-authoring-fixture-classification-repair.md`. This does not claim an all-tests
result or Course-feature completion.

Current bounded progress receipt: root `CARGO_INCREMENTAL=0 cargo check -p server_core -p
project-tools --tests` passed in 11.37 seconds; `cargo tsgen` generated 369 types; and fresh
PostgreSQL 17 canonical installation through `ple_migrator` was repeated and passed. These results
do not establish all SQL or whole-feature acceptance. The self-contained actual-role
Course-classification proof passed at `/private/tmp/ple-course-classification-actual-role-result.log`,
covering create/authorization, ETag no-op/stale/history behavior, no Revision change, exact pins,
65 hierarchy tags, and fork/Instance independence. The corrected actual-role support proof passed
constraints and rollback at `/private/tmp/ple-support-exact-authority-result.log`; independent
review accepted source and corrected proof. This is Student-roster-only support evidence: Course
and content remain open, while durable HTTP support E2E, browser, and deployed acceptance remain
unclaimed because the E2E did not run and helpers remain under repair.

Independent source/render review accepted compact Student S04/S05 details at
`/private/tmp/ple-student-rules-disclosure-review.md`. Root
`node /private/tmp/ple-student-rules-proof.mjs` exited 0 at 1280 and 390 pixels. This is a
component-only receipt, not full shell, theme, zoom, or all-Student-interface acceptance.

Accepted R-4 desktop/phone terminal receipts hide active navigation and visibly label three
no-response records Unanswered, incorrect `0 / 1`; the four exact MATCH pairs remain correct
`1 / 1`, total `1 / 4`. Native diagnostic `AZA01TD` / `R-5` follow-up saved/reloaded FIB, MA,
MULTI-FIB, NUM, and ORDER, then submitted the whole Attempt: four correct `1 / 1` responses,
deliberately partial-reordered ORDER incorrect `0 / 1`, total `4 / 5`. Practice-default permitted
correct answers are displayed separately from retained responses. The supplied ledgers and
`submitted-review-1280.png` / `submitted-review-390.png` under
`/private/tmp/ple-student-types-proof/` are bounded receipts, not all-eight-type, complete keyboard,
touch, or contrast acceptance. HOTSPOT and WeBWorK coverage remain open. Earlier topical
inventories/correction IDs and superseded contradictions below are historical provenance.

Later bounded accommodation receipt: actual current-demo Elena controls saved Avery's 1.5X/2X
as 2700/3600 seconds from base 1800, custom 100 as 86400 capped, then restored Standard 1800
uncapped and the original null multiplier. Authenticated Avery Attempt-context reads retained
expiresAt=1789577036608 before/after. `/private/tmp/ple-accommodation-live-restored.png` records
restored state. Both accommodation rows remain open for independent source review and remaining
default/override, malformed-input, authorization-denial and race proof; earlier incomplete-slice
notes below are historical context, not a rejection of these newly observed valid writes.

## Authority and method

### September 16 bounded SQL corrections receipt

Accepted bounded corrections are recorded: finalized `QuestionResponse` identities are
consistent across code, API, and SQL consumers, with parent Assessment submission, owning Attempt,
and equal finalization time proven by four fresh PostgreSQL 17.11 cases; `save` and `formatOnly`
keep native responses editable after an accepted save; private receipt-gated anonymous statistics
increment atomically, retain no identity ledger, and survive Unrelease; and the support repair
command removes its redundant Account lock while fixing ambiguous use/revoke references without
new Account grants. The connected Unrelease oracle, actual-role support proof, and the 25-case
installed mandatory-NULL LIKE matrix passed. Root-observed `cargo tsgen` (367 types), combined
server-core/project-tools tests, wasm-bridge tests, wasm32 target-feature check, TypeScript, and
19 focused response-control tests passed.
Fresh PostgreSQL 17.11 installation through `ple_migrator` retained exactly three valid unique
indexes with no duplicate access paths; independent review accepted the three removals.

This is not whole-Course retention, public-statistics disclosure, connected-browser/deployed-app,
or global Human Guidance acceptance. Course/Pool classification, Bloom, Change Proposals, recovery,
and resource-specific support authority remain audit work. The current reconciliation receipt above
supersedes the former generator-drift statement without changing this evidence boundary.

### September 16 shared classification finding

Current HG requires one global Discipline -> Subject -> Topic -> Subtopic vocabulary for Courses
and Library Objects, not Assessments. Courses require Discipline and may have Subject; Library
Objects require both. Subjects may belong to multiple Disciplines; Topics and Subtopics each have
one parent. The independently accepted prerequisite now includes
[`content_classification.sql`](../../../../schemas/base_schema/content_classification.sql) and
[`content_classification_operations.sql`](../../../../schemas/base_schema/content_classification_operations.sql):
four global UUID vocabulary tables, a global case-insensitive Subject-name index, composite
Subject-Discipline identity and foreign keys, mandatory Topic/Subtopic parent foreign keys,
owner-only `FORCE ROW LEVEL SECURITY`, and no direct runtime DML grants.

The isolated fresh PostgreSQL 17 runner exited 0. Its
`/private/tmp/ple-classification-commands-artifacts.fZuHxB/{install,proof,concurrency-result}.log`
records trim-before-validation, bounded-name rejection, global Subject uniqueness, role-aware
creation and selectors, explicit existing-Subject association, and Sysadmin-only nonempty shared
association replacement with rollback. Vetted active Instructors create Subjects, Topics, and
Subtopics and may add associations; active Sysadmins create Disciplines and perform replacement.
The real two-session result shows addition waits for replacement and retains both associations.
Independent review accepted this bounded SQL command receipt. No hash attestation was recorded for
the logs, and no live database was modified.

This is not global classification closure or deployed-state evidence. HTTP, editors, content
attachments, search, lifecycle, and deployment remain outside it; existing Question free text is
unchanged. No inheritance or synchronization behavior is implied.

### September 16 Published Question classification receipt

The isolated fresh PostgreSQL 17 runner exited 0 for the bounded Published Question slice; its five
PASS notices are in `/private/tmp/ple-question-classification-artifacts.wOeMb3/proof.log`. New
Question-lineage publication and bulk metadata replacement now carry UUID hierarchy metadata:
Discipline and Subject are required, while Topic and Subtopic are optional only when the complete
stored hierarchy is valid. The proof covers valid publication, atomic invalid or stale refusal,
exact Question Revision and source-byte preservation, installed Student mutation denial, and
retained association repair with referenced-association removal refusal. The root-observed
`cargo check -p server_core` also exited 0; independent static review accepted the Rust handoff.

This is not HTTP, browser, producer/tool, or whole-classification closure. Projecttools input
contracts and TypeScript generation/browser consumers remain pending. No checklist count or
whole-checklist status changes follow from this receipt.

### September 16 classification consumer and publisher receipt

The classification selector transport, its strict browser decoder, and the Published Question
consumer source are now implemented. Publish Review and bulk metadata editing select shared UUID
classification-hierarchy identities without defaults or text-to-identity resolution: Discipline and
Subject remain required, Topic and Subtopic remain optional, parent changes clear only dependent
local selections, and bulk editing retains explicit Keep/Replace/Clear behavior. The source reports
require pre-existing authorized hierarchy identities and parent associations for new imports; they
do not provision vocabulary, create associations, infer a classification, or supply automatic
defaults.

Both publisher inputs now carry authored classifications for all 42 curriculum sources and two
Pilot chapters. The curriculum subjects retain their authored PG `DBsubject` mapping. The supporting
restriction-enzyme Genetics exercise carries Biology/Genetics. Authored source input therefore no
longer blocks a full batch, though actual publication remains unverified. Recovery returns an
existing immutable publication without comparing or resolving mutable classification metadata; all
new-lineage classifications are resolved before publisher writes.

The root-observed `cargo check -p project-tools` passed after the recovery correction in 7.51
seconds. Shared `npx tsc --noEmit` passed earlier, and `cargo tsgen` refreshed 367 types. The
previously recorded fresh PostgreSQL behavioral proof remains the evidence for storage behavior;
these source, compiler, and temporary protocol receipts do not establish installed-session HTTP
reads, browser interaction, deployed state, import execution, or a global classification feature.
No new count-audit machinery, checklist-count change, or whole-checklist closure follows from this
receipt.

The fresh-install and replay fixture at `schemas/installation_data/content_vocabulary.sql` now
provisions two Disciplines, six Subjects, and six Subject-Discipline links while preserving both
identity and association behavior; it creates no Account or session. The root fixture receipt
exited 0 at `/private/tmp/ple-installation-vocabulary-artifacts.Fp9q5z/proof.log`, and
`cargo check -p project-tools` passed in 1.22 seconds. Independent review accepted the fixture;
`cargo check -p project-tools --tests` also passed in 8.06 seconds. This bounded installation-
vocabulary evidence does not establish full HTTP/browser behavior, deployment, or project-tools
test execution.

### September 16 isolated classification HTTP/SQL receipt

An isolated fresh disposable PostgreSQL 17 and MinIO installation running the current host binary
exited 0 for a narrow installed-session receipt for the shared classification hierarchy. A vetted
active Instructor and an MFA-attested Sysadmin each received `200` from the Discipline,
parent-filtered Subject, parent-filtered Topic, and parent-filtered Subtopic selectors. Anonymous,
Student, and inactive-Instructor requests received identical concealed `404` responses. The
receipt then published one native Question with explicit Biology/Genetics metadata and changed
only its Subject to Biochemistry through bulk metadata editing. Exact Question Revision and source
binding rows were unchanged before and after the edit; a stale metadata request returned `412` and
left the metadata unchanged. The temporary runner and retained artifacts were inspected as
evidence only, not linked as permanent documentation; no Live Demo state changed.

This is not browser HTTPS acceptance, provisioned full-Course acceptance, or rendered
selector-workflow acceptance. Those boundaries, deployment, and global classification closure
remain open. This receipt does not change checklist counts or claim Human Guidance closure.

### September 16 native Student rendering receipt

Ordinary fake-Instructor publication and Assessment creation produced diagnostic Assessment
`AZA01TD` and Student Attempt `R-5`. Actual Student FIB, MA, MULTI-FIB, NUM, and ORDER responses
were saved and restored through reload or a fresh session. Temporary ledgers and 1280/390 captures
are under `/private/tmp/ple-student-types-proof/`; ORDER includes keyboard movement. These checks
are one-time evidence, not permanent tests or complete keyboard/touch/contrast acceptance.
This initial save/reload receipt did not establish whole-Attempt submission. The later `R-5`
follow-up above adds bounded submission/review proof for these five types only. HOTSPOT and
WeBWorK rendering coverage remain open.

Observed wording that implied independent Question finalization was corrected without changing
persistence or grading.
Build `2bf1a166` and a fresh NUM save/reload capture show "ready to save" alongside the separate
whole-Assessment Submit action. TypeScript, scoped formatting, and 14 existing response-control
tests pass. Independent review identified three false claims in the first patch; corrections
remove local-reset persistence claims, preview feedback promises, and unavailable preview-action
instructions. Independent re-review accepted the corrected diff with no remaining bounded
findings; broad UI checklist rows remain open.

[Human Guidance](../../../HUMAN_GUIDANCE.md) is the product authority. This report summarizes the current
statuses in the authoritative
[implementation checklist](../../audits/human_guidance_implementation_checklist.md), not a
replacement for it. Docs-pass compliance reports provide context only. The counts below are recomputed
from the checklist by `devel/human_guidance_checklist.py`. Each `[x]` is required by the checklist to
carry repository evidence, while `N/A` records document meta-guidance, human ownership, or explicitly
future/unlocked items rather than skipped work.

This refresh does not independently reconcile the topical-report inventories or correction-milestone
mappings. The topical reports are non-additive narrative views, not sources for checklist totals or
per-bullet status. Later duplicate open bullets with an `Owner:` pointer are excluded from the
owning-open count. The unfinished implementation-compliance product goal remains tracked by the
checklist, gap map, and active plan.

## Current checklist status

| Checklist part | Verified `[x]` | Open `[ ]` | N/A | Total |
| --- | ---: | ---: | ---: | ---: |
| 01 | 12 | 39 | 33 | 84 |
| 02 | 25 | 18 | 8 | 51 |
| 03 | 35 | 62 | 0 | 97 |
| 04 | 68 | 70 | 0 | 138 |
| 05 | 27 | 34 | 2 | 63 |
| 06 | 33 | 74 | 0 | 107 |
| 07 | 77 | 113 | 2 | 192 |
| 08 | 75 | 78 | 4 | 157 |
| 09 | 89 | 59 | 1 | 149 |
| **Total** | **441** | **547** | **50** | **1,038** |

The Assessment part closes four exact current-rescore rows. Independent review accepted
private PostgreSQL 17 production-SQL lifecycle proof at
`/private/tmp/ple-current-rescore-proof/proof.sql`; the artifact
`/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` exited 0. It retained `0.5` credit,
used authorized expected-current `ple_api.save_assessment` to change points from `8` to `13` and
advance the edit number, then observed `6.5 / 13`, unanswered `0 / 13`, and `6.5 / 26` through
three replay, history, Student landing, and Instructor Gradebook reads. The evidence hashes for
grading results, finalized Question responses, Assessment submissions, and automated grading receipts were
unchanged. The no-Backend-interaction row remains open: SQL `already_submitted` has no work fields
and the server bypasses ready-path Backend work, but actual HTTP request counts and Backend
transport were not observed. C510 is closed: the production Gradebook and Student landing share the highest-score
selector, while latest-Attempt progress remains separate; the stale configurable grade-rule enum,
field, SQL columns, and editor choices were removed. Accepted independent PostgreSQL 17 evidence
observed individual scores `12`, `4`, `16`, then an in-progress `NULL` fourth Attempt, with both
projections reporting `12 / 16`, `12 / 16`, then `16 / 16`. The fixture limit was four Attempts,
so this does not establish unlimited-Attempt eligibility; it also does not establish HTTP,
rendering, or actual backend grading. Artifact:
`/private/tmp/ple-highest-score-proof/artifacts.rQxVs8/proof.log` (exit 0).


The Blueprint authoring and lifecycle checklist now reflects the current narrow evidence. Reusable
Blueprints have no delivery dates; their availability is exactly Private, Public, or Archived; new
Blueprint creation is Private; owner-only Private visibility and Public-only adoption are enforced;
and the accepted lifecycle contract covers the directed transition and Archived read boundary.
Private is not adoptable. The selected Blueprint Assessment editor's Questions and Properties tasks
now have accepted compiled-main, actual-loopback-HTTP evidence for one selected Assessment, shared
drafts, all six Student-feedback controls, ordinary Save, and exact reload. The full Guidance
Properties scope remains open because scoring, attempt, and late-work activity controls were not
exercised. Public
Blueprint search is verified only for the accepted C47 name-search workflow; no Course creation,
login/TLS, full accessibility, or broad chooser acceptance follows.
Accepted bounded history proof covers Public and Archived Instructor Revision and recorded-metadata
facts, an exact older read-only Revision, Private-foreign and Student denials, and a read-only data
fingerprint at `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB`; it does not claim browser
pagination or retry, local-draft preservation, login/TLS, or populated Student Work.

The fixed-Question Assessment reference invariant is verified by accepted authenticated HTTP
evidence: Revision 1 remains pinned after fixture head Revision 2, and ID-only/missing-Revision
payloads are rejected without product changes. Pool-copy, initial-member-pin, independent-fork,
and no-Pool-member invariants remain source-audit pending; this does not claim broad runtime
verification.

Accepted current-pair evidence now verifies Public-to-Private fork creation, fresh local Assessment
and Pool identities with exact Question Revision membership, source provenance, independent fork
changes, and explicit owner-selected Apply. The 18-request lineage receipt at
`/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`
covers visible sibling and transitive pairs, Private concealment, current heads, shared Question-ID
relationships through renamed/reordered/split content, and unrelated-pair denial without product or
Student Work mutation. The 84-request Apply receipt at
`/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo/blueprint-local-id-apply-http-proof.json`
proves target-local existing/new destinations, four zero-write denials, and injected-fault rollback.
Compiled-main evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json`
completes comparison and selected Apply at 1280px and 390px. Separately accepted compiled-main,
actual-loopback-HTTP evidence at
`/private/tmp/ple-blueprint-owned-pool-artifacts.8PN6aH/comparison-browser.json` expands changed
content and verifies DTO-backed titles, instructions, fixed Question IDs, Revisions, and points.
No populated Student Work, login/TLS, full accessibility, or broader C413 workflow claim follows.

The Archived Blueprint read-only row is verified. Owner Save and rename lock the Blueprint and
reject Archived state before replay, CAS, or no-op handling; the existing lifecycle regression
preserves metadata, Revision, content, events, and receipts across denied writes, while restoration
permits Private/Public writes. Accepted actual HTTP proof recorded five `409` denials with unchanged
Blueprint state, `200` owner/nonowner historical reads, `404` nonowner writes, and `200` restored
writes: `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json`.
That read-only receipt does not itself verify explicit-include Archived browsing, forking,
adoption, populated daughters, concurrency, or a broader Course milestone.

Two Archived Blueprint discovery rows are verified. The list Store carries an explicit boolean
through `ple_api.list_blueprint_courses`; default and `false` return only Public plus the caller's
Private Blueprint, while `true` also returns Archived Blueprints to both active vetted Instructors.
Independent review accepted the source/runtime evidence. Accepted actual-server HTTP proof covers
owner/nonowner membership, nonowner Archived detail, Private concealment, Student denials, and
invalid-query `400` results. Accepted compiled-main browser proof covers default-off, Include
Archived, a read-only actual Archived detail, and return to off with eight GETs and zero writes.
Artifacts:
`/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json` and
`/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json`.
The fixture uses privileged Published-Question seed data and fixture sessions; it does not claim
login, TLS, pagination, concurrency, publisher-caller preservation, forking, adoption, or a
broader Course milestone.

Six Course-update rows are newly verified. An authorized Instructor lazily opens the current-parent
Course summary and receives changed, matching, removed-source, Type-mismatch, and automatically-
added adopted-Assessment rows; direct local Assessments are excluded. At 1280 by 900 and 390 by
844, accepted actual-server and compiled-main proof reviewed/reopened the summary, used the
existing detail Apply with exact source Revision 2 and daughter Edit CAS, and refreshed to the
matching row. Cancel made zero POST requests. Student and unrelated-Instructor reads returned
`404 no-store`; a private parent was concealed from another Instructor in the privileged-
availability fixture; Archived review remained available and new adoption was denied. Artifact:
`/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`. The earlier one-Assessment proof
separately covers exact pins, stale/no-op/invalid-Released cases, dates, status, origin, and one
populated Assessment Attempt hash: `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`.
Neither receipt claims direct Assessments or all Student Work. No persisted offer, receipt,
comparison baseline, or new update table is introduced, and no whole-Course lifecycle milestone is
closed. The fresh full
database-baseline gate passed as `database baseline E2E: PASS` in
`/private/tmp/ple-course-summary-baseline.log`.

C15's higher-security Sysadmin row, three automatic-new Blueprint Assessment rows, two
existing-Assessment non-silent-change negative invariants, and two older-Blueprint-Revision
indication rows are newly verified. The revision indication uses the existing authorized Course
load and Course page: an accepted independent actual-server/exact-main browser proof covers empty,
current, newer, and synthetic Private-origin states, with nonenumerating Student/unrelated-Instructor
`404 no-store` denials and no extra Blueprint fetch or write. Its visually inspected newer capture
shows adopted and current Revision values; the proof preserved the original adoption pin, Assessment,
and entries, while Work tables were empty. Artifact:
`/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`. It supplies no update offer, review,
approval, or apply workflow. Accepted independent
SQL and actual-server HTTP receipts require genuine private TOTP
before Sysadmin session creation and deny missing/wrong binding, bad codes, replay, expiry,
counter reuse, and a fresh unused valid counter after five failed attempts. Ordinary Student and
Instructor sessions and limited grants remain intact. Artifacts:
`/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H` and
`/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`. Transport was loopback HTTP,
not deployed TLS. Connected Blueprint Save proof preserves exact pins/settings, fresh daughter
Pool IDs, Unreleased state and unset dates across two daughters including an inactive Course,
existing actual Student Work, and the original adoption pin; an unrelated empty Course remained
unchanged, and replay/no-op/stale saves do not duplicate the append. Changing a retained source
Assessment title leaves existing daughter content, entries, and actual Student Work unchanged.
The extended existing permanent lifecycle test passed and supplemental temporary bad-payload rollback was
accepted: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Existing connected adoption
lifecycle regression passed 1 test with 0 ignored:
`/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Existing-Assessment update offers,
C410, full Course/Assessment milestones, and offline aggregate acceptance are not claimed.

The final append receipt runs the extended existing permanent
`revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` test, which invokes
the `append.rs` helper `assert_new_assessment_save_preserves_daughter_work`.
No separate seed-sharing test was retained. Fresh Sysadmin SQL and the unchanged permanent security
catalog passed as `ple_migrator` in `/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H`.
Broad baseline/offline aggregate acceptance remains pending, not passed.

Four Assessment-content rows are newly verified. Accepted actual-server/private bundled-main
browser proof saved/reloaded mixed Fixed Question/Pool order and exact pins, moved the Pool across
a Fixed Question, removed both kinds from current content while retaining private retired IDs and
the old Pool Revision, and reimported distinct fork identities without changing source Pool JSON.
Student direct real HTTP returned 404 without current-state change; browser error arrays were empty.
Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. Corrected SQL permits atomic
current-position swaps and retired-position reuse and excludes retired entries from workspace reads.
The existing connected adoption regression passed 1 test with 0 ignored under that SQL; two
supplemental Type projections retained unchanged pins:
`/private/tmp/ple-shared-assessment-adoption-artifacts.UmP416`. The exact **Randomize question order**
label uses the earlier accepted part 04 actual-main/HTTP checkbox receipt, not this mixed-entry
proof. No Student Work history, authentication/TLS, full Live Demo, WeBWorK delivery, C505 filename
rename, or whole-milestone completion is claimed.

Ten bounded Assessment Type purpose/capability rows are newly verified from the real Type
presentation registry, both Instructor creation entry points, and current points/scoring/Properties
capabilities. Neighboring accepted Bonus `8 / 0` and Quiz/Exam one-Attempt receipts remain distinct
runtime evidence. The broad appropriate-defaults row and cross-backend Practice immediate-answer
row remain open; this docs-only receipt does not claim complete settings persistence/enforcement,
automatic collaboration policy, learning-age inference, exam calendars, or Student delivery.

The shared underlying Assessment-model row is also verified. Canonical teaching types and
the ordinary adoption materialization path connect reusable Blueprint content to current Course
Instance Assessments; their distinct storage/lifecycle projections are intentional. Accepted fresh
PostgreSQL 17 connected adoption proof passed 1 test with 0 ignored, preserving mixed ordered
Pool/Fixed entries, nondefault teaching rules, exact Revision pins, independent daughter Pool IDs,
and unset dates. Artifact: `/private/tmp/ple-shared-assessment-adoption-artifacts.IkYuXY`.
Every Type's Student delivery and completion remain outside this architecture receipt.

HG608's distinct algorithmic-Question Pool purpose is verified by accepted fresh actual-server
and private bundled-main HTTP-proxy browser proof: explicit Instructor selection/attestation
created a two-Question reusable Pool and a distinct Assessment-owned fork selecting one Question.
Real WeBWorK response save/resume preserved exact provenance and reproduction facts; whole-Attempt
submit and fresh new-Attempt selection/issued IDs passed. Artifact:
`/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. This closes only HG608, not full Live Demo,
authentication/TLS, all-backend acceptance, or Correct answer disclosure (configured Never).

C351 closes the manual Draft-deletion row. Source evidence follows owner-scoped SQL with locked
Edit Number CAS through the PostgreSQL Store, authenticated server route, and explicit browser
confirmation. Accepted isolated PostgreSQL 17/MinIO actual-server and focused browser proof
cancelled and confirmed a delete, reloaded the list, denied collaborator, unrelated Instructor,
Student, Sysadmin, and anonymous current-ETag deletes as 404 without changing source/Edit Number,
and returned 428, 400, and 412 for missing, malformed, and stale preconditions. A published-origin
Draft deletion retained parsed Published Question lineage and Revision JSON; repeated DELETE and PUT
returned 404. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`. This bounded receipt does
not claim S3 erasure, automatic cleanup, authentication acceptance, full Live Demo browser
acceptance, or healthy backend behavior with the renderer disabled.

C831 closes the bounded native-reproduction row. The source model tags native PLE JSON as
`Static`, while renderer-backed Questions retain the paired seeded reproduction facts. Accepted
isolated PostgreSQL 17 evidence issued a shuffled-position-2 native Question with null seed and
hash, retained a numeric seed and 64-character hash only for the real WeBWorK Question, omitted
both fields from all public start/read/save/resume/restored payloads, and restored an identical
issued vector plus saved native response. The database rejected an invalid native seed through
`validate_issued_question_reproduction`. This one-time proof does not claim JavaScript support,
backend parity, or broader randomness behavior. Artifact:
`/private/tmp/ple-native-seed-artifacts.KfY7Op`.

C910 closes three bounded feedback rows. Author-managed General feedback remains immutable Question
Revision metadata; accepted actual Student HTTP and exact-main browser proof rendered it after
submission with all six disclosure timings `Never`, while response, score, correctness, answer, and
explanation remained absent and nonowners received 404. PLE does not reconstruct transient WeBWorK
feedback after the renderer stops. Source and focused domain tests establish that supplied native
Question Feedback is independent of answer disclosure; the runtime WeBWorK fixture supplied no
transient backend feedback. The broader Student workflow and opaque backend-answer gaps remain open.

Bulk Published Question shared metadata now has a bounded C368 behavior closure. The closed
`tags`/`subject`/`topic` DTO, generated 1,000-item bound, canonical-ID validation before Store
access, no-store current read, and one-transaction database command passed fresh PostgreSQL 17
SQL/API proof and independent rerun. Source review establishes once-only native `PLE authoring`/
`Pilot` tags and an empty WebWork start; explicit initial-tag SQL/API proof rejects null elements
before database/publication/object side effects and preserves an empty clear in a successor Revision.
Accepted isolated actual-server HTTP and private exact-main browser proof selected Published
Questions, replaced tags and subject, cleared subject, observed search projection, and returned
all-or-none 412 after an authorized nonowner Instructor advanced one selected Question. The browser
refreshed current metadata without automatically writing again; source, Revision, and availability
were unchanged, and anonymous/Student calls were denied. C366's thousands-Question practical
workflow and the 13k cleanup claim remain open; this three-Question proof is not screenshot-corpus
acceptance. Separately, C58 now has accepted private PostgreSQL 17 and
actual-server HTTP evidence for ordinary words, quoted phrases, minus exclusion, all five PLE
fields, active-vetted-Instructor access, anonymous and Student concealment, and `no-store`; the
bounded 69-Question fixture does not establish expert very-large-library usability or performance.
C60 now has accepted routed-component, actual-server, and private exact-main full-app evidence for
grouped Browse, full-snapshot counts, exact Browse-to-Search transfer, distinct routes, connected
exploration, and shared dense presentation at 1280 by 800. The private asset transport does not
establish deployment-gateway or WASM-runtime behavior. The 13k cleanup remains open as well. C59's
native Search tips disclosure makes the grammar discoverable without obscuring normal controls.
C57 closes the three Search-landing and return-state bullets with accepted one-time compiled-browser
evidence: idle Search did not fetch or show filters/results; entering a query showed results; and
visible return and browser Back restored query, filter, 80 rows, and virtual-list position while a
changed session returned to the empty landing. The temporary harness and screenshots were removed
after acceptance.
C61's independently accepted actual Ribbon/page proof closes the required Assessments labels
and reusable Template-design rows. Follow-up private actual-server and exact-main browser evidence
closes three Due Soon rows with authorized cross-Course results and visible state/Course/Due
emphasis. The first owned Course list retained readable Account-zone Due values in three rows at
1280px and 720px with a different browser zone; a second owned Course showed three mixed
Released/Unreleased rows and distinct Due values matched to its actual 200/`no-store` response.
This closes the bounded Assessment-list scan row. The broader spreadsheet-like collection-density
judgment, empty/error states, release workflow, WASM runtime, deployment gateway, and connected
Template delivery were not claimed.
C62's independently accepted private actual-HTTP and exact-main browser proof closes all five
Assessment-editor-shell rows: the two named editors, Question selection/add/remove/order behavior,
their distinct tasks, grouped responsive Properties, and fixed-Question point-value editing.
Question order, one instructions edit, and point values `2.5` and `1` survived actual-server
reloads. Cancel and Stay/Discard issued no unintended save, and a real stale write retained the
point draft until explicit reload/discard synchronized the latest full Assessment. This does not
claim score recalculation, Released-Assessment editing, deployment-gateway, or WASM-runtime
behavior. Broader spreadsheet-like collection-density judgment stays open under C44.
C64 and C65 close both Assessment-randomization rows. Accepted authenticated Student HTTP evidence
persisted authored and shuffled rules, issued a complete exact fixed-and-Pool vector, and retained
immutable shuffled order on resume after a current-rule edit. Accepted exact-main browser evidence
saved and reloaded the visible checkbox through actual HTTP. The Question authoring/codec/adapter
and presentation-builder source chain owns native choice order, while the closed Assessment rules
have no choice override. This establishes the ownership boundary without claiming a runtime matrix
of every native choice permutation.
Accepted actual-server Pool evidence closes the fork-independence and new-Attempt-selection rows:
an Assessment-owned fork advanced to Revision 2 with reversed exact member pins while its reusable
source remained Revision 1 in original order; Attempt 1 resumed its retained pin and nonce, while
submitted Attempt 2 received a distinct selection ID and nonce. Accepted actual-main Instructor
evidence also created a reusable Pool, imported it, reduced its count from 2 to 1, attested and
reordered its two exact Revision 1 members, and reloaded the Assessment-owned Revision 2 with its
source unchanged. The same accepted receipts establish an ordered Published-Question Pool, an
Instructor interchangeability attestation rather than automatic pedagogy evaluation, and canonical
public `AAAA-ZBBB` IDs for Questions and Pools. They do not establish ID-collision handling,
backend-independence, or broader Pool discovery.
Accepted actual-server sharing evidence also let a second vetted Instructor list and read a root and
child published Pool with exact public pins and no Course facts; it denied nonmember mutation without
change and Student/anonymous Pool reads with no-store 404. This closes only vetted-Instructor Pool
availability.
Accepted actual-main Student evidence then denied three Question Library routes without Library API
calls, while Coursework navigation reached a Released Assessment and the earlier native Attempt proof
delivered its Question content there. This closes the Student Library-access boundary only, without
claiming screenshot or interface-aesthetics acceptance.
Accepted private actual-HTTP and exact-main evidence closes C63's visible-order and direct
Search/Browse rows: numbered order, reorder, save, reload, both direct paths, browser Back, and
Stay/Discard unsaved-state handling were visibly accepted. Exact-revision metadata navigation is
wired, and the authorized private source and checksum now reach the opaque WeBWorK adapter and
hardened iframe. Private PostgreSQL 17/MinIO plus unchanged-renderer HTTP evidence returned 200
with hardened headers and concealed missing Revision, Student, and anonymous requests; exact-main
browser evidence rendered the prompt and five choices, with all five Student Work counts remaining
zero before and after in this isolated preview path. C63 stays open because the renderer JavaScript
dereferences `window.frameElement.id` when the hardened sandbox has no same-origin frame element,
before focus, popover, and parent telemetry. Do not loosen the sandbox or rewrite sibling renderer
HTML; successful hardened-embed behavior and a state-preserving return remain unverified.

This refresh closes the two C207 deadline-cap rows, three C523 due/late default rows, the narrow
C514-C516 Template rows, and the C525 completion and Attempt-limit rows.
Accepted independent PostgreSQL 17 actual-API receipts establish Course-first atomic synchronization
of the current maximum Due date, the immutable six-month Active cap and no-Due fallback, frozen
archived/deleted retention anchors, default Due-based start/save/commit rejection, accommodated
deadlines, valid `accept` and `mark_late` overrides, and expiry finalization that preserves accepted
pre-Due work. The ignored one-time proofs were removed after acceptance. Retention notification,
archive/delete processing, broad release validation, visible unanswered UI, cross-session resume,
and backend-wide behavior remain outside these closures.

This refresh closes the Bonus zero-points-possible/direct-earned row and both occurrences of the
highest-submitted-Attempt score rule. Gradebook and Student landing share the selected score while
latest-Attempt progress remains separate. The fresh C510 lifecycle receipt observed scores `12`,
`4`, `16`, and an in-progress `NULL` fourth Attempt, with both projections retaining `16 / 16`.
The configurable grade-rule model/UI/SQL boundary was removed. The focused PostgreSQL LDA test
compilation used `--features postgres,test-support`; no default-feature or full Rust gate is
claimed. This privileged fixture and simulated backend evidence does not prove HTTP, rendering,
actual backend grading, unlimited-Attempt eligibility, Course totals, or grade calculation.

This refresh closes the narrow C519--C521 release-date and valid-range rows: authorized automated
and interactive validation, the 24-hour and Course Active-limit boundary, date ordering, correction
and rerun, hard-gated release, public-save point bounds, positive-or-null whole-Assessment
Attempt/time limits, and the release-required time limit. The accepted fresh PostgreSQL 17 actual-API
receipt atomically rejected out-of-range and excess-precision point values, then saved and released
the exact maximum. The broader missing, invalid, or unreasonable-values row and the Question-validity
row remain open. The actual Properties component receipt complements source evidence; the main
integrated 7,042-pytest and 329-Node runs passed. Current follow-up evidence also built the
`project-tools` binary, passed its nine focused curriculum-content tests, and passed all 7,155
pytest tests; these offline checks do not establish connected integration behavior.

C838/C839 now close the seven bounded algorithmic-source rows. Fresh PostgreSQL 17/MinIO evidence
published all 42 canonical PGML sources (41 official biologyproblems-website sources plus HLA) as
ordinary available WeBWorK Question lineages at Revision 1 across nine topics, with 42 direct Fixed
entries and zero Pools. Exact replay made no mutation; a same-short-name conflict made no mutation.
Pilot's `validated_question_format` source boundary and four focused tests enforce explicit PG/PGML
format with its matching extension; connected binding proof independently preserved source SHA/size/
path, immutable replay, and stale refusal after an intervening metadata edit. Artifacts:
`/private/tmp/ple-fresh-genetics-artifacts.5ERV83` and
`/private/tmp/ple-pilot-format-binding-artifacts.CKzka1`. Chargaff remains unaccepted; the 76 banks
and 13,434 old generated rows remain unresolved non-published inventory. C840--C841 remain
conditional retained-database work; this receipt does not claim migration, static-row equivalence,
historical rewrite, or Pool retirement.

## Report routes

This refresh closes C512's Bonus Assignment and Quiz icon rows. The genuine bundled Free Solid
sprite uses `star` for Bonus Assignment and `circle-question` for Quiz; all five Assessment Types
now have a fixed bundled glyph and a visible label.

- [Product conflicts](product_conflicts.md) groups cross-cutting incompatibilities and points to their owning inventories.
- [Unresolved or ambiguous items](unresolved_or_ambiguous_items.md) contains only HG-unlocked product design.
- [Terminology and model changes](terminology_and_model_changes.md)
- [Authorization and FERPA changes](authorization_and_ferpa_changes.md)
- [Question and assessment changes](question_and_assessment_changes.md)
- [UI and workflow changes](ui_and_workflow_changes.md)
- [Architecture and implementation changes](architecture_and_implementation_changes.md)
- [Gap map](../../audits/human_guidance_gap_map.md) (implementation planning companion).

## Generated-evidence follow-up

Re-run the checklist consistency check after Human Guidance or checklist status changes. This summary
does not establish that the topical reports cover every owning-open record or that their mappings are
current. Runtime/browser evidence remains a separate acceptance layer and is only used where the
checklist records a valid current receipt.

Latest Student response distinction rewrite: live HG requires visually distinct current Question,
saved-response status and keyboard focus, plus response-effect labels distinguishing Save/Clear/
change from whole Coursework submission. Both rows are open; earlier navigation styling receipts
are retained as partial proof, not blanket acceptance of native response controls/actions.
