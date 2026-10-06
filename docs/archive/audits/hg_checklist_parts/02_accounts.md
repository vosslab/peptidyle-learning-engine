## Accounts and roles

### Account rules

- [ ] PLE accounts should be global across PLE and use passwordless passkeys and email authentication.
  - Evidence (source): `schemas/base_schema/20_tables/authentication.sql` `ple_private.passkey` and `ple_private.account_authentication_email`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
  - Evidence (source): `crates/server/src/auth/live_demo.rs` `live_demo_router` mounts only the closed seeded-persona selector at `/api/auth/live-demo/accounts` and deliberately does not mount an email-code ceremony or email delivery.
  - Evidence (source): `src/pages/sign_in_page.tsx` `selectSeededDemoAccount` invokes the visible seeded-role entry from the sign-in page.
  - Evidence (test): `tests/test_live_demo_transport.mjs` `direct-role requests stay same-origin, no-store, and carry only persona` protects the closed seeded-role browser transport.
  - Evidence (test): `crates/server/src/auth/live_demo.rs` `surviving_persona_issues_an_ordinary_session_with_its_account_role` protects seeded selection and the resulting ordinary session.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] The local Live Demo should not enforce a single browser origin; I want to reach it over a
  firewalled LAN or Tailscale by binding to 0.0.0.0. The local TLS certificate stays because the
  login security tests depend on it.
  - Evidence (source): `containers/Caddyfile` `https://:8080` is the one site, with `bind 0.0.0.0` and `tls internal` on demand from the local CA.
  - Evidence (source): `crates/server/src/auth/browser_boundary.rs` `production_cookie_boundary` accepts one valid request host and its matching HTTPS origin only when `accepting_request_host` is set.
  - Evidence (source): `local_stack_control/live_demo_target.py` `PLE_ACCEPT_REQUEST_BROWSER_HOST` is written as 1 beside the localhost `PLE_BROWSER_ORIGIN`.
  - Evidence (test): `crates/server/src/auth/tests.rs` `live_demo_request_host_accepts_lan_and_tailscale_origins` accepted `100.64.1.2:8443`, `studio.tailnet.ts.net:8443`, an IPv6 host, and `localhost:8443`, and exact mode still rejected the LAN host.
  - Evidence (test): `tests/test_live_demo_target.py` `test_writer_emits_fixed_production_auth_manifest` saw `PLE_ACCEPT_REQUEST_BROWSER_HOST=1` and the hostless Caddy site.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] The three major user types are **Sysadmins**, **Instructors**, and **Students**.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account.user_role` CHECK constraint.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A Potential future user roles are **Course Observers**, **Student Observers**, and **Graders**.
  - Reason: explicitly future product possibility, not current implementation behavior.
- [ ] **Students** are required to use their university or institutional (`.edu` in the USA) email accounts.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.enforce_account_authentication_email_role` rejects a Student Authentication Email whose domain is not institutional before the row is stored.
  - Evidence (source): `crates/learning-data-access/src/authentication_email.rs` `require_institutional_student_domain` is the same United States `.edu` rule used before Student account resolution.
  - Evidence (test): `crates/learning-data-access/src/authentication_email.rs` `student_authentication_email_requires_an_institutional_domain` drives `require_institutional_student_domain` for an institutional address, a public mailbox, and a lookalike suffix.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Sysadmin** accounts should require higher security than other accounts, like TOTP authentication
  - Evidence (source): `schemas/base_schema/20_tables/authentication.sql` `ple_private.sysadmin_totp_credential` stores private Sysadmin TOTP credentials alongside browser-bound expiring attestations, used counters, and bounded verification attempts; `ple_private.create_authenticated_session` rejects the stored Sysadmin role at the database generic-session boundary. `crates/server/src/auth/sysadmin_totp.rs` routes trusted Sysadmin primary outcomes to pending genuine TOTP verification before creating the ordinary Sysadmin session.
  - Evidence (runtime): `schemas/base_schema/20_tables/authentication.sql` `ple_private.sysadmin_totp_attestation` passed accepted independent SQL boundary proof (`/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H`), denying generic Sysadmin issuance while preserving ordinary Student/Instructor sessions and limited grants. Actual-server HTTP proof (`/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`) observed pending MFA with no session, protected denial, missing/wrong browser-binding and bad-code denial, one valid success, replay/expiry/counter-reuse denial, and a five-attempt lock denying a fresh unused valid counter.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [x] Every Account has exactly one User Role: **Student**, **Instructor**, or **Sysadmin**.
  - Evidence (source): `schemas/base_schema/20_tables/account.sql` `ple_private.account` stores one non-null User Role per Account; `schemas/base_schema/50_functions/accounts.sql` `reject_account_identity_change` prevents changing that role. Fresh source inspection on 2026-10-04; no new live identity-mutation test.

- [x] User Role is locked and cannot change during the lifetime of an Account.
  - Evidence (source): `schemas/base_schema/20_tables/account.sql` `ple_private.account` stores one non-null User Role per Account; `schemas/base_schema/50_functions/accounts.sql` `reject_account_identity_change` prevents changing that role. Fresh source inspection on 2026-10-04; no new live identity-mutation test.

- [x] A person who needs more than one User Role uses separate Accounts.
  - Evidence (source): `schemas/base_schema/20_tables/account.sql` `ple_private.account` stores one non-null User Role per Account; `schemas/base_schema/50_functions/accounts.sql` `reject_account_identity_change` prevents changing that role. Fresh source inspection on 2026-10-04; no new live identity-mutation test.

- [x] Instructor Accounts can be deactivated while preserving their content, Course relationships,
  and historical records.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `change_instructor_account_state` appends active/deactivated state to the same Instructor Account without changing its role or deleting content; `schemas/base_schema/50_functions/authorization.sql` `current_session_account_has_active_role` reads that current state. Fresh source inspection on 2026-10-04; live deactivate/reactivate flow not rerun.

- [x] Reactivating an Instructor Account restores access to the same Account and User Role.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `change_instructor_account_state` appends active/deactivated state to the same Instructor Account without changing its role or deleting content; `schemas/base_schema/50_functions/authorization.sql` `current_session_account_has_active_role` reads that current state. Fresh source inspection on 2026-10-04; live deactivate/reactivate flow not rerun.

### Instructor role

- [x] All **Instructors** have the same product capabilities.
  - Evidence (source): `schemas/base_schema/20_tables/account.sql` `ple_private.account` has one `user_role`, and `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_instructor` has no separate Instructor tier or status.
  - Evidence (test): `crates/server/src/instructor_account.rs` `create_instructor_account` request tests accept the same active Instructor role without a vetted or verified flag.
  - Evidence (test): `tests/e2e/e2e_live_demo_instructor_accounts.sh` `assert_created_account` verified a newly created active Instructor has the closed ordinary Instructor account projection, with no tier or status field.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_instructor` delegates to the active role predicate.

- [x] Once admitted to PLE, all Instructors are equal; there is no separate Verified Instructor role or status.
  - Evidence (source): `schemas/base_schema/20_tables/account.sql` `ple_private.account` and `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_instructor` contain no Verified Instructor role or status.
  - Evidence (test): `tests/e2e/e2e_live_demo_instructor_accounts.sh` `assert_created_account` verified the current Live Demo creation response has no verified role or status.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_instructor` delegates to the active role predicate.

- [x] A **Sysadmin** vets an Instructor before creating their Account; vetting happens outside PLE.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `create_instructor_account` accepts the Sysadmin session and account fields only; it has no in-PLE vetting receipt or approval state.
  - Evidence (test): `crates/server/src/instructor_account.rs` `create_instructor_account` request tests construct the four account fields without a vetting field.
  - Evidence (test): `tests/playwright/e2e_live_demo_instructor_accounts_browser.mjs` `Create Instructor Account` completed the current Sysadmin browser form with no in-PLE vetting or approval control.

- [x] PLE has no Instructor approval workflow or approval status.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `create_instructor_account` and `schemas/base_schema/20_tables/account.sql` expose no approval workflow or approval status.
  - Evidence (test): `tests/playwright/e2e_live_demo_instructor_accounts_browser.mjs` `Create Instructor Account` completed the current Sysadmin browser form with no approval workflow or status.

- [x] A **Sysadmin** creates an Instructor Account with an email address, first name, last name, and affiliation.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `create_instructor_account` validates normalized email, first name, last name, and affiliation in its atomic account/Profile write.
  - Evidence (test): `crates/server/src/instructor_account.rs` `create_instructor_account` request tests bind all four fields.
  - Evidence (test): `tests/playwright/e2e_live_demo_instructor_accounts_browser.mjs` `Create Instructor Account` fills email, first name, last name, and affiliation before creating an Instructor in the current Live Demo.

- [ ] PLE sends a setup email to that address so the Instructor can set up their Account.
  - Evidence (source): `crates/server/src/instructor_setup_email.rs` `ConfiguredInstructorSetupEmailDelivery` sends the fixed setup notice after account creation, and `crates/server/src/composition.rs` `instructor_setup_email_delivery` fails closed when SMTP is absent.
  - Evidence (test): `crates/server/src/auth/email_code.rs` email-code tests cover configured local capture delivery and the unavailable provider path.
  - Verification pending: External provider acceptance and inbox delivery are blocked until the human configures the email account. The unconfigured path truthfully returns `setupEmailSent: false`.

- [ ] Course membership determines which private Course records an Instructor may use.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_course_instructor`.
  - Evidence (test): `crates/domain/src/teaching_authority.rs` `foreign_course_membership_cannot_authorize_an_instructor`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Instructors** can search and browse the global **Question Library**.
  - Evidence (source): `crates/server/src/question_library.rs` `question_library_router`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Instructors** can browse the content of Public and Archived **Blueprint Courses**.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.load_blueprint_course` returns current Revision content to an Instructor who owns the course or when availability is Public or Archived.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.list_blueprint_courses` shows Public courses to Instructors and Archived courses only when archived inclusion is requested.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` loads an Archived Blueprint for another Instructor.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Instructors** log in only with a passkey or email code; no passwords.
  - Evidence (source): `crates/learning-data-access/src/authentication_ceremony.rs` `passwordless_primary_account` accepts an Instructor email code or passkey and has no password method.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.consume_email_authentication_challenge` returns only a Student or Instructor Account.
  - Evidence (test): `crates/server/src/auth/tests.rs` `passwordless_login_issues_student_and_instructor_sessions_only` issues the Instructor passkey session through `issue_passwordless_login`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Instructors** should have a clearly labeled, answer-free **Student** view without changing their identity.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage`.
  - Evidence (source): `crates/server/src/assessment_student_view.rs` `instructor_with_sessions`.
  - Evidence (test): `crates/server/src/assessment_student_view.rs` `student_view_authenticates_only_instructors_without_session_writes`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.

### Student role

- [ ] **Students** log in only with a passkey or email code; no passwords.
  - Evidence (source): `crates/server/src/auth.rs` `issue_passwordless_login` issues a Student session only after `passwordless_primary_account` accepts an email code or passkey.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.consume_passkey_authentication` returns only a Student or Instructor Account.
  - Evidence (test): `crates/learning-data-access/src/authentication_ceremony.rs` `student_and_instructor_login_accepts_only_email_code_or_passkey` drives `passwordless_primary_account`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Students may use multiple passkeys across their devices.
  - Evidence (source): `schemas/base_schema/20_tables/authentication.sql` `ple_private.passkey` non-unique `account_id` foreign key.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] An **Instructor** can reset Student login access and send a new signup code when needed.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster_access.sql` `reset_student_signup_access`.
  - Evidence (source): `crates/server/src/course_roster.rs` `reset_student_signup_access`.
  - Evidence (test): `tests/test_student_signup_reset.py` `test_signup_reset_invalidates_prior_access_and_issues_one_invitation`.
  - Evidence (test): `crates/server/src/course_roster.rs` `signup_reset_mailer_document_carries_one_fresh_code`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster.sql` `course_roster_profile` contains no duplicate Student email; ordinary roster is email-free and direct-Instructor-only.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` removes identifiable Course Student records while retaining Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 purge-preservation probe on 2026-09-15.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Student Course data falls under FERPA; treat it as radioactive.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records`.
  - Evidence (source): `schemas/base_schema/60_policies/course_retention_transitions.sql` `ple_course_retention_executor`.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_course_instructor`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Student email addresses are immutable.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.enforce_account_authentication_email_role` rejects an Authentication Email update when the Account is not an Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `account_authentication_email_role_is_enforced` runs that function before insert or update.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Student Accounts persist across Courses and semesters.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `ple_private.account` has no Course foreign key.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] A Student Account is global and is not owned by or permanently tied to a Course Instance.
  - Evidence (source): `schemas/base_schema/20_tables/course_membership.sql` `student_record` maps global `student_account_id` to a Course.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Roster import uses institutional email to find an existing Student Account or create one when needed.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `ple_private.resolve_or_create_student_account` looks up a Student Account by normalized institutional email and creates one only when that email is absent.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `ple_api.import_course_roster` calls `resolve_or_create_student_account` for each reviewed row.
  - Evidence (test): `crates/learning-data-access/src/course_roster.rs` `roster_import_uses_one_normalized_institutional_email_for_account_resolution` drives `validated_entries` so one institutional email is the account-resolution key.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Each Course Instance has its own Student Record and enrollment for the Student Account.
  - Evidence (source): `schemas/base_schema/20_tables/course_membership.sql` `student_record` unique `(course_id, student_account_id)` and `course_membership`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Student Work, Attempts, submissions, and grades are kept or deleted under the Course retention
  policy, independently of the Student Account.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records`.
  - Evidence (test): `crates/server/src/course_retention_worker.rs` `course_retention_delete_follows_the_course_not_the_account`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `revoke_course_roster_entry`.
  - Evidence (test): `tests/e2e/e2e_live_demo_roster.sh` `assert_database_evidence` keeps the Student record and Account email after access revocation and requires the revoked audit event.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Student Work and grades remain subject to the normal Course retention policy after enrollment ends.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] An **Instructor** can deactivate a Student's access to their Course.
  - Evidence (source): `crates/server/src/course_roster.rs` `revoke_course_roster_entry`.
  - Evidence (test): `tests/e2e/e2e_live_demo_roster.sh` `prove_import`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Deactivating Course access does not delete the Student Account or Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `revoke_course_roster_entry`.
  - Evidence (test): `tests/e2e/e2e_live_demo_roster.sh` `assert_database_evidence` keeps the Student record and Account email after access revocation and requires the revoked audit event.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] An **Instructor** can restore the Student's Course access later.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster_access.sql` `restore_student_course_access`.
  - Evidence (source): `crates/server/src/course_roster.rs` `restore_student_course_access`.
  - Evidence (test): `tests/test_student_signup_reset.py` `test_course_access_restore_reuses_the_student_record`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Instructors** can bulk add Students to a Course Instance through roster import.
  - Evidence (source): `crates/server/src/course_roster.rs` `import_course_roster` POST route.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] **Instructors** remove Students individually.
  - Evidence (source): `crates/server/src/course_roster.rs` `revoke_course_roster_entry` single `{roster_id}` route.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] PLE does not provide bulk Student removal from a Course Instance.
  - Evidence (source): `crates/server/src/course_roster.rs` `course_roster_router` registers only single-entry revoke.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.

### Sysadmin role

- [ ] **Sysadmins** have god powers: full administrative authority over PLE.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_has_platform_administration` grants that capability from the active Sysadmin role and does not consult Course membership.
  - Evidence (test): `tests/e2e/database_baseline_security_catalog.sql` `current_session_account_has_platform_administration` denies an ordinary Sysadmin session, then accepts platform administration and refuses Course membership for the same active Account. A disposable database executed that contract and was removed. No Live Demo stack was started.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Sysadmins vet **Instructors** and create Instructor Accounts.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `require_completed_instructor_identity_vetting`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A Sysadmins can help Instructors repair Courses, Students, and content.
  - Reason: Concrete Sysadmin support/Course-administration functionality beyond Instructor Account creation is deferred by HG. Full Sysadmin authority and audit requirements guide future implementation; HG-ACC-03 records the existing authority conflict.

- N/A Sysadmins should be reluctant to get involved in content.
  - Reason: Human constraint on content intervention; not a separate feature to implement. Broader Sysadmin work remains deferred.

- N/A The human developer, Dr. Neil Voss, is currently both a **Sysadmin** and an **Instructor**.
  - Reason: human ownership statement, not an implemented PLE behavior.
- N/A Neil uses separate Sysadmin and Instructor logins so the roles remain distinct.
  - Reason: human ownership and approval statement, not an implementation requirement.
- [ ] **Sysadmins** can access Course and Student records for administrative work.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_has_platform_administration`.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_course_instructor`.
  - Evidence (test): `tests/e2e/database_baseline_security_catalog.sql` `current_session_account_is_course_instructor` denies Course membership for an active Sysadmin session.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.

- [ ] Before accessing FERPA-sensitive Student data, a Sysadmin confirms that access is needed
  for administrative work.
  - Verification pending: Implement deliberate Sysadmin access confirmation; see docs/TODO.md.
- [ ] Sysadmin access to FERPA-sensitive Student data is recorded for audit.
  - Verification pending: Verify audit records for confirmed Sysadmin access; see docs/TODO.md.

- [ ] Sysadmins use their full administrative access to support Instructors and resolve problems.
  - Verification pending: Reconcile implementation with full Sysadmin authority; see docs/TODO.md.


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
