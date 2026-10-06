# Deeper Human Guidance design review

Status: deeper static review complete; implementation and runtime acceptance remain separate.

This review follows the earlier broad static audit. Its purpose is stronger causal evidence, not
more findings or a larger file count. The reference is `e67eacab4ceeafe655d5b2b7ca051b7acda7c8df`;
the worktree HEAD is `57f180c26975b54be367becb036e64767b95e8ec`, with existing uncommitted changes.
The boundary contains 515 tracked changed paths, plus surrounding producers and consumers needed
to understand their behavior. The earlier 2,654-path inventory remains a separate scope record.

Human Guidance supplies product direction and can contain ambiguous or stale wording. Optional
features need an actual use; neither the existence of a guidance sentence nor a missing checklist
mark automatically warrants implementation. Explicitly deferred work and temporary incomplete
wiring remain outside acceptance scope. PLE is pre-production; compatibility state has no special
claim to preservation. No tests, builds, services, browsers, or compliance gates are executed.

The [per-file review](human_guidance_deeper_code_review_2026_10_01_files.md) and
[CSV evidence ledger](human_guidance_deeper_code_review_2026_10_01_files.csv) account for all
515 changed paths, including two deletions. Six review areas cover schema/data access (102),
tests/gates (114), frontend workflows (83), guidance (48), backend/tooling (81), and browser
contracts/components (87). Each row records its assessment, evidence and limits; the CSV also
records review depth, surrounding files, and source/diff hashes.

All 515 captured file hashes matched at final ledger assembly, before adding this review's
changelog receipt. This protects the review boundary; it does not prove correctness. Generated
documents, historical reports, and nonbehavioral changes received explicitly narrower reads.
Reviewer cross-references that could not be resolved are marked in the affected rows and excluded
from verified trace lists. The principal findings below received separate coordinator traces;
the ledger is supporting review evidence, not 515 independent proofs of correctness.

## Designs worth preserving

- Immutable Question revision pins keep historical delivery separate from current library labels.
  Recognition-title projections can improve browsing without rewriting those pins.
- Assessment version checks and transactional Pool forks protect the target mutation and rollback.
  The missing source-version check below is a specific gap in an otherwise useful boundary.
- Classification composite foreign keys reject incompatible saved hierarchies. Keep this database
  ownership while repairing stale client intent rather than weakening persistence validation.
- Draft support fields and explicit disclosure rules have an actual product meaning. Preserve
  their separation from backend grading and immutable published revision content.
- Real-component browser harnesses, typed fake clients, deterministic grading checks, and explicit
  authorization contracts supply useful evidence. Their existence does not justify broad prose gates.

## New high-priority finding: Pool import loses the membership the Instructor reviewed

**Trigger:** an Instructor selects a Pool at member-list Edit 7. Another Instructor changes its
members before the first Instructor clicks Import. Keep the same member count to isolate the
failure: the command succeeds, but different Questions enter the Assessment.

| Boundary | Actual contract |
|---|---|
| `src/features/question_pool_picker/question_pool_picker.tsx` (historical path; retired) `selectPool`, 143-170 | Rejects a list/detail edit mismatch; the UI deliberately requires reviewed membership |
| Same file `confirm`, 174-190 | Returns the selected Pool ID, edit number, and member count |
| [Assessment page](../../../src/pages/assessment_workspace/assessment_workspace_questions_page.tsx) `choosePool`, 610-616 | Retains the selection and explicitly says Edit N is ready to import |
| Same file `importPool`, 618-644 | Validates against the reviewed count, then sends the Pool ID without the reviewed edit number |
| [Browser input](../../../src/api/assessment_pool_fork.ts), 17-24; [store input](../../../crates/learning-data-access/src/assessment_pool_fork.rs), 12-29 | Neither command has an expected source membership version |
| [SQL import](../../../schemas/base_schema/50_functions/assessment_pool_forks.sql), 10-57 | Checks the target Assessment's edit number, then forks the source Pool's current membership |

**Why existing protection is insufficient:** the Assessment version protects the target, not the
source the Instructor inspected. The Pool lock makes the copy coherent at commit time; it does
not establish that this is the reviewed member set. This is separate from mutable Pool support
metadata and does not require changing the meaning of a membership edit number.

**Smallest coherent correction:** pass `expectedSourceQuestionPoolEditNumber` from the existing
selection through the browser/server/store command, then lock and compare the source before
forking in the same transaction. A stale source leaves the Assessment unchanged and requires a new
review. Do not send raw browser member pins, copy a stale client snapshot, or add automatic retries.

**Counterevidence and limit:** the fork, entry, and ownership association are already transactional.
If the source shrinks below the requested selection count, SQL raises and rolls back; this finding
does not claim an orphan fork or partial commit. The same-count changed-members case establishes
the user-intent failure without relying on a runtime test. Two independent traces confirmed it.

## New medium finding: classification creation loses its parent context

[AuthoringClassificationLevel](../../../src/components/authoring_classification_level.tsx)
captures `parentUuid` before `createName` (41-49), but after the await it offers or selects the
returned child without checking that parent again (50-59). `offer` contains only UUID/name; its
later acceptance reads the current parent rather than the parent under which the offer was made
(67-81). The child-local `pending` state does not disable its parent's selector.

In [CourseClassificationFields](../../../src/components/course_classification_fields.tsx),
39-76, changing Discipline clears Subject/Topic/Subtopic but leaves the same child component
mounted. The Subject callback merges its argument into the current `props.value`. Therefore:

1. Start creating a Subject under Discipline A.
2. Change Discipline to B while that request is pending.
3. The A request returns; `selectCreated` sets its Subject into the current B draft.

The analogous Topic/Subtopic flow can attach an old request's result to a newly selected parent.
An existing-Subject offer can also survive a parent change and be accepted against a different
Discipline. This is a client intent/context failure, not proof that invalid relationships can be
stored: database validation and role authorization remain separate boundaries.

**Smallest coherent correction:** tie pending creation/acceptance and its offer to the originating
parent/context generation. On a parent change or disposal, invalidate that UI operation; discard
its eventual selection/offer effect while leaving any legitimately created vocabulary row alone.
Use the request-generation pattern already used by the Question picker. Avoid compensating deletes,
global form locks, or a new workflow manager for an ordinary stale UI response.

**Counterevidence:** the selector's `createResource` does key list loads by parent. That protects the
list query; it does not guard the separate `create()`/`accept()` continuations. The parent Course
form's busy flag covers its own save/create command, not these child vocabulary requests.
An independent caller/schema trace confirmed the missing guard and the containment supplied by
composite classification foreign keys in Course, Blueprint and Published Question tables.

## Earlier Pool concurrency finding: preserve the distinction between versions

The earlier stale-write finding remains valid, but its correction must respect the source contract.
[QuestionPoolPleManagedSupport](../../../crates/learning-data-access/src/question_pool_support.rs)
explicitly calls the current token a member-list Edit Number. HG also says saving the member list
advances that number, and immutable Published Question Revisions govern historical grading.

The concrete lost update is:

1. Editors A and B read membership version N and support text S.
2. A saves S1. SQL locks the Pool, compares N, writes S1, and returns N.
3. B acquires the lock, still passes N, writes S2, and silently replaces S1.

[Support SQL](../../../schemas/base_schema/50_functions/question_pool_support.sql), 106-131,
and [bulk search metadata SQL](../../../schemas/base_schema/50_functions/question_pool_search_metadata.sql),
120-175, implement this behavior. The
[update trigger](../../../schemas/base_schema/50_functions/question_pools.sql), 24-48,
explicitly separates membership changes from mutable support/search metadata changes.

The lock is useful and the membership token is meaningful. Neither protects stale replacements of
support or metadata. Establish an advancing concurrency token for the mutable state being replaced,
including provenance if it shares that edit lifecycle. Do not blindly increment the membership
number for every metadata edit, demand historical copies of search tags, or introduce a general
event/history framework merely to repair a save contract.

## Corrected finding: the claimed membership-helper privilege failure was wrong

The earlier report inferred function ownership from the beginning of each SQL file. It missed later
role changes:

- [Student landing](../../../schemas/base_schema/50_functions/student_assessment_landing.sql)
  switches to `ple_api_owner` at 328, before calls at 389 and 488.
- [Student history](../../../schemas/base_schema/50_functions/student_course_attempt_history.sql)
  switches at 156, before the call at 190.
- [Student response statistics](../../../schemas/base_schema/50_functions/student_course_response_stats.sql)
  switches at 123, before the call at 158.
- That owner has table SELECT in
  [membership grants](../../../schemas/base_schema/70_grants/course_membership.sql), 7,
  and the corresponding FORCE-RLS policy in
  [membership policies](../../../schemas/base_schema/60_policies/course_membership.sql), 21.

F9 in the earlier report and its affected ledger rows have been corrected. Do not change the helper
to SECURITY DEFINER on the basis of that finding. A granted-but-unused capability is a different,
smaller question; it does not prove a broken Student workflow.

## Interpretation and complexity decisions

**Keep optional support content when it serves the described workflow.** A reviewer treated the
word "may" as evidence that Hints and Worked Solutions lacked authorization. That conclusion does
not survive the surrounding guidance: HG 1133-1134 explicitly includes them in revision changes,
and 1149-1154 specifies their separate backend ownership and disclosure settings. Nullable content
and independent disclosure therefore have a grounded meaning. This does not authorize a generalized
support-content platform; equally, their presence alone is not overengineering.

**Keep tests that mount real production components.** A harness under `tests/support` and a fake API
are not, by themselves, a duplicate implementation. The Assessment delivery harness imports the
actual `AssessmentAttemptPage`; the Blueprint harness imports actual Course/Assessment pages.
Evaluate the assertions and replaced boundaries before recommending deletion.

**Keep small rules that have a real owner and recovery.** The explicit repository prohibition on
`__future__` imports is accompanied by a narrowly named check and correction instruction. That is
different from turning every broad UI preference into a permanent exact-pixel or source-inventory
gate. A source-reading test is not automatically unjustified.

**Treat current-state aggregates without consumers as a refinement, not a broken workflow.**
`StudentWork::collect` adds a public assembly API currently used only by model tests; real delivery
and recovery use existing store projections. A combined in-memory value cannot itself establish
an authorized, coherent database snapshot. Retain it only for a named consumer; do not invent
loaders or a second ownership layer merely to make a glossary term concrete. Ongoing wiring alone
does not establish a defect, and this does not require adding a new workflow.

**Clarify shared Pool editing without inventing ownership rules.** Provenance replacement currently
uses broad active-Instructor authority. The source establishes that behavior, but does not prove
that author-only editing is the intended product. Document the intended shared edit policy before
adding roles, mandatory audit history, or restrictive ownership checks. Its missing stale-write
protection is concrete and belongs with the mutable-state concurrency correction above.

## Earlier findings retained

The earlier [report](fresh_codebase_audit_2026_10_01.md) retains the detailed evidence for:

- Native exact-match answers longer than the Student response limit and the nonterminating
  short-answer candidate loop. Use exact match for the impossible-answer proof; normalized matching
  can have shorter equivalent responses and should not be conflated with that case.
- Shared Library statistics gated by issued observations rather than the actual contributing cohort.
  Four blanks and one answer can expose the single answer's credit. This is a disclosure-contract
  issue, not a legal FERPA determination or a claim that a named Student was identified.
- Watch materialization whose `processed_at` write has no inbox consumer; retain synchronous
  recipient snapshotting and remove the no-effect worker path rather than all background work.
- Two-command Discipline request fulfillment, collections without continuation, visible UUID
  navigation, legacy-duration repair gates, and source/pixel compliance overreach.

The deeper review is a supplement, not a claim that every earlier finding is new or that all
unmodified source received another equally deep pass. Correct findings should survive counterevidence;
incorrect ones should be withdrawn rather than protected to preserve the audit's finding count.

## Gate and maintenance refinements with concrete owners

- **Fast decoder test launches a browser:**
  [test_instructor_account_decoder.mjs](../../../tests/test_instructor_account_decoder.mjs),
  50-77, first tests a local rejected request, then spawns a named Playwright case. That case builds
  a real page harness and launches Chromium. Keep local decoder/client validation in the Node lane
  and the approval interaction in its browser lane; a missing browser should not masquerade as a
  decoder failure. This is test ownership, not scrutiny of deferred email/login content.
- **Saved-response oracle owns unrelated subjects:** its seven receipts cover saved response,
  Pool, Blueprint, FERPA and stewardship behavior. The assertions can be useful while their shared
  runner is the wrong diagnostic boundary. Split by actual behavior owner if retained; do not
  build another test framework merely to split a file.
- **Direct database checks need honest evidence labels:** `all_test.sh` does call
  `local_stack.py acceptance`. Its current dispatcher selects three database/service lanes;
  four newer Assessment E2E scripts are separate entry points. This establishes dispatch scope,
  not absence of all equivalent regression coverage or a mandatory need to add four gates.
  Label maintainer-invoked evidence accurately and avoid claiming the aggregate proves those leaf
  scenarios unless it actually selects them. Ongoing runner wiring is not an architecture defect.
- **TODO can commission duplicate work:** it still says to build personal-library routes that
  now have actual clients and pages. Separate absent implementation from missing runtime/capture
  acceptance. A generated route ledger alone is insufficient, but actual source consumers establish
  that these capabilities are no longer wholly absent.
- **Checklist inputs are active despite their archive path:** nine `hg_checklist_parts` files are
  live generator input. Clarify their owner/location, and keep concrete accepted contracts distinct
  from broad guidance. A status-bearing transcription is evidence organization, not permission to
  create a product mechanism for every bullet.

Retain deterministic grading/source-checksum tests, authorization boundaries, real-component tests
with typed fake clients, and protocol equality when equality is the actual contract. Refine or
remove permanent visual-recipe/source-inventory assertions whose only owner is a derived reading
of broad prose. Fewer tests is a design choice, not a blanket rule to discard useful evidence.
