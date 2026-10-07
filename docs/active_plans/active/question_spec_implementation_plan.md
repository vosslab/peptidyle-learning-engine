# Question-spec implementation plan

Status (2026-10-07): Complete. All 29 milestones and 35 scoped TODO behaviors are accepted; see the final closeout in the execution ledger.

Saved October 5, 2026. This is the approved 29-milestone plan covering all 35 bullets under
[TODO.md](../../TODO.md#question-spec-implementation-follow-up). Track owners, status, evidence,
and reviews in [question_spec_implementation_ledger.md](../reports/question_spec_implementation_ledger.md).
Resolve routine engineering choices from repository guidance and evidence; record material unresolved
questions for Neil in [question_spec_implementation_uncertainties.md](../decisions/question_spec_implementation_uncertainties.md).
The manager and subagents complete every milestone locally using automated tests and independent
assessment. External email setup, production deployment, and human approval are outside its gates.

## Authority and outcome

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and Neil's explicit decisions define intended behavior.
Use [README.md](../../QUESTION_SPECS/README.md) for the supporting Question specifications.
Reconcile the findings in
[QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md](../reports/QUESTION_SPEC_SETTLED_DECISIONS_REVIEW_2026_10_05.md)
and [FORK_MODEL_CODE_AUDIT_2026_10_05.md](../reports/FORK_MODEL_CODE_AUDIT_2026_10_05.md).
Existing code, tests, and old plans describe implementation evidence rather than product authority.

Finish with PostgreSQL, Rust, API JSON, TypeScript, browser behavior, tests, and supporting docs aligned
with that intended model. Fresh-database and browser evidence must demonstrate the resolved behavior.

## Intended product model

- Library Object means Published Question or Question Pool; both appear in the combined Library.
- A Question Revision is a complete record. Permitted metadata corrections preserve its Revision
  Number; changes to source or grading create another complete Revision with applicable attributes.
- Question metadata belongs on the Question record. Native JSON holds content needed to display
  and grade. Preserve supplied language; language is not required Instructor input.
- Bloom dimensions are ordinary independently nullable metadata. Owner/Sysadmin correction preserves
  the Revision Number. Pending AI assignment has no deadline.
- The owning Instructor or a Sysadmin edits Published Question content and metadata. Every Instructor
  can read, use, and fork Published Questions.
- Forking creates an ordinary new object of the same type, with its own ID, owner, and lifecycle,
  plus a source/parent reference. Question forks copy license, authors, metadata, and content;
  publication begins at Revision 1 under the new ID. Parent history stays with the parent.
- A Pool is an unordered set of Question Revision Tuples plus metadata, with at most one Revision of
  any Published Question. Saving a changed tuple set advances its Edit Number; sorting is display.
- Pool Questions satisfy the settled Type, Backend, Discipline, Subject, and compatible-license rules.
  The Pool has its own searchable metadata and automatically calculated license.
- An Assessment references an existing Pool and specifies how many Questions to select. Explicit
  forking creates an ordinary reusable Pool with copied tuples/metadata, new ID and owner, Edit
  Number 1, and source Pool pointer. Saved edits affect future selections; existing Attempts keep theirs.
- Before release, save unfinished work, show specific problems, and validate release. A valid Pool
  with two Questions can save while an unreleased Assessment asks for three; release stays blocked.
  Preserve established post-issue limits and exact Student Work evidence.
- Archive is an ordinary state: read-only, outside normal discovery, preserved with existing
  references, restorable, and forkable.
- Watches cover the settled content changes and forks. Fork-and-fix is the constructive workflow for
  faulty shared content. Remove the manually written impact-notice implementation.
- Draft autosave preserves empty, incomplete, and broken work with saved status. Native JSON and
  PG/PGML share write/import, preview, test, refine, metadata, and publication workflows.
- Backends always grade and PLE stores the fraction. New Assessments enable partial credit. Current
  Assessment settings convert stored fractions into points for all Attempts and highest Attempt;
  changing settings recalculates points without Backend regrading.
- Matching awards correct pairs divided by all prompts. MULTI-FIB averages independent equal-weight
  FIB results; wrong and blank answers earn zero. FIB supports answer lists and regex.
- Native JSON Multiple Answer uses the final linear choice-count rule. ORDER averages correct-position
  and correctly ordered-pair fractions. MC/HOTSPOT remain binary and Numeric retains tolerance.
- Statistics use actual deliveries and stored credit, separately per Question Revision and Pool:
  times received, graded-response count, average credit, full-credit percentage, zero-credit percentage.
  Pool outcomes originate from that Pool, rather than every use of its present Questions.
- Search state is temporary. Combined Library results carry shared fields plus kind-specific fields.
  Pool text/metadata matching uses the Pool's own metadata.
- Sysadmins have full administrative authority. Confirmation precedes access to FERPA-sensitive
  Student data, and that access is recorded for audit.
- A Course created from a Blueprint starts with its Theme, then changes Theme independently.
- Supported imports and Blueprint assembly generate PLE identities through ordinary services/API
  and verify readback; direct SQL fixtures are insufficient workflow evidence.

## Implementation rules

Correct the base schema directly for this pre-production repository. Update producers and consumers
of shared fields and names together, regenerate contracts from their owners, and rebuild disposable
databases. Each milestone includes affected SQL, Rust, API, UI, fixtures, tests, and documentation.

Apply the approved implementation choices:

- Use `BlueprintCourseRevisionTuple` and language-native casing across layers.
- Use `/api/library-objects/search` for combined results; replace `membership` with
  `questions=all|inNoPool`, represented by `PublishedQuestionFilter`. Retain the object-kind filter.
- Assessment Pool input uses an existing Pool ID and number to select; explicit fork is separate.
- Use ordinary nullable Bloom metadata editing/concurrency; sort unclassified entries last.
- Draft Save accepts unfinished source; preview, testing, and publication validate appropriately.
- Use explicit regex mode and the existing Rust regex library; preserve accepted-answer lists and
  literal matching modes.
- Reuse ordinary import/Course-assembly operations and existing Sysadmin role/audit facilities.

Execution choice for this implementation: assign Luna 6 to all delegated workers, and put material
unresolved decisions in the linked uncertainty log for the manager's next-day review. Treat listed
milestone dependencies as final acceptance requirements. Independent implementation preparation
and focused checks may overlap after their exact contracts and required environment are ready; a
milestone cannot be accepted until its dependency evidence is complete.

M02 began after M01 coverage was reconciled and the local environment baseline was established
(239 Rust tests, 76 generated-contract Node tests, and TypeScript/bootstrap setup). The tuple naming
work does not depend on connected-database or WeBWorK acceptance. This scheduling choice does not
complete M01 or accept M02: M01 fresh-database and runtime evidence, and M02 independent reviews,
remain required before downstream acceptance.

Keep Class 2 limits as implementation details unless they cause a demonstrated content/workflow
problem. Preserve deferred AI, automatic WeBWorK Type detection, Instructor bulk edit, optional
feedback timing, answer-key regrading, NC/ND content, abandoned-Draft cleanup, and converter handoff.
Keep the candidate initial search view tentative.

Planning found `ple_private.saved_question_search` in the authoring schema, with policies and an index.
A targeted search found no Rust/UI caller. M25 traces permanent storage and removes it. This finding
is implementation evidence, correcting the earlier narrower TODO search rather than creating policy.

## Milestones

Every milestone has one implementation owner, focused behavior evidence, fresh specification review,
and then a different fresh quality reviewer. All start pending. Plan saving is a separate task.

### M01: Establish implementation coverage

Dependencies: None.

Map all 35 TODO bullets and audit findings to milestones; record baseline failures and passing behavior.
Confirm local database, object-storage, WeBWorK, and browser runners with synthetic accounts. Install
missing dependencies through existing bootstrap tools. Check: complete coverage and recorded local
runner results, without external account setup.

### M02: Align Revision Tuple names

Dependencies: M01.

Rename Blueprint Course Revision Tuple identifiers across Rust, TypeScript, JSON, decoders, tests, and
docs; regenerate owned contracts. Check: identity round trips preserve exact ID and Revision Number.

### M03: Complete Question Revision records

Dependencies: M02.

Put metadata on complete Revision records. Permitted metadata edits preserve the number; source/grading
publication creates another complete record with carried attributes. Check: metadata corrections,
publication carry-forward, and existing exact references remain correct.

### M04: Separate Native JSON metadata

Dependencies: M03.

Align authoring, publication, reads, examples, import/export callers, and language requirements with
Question-record metadata. Check: Title corrections appear in detail/search with unchanged source and
Revision; supplied language is preserved without required Instructor input.

### M05: Make Bloom ordinary metadata

Dependencies: M03.

Merge Bloom storage, saves, types, and concurrency into ordinary editing; permit independently nullable
dimensions and sort unclassified entries last. Check: one-dimension corrections with the other absent
or unchanged preserve Revision and use ordinary concurrency.

### M06: Correct content-edit permissions

Dependencies: M03, M05.

Apply owner/Sysadmin writes for content, metadata, Type, and Bloom through server, SQL, and UI.
Bulk editing remains deferred. Check: owner/Sysadmin writes succeed; another Instructor reads, uses,
and forks while editing controls and server permissions follow ownership.

### M07: Store Question fork parents

Dependencies: M04, M05, M06.

Store exact source Question Revision Tuple on ordinary Draft/Question records through publication;
copy license, authors, metadata, and content; preserve ordinary repeated-request handling.
Check: new ID/Revision 1, immediate parent for fork-of-fork, independent edits, repeated-request result.

### M08: Store Blueprint fork parents

Dependencies: M02.

Put source Blueprint Course Revision Tuple on ordinary Blueprint records; simplify fork/receipt storage
while preserving ordinary creation and repeated requests. Check: immediate source and the new
Blueprint's own normal Revision history.

### M09: Simplify Pool storage and Save

Dependencies: M05, M06.

Use unordered tuples with one Revision per Question. Remove checkbox, required attestation flag,
attester/time storage, and the unsupported persisted Assessment Pool order policy; preserve Type,
Backend, classification, license, and permission constraints. Pool member sorting is display-only
and cannot change the tuple set, random selection, or Edit Number. Keep canonical tuple ordering only
inside the random sampler for input-permutation independence. Check: ordinary Save changes tuples/
Edit Number without certification or order input; sorting affects display. M09 owns the ordinary
Owner/Sysadmin Pool Add/Remove/Save editor and its display-only row sorting. M10/M11 retain
ownership of direct Pool references and forced-copy removal; see the [M09 order correction
report](../reports/QUESTION_SPEC_M09_ORDER_CORRECTION.md).

### M10: Reference Pools directly

Dependencies: M09.

Remove Assessment-owned association, exclusivity, and forced fork; align editor/API input with direct
Pool references and each Assessment's requested number. Check: two Assessments use one Pool with
different requested numbers while the owner remains unchanged.

### M11: Preserve Pools through Course operations

Dependencies: M08, M10.

Preserve references through Blueprint persistence, adoption, copying, and updates. Explicit forks copy
all metadata including Bloom and start at Edit Number 1. Check: shared edits affect future selections,
parent/fork edits stay independent, and existing Attempts preserve selected tuples.

### M12: Validate Assessment release

Dependencies: M03, M11.

Save unfinished pre-release work, show specific Pool/Assessment problems, and validate release.
Save valid Question metadata corrections while reporting affected Pool rules. Preserve post-issue
limits, selections, and fair earned/possible point removal when an entire Pool is removed.
Check: two valid Questions/request three saves and blocks release until resolved; classification
corrections and already-issued behavior follow HG.

### M13: Align Question Archive

Dependencies: M06, M07.

Use ordinary availability for archived read-only Questions, discovery exclusion, preserved references,
restore, and fork. Check: preserved source/read-only behavior and normal restore/fork paths.

### M14: Remove written impact notices

Dependencies: M09.

Remove manually written notices from schema, SQL/grants, API/types, UI, and tests. Preserve settled
Watch events and Change Proposals. Check: Watch events still cover Revisions, forks, and changes to
the Questions in a Pool.

### M15: Autosave unfinished Drafts

Dependencies: M04, M06.

Save empty, incomplete, and broken source for both Backends. Debounce/serialize using ordinary edit
checks; Saved means latest edits are acknowledged; errors preserve work. Check: reopen unfinished
Native JSON/PG/PGML work and publish the latest saved work after content/metadata completion.

### M16: Complete Draft preview and testing

Dependencies: M15.

Use shared Draft/publication services for browser PG/PGML and existing renderer/evaluator for testing.
Check: write/import, save, preview, test, refine, and publish for Native JSON, PG, PGML; errors preserve
Draft work and Draft testing creates no Student Work.

### M17: Apply partial-credit settings

Dependencies: M03.

Always store Backend fractions; default partial credit on. Calculate points and highest Attempt from
current Assessment settings across all Attempts. Check: both toggle directions preserve fractions and
recalculate points without regrading; copied Blueprint settings retain their intended value.

### M18: Grade Matching proportionally

Dependencies: M17.

Award correct pairs/all prompts; wrong/blank pairs zero. Preserve partial saved responses and keyed
identities through shuffling. Check: three of five earns 0.6 through storage, points, and Student display.

### M19: Grade FIB and MULTI-FIB

Dependencies: M17.

Add explicit regex using the existing Rust library while retaining accepted-answer lists/literal modes.
Grade blanks independently/equally. Validate completed publication while saving broken Draft patterns.
Check: correct/wrong/blank outcomes, denominator, stored fraction, and display.

### M20: Grade Multiple Answer

Dependencies: M17.

Implement [MULTIPLE_ANSWER_SCORING_SPEC.md](../../QUESTION_SPECS/MULTIPLE_ANSWER_SCORING_SPEC.md):
`max(0, (TP - FP) / C) * min(K, C) / max(K, C)`, with K=0 scoring zero.
Check: examples, bounds, selections, storage/display; eight correct of ten with all selected earns 60%.

### M21: Grade ORDER

Dependencies: M17.

Implement [ORDER_SCORING_SPEC.md](../../QUESTION_SPECS/ORDER_SCORING_SPEC.md): average correct-position
and correctly ordered-pair fractions; preserve complete-permutation validation. Check: DABC/ABCD 25%,
reversals, permutation tables, swaps, storage, and Student display.

### M22: Count deliveries and outcomes

Dependencies: M11, M17.

Count committed deliveries once separately from submission; use exact Revision/originating Pool,
ordinary duplicate protection, and existing privacy rules. Check: unsubmitted delivery, retries,
submitted fractional response, and separate direct/Pool totals.

### M23: Display agreed statistics

Dependencies: M22.

Show times received, graded count, mean stored credit, full-credit %, and zero-credit % per Revision/
Pool; replace cross-Revision/current-member-derived reads. Check: later Pool edits preserve origin
attribution; Assessment settings/points do not change Question outcome statistics.

### M24: Complete Library Object results

Dependencies: M04, M05, M11.

Carry shared fields through Rust/API/decoder/rows; retain kind-specific fields. Apply Type matching to
both kinds, Pool-own metadata, and search/filter renames. Check: both result paths, filters/counts,
ordering, paging, and pickers.

### M25: Remove persistent search storage

Dependencies: M24.

Trace saved-search table/policies/index/callers; remove permanent prompt/result storage and stale names.
Retain ordinary parsing/round trips and finish Used in my Courses removal. Check: fresh-database
filtering/pagination with temporary search state.

### M26: Align Sysadmin access

Dependencies: M01.

Use Sysadmin role authority, confirmation before protected Student-data retrieval, and audit. Replace
Instructor-grant requirements; keep unproven tools deferred. Check: confirmed access/audit, cancellation
without disclosure, direct requests, and other roles through server-side authorization.

### M27: Copy Blueprint Themes

Dependencies: M11.

Carry starting Theme through Course creation using existing values/controls. Check: copied initial
Theme and independent later Course changes.

### M28: Verify supported import

Dependencies: M07, M08, M11, M16, M27.

Prove authenticated import/Blueprint assembly with generated IDs and readback; reuse ordinary operations
and correct bypassing loaders. Check: Native JSON, PG/PGML, assets, metadata, ownership, exact Question
references, reusable Pools, and Theme. Converter handoff remains deferred.

### M29: Verify integrated completion

Dependencies: M01-M28.

Run fresh DB, full repository checks, production-browser journeys, Live Demo, and screenshots. Reconcile
all TODO/audit findings; regenerate schema/API docs, checklists, and changelog; obtain fresh integration
review. Check: all 35 items have decisive evidence/reviews and complete integrated acceptance passes.

## Manager and subagent execution

The manager owns dependencies, shared-schema decisions, evidence, review routing, and acceptance.
Fresh subagents perform every change, correction, review, re-review, and integration task. Briefs name
owned files/behavior, authority, dependencies, checks, and handoff evidence. Preserve concurrent edits
and invite clarification, challenges, and escalation when instructions/shared boundaries are unclear.
Use the smallest capable delegate model and record capability reasons for larger models.

Parallelize independent files/contracts. Give shared schema/types/generated changes one owner at a
time. M18-M21 can run independently after the grading boundary is ready with focused module ownership;
M26 is independent after M01. Record reasons for serialization.

Resolve ordinary engineering choices from authority, existing conventions, bounded experiments, and
independent assessment. Keep deferred product choices deferred so no milestone depends on a human
answer. Automated browser checks exercise product confirmation controls.

Fresh specification review comes first. A different fresh quality reviewer then checks correctness,
maintainability, security, and evidence. Assign fixes/re-reviews to fresh agents. Failure produces a
focused correction and re-review; tests remain evidence rather than authority. Final fresh integration
review checks composition, ownership, architecture, and complete coverage.

## Verification and completion

Use durable focused behavior tests and label temporary inventories/one-time probes. Run expensive
builds after coherent changes; repeat broad checks when new changes, failures, or concerns justify them.

Before adding or retaining a permanent test, answer every item in the [permanent test checklist](../../PYTEST_STYLE.md#permanent-test-checklist). Keep implementation proof in ignored `tests/_temp/` while permanence is unclear. Before closing this plan, review every plan-specific check in `tests/_temp/`, promote only checks that earn permanent coverage under that checklist, and remove the rest. Hold connected runtime acceptance until this cleanup and the assigned source reviews are complete.

- Fresh disposable database checks cover forks, Pool tuples/Edit Numbers, release, permissions, and
  preserved Student Work.
- Grading checks cover Rust algorithms, submission/storage, points, and display; retain MC/HOTSPOT/NUM.
- Real-browser checks cover Drafts, Pools, Archive, Library search/pickers, statistics, Themes, and
  Sysadmin confirmation/access. Screenshots receive independent agent assessment.
- Existing security controls apply: server validation (ASVS 2.2.2, 2.3.3), authorization (8.2.1-8.2.3,
  8.3.1), and server audit without sensitive data/answer payloads (16.2.1, 16.2.5).

Install missing dependencies through the existing bootstrap:

```bash
source ./source_me.sh && ./devel/setup_typescript.sh
source ./source_me.sh && ./devel/setup_playwright.sh
```

Follow repository ownership rules for local services/browser runners and disposable database rebuilds.
Use existing synthetic accounts. Verify the fresh baseline and complete compliance:

```bash
source ./source_me.sh && bash tests/e2e/e2e_database_baseline.sh
source ./source_me.sh && ./launchers/all_test.sh
```

`local_stack.py acceptance` covers database baseline, installation data, and Course appearance;
production-browser journeys are a separate gate. Run affected journeys and capture affected interfaces:

```bash
source ./source_me.sh && ./devel/run_playwright_tests.sh --build
source ./source_me.sh && ./devel/capture_screenshots.sh
```

Verify local Markdown links/formatting and regenerated contracts/docs, then the living checklist and
whitespace:

```bash
source ./source_me.sh && python3 devel/human_guidance_checklist.py --diff
source ./source_me.sh && python3 devel/human_guidance_checklist.py --consistency
git diff --check
```

Record exact commands, exit statuses, decisive output, failures, and verification limits in the ledger.
Accept a milestone after outcome, focused evidence, and both independent reviews pass. The plan was accepted locally on 2026-10-07 after all 35 TODO items, audit findings, full checks, browser evidence, screenshots, and fresh integration review passed. The separate raw unfinished numeric-literal persistence question remains deferred in the uncertainty log and does not block the accepted Draft criteria. Local completion has no human approval or external SMTP dependency. Production deployment and other deferred capabilities retain their separate scopes.
