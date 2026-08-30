# WP-SD1-A5 current alignment review

## Scope

This independent review checks the current single-installation authority against
`docs/HUMAN_GUIDANCE.md`, the active SD1 plans, the staged principal baseline,
and the material validation surface. It is a pre-acceptance review only. It
does not accept SD1-B through SD1-G or promote any staged migration.

## Result

**REVISE.** The owner guidance and active authority documents agree on the
target, and the staged principal baseline follows that target. The compiled
PostgreSQL Store boundary is still coupled to retired curriculum contracts, so
the staged database oracle cannot currently reach its migration gate. The
current package must remain `WP-SD1-A-decisions-and-impact-contract`.

## Reviewed owner decisions

| Owner decision                                      | Authority evidence                                                     | Current result                                                                                                                                                                                                     |
| --------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| One installation and global accounts                | `HUMAN_GUIDANCE.md`, `CONTRACTS.md`, `DATABASE_AUTHORIZATION.md`       | Consistent target; the historical installation-scope source remains assigned to SD1-B through SD1-D replacement.                                                                                                   |
| Fixed Student, Instructor, or Sysadmin account role | `HUMAN_GUIDANCE.md`, `USER_ROLES.md`, `IDENTITY_CONTRACTS.md`          | `2026082902` now stores one closed immutable account/session role and was directly checked in disposable PostgreSQL 17; its project-tools oracle remains blocked by the independent stale adapter compile failure. |
| Exact course and Student FERPA authority            | `HUMAN_GUIDANCE.md`, `AUTHORIZATION_CONTRACTS.md`, `SECURITY_MODEL.md` | Consistent target; connected RLS proof remains SD1-C/D work.                                                                                                                                                       |
| Fully automated grading                             | `HUMAN_GUIDANCE.md`, `AUTHORIZATION_CONTRACTS.md`                      | The internal `ple_grader` capability is not a human manual-grading role; human Grader remains future scope.                                                                                                        |
| BlueprintCourse source and CourseInstance delivery  | `HUMAN_GUIDANCE.md`, `CONTRACTS.md`, SD1 scope register                | Consistent target; old PostgreSQL implementation has not completed its replacement.                                                                                                                                |
| Human-readable public identifiers                   | `HUMAN_GUIDANCE.md`, `DESIGN_DECISIONS.md`                             | Published `AAA-BBBB` Question IDs remain the stated public locator.                                                                                                                                                |

## Evidence

- `source source_me.sh && python3 -m pytest tests/test_guidance_doc_format.py
tests/test_vendored_headers.py tests/test_markdown_links.py` passed: 277 tests.
- `bash -n tests/e2e/e2e_sd1_staged_database.sh` passed.
- Direct inspection of `schemas/staged_migrations/2026082901_principal_baseline.sql`,
  `crates/project-tools/src/database_sd1_staging.rs`,
  `crates/acceptance-runtime/src/sd1_staged_database.rs`, and the staged owner
  shows a closed acceptance-runtime entry point, fixed staging directory, and
  database principal/default-deny ACL checks. The reviewed staged files contain
  no obsolete installation-scope, institution, or Alpha product vocabulary.
- One-time disposable PostgreSQL 17 probe applied `2026082901` and `2026082902`
  in one transaction per migration (the same `SET LOCAL ROLE` scope used by
  SQLx). It confirmed private-owner forced RLS/default-deny relations, the
  closed account/session role set, immutable account and session identity,
  irreversible revocation, and foreign-key rejection of a mismatched session
  role. This is narrow migration evidence, not the project-tools acceptance
  oracle or release acceptance.
- Fresh SD1 migrations `2026082903` through `2026082907` now provide private
  passwordless-email, WebAuthn/passkey, Instructor-approval, actor-resolution,
  and shared answer-free catalog roots. Each was applied transactionally with
  its predecessors in disposable PostgreSQL 17. `2026082908` adds append-only
  catalog lifecycle evidence and has only static/documentation evidence so far.
- `source source_me.sh && cargo test -p project-tools database::tests` did not
  compile. `learning-data-access` reports 93 errors in the PostgreSQL
  reusable-curriculum and curriculum-adoption adapters. They import retired
  `Alpha*`, old preview, and removed operation symbols from `question_model`.

## Required correction before ACCEPT

The SD1-C/D PostgreSQL curriculum owner must replace the legacy adapters as one
coordinated boundary with the current closed BlueprintCourse/CourseInstance
contract. Do not restore Alpha aliases or deleted public exports to make the
old adapter compile. The replacement must make the project-tools staged command
compile, then run the disposable staged migration oracle with fresh, no-op,
checksum, role, ACL, and cleanup evidence.

## Acceptance boundary

The staged migration is not promoted, and `schemas/migrations/` remains the
active runtime migration directory. No current result proves PostgreSQL/RLS,
protected services, browser behavior, live delivery, or final release
acceptance.
