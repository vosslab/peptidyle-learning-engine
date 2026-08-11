# Instructor-to-student walkthrough plan review

## Scope and verdict

Independently reviewed the corrected walkthrough plan, its companion evidence,
`HUMAN_GUIDANCE.md`, implementation status, release-plan references, current
course/roster routes, local-identity composition, and current persistence
contracts. **CHANGES REQUIRED before M9 dispatch.** The teaching-loop charter
is now faithful to owner guidance and removes email/canonical onboarding from
walkthrough acceptance, but WP-I2 needs one explicit, source-proven persistence
contract and the companion evidence has one stale arrangement statement.

## Confirmed alignment

- The binding sequence is now visible instructor course creation, local active
  student addition, corpus-backed assignment construction, keyboard student
  take/retry/repeat, and instructor gradebook proof. This matches
  `HUMAN_GUIDANCE.md`.
- Email, SMTP, mailbox, invitation, passkey, and canonical-account work are
  non-goals, not report rows or blockers. WP-RC8 remains separate release work.
- The plan preserves the important boundary: only private native-corpus
  publication is an API arrangement; course, roster, assignment, student work,
  repeat, and score observation are browser journeys.
- The visible course route and assignment-create API already exist; the planned
  browser client and UI gaps are truthful. The local identity composition is
  selected only by the exact development flag, and production remains separate.
- Milestones, one-owner packages, dependencies, review boundaries, maximum
  parallelism, report semantics, and two-run close-out gates are execution-ready
  once the findings below are repaired.

## Findings

### High - WP-I2 must require a `local_development` roster source migration

- **Location:** `docs/active_plans/peptidyle-walkthrough-plan.md:311-333`,
  `:527-533`, and `:634-640`.
- **Evidence:** The current `course_roster_member.source` constraint permits
  only `invitation` and `legacy`; `legacy` intentionally exists for old
  memberships, while `invitation` requires the email/roster fields that this
  charter excludes. The plan requires a truthful local-development source, so
  neither current value is semantically valid. The existing domain roster
  projection also does not expose a source enum, and the local identity file has
  no configured learner alias field.
- **Fix:** Make this an explicit WP-I2 deliverable rather than an open choice:
  add a forward migration admitting `local_development`, a narrow Store command
  and Memory/PostgreSQL conformance implementation that persists it, and an
  internal source type or equivalent closed representation. Add an exact,
  unique server-side local learner alias to the local identity configuration;
  the browser request may supply only that alias, never an ID, credential,
  tenant, display name, or role. Preserve `legacy` for legacy reconciliation
  rather than reusing it for the new path.
- **Retest:** Memory and PostgreSQL prove the manager-only, idempotent atomic
  active-member/enrollment result; unknown, instructor, foreign-tenant, and
  production requests fail closed; a PostgreSQL migration test proves the new
  source and its null email/roster-ID shape are valid.

### Medium - companion evidence still says assignments are API-created

- **Location:**
  `docs/active_plans/decisions/peptidyle_walkthrough_evidence.md:40-43`.
- **Evidence:** Under the current observed snapshot, it says the walkthrough
  must create Mastery and Exam assignments through supported APIs. The binding
  plan and human guidance instead require visible instructor assignment
  construction and exclude the Exam contrast from the corrected core.
- **Fix:** Move that statement into explicitly labeled superseded historical
  assumptions, or rewrite it to say that only retry-corpus publication remains
  an API arrangement and the instructor creates the one core Mastery assignment
  visibly. Keep the old API-arranged Mastery/Exam evidence as historical only.
- **Retest:** Search active walkthrough plan and decision documents for
  `api-mastery-assignment`, `api-exam-assignment`, and equivalent wording; any
  hit must be explicitly historical, not a current task or acceptance rule.

## Validation

- `source source_me.sh && python3 tests/check_ascii_compliance.py -i <each reviewed plan, companion, guidance, and status document>` - passed.
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q` - 136 passed.
- `git diff --check -- <reviewed documentation>` - passed.
- Line counts: walkthrough plan 653, evidence companion 88, human guidance
  375, and implementation status 450; all are below the repository limit.

## Acceptance condition

After WP-C0 records the source migration/alias contract and repairs the stale
companion statement, this plan is ready to dispatch M9's independent course,
roster, and assignment packages. No email or canonical-onboarding prerequisite
should be reintroduced.

## Re-review - 2026-08-11

### Verdict

**ACCEPTED.** The corrected plan is ready for the M9 pilot walkthrough work.
It keeps the intended visible instructor-to-student teaching loop separate from
production email and canonical-onboarding acceptance.

### Resolved findings

- **WP-I2 persistence and input boundary:** WP-I2 now owns the reserved
  `2026080913_local_development_roster.sql` migration, a closed
  `local_development` source, the corresponding narrow Store command, and
  Memory/PostgreSQL conformance. Its exact bounded unique server-side ASCII
  alias contract permits browser input of only the configured learner alias;
  tenant, user, display name, and roles remain server-derived. The migration
  policy preserves existing `legacy` and `invitation` behavior and permits the
  local source's null email/roster-ID shape. `DATABASE_STRUCTURE`, the release
  plan, and implementation status reserve the same migration without claiming
  it satisfies WP-RC8.
- **Visible assignment construction:** The evidence companion now identifies
  API-arranged Mastery/Exam assignments as historical schema-v1 evidence. The
  current core only arranges private retry-corpus publication through supported
  APIs; the instructor visibly creates the one Mastery assignment from that
  corpus. It does not reintroduce the Exam contrast.

### Re-review validation

- ASCII compliance passed for the plan, companion, human guidance, database
  structure, release plan, implementation status, and this audit.
- Markdown links: `136 passed`.
- `git diff --check -- <reviewed documentation>` passed.
- The plan has 653 lines and the companion 88, both below 1,000 lines.
