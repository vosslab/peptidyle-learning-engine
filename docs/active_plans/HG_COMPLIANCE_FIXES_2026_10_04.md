# Human Guidance compliance fixes

The user authorized fixing the settled gaps in the
[October 4 audit](reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md).
[Human Guidance](../HUMAN_GUIDANCE.md) remains the product authority.

## Work and acceptance

- Assessment fairness: exclude removed Pools from earned and possible points for
  all Attempts; enforce post-issue edit limits and protect issued Pool members.
  Verify normal saves against submitted and in-progress Attempts in PostgreSQL.
- Scores and answers: show submitted automated scores immediately; make the
  Quiz/Exam all-completed answer rule an overridable default. Verify domain,
  database, API, and displayed behavior.
- Instructor setup: replace internal vetting with Sysadmin creation using email,
  first name, last name, and affiliation, followed by setup email. Verify local
  delivery and access boundaries.
- Profiles: expose Instructor Profiles and images to signed-in Accounts,
  including Students, and link them from supported author and Star
  representations. Verify that private account fields remain private. The
  link from a Question Pool owner's name to their Instructor Profile remains
  unfinished.
- Search: use shared result presentation with compact, list, and poster modes;
  open list items separately; confirm discarding an existing search; remove
  stored search restoration. Verify browser behavior. This does not certify a
  shared spreadsheet-style search. Source inspection confirms shared result
  displays but separate search forms and no common column-filter interface;
  completing that interface remains implementation work.
- Blueprint search: supply all named sorts and comparison metadata, preserving
  the real author and aggregate historical Student count. Verify query and UI.
- Remove the general Question/Pool forum implementation, preserving Blueprint
  Change Proposals and supported content lifecycle features.

Each independent lane has its own implementation report and narrow checks.
The manager integrates shared changes, runs aggregate checks, regenerates schema
documentation, captures UI evidence, and records remaining limits honestly.
ASVS 2.3.2/2.3.3 and 8.2.1/8.2.2 guide enforcement at trusted write and access
boundaries; HG defines the actual product permissions and workflows.

## Boundaries

Optional feedback timing, submitted-response regrading, broader Sysadmin support,
System-wide settings, and a new Question Change Proposal workflow remain deferred.
Whole standalone Question removal after issue remains tentative. No production
data migration, external email delivery, Git staging, or commit is part of this
work. Existing unrelated worktree changes are preserved.

## Status

The priority repairs and six-pass audit cleanup are implemented, with the
remaining limits listed below. Fresh PostgreSQL proof covers
submitted, active, and future Attempts after whole-Pool removal, including the
post-issue edit restrictions. Focused server and domain tests cover automatic
scores and the configurable Quiz/Exam answer condition. The pre-fix audit stays
historical; the checklist only closes requirements with fresh evidence.

Real email-provider acceptance awaits the user's email-account setup. Local
capture tests already verify browser-bound code delivery and Session creation;
provider failure preserves the earlier valid code. No external email was sent.

The user clarified that Blueprint Watches follow the GitHub repository model.
Search sorts by total notification subscribers; personal Watch/Unwatch controls
subscription. Watcher identities are not public search-result metadata. The
[interview record](decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md) distinguishes
this human direction from the implementation mapping and checked GitHub sources.

Removing forum threads preserves impact notices and their current permissions.
Pool impact-notice authorship remains unresolved; this work does not certify the
existing Sysadmin-only restriction or invent a Pool owner role.

## Evidence

- [Assessment fairness](reports/HG_FAIRNESS_FIXES_2026_10_04.md): fresh PostgreSQL
  proof for submitted, active, and future Attempts and post-issue edit restrictions.
- [Scores and answers](reports/HG_SCORE_FIXES_2026_10_04.md): current-policy answer
  timing and immediate automated scores; full Rust workspace gate passes.
- [Instructor setup](reports/HG_ACCOUNT_FIXES_2026_10_04.md) and
  [email sign-in](reports/HG_EMAIL_UI_FIXES_2026_10_04.md): local delivery capture,
  browser-bound authentication, and a fresh PostgreSQL code-consumption proof.
  Real inbox delivery awaits the user's email-account setup.
- [Public Profiles](reports/HG_PROFILE_FIXES_2026_10_04.md): Account-backed author
  and Star links, public Instructor image access, and shared simultaneous reads.
- [Search navigation](reports/HG_SEARCH_FIXES_2026_10_04.md) and
  [Blueprint comparison](reports/HG_BLUEPRINT_FIXES_2026_10_04.md): shared display
  modes and a fresh 251-result PostgreSQL sort/pagination proof.
- [Forum removal](reports/HG_DISCUSSION_REMOVAL_2026_10_04.md): unsupported general
  threads and posts removed; impact notices retain their existing boundary.

The verbatim [checklist](audits/human_guidance_implementation_checklist.md) still
contains all 1177 HG bullets. It distinguishes fresh proof from pending review;
these priority repairs do not claim that every HG requirement was re-audited.

## Integration checks

The full Rust workspace gate passes, including strict Clippy, both feature
configurations, tests, doctests, and WebAssembly. The Python suite passes with
10217 tests and nine source-size advisories after removal of the obsolete
approval harness. Schema regeneration/style and the
1177-bullet checklist diff, consistency, and evidence checks pass.
The TypeScript checks, ESLint, Prettier, and all 536 Node tests pass. Fresh
screenshot publication captured 247 images with matching manifest and receipt;
the focused screenshot corpus checks pass all 20 tests.

Live account creation/lifecycle and Question Library navigation pass. The
Question detail has no Discussion control, and the removed general discussion
endpoint returns 404. Integration checks also found and repaired missing
publisher Profile data, Profile SQL column/timestamp mismatches, and the missing
Rust decoder case for the new Quiz/Exam answer timing. The final application
rebuild and Profile/policy replay cover those last boundary repairs.

Live Quiz UI and Exam API policy acceptance pass, including the saved Instructor
override. An Instructor could upload and replace a Profile image; a signed-in
Student could view that Instructor's image. Requests without sign-in were denied,
and replaced-image cleanup passed. A Sysadmin's attempt to upload their own
Profile image returned "not found" (HTTP 404); that failure remains open. The
test for who can see the names of Instructors who Star a Question was updated to
use current account creation, but the complete test against the application and
database was not run again.

Neil subsequently clarified that Students do not get avatars. The older HG
Student gallery-avatar rules conflict with this direction and still need to be
reconciled with the implementation. The Student check above only tested viewing
an Instructor's image.
The strengthened Blueprint sort proof passed on a fresh PostgreSQL 17 database.

The [six-pass audit](reports/HG_SIX_PASS_AUDIT_2026_10_04.md) records corrected
findings and residual design/evidence limits.

One startup attempt compiled both
Rust binaries but exhausted the Podman VM disk while saving the installer image.
The repository image cleanup removed unused, untagged build cache, reducing image
storage from 27.3 GB to 10.0 GB; tagged images and stored data were preserved.
The retry succeeded. A later fresh start also verified the bundled publisher's
Profile repair from installation SQL; it reused the Rust build and finished in
97.5 seconds. The running demo is healthy at `https://localhost:8263`.

The existing demo picker labels its Instructor Elena Rivera, while the stored
sample Account is Elena Martinez. Public Profiles and author projections should
use the stored Account name. Aligning those sample labels is a low-impact follow-up;
this mismatch is separate from Account ownership and Profile access.
