## Accounts and roles

### Account rules

- [x] PLE accounts should be global across PLE and use passwordless passkeys and email authentication.
  - Evidence (source): `schemas/base_schema/authentication.sql` `ple_private.passkey` and `ple_private.account_authentication_email`.
- [x] Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
  - Evidence (source): `crates/server/src/auth/live_demo.rs` `live_demo_router` mounts only the closed seeded-persona selector at `/api/auth/live-demo/accounts` and deliberately does not mount an email-code ceremony or email delivery.
  - Evidence (source): `src/pages/sign_in_page.tsx` `selectSeededDemoAccount` invokes the visible seeded-role entry from the sign-in page.
  - Evidence (test): `tests/test_live_demo_transport.mjs` `direct-role requests stay same-origin, no-store, and carry only persona` protects the closed seeded-role browser transport.
  - Evidence (test): `crates/server/src/auth/live_demo.rs` `surviving_persona_issues_an_ordinary_session_with_its_account_role` protects seeded selection and the resulting ordinary session.
  - Decision: One-time runtime proof observed five personas, a seeded POST session, and an email-start request with no delivery, 404, and no cookie; it was removed rather than retained as a permanent test. The word "yet" leaves a future Live Demo email path unlocked, and no retired URL is a permanent contract.
- [x] The three major user types are **Sysadmins**, **Instructors**, and **Students**.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account.product_role` CHECK constraint.
- N/A Potential future user roles are **Course Observers**, **Student Observers**, and **Graders**.
  - Reason: explicitly future product possibility, not current implementation behavior.
- [ ] **Students** are required to use their university or institutional (`.edu` in the USA) email accounts.
  - Verification pending: `crates/learning-data-access/src/course_roster.rs` `CourseRosterImportInput::validated_entries` rejects non-`.edu` addresses and lookalike suffixes before Account resolution. This USA roster-input boundary does not establish current runtime/global institutional-email policy or mailbox ownership across Student authentication paths; C14 remains open.
- [x] **Sysadmin** accounts should require higher security than other accounts, like TOTP authentication
  - Evidence (source): `schemas/base_schema/authentication.sql` `ple_private.sysadmin_totp_credential` stores private Sysadmin TOTP credentials alongside browser-bound expiring attestations, used counters, and bounded verification attempts; `ple_private.create_authenticated_session` rejects the stored Sysadmin role at the database generic-session boundary. `crates/server/src/auth/sysadmin_totp.rs` routes trusted Sysadmin primary outcomes to pending genuine TOTP verification before creating the ordinary Sysadmin session.
  - Evidence (runtime): `schemas/base_schema/authentication.sql` `ple_private.sysadmin_totp_attestation` passed accepted independent SQL boundary proof (`/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H`), denying generic Sysadmin issuance while preserving ordinary Student/Instructor sessions and limited grants. Actual-server HTTP proof (`/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`) observed pending MFA with no session, protected denial, missing/wrong browser-binding and bad-code denial, one valid success, replay/expiry/counter-reuse denial, and a five-attempt lock denying a fresh unused valid counter.
  - Decision: C15 closes only this higher-security row. The actual-server transport was loopback HTTP, not deployed TLS; full Live Demo authentication or broader Sysadmin authority is not claimed.
- [x] Every Account has exactly one Product Role: **Student**, **Instructor**, or **Sysadmin**.
  - Evidence (source): `schemas/base_schema/accounts.sql` `product_role text NOT NULL CHECK (product_role IN ('student', 'instructor', 'sysadmin'))`.
- [x] Product Role is locked and cannot change during the lifetime of an Account.
  - Evidence (source): `schemas/base_schema/accounts.sql` `reject_account_identity_change` trigger function.
- [x] A person who needs more than one Product Role uses separate Accounts.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account` single immutable `product_role` column.
- [ ] Instructor Accounts may be deactivated without deleting their authored content, Course relationships, or historical records.
  - Mismatch: `schemas/base_schema/accounts.sql` `change_instructor_account_state` and `tests/e2e/e2e_live_demo_instructor_accounts.sh` establish the account-state transition, but do not prove preservation across authored content, Course relationships, and historical records.
- [x] Reactivating an Instructor Account restores access to the same Account and Product Role.
  - Evidence (source): `schemas/base_schema/accounts.sql` `change_instructor_account_state` preserves `account_id` and `product_role`.
  - Evidence (test): `tests/e2e/e2e_live_demo_instructor_accounts.sh` `reactivated` scenario.

### Instructor role

- [x] All vetted **Instructors** have the same product capabilities.
  - Evidence (source): `crates/server/src/question_library.rs` `instructor_session_hash` authorizes only shared `ProductRole::Instructor`.
- [ ] A **Sysadmin** vets an Instructor's real identity before creating the Instructor Account.
  - Mismatch: `crates/server/src/instructor_account.rs` `create_instructor_account` accepts an email but records no identity-vetting decision.
- [x] Course membership determines which private Course records an Instructor may use.
  - Evidence (source): `schemas/base_schema/authorization.sql` `current_session_account_is_course_instructor`.
  - Evidence (test): `crates/domain/src/teaching_authority.rs` `foreign_course_membership_cannot_authorize_an_instructor`.
- [x] **Instructors** can search and browse the global **Question Library**.
  - Evidence (source): `crates/server/src/question_library.rs` `question_library_router`.
- [ ] **Instructors** can browse the content of Public and Archived **Blueprint Courses**.
  - Mismatch: `crates/server/src/blueprint_course.rs` authorizes current Instructor sessions but has no public-or-archived Blueprint Course browse policy.
- [x] **Instructors** log in only with a passkey or email code; no passwords.
  - Evidence (source): `schemas/base_schema/authentication.sql` `consume_email_authentication` and `consume_passkey_authentication`.
- [ ] **Instructors** should have a clearly labeled, answer-free **Student** view without changing their identity.
  - Mismatch: `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage` renders "Student view unavailable" and states that the direct Assessment workspace has no answer-free Student-view projection.

### Student role

- [x] **Students** log in only with a passkey or email code; no passwords.
  - Evidence (source): `schemas/base_schema/authentication.sql` `consume_email_authentication` and `consume_passkey_authentication`.
- [x] Students may use multiple passkeys across their devices.
  - Evidence (source): `schemas/base_schema/authentication.sql` `ple_private.passkey` non-unique `account_id` foreign key.
- [ ] An **Instructor** can reset Student login access and send a new signup code when needed.
  - Mismatch: `crates/server/src/course_roster.rs` has invitation claim and revocation routes but no Instructor Student-login reset or code-delivery route.
- [x] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Evidence (source): `schemas/base_schema/course_roster.sql` `course_roster_profile` contains no duplicate Student email; ordinary roster is email-free and direct-Instructor-only.
  - Evidence (source): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` removes identifiable Course Student records while retaining Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 purge-preservation probe on 2026-09-15.
- [ ] Student Course data falls under FERPA; treat it as radioactive.
  - Mismatch: `crates/server/src/support_capability.rs` `read_roster` and `tests/e2e/e2e_live_demo_support_capability.sh` establish scoped support access for roster data, not repository-wide FERPA handling for Student Course data.
- [x] Student email addresses are immutable.
  - Evidence (source): `schemas/base_schema/authentication.sql` `enforce_account_authentication_email_role` rejects changed Student Authentication Email.
- [x] Student Accounts persist across Courses and semesters.
  - Evidence (source): `schemas/base_schema/accounts.sql` `ple_private.account` has no Course foreign key.
- [x] A Student Account is global and is not owned by or permanently tied to a Course Instance.
  - Evidence (source): `schemas/base_schema/course_membership.sql` `student_record` maps global `student_account_id` to a Course.
- [ ] Roster import uses institutional email to find an existing Student Account or create one when needed.
  - Verification pending: `crates/learning-data-access/src/course_roster.rs` `CourseRosterImportInput::validated_entries` requires USA `.edu` addresses before Account resolution; current connected proof of institutional-email lookup/create behavior and mailbox ownership is still needed. This row remains open.
- [x] Each Course Instance has its own course-scoped Student Record and enrollment for the Student Account.
  - Evidence (source): `schemas/base_schema/course_membership.sql` `student_record` unique `(course_id, student_account_id)` and `course_membership`.
- [ ] Student Work, Attempts, submissions, and grades follow Course retention independently of the Student Account.
  - Mismatch: needs retention runtime or test evidence; this A2 source audit does not establish the required lifecycle behavior.
- [ ] Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work.
  - Mismatch: `schemas/base_schema/course_operations.sql` `ple_api.revoke_course_roster_entry` records an access-revocation event, but the audited evidence does not prove preservation of every Course record and Student Work category.
- [ ] Student Work and grades remain subject to the normal Course retention policy after enrollment ends.
  - Mismatch: needs retention runtime or test evidence; this A2 source audit does not establish the required lifecycle behavior.
- [x] An **Instructor** can deactivate a Student's access to their Course.
  - Evidence (source): `crates/server/src/course_roster.rs` `revoke_course_roster_entry`.
  - Evidence (test): `tests/e2e/e2e_live_demo_roster.sh` `prove_import`.
- [ ] Deactivating Course access does not delete the Student Account or Student Work.
  - Mismatch: `schemas/base_schema/course_operations.sql` `ple_api.revoke_course_roster_entry` establishes revocation, but no source or observed test proves non-deletion of both the Account and Student Work.
- [ ] An **Instructor** can restore the Student's Course access later.
  - Mismatch: `crates/server/src/course_roster.rs` exposes claim and revoke only; it has no Instructor restore route.
- [x] **Instructors** can bulk add Students to a Course Instance through roster import.
  - Evidence (source): `crates/server/src/course_roster.rs` `import_course_roster` POST route.
- [x] **Instructors** remove Students individually.
  - Evidence (source): `crates/server/src/course_roster.rs` `revoke_course_roster_entry` single `{roster_id}` route.
- [x] PLE does not provide bulk Student removal from a Course Instance.
  - Evidence (source): `crates/server/src/course_roster.rs` `course_roster_router` registers only single-entry revoke.

### Sysadmin role

- [ ] A **Sysadmin** has full administrative authority over PLE.
  - Mismatch: `crates/server/src/instructor_account.rs` provides Instructor-account administration, but the repository has no demonstrated full-platform Sysadmin authority surface.
- [ ] Sysadmins vet **Instructors** and create Instructor Accounts.
  - Mismatch: `crates/server/src/instructor_account.rs` proves Sysadmin-gated account creation but records no real-identity vetting decision.
- [ ] Sysadmins can help Instructors repair Courses, Students, and content.
  - Mismatch: `crates/server/src/support_capability.rs` `support_capability_router` and `tests/e2e/e2e_live_demo_support_capability.sh` establish scoped course-roster support, not repair authority for Courses, Students, and content.
- N/A The human developer, Dr. Neil Voss, is currently both a **Sysadmin** and an **Instructor**.
  - Reason: human ownership statement, not an implemented PLE behavior.
- N/A Neil uses separate Sysadmin and Instructor logins so the roles remain distinct.
  - Reason: human ownership and approval statement, not an implementation requirement.
- [ ] **Sysadmins** have full platform-administration capability but do not automatically have access to FERPA Course records.
  - Mismatch: `crates/server/src/support_capability.rs` `read_roster` supports scoped roster access, but the repository does not demonstrate full platform-administration capability.
- [x] A Sysadmin may access Course or Student records when needed to resolve a specific support problem.
  - Evidence (source): `crates/learning-data-access/src/support_capability.rs` `IssueSupportRepairCapabilityInput`.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue`.
- [x] Sysadmin support access should be limited to that support task and recorded for audit.
  - Evidence (source): `schemas/base_schema/support_repair_capability.sql` `ple_audit.support_repair_capability_event`.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue`.
- [ ] Sysadmin support does not make the Sysadmin an **Instructor** or Course member.
  - Mismatch: support-capability issuance is separate from Course membership, but the audited source and runtime evidence do not directly prove that issuance cannot create Instructor or Course-member authority.

### Future Course roles

- N/A PLE may eventually support **Course Observer**, **Student Observer**, and **Grader** roles.
  - Reason: explicitly future product possibility, not current implementation behavior.
- N/A **Course Observers** are read-only participants with access to Course content and non-FERPA aggregate information.
  - Reason: future-role design detail, not a current implemented role requirement.
- N/A **Student Observers** are read-only participants with authorized access to a particular Student's Course information.
  - Reason: future-role design detail, not a current implemented role requirement.
- N/A **Graders** are not currently needed because Assessment grading is automatic.
  - Reason: explicitly current product-scope decision, not an implementation behavior claim.
- N/A Course authorization should remain adaptable enough to add these relationships later.
  - Reason: explicitly future design flexibility, not a current implementation behavior claim.
