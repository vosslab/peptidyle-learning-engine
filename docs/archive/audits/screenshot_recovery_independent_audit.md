# Independent screenshot recovery audit

Date: 2026-09-21

Status: read-only independent review of the active working tree. Six fresh passes completed
(Plan, Test, Style, Documentation, Legacy, and Comment). The first Style pass ended when its
workspace ran out of credits; a fresh replacement completed the required pass. The Student
answer-state expansion arrived while the first five passes were running, so it received a static
source addendum and the replacement Style pass, not a second six-pass review. This is not test or
screenshot-replay evidence.

## Scope

The coding manager's stated order is:

1. Finish the `plpgsql_check` cleanup.
2. Make `./launchers/all_test.sh` green.
3. Build the Live Demo with `./launchers/run_live_demo.sh`.
4. Publish refreshed Instructor and Student screenshots, including every current Student Question
   format in the relevant views.

Sysadmin screenshots are deferred and are not required acceptance evidence. The repository's
current capture front door is `./devel/capture_screenshots.sh`.

The audit inspected the current cross-layer diff: PostgreSQL schema functions and policies, Rust
PostgreSQL adapters and tests, Live Demo seed and E2E scripts, screenshot scenarios and
publication, captures, and documentation. It used
[docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md),
[docs/TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md),
[docs/DATABASE_STYLE.md](../../DATABASE_STYLE.md), and the repository and test style guides.
`git diff --check` passed. No audit-specific test, Live Demo, capture replay, or PL/pgSQL rerun
was performed.

## Findings

### High: Published Question revisions retain ambiguous generic identity names

[docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) distinguishes Draft Questions, Published
Questions, Question Revisions, Pools, and Question Image Assets, and requires distinct concepts
to use distinct clear names. The current terminology contract still defines
`QuestionRevisionTuple { questionId, revisionNumber }`, while a Question Revision is specifically
the immutable revision of a Published Question. The repair makes some SQL output more precise as
`published_question_id`, but Rust, JSON, generated TypeScript, and the terminology contract retain
the generic `QuestionId` and `QuestionRevisionTuple` names.

- Evidence: [docs/HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md):93-101,148-159;
  [docs/TERMINOLOGY_CONTRACT.md](../../TERMINOLOGY_CONTRACT.md):35-38,238-253,772-791; and
  [assessment_attempt.rs](../../../crates/learning-data-access/src/postgres/assessment_attempt.rs):4-7.
- Impact: boundary code must infer which class of Question owns an identity. The mixed SQL versus
  Rust/JSON vocabulary invites the same stale-column and wrong-tuple errors this repair batch had
  to correct.
- Smallest durable correction: settle the terminology contract first, then make one direct
  pre-production rename without compatibility aliases. For example, use
  `PublishedQuestionId`, `PublishedQuestionRevisionTuple`, `published_question_id`,
  `publishedQuestionId`, and `publishedQuestionRevisionTuple` consistently at every boundary.

### High: The PL/pgSQL receipt is not a receipt for the final source

The diagnostic calls its result a 2026-09-21 current-source rerun, yet reports two trigger errors
from a `TG_TABLE_NAME = 'course_instance'` branch. The current schema no longer contains that
branch; it now always derives the Course Instance through `NEW.course_membership_id`. The
changelog also says the unreachable branch was removed.

- Evidence: [PLPGSQL_CHECK_DIAGNOSTIC.md](../../PLPGSQL_CHECK_DIAGNOSTIC.md):216-230;
  [course_membership.sql](../../../schemas/base_schema/50_functions/course_membership.sql):64-84;
  and [docs/CHANGELOG.md](../../CHANGELOG.md):21-24.
- Impact: the stated first acceptance milestone cannot rely on the receipt to establish whether
  the final source is clean.
- Smallest durable correction: run the documented disposable checker against the final source and
  replace the trigger result and digest. Until then, label the two-error result as pre-removal
  evidence rather than a current-source cleanup result.

### High: The security catalog now bypasses the public-ID reservation boundary without checking its substitute controls

The E2E catalog excludes `ple_private.public_id_reservation` from its table-wide FORCE RLS
assertion because the table ACL and an immutable trigger supposedly protect it. The same test adds
no assertions for either control. A later grant to an application role or loss of the trigger
would therefore pass this permanent security catalog while weakening the global public-ID
allocation registry.

- Evidence: [database_baseline_security_catalog.sql](../../../tests/e2e/database_baseline_security_catalog.sql):145-159.
- Impact: the test no longer protects the security boundary it says replaces FORCE RLS.
- Smallest durable correction: retain the exemption only with exact assertions that `PUBLIC` and
  untrusted application roles lack table privileges and that the named immutable no-update/
  no-delete trigger exists and is enabled. This is a permanent security invariant, so the focused
  catalog assertions earn their maintenance cost.

### High: Presentation child keys claim to be binding IDs but are actually Question Attempt IDs

The response-item and image-rendition tables name their foreign-key columns
`question_attempt_presentation_*_binding_id`, but both reference a parent's
`question_attempt_id`. The changed presentation readers correctly compare the columns to
`question_attempt.question_attempt_id`; the names nevertheless imply a distinct binding identity
that the schema does not have.

- Evidence: [assessment_attempt.sql](../../../schemas/base_schema/20_tables/assessment_attempt.sql):288-324;
  [assessment_attempt_presentation.sql](../../../schemas/base_schema/50_functions/assessment_attempt_presentation.sql):213-217,525-538,595-611.
- Impact: a future maintainer can reasonably look for or create a nonexistent binding ID. This
  violates the Human Guidance distinct-concept naming rule and hides a shared-key relationship.
- Smallest durable correction: in pre-production, rename both columns to `question_attempt_id`
  throughout the schema, SQL readers, Rust, and tests. A real binding ID is unnecessary unless a
  demonstrated workflow needs an independent binding identity.

### High: Expanded type captures dropped HOTSPOT interaction, reload, and submission proof

The all-format Student scenario previously called `exerciseHotspot` after its answer-free
captures. That helper proved both pointer and keyboard selection on the uploaded image region,
saved-response reload, submission, and the released grading outcome. The expanded capture loop
removes its only import and call, while its generic HOTSPOT answer path merely checks a form input
and saves. `exerciseHotspot` is now exported but has no caller.

- Evidence: [scenarios_student_types.ts](../../../tests/playwright/screenshot_corpus/scenarios_student_types.ts):1-12,482-490,493-546;
  [hotspot_workflow.ts](../../../tests/playwright/screenshot_corpus/hotspot_workflow.ts):163-224.
- Impact: a screenshot expansion for the most interaction-specific Question format silently
  removes the only durable browser proof that the delivered image target works for mouse and
  keyboard users, persists, submits, and grades. Answered screenshots alone cannot prove those
  behaviors.
- Smallest durable correction: restore the uncaptured `exerciseHotspot` pointer and keyboard
  checks after the visual captures, or replace them with an equally direct visible interaction,
  reload, submission, and outcome proof. This restores existing meaningful coverage; it does not
  require a new permanent test.

### High: The repair batch has no approved causal map for its cross-layer scope

The existing [screenshot_repair_code_audit.md](screenshot_repair_code_audit.md) is a historical
read-only audit, not a repair plan. The present work spans schema functions and policies, Rust
adapters, E2E fixtures, Live Demo seeding, screenshot publication, and documentation. The four
outcome goals do not explain each boundary's reproduced failure, required behavior, or what
acceptance evidence proves the direct fix.

- Evidence: [screenshot_repair_code_audit.md](screenshot_repair_code_audit.md):5-20,154-158;
  current `git diff --name-only`.
- Impact: unrelated diagnostic cleanups and required root-cause repairs are indistinguishable at
  review, so future changes can retain accidental workarounds or reverse a necessary repair.
- Smallest durable correction: record a short approved repair plan before further acceptance work.
  Map every changed boundary to the reproduced cause, direct correction, required evidence, and
  explicit non-goals; keep generated artifacts separate from source changes.

### Medium: Screenshot replay permits its manifest to be absent

The replay writer allows `current_capture_manifest.json` as metadata but passes an empty required
metadata list to `inspectCorpus`. It can therefore produce a receipt and atlas from a staging
directory without the manifest whose digest the receipt claims to bind.

- Evidence: [publication.ts](../../../tests/playwright/screenshot_corpus/publication.ts):174-179,375-400.
- Impact: replay artifact provenance is weaker than active-corpus verification, even though the
  replay creates publication-ready metadata.
- Smallest durable correction: require `current_capture_manifest.json` in the replay staging root.
  Add one focused Node test that rejects a staging root missing that file; it protects a real
  provenance boundary and satisfies the permanent-test checklist.

### Medium: Deferred Sysadmin captures silently bypass corpus duplicate-byte integrity

The publication code retains Sysadmin paths in the manifest and receipt but skips them during the
global duplicate-byte check. The exception is justified only by their deferred status, leaving a
corpus that appears complete while stale or duplicate Sysadmin artifacts silently pass integrity
verification.

- Evidence: [publication.ts](../../../tests/playwright/screenshot_corpus/publication.ts):175-199.
- Impact: the deferred lane has an implicit and weaker integrity contract rather than an explicit
  historical or acceptance boundary. A future maintainer cannot tell from the manifest and receipt
  which entries have complete integrity evidence.
- Smallest durable correction: exclude Sysadmin artifacts from the required current corpus and
  receipt while they are deferred, or keep them present and apply the same duplicate-byte check.
  The former matches the stated acceptance scope.

### Medium: E2E stack cleanup hides controller failures and treats receipt-file absence as cleanup proof

The installation E2E retries `run_live_demo.sh stop` eight times while suppressing all output.
After failure it accepts absent local receipt files as proof that the fixed-suite lease released.
Receipt absence is not authoritative evidence that labelled containers and networks are gone, and
the suppressed failure removes the diagnostic that tells a maintainer why cleanup failed.

- Evidence: [e2e_installation_data.sh](../../../tests/e2e/e2e_installation_data.sh):30-45.
- Impact: a failed cleanup can appear successful, leaving the next Live Demo lane to fail with an
  unrelated ownership or port symptom.
- Smallest durable correction: preserve the controller's failure output and use its owned-resource
  check. If retries have a demonstrated transient cause, record it and verify controller/resource
  state rather than the absence of state files.

### Medium: A five-second publication click timeout converts slow success into capture failure

The Student type-capture preparation gives `Confirm and publish` a five-second action timeout,
then already waits for the visible `Published` heading. The heading is the meaningful user-facing
readiness condition; the shorter action timeout is an unmeasured immediate-failure workaround.

- Evidence: [scenarios_student_types.ts](../../../tests/playwright/screenshot_corpus/scenarios_student_types.ts):270-272.
- Impact: a slow but healthy Live Demo can fail before the semantic readiness check runs.
- Smallest durable correction: remove the timeout. If an actual actionability race remains,
wait for the specific enabled or visible state that proves it is ready.

### Medium: A document labelled current canonical evidence still points to removed screenshot names

The UI density audit calls itself a review of the current canonical corpus, but lists old paths
such as `instructor/question_drafts.png`, `instructor/assignment_release_draft.png`, and
`student/course_in_progress_laptop.png`. The current manifest and atlas use the renamed paths.

- Evidence: [ui_density_and_layout_audit.md](ui_density_and_layout_audit.md):3-11,79-95,262-297;
  [docs/SCREENSHOT_ATLAS.md](../../SCREENSHOT_ATLAS.md):180-191.
- Impact: a document presented as current evidence has dead links and may drive UI decisions from
  unreviewable screenshots.
- Smallest durable correction: mark the audit historical, or deliberately refresh its evidence and
  findings against the renamed current corpus. Do not mechanically rename paths without confirming
  that each conclusion still applies.

### Medium: The Human Guidance compliance report links Student invitations outside their viewport folder

The capture rename moved the two Invitation images into the Student laptop folder, and the
manifest, receipt, and atlas agree on those paths. The compliance report instead links both at the
Student root, so the repository Markdown-link gate fails.

- Evidence: [COMPLIANCE_SUMMARY.md](../reports/human_guidance_compliance/COMPLIANCE_SUMMARY.md):369-370;
  [current_capture_manifest.json](../../screenshots/current_capture_manifest.json):713-739; and
  [SCREENSHOT_ATLAS.md](../../SCREENSHOT_ATLAS.md):143.
- Impact: `tests/test_markdown_links.py` reports 307 passing files and one failure, preventing
  the documentation lane from establishing link integrity.
- Smallest durable correction: point both links at `student/laptop/invitation_detail.png` and
  `student/laptop/invitation_index.png`. This is a direct documentation repair; no new test is
  needed because the existing repository-wide link check already protects it.

## Student coverage and residual risk

Static inspection supports the new Student priority. The corpus contains all eight current native
formats (MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, HOTSPOT) plus WeBWorK as both unanswered and
saved-answer views on laptop and phone: 36 answering captures. MC additionally appears at tablet
and square viewports. Selected, resumed, navigation, submitted, and history captures supplement
the per-type response views. Login captures are public-role artifacts and are not standing in for
Student answering coverage.

This does not prove freshness or rendering quality because the audit did not replay or inspect the
new PNGs. It also does not establish every Question format at tablet and square, or in every
lifecycle state. Those expansions should follow a product-owner decision rather than a speculative
capture matrix.

## Review method and evidence limits

- Completed independent passes: Plan, Test, Style, Documentation, Legacy, and Comment. The Style
  pass required one fresh replacement after the first reviewer exhausted its workspace credit.
- Late source addendum: the Student saved-answer and viewport expansion was statically inspected
  and received the fresh replacement Style pass. The removed HOTSPOT interaction proof is reported
  above; the late expansion did not receive a second complete six-pass review.
- Targeted artifact checks: `git diff --check` and the ASCII check of this report passed.
  `source source_me.sh && python3 -m pytest tests/test_markdown_links.py` found the two broken
  Invitation links above (307 passed, 1 failed).
- No new permanent test is recommended except the focused replay-manifest provenance assertion
  described above.
- The working tree was active during review. Revalidate each cited path before applying a repair.
