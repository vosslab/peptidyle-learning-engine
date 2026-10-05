# Human Guidance access compliance audit — 2026-10-04

Scope: the current `docs/HUMAN_GUIDANCE.md` rules for Instructor admission,
Instructor Profiles and images, and Sysadmin support. This is a source audit,
not a claim that the running Live Demo was exercised.

## Result

The implementation retains two current Human Guidance mismatches that affect
ordinary Instructor use. A third conflict is an already-deferred Sysadmin
support implementation that needs reconciliation only when that work resumes.

| Priority | Finding | Human Guidance | Source evidence | Result |
| --- | --- | --- | --- | --- |
| P1 | HG-ACC-01 — Instructor creation is a two-step, in-PLE vetting workflow and requires a `Verified Instructor Display Name`. It accepts neither first name, last name, nor affiliation, and the creation path records an authentication email only. | [Instructor role](../../HUMAN_GUIDANCE.md#instructor-role): vet before Account creation **outside PLE**, no approval workflow/status; Sysadmin enters email, first name, last name, and affiliation; PLE sends setup email. | [instructor_accounts_page.tsx](../../../src/pages/instructor_accounts_page.tsx#L198-L227) calls `completeInstructorIdentityVetting` before creation; [the form](../../../src/pages/instructor_accounts_page.tsx#L427-L455) asks only for email and the verified name. [accounts.sql](../../../schemas/base_schema/50_functions/accounts.sql#L335-L365) rejects creation without a vetting decision and inserts only the authentication email; [lines 370-394](../../../schemas/base_schema/50_functions/accounts.sql#L370-L394) implement the in-PLE decision. | **Mismatch.** No setup-email dispatch was found in this path; inserting a delivery address is not evidence that mail is sent. The source does **not** prove a capability tier among admitted Instructors: `current_session_account_is_instructor()` checks only the active Instructor role. |
| P1 | HG-ACC-02 — Uploaded Instructor Profile images are private to their owner. There is no cross-account Profile/image route or projection for Students viewing Question authors or Question Pool owners. | [Instructor Profile visibility](../../HUMAN_GUIDANCE.md#instructor-profile-visibility): Profiles are visible within PLE to every Account, including Students; Question-author/Pool-owner images have no separate permission mechanism. | [profile_avatar.rs](../../../crates/server/src/profile_avatar.rs#L1-L52) defines only the authenticated Account avatar surface. Its image-delivery route resolves `resolve_current_account_profile_image`; [profile_media.sql](../../../schemas/base_schema/50_functions/profile_media.sql#L379-L381) requires `avatar.account_id = current_session_account_id()` and permits only Instructor/Sysadmin sessions. The browser API calls the summary's `providedAvatarId` a static cross-account projection and says private Profile images are concealed in [instructor_account.ts](../../../src/api/instructor_account.ts#L17-L24). | **Mismatch.** The profile system supports upload and self-display, but not the required all-Account audience or Question-author/Pool-owner use. |
| Deferred | HG-ACC-03 — A Sysadmin cannot start a scoped repair alone. An Instructor must issue the capability and can revoke it; the SQL rechecks the issuer's Course membership. | [Sysadmin role](../../HUMAN_GUIDANCE.md#sysadmin-role): Sysadmin support is scoped and audited, but the Sysadmin acts under their own authority and needs no Instructor permission/grant. | [support_capability.rs](../../../crates/server/src/support_capability.rs#L1-L6) calls this `Direct-Instructor issuance`; routes issue and revoke at [lines 35-39](../../../crates/server/src/support_capability.rs#L35-L39), using the Instructor-only session check at [lines 210-227](../../../crates/server/src/support_capability.rs#L210-L227). [support_repair_capability.sql](../../../schemas/base_schema/50_functions/support_repair_capability.sql#L146-L178) rejects any non-Instructor issuer and requires Course authority; [lines 215-229](../../../schemas/base_schema/50_functions/support_repair_capability.sql#L215-L229) lets that issuer revoke it. | **Deferred implementation conflict.** Sysadmin support beyond account creation is deferred. Preserve the good parts—exact scope, audit, and no automatic Course membership—when that work is resumed. |

## Additional confirmed gaps

- HG-ACC-04 — The old Verified-Instructor *name* remains in Star and discovery code. For example,
  [question_pool_star_control.tsx](../../../src/components/question_pool_star_control.tsx#L42-L46)
  deliberately renders only verified names and forbids Profile links or avatars. This conflicts
  with the Profile visibility rule and the user’s GitHub-Star direction. This audit found gates
  that require a non-null stored verified name, but not evidence of a second capability-bearing
  Instructor role: current account creation makes that name an invariant.
- HG-ACC-05 — A general Question and Pool discussion system remains shipped. It is explicitly described as
  a Vetted-Instructor system at [library_discussion.rs](../../../crates/server/src/library_discussion.rs#L1-L4)
  and registers one shared Questions-and-Pools endpoint at
  [lines 38-45](../../../crates/server/src/library_discussion.rs#L38-L45). This is not a
  Blueprint Course Change Proposal. Human Guidance has no general discussion-thread feature,
  and the interview rejected it. Treat removal or replacement as a separate content-stewardship
  task; do not mislabel it as an accepted Change Proposal.

## Confirmed behavior that remains aligned

- The role model itself has one locked role per Account, and the Instructor Account routes are
  Sysadmin-gated. This is consistent with the account-role boundary, apart from the extra vetting
  workflow above.
- Instructor deactivation and reactivation are implemented through dedicated Sysadmin routes in
  [instructor_account.rs](../../../crates/server/src/instructor_account.rs#L42-L51), consistent
  with the relevant Human Guidance rule.
- The present repair code keeps support scope narrow and records capability events. Those are
  compatible with Human Guidance; only who may initiate/revoke support is wrong.

## Validation performed and remaining

Static inspection covered the TypeScript interface, Rust HTTP routes and data-access boundary,
and PostgreSQL procedures. The focused existing decoder/crop tests passed:

```text
node --import tsx --test tests/test_instructor_account_decoder.mjs \
  tests/test_library_discussion_decoder.mjs tests/test_profile_image_crop.mjs
9 passed
```

Those tests prove decoding and image-crop validation only. They do not exercise account setup
email delivery, cross-account Profile visibility, or a real support-repair authorization flow.
The existing Live Demo E2E tests still encode the old vetting and Instructor-issued-capability
rules, so they are evidence of the mismatch rather than current Human Guidance compliance.
