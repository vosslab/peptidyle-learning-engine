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
  - Mismatch: `crates/learning-data-access/src/course_roster.rs` `CourseRosterImportInput::validate` accepts a syntactically valid email without an institutional-domain requirement.
- [ ] **Sysadmin** accounts should require higher security than other accounts, like TOTP authentication
  - Mismatch: `schemas/base_schema/authentication.sql` provides passkey and email authentication but no Sysadmin TOTP credential or ceremony.
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
- [x] **Instructors** should have a clearly labeled, answer-free **Student** view without changing their identity.
  - Evidence (source): `src/pages/assignment_workspace/assignment_workspace_student_view_model.ts` `STUDENT_VIEW_CUE`.
  - Evidence (test): `tests/test_assignment_workspace_student_view.mjs` `Student view presentation stays answer-free and preserves live delivery facts`.

### Student role

- [x] **Students** log in only with a passkey or email code; no passwords.
  - Evidence (source): `schemas/base_schema/authentication.sql` `consume_email_authentication` and `consume_passkey_authentication`.
- [x] Students may use multiple passkeys across their devices.
  - Evidence (source): `schemas/base_schema/authentication.sql` `ple_private.passkey` non-unique `account_id` foreign key.
- [ ] An **Instructor** can reset Student login access and send a new signup code when needed.
  - Mismatch: `crates/server/src/course_roster.rs` has invitation claim and revocation routes but no Instructor Student-login reset or code-delivery route.
- [ ] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Mismatch: the repository has no implemented, auditable collection-minimization and predictable Student-data purge policy.
- [ ] Student Course data falls under FERPA; treat it as radioactive.
  - Mismatch: `crates/server/src/support_capability.rs` `read_roster` and `tests/e2e/e2e_live_demo_support_capability.sh` establish scoped support access for roster data, not repository-wide FERPA handling for Student Course data.
- [x] Student email addresses are immutable.
  - Evidence (source): `schemas/base_schema/authentication.sql` `enforce_account_authentication_email_role` rejects changed Student Authentication Email.
- [x] Student Accounts persist across Courses and semesters.
  - Evidence (source): `schemas/base_schema/accounts.sql` `ple_private.account` has no Course foreign key.
- [x] A Student Account is global and is not owned by or permanently tied to a Course Instance.
  - Evidence (source): `schemas/base_schema/course_membership.sql` `student_record` maps global `student_account_id` to a Course.
- [ ] Roster import uses institutional email to find an existing Student Account or create one when needed.
  - Mismatch: `crates/learning-data-access/src/course_roster.rs` `CourseRosterImportInput::validate` accepts syntactically valid email without requiring an institutional domain.
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
  - Evidence (source): `crates/learning-data-access/src/support_capability.rs` `IssueSupportCapabilityInput`.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `purpose` receipt assertion.
- [x] Sysadmin support access should be limited to that support task and recorded for audit.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_audit.course_roster_support_capability_event`.
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
