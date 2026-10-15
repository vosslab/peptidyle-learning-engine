# BPW import deferred findings

Recorded 2026-10-14. This report preserves confirmed defects outside the BPW import scope for
later work. It is diagnostic evidence, not an import requirement or acceptance gate. The defects
remain unfixed; this report adds no implementation or tests.

## PGML renderer failure observed during import diagnostics work

The existing `localhost/pg-renderer:reviewed` image returned `flags.error_flag = 1` at seed 1
for BPW `site_docs/genetics/topic04/downloads/genetics_terminology-matching.pgml` on 2026-10-14.
`debug.pg_warn` was null; `debug.internal` reported
`Problem failed during render - no PGcore received.` Both the existing form protocol and the
lint helper's JSON request reproduced the result. A minimal valid PG source rendered successfully
in the same isolated instance, and an explicit PG warning produced nonfatal `debug.pg_warn`.

The follow-up [renderer and generator investigation](webwork_pgcore_investigation.md) established
the cause: BPQ's matching generator emits a plain-Perl apostrophe escape that fails PG
preprocessing. Both the failing image and a clean renderer HEAD build reproduce it; all required
macros load successfully. An in-memory generator change using BPQ's existing complete-string
literal helper succeeds under both images. Generator repair and regeneration are deferred;
neither repository was modified. The import diagnostics now expose the error and leave that
Draft unpublished instead of treating HTTP success as successful PG execution. Earlier full
import receipts predate this renderer validation and do not establish that every algorithmic
source renders successfully.

## Blueprint metadata history contract

The metadata history result shape, continuation key, validation, and ordering disagree. These
are related parts of one history contract issue, affecting first-page reads and subsequent pages.

### Current source evidence

- [blueprint_history.sql](../../../schemas/base_schema/50_functions/blueprint_history.sql#L9),
  lines 9-13: `list_blueprint_history` declares 11 result columns and omits `theme_id`.
- [blueprint_history.sql](../../../schemas/base_schema/50_functions/blueprint_history.sql#L59),
  lines 59-64: the metadata `RETURN QUERY` produces 12 columns, including `event.theme_id`.
  Its first column emits `blueprint_edit_number::text` as the continuation key.
- [blueprint_history.rs](../../../crates/learning-data-access/src/postgres/blueprint_history.rs#L81),
  lines 81-87: the Rust metadata decoder expects a `theme_id` column. Lines 44-48 preserve the SQL
  continuation key when another page exists.
- [history.rs](../../../crates/server/src/blueprint_course/history.rs#L141), lines 141-152:
  server validation requires metadata continuation keys to be canonical UUIDs. Lines 119-134
  reject a cursor whose key fails that validation.
- [blueprint_history.sql](../../../schemas/base_schema/50_functions/blueprint_history.sql#L54),
  lines 54-56: SQL also requires a UUID-shaped metadata continuation key. Lines 67-72 then cast
  that key to `bigint`, find the matching edit number, and filter/order by metadata event UUID.
- [blueprint_course.sql](../../../schemas/base_schema/20_tables/blueprint_course.sql#L130),
  line 130: metadata event IDs use `gen_random_uuid()`. Lines 144-146 provide the positive edit
  number and its per-Blueprint uniqueness constraint.

### Observable consequences

The first metadata page encounters a SQL result-shape mismatch: its 12 selected columns do not
match the declared 11-column result. The Rust decoder also expects the missing declared column.

The continuation mismatch is an independent defect currently masked by that first-page failure.
If the result shape alone were repaired, the emitted decimal edit number would fail the server's
UUID validation on the next request. A UUID-shaped key would pass that validation but fail SQL's
`bigint` cast. Ordering by random event UUID also does not express metadata edit chronology.

These consequences follow from the current source contracts. This report does not claim a new
database runtime reproduction or complete history acceptance.

### Provenance and future work

All of these mismatches are present in HEAD `d986ede367d8af9607f7cffdaf018d74856a2649`, as verified
with `git show HEAD:<path>` for the SQL function, server cursor validation, and Rust decoder.
The import's current history-file differences only remove private visibility/decoding; they do
not repair these defects. The server history file has no diff from HEAD.

Consider a separate bounded history task that aligns the SQL row shape with the Rust decoder
and chooses one continuation key consistent with validation and chronological ordering. Verify
the first page and continuation against the database in that task. The current BPW import keeps
its approved scope and gates.

## Live Demo Instructor display name

The existing Live Demo sign-in label identifies Elena as Rivera, while the seeded Account and
its Profile identify her as Martinez. The capture run observed `/sign-in` offering
`Assume the role of Instructor Dr. Elena Rivera`; a selector requiring `Elena Martinez` could
not choose that role.

- [composition_live_demo.rs](../../../crates/server/src/composition_live_demo.rs#L24) supplies
  `Elena Rivera` as the Live Demo display name.
- [sign_in_page.tsx](../../../src/pages/sign_in_page.tsx#L350) renders that display name in the
  role button; [live_demo_auth_model.ts](../../../src/pages/live_demo_auth_model.ts#L15) also
  describes Rivera's seeded Account.
- [prepublication_context.sql](../../../schemas/installation_data/prepublication_context.sql#L130)
  seeds the Account name as `Elena Martinez`.

The composition label and seeded Account name are both present in HEAD
`d986ede367d8af9607f7cffdaf018d74856a2649`, verified with `git show HEAD:<path>`. This is a
preexisting product inconsistency. The BPW capture correction selects the uniquely named Elena
role and retains the existing Martinez Profile assertion. Product name alignment is deferred.

## Screenshot replay and released-answer privacy profile

The existing replay's response-monitor lifetime and privacy profiles disagree with authorized
completed Native feedback. This is a preexisting capture contract mismatch exposed by the
imported Assessment's configured answer release. The capture remains failed; this finding
does not establish an unauthorized answer disclosure or completed screenshot acceptance.

### Observed response evidence

The 2026-10-14 run in `/tmp/ple_import_capture_progress.log` passed Course Progress, Due Soon,
Completed Coursework, and Response Stats at all four viewports, then rejected the laptop
Course Attempt History checkpoint for repeated protected key `questionAnswer`.

A read-only inspection of existing fictional Live Demo data at `https://localhost:8216`,
recorded in `/tmp/ple_history_privacy_read_receipt.json`, found:

- Mary's Course `CICZ85Q6JH` Attempt list and `/api/student/latest-feedback` returned `200`
  and contained no `questionAnswer` fields.
- Three existing submitted Attempts for Assessment `AE7Q8VYGP` (`Life Molecules B`) returned
  `200` from their selected `/history` routes. Each contained
  `questions[0].questionAnswer` as an array with one HTML content block. This is actual
  answer content, rather than answer-free availability metadata.
- Elena's authorized read of that Assessment returned `200` with
  `studentFeedbackReleaseRule.question_answer: "after_submit"`. The inspected Attempts
  were already submitted. The receipt records field paths, types, and timing without
  copying answer text.

### Source contract and provenance

- [privacy_profiles.ts](../../../tests/playwright/screenshot_corpus/privacy_profiles.ts#L25)
  omits `questionanswer` from both the self-only and released-feedback allowances. Lines
  155-162 exempt release-timing strings, but not actual answer content. Lines 230-235 inspect
  all response bodies accumulated since the prior checkpoint against the next profile.
- [scenarios_student_progress.ts](../../../tests/playwright/screenshot_corpus/scenarios_student_progress.ts#L266)
  seeds up to 40 submitted Attempts before the History checkpoint, visiting the completed
  review after each submission. Its History checkpoint uses `student_self`; the subsequent
  Latest Feedback checkpoint uses `student_feedback_released`.
- [assessment_attempt_history.sql](../../../schemas/base_schema/50_functions/assessment_attempt_history.sql#L82)
  reads current Assessment disclosure settings. [student_feedback_release.rs](../../../crates/domain/src/student_feedback_release.rs#L264)
  releases `after_submit` when submission exists. [history.rs](../../../crates/server/src/assessment_delivery/history.rs#L309)
  applies that decision before assigning Native Question Answer content.

HEAD `d986ede367d8af9607f7cffdaf018d74856a2649` already contains these guard allowances,
response accumulation, history seeding, checkpoint profiles, and answer-disclosure projection.
The privacy-profile and domain-feedback files have no working-tree diff. The import's history
route changes add image-access registration after disclosure; they do not change the answer
decision. The observed history bodies are consistent with the authorized post-submit policy.

A separate bounded capture task can align the replay's setup/inspection boundary and released
feedback profile with the existing disclosure contract. This review does not broaden the
guard, change product disclosure, rerun publication, or add an import acceptance requirement.

## Report boundary

This report records the confirmed history contract, Live Demo display-name, and screenshot
capture contract issues above. It introduces no speculative findings, new product states,
import behavior, or additional acceptance requirements.
