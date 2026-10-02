# Human Guidance implementation compliance checklist

Latest evidence date: 2026-09-16

Source: `docs/HUMAN_GUIDANCE.md`. Human Guidance remains authoritative. This file records
current implementation status only. `Deferred product behavior`, `How to use this guidance`,
and `Product vocabulary and glossary` remain authoritative, but are not checklist items.

- [x] Verified: implemented behavior matches the bullet. Evidence follows.
- [ ] Unverified: Mismatch identifies missing or incorrect behavior; Verification pending identifies implemented behavior awaiting named proof.
- N/A: audited and not an implementation requirement. Reason follows.

# Human guidance

## Development principles
### Agent working principles
- [x] The stack takes a long time to rebuild. Use `./launchers/run_fast_checks.sh` for faster
  interface checks.
  - Evidence (source): `launchers/run_fast_checks.sh` `run_fast_checks` runs the offline aggregate gates, including schema style, Rust, and TypeScript, without starting the live stack.

- N/A Read and learn the core principles in docs/REPO_STYLE.md
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Apply the Keep It Simple, Stupid (KISS) philosophy aggressively.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Prefer the smallest coherent design that meets actual requirements and known failure modes.
  - Reason: Not separately testable or independently closable product behavior; this remains a binding design and implementation-review constraint.

- N/A Complexity must earn its place.
  - Reason: Not separately testable or independently closable product behavior; this remains a binding design and implementation-review constraint.

- N/A Time should be used efficiently. Agents and tokens are cheap; wall time is not.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Hard work should be broken into small, independently completable tasks.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Write plans in plain, concrete language. Use technical terms when they add precision.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Prioritize positive prompting. Phrase instructions as concrete actions such as "Do X" or "Use Y".
  - Reason: audited agent workflow guidance; it makes no claim about implemented PLE behavior.

- N/A Name only the tools and responsibilities needed for the assigned task. Positive prompting plus
  omission keeps agent instructions focused on the intended actions.
  - Reason: audited agent workflow guidance; it makes no claim about implemented PLE behavior.

- N/A Small LMs may interpret negative instructions as actions to perform. State the desired behavior
  directly, including when assigning responsibilities to agents.
  - Reason: audited agent workflow guidance; it makes no claim about implemented PLE behavior.

- [x] Python code uses the current interpreter's defaults; `from __future__ import ...` belongs nowhere
  in this repo. `tests/test_no_future_imports.py` enforces it.
  - Evidence (test): `tests/test_no_future_imports.py` `test_python_sources_do_not_import_from_future` fails when a tracked Python file imports from __future__.
  - Evidence (source): `devel/change_scope.py` `decide` and `devel/capture_screenshots.py` `print_step` are ordinary Python 3.12 modules with no __future__ import.

- N/A Long local operations must be robust and informative: keep going through imperfect state where
  useful, recover gracefully, and tell me what is happening while I wait. I am impatient.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Classify one-time checks separately from permanent tests.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Finish the obvious. Continue while the next safe step is defined by the plan, implied by the current task.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Robust means the software continues to function despite imperfect inputs, data, state, or behavior.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Treat tests as liabilities as well as protection. Keep only requirements and gates grounded in actual needs.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Plans should be finishable by the manager and subagents without additional human interaction.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Prefer more small, independently verifiable milestones over a few large milestones.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Fix the design that causes a problem rather than adding a workaround for its symptom.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Prefer durable long-term fixes when the additional cost is justified.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Prefer adaptable boundaries and simple domain concepts over speculative edge-case machinery.
  - Reason: not separately testable or individually closable product behavior; it remains a binding review constraint on implementation choices.

- N/A Add product states, workflows, background processing, and recovery mechanisms only for a
  demonstrated product or Question Backend need.
  - Reason: agent instruction, not implemented PLE product behavior.

- N/A Stay focused on the requested work. Complete the required work and avoid adding unplanned functionality.
  - Reason: agent instruction, not implemented PLE product behavior.

### Codebase development rules
- [x] Language-native casing is the right system: SQL stays account_id, Rust/TS types stay AccountId, JSON stays accountId.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account_id` is the PostgreSQL Account column.
  - Evidence (source): `crates/question_model/src/public_route.rs` `AccountId` is the Rust and generated TypeScript Account type.
  - Evidence (source): `src/api/course_instance.ts` `accountId` is the JSON Account field.

- [x] Use `source ./source_me.sh && ./launchers/run_fast_checks.sh` for a quicker compliance check
  - Evidence (source): `launchers/run_fast_checks.sh` `run_fast_checks` is the offline compliance subset, and `source_me.sh` `source_me` prepares the command environment.

- [x] Use `source ./source_me.sh && ./launchers/all_test.sh` for a complete compliance check
  - Evidence (source): `launchers/all_test.sh` `check_schema_style` runs schema style, Rust, the codebase check, pytest, and local-stack acceptance after `source_me.sh` `source_me`.

- [x] Use `source ./source_me.sh && ./devel/capture_screenshots.sh` for UI work to capture fresh screenshots
  - Evidence (source): `devel/capture_screenshots.sh` `capture_screenshots` runs the screenshot driver after `source_me.sh` `source_me`.

- [x] Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
  - Evidence (test): `tests/test_source_file_line_limit.py` `test_source_file_line_limit` fails when a tracked authored source file reaches 1000 lines. The current tree passed; three files remain under that limit and warned.

- [x] PLE is pre-production with no users or durable production data. Fix the design directly;
  there is no legacy behavior to preserve.
  - Evidence (source): `crates/project-tools/src/database_coordinator.rs` `base_is_pre_production` refuses database migrate and tells the operator to update the base schema directly until production freeze.

- N/A Use the pre-production state to improve foundational schemas, contracts, and abstractions
  whenever that produces a stronger long-term system.
  - Reason: repository process guidance; it makes no current PLE product behavior claim.

- [x] Use SQL directly to create the initial PostgreSQL database structure. Insertions have more flexibility.
  - Evidence (source): `schemas/base_schema/install.sql` `00_roles.sql` installs the structural schema as ordered SQL includes.
  - Evidence (source): `schemas/installation_data/live_demo.sql` `course_roster_profile` inserts Live Demo rows separately from that structural manifest.

- [x] Before production, edit the main database design directly as the design changes.
  - Evidence (source): `crates/project-tools/src/database_coordinator.rs` `run` reports that the base schema must be updated directly until production freeze.

- [x] After production, update existing databases without rebuilding them from scratch.
  - Evidence (source): `local_stack_control/lifecycle_migrations.py` `database_operation_for` selects `migrate` after initial installation.

- [x] Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
  - Evidence (source): `devel/development_conformance_audit.py` `current_source_paths` inventories tracked and untracked current-worktree source files without opening deleted paths; `source_name_violations` enforces readable snake_case names.
  - Evidence (source): `devel/development_conformance_audit.py` `load_allowlist` permits only exact documented external-name exceptions owned by approved authority sections.
  - Decision: The one-time adversarial and current-worktree proof was removed after validation; the durable audit command is the regression boundary.

- N/A Give variables for distinct concepts distinct names. For example, `qti_package_upload_file`,
  `qti_package_archive`, `qti_package_extracted_image`, `question_image_asset`,
  `question_image_rendition`, and `object_id` name different roles.
  - Reason: authoring guidance with examples; it makes no separately closable product behavior claim.

- N/A Use the clearest variable name, including a longer name when it better expresses the value; name length has no runtime cost.
  - Reason: authoring guidance; it makes no separately closable product behavior claim.

- N/A Adaptability should be a focus so the software can evolve as requirements and insights change.
  - Reason: future development-direction guidance; it makes no current PLE product behavior claim.

- [x] Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
  - Evidence (source): `devel/dependency_freshness_audit.py` `check_cargo`, `check_node`, and `check_pypi` fail closed unless every direct manifest dependency matches the dated primary-registry snapshot.
  - Evidence (source): `devel/dependency_freshness_snapshot.json` `"snapshot_date": "2026-09-14"` records every direct dependency. `cargo_exceptions` is empty; the only range exception is TypeScript's documented upstream compatibility cap.
  - Evidence (test): `tests/test_crate_boundaries.py` `test_registry_dependencies_use_open_latest_first_requirements` retains Cargo's open latest-first manifest contract.
  - Decision: PyPI declarations use one `>=` floor each, and `aws-sdk-s3` resolves the recorded current 1.147.0 release; no frozen AWS release is permitted. This declaration-and-resolution audit does not duplicate the separate workspace build gate, whose upstream Smithy failure remains an open build report. TypeScript retains its documented temporary upstream compatibility cap.

- N/A If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
  - Reason: conditional future implementation option; it makes no current PLE product behavior claim.

- [ ] All fields, identifiers, domain concepts, and terminology in the PostgreSQL database structure, Rust code,
  TypeScript code, JSON/API contracts, Terminology Contract, and Human Guidance are in alignment.
  - Mismatch: no transferred audit status after Human Guidance regeneration.

### PLE development rules
- [ ] A fresh production installation includes the complete Live Demo by default.
  - Evidence (source): `local_stack_control/lifecycle.py` `provision_ready_installation_data` provisions installation data after readiness.
  - Evidence (test): `tests/test_local_stack_demo_provisioning.py` `test_ready_installation_data_uses_one_canonical_migrator_command_after_readiness` verifies default provisioning.
  - Verification pending: installation source is implemented, but fresh default installation acceptance of the complete current Live Demo and retained Public Genetics example remains required.

- [x] Treat the initial course content as shipped examples.
  - Evidence (source): `schemas/installation_data/live_demo.sql` `ple_data.course_instance` is seeded as installation-owned Live Demo teaching data.

- [x] BiologyProblems.org content is free and open source.
  - Evidence (source): `content/genetics/ATTRIBUTION.md` `CC BY 4.0` records the bundled Biology Problems OER content license.

- [ ] The Genetics Blueprint Course from BiologyProblems.org ships as the example course.
  - Evidence (source): `content/genetics/manifest.yaml` `short_name` and `long_name` define the bundled Genetics Blueprint.
  - Evidence (runtime): 2026-09-16, ordinary discovery through `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.list_blueprint_courses` confirms the current Live Demo has `BPSPXHX6` (`Genetics` / `Fall Genetics`) owned by the Example Content Account (`00000000-0000-0000-0000-000000000106`) and Public, with nine Assessments and 42 distinct Questions. Elena's ordinary Instructor discovery returns it as Public and not owned by her; its live detail route is `https://localhost:8269/blueprint-courses/BPSPXHX6`. The only teaching Course Instance remains `BCHM301`; no Course Instance was created for this correction.
  - Decision: the ordinary owner API published only `BPSPXHX6` using its current ETag. This confirms current-demo discoverability without changing the normal Private-at-creation rule for new Blueprint Courses.
  - Evidence (source): `crates/project-tools/src/installation_data.rs` `publish_bundled_genetics` resolves the validated receipt, requires Example Content owner access, publishes only a Private retained example with its current metadata ETag, and skips an already-Public replay. Reload requires Public, unchanged ownership, and unchanged content Revision; generic `curriculum_content/publication.rs` imports remain Private.
  - Evidence (test): `tests/test_local_stack_demo_provisioning.py` `test_explicit_demo_opt_out_keeps_bundled_content_provisioning` passes with five other focused controller checks. The parent reports `cargo check -p project-tools` passed in 18.03 seconds; both changed Rust files pass rustfmt, the controller passes Pyflakes, and the temporary fixed-shell generation probe passes. These checks do not establish fresh-install behavior.
  - Verification pending: fresh default and opt-out installation, unchanged-Revision replay, and ordinary non-owner Instructor discovery/adoption need connected disposable proof. `local_stack_control/lifecycle.py` `require_bundled_genetics_without_live_demo` now uses the current six-argument Public-only discovery call and checks `availability = 'public'` plus `is_owner`; the separate `read_course_theme(uuid)` check is unchanged. The corrected connected oracle was not run. Independent review of the installation fix remains pending.

- N/A All Podman content on the Mac-Studio-36G machine belongs to this project.
  - Reason: human ownership statement about a named machine, not implemented PLE behavior.

- N/A Neil pre-approves pruning Podman images, volumes, and containers on Mac-Studio-36G as needed.
  - Reason: human authorization statement, not implemented PLE behavior.

- N/A The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
  - Reason: human-owned project priority, not implemented PLE behavior.

- [x] PLE should use one global installation with no institution boundaries.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `ple_private.account` has no institution column or foreign key; user roles are global account data.

- [x] Project images and simulated live-stack data are disposable acceptance infrastructure.
  - Evidence (source): `local_stack_control/disposable_stack_adapter.py` `disposable_target` creates a closed disposable Compose target for acceptance infrastructure.

- [x] `./launchers/run_live_demo.sh` is the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && python3 local_stack.py`.
  - Evidence (source): `launchers/run_live_demo.sh` launcher delegates the normal Live Demo start to `local_stack.py`.
  - Evidence (source): `local_stack.py` `main` is the direct local-stack controller entry point.
## Accounts and roles
### Account rules
- [x] PLE accounts should be global across PLE and use passwordless passkeys and email authentication.
  - Evidence (source): `schemas/base_schema/20_tables/authentication.sql` `ple_private.passkey` and `ple_private.account_authentication_email`.

- [x] Email is not configured for the Live Demo yet; use the visible seeded-role entry for demo access.
  - Evidence (source): `crates/server/src/auth/live_demo.rs` `live_demo_router` mounts only the closed seeded-persona selector at `/api/auth/live-demo/accounts` and deliberately does not mount an email-code ceremony or email delivery.
  - Evidence (source): `src/pages/sign_in_page.tsx` `selectSeededDemoAccount` invokes the visible seeded-role entry from the sign-in page.
  - Evidence (test): `tests/test_live_demo_transport.mjs` `direct-role requests stay same-origin, no-store, and carry only persona` protects the closed seeded-role browser transport.
  - Evidence (test): `crates/server/src/auth/live_demo.rs` `surviving_persona_issues_an_ordinary_session_with_its_account_role` protects seeded selection and the resulting ordinary session.
  - Decision: One-time runtime proof observed five personas, a seeded POST session, and an email-start request with no delivery, 404, and no cookie; it was removed rather than retained as a permanent test. The word "yet" leaves a future Live Demo email path unlocked, and no retired URL is a permanent contract.

- [x] The local Live Demo should not enforce a single browser origin; I want to reach it over a
  firewalled LAN or Tailscale by binding to 0.0.0.0. The local TLS certificate stays because the
  login security tests depend on it.
  - Evidence (source): `containers/Caddyfile` `https://:8080` is the one site, with `bind 0.0.0.0` and `tls internal` on demand from the local CA.
  - Evidence (source): `crates/server/src/auth/browser_boundary.rs` `production_cookie_boundary` accepts one valid request host and its matching HTTPS origin only when `accepting_request_host` is set.
  - Evidence (source): `local_stack_control/live_demo_target.py` `PLE_ACCEPT_REQUEST_BROWSER_HOST` is written as 1 beside the localhost `PLE_BROWSER_ORIGIN`.
  - Evidence (test): `crates/server/src/auth/tests.rs` `live_demo_request_host_accepts_lan_and_tailscale_origins` accepted `100.64.1.2:8443`, `studio.tailnet.ts.net:8443`, an IPv6 host, and `localhost:8443`, and exact mode still rejected the LAN host.
  - Evidence (test): `tests/test_live_demo_target.py` `test_writer_emits_fixed_production_auth_manifest` saw `PLE_ACCEPT_REQUEST_BROWSER_HOST=1` and the hostless Caddy site.

- [x] The three major user types are **Sysadmins**, **Instructors**, and **Students**.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account.user_role` CHECK constraint.

- N/A Potential future user roles are **Course Observers**, **Student Observers**, and **Graders**.
  - Reason: explicitly future product possibility, not current implementation behavior.

- [x] **Students** are required to use their university or institutional (`.edu` in the USA) email accounts.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.enforce_account_authentication_email_role` rejects a Student Authentication Email whose domain is not institutional before the row is stored.
  - Evidence (source): `crates/learning-data-access/src/authentication_email.rs` `require_institutional_student_domain` is the same United States `.edu` rule used before Student account resolution.
  - Evidence (test): `crates/learning-data-access/src/authentication_email.rs` `student_authentication_email_requires_an_institutional_domain` drives `require_institutional_student_domain` for an institutional address, a public mailbox, and a lookalike suffix.

- [x] **Sysadmin** accounts should require higher security than other accounts, like TOTP authentication
  - Evidence (source): `schemas/base_schema/20_tables/authentication.sql` `ple_private.sysadmin_totp_credential` stores private Sysadmin TOTP credentials alongside browser-bound expiring attestations, used counters, and bounded verification attempts; `ple_private.create_authenticated_session` rejects the stored Sysadmin role at the database generic-session boundary. `crates/server/src/auth/sysadmin_totp.rs` routes trusted Sysadmin primary outcomes to pending genuine TOTP verification before creating the ordinary Sysadmin session.
  - Evidence (runtime): `schemas/base_schema/20_tables/authentication.sql` `ple_private.sysadmin_totp_attestation` passed accepted independent SQL boundary proof (`/private/tmp/ple-sysadmin-session-boundary-artifacts.kSMr1H`), denying generic Sysadmin issuance while preserving ordinary Student/Instructor sessions and limited grants. Actual-server HTTP proof (`/private/tmp/ple-sysadmin-session-boundary-http-artifacts.zexsoO`) observed pending MFA with no session, protected denial, missing/wrong browser-binding and bad-code denial, one valid success, replay/expiry/counter-reuse denial, and a five-attempt lock denying a fresh unused valid counter.
  - Decision: C15 closes only this higher-security row. The actual-server transport was loopback HTTP, not deployed TLS; full Live Demo authentication or broader Sysadmin authority is not claimed.

- [x] Every Account has exactly one User Role: **Student**, **Instructor**, or **Sysadmin**.
  - Evidence (source): `schemas/base_schema/10_types.sql` `ple_data.user_role` is the closed Student, Instructor, and Sysadmin enum.
  - Evidence (source): `schemas/base_schema/20_tables/account.sql` `user_role ple_data.user_role NOT NULL` stores one role on each Account.

- [x] User Role is locked and cannot change during the lifetime of an Account.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `reject_account_identity_change` trigger function.

- [x] A person who needs more than one User Role uses separate Accounts.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account` single immutable `user_role` column.

- [x] Instructor Accounts may be deactivated without deleting their authored content, Course relationships, or historical records.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `change_instructor_account_state` changes account state without deleting authored rows or Course membership.
  - Evidence (test): `tests/e2e/e2e_live_demo_instructor_accounts.sh` `assert_deactivation_preserves_records` requires authored workspaces, Blueprints, Questions, assigned Courses, memberships, and history counts to stay unchanged while one state event is appended.

- [x] Reactivating an Instructor Account restores access to the same Account and User Role.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `change_instructor_account_state` preserves `account_id` and `user_role`.
  - Evidence (test): `tests/e2e/e2e_live_demo_instructor_accounts.sh` `reactivated` scenario.

### Instructor role
- [x] All vetted **Instructors** have the same product capabilities.
  - Evidence (source): `crates/server/src/question_library.rs` `instructor_session_hash` authorizes only shared `UserRole::Instructor`.

- [x] A **Sysadmin** vets an Instructor's real identity before creating the Instructor Account.
  - Evidence (source): `crates/server/src/instructor_account.rs` `complete_instructor_identity_vetting` records the Sysadmin's completed identity check before account creation.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `ple_private.require_completed_instructor_identity_vetting` rejects creation unless that decision matches the candidate email.
  - Evidence (test): `crates/learning-data-access/src/instructor_account.rs` `create_instructor_account_requires_a_completed_vetting_decision` drives `CreateInstructorAccountInput` and rejects a create request that omits the decision.

- [x] Course membership determines which private Course records an Instructor may use.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_course_instructor`.
  - Evidence (test): `crates/domain/src/teaching_authority.rs` `foreign_course_membership_cannot_authorize_an_instructor`.

- [x] **Instructors** can search and browse the global **Question Library**.
  - Evidence (source): `crates/server/src/question_library.rs` `question_library_router`.

- [x] **Instructors** can browse the content of Public and Archived **Blueprint Courses**.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.load_blueprint_course` returns current Revision content to an Instructor who owns the course or when availability is Public or Archived.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.list_blueprint_courses` shows Public courses to Instructors and Archived courses only when archived inclusion is requested.
  - Evidence (test): `tests/test_blueprint_instructor_browse.py` `test_instructor_load_includes_public_and_archived_content` reads those shipped visibility predicates.

- [x] **Instructors** log in only with a passkey or email code; no passwords.
  - Evidence (source): `crates/learning-data-access/src/authentication_ceremony.rs` `passwordless_primary_account` accepts an Instructor email code or passkey and has no password method.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.consume_email_authentication_challenge` returns only a Student or Instructor Account.
  - Evidence (test): `crates/server/src/auth/tests.rs` `passwordless_login_issues_student_and_instructor_sessions_only` issues the Instructor passkey session through `issue_passwordless_login`.

- [x] **Instructors** should have a clearly labeled, answer-free **Student** view without changing their identity.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage`.
  - Evidence (source): `crates/server/src/assessment_student_view.rs` `instructor_with_sessions`.
  - Evidence (test): `crates/server/src/assessment_student_view.rs` `student_view_authenticates_only_instructors_without_session_writes`.

### Student role
- [x] **Students** log in only with a passkey or email code; no passwords.
  - Evidence (source): `crates/server/src/auth.rs` `issue_passwordless_login` issues a Student session only after `passwordless_primary_account` accepts an email code or passkey.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.consume_passkey_authentication` returns only a Student or Instructor Account.
  - Evidence (test): `crates/learning-data-access/src/authentication_ceremony.rs` `student_and_instructor_login_accepts_only_email_code_or_passkey` drives `passwordless_primary_account`.

- [x] Students may use multiple passkeys across their devices.
  - Evidence (source): `schemas/base_schema/20_tables/authentication.sql` `ple_private.passkey` non-unique `account_id` foreign key.

- [x] An **Instructor** can reset Student login access and send a new signup code when needed.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster_access.sql` `reset_student_signup_access`.
  - Evidence (source): `crates/server/src/course_roster.rs` `reset_student_signup_access`.
  - Evidence (test): `tests/test_student_signup_reset.py` `test_signup_reset_invalidates_prior_access_and_issues_one_invitation`.
  - Evidence (test): `crates/server/src/course_roster.rs` `signup_reset_mailer_document_carries_one_fresh_code`.

- [x] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster.sql` `course_roster_profile` contains no duplicate Student email; ordinary roster is email-free and direct-Instructor-only.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` removes identifiable Course Student records while retaining Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 purge-preservation probe on 2026-09-15.
  - Owner: 02_accounts.md / Student role (first occurrence; identical requirement and status).

- [x] Student Course data falls under FERPA; treat it as radioactive.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records`.
  - Evidence (source): `schemas/base_schema/60_policies/course_retention_transitions.sql` `ple_course_retention_executor`.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_course_instructor`.
  - Evidence (test): `tests/test_course_retention_ferpa.py` `test_course_retention_deletes_student_work_without_the_account`.

- [x] Student email addresses are immutable.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `ple_private.enforce_account_authentication_email_role` rejects an Authentication Email update when the Account is not an Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `account_authentication_email_role_is_enforced` runs that function before insert or update.

- [x] Student Accounts persist across Courses and semesters.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `ple_private.account` has no Course foreign key.

- [x] A Student Account is global and is not owned by or permanently tied to a Course Instance.
  - Evidence (source): `schemas/base_schema/20_tables/course_membership.sql` `student_record` maps global `student_account_id` to a Course.

- [x] Roster import uses institutional email to find an existing Student Account or create one when needed.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `ple_private.resolve_or_create_student_account` looks up a Student Account by normalized institutional email and creates one only when that email is absent.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `ple_api.import_course_roster` calls `resolve_or_create_student_account` for each reviewed row.
  - Evidence (test): `crates/learning-data-access/src/course_roster.rs` `roster_import_uses_one_normalized_institutional_email_for_account_resolution` drives `validated_entries` so one institutional email is the account-resolution key.

- [x] Each Course Instance has its own course-scoped Student Record and enrollment for the Student Account.
  - Evidence (source): `schemas/base_schema/20_tables/course_membership.sql` `student_record` unique `(course_id, student_account_id)` and `course_membership`.

- [x] Student Work, Attempts, submissions, and grades follow Course retention independently of the Student Account.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records`.
  - Evidence (test): `crates/server/src/course_retention_worker.rs` `course_retention_delete_follows_the_course_not_the_account`.
  - Evidence (test): `tests/test_course_retention_ferpa.py` `test_course_retention_deletes_student_work_without_the_account`.

- [x] Removing a **Student** from a Course revokes future Course access but does not immediately delete the Student's Course records or Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `revoke_course_roster_entry`.
  - Evidence (test): `tests/test_course_retention_ferpa.py` `test_roster_revoke_keeps_student_records_and_work`.

- [x] Student Work and grades remain subject to the normal Course retention policy after enrollment ends.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records`.
  - Evidence (test): `tests/test_course_retention_ferpa.py` `test_course_retention_deletes_student_work_without_the_account`.

- [x] An **Instructor** can deactivate a Student's access to their Course.
  - Evidence (source): `crates/server/src/course_roster.rs` `revoke_course_roster_entry`.
  - Evidence (test): `tests/e2e/e2e_live_demo_roster.sh` `prove_import`.

- [x] Deactivating Course access does not delete the Student Account or Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `revoke_course_roster_entry`.
  - Evidence (test): `tests/test_course_retention_ferpa.py` `test_roster_revoke_keeps_student_records_and_work`.

- [x] An **Instructor** can restore the Student's Course access later.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster_access.sql` `restore_student_course_access`.
  - Evidence (source): `crates/server/src/course_roster.rs` `restore_student_course_access`.
  - Evidence (test): `tests/test_student_signup_reset.py` `test_course_access_restore_reuses_the_student_record`.

- [x] **Instructors** can bulk add Students to a Course Instance through roster import.
  - Evidence (source): `crates/server/src/course_roster.rs` `import_course_roster` POST route.

- [x] **Instructors** remove Students individually.
  - Evidence (source): `crates/server/src/course_roster.rs` `revoke_course_roster_entry` single `{roster_id}` route.

- [x] PLE does not provide bulk Student removal from a Course Instance.
  - Evidence (source): `crates/server/src/course_roster.rs` `course_roster_router` registers only single-entry revoke.

### Sysadmin role
- [x] A **Sysadmin** has full administrative authority over PLE.
  - Decision: Human Guidance defines this authority as platform administration for an active Sysadmin, separate from FERPA Course records and from scoped repair.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_has_platform_administration` grants that capability from the active Sysadmin role and does not consult Course membership.
  - Evidence (test): `tests/e2e/database_baseline_security_catalog.sql` `current_session_account_has_platform_administration` denies an ordinary Sysadmin session, then accepts platform administration and refuses Course membership for the same active Account. A disposable database executed that contract and was removed. No Live Demo stack was started.

- [x] Sysadmins vet **Instructors** and create Instructor Accounts.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `require_completed_instructor_identity_vetting`.

- [ ] Sysadmins can help Instructors repair Courses, Students, and content.
  - Mismatch: `crates/server/src/support_capability.rs` `support_capability_router` and `tests/e2e/e2e_live_demo_support_capability.sh` establish scoped course-roster support, not repair authority for Courses, Students, and content.

- N/A The human developer, Dr. Neil Voss, is currently both a **Sysadmin** and an **Instructor**.
  - Reason: human ownership statement, not an implemented PLE behavior.

- N/A Neil uses separate Sysadmin and Instructor logins so the roles remain distinct.
  - Reason: human ownership and approval statement, not an implementation requirement.

- [x] **Sysadmins** have full platform-administration capability but do not automatically have access to FERPA Course records.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_has_platform_administration`.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_course_instructor`.
  - Evidence (test): `tests/test_course_retention_ferpa.py` `test_sysadmin_platform_administration_does_not_grant_course_records`.

- [x] A Sysadmin may access Course or Student records when needed to resolve a specific support problem.
  - Evidence (source): `crates/learning-data-access/src/support_capability.rs` `IssueSupportRepairCapabilityInput`.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue`.

- [x] Sysadmin support access should be limited to that support task and recorded for audit.
  - Evidence (source): `schemas/base_schema/50_functions/support_repair_capability.sql` `ple_audit.support_repair_capability_event`.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue`.

- [x] Sysadmin support does not make the Sysadmin an **Instructor** or Course member.
  - Evidence (source): `schemas/base_schema/50_functions/support_repair_capability.sql` `issue_support_repair_capability`.

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
## Interface design
### General interface design
- [ ] Design around what users need to find and do.
  - Mismatch: No repository-wide behavioral or usability evidence establishes this broad design outcome.

- [x] Important information should stand out from supporting information.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__title` gives the record title the accent color and leaves the facts muted.
  - Evidence (source): `src/components/record_list/record_family.css` `.record-table thead th` gives each column header a heavier face and a distinct background.
  - Evidence (source): `src/components/record_list/record_family.css` `.record-table tbody th small` keeps the supporting id lighter than the row name.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Important information should stand out from supporting information.` rendered the shipped Gradebook, Course roster, My Active Courses, and Question Library in headless Chromium at 1280 by 800. Titles and column headers stood out from facts, cells, and supporting ids by weight or color. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Related information should be visually grouped and aligned.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__facts` groups each record's related facts in one aligned grid.
  - Evidence (source): `src/components/record_list/record_list.tsx` `record-list__facts` places those facts together under the title.
  - Evidence (source): `src/components/record_list/record_table.tsx` `record-table` keeps each record's related cells on one row.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Related information should be visually grouped and aligned.` rendered the shipped Gradebook, Course roster, My Active Courses, and Question Library in headless Chromium at 1280 by 800. Related cells shared a row and lined up with their headers. Related facts stayed in one group below the title and above the actions, aligned within that group and across repeated records. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Similar pages should place similar controls in consistent locations.
  - Evidence (source): `src/components/record_list/record_sort_control.tsx` `RecordSortControl` places the order label above its select.
  - Evidence (source): `src/pages/library_page.tsx` `Order results` is the Question Library sort control above the result window.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `Sort Public Blueprint Courses` is the Blueprint search sort control above its results.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Similar pages should place similar controls in consistent locations.` rendered the shipped Question Library and Public Blueprint Course search in headless Chromium at 1280 by 800. Each sort control sat above its results, aligned to that result region, with the label above the select and the same select inset. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] A page describes what each record shows and what the user can do. Shared record and page components own the markup, spacing, and reflow, so the same facts and actions stay readable when the page gets narrower.
  - Evidence (source): `src/pages/course_list_page.tsx` `courseContent` names each Course, its classification, its Term, and Open Course.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordList` renders that description in the shared semantic record row.
  - Evidence (source): `src/components/record_list/record_list.css` `record-list--semantic` lays the shared list out as one column so its facts and actions remain in the row when the page is narrower.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkSemanticContent` kept the classification fact, title, facts, and actions visible at a 393px width.

- [x] Use headings and action labels that reflect the current state and next useful step.
  - Evidence (source): `src/pages/student_coursework_presentation.ts` `studentCourseworkDisplay` maps a resumable Attempt to Resume and In progress, a completed Attempt to Review and Completed, and upcoming Coursework to Open and Upcoming.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `assessmentContent` puts the Coursework title, that status, and that action on each row.
  - Evidence (test): `tests/playwright/student_course_entry_m6_evidence.mjs` `Use headings and action labels that reflect the current state and next useful step.` rendered the shipped Course landing in headless Chromium and showed Protein structure practice as In progress with Resume Weekly Assignment, Bonus protein challenge as Completed with Review Bonus Assignment, and Peptide quiz as Upcoming with Open Quiz. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Match feedback wording and visual emphasis to the outcome: success, information, warning, or
  error. Make the result and any next action easy to recognize.
  - Evidence (source): `src/pages/course_roster_page.tsx` `Course access was removed` states the successful roster result, and `Reload and try again` states the error's next action.
  - Evidence (source): `src/components/record_list/record_family.css` `record-collection__state--loading` gives collection loading its own information emphasis, separate from `record-collection__state--error`.
  - Evidence (source): `src/style.css` `confirmation-dialog` uses the warning emphasis, and `attempt-error` uses the danger emphasis.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `Match feedback wording and visual emphasis to the outcome: success, information, warning, or error. Make the result and any next action easy to recognize.` opened the shipped Course roster in headless Chromium. Loading used the information emphasis, the removal confirmation used the warning emphasis and named Keep it or Revoke course access, the successful removal used the success emphasis, and the refused removal used the danger emphasis with Reload and try again. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Primary actions should be easy to find and appear near the content or workflow they affect.
  - Evidence (source): `src/pages/gradebook_page.tsx` `Download CSV` places the score download with the Gradebook table.
  - Evidence (source): `src/pages/course_roster_page.tsx` `Remove course access` places the roster action in the student row.
  - Evidence (source): `src/pages/course_list_page.tsx` `Open Course` places the Course action in the Course record.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `label: "Open"` places the Question action in the Question record.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Primary actions should be easy to find and appear near the content or workflow they affect.` rendered the shipped Gradebook, Course roster, My Active Courses, and Question Library in headless Chromium at 1280 by 800. Each named action stayed in that viewport, inside its record or within the export group above the score table. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Identify the object and relevant context before an action that changes membership or stored
  settings, so users can recognize what they are accepting or changing.
  - Evidence (source): `src/pages/roster_confirmation_dialog.tsx` `Remove course access for` names the student, and the dialog body includes that student's roster ID, before Course access changes.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `instructor-deactivation-copy` names the Instructor Account id before deactivation.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `confirmationTitle` requires the current Assessment title before Unrelease.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `Identify the object and relevant context before an action that changes membership or stored settings, so users can recognize what they are accepting or changing.` opened the shipped Course roster in headless Chromium, showed Morgan Lee and Avery Thompson with their roster IDs before any revoke request, left the roster unchanged on Keep it, and revoked RU-001 only after confirmation. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use concise helper text near the control it explains. Present shared explanations once per
  relevant group and keep the main task information easy to scan.
  - Evidence (source): `src/pages/gradebook_page.tsx` `Export Assessment points` explains the Gradebook download once, beside those actions.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `question-library-bulk-help` explains the bulk actions once, inside that toolbar.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Use concise helper text near the control it explains. Present shared explanations once per relevant group and keep the main task information easy to scan.` rendered the shipped Gradebook and Question Library in headless Chromium at 1280 by 800. Each explanation appeared once, next to its control, while the score table and the first Question stayed in view. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] Avoid scattering related actions across page headers, menus, navigation, and content areas.
  - Mismatch: Current top-bar Sign Out contradicts the specified Profile-menu location.

- [ ] Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
  - Mismatch: No repository evidence can verify this whole-product qualitative outcome.

- [x] Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Evidence (source): `src/features/profile_avatar/profile_avatar_role.ts` `profileRoleMayManageImage` shows a Profile image upload only to an Instructor or Sysadmin.
  - Evidence (source): `crates/server/src/profile_avatar.rs` `replace_profile_image` refuses every other role before reading an image.
  - Evidence (source): `crates/server/src/draft_question_images.rs` `instructor_session_hash` admits only an Instructor session before a Draft Question image upload.
  - Evidence (source): `crates/server/src/course_appearance.rs` `instructor_session_hash` admits only an Instructor session before a Course Banner upload.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx` `Student-facing prompt` is a text box for Instructor-created Question content.
  - Evidence (source): `src/pages/course_roster_page.tsx` `importRoster` records Course roster rows from a text box.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `Students should have no upload capabilities. Instructor-created content should use text boxes` found file inputs only for the Course Banner, Profile image, and HOTSPOT image, refused those Instructor routes and the Profile image control for a Student, and kept the Question prompt and roster import as text boxes.

- [x] Buttons should look intentionally designed rather than like native browser controls.
  - Evidence (source): `src/style.css` `:where(button)` sets appearance to none and gives each button a border, corner radius, background, and type weight.
  - Evidence (source): `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile` and `ple-app-ribbon__profile-menu-item` keep a designed border and corner radius and do not set appearance.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Buttons should look intentionally designed rather than like native browser controls.` rendered the shipped Ribbon with those stylesheets in headless Chromium. Profile and Sign out computed appearance none and a non-zero corner radius. No Live Demo stack was started. No PostgreSQL proof was run.

### Rounded rectangles preference
- [x] Rounded rectangles are preferred for all interface objects, especially buttons, input fields, cards, avatars, tags, and interactive controls.
  - Evidence (source): `src/style.css` `:where(button)` rounds buttons, the text input rule rounds fields, `.course-card` rounds a card, and `.calm-status` rounds the status tag.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon__profile` rounds the Profile avatar.
  - Evidence (source): `src/components/question_response_control_styles.ts` `choice-card` rounds an answer choice.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Rounded rectangles are preferred for all interface objects, especially buttons, input fields, cards, avatars, tags, and interactive controls.` measured those named shipped objects in headless Chromium. The button, input, card, avatar, and answer choice were above 0px and below 16px. The status tag kept its compact pill. Structural regions stay square under the later structural rule. No Live Demo stack was started. No PostgreSQL proof was run.

- N/A Rounded corners generally feel softer, friendlier, and more contemporary.
  - Reason: supporting descriptive rationale, not independently closable; it remains binding design context for the rounded-object requirement.

- N/A Rounding also helps users visually distinguish discrete objects from the surrounding page.
  - Reason: supporting descriptive rationale, not independently closable; it remains binding design context for the rounded-object requirement.

- [x] Use corner radius to reinforce interface hierarchy.
  - Evidence (source): `src/style.css` `--ple-radius-surface` is smaller than `--ple-radius-control`, and `--ple-radius-control` is smaller than `--ple-radius-inset`. `body` sets no corner radius.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon` sets no corner radius.
  - Evidence (source): `src/features/question_picker/question_picker.css` `.question-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use corner radius to reinforce interface hierarchy.` measured the page body and Ribbon at 0px, then a course card, a button, and the Question picker dialog in increasing order, with the dialog below 16px, in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Interactive and self-contained objects should generally be more rounded than structural containers.
  - Evidence (source): `src/style.css` `:where(button)` and the text input rule use the control radius, `.confirmation-dialog` uses that same radius, and `.course-card` uses the surface radius.
  - Evidence (source): `src/components/question_response_control_styles.ts` `choice-card` uses the control radius for an answer choice.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Interactive and self-contained objects should generally be more rounded than structural containers.` measured a shipped button, input, answer choice, confirmation dialog, and course card above 0px and below 16px after the page body, main region, Ribbon, tab navigation, and breadcrumb prelude measured 0px. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use moderate rounding for buttons, input fields, answer choices, dialogs, and similar interactive controls.
  - Evidence (source): `src/style.css` `:where(button)` and the text input rule use `--ple-radius-control`, and `.confirmation-dialog` uses that same control radius.
  - Evidence (source): `src/components/question_response_control_styles.ts` `choice-card` uses the control radius for an answer choice.
  - Evidence (source): `src/features/question_picker/question_picker.css` `.question-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/question_pool_picker/question_pool_picker.css` `.question-pool-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/pages/course_instance_page.css` `.course-instance-blueprint-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/blueprint_course/blueprint_course.css` `.blueprint-course-create-dialog` uses `--ple-radius-inset`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use moderate rounding for buttons, input fields, answer choices, dialogs, and similar interactive controls.` measured those named shipped controls in headless Chromium above 4px and below 16px. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use subtle rounding for cards, tables, panels, and other content containers.
  - Evidence (source): `src/style.css` `.course-card` and `.question-card` use `--ple-radius-surface`, which is smaller than `--ple-radius-control`.
  - Evidence (source): `src/components/record_list/record_family.css` `.record-table__scroll` uses `--ple-radius-surface` for the table container.
  - Evidence (source): `src/pages/question_statistics_panel.css` `.question-statistics-panel` uses `--ple-radius-surface`.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_authoring.css` `.assessment-editor-panel` uses `--ple-radius-surface`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use subtle rounding for cards, tables, panels, and other content containers.` measured those shipped containers in headless Chromium above 0px and below the button radius. Dialogs, avatars, and compact media keep the inset radius. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Keep large page regions, navigation bars, breadcrumbs, and other structural layout elements square or nearly square.
  - Evidence (source): `src/style.css` `body` and the main content region set the page surface without a corner radius.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon` is the navigation bar and sets no corner radius. Tab links on that bar keep the control radius.
  - Evidence (source): `src/ribbon/app_ribbon_density.css` `.ple-shell__breadcrumb-prelude` is the breadcrumb trail and sets no corner radius.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Keep large page regions, navigation bars, breadcrumbs, and other structural layout elements square or nearly square.` measured the shipped Ribbon, its tab navigation, a breadcrumb prelude using the shipped class, body, and main at 0px on every corner in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Pills and fully rounded shapes should be reserved for compact objects such as tags, badges, timers, and avatars.
  - Evidence (source): `src/style.css` `.calm-status` is the fully rounded status badge.
  - Evidence (source): `src/pages/library_page.css` `.question-library-browse-active` wraps the fully rounded active-filter tag.
  - Evidence (source): `src/components/question_response_control_styles.ts` `QUESTION_RESPONSE_CONTROL_STYLES` fully rounds the choice number and the status spinner.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Pills and fully rounded shapes should be reserved for compact objects such as tags, badges, timers, and avatars.` inventories every fully rounded border radius under src and measures those four compact elements in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Apply corner radii consistently to objects that serve the same purpose.
  - Evidence (source): `src/style.css` `.primary-action` and `.quiet-action` share `--ple-radius-control` with `:where(button)`.
  - Evidence (source): `src/style.css` `input:not([type="radio"])` shares `--ple-radius-control` with select and textarea.
  - Evidence (source): `src/style.css` `.course-card` and `.question-card` share `--ple-radius-surface`.
  - Evidence (source): `src/features/question_picker/question_picker.css` `.question-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/question_pool_picker/question_pool_picker.css` `.question-pool-picker-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/pages/course_instance_page.css` `.course-instance-blueprint-dialog` uses `--ple-radius-inset`.
  - Evidence (source): `src/features/blueprint_course/blueprint_course.css` `.blueprint-course-create-dialog` uses `--ple-radius-inset`.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Apply corner radii consistently to objects that serve the same purpose.` measured three action buttons at one radius, three text fields at one radius, two cards at one radius, and four task dialogs at one larger radius in headless Chromium. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use the application's typography, spacing, corner radius, borders, and interaction states consistently.
  - Evidence (source): `src/style.css` `font-family: inherit` keeps buttons and text fields on the application face. `:where(button)` sets the shared control size, padding, border, radius, and hover. `:focus-visible` sets the shared focus ring.
  - Evidence (source): `src/style.css` `input:not([type="radio"])` shares field padding, border, and radius with select and textarea.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use the application's typography, spacing, corner radius, borders, and interaction states consistently.` measured plain, primary, quiet, and disabled buttons, plus input, select, and textarea, in headless Chromium with the shipped grass light tokens. They shared the application font and a 1px solid border. Each group shared padding and radius. The disabled button used a not-allowed cursor, keyboard focus showed a 2px solid ring, and hover changed the button border. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Primary, secondary, and low-emphasis actions should be visually distinct.
  - Evidence (source): `src/style.css` `:where(button)` is the unclassified secondary face, `.primary-action` is the filled primary face, and `.quiet-action` is the transparent low-emphasis face.
  - Evidence (source): `src/components/unsaved_changes_guard.tsx` `Stay and keep editing` is the low-emphasis action, the unlabeled save button is the secondary face, and `Discard and continue` is the primary action.
  - Evidence (source): `src/appearance/theme_registry.ts` `themeStyle` supplies the grass light tokens used to measure those faces.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Primary, secondary, and low-emphasis actions should be visually distinct.` rendered the shipped unsaved-changes dialog in headless Chromium. The quiet, unclassified, and primary buttons computed three different backgrounds, and the quiet border differed from the secondary border. No Live Demo stack was started. No PostgreSQL proof was run.

### Information density and layout
- [x] Design Instructor and **Sysadmin** workflows for laptop browsers, using a 1280 by 800 viewport
  as the layout target.
  - Evidence (source): `tests/playwright/ui_corpus_manifest.ts` `RIBBON_RESPONSIVE_PROFILES` and `SYSADMIN_DESKTOP_CONTEXT_OPTIONS` declare 1280 by 800 desktop contexts for both staff roles.
  - Evidence (test): `tests/playwright/ribbon_responsive_evidence.mjs` `assertResponsiveRows` verifies the Instructor desktop shell and `assertSysadminDesktopRibbon` verifies the Sysadmin Ribbon has no overflow with Instructor Accounts and Scoped Support visible.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminHomePage` presents the backed Instructor Accounts and Scoped Support operations reached by the checked Sysadmin desktop model.
  - Decision: retained viewport-target evidence supports this equivalent design-target rewrite, not whole-product usability or all staff workflows.

- [x] PLE often presents large collections where users need to find a few relevant items.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders the Question Library collection surface.

- [ ] Optimize large collections for scanning, searching, filtering, and comparison.
  - Mismatch: The broad all-collection outcome lacks complete implementation evidence.

- [x] Show enough useful information at once to support comparison without excessive scrolling.
  - Evidence (source): `src/pages/library_page.css` `question-library-bulk-toolbar` keeps the Question Library bulk strip compact and gives the result column the remaining width at the 1280 by 800 layout.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `comparableLibraryRows` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. Two Library records stayed fully in that viewport with their titles and facts, and two roster rows stayed fully in that viewport. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Search and filters should help users quickly narrow large collections.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` applies search text, author, backend, tag, classification, Question type, license, course use, and capability changes to the current Question Library query.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` excludes a Question that misses the requested text, author, tag, classification, type, or license.
  - Evidence (source): `crates/learning-data-access/src/postgres/question_library/search.rs` `search_published_question_library_entries` calls that shipped PostgreSQL function.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/question_library.rs` `nonmatching_question_id` adds one outsider beside 65 matching Questions. `question_library_search_filters_and_pages_in_postgresql` runs `PostgresQuestionLibraryStore` on disposable PostgreSQL. The shared filters return those 65 Questions and omit the outsider. The outsider's tag returns only that Question. Excluding the matching text keeps the outsider. This proof is 66 Questions. It does not time a 13,000 Question library.

- [x] Dense pages should remain easy to scan.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` keeps each Published Question title and its facts on one record row.
  - Evidence (source): `src/components/record_list/record_list.css` `record-list__facts` places those facts in a grid on the row.
  - Evidence (source): `src/pages/course_roster_page.tsx` `CourseRosterPage` keeps each Student, state, and action on one roster row.
  - Evidence (test): `tests/playwright/test_dense_page_scan.mjs` `DENSE_PAGE_SENTENCE` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. Two Library records stayed in the viewport with their titles and facts on one line. Two roster rows stayed in the viewport with aligned Student and state columns. The rows were square, unshadowed, and separated by a divider. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Treat screen space as a limited resource. Prefer useful information over decorative whitespace.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` places each Published Question in the result region.
  - Evidence (source): `src/pages/library_page.css` `library-browse-record-list__window` keeps desktop result rows on short block padding so the record text uses the row.
  - Evidence (source): `src/pages/course_roster_page.tsx` `CourseRosterPage` renders the Current roster in `src/pages/course_roster_page.css` `roster-section`, which sets no padding and no reserved block under the table.
  - Evidence (test): `tests/playwright/test_screen_space.mjs` `SCREEN_SPACE_SENTENCE` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. The result region had no padding and its visible area was filled by records. Each visible record's title, description, facts, and action were taller than the row padding, and the space between those parts stayed within 16 pixels. The roster section ended at the table, and each Student cell's text was taller than its padding. Leftover viewport under the short roster is the end of the page. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use spacing to separate meaningful groups rather than simply making pages spacious. Large gaps should communicate a meaningful change in section or task.
  - Evidence (source): `src/pages/library_page.tsx` `library-page__filters` stacks Question Library filters apart from `src/pages/library_page.css` `library-page__results`, so the result window does not open a hole between filter groups.
  - Evidence (source): `src/pages/course_roster_page.css` `roster-tools` separates the import tools from the Current roster.
  - Evidence (test): `tests/playwright/test_group_spacing.mjs` `GROUP_SPACING_SENTENCE` rendered the shipped Question Library and Course roster in headless Chromium at 1280 by 800. Filter groups stayed 8 to 16 pixels apart, and the space inside one record stayed within that. Filters and results stayed 8 to 16 pixels apart. The result stack stayed within 24 pixels, and the result region stayed inside the viewport. Roster rows met at a divider, and the import tools followed the roster across a larger gap of at most 32 pixels. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
  - Evidence (source): `src/pages/library_browse_controls.tsx` `LibraryBrowseControls` names each browse group with a heading.
  - Evidence (source): `src/pages/library_page.css` `question-library-browse-groups` separates those groups with a bottom divider and gives the choices no card, border, or nested box.
  - Evidence (test): `tests/playwright/test_alignment_dividers.mjs` `ALIGNMENT_DIVIDER_SENTENCE` rendered the shipped Question Library browse groups in headless Chromium at 1280 by 800. Subjects, Tags, and Question Types shared one column. Each heading aligned with its group. Each group closed with a one-pixel divider and had no fill, corner radius, shadow, or side border. The choice list inside each group was not another box. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Cards and rounded containers should earn their space by representing a distinct object or interaction, not merely grouping nearby content.
  - Evidence (source): `src/pages/library_page.css` `question-library-controls` leaves Classification filters and Bloom filters unpadded, square, unfilled, and unshadowed. `src/pages/library_page.css` `question-library-bloom-report` does the same for the count lists. `src/pages/library_page.css` `question-library-bulk-toolbar` does the same for the action strip.
  - Evidence (source): `src/pages/library_page.css` `question-library-facet-choices` keeps a border and control radius on each choice, and `src/pages/library_browse_controls.tsx` `question-library-browse-active` keeps the selected Tag as its own pill.
  - Evidence (test): `tests/playwright/test_distinct_object_cards.mjs` `DISTINCT_OBJECT_SENTENCE` rendered the shipped Question Library browse page in headless Chromium at 1280 by 800. Classification filters, Bloom filters, the Bloom count lists, and the bulk-action strip grouped more than one field or button and had no border, radius, shadow, or fill. One facet button kept its border and radius, and the Tag pill kept its radius. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Avoid the modern dashboard style of large rounded cards, generous padding, and isolated islands of content.
  - Evidence (source): `src/pages/assessment_templates_page.css` `assessment-template-overview` places Your Templates and the editor in one workspace without card fill, radius, or shadow. `src/pages/assessment_templates_page.css` `assessment-template-editor-empty` leaves Choose a Template unpadded and square.
  - Evidence (source): `src/pages/library_page.css` `question-library-controls-initial` keeps the opening search on the page without hero padding. `src/pages/library_page.css` `question-library-search-tips` leaves the tip text without a rounded shadow box. `src/pages/library_browse_record_list.css` `library-browse-record-list__window` keeps the result rows in a scroll region without a floating card.
  - Evidence (test): `tests/playwright/test_dashboard_islands.mjs` `DASHBOARD_SENTENCE` rendered the shipped Template workspace, opening Question Library search, and Question Library browse page in headless Chromium at 1280 by 800. The Template columns shared one top edge and stayed within 24 pixels, with no padding, radius, shadow, or fill. Choose a Template was flat. The opening search had no block padding, and the opened search tips were flat text. The result region was at least 300 pixels tall, held two records, and had no card fill or shadow. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use horizontal and vertical space efficiently without crowding information together. Related information should form clearly readable rows, columns, or groups.
  - Evidence (source): `src/pages/assessment_templates_page.css` `assessment-template-editor-empty` separates Choose a Template from its message. `src/pages/assessment_templates_page.css` `assessment-template-overview` stacks Your Templates as one column beside the editor.
  - Evidence (source): `src/pages/library_page.css` `question-library-controls` keeps each classification name off its field. `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` stacks a Question title, description, facts, and action. `src/components/library_bloom_discovery.tsx` `question-library-bloom-report` places each count beside its name.
  - Evidence (test): `tests/playwright/test_readable_groups.mjs` `READABLE_GROUP_SENTENCE` rendered the shipped Template workspace and Question Library browse page in headless Chromium at 1280 by 800. The Template message sat 4 to 16 pixels under its heading, the overview stacked in one column, and the editor stayed 8 to 24 pixels to its right. A classification name sat 4 to 16 pixels above its field. A Question record stacked its parts in that same range, and at least two facts shared a row. A Bloom count sat at least 8 pixels from its name, and the two count lists stayed 4 to 16 pixels apart. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Size controls and content regions for their contents and task. Avoid unnecessarily tall panels, empty states, Question previews, and other fixed-height regions.
  - Evidence (source): `src/pages/library_page.css` `library-browse-record-list__window` keeps the desktop Question Library result region inside the viewport instead of a 65vh panel measured from the top of the screen.
  - Evidence (source): `src/pages/assessment_templates_page.css` `assessment-template-editor-empty` gives the Template empty state no reserved block size and no trailing message margin.
  - Evidence (source): `src/components/opaque_webwork_preview_frame.tsx` `OpaqueWebworkPreviewFrame` sizes the Question preview from a resize report between 160 and 1200 pixels.
  - Evidence (test): `tests/playwright/test_region_sizing.mjs` `REGION_SENTENCE` rendered those shipped surfaces in headless Chromium at 1280 by 800. The result region stayed inside the viewport and held two records. The empty state was shorter than 160 pixels with at most 24 pixels under its message. The preview ignored a 2000 pixel report and adopted 420 pixels. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Keep the visual design compact, flat, information dense, and consistent across PLE.
  - Evidence (source): `src/pages/library_page.css` `question-pool-create-review-grid` and `src/components/question_pool_create_dialog.css` `question-pool-create-review-grid` use the same flat groups as Question Library browse.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` and `src/pages/course_roster_page.tsx` `CourseRosterPage` keep collection rows compact and unshadowed, with the shared action height.
  - Evidence (test): `tests/playwright/test_visual_consistency.mjs` `VISUAL_CONSISTENCY_SENTENCE` rendered the shipped Question Library, Course roster, and Question Pool review in headless Chromium at 1280 by 800. Two Library records and two roster rows stayed in view, shared one font, and had no corner radius or shadow. Their action heights matched within 2 pixels. The Pool review groups matched the browse groups: no padding, fill, radius, or shadow, in two columns 8 to 24 pixels apart. This proof does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use compact rows, restrained corner rounding, and controls sized to their task.
  - Evidence (source): `src/style.css` `.instructor-list__row` uses `--ple-radius-surface` and `--ple-list-row-min-block-size`.
  - Evidence (source): `src/style.css` `:where(button)` uses `--ple-control-min-height` for the shared control size.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__row` uses compact block padding and a divider, and sets no corner radius.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Use compact rows, restrained corner rounding, and controls sized to their task.` measured an instructor row and a record row in headless Chromium. Each row was at least as tall as its button and shorter than three button heights. The instructor row radius was above 0px and below the button radius. The record row radius was below the button radius. Both buttons shared one height. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Present short labels and values in aligned rows or compact grids, adapting to stacked groups
  when the available width requires them.
  - Evidence (source): `src/pages/question_statistics_panel.tsx` `QuestionStatisticsPanel` renders Blank rate, Answered rate, Correct rate, Partial rate, Incorrect rate, and Mean credit as short labels with their values.
  - Evidence (source): `src/pages/question_statistics_panel.css` `question-statistics-measures` places those pairs in two columns and stacks them to one column below 40rem.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Present short labels and values in aligned rows or compact grids, adapting to stacked groups when the available width requires them.` rendered QuestionStatisticsPanel in headless Chromium, measured two aligned columns at 1280px, then one stacked column at 480px. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Give each object one clear title within its list entry. Group its metadata and actions beneath
  or alongside that title.
  - Evidence (source): `src/pages/course_list_page.tsx` `courseContent` gives each Course one title, its classification and Term, and Open Course.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordSemanticContent` places that one title, then the record's facts, then its actions.
  - Evidence (source): `src/components/record_list/record_table.tsx` `RecordTable` gives each record one row header and places the other columns in that same row.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkAlignment` kept one title per list entry and grouped its metadata and actions beneath that title.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkRecordFamily` kept one row-header title with its metadata and action in the same row, and one review heading with its note and action.

- [x] Preserve readable text and reachable controls as users enlarge text or zoom the page.
  - Evidence (source): `src/style.css` `font-size: 100%` lets the document root follow the user's text size, and Ribbon labels and buttons use rem sizes that grow with that root.
  - Evidence (source): `src/index.html` `initial-scale=1.0` leaves page zoom available.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Preserve readable text and reachable controls as users enlarge text or zoom the page.` rendered the shipped Ribbon in headless Chromium, measured the Courses label and the outside button at a 200% root and at zoom 2, and kept each text run visible and each control reachable. No Live Demo stack was started. No PostgreSQL proof was run.

### Interaction design
- [x] Use progressive disclosure to keep common tasks compact while making supporting details easy
  to find.
  - Evidence (source): `src/style.css` `details:not([open]) > :not(summary)` hides supporting content until its disclosure is opened.
  - Evidence (source): `src/pages/course_roster_page.tsx` `roster-tools` keeps the Roster tools summary available and places the import explanation inside that disclosure.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Use progressive disclosure to keep common tasks compact while making supporting details easy to find.` rendered the Roster tools disclosure in headless Chromium, kept the summary visible while the import explanation had no box, then opened the section and showed that explanation. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use tooltips for brief supplementary explanations, available on hover and keyboard focus.
  - Evidence (source): `src/style.css` `attr(title)` shows the Profile control and display-mode switch titles on hover and keyboard focus, and hides the Profile tip while its menu is open.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `title="Profile"` explains the icon Profile button, and `src/appearance/display_mode_toggle.tsx` `Switch to ` explains the display-mode glyph.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Use tooltips for brief supplementary explanations, available on hover and keyboard focus.` rendered the shipped Ribbon Profile button in headless Chromium, showed its title below the button on hover, showed that same title on keyboard focus, and hid it at rest and while the Profile menu was open. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use clearly labeled expandable sections with chevrons for longer details and secondary settings,
  supporting keyboard, pointer, and touch interaction.
  - Evidence (source): `src/style.css` `details > summary::before` draws one disclosure chevron, and `details[open] > summary::before` turns that chevron when the section is open.
  - Evidence (source): `src/pages/course_roster_page.tsx` `roster-tools` labels the secondary roster section Roster tools.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Use clearly labeled expandable sections with chevrons for longer details and secondary settings, supporting keyboard, pointer, and touch interaction.` rendered that Roster tools disclosure in headless Chromium, opened it from the keyboard, closed it with a pointer click, and opened it again from a touch tap. The chevron stayed visible and changed direction while open. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Keep essential information, primary actions, and current status visible in the main interface.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` shows the signed-in role in the Ribbon.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `aria-current` marks the selected Ribbon tab, and the Ribbon tabs are the primary actions.
  - Evidence (test): `tests/playwright/ribbon_narrow_routed_shell.mjs` `Keep essential information, primary actions, and current status visible in the main interface.` rendered the shipped Ribbon in headless Chromium at 1280 by 800. The Instructor role, every Ribbon tab, and the current tab were visible in that bar while the Profile menu was closed. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Use drag-and-drop where it makes reordering faster and more natural.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `RecordSequence` orders Assessment Entries through the shared record movement.
  - Evidence (source): `src/components/record_list/record_sequence.tsx` `completeDrop` accepts a native drop only when the dragged record and the target are distinct enabled records in the current order.
  - Evidence (source): `src/components/record_list/record_list_reorder.tsx` `RecordMoveControls` renders the native drag control for that movement.
  - Evidence (test): `tests/test_record_list_reorder.mjs` `native drag moves Alpha onto Charlie` moves Alpha onto Charlie, refuses a disabled or same-record drop, and renders the enabled drag control.

- [x] Reordering must also have a precise keyboard-accessible method.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `moveEntry` and `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `move` back their labelled native-button Move earlier and Move later controls.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_choice_list.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multiple_answer_editor.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multi_fill_in_editor.tsx` `onMoveBlank`, `src/features/ple_question_json_authoring/question_json_matching_editor.tsx` `onMoveItem`, and `src/features/ple_question_json_authoring/question_json_ordering_editor.tsx` `onMoveItem` give every native JSON reorderer the same precise buttons.
  - Evidence (test): `tests/test_blueprint_course_model.mjs` `reusable entries preserve fixed and Question Pool interleaving`, `tests/test_ple_question_json_editor_model.mjs` `choice edits retain semantic IDs and enforce choices and correct-answer invariants`, `tests/test_ple_question_json_multiple_answer_editor.mjs` `multiple-answer text edits and reordering retain choice IDs and exact correct IDs`, and `tests/test_ple_question_json_multi_fill_ordering_authoring.mjs` `ORDER treats Ordering Items as the source of truth and derives correctOrder after movement` protect the stable reorder results.
  - Decision: The one-time seven-surface keyboard-control inventory passed and was removed rather than becoming a permanent implementation-inventory test. Drag-and-drop is checked on its own bullet.

- [x] UUIDs should never appear in visible content, navigation URLs, or copyable links.
  - Evidence (test): `tests/test_frontend_contract.mjs` `UUIDs should never appear in visible content, navigation URLs, or copyable links.` rendered Discipline: Name unavailable, built the Blueprint search description as Name unavailable, titled the retained Attempt Quiz: Attempt 2 with roster RU-001, and showed Impact notice activity for 7K3M-79QP. That visible text omitted aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee. The activity link kept that UUID only in its route fragment. No Live Demo stack was started. No PostgreSQL proof was run.
  - Evidence (source): `src/components/course_classification_summary.tsx` `vocabularyName` returns Name unavailable when the vocabulary item is missing.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `blueprintClassificationDescription` uses Name unavailable for a missing classification name.
  - Evidence (source): `src/components/course_student_work_recovery.tsx` `archivedAttemptVisibleFacts` titles the Attempt number and roster ID.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `notificationContent` keeps the activity UUID in the route fragment.
  - Evidence (source): `src/components/copyable_question_id.tsx` `CopyableQuestionId` copies only a canonical Question ID.
  - Evidence (source): `src/pages/profile_account_id.tsx` `validateCanonicalPublicId` copies only a canonical Account ID.
  - Evidence (test): `tests/test_public_navigation.mjs` `objects without a public ID use their UUID in routes and reject a secondary Id` accepts the UUID route for an Attempt, Draft, and proposal and rejects a second Id.
  - Decision: The later rule uses a UUID Id in routes and JSON for an object without a public ID. Visible text and clipboard copies stay free of that UUID.

### Role badges
- [x] The role badge is always in the upper left, just left of the logo.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` is the first control in `ple-app-ribbon__context-identity`, immediately before `ple-app-ribbon__brand`.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon__user-role` stays displayed at phone width for Student, Instructor, and Sysadmin.

- [x] **Sysadmin** uses tomato red as its role color.
  - Evidence (source): `src/styles/user_role.css` `[data-user-role="sysadmin"]` defines `--ple-role-accent: #ff6347`.

- [x] **Instructor** uses teal green as its role color.
  - Evidence (source): `src/styles/user_role.css` `[data-user-role="instructor"]` defines `--ple-role-accent: #168575`.

- [x] **Student** uses lavender purple as its role color.
  - Evidence (source): `src/styles/user_role.css` `[data-user-role="student"]` defines `--ple-role-accent: #8861b5`.

- [x] The student role badge in mobile is shortened.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` shows the Student role label.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon__user-role[data-user-role="student"]` keeps that badge visible and narrower than the desktop reservation on a phone.

- [x] Role colors should be used consistently in role labels and other appropriate interface cues.
  - Evidence (source): `src/styles/user_role.css` `.live-demo-persona-action[data-user-role]` and `.ple-app-ribbon__user-role[data-user-role]` consume the shared role tokens.

- [x] Demo role selection should clearly state both the user's role and name.
  - Evidence (test): `tests/playwright/e2e_live_demo_authoring_browser.mjs` selects `Assume the role of Instructor Dr. Elena Rivera`.

### Themes
- [x] The interface uses a fixed set of visually distinct biome and habitat themes.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` checks the closed Theme catalog, habitat names, and distinct canvases.
  - Evidence (source): `src/appearance/theme_registry.ts` `THEME_REGISTRY` is that fixed catalog.

- [x] Each theme has coordinated Light and Dark appearances.
  - Evidence (source): `src/appearance/theme_registry.ts` `THEME_REGISTRY` stores a light and dark palette for every Theme.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` reads both appearances.

- [x] An Instructor has a personal theme for global Instructor pages and independently controls the theme assigned to each Course.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updatePersonalTheme` saves the Instructor personal theme.
  - Evidence (source): `src/appearance/profile_appearance.tsx` `savePersonalTheme` offers that control only to an Instructor.
  - Evidence (source): `crates/server/src/course_appearance.rs` `update_theme` persists the Course theme for an Instructor.

- [x] Course pages use the Course theme. Global Instructor pages use the Instructor's personal theme.
  - Evidence (source): `src/appearance/appearance_rules.ts` `resolveTheme` selects the Course theme when a page has one and the personal theme on global Instructor pages.
  - Evidence (test): `tests/test_appearance_rules.mjs` `Course pages use their Course Theme` covers a Course page.
  - Evidence (test): `tests/test_appearance_rules.mjs` `global Instructor pages use the Instructor personal Theme` covers a global Instructor page.

- [x] Light/Dark is a separate user preference that applies across all themes and pages.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updateDisplayModePreference` saves display mode without a theme.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `dataset.displayMode` applies that one preference on the document.

- [x] Light/Dark has only two selectable values: Light and Dark. When no explicit preference is saved, follow the browser setting.
  - Evidence (source): `generated/api/DisplayMode.ts` `DISPLAY_MODE_VALUES` lists light and dark.
  - Evidence (test): `tests/test_appearance_rules.mjs` `an unset display preference follows the browser` uses the browser value when no preference is saved.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `prefers-color-scheme` reads the browser setting.

- [x] Changing Light/Dark must not change the selected theme. Changing a theme must not change Light/Dark.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updateDisplayModePreference` writes only the display-mode preference.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `updateInstructorPersonalTheme` writes only the personal theme.
  - Evidence (source): `crates/server/src/course_appearance.rs` `update_theme` writes only the Course theme.

- [x] Themes should affect major page surfaces so each theme is visually distinct across the whole interface.
  - Evidence (source): `src/appearance/appearance_owner.tsx` `themeStyle` applies the resolved theme tokens to the document root.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` checks page, surface, secondary, highlight, and accent colors.

- [x] Each Light or Dark theme appearance is defined by five source colors: Canvas, Surface, Secondary, Accent, and Highlight.
  - Evidence (source): `src/appearance/theme_registry.ts` `ThemePalette` names canvas, surface, secondary, accent, and highlight.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` compares those five colors with the palette document.

- [x] Theme colors should remain accessible in their actual interface uses.
  - Evidence (source): `src/appearance/theme_registry.ts` `themeStyle` publishes the shared tokens used by the shell, Ribbon, and result surfaces.

- [x] Light themes should use clearly light page backgrounds. Dark themes should use clearly dark page backgrounds.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` requires every light canvas to stay clearly light and every dark canvas clearly dark.
  - Evidence (source): `src/appearance/theme_registry.ts` `themeStyle` publishes that canvas as the page background.

- [x] Check text, controls, borders, and interaction states against their actual rendered backgrounds.
  - Evidence (source): `src/ribbon/app_ribbon.css` `background-color` fills the selected Ribbon tab without clearing its pending gradient.

- [x] Apply the contrast requirements for text, controls, and other semantic uses in
  [BIOME_THEME_PALETTES.md](/docs/BIOME_THEME_PALETTES.md) to rendered components in both Light
  and Dark modes, including gradients and state backgrounds.
  - Evidence (source): `src/style.css` `brand-mark` paints its label on an accent gradient whose stops stay readable.

- [x] Pair color cues with text, icons, or shapes so selection, focus, saved status, and results remain
  recognizable across themes and color-vision differences.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `is-saved` marks a saved Question with a check and a thick edge.
  - Evidence (source): `src/components/student_feedback_panel.tsx` `studentFeedbackAnnouncement` states Correct or Not quite in words.

- [x] Theme IDs are durable. Changing a theme's display name or colors should not require a new ID.
  - Evidence (source): `src/appearance/theme_registry.ts` `grass` remains the Theme id while its display name is Grassland.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` checks that id and name pair.

- [x] Follow `docs/BIOME_THEME_PALETTES.md` for theme names, palettes, accessibility, and implementation.
  - Evidence (source): `src/appearance/theme_registry.ts` `THEME_REGISTRY` stores the palette document's habitat names and five source colors.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `the fixed biome catalog keeps durable habitat names and distinct light and dark surfaces` compares those names and colors with the palette document.

### Typography
- [x] Use [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) as the main PLE font.
  - Evidence (source): `src/style.css` `:root` sets `Atkinson Hyperlegible Next` as the first font family.
  - Evidence (runtime): `src/style.css` `:root` was confirmed by a current authorized Student WeBWorK iframe probe at 1440 and 390 CSS pixels: `document.fonts` was loaded and computed `Atkinson Hyperlegible Next` first on both the body and its visible input. Receipt: `/private/tmp/ple-resumed-types-20260916.md`.

- [x] Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-mono` loads the local Atkinson Hyperlegible Mono face for code and monospace text.

- [x] Prefer the official Braille Institute font files and include the needed weights locally with PLE.
  - Evidence (source): `src/styles/browser_fonts.css` `@font-face` loads local Atkinson Hyperlegible Next variable font files.

- [x] When a narrow font is needed, use `IBM Plex Sans Condensed` for long unbreakable strings such as URLs.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-narrow` declares the local `IBM Plex Sans Condensed` face and applies it only to the PLE Question JSON editor's Citation URL input; `pipeline/build.mjs` `BROWSER_FONT_BUNDLES` copies and verifies that same-origin asset.
  - Evidence (runtime): `src/features/ple_question_json_authoring/question_json_editor_styles.ts` `PLE_QUESTION_JSON_EDITOR_STYLES`: a one-time Chromium fixture imported the actual injected editor CSS against production-built local font assets on 2026-09-15; at 360 and 1280 CSS pixels in light and dark OS preferences it requested the local font, computed the narrow family on the Citation URL input, and retained normal input value, horizontal-scroll, and overflow behavior.

- [x] With `IBM Plex Sans Condensed`, try `font-variant-numeric: slashed-zero` to better distinguish `0` from `O`.
  - Evidence (source): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero` applies it to that narrow Citation URL input; `src/assets/fonts/ibm_plex_sans_condensed/provenance.txt` records the locally retained IBM Plex Sans Condensed Regular asset, OFL provenance, and its verified OpenType `zero` GSUB feature.
  - Evidence (runtime): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero`: that same one-time Chromium fixture computed `slashed-zero` on the local IBM face without a fallback; it was removed rather than retained as a permanent implementation-coupled test.

- [x] Question Backend-rendered content may use its own fonts when needed for correct display.
  - Evidence (source): `src/styles/ple_embed.css` `Question-authored font rules remain in charge` keeps the readable baseline and leaves later Question, math, and icon families in effect.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Question Backend-rendered content may use its own fonts when needed for correct display` kept Atkinson on ordinary text and plain fields, STIX Two Text on the later Question rule, MathJax_Main on the math element, and Question Widget on the classed answer field.

### Ribbon and page layout
- [x] The top Ribbon is the persistent navigation area for signed-in PLE pages.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` renders `AppRibbon` inside the persistent shell.

- [x] The Ribbon should remain in the same location and use the same overall structure while navigating.
  - Evidence (test): `tests/playwright/ribbon_responsive_evidence.mjs` `assertResponsiveRows` verifies declared Ribbon rows across route-model changes.

- [x] Navigation choices should remain in predictable locations as users move between related pages.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` and `RIBBON_TASK_CATALOG` own fixed navigation identities.

- [x] Each Tier 1 choice has one Tier 2 set. If Tier 1 stays the same, Tier 2 stays the same. Opening a Course, Assessment, Question, or other item does not change the Tier 2 choices or their order.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` declares one Tier 2 set for each Tier 1 area, independent of the open route.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `settled Instructor Tier 2 destinations and order stay fixed across deeper routes` compares Instructor rows for the same Tier 1 area.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Tier 2 choices and order stay fixed across routes and Course context` compares Student rows while opening Course and Coursework routes.

- [x] Changing a Ribbon selection should change the content below the Ribbon without moving the main content area up or down.
  - Evidence (source): `src/application_shell.tsx` `ContentRegion` keeps `BreadcrumbPrelude` in place and swaps the routed page when `pathname` changes.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `shell geometry drifts` compares breadcrumb and main-content tops across signed-in routes.

- [x] Ribbon rows should keep their space when needed so changing selections does not make the content area jump.
  - Evidence (source): `src/ribbon/app_ribbon.css` `--ple-ribbon-reserved-task-size` keeps the task row in the Ribbon grid.

- [x] Page actions should appear near the content they affect rather than changing the Ribbon layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` renders only catalog navigation and Sign Out in `AppRibbon`; task content stays in `ApplicationShell` content.

- [x] See **User top bar** and **Breadcrumbs** for the persistent elements that make up the top of the page.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` composes `AppRibbon` and `BreadcrumbPrelude`.

### User top bar interface
- [x] All signed-in users share the same top-left logo/account and top-right profile bar layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `AppRibbon` renders the shared leading `ple-app-ribbon__context-identity` logo/account block and shared trailing `ple-app-ribbon__profile-endcap` Profile control for every role model; `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile-endcap` pins the Profile control at the inline end, while Student narrow rules adapt only the middle navigation arrangement.

- [x] The top bar remains in a consistent location as users navigate.
  - Evidence (test): `tests/playwright/ribbon_responsive_evidence.mjs` `assertResponsiveRows` measures the persistent top row across model changes.

- [x] The PLE logo and product name appear at the upper left and link to the user's home dashboard.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__brand` is the upper-left Peptidyle home link immediately after the role badge.

- [x] Each User Role has its own home dashboard and navigation.
  - Evidence (source): `src/pages/role_home_pages.tsx` `InstructorHomePage` opens the Instructor dashboard, `StudentHomePage` opens Student Coursework, and `SysadminHomePage` opens system administration.
  - Evidence (source): `src/route_contract.ts` `userRoleHomeRouteId` selects a different home route for each User Role.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_ONE` declares separate Tier 1 navigation for each User Role.

- [x] User Role appears once next to the PLE name.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__user-role` occurs once in the shared identity block beside `ple-app-ribbon__brand`.

- [x] Role-specific navigation appears between the product identity and Profile.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` places `ple-app-ribbon__tabs` after identity and before account controls.

- [x] Profile appears at the far right as an icon-only avatar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__profile-endcap` renders the shared icon-only Profile button after the navigation region; `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile-endcap` anchors that endcap at the inline end.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in User Role has one accessible generic Profile end control` verifies the one accessible, text-free Profile control for Student, Instructor, and Sysadmin.
  - Decision: A one-time real-shell probe verified the isolated Profile control at 1280 and 320 CSS pixels with a coarse pointer, including thumbnail-request isolation; it was removed rather than retained as a permanent browser test.

- [x] Clicking the Profile avatar opens the Profile menu.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `openProfileMenu` controls the Profile trigger's `profileMenuOpen` state and renders the labelled `ple-profile-menu` menu.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` protects pointer and keyboard opening, focus, dismissal, and action dispatch.

- [x] I want the Profile menu to contain Profile and Sign Out.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-profile-menu` renders the Profile link and the Sign out action.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` checks that the open menu contains those two commands.

- [x] I want Profile to be the only page that names the Account's time zone; other pages should show times
  without repeating the zone name.
  - Evidence (source): `src/pages/profile_page.tsx` `profile-time-zone-heading` names the Account time zone.
  - Evidence (source): `src/format_datetime.ts` `createDisplayDateTimeFormatter` formats a time without a zone name.
  - Evidence (test): `tests/test_pending_invitations_model.mjs` `invitation expiry uses the explicit viewer zone` checks that the shown expiry omits the zone name.

- [x] Sign Out belongs in the Profile menu rather than the main top bar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `data-ribbon-action={props.model.context.signOutAction.id}` renders Sign Out as a Profile-menu item and closes that menu after dispatch.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` verifies no top-bar Sign Out button and one dispatched Profile-menu Sign Out action.

- [x] See **Ribbon and page layout** for the overall navigation and page-position rules.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` is the shared shell that composes the top bar and content region.

### Profile avatar interface
- [x] Every Account is randomly assigned an avatar from the PLE avatar gallery when the Account is created.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `record_initial_account_avatar` chooses one selectable gallery avatar when an Account row is inserted.

- [x] The same avatar gallery collection is available to all User Roles.
  - Evidence (source): `src/pages/profile_page.tsx` `ProvidedAvatarPicker` presents the gallery for the signed-in Account.
  - Evidence (source): `src/features/profile_avatar/provided_avatar_picker.tsx` `selectableEntries` reads the one selectable catalog.

- [ ] The current avatar or Profile image appears consistently anywhere PLE represents that user.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.
  - Owner: 03_shell.md / Profile avatar interface (first occurrence; identical requirement and status).

#### Student avatars
- [x] **Students** select avatars from the PLE-provided avatar gallery collection and cannot upload Profile images.
  - Evidence (source): `src/pages/profile_page.tsx` `ProvidedAvatarPicker` is the Student gallery, and `profileRoleMayManageImage` keeps image upload off that page.
  - Evidence (source): `src/features/profile_avatar/profile_avatar_role.ts` `profileRoleMayManageImage` admits image controls for Instructor and Sysadmin only.

- [x] Student avatar selection should be visual and playful.
  - Evidence (source): `src/features/profile_avatar/provided_avatar_picker.tsx` `RecordListImageBrowser` shows the gallery, and the introduction says the choice is playful.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list--gallery .record-list__media` gives each avatar a visual tile.

- [x] All avatars in the gallery are available for selection.
  - Evidence (source): `src/features/profile_avatar/provided_avatar_picker.tsx` `selectableEntries` offers each selectable catalog avatar as a radio.

- [x] Students may select another avatar at any time.
  - Evidence (source): `src/pages/profile_page.tsx` `selectProvidedAvatar` saves a new provided avatar from the Profile gallery.

#### Instructor and Sysadmin Profile images
- [x] **Instructors** and **Sysadmins** share the same Profile backend and functionality.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `prepare_account_profile_image` admits an Instructor or a Sysadmin to the same image preparation.
  - Evidence (source): `src/pages/profile_page.tsx` `ProfilePage` is the one Profile surface for every signed-in role.

- [x] **Instructors** and **Sysadmins** may select from the PLE avatar gallery or upload their own Profile image.
  - Evidence (source): `src/pages/profile_page.tsx` `StaffAvatarSettings` is shown when `profileRoleMayManageImage` is true.

- [x] Image upload accepts any aspect ratio with a minimum of 128 pixels in both dimensions.
  - Evidence (source): `src/api/decoders/profile_avatar.ts` `decodeProfileImageCropInput` requires both source sides to be at least 128 pixels.
  - Evidence (source): `src/features/profile_avatar/profile_image_crop.ts` `decodeProfileImage` applies that same minimum to the decoded image.
  - Evidence (test): `tests/test_profile_image_crop.mjs` `Profile crop preserves a square and reaches the chosen edges at each zoom` accepts wide and tall sources whose shorter side is 128 pixels.
  - Evidence (test): `tests/test_profile_image_crop.mjs` `Profile crop rejects undersized, oversized, and invalid crop inputs` rejects a side below that minimum.

- [x] After upload, Instructors and Sysadmins can position and crop the image within a square Profile preview.
  - Evidence (source): `src/features/profile_avatar/staff_avatar_settings.tsx` `PROFILE_IMAGE_SIDE_PIXELS` sizes the Profile preview canvas to a square.
  - Evidence (source): `src/features/profile_avatar/profile_image_crop.ts` `profileImageCrop` returns one square region from the chosen position and zoom.
  - Evidence (test): `tests/test_profile_image_crop.mjs` `Profile crop preserves a square and reaches the chosen edges at each zoom` moves that square to the source edges.

- [x] Instructors and Sysadmins may replace their Profile image or select a provided avatar at any time.
  - Evidence (source): `src/pages/profile_page.tsx` `selectProvidedAvatar` saves a gallery avatar from Profile while a Profile image can already be current.
  - Evidence (source): `src/features/profile_avatar/staff_avatar_settings.tsx` `replaceImage` submits a new Profile image through the signed-in Account.

- [ ] The current avatar or Profile image appears consistently anywhere PLE represents that user.
  - Verification pending: Current Human Guidance requirement has no independently accepted implementation proof; audit the current Profile avatar interface boundary.
  - Owner: 03_shell.md / Profile avatar interface (first occurrence; identical requirement and status).

### Breadcrumbs interface
- [x] All signed-in users have a permanent breadcrumb row below the top Ribbon.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` is the first region inside `ContentRegion`, under `AppearanceRibbon`.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-ribbon-shell-grid` keeps the Ribbon in the first shell row.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `shellGeometry` requires the breadcrumb row on every signed-in shell materialization.

- [x] The breadcrumb row remains in the same location and keeps the same space as users navigate.
  - Evidence (source): `src/ribbon/app_ribbon_density.css` `.ple-shell__breadcrumb-prelude` reserves one fixed block size for the breadcrumb row.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `shell geometry drifts` fails when that row or the main content moves between signed-in routes.

- [x] Breadcrumbs show the path from the user's home dashboard to the current page.
  - Evidence (source): `src/ribbon/ribbon_contract.ts` `breadcrumbsFor` starts each signed-in trail at the role home.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in route reserves linked breadcrumbs rooted at its role home` checks that the first crumb is the role home and the last crumb is the current route.

- [x] Each breadcrumb level links back to its corresponding page.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` renders every crumb through `breadcrumb.href`.
  - Evidence (source): `src/ribbon/ribbon_contract.ts` `RibbonBreadcrumbModel` requires `href` on every crumb.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in route reserves linked breadcrumbs rooted at its role home` checks that each crumb href is a declared route.

- [x] Breadcrumbs use human-readable names rather than internal identifiers.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` renders `RibbonBreadcrumbModel.label`, not an ID.

- [x] Course and Assessment breadcrumbs preserve the current Course context.
  - Evidence (source): `src/ribbon/route_scope_controller.ts` `createRouteScopeController` resolves route labels while retaining course scope.

- [x] Keeping the breadcrumb row in place prevents the main content from moving up or down as breadcrumb depth changes.
  - Evidence (test): `tests/playwright/ribbon_shell_evidence.mjs` `label resolution preserves the reserved breadcrumb-prelude geometry` verifies stable shell geometry through deferred resolution.

- [x] See **Ribbon and page layout** for the overall page-position rules.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` composes the Ribbon and the breadcrumb row in one shell.
### Instructor interface
- [ ] The Instructor interface should make frequent teaching tasks fast and easy to find.
  - Mismatch: the main Instructor task areas still contain deferred destinations and no end-to-end usability evidence establishes this broad workflow claim.

- [x] Keep the teaching content central in authoring and inspection workflows, with metadata and supporting explanations arranged compactly around it.
  - Evidence (source): `src/pages/question_detail_page.tsx` `aria-label="Question prompt"` renders the prompt before the Question Description and the metadata strip.
  - Evidence (source): `src/pages/question_detail_page.css` `question-detail-description` keeps that explanation compact, with no card padding, fill, or shadow.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx` `Student-facing prompt` stays in the authoring column, and Question Library metadata sits with the preview.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_styles.ts` `editor-grid` places the preview and library metadata beside the prompt.
  - Evidence (test): `tests/playwright/test_teaching_content_central.mjs` `TEACHING_CONTENT_SENTENCE` rendered the shipped Question inspection page and private Draft editor in headless Chromium at 1280 by 800. The prompt led the description and metadata, and the description had no card padding, fill, or shadow. The Draft preview and Question Description for Instructors sat beside the prompt. The check covers those two workflows and does not measure every authoring or inspection workflow. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Gradebook rows should identify Students by their Course roster names and Coursework by title, with IDs as supporting information where useful.
  - Evidence (source): `src/pages/gradebook_page.tsx` `gradebookRowHeader` shows the roster name with the roster ID in supporting text, and the Coursework cell shows the Assessment title.

- [x] The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `productAssessments` labels the third top-bar tab Assessments.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks the Courses, Questions, and Assessments tab labels.

- [x] Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
  - Evidence (source): `src/features/profile_avatar/ribbon_account_avatar.tsx` `RibbonAccountAvatar` falls back to `RibbonIcon` `circle-user` when no Profile image or provided avatar exists.

- [x] All required Ribbon choices remain visible even when their collection is empty.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` declares each required Tier 2 choice without reading collection size.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `settled Instructor Tier 2 destinations and order stay fixed across deeper routes` keeps those choices on every Instructor route in the Tier 1 area.

- [x] Required Instructor Ribbon choices whose workflows are unavailable remain visible as unavailable, annotation-free controls. They become usable only when their workflow exists.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `UnavailableRibbonChoice` keeps the choice visible, disabled, and labeled without a navigation href.
  - Evidence (test): `tests/e2e/e2e_ribbon_app_component.mjs` `required Instructor choices remain visible without inventing unfinished routes` checks that unavailable choices stay visible and available choices keep their routes.

- [x] A working navigation destination remains visible when its collection is empty.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` retains the courses route and renders an empty state.

- [x] A future or unavailable capability should not appear as a usable control until its workflow exists.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `RibbonDestination` distinguishes `route` from `future` destinations.

- [x] Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` explains the shared Published Question collection and shows Create a Draft Question only when mayMutateLibrary is true.

- [x] Similar pages should place similar actions in consistent locations.
  - Evidence (source): `src/components/record_list/record_list.tsx` `record-list__actions` places each record action after the title and facts.
  - Evidence (source): `src/pages/course_list_page.tsx` `Open Course` is the Course list open action in that shared slot.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `label: "Open"` is the Question Library open action in that same slot.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Similar pages should place similar actions in consistent locations.` rendered the shipped My Active Courses and Question Library pages in headless Chromium at 1280 by 800. Open Course and Open both sat in the shared actions slot, below the title and facts, with the same inset from the record content. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor pages should be composed around the teaching task rather than collections of padded components.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_authoring.css` `assessment-editor-panel` keeps the subtle panel radius and removes the padding, fill, and shadow around the teaching task.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_authoring.css` `assessment-editor-actions` places the task buttons in a strip without a card.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace.css` `assessment-editor-policy-panel` keeps Assessment Properties in flat groups.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace.css` `assessment-create-method` names the create choice without a padded box.
  - Evidence (source): `src/pages/course_instance_page.tsx` `course-instance-page__assessments` puts the ordered Assessments ahead of Course administration.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `Ordered Assessment Entries` is the Question editor list ahead of adding Questions.
  - Evidence (test): `tests/playwright/test_teaching_task_pages.mjs` `TEACHING_TASK_SENTENCE` rendered the shipped Course editor, Create Assessment form, and Assessment Question editor in headless Chromium at 1280 by 800. The Assessment list leads the Course page. The create choice and Question editor sections have no card padding, fill, or shadow, and the Question editor panel keeps a subtle radius below the button. The check does not measure every Instructor page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards.
  - Evidence (source): `src/components/record_list/record_table.tsx` `RecordTable` places each Gradebook and roster record on one table row under named columns.
  - Evidence (source): `src/components/record_list/record_list.css` `record-list__row--semantic` keeps each Question Library record a full-width row with a divider and no card radius.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Instructor lists and repeated records should be dense and easy to scan, more like a spreadsheet than cards.` rendered the shipped Gradebook, Course roster, and Question Library in headless Chromium at 1280 by 800, kept those records in one full-width column with square corners, no record shadow, and a hairline divider, and showed several Library titles in the list window. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage` renders the answer-free cue, loads the manifest and selected Question, and disables the native response preview.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_student_view.sql` `load_instructor_student_view_question_source` is a stable read of one authorized Question source.
  - Evidence (source): `crates/server/src/assessment_student_view.rs` `assessment_student_view_router` registers only GET routes for the manifest, presentation, and document.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.` opened the shipped Student View in headless Chromium, showed the peptide-bond prompt and the answer-free cue, kept the response preview disabled and inert, and recorded only the workspace read plus the Student View manifest and Question reads. No Save response, Submit, or Start attempt control was present. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Instructor lists and repeated records should favor compact rows or tables with clear columns over cards or loosely concatenated text.
  - Evidence (source): `src/components/record_list/record_table.tsx` `scope="col"` names each Gradebook and roster column.
  - Evidence (source): `src/components/record_list/record_list.css` `repeat(auto-fit, minmax(min(100%, 8rem), 1fr))` places each Question Library fact in an equal column.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `Instructor lists and repeated records should favor compact rows or tables with clear columns over cards or loosely concatenated text.` rendered the shipped Gradebook, Course roster, and Question Library in headless Chromium at 1280 by 800, lined each table cell up with its column header, and kept Library facts in equal columns that align across records. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] At 1280 x 800, Instructor pages should expose enough of the current workflow to minimize unnecessary scrolling.
  - Evidence (source): `src/pages/gradebook_page.tsx` `GradebookCoursePage` places the score download and the score table in the Gradebook workflow.
  - Evidence (source): `src/pages/course_roster_page.tsx` `current-roster-heading` places the roster table in the Students workflow.
  - Evidence (source): `src/pages/library_page.css` `grid-column: 2` places the Question Library result window beside the filters at laptop width.
  - Evidence (test): `tests/playwright/fast_ui_route_composition.mjs` `At 1280 x 800, Instructor pages should expose enough of the current workflow to minimize unnecessary scrolling.` rendered the shipped Gradebook, Course roster, and Question Library in headless Chromium at 1280 by 800 and kept each heading, first record, and workflow action inside that viewport. No Live Demo stack was started. No PostgreSQL proof was run.

#### Course interfaces
- [x] The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `myBlueprintCourses` is one of the four Courses-area route destinations.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks those four Courses labels.

- [x] Course lists should support scanning and comparison without opening each Course.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` puts the Course long name, classification, and term on each list row beside Open Course.

- [x] The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseInstancePage` lists the Course and its Assessments in Course order.

- [x] Selecting an Assessment in the Course Editor opens that Assessment for editing.
  - Evidence (source): `src/pages/course_instance_page.tsx` `assessmentQuestionsPath` points Edit Assessment at that Assessment's Question editor.

- [x] Assessment content and Assessment properties should remain separate editing tasks.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `AssessmentWorkspaceQuestionsView` is the Question editor, and `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` is the Properties editor.

- [x] My Active Courses and My Inactive Courses should both be available from the Courses area.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `myActiveCourses` and `myInactiveCourses` are route destinations in the Courses area.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks both labels on the Instructor Courses row.

- [x] Course Discipline selection should provide a clear way to request a new Discipline when the needed
  Discipline is unavailable.
  - Evidence (source): `src/components/course_classification_fields.tsx` `DisciplineRequestControl` asks for a Discipline the Course selector does not offer.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `request_content_discipline` stores the requested name for an Instructor or Sysadmin.
  - Evidence (source): `src/pages/content_disciplines_page.tsx` `ContentDisciplinesPage` lists open requests with the requester Account ID.
  - Evidence (source): `src/components/discipline_request.ts` `createDisciplineFromRequest` calls createDiscipline and then resolveDisciplineRequest.
  - Evidence (test): `tests/test_frontend_contract.mjs` `Course Discipline selection requests a new Discipline without creating it` recorded trimmed Genetics through requestContentDiscipline.
  - Owner: Course interfaces (first occurrence).

##### Blueprint Course interface
- [x] **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCoursesWorkspace` presents reusable Blueprint Course content and adoption information.

- [x] **Search Public Blueprint Courses** helps Instructors find relevant Blueprint Courses in a growing shared collection.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` searches public Blueprint Courses by name and classification.

- [x] Public Blueprint Course search should combine ordinary text search with shared classification
  filters beginning with Discipline and following Discipline -> Subject -> Topic -> Subtopic.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` keeps the name field and `BlueprintSearchClassification` on one search form.

- [x] Selecting a Discipline should limit Subject choices to Subjects associated with that Discipline.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `BlueprintSearchClassification` loads Subject choices for the selected Discipline.

- [x] After selecting a Subject, Instructors should have an explicit option to include Blueprint Courses
  associated with that Subject across its other Disciplines.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `BlueprintSearchClassification` shows Include this Subject across Disciplines only after a Subject is selected.

- [x] Tags should provide additional filters outside the hierarchy.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` places the Tag field outside `BlueprintSearchClassification`, and `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` matches that exact tag outside the Discipline predicates.
  - Evidence (test): `crates/server/src/blueprint_course/list.rs` `public_search_tag_binds_the_request_and_rejects_a_different_tag` binds the trimmed tag.

- [x] Search results should use a compact, information-rich layout that supports scanning and comparison.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchResultList` keeps public Blueprint results on the semantic record list.

- [ ] Results should show Course name, classification, author, institution, and useful usage or
  stewardship signals directly in the result list to support scanning and comparison.
  - Mismatch: `src/pages/blueprint_course_search_page.tsx` `publicBlueprintContent` shows the Course name, classification, adoptions, and students. `crates/question_model/src/blueprint_course.rs` `BlueprintCourseSummaryView` has no author or institution field.

- [ ] Public Blueprint Course search should support sorting by relevant fields such as Stars, Watches, Adoptions, Students who have taken the Course, and most recent edit.
  - Mismatch: `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` sorts by name, adoptions, and students. `crates/question_model/src/blueprint_course.rs` `BlueprintCourseSummaryView` has no stars, watches, or last-edit field.

- [x] Search terms, active filters, and the selected sort should remain visible while reviewing results.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` keeps the search form, applied name, tag, classification, and sort beside the results.

- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
  - Owner: Blueprint Course interface (first occurrence).

- [x] Opening a result and returning should preserve the Instructor's search, filters, sort, and scroll
  position.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` restores the saved query, tag, classification, sort, cursor, and scroll.
  - Evidence (source): `src/pages/blueprint_course_search_return_state.ts` `saveBlueprintSearchReturnState` keeps that search snapshot for the same session.
  - Evidence (test): `tests/test_blueprint_search_return.mjs` `takeBlueprintSearchReturnState` returns the saved snapshot once to the same session.

- [x] A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` renders "Create Course Instance from this Blueprint."
  - Evidence (runtime): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` is exercised by accepted C47 compiled-main browser proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.C6RwpH/public-search-browser.json`, which follows the existing detail action and preselects the matched Blueprint beyond the first 50 search rows. It does not submit or create a Course Instance.

##### Blueprint Course editing interface
- [x] Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `onToggleEditor` opens the Course Editor from the overview and clears any selected Assessment.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `selectAssessment` opens that Assessment's editor from the Course Editor list.

- [x] The Course Editor should show the Blueprint Course structure without editing every Question on one page.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` lists modules and Assessments before selecting an editor.

- [x] Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` receives the selected Assessment content and renders its Questions and Properties tasks.
  - Evidence (runtime): accepted compiled-main, actual-loopback-HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json` opens one selected Assessment and exercises both tasks through `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor`.

- [x] Only the selected Blueprint Assessment's Questions should appear in its editor.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` renders entries from its selected `content` input only.
  - Evidence (runtime): accepted compiled-main, actual-loopback-HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.ay40bT/blueprint-properties-browser.json` verifies one selected Assessment, its separated Questions task, and an unchanged sibling after ordinary Save and exact reload through `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor`.

- [x] **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` selects through the Question picker, adds with confirmFixedQuestions, removes with removeEntry, and orders through its RecordSequence.

- [x] **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `updateReusableEntryScoring` sets points and the scoring rule beside the attempt limit, late work, and Student feedback fields.

- [x] Blueprint Courses should not contain Assessment dates or relative Assessment schedules.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` describes reusable structure without Students, deadlines, or course delivery settings; the reusable Blueprint content model has no delivery-date fields.

##### Course Instance interface
- [x] **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
  - Evidence (source): `src/pages/role_home_pages.tsx` `InstructorHomePage` opens the Active Course list, and `src/pages/course_list_page.tsx` `TeachingCourseListPage` keeps that home to Active Course Instances.

- [x] Active Course Instances should make upcoming Assessments and important course activity easy to find.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` lists upcoming Assessment titles, Courses, and due times for the Courses the Instructor teaches, and `src/ribbon/ribbon_catalog.ts` `assessmentsDueSoon` opens that page from the Assessments ribbon.

- [x] **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
  - Evidence (source): `src/pages/course_list_page.tsx` `InactiveCourseListPage` lists only Inactive Course Instances, separate from the Active home.

- [x] Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `validate_course_blueprint_adoption` requires adopted Assessments, Questions, pools, and settings to match the exact Revision.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` checks that adopted Course against the sealed Revision.

- [x] Assessments created from a Blueprint Course start unreleased with dates unset.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `append_course_assessments` stores null available, due, and close instants and leaves the Assessment unreleased.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` checks the unreleased status and the three null dates.

- [x] A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
  - Evidence (source): `crates/question_model/src/course_term.rs` `ensure_within_active_lifetime` rejects a term end after the UTC calendar date six months from creation and accepts that cutoff date.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` rejects a term end after that same UTC cutoff.
  - Evidence (source): `src/pages/course_list_page.tsx` `createCourseInstance` refuses that end date before creating the Course.
  - Evidence (test): `crates/question_model/src/course_term.rs` `active_lifetime_includes_the_cutoff_date_and_rejects_the_next_day` accepts the cutoff date, including a term longer than six months that ends on it, and rejects the next day.

- [x] Course banners use a 5:1 aspect ratio.
  - Evidence (source): `src/features/course_appearance/course_entry_banner.tsx` `aspect-ratio: 5 / 1` sizes the Course entry banner frame.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `the shipped course entry banner keeps a 5:1 ratio as its width changes` measures that rendered frame.

- [x] 1280 by 256 pixels is the recommended Course banner authoring size.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `AppearanceBannerEditor` tells the Instructor that 1280 by 256 pixels is recommended.

- [x] Higher-resolution 5:1 Course banner images are supported.
  - Evidence (source): `crates/objects/src/image_validation.rs` `verify_course_banner_still_image` accepts an exact 5:1 source with no minimum size.
  - Evidence (source): `crates/server/src/course_appearance/banner.rs` `stage_banner_upload` stages that verified source, and promotion scales it to the delivery rendition.
  - Evidence (test): `crates/objects/src/image_validation.rs` `course_banner_accepts_a_higher_resolution_five_to_one_source` accepts a source larger than the delivery rendition and scales it to that rendition.

- [x] PLE responsively scales Course banners while preserving their aspect ratio.
  - Evidence (source): `src/features/course_appearance/course_entry_banner.tsx` `CourseEntryBanner` frames the banner with `aspect-ratio: 5 / 1` and a viewport-bounded inline size.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `the shipped course entry banner keeps a 5:1 ratio as its width changes` measures the frame at three viewport widths.

- [x] Course banners appear as small centered banners rather than full-width page heroes.
  - Evidence (source): `src/features/course_appearance/course_entry_banner.tsx` `course-entry-banner-frame` caps the banner and centers it with `margin-inline: auto`.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `the shipped course entry banner keeps a 5:1 ratio as its width changes` checks that a wide viewport leaves the banner narrower than the page and centered.

- [x] An Instructor can upload a Course banner and select a Course Theme from the fixed theme catalog.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `saveBanner` uploads the chosen Course banner from the Instructor appearance page.
  - Evidence (source): `src/appearance/theme_chooser.tsx` `THEME_OPTIONS` offers every theme in the fixed catalog.
  - Evidence (test): `tests/playwright/course_appearance_m8_evidence.mjs` `bannerUploadCalls` saves a banner after the Instructor selects Ocean and Magma.

- [x] Course Instance Assessments have two editors:
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_overview_page.tsx` `Assessment Question Editor` links to the Question editor and `Assessment Properties Editor` links to the properties editor.

  - [x] **Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `Assessment Question Editor` is the Question editor title.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `appendAvailableFixedQuestion` adds a selected Question, `removeAssessmentEntry` removes one Entry, and `moveAssessmentEntry` reorders Entries.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `appendAvailableFixedQuestion` adds a selected Published Question.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `removeAssessmentEntry` removes only the chosen Entry.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `moveAssessmentEntry` changes Entry order.

  - [x] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `Assessment Properties Editor` edits dates, attempt limit, late work, and Student feedback.
  - Evidence (source): `src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx` `AssessmentFixedQuestionPointsEditor` edits fixed-Question points on that page.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `assessmentPolicySaveInput` keeps dates and saves the attempt limit, late-work rule, and Student feedback release.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `withFixedQuestionPointValues` changes only the chosen fixed-Question points.

#### Question interface
- [x] The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `RIBBON_TASK_CATALOG` includes those six Questions-area labels.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Instructor Product routes reserve owner-ordered task groups despite unavailable entries` checks those six Questions labels.

- [x] **My Questions** should make the Instructor's Published Questions easy to find and manage.
  - Evidence (source): `src/pages/my_questions_page.tsx` `MyQuestionsPage` lists the signed-in Instructor's Published Questions, links each one to its Question page, and pages them with RecordPageControls.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `myQuestions` opens that list from the Questions ribbon.
  - Evidence (test): `tests/test_library_classification_search.mjs` `My Questions continues the authored library page` loads the next authored page at 50, 100, or 250 and renders Previous, Next, and Records per page.

- [x] **My Draft Questions** should emphasize Questions that still need work before publication.
  - Evidence (source): `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` presents the current Instructor's drafts.

- [x] **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
  - Evidence (source): `src/pages/starred_questions_page.tsx` `StarredQuestionsPage` lists the signed-in Instructor's Starred Questions and links each one to its Question page.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `list_current_starred_questions` returns that Instructor's Stars, newest first.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `starred` opens that collection from the Questions ribbon.

- [x] **Watched** should help Instructors follow Questions where changes or activity matter to them.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `LibraryWatchNotificationsPage` lists changes and stewardship activity for Published Questions the Instructor watches.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `watched` opens that inbox from the Questions ribbon.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `getLibraryWatchNotifications` returns Revision, fork, and stewardship activity for a watched Question.

- [x] Published Questions should offer a **Create Pool from Question** action.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionPoolFromQuestionControl` offers Create Pool from Question on a Published Question.

- [x] Pool creation should show the starting Question and its Discipline and Subject alongside the
  Pool Title field.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` shows the starting Question title, Discipline, and Subject beside Pool Title.

- [x] Creating the Pool includes the starting Question and uses its Discipline and Subject.
  - Evidence (source): `src/components/question_pool_create_model.ts` `questionPoolMemberTuples` places the starting Question first and requires its Discipline and Subject.
  - Evidence (test): `tests/test_question_pool_source_binding.mjs` `questionPoolMemberTuples` keeps the starting Revision first.

- [x] Adding Questions to a Pool should begin with Question Library results filtered to the Pool's
  Discipline and Subject.
  - Evidence (source): `src/components/question_pool_create_model.ts` `questionPoolSourcePickerRepository` keeps Question Library rows from the starting Discipline and Subject.
  - Evidence (test): `tests/test_question_pool_source_binding.mjs` `questionPoolSourcePickerRepository` requests the starting Subject and drops a different Discipline.

##### Search Question Library interface
- [x] **Search Question Library** helps Instructors find specific Questions in a large library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies the searchable published Question Library.

- [x] Search should begin with a prominent search box, similar to Google Search.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` starts in `initial` state and applies `question-library-controls-initial` to the single `Search published questions` entry.
  - Evidence (runtime): one-time accepted compiled-browser exercise of `src/pages/library_page.tsx` `LibraryPage` confirmed idle Search makes no fetch and shows neither filters nor result rows; the temporary harness and landing screenshot were removed after the accepted proof.

- [x] The initial Search page should stay simple and focus attention on entering a search.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders filters and results only outside `initial` state, while `changeQuery` begins the current Search after input.
  - Evidence (runtime): one-time accepted compiled-browser exercise of `src/pages/library_page.tsx` `LibraryPage` confirmed the initial single-entry landing, then filters and 50 loaded results after entering `genetics`; the temporary harness and screenshots were removed after the accepted proof.

- [x] Search should assume the Instructor has some idea what they want to find.
  - Evidence (source): `src/pages/library_page.tsx` search input placeholder `Title or concept` directs a known-item search.

- [x] Question Library search should work well with ordinary words by default.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` passes ordinary `search` text to the Question Library query.

- [x] Instructors should not need to learn search syntax to use Search Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` exposes ordinary search and labeled filter controls without syntax requirements.

- [x] Search results should switch to a dense, information-rich layout.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `LibraryBrowseRows` shows the published Question list only after Search leaves initial.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordList` keeps those results on the semantic list unless a caller asks for gallery.

- [x] Results should make it easy to scan many Questions quickly.
  - Evidence (source): `src/pages/library_browse_record_list.css` `library-browse-record-list__window` keeps the current result page in one scroll region.

- [x] Results should show the information needed to judge relevance without opening each Question.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `questionDetails` shows authors, Discipline, Bloom, format when present, and the Question ID on each result.

- [x] Search terms and active filters should remain visible while reviewing results.
  - Evidence (source): `src/pages/library_page.tsx` `query` signal remains bound to the search input and filter selects while rows render.

- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
  - Owner: Blueprint Course interface (first occurrence).

- [ ] Should opening a result and returning preserve the Instructor's search and position.
  - Reason: product decision still unclear
  - Question: Must Question Library restore the Instructor's search and scroll after opening a result, or is that still an open design choice?
  - Mismatch: `src/pages/library_page_model.ts` `takeQuestionLibraryReturnState` already restores one saved query and scroll position. One reading treats that as required. The other keeps this sentence as an open question beside the hover-preview and new-tab note, so Human Guidance has not locked the requirement.

  - [ ] we should offer some hover preview and open items in a new browser tab by default
    - Reason: product decision still unclear
    - Question: Should a Question Library result show a hover preview and open in a new browser tab by default?
    - Mismatch: Question links open in the same tab and have no hover preview. One reading requires both. The other treats the sentence as an unsettled design note rather than a requirement.

- [ ] Advanced Search considerations:
  - Reason: product decision still unclear
  - Question: Does this bullet require a separate advanced-search mode, or does it only introduce the comparison notes that follow?
  - Mismatch: Question Library has one search box plus filters. One reading requires another advanced-search mode. The other treats the bullet as a heading for the notes below.

  - [ ] Simple and advanced searches could use the same search box, but we should seriously consider
    advanced search versus simple search interfaces forms.
    - Reason: product decision still unclear
    - Question: Should simple and advanced Question Library search share one box, or use separate forms?
    - Mismatch: `src/pages/library_page.tsx` `LibraryPage` uses one search box. One reading keeps that box. The other asks for a separate advanced form. Human Guidance says to consider both.

  - [ ] The interface should be minimal, show options by priority, not overwhelming to new users;
    - Reason: product decision still unclear
    - Question: Does this require a new minimal advanced-search form, or does the existing simple Search page already satisfy it?
    - Mismatch: The initial Search page is one search box. One reading asks for a further prioritized advanced form. The other treats the sentence as a design note under Advanced Search considerations.

  - [ ] Movie Lens as a tiered filter system https://movielens.org/explore/
    - Reason: product decision still unclear
    - Question: Must Question Library filters copy the MovieLens tiered explore layout?
    - Mismatch: No MovieLens-style explore layout is shipped. One reading requires that layout. The other treats the link as a comparison note, not a requirement to copy the product.

  - [ ] IMDB advanced search page is wel designed, https://www.imdb.com/search/title/ but questions
    would not be displayed as movie posters
    - Reason: product decision still unclear
    - Question: Must Question Library search copy the IMDb advanced title search, while keeping Questions off poster cards?
    - Mismatch: Results use a semantic list rather than posters. One reading requires an IMDb-style advanced form. The other treats the sentence as a comparison note.

  - [ ] Google advanced image search is more user friendly design https://www.google.com/advanced_search
    - Reason: product decision still unclear
    - Question: Must Question Library search copy Google advanced search?
    - Mismatch: No Google advanced-search form is shipped. One reading requires that form. The other treats the link as a comparison note.

  - [ ] Pubmed is clean, but not obvious to use https://pubmed.ncbi.nlm.nih.gov/advanced/
    - Reason: product decision still unclear
    - Question: Must Question Library search copy the PubMed advanced-search page?
    - Mismatch: No PubMed advanced form is shipped. One reading requires that page. The other treats the link as a comparison note.

  - [ ] Ebay is dated, but perhaps a useful comparison https://www.ebay.com/sch/ebayadvsearch
    - Reason: product decision still unclear
    - Question: Must Question Library search copy eBay advanced search?
    - Mismatch: No eBay advanced-search form is shipped. One reading requires that form. The other treats the link as a comparison note.

  - [ ] Should Question IDs have a preview image/movie poster style?
    - Reason: product decision still unclear
    - Question: Should a Question ID in search results include a preview image, or stay text?
    - Mismatch: `src/components/copyable_question_id.tsx` `CopyableQuestionId` shows the text Question ID. One reading adds a poster image. The other keeps the text ID.

##### Search Question Library filters
- [x] Search results should support filters for narrowing the Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies author, backend, tag, Question Type, license, and capability filters.

- [x] Classification browsing and filtering should begin with Discipline and follow the shared
  Discipline -> Subject -> Topic -> Subtopic hierarchy.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` offers Discipline, then Subject, Topic, and Subtopic.

- [x] Tags should provide additional filters outside the hierarchy.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `PublicBlueprintSearchPage` places the Tag field outside `BlueprintSearchClassification`, and `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` matches that exact tag outside the Discipline predicates.
  - Evidence (test): `crates/server/src/blueprint_course/list.rs` `public_search_tag_binds_the_request_and_rejects_a_different_tag` binds the trimmed tag.
  - Owner: Blueprint Course interface (first occurrence).

- [x] Selecting a Discipline should limit Subject choices to Subjects associated with that Discipline.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` loads Subject choices for the selected Discipline.
  - Owner: Blueprint Course interface (first occurrence).

- [x] After selecting a Subject, Instructors should have an explicit option to include Library Objects
  associated with that Subject across its other Disciplines.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` shows Include this Subject across Disciplines only after a Subject is selected.

- [x] Filters should update the current search rather than start a separate workflow.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` resets one `QuestionLibraryBrowseSession` with the updated query.

##### Search Question Library syntax
- [x] Search should support Google-like syntax for more precise queries.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `pub(super) struct QuestionTextQuery` accepts ordinary AND words, quoted phrases, minus exclusions, PLE field tags, and exact Question IDs with filters through its parser.
  - Evidence (runtime): accepted one-time private PostgreSQL and actual-server HTTP proof exercised `crates/server/src/question_library.rs` `search_questions` with ordinary AND words, a quoted phrase, minus exclusion, and all five PLE fields under an active vetted Instructor; anonymous and Student requests remained concealed and responses were `no-store`. The temporary proof is retained outside Git through documentation acceptance.

- [x] Quoted text should search for an exact phrase.
  - Evidence (source): `crates/server/src/library_search_terms.rs` `term_value` retains quoted text as one exact phrase term.
  - Evidence (runtime): historical C58 actual-server HTTP proof returned only `Alpha enzyme kinetics` for the quoted phrase `"enzyme kinetics"`; current locator `crates/server/src/library_search_terms.rs` `term_value` records the extracted behavior.

- [x] A minus sign should exclude matching terms.
  - Evidence (source): `crates/server/src/library_search_terms.rs` `exclusion_prefix` records a leading minus as an excluded search term.
  - Evidence (runtime): historical C58 actual-server HTTP proof returned `Alpha enzyme kinetics` for `enzyme -inhibitor` while excluding the matching inhibitor Question; current locator `crates/server/src/library_search_terms.rs` `exclusion_prefix` records the extracted behavior.

- [x] Search should support PubMed-like field syntax such as `discipline:biology` and
  `subject:genetics`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` parses discipline and subject prefixes into store terms.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `discipline:biology` and `subject:genetics` from the query text.

- [x] Classification field examples include `topic:"chromosomal inheritance"` and `tags:review`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` parses topic and tags prefixes into store terms.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `topic:"chromosomal inheritance"` and `tags:review` from the query text.

- [x] A Subtopic field example is `subtopic:"x-linked recessive crosses"`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` parses the subtopic prefix into a store term.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `subtopic:"x-linked recessive crosses"` from the query text.

- [x] Search fields should use PLE concepts and vocabulary.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` accepts discipline, subject, topic, subtopic, tags, type, and author.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` renders those prefixes back from the store terms.

- [x] Useful fields may include Discipline, Subject, Topic, Subtopic, Tags, Question Type, and author.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` maps type and author onto Question Type and author store fields.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `pubmed_style_fields_round_trip_through_the_store_terms` keeps `type:"multiple choice"` and `author:Ada` with the classification fields.

- [x] The interface should make useful search syntax discoverable when needed.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders `question-library-search-tips` as a native disclosure beside the ordinary Search box with words, quotes, minus, PLE fields, and examples.
  - Evidence (runtime): accepted corrected desktop `src/pages/library_page.tsx` `LibraryPage` component proof opened Search tips without obscuring filters or bulk controls; the full `./check_codebase.sh` gate passed.

- [ ] Search syntax should help expert users quickly narrow a very large Question Library.
  - Evidence (runtime): accepted C58 actual-server HTTP evidence proves the grammar through the production Store and route across a bounded 69-Question fixture.
  - Verification pending: the bounded proof does not establish usability or performance for a very large production Question Library.

##### Browse Question Library interface
- [x] **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
  - Evidence (source): `src/route_contract.ts` `libraryBrowse` is a distinct Instructor route and `src/pages/library_page.tsx` `LibraryPage` provides an overview-first grouped Browse mode without requiring Search text.
  - Evidence (runtime): accepted private full-app browser evidence exercised `src/main.tsx` `render` against the actual server through overview-first Browse, Biology, Enzymes, and focused Search without starting from Search text; every actual search response was `no-store`.

- [x] Browse should help Instructors understand what the Question Library contains.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` explains that counts cover all authorized matching Questions and presents subject, topic, tag, and Question Type groups.
  - Evidence (runtime): accepted C60 component evidence rendered `src/pages/library_page.tsx` `LibraryPage` overview groups; actual-server HTTP evidence exercised `crates/server/src/question_library.rs` `search_questions` over the full authorized matched snapshot.

- [x] Browse should begin with Discipline and make moving through Subject, Topic, and Subtopic easy.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` places `LibraryClassificationSearch` at the start of Browse.

- [x] Selecting a Discipline limits browsing to Subjects associated with that Discipline.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` loads Subject choices for the selected Discipline.

- [x] Browse should also offer Tags, Question Types, and other useful groupings.
  - Evidence (source): `src/pages/library_browse_controls.tsx` `LibraryBrowseControls` offers Subject, Topic, Tags, and Question Types groups.

- [x] Browse should show useful counts where they help Instructors choose where to explore.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` group choices render their server-owned counts and state that counts cover all authorized matches, not only loaded rows.
  - Evidence (runtime): accepted actual-server HTTP evidence exercised `crates/server/src/question_library.rs` `search_questions` with `page_size=1` yet returned Biology topic counts of Enzymes 2 and Metabolism 1 over all three matching Questions.

- [x] Browse results should use the same dense Question presentation used by Search where practical.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` gives Search and Browse the same virtualized `question-library-row` result renderer.
  - Evidence (runtime): accepted private full-app browser evidence rendered `src/pages/library_page.tsx` `LibraryPage` at 1280 by 800 with two shared dense Question rows, and root manager visual review accepted the initial Browse, narrowed Browse, and Search handoff screenshots.

- [x] Instructors should be able to move from browsing into a more focused search.
  - Evidence (source): `src/pages/library_page.tsx` `searchWithinResultsPath` serializes exact subject, topic, tag, and Question Type filters into the Search route.
  - Evidence (runtime): accepted routed component evidence exercised `src/pages/library_page.tsx` `LibraryPage` and preserved Biochemistry and Enzymes visibly and in subsequent repository requests through text editing and detail return, even when response facets omitted selected values.

- [x] Search and Browse are different paths into the same **Question Library**.
  - Evidence (source): `src/route_contract.ts` `ROUTE_CONTRACT` defines `library` and `libraryBrowse` as distinct Instructor routes backed by the same repository and `LibraryPage` row implementation; the Ribbon Browse task targets `libraryBrowse`.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `declared routes select only Ribbon topology and task areas that exist` and focused Ribbon, screenshot-manifest, and picker checks passed after the distinct Browse route was added.

#### Assessment interface
- [x] The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `assessmentsDueSoon` and `assessmentTemplates` are admitted route destinations with the required labels.
  - Evidence (runtime): accepted independent `src/ribbon/app_ribbon.tsx` `AppRibbon` and `src/ribbon/ribbon_contract.ts` `deriveRibbonModel` proof verified both actual Ribbon choices and routes.

- [x] **Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` presents upcoming deadlines across Courses the Instructor teaches.
  - Evidence (runtime): accepted private full-app evidence exercised `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` against actual HTTP and visibly emphasized the seven-day Account-zone window, release state, and Due group for each upcoming Assessment.

- [x] Assessment lists should make Course, release status, due date, and other important state easy to scan.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `DueSoonAssessmentRow` presents each Assessment with its Course, release state, and Account-zone Due; `src/pages/course_instance_page.tsx` `AssessmentRow` presents title, release state, and readable Account-zone Due under the Course heading.
  - Evidence (runtime): accepted private actual-server and exact-main browser proof exercised `src/pages/course_instance_page.tsx` `CourseInstancePage` alongside Due Soon. The first owned Course retained three readable rows at 1280px and 720px in a Los Angeles browser with a Chicago Account zone; a second owned Course showed three Assessment rows with Released/Unreleased states and distinct Due values matched row-by-row to its actual 200/`no-store` Assessment-list response. The Due Soon list separately showed Course identity on each cross-Course row. The fixture used privileged projection state for Released rows, not the release workflow.

- [x] **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
  - Evidence (source): `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` foregrounds reusable Assessment settings and excludes Questions, Pools, and Course dates.
  - Evidence (runtime): accepted independent `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` proof verified the heading, lede, legend, and normal Ribbon route without claiming HTTP, CRUD, or copy workflow acceptance.

- [x] Assessment editing has two editors:
  - Evidence (source): `src/route_contract.ts` `assessmentWorkspaceQuestions` and `assessmentWorkspacePolicies` declare distinct Assessment Question and Properties routes; the Ribbon tasks and breadcrumb use the same names.
  - Evidence (runtime): accepted private exact-main browser evidence navigated from `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` into the independently rendered Properties Editor.

  - [x] **Assessment Question Editor**: Selects, adds, removes, and orders Questions.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` provides Add, Move, Remove, and Save controls.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage`, adding two exact Questions, moving, removing, re-adding, saving, and reloading them in the persisted visible order.

  - [x] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` provides dates, instructions, Attempt/time limits, late work, order, disclosure, and focused fixed-Question point-value editing through `AssessmentFixedQuestionPointsEditor`.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx` `AssessmentFixedQuestionPointsEditor`: it saved `2.5` and `1` for two ordered exact Questions, reloaded the persisted values, preserved the other full-Assessment fields, exercised Cancel and Stay/Discard, and recovered from a real concurrent-write conflict by explicitly reloading and discarding the retained point draft.

- [x] The two Assessment editors should remain clearly distinct.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` and the Properties page have separate canonical routes, headings, Ribbon tasks, state models, and save operations.
  - Evidence (runtime): accepted private exact-main browser evidence showed `src/ribbon/ribbon_catalog.ts` `assessmentPolicies` as a distinct selected task, with Question order and Properties instructions each surviving their own actual-HTTP reload.

- [x] The Assessment Question Editor should make Question order easy to understand at a glance.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` renders the ordered entries as one numbered list with adjacent Move and Remove controls.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` with two visibly numbered Questions, changed their order, and reloaded the persisted order; root manager visual review and independent review accepted the rendered order.

- [x] Adding Questions should provide direct paths to Search and Browse Question Library.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` now renders direct Search and Browse Question Library links inside Available published Questions.
  - Evidence (runtime): accepted private actual-HTTP and exact-main browser evidence exercised `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` through both direct paths and browser Back; the unsaved-changes guard preserved a title draft on Stay and required deliberate Discard before navigation.

- [x] Instructors should be able to inspect a Question before adding it to an Assessment.
  - Evidence (source): `src/features/question_picker/question_picker_model.ts` `inspectQuestionPickerRow` returns the current selection and the result Revision without adding the Question.
  - Evidence (source): `src/features/question_picker/question_picker.tsx` `inspectQuestionPickerRow` opens that inspection inside the picker, and Confirm adds only the selection.
  - Evidence (source): `src/features/question_picker/question_picker_inspection.tsx` `QuestionPickerInspection` shows the answer-free prompt and states that inspection does not add the Question.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `getQuestionRevision` loads that inspection for the Assessment Question Editor.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `loadQuestionInspection` passes the same inspection into the Assessment picker and the Pool picker.
  - Evidence (source): `src/api/decoders/question_library.ts` `decodeQuestionDetails` rejects source and grading fields.
  - Evidence (test): `tests/test_question_picker.mjs` `inspectQuestionPickerRow keeps the current selection and QuestionPickerInspection shows the prompt without adding the Question` kept 7K3M-79QP selected, rendered Enzyme active site, and withheld Add selected Questions and Add pinned Question.

- [x] Instructors should be able to quickly add questions by Question ID to an assessment in bulk.
  - Evidence (source): `src/pages/assessment_workspace/assessment_question_id_batch.ts` `parseQuestionIdBatch` accepts a paste of canonical Question IDs and rejects any token that is not a Question ID before lookup.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `addQuestionsById` resolves each ID with `resolveQuestion` and adds only an available current Revision, within the 250 Question limit.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_view.tsx` `Add by Question ID` is the paste control beside Choose published Questions.
  - Evidence (test): `tests/test_assignment_workspace_questions.mjs` `Question ID paste adds the canonical batch and withholds invalid or missing IDs` collapsed a repeated `7K3M-79QP`, withheld missing `2R5X-E7YA`, refused `not-an-id` before lookup, and refused a paste larger than the remaining capacity.

- [x] Assessment Properties should group related settings so important settings are easy to find.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` groups Assessment and delivery controls separately from Student feedback, and the owning CSS uses a two-column desktop grid that collapses to one column below 60rem.
  - Evidence (runtime): accepted private exact-main browser evidence rendered `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` at 1280 by 800; computed-style checks verified the two-column desktop and one-column 720px layouts, restored panel/control styling, and persisted edited instructions through actual HTTP and reload.

- [x] Present timing settings in familiar units such as minutes, with explicit units and clear
  meanings for optional or unlimited values.
  - Evidence (source): `src/assessment_duration.ts` `assessmentDurationDefaultDescription` states the calculated default in minutes.
  - Evidence (source): `src/assessment_duration.ts` `OPTIONAL_DURATION_OVERRIDE_GUIDANCE` names a blank duration override, `NO_OPENING_TIME_GUIDANCE` names a blank opening time, `NO_CLOSING_TIME_GUIDANCE` names a blank close, and `UNLIMITED_ATTEMPTS_GUIDANCE` names a blank Attempt limit.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` shows those meanings beside the timing controls. A Quiz or Exam keeps one Attempt.
  - Evidence (test): `tests/test_assessment_duration.mjs` `timing settings name minutes, a blank override, no closing time, and unlimited Attempts` checked the minute default and those blank meanings, and checked that the properties page renders the guidance constants.

- [x] Instructors can randomize Question order for an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `updateOrder` binds the current Assessment Properties checkbox to authored or shuffled Question order. The Student start transaction persists that rule with the Attempt and shuffles the complete fixed-and-Pool issued vector only for `shuffled`.
  - Evidence (runtime): accepted private actual-HTTP evidence exercised `crates/learning-data-access/src/postgres/assessment_delivery_start.rs` `start_current_assessment_attempt`: it persisted authored and shuffled rules, observed a concurrent Instructor save blocked behind the Student-start lock, issued exact fixed-and-Pool pins, and resumed the immutable shuffled Attempt after a current-rule edit. Accepted exact-main browser evidence saved and reloaded `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s visible **Randomize question order** checkbox through actual HTTP.

- [x] Answer-choice randomization belongs to the Question, not the Assessment.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_model.ts` `setChoiceRandomization` and the strict Question codec own `randomizeChoices`; `crates/adapters/ple/src/question_json/source_document.rs` `compile_choices` compiles it to `NativeChoiceOrder`, and `crates/question_model/src/presentation/builder.rs` `pending_items` applies its nonce-derived choice permutation. The closed Assessment activity rules contain Question-order policy but no answer-choice override.
  - Evidence (runtime): accepted exact-main browser evidence preserved `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s explicit Question-owned choice-order explanation while saving and reloading the independent Assessment Question-order rule. This verifies the ownership boundary without claiming a runtime matrix of every native choice permutation.

- [x] **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` calls `listAssessmentsDueSoon` and states its across-Courses scope.
  - Evidence (runtime): accepted private actual-server evidence exercised `schemas/base_schema/50_functions/assessment_operations.sql` `list_assessments_due_soon` across two owned Courses and one outsider Course; each Instructor saw only their own Course rows, while anonymous and Student requests received the same concealed response.

- [x] Assessments Due Soon shows the Course and due time for each Assessment.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `dueSoonContent` shows the Course long name and the formatted due time on each row.

#### Assessment type appearance
- [x] Each Assessment Type has its own PLE-defined Font Awesome icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` assigns one required Font Awesome icon name and bundled glyph to every canonical Type.

- [x] Assessment Type icons remain consistent across PLE themes.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` owns the theme-independent Type-to-icon mapping.

- [x] Each Assessment Type also has its own theme-defined color.
  - Evidence (source): `src/styles/assessment_types.css` `--ple-assessment-type-regular-assignment` defines the five semantic Type color properties at the root and Course-theme scope.

- [x] Themes may change Assessment Type colors but preserve the meaning of each Type.
  - Evidence (source): `src/styles/assessment_types.css` `html[data-display-mode="dark"]` changes the five Type color values.
  - Evidence (source): `src/assessment_type_presentation.ts` `assessmentTypePresentation` keeps the Type icon and label when those colors change.

- [x] Assessment Type should never be communicated by color alone.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordFactView` shows the Type icon and label inside the colored Type fact.

- [x] Icons and labels should remain sufficient to identify the Assessment Type without color.
  - Evidence (source): `src/assessment_type_presentation.ts` `assessmentTypePresentation` returns the canonical Type label and bundled icon metadata independently of color.

- [x] **Weekly Assignment** uses the Font Awesome `pen-to-square` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Weekly Assignment to `pen-to-square`.

- [x] **Unit Review Assignment** uses the Font Awesome `arrows-spin` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Unit Review Assignment to `arrows-spin`.

- [x] **Bonus Assignment** uses the Font Awesome `star` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Bonus Assignment to `star` and the genuine bundled glyph.

- [x] **Quiz** uses the Font Awesome `circle-question` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Quiz to `circle-question` and the genuine bundled glyph.

- [x] **Exam** uses the Font Awesome `file-signature` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Exam to `file-signature` and the genuine bundled glyph.

#### High-consequence actions
- [x] Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionArchiveControl`, `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`, and `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `BlueprintCourseLifecycleControls` place those three actions in Danger Zone controls.

- [x] Danger Zone should be visually separate from ordinary editing actions.
  - Evidence (source): `src/pages/question_detail_page.css` `question-archive-danger-zone`, `src/pages/assessment_workspace/assessment_workspace.css` `assessment-workspace-unrelease-danger-zone`, and `src/features/blueprint_course/blueprint_course.css` `blueprint-course-archive-danger-zone` each use the danger border and danger-tinted background.

- [x] Assessment Unrelease should explain that Student work will be deleted.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-unrelease-confirmation-help` says Unrelease permanently deletes the represented Student Work, and the action is labeled "Unrelease and delete Student Work."

- [x] Assessment Unrelease should require typing the Assessment title before confirmation.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `confirmationTitle` disables Unrelease unless the entered value exactly matches the current Assessment title, while `schemas/base_schema/50_functions/unrelease.sql` `ple_api.unrelease_assessment` independently rejects a nonmatching confirmation title.

- [x] Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionArchiveControl` explains that archiving removes the Published Question from new selection and disables confirmation until the exact title, and `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `BlueprintCourseLifecycleControls` does the same for the Blueprint Course long name.

- [x] Restore actions should use ordinary availability controls.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `BlueprintCourseLifecycleControls` presents Restore alongside ordinary availability actions, without the Archive name-confirmation input; `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `restore` performs the existing authorized availability change.
### Student interface
- [ ] Guidance about the student interface
  - Reason: product decision still unclear
  - Question: Does this bullet require a Student guidance surface, or does it only introduce the Student interface rules that follow?
  - Mismatch: the bullet names no Student-facing behavior. One reading treats it as the heading for the following Student rules; the other would require a guidance surface Human Guidance does not describe.

#### General Student interface
- [x] The Student interface should focus on current Courses, Coursework, and work that needs attention.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` lists current courses; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` lists assigned work.

- [x] **Coursework** is the Student-facing collective term for Weekly Assignments, Unit Review Assignments, Bonus Assignments, Quizzes, and Exams.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` names those five Types, and `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student collection Coursework.

- [x] Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `assessmentContent` puts `assessmentTypePresentation` on each item and in its open action.

- [x] The Student interface should make the next useful action easy to find.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` puts Open Course on each current Course.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentList` puts the typed Coursework action on each item.

- [x] Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
  - Evidence (source): `src/components/page_frame.css` `width: min(100%, var(--ple-reading-max-inline))` keeps Student page content within the viewport width.
  - Evidence (source): `src/components/record_list/record_list.css` `overflow-wrap: anywhere` keeps Course and Coursework rows inside the list width.
  - Evidence (test): `tests/playwright/student_course_entry_m6_evidence.mjs` `Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.` rendered the shipped Your courses list, All Coursework page, and Course landing in headless Chromium at the laptop, portrait-tablet, narrow-phone, and square viewports, kept Open Course and Resume Weekly Assignment inside each width, and hit those actions. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student layouts should adapt smoothly at intermediate widths, with readable long titles and controls that wrap or rearrange in the task's reading order.
  - Evidence (source): `src/components/page_frame.css` `overflow-wrap: anywhere` keeps a long page title inside the content width.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-action-region` places the primary action before the secondary action and stacks that region at max-width 56rem.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Student layouts should adapt smoothly at intermediate widths, with readable long titles and controls that wrap or rearrange in the task's reading order.` rendered the shipped Coursework landing in headless Chromium, kept a long title inside the viewport at 1280 and at 800, kept the primary action left of the secondary action at 1280, and stacked the secondary action under the primary action at 800. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Every Student browser action should be usable with the keyboard alone.
  - Evidence (source): `src/components/record_list/record_list.tsx` `RecordActionControl` renders each Course and Coursework action as a link or a button.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `Your Courses` is a link back to the Student course list.
  - Evidence (test): `tests/playwright/student_course_entry_m6_evidence.mjs` `Every Student browser action should be usable with the keyboard alone.` reached Resume Weekly Assignment, Review Bonus Assignment, Start Bonus Assignment, Open Quiz, Your Courses, and Open Course by Tab only, activated each with Enter, and rejected any focused control that was not a native link, button, or form control. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student pages should use names meaningful to Students.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` uses "Your courses" and "Open assigned work".

- [x] Student navigation and pages should contain only Student interfaces and capabilities.
  - Evidence (source): `src/route_access_boundary.tsx` `withRouteAccessBoundary` admits an authenticated session only through `userRoleMayAccessRoute` and otherwise says the page is not available to this account.
  - Evidence (source): `src/routes.ts` `appRoutes` wraps every declared product route in that boundary.
  - Evidence (source): `crates/server/src/live_student_course_landing.rs` `student_profile_role_is_allowed` admits only the Student role before Student Course reads.
  - Evidence (source): `crates/server/src/assessment_delivery.rs` `student_with_sessions` returns a session hash only when the authenticated role is Student.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student navigation and pages should contain only Student interfaces and capabilities.` denied Instructor and Sysadmin access to every Student route, kept Student Ribbon links on those routes or Profile, withheld a Student Profile image upload, and bound each Student route to its Student page.

- [x] Student content entry should use the response controls provided by Questions and other Student activities.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` passes the current Question presentation's response format to `QuestionPresentationResponseControl`.

- [x] Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Evidence (source): `src/features/profile_avatar/profile_avatar_role.ts` `profileRoleMayManageImage` shows a Profile image upload only to an Instructor or Sysadmin.
  - Evidence (source): `crates/server/src/profile_avatar.rs` `replace_profile_image` refuses every other role before reading an image.
  - Evidence (source): `crates/server/src/draft_question_images.rs` `instructor_session_hash` admits only an Instructor session before a Draft Question image upload.
  - Evidence (source): `crates/server/src/course_appearance.rs` `instructor_session_hash` admits only an Instructor session before a Course Banner upload.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_workspace.tsx` `Student-facing prompt` is a text box for Instructor-created Question content.
  - Evidence (source): `src/pages/course_roster_page.tsx` `importRoster` records Course roster rows from a text box.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `Students should have no upload capabilities. Instructor-created content should use text boxes` found file inputs only for the Course Banner, Profile image, and HOTSPOT image, refused those Instructor routes and the Profile image control for a Student, and kept the Question prompt and roster import as text boxes.
  - Owner: 03_shell.md / General interface design (first occurrence; identical requirement and status).

#### Student Ribbon interface
- [x] The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels Student navigation Coursework, Grades, and Courses.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Ribbon labels use Coursework language`.

- [x] Student Tier 1 navigation uses **Coursework**, **Grades**, and **Courses**, in that order.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_ONE` declares that Student order.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Coursework keeps its collective Tier 1 label without an Attempt-only row`.

- [x] **Coursework** and **Grades** show relevant information across all of the Student's enrolled Courses. When a Student has more than one Course, records clearly identify their Course.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentAllCourseworkPage` and `src/pages/student_course_grades_page.tsx` `StudentScoresPage` group every enrolled Course by short name and long name.

- [x] **Coursework** and **Grades** each have a stable Tier 2 set. Opening Coursework, an Attempt, a review, or another item does not change the Tier 2 choices or their order.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` fixes the Student rows independently of the route.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Tier 2 choices and order stay fixed across routes and Course context`.

- [x] I accept grayed-out Tier 2 choices; keep each in place instead of hiding or removing it, then dynamically inserting or restoring it.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `UnavailableRibbonChoice` stays in `isRequiredTaskArea` rows, including Student Coursework and Grades.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `the fixed shortcut stays disabled when no resumable Attempt is reported`.

- [x] Student Tier 2 must include the choices below. Additional choices may be added when they provide a useful Student navigation destination.
  - Evidence (source): `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` lists the Student Coursework, Grades, and Courses choices.

  - [x] **Coursework**
    - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student tab Coursework.

    - [x] **All Coursework**: Shows all Coursework across the Student's enrolled Courses.
      - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentAllCourseworkPage`.

    - [x] **Due Soon**: Shows Coursework across the Student's enrolled Courses that is approaching its due date.
      - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentDueSoonPage`.
      - Evidence (test): `tests/test_student_coursework_presentation.mjs` `isInStudentDueSoonWindow`.

    - [x] **Completed**: Shows Coursework across the Student's enrolled Courses for which the Student has submitted at least one Attempt. Completed does not mean that the Student earned a perfect score.
      - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentCompletedPage`.
      - Evidence (test): `tests/test_student_coursework_presentation.mjs` `hasSubmittedStudentAttempt`.

    - [x] **Active Attempt**: I want a quick return to an Assessment with a running clock. Keep it visible but disabled when no Attempt's clock is running.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `deriveRibbonModel`.
      - Evidence (test): `tests/test_ribbon_contract.mjs` `Active Attempt links directly to the server-selected resumable Attempt`.

  - [x] **Grades**
    - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student tab Grades.

    - [x] **Scores**: Shows the Student's released Coursework scores across their enrolled Courses.
      - Evidence (source): `src/pages/student_course_grades_page.tsx` `StudentScoresPage`.

    - [x] **Response Stats**: Shows statistics about the Student's responses across Coursework, subject to Student-visible score and per-Question feedback rules.
      - Evidence (source): `src/pages/student_course_response_stats_page.tsx` `ResponseStatsRecord`.
      - Evidence (test): `crates/browser-api-contract/src/student_course_response_stats.rs` `contract_contains_only_disclosed_counts_and_exact_question_revision`.

    - [x] **Attempt History**: Shows the Student's previous Assessment Attempts across their enrolled Courses and provides access to review them when permitted.
      - Evidence (source): `src/pages/student_course_attempt_history_page.tsx` `StudentAttemptHistoryPage`.

    - [x] **Latest Feedback**: Provides quick access to the most recent feedback available to the Student across their enrolled Courses. It remains visible but disabled when no feedback is available.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `studentLatestFeedback`.
      - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Tier 2 choices and order stay fixed across routes and Course context`.

  - [x] **Courses**
    - Evidence (source): `src/ribbon/ribbon_schema.ts` `CURRENT_STUDENT_COURSES`.

    - [x] Tier 2 shows the short names of the Student's currently enrolled Courses.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `taskAreasFor`.

    - [x] Selecting a Course opens that Course.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `studentCourseLanding`.

    - [x] The current Course is visually identified when the Student is viewing Course-specific content.
      - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Coursework keeps its collective Tier 1 label without an Attempt-only row`.

    - [x] The Course list changes when the Student's Course enrollment changes; navigating within a Course does not change the list or its order.
      - Evidence (source): `src/ribbon/ribbon_contract.ts` `taskAreasFor` keeps the supplied `studentCourses` order.

- [x] On narrow screens, use a compact navigation arrangement that keeps the product identity, current location, navigation controls, and Profile readable and reachable.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `AppRibbon` renders the P mark, selected location, Tier 1 controls, and Profile.
  - Evidence (source): `src/ribbon/app_ribbon.css` `@media (max-width: 40rem)` compacts that navigation on a narrow screen.

#### Student Course and Coursework interface
- [x] I use Coursework and Grades across all my enrolled Courses; opening a Course from Courses does not pin or filter those global views.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentAllCourseworkPage` and `src/pages/student_course_grades_page.tsx` `StudentScoresPage` load every enrolled Course.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `global Student Tier 2 destinations stay available without Course context`.

- [x] When I view Course-specific content, the Ribbon, breadcrumb, PageFrame, and relevant records should make the Course clear.
  - Evidence (source): `src/ribbon/ribbon_contract.ts` `courseBreadcrumbItem` and `src/pages/student_course_landing_page.tsx` `StudentCourseworkCourseSection`.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `an Attempt breadcrumb identifies its explicit Course context`.

- [x] Students should be able to see their active Courses and Coursework from the main navigation.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` provides the current-Course index; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` provides its work.

- [x] Course invitations should show the Course name and relevant Instructor and term information before the Student accepts the invitation.
  - Evidence (source): `src/pages/student_course_invitation_page.tsx` `StudentCourseInvitationPage` shows the Course name, Instructor, and term before Accept invitation.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_pending_student_course_invitations` returns only that Student's pending Course, Instructor, and term.

- [x] Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
  - Evidence (source): `src/pages/student_coursework_presentation.ts` `studentCourseworkDisplay` and `src/pages/student_course_landing_page.tsx` `assessmentContent`.
  - Evidence (test): `tests/test_student_coursework_presentation.mjs` `Coursework status labels distinguish upcoming, available, completed, and missed work`.

- [x] Coursework lists should make due dates, Type, and completion status easy to scan.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `assessmentContent`.

- [x] Progress should keep Coursework visible when it has Attempts but no released score, clearly marked **Score not released**.
  - Evidence (source): `src/pages/student_course_progress_page.tsx` `StudentCourseProgressPage` and `src/student_course_progress_presentation.ts` `studentAssessmentScoreStateLabel`.

- [x] Submitted work should remain distinct from a perfect score; **Completed** means at least one submitted Attempt.
  - Evidence (source): `src/student_course_progress_presentation.ts` `completedStudentAssessmentCount` and `studentAssessmentScoreStateLabel`.
  - Evidence (test): `tests/test_student_coursework_presentation.mjs` `hasSubmittedStudentAttempt`.

- [x] **Response Stats** should show actual Student response outcomes across Assessment Types, subject to Student-visible score and per-Question feedback rules.
  - Evidence (source): `src/pages/student_course_response_stats_page.tsx` `ResponseStatsRecord`.
  - Evidence (test): `crates/browser-api-contract/src/student_course_response_stats.rs` `contract_contains_only_disclosed_counts_and_exact_question_revision`.

- [x] Measured Question display time should be labeled as approximate **time shown with the Question**, with its sample count. It does not measure attention or effort and does not affect grades.
  - Evidence (source): `src/pages/student_course_response_stats_page.tsx` `durationLabel`.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `checkpoint_student_question_display_duration`.

- [x] Keep Coursework entries compact in height so Students can scan several items at once.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentList` renders each Coursework item through the shared record row.
  - Evidence (source): `src/components/record_list/record_list.css` `.record-list__row--semantic` keeps that row a short bordered list entry.

- [x] Keep essential Coursework information and the main action visible, with fuller access and timing details available through progressive disclosure.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` keeps the start action next to `StudentAssessmentStartFacts`.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentDecisionDetails` places Available, Closes, and Late work in `student-assessment-decision__details`.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-decision__details:not([open]) > :not(summary)` hides that fuller timing until the Student opens it.

- N/A Coursework lists may provide filters for **Weekly Assignments**, **Unit Review Assignments**, **Bonus Assignments**, **Quizzes**, and **Exams**.
  - Reason: Human Guidance permits these Type filters and does not require a Coursework list to offer them.

- [x] Each Coursework item should clearly show its Assessment Type using its label and Type icon.
  - Evidence (source): `src/components/record_list/record_list.tsx` `assessmentTypePresentation` renders the Type icon and label together.

- [x] Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` presents the title, specific Type, Question count, points possible, time limit, and previous Attempts before start.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` owns the compact Question, points, and time-limit facts.
  - Evidence (runtime): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` was exercised by accepted isolated actual-server/exact-main Student proof `/private/tmp/ple-course-empty-artifacts.JCFLm9` after a real roster import/claim. It opened a Released direct Practice Assessment before Start and showed title, Practice Type, one Question, one point, the exact one-hour limit, and an explicit zero-previous-Attempt state; an Unreleased sibling was omitted and an outsider received 404. A separate accepted native Student HTTP/browser run `/private/tmp/ple-course-empty-artifacts.ONrLSK` whole-submitted a real graded 1/1 Attempt, then reopened the overview before starting another. The same six facts included an actual "Previous attempts" Attempt 1 Submitted link; its clicked history showed recorded PKU response and 1/1 score. The overview's previous-Attempt score is optional under the current DTO, so this row does not require that optional value or claim every Student viewport.

- [x] Present the "Before you start" settings as a compact summary. Keep each label beside its value in aligned rows, using a compact grid when width permits.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` renders the Before you start summary.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-start-facts .assessment-facts > div` places each label beside its value, and the facts list uses a compact grid.
  - Evidence (source): `src/browser_environment.ts` `student_assessment_presentation.css` loads that layout for the application shell.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Before you start keeps each label beside its value in a compact grid` renders `StudentAssessmentStartFacts`.

- [x] Group Question count and points together, and group availability, deadlines, and Attempt rules into clearly readable sections with concise spacing.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` keeps Questions and Points possible together, and `StudentAssessmentDecisionDetails` sections Deadlines and Attempt rules apart from Availability.
  - Evidence (source): `src/components/student_assessment_presentation.css` `.student-assessment-decision__sections` spaces those sections with the compact decision gap.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Coursework start facts group questions, deadlines, and Attempt rules` renders `StudentAssessmentStartFacts`.

- [x] Express unset or unlimited settings in Student language, such as "No closing time" or "Unlimited Attempts". Format dates in the selected display zone.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `No closing time` labels an unset close, and `Unlimited Attempts` labels an unset Attempt limit.
  - Evidence (source): `src/format_datetime.ts` `createDisplayDateTimeFormatter` formats those instants in the selected display zone.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `unset Coursework limits use Student language in the selected display zone` renders `StudentAssessmentDecisionDetails`.

- [x] Keep the start action close to this summary so Students can review the rules and begin with minimal scrolling.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` renders the start button in `student-assessment-action-region` immediately after `StudentAssessmentStartFacts`.

#### Student Coursework interface
- [x] Students see one Question at a time while completing Coursework.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders one keyed current presentation in one `article.question-card`.

- [x] While completing Coursework, navigation should provide access to every Question and its saved
  status, with direct jumps between Questions.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` renders every position, saved-status label, and position button.
  - Evidence (test): `tests/test_student_assessment_attempt_navigation.mjs` `Student Question navigation renders ordered, answer-free states with one current Question`.

- [x] Leaving a Question and returning should preserve its saved response.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `activatePosition` saves before changing position; `src/pages/assessment_attempt_page.tsx` `loadPresentation` restores the persisted `savedResponse` when the Student returns.

- [x] The current Question and overall progress should remain easy to see.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` owns the visible current/total Question and saved-count summary; `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders this component without the retired duplicate eyebrow.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, accepted supplied `/private/tmp/ple-compact-student-navigation.md` and independent `/private/tmp/ple-compact-navigation-independent-review.md` show visible current/total progress at 1280 and 390 pixels.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders active Question navigation and its loading state only while the Attempt is active; submitted and expired states retain the terminal message without current/saved navigation.
  - Evidence (runtime): `src/pages/assessment_attempt_page.tsx` `AttemptExperience`, accepted authenticated Avery R-4 browser receipt `/private/tmp/ple-attempt-finished-nav-fixed.png`, shows the terminal heading with no active Question navigation, no misleading `Question - of 4`, and no `0 saved` summary. Independent `/root/terminal_attempt_ui_review` accepted the rendered terminal state with no findings.

- [x] The timer should be subtle and keep the focus on the Questions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` places the `calm-status` timer in the Assessment Attempt header, outside the Question card.

- [x] For timed Coursework, the remaining time should stay visible while moving between Questions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders `remainingMilliseconds` in the persistent header while keyed Question presentations change below it.

- [x] Submission status should be obvious and use plain language.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders "Submitting Assessment...", "Your answers were accepted", and plain-language save or submission errors from the submission state.

- [x] Present Question navigation as a compact horizontal row of numbered controls, with distinct
  current-Question and saved-status cues.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] For long Question sets, use forum-style pagination with Previous and Next controls, the first
  and last Question numbers, a range around the current Question, and ellipses for omitted ranges.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] Adapt the visible number range to the available width while keeping every Question reachable.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] Keep the Question prompt and response controls near the top of the working area. Give the
  title, timing summary, and Question navigation only the space needed to orient Students.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` provides numbered current/saved controls, width-adaptive first/last/range pagination, ellipses, and Previous/Next.
  - Evidence (runtime): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation`, supplied 2026-09-16 receipts `/private/tmp/ple-compact-student-navigation.md` and independent acceptance `/private/tmp/ple-compact-navigation-independent-review.md`: actual four-Question 1280/390px saved-response navigation and styled 250-Question harness keyboard traversal, first/last jumps, width adaptation, and reachable local scrolling at 200% enlargement. This is bounded Attempt-navigation evidence, not full Student/browser/theme acceptance.

- [x] Make the current Question, saved-response status, and keyboard-focused control visually distinct
  so Students can recognize where they are, what work is saved, and which action they will activate.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.css` `.is-current` gives the current Question a heavier weight and accent fill, and `.is-saved` adds a saved marker.
  - Evidence (source): `src/components/question_response_control_styles.ts` `QUESTION_RESPONSE_CONTROL_STYLES` paints `.format-status.saved` apart from an unsaved status.
  - Evidence (source): `src/components/question_response_controls/common.tsx` `Status` adds `saved` when the response is saved.
  - Evidence (source): `src/style.css` `:focus-visible` outlines the keyboard-focused control.

- [x] Label response actions by their effect, such as "Save response" and "Clear response", so Students
  can distinguish recording their work from changing it or submitting the whole Coursework.
  - Evidence (source): `src/components/question_response_controls/common.tsx` `Actions` renders the record action, and the Attempt page passes `Save response`.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `Clear response` is the visible change action; its accessible name is `Clear response for` the prompt. Ordering changes use `Move Ordering Item`.
  - Evidence (source): `src/pages/assessment_attempt_finish.ts` `submitAttemptActionLabel` names whole-Attempt submission `Submit Attempt`, and `src/pages/assessment_attempt_page.tsx` `submitAttemptActionLabel` is the Finish Attempt button.
  - Evidence (test): `tests/test_question_response_controls.mjs` `response actions distinguish Save response, Clear response, and Submit Attempt` renders `QuestionPresentationResponseControl` and calls `submitAttemptActionLabel`.

- [x] Group response feedback near the response controls and keep routine saved-status messages brief.
  - Evidence (source): `src/components/question_response_controls/common.tsx` `responseStatusMessage` places one saved-status sentence in `Status`, directly above the response actions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `persistenceNotice` passes the Attempt save state into that status line.
  - Evidence (test): `tests/test_question_response_controls.mjs` `saved response status stays one brief message beside the response actions` renders `QuestionPresentationResponseControl` with `responseStatusMessage`.

#### Student Coursework review interface
- [x] Scores and feedback should appear where the Coursework settings allow them.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_history` applies the server-owned feedback-release decision before projecting scores and per-Question feedback; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` renders only the released fields present in that projection.

- [x] Completed Coursework should remain easy to find and review.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` lists and links previous Attempts; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` presents the selected Attempt's score and recorded work.

- [x] Group each reviewed Question's number, result, points, recorded response, and permitted feedback
  into a compact, clearly separated unit.
  - Evidence (source): `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` groups the Question number, result, points, recorded response, and released feedback in one unit.
  - Evidence (source): `src/pages/assessment_attempt_summary_styles.ts` `.attempt-summary .attempt-summary__question` separates that unit on the shipped summary content class.

### Sysadmin interface
- N/A Sysadmin interface work is LOW, LOW priority and can be done on an as-needed basis.
  - Reason: audited human-owned work priority and scheduling guidance, not a claim about implemented PLE behavior. The following Sysadmin capability requirements remain binding.

- [x] The Sysadmin interface should focus on system administration.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` labels its workspace "System administration" and manages Instructor Accounts.

- [ ] The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find.
  - Reason: product decision still unclear
  - Question: Does system configuration need its own menu destination now, or is the menu complete while installation-wide settings remain an open inventory?
  - Mismatch: `src/pages/role_home_pages.tsx` `SysadminHomePage` reaches Instructor Accounts and installation Courses, and `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_ONE` includes `instructorAccounts`, `courses`, and `disciplines`, while no system-configuration destination exists. One reading adds that destination once system-wide settings are identified. The other treats Accounts, Instructors, and Courses as the findable menu until a setting exists.

- [x] Sysadmins should be able to find users quickly by name or email.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `list_instructor_account_avatar_summaries` matches a vetted display name or a normalized authentication email and returns only the closed Account summary.
  - Evidence (source): `crates/server/src/instructor_account.rs` `find_instructor_accounts` accepts that query in the request body and returns the same summary.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `findInstructorAccounts` submits the Sysadmin's name or email from the Instructor Accounts page.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `Sysadmins should be able to find users quickly by name or email.` drives the shipped page and client for Ada Lovelace and ada@university.edu, then shows the returned Account ID without the name or email. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Account lists should support searching, filtering, and scanning large numbers of users.
  - Evidence (source): `schemas/base_schema/50_functions/profile_media.sql` `list_instructor_account_avatar_summaries` filters one Account State and returns one page of 50, 100, or 250 closed Account summaries.
  - Evidence (source): `crates/server/src/instructor_account.rs` `find_instructor_accounts` accepts the state, page size, and cursor in the request body.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `instructor-account-state` filters the Instructor Account list and moves through pages of 50, 100, or 250.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `Account lists should support searching, filtering, and scanning large numbers of users.` searched Ada Lovelace, filtered to deactivated accounts, chose 100 accounts per page, and opened the next page on the shipped Instructor Accounts page. The list showed the returned Account ID without the name. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] User pages should clearly show role, account status, and other important administrative information.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `accountContent` shows Role, account state, and last successful sign-in on each Instructor Account row. The list is Instructor Accounts only, so the role is Instructor. Email and display name stay off the row.
  - Evidence (test): `tests/playwright/test_course_entry_banner.mjs` `User pages should clearly show role, account status, and other important administrative information.` read Role Instructor, State Active, and the last successful sign-in for U7K3M2PA0 on the shipped Instructor Accounts page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Sysadmins create accounts and manage account access.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` provides Instructor Account creation, deactivation, and reactivation actions.

- [x] Sysadmins approve Instructors before they receive Instructor capabilities.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `createAccount` completes Instructor identity vetting and then creates the account with that decision id.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `require_completed_instructor_identity_vetting` rejects account creation unless that decision is completed for the same email, and `create_instructor_account` calls it before the account insert.
  - Evidence (test): `tests/test_instructor_account_decoder.mjs` `Sysadmins approve Instructors before they receive Instructor capabilities.` refused an account-creation request that omitted the vetting decision before any fetch, then drove the shipped Instructor Accounts page in headless Chromium. The page posted identity vetting and then account creation with the returned decision id. A rejected vetting decision sent no account-creation request. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] Instructor approval status should be easy to find and change.
  - Reason: product decision still unclear
  - Question: Is Instructor approval the completed pre-account vetting decision, with later access changed only by deactivate and reactivate, or a distinct Account status a Sysadmin can find and change after creation?
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` exposes active, deactivated, and closed lifecycle state, and `schemas/base_schema/20_tables/account.sql` `instructor_identity_vetting_decision` stores a completed vetting decision that is not an Account state. One reading keeps approval as that immutable decision and changes access only through deactivate and reactivate. The other adds a separate post-creation approval status. Human Guidance asks for a status that is easy to find and change, and it also says a Sysadmin approves an Instructor before that Account receives Instructor capabilities.

- [x] Sysadmins should be able to find and inspect Courses across the installation.
  - Evidence (source): `schemas/base_schema/50_functions/sysadmin_course_inspection.sql` `list_installation_courses` returns one page of installation Courses, and `src/pages/sysadmin_course_inspection_page.tsx` `SysadminCourseInspectionListPage` finds them from the Sysadmin home.
  - Evidence (test): `tests/test_sysadmin_course_inspection.mjs` `sysadmin installation course inspection shows instructor and status` renders the shipped list for an installation Course. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Course administration should show the Instructor and important Course status information.
  - Evidence (source): `schemas/base_schema/50_functions/sysadmin_course_inspection.sql` `load_installation_course` returns the active Instructor display names, activity, and retention state, and `src/pages/sysadmin_course_inspection_page.tsx` `SysadminCourseInspectionPage` shows those facts.
  - Evidence (test): `tests/test_sysadmin_course_inspection.mjs` `sysadmin installation course inspection shows instructor and status` renders the Instructor, activity, and retention on the shipped inspection page. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
  - Decision: Human Guidance assigns Sysadmins creation of a Course Instance for an Instructor who teaches it. Retention transitions stay with the background process.
  - Evidence (source): `src/components/sysadmin_course_creation.ts` `submitSysadminCourseCreation` creates an empty Course for the selected Instructor, and `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` assigns that Instructor when the actor is a Sysadmin.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminCourseCreation` is the Sysadmin home control for that creation.
  - Evidence (test): `tests/test_frontend_contract.mjs` `a Sysadmin creates a Course that an Instructor teaches` drives `submitSysadminCourseCreation` and records the assigned Instructor. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] System-wide settings should have their own area, separate from user and Course administration.
  - Reason: product decision still unclear
  - Question: Which implemented installation-wide settings must Sysadmins view or change, and which source-of-truth boundary owns each?
  - Mismatch: `src/ribbon/ribbon_catalog.ts` has no system-settings destination. The guidance could require a page for actual platform settings, or no page until implemented system-owned settings exist; current evidence cannot select between those readings.

- [x] Everyday navigation should emphasize frequently used administrative tasks.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `instructorAccounts` is a primary critical task while `supportRoster` is supporting normal priority.

- [ ] Rare installation and configuration tasks should remain available through secondary navigation.
  - Reason: product decision still unclear
  - Question: Does this require a secondary Ribbon row now, or does reachability from the Sysadmin home satisfy it while the complete Ribbon layout stays unlocked?
  - Mismatch: `src/ribbon/ribbon_schema.ts` `PRODUCT_TIER_TWO` has no Sysadmin destinations, and `src/pages/role_home_pages.tsx` `SysadminHomePage` still links Disciplines, Library activity, and scoped roster support. `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` already includes `instructorAccounts` and `disciplines` as primary tasks. One reading adds a secondary Ribbon row for rare tasks. The other keeps those tasks on the Sysadmin home until Human Guidance locks the complete Sysadmin Ribbon layout.

- [x] High-consequence administrative actions should have a visually distinct area.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `renderAccountBody` places deactivation in `instructor-account-consequence`, apart from ordinary account facts.
  - Evidence (source): `src/pages/instructor_accounts_page.css` `.instructor-account-consequence` gives that area its own border and fill.

- [x] Confirmation for destructive actions should clearly state what will happen.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `requestDeactivation` states that the Instructor Account is deactivated, current sessions end, and sign-in stays blocked until reactivation, and `deactivate` runs only after that confirmation.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `revoke_sessions_after_account_deactivation_or_closure` ends current sessions when the account is deactivated.

- [ ] The complete Sysadmin Ribbon task layout does not have a locked-in design yet.
  - Reason: HG: no locked-in design.
  - Mismatch: no complete Sysadmin Ribbon task layout can be verified until the design is locked.
## Data and history
- [x] Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` copies a Question Answer, its explanation, a score, or per-item correctness only when the server disclosure decision releases that field.
  - Evidence (source): `crates/browser-api-contract/src/assessment_delivery.rs` `StudentQuestionPresentation` gives an open Attempt the prompt and answer-free response controls.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document/author_script.rs` `compile_author_content` retains the author script and does not copy the Answer Key.
  - Evidence (source): `crates/grading/src/ple_question_json.rs` `answer_key` stays on the private grading value that `evaluate_response` uses on the server.
  - Evidence (source): `crates/server/src/webwork_document_route.rs` `permitted_answer_revision_tuple` withholds the WeBWorK correct-answer document unless the server disclosure decision releases the Question Answer.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `SavedResponseAcknowledgement` confirms a save with the Attempt id, position, and saved state.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_provided_feedback_is_shown` passed on 2026-09-30. The withheld projection omitted questionAnswer and questionAnswerExplanation. `weekly_and_bonus_keep_the_correct_answer_hidden_after_submission` kept Weekly and Bonus correct answers hidden after submission and still showed correctness. Fifteen student_feedback_release tests passed.

- [x] Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` stores identity-free issued, blank, answered, outcome, and credit-sum counts in ple_data, and `question_statistics_observation_receipt` stores the Course, Issued Question, and submission receipt in ple_private.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `assessment_attempt_saved_response` stores student_response in ple_private.
  - Evidence (source): `schemas/base_schema/20_tables/authentication.sql` `account_authentication_email` stores normalized_email in ple_private.
  - Evidence (source): `schemas/base_schema/00_roles.sql` `ple_private` revokes schema privileges from PUBLIC. ple_app and ple_student receive ple_api usage and do not receive ple_private usage.
  - Evidence (source): `schemas/base_schema/70_grants/statistics.sql` `question_revision_statistics` revokes PUBLIC and grants SELECT only to ple_api_owner, and `question_statistics_observation_receipt` revokes PUBLIC.
  - Evidence (runtime): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` on a fresh PostgreSQL 17 install had columns published_question_id, revision_number, issued_count, blank_count, answered_count, correct_count, partial_count, incorrect_count, credit_sum, credit_sum_sq, created_on, and updated_on. The receipt held course_instance_id, issued_question_id, and assessment_submission_id. `schemas/base_schema/20_tables/assessment_attempt.sql` `assessment_attempt_saved_response` held student_response. `schemas/base_schema/20_tables/authentication.sql` `account_authentication_email` held normalized_email. ple_app, ple_student, and public had no SELECT on any ple_private table, view, materialized view, or partition, and no USAGE on ple_private. public had no USAGE on ple_data. SET ROLE ple_app was denied SELECT on the receipt, saved-response, and email tables with permission denied. No rows were inserted. A disposable PostgreSQL session ran that check on 2026-09-30 and was not kept. No Question Backend was called.

- [x] Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_selected_entry.ts` `selectedAssessmentEntryContent` uses the Question or Pool title as the record heading and keeps Question ID, Revision, Question Pool ID, and Edit as labeled facts.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_entry_content.ts` `blueprintAssessmentEntryContent` uses the Question or Pool title as the Blueprint Assessment entry heading and keeps the public ID, Revision, and Edit as labeled facts.
  - Evidence (source): `src/features/blueprint_course/blueprint_pool_members_editor.tsx` `BlueprintPoolMembersEditor` titles the Pool and each member and keeps Question Pool ID, Question ID, Edit, and Revision as labeled facts.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_review.tsx` `AssessmentSnapshot` titles each compared Question and Pool and lists shared Question IDs with their titles.
  - Evidence (source): `src/features/blueprint_course/blueprint_history.tsx` `historicalEntryContent` titles each saved Blueprint entry and keeps the public ID as a labeled fact.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `AcceptedResult` titles each committed Question and Pool entry and keeps the public ID as a labeled fact.
  - Evidence (source): `src/components/course_student_work_recovery.tsx` `CourseStudentWorkRecovery` titles each recovered Issued Question and keeps its Question ID and Revision as a labeled fact.
  - Evidence (source): `src/pages/student_course_response_stats_page.tsx` `ResponseStatsRecord` titles each disclosed Question and keeps its Question ID and Revision as a labeled fact.
  - Evidence (source): `schemas/base_schema/50_functions/student_course_response_stats.sql` `list_live_student_course_response_stats` returns question_title with the Published Question ID. This source was not executed in PostgreSQL.
  - Evidence (source): `schemas/base_schema/50_functions/recognition_titles.sql` `load_recognition_titles` returns the current Question and Pool titles for the requested public IDs. This source was not executed in PostgreSQL.
  - Evidence (source): `src/api/http_client/blueprint_course.ts` `loadRecognitionTitles` posts those public IDs and decodes the returned titles.
  - Evidence (source): `src/components/copyable_question_id.tsx` `CopyableQuestionId` shows the Question title beside the labeled public Question ID, and copies that ID.
  - Evidence (test): `tests/test_student_course_response_stats.mjs` `decodeStudentCourseResponseStats` accepted questionTitle Peptide bond for Question ID 7K3M-79QP on 2026-09-30.
  - Evidence (test): `src/features/blueprint_course/blueprint_assessment_entry_content.ts` `blueprintAssessmentEntryContent` returned heading Peptide bond with labeled Question ID 7K3M-79QP, and heading Amino acids with labeled Question Pool ID 2R5X-E7YA, on 2026-09-30. `selectedAssessmentEntryContent` returned the same Question heading and labeled Question ID. `cargo check -p learning-data-access --features postgres` and `cargo check -p server_core` finished. `contract_contains_only_disclosed_counts_and_exact_question_revision` passed. No PostgreSQL session ran `load_recognition_titles`. No Question Backend was called.

- [x] FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage.
  - Evidence (source): `src/log.ts` `log` prints a closed public phrase and withholds an unrecognized event. Extra arguments are ignored.
  - Evidence (source): `crates/learning-data-access/src/contracts/store_error.rs` `diagnostic_kind` names a store failure without the record message, identity, response, or answer.
  - Evidence (source): `crates/server/src/worker.rs` `log_expiry_error` records the expiry stage and `diagnostic_kind` and does not record the Attempt id.
  - Evidence (source): `crates/server/src/request_lifecycle.rs` `request_lifecycle` records the method, status, elapsed time, and a server-minted request id.
  - Evidence (source): `crates/server/src/webwork_document_route.rs` `without_preview_parent_telemetry` removes the parent-frame telemetry loader from a preview document.
  - Evidence (source): `src/auth/secret_fragment.ts` `consumeTokenFragment` reads one token fragment and removes it from the visible URL.
  - Evidence (source): `src/api/http_client/request.ts` `requestSameOrigin` sends same-origin requests with cache no-store.
  - Evidence (runtime): `src/log.ts` `log` printed `[ple] peptidyle client booting` for clientBooting on 2026-09-30. An email address, an answer-key object, and a Student response were not printed. The unrecognized event printed `[ple] diagnostic withheld`. No Question Backend was called.

- [x] Opaque IDs remain FERPA-sensitive when they link a Student to Course activity.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_owns_student_record` authorizes a Student record only through its Course and active membership.

### Human-facing public IDs
- [x] Public IDs are intended for content creators (vetted Instructors) and for Sysadmins providing Instructor support.
  - Evidence (source): `src/pages/library_page_model.ts` `displayId` requires a canonical Question ID on Instructor library records.
  - Evidence (source): `crates/server/src/instructor_account.rs` `list_instructor_accounts` lists Instructor Account IDs only for a Sysadmin session.

- [x] Here the public refers to vetted Instructor users and Sysadmins.
  - Evidence (source): `src/pages/library_page_model.ts` `displayId` is the Question ID Instructors read and open in the library.
  - Evidence (source): `crates/server/src/instructor_account.rs` `list_instructor_accounts` is the Sysadmin Account ID lookup, guarded by `sysadmin_session_hash`.

- [x] Human-facing public IDs should be short, opaque, and easy to communicate.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QUESTION_ID_CANONICAL_LENGTH` fixes the Question and Pool form at nine characters, and `QUESTION_ID_ALPHABET` omits ambiguous Crockford characters.
  - Evidence (source): `crates/question_model/src/public_route.rs` `PUBLIC_ID_RANDOM_LENGTH` fixes seven random characters before the checksum on prefixed IDs.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredPublicId` accepts spoken Crockford aliases and returns the canonical form.

- [x] Human-facing public IDs should not reveal creation order, counts, database keys, ownership, or other object metadata.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `crockford_id_suffix` draws seven characters from random UUID bytes and skips the UUID version byte so the suffix does not expose creation order.
  - Evidence (source): `crates/question_model/src/question_library.rs` `PublishedQuestionId` is a non-sequential identity, and `public_id_checksum_character` derives the final character from the other ID characters.

- [x] A public ID is the one universal, canonical human-facing identifier for a PLE object that needs one.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` records one canonical value per public object kind, while the owning tables store that value directly.

- [x] Give an internal object a human-facing public ID when a useful workflow needs it.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `assign_public_id` issues Account, Course Instance, Assessment, and Blueprint Course IDs for the workflows that display and support those objects.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `assessment_attempt_id` keeps an Attempt as a UUID because resuming that Attempt does not ask anyone to type an Attempt ID.

- [x] Useful human-facing ID workflows include display, search, communication, and support.
  - Evidence (source): `src/pages/profile_account_id.tsx` `ProfileAccountId` shows a checksum-validated Account ID and copies that canonical value.
  - Evidence (source): `src/pages/library_browse_rows.tsx` `questionDetails` places the Question ID on each library row.
  - Evidence (source): `src/api/question_library_repository.ts` `questionSearchRequest` forwards the search box text unchanged.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `QuestionTextQuery` treats a canonical Question ID as the exact library search identity.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `accountContent` titles each Sysadmin Instructor Account row with that Account ID.
  - Evidence (source): `src/components/sysadmin_course_creation_form.tsx` `assignedInstructorAccountId` names the Instructor who teaches a Course a Sysadmin creates.
  - Evidence (test): `tests/test_frontend_contract.mjs` `the profile page shows the signed-in Account ID as a labeled fact` rendered Account ID U0000035E with its copy control and hid a non-canonical value.
  - Evidence (test): `crates/server/src/question_library/search_query.rs` `exact_canonical_question_id_is_the_library_search_identity` accepted ABCD-XEFG as the exact Question ID and rejected a bad checksum and lowercase text.
  - Evidence (test): `tests/test_frontend_contract.mjs` `a Sysadmin creates a Course that an Instructor teaches` sent Instructor Account ID U0000035E and refused a blank Account ID.

- [ ] Store and use the exact same public ID in the database, Rust, JSON, URLs, object storage, hashes, logs, and browser UI.
  - Verification pending: SQL, Rust, generated TypeScript, and route contracts use exact canonical values, but object-storage, hash, log, and every browser projection still need a complete inventory.

- [ ] Preserve the canonical ID exactly across system boundaries.
  - Verification pending: typed SQL, Rust, and browser validators are exact, but every transport, persistence, logging, object-storage, and display boundary has not been inventoried.

- [ ] Parsing, serialization, API transport, persistence, and display do not reformat or translate the canonical ID.
  - Verification pending: strict Rust and browser parsing plus canonical SQL storage are implemented; a complete serialization, API, persistence, and display inventory remains pending.

- [x] ID generation enforces global uniqueness across all public IDs and retries random collisions.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` provides one global collision boundary, and `assign_public_id` retries the shared `QP001` collision signal.

- [x] Once issued, a public ID permanently identifies that object.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation_is_permanent` rejects reservation update or deletion.

- [x] Never reuse a public ID for another object, including after deletion or archival.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` is an append-only global registry retained independently of object lifecycle state.

#### Public ID alphabet and canonical form
- [x] Public IDs use the Crockford Base32 alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QUESTION_ID_ALPHABET` is the shared public-ID alphabet used by the Rust issuers and generated browser contract.

- [x] Public IDs have one canonical uppercase ASCII form.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId::from_str` and `crates/question_model/src/public_route.rs` `impl_public_id` reject every noncanonical form.

- [x] In ID format notation, `X` denotes a cryptographically random Crockford Base32 character.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `crockford_id_suffix` and `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` mint each `X` from operating-system randomness.

- [x] In ID format notation, `Z` denotes the calculated checksum character.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` calculates `Z` from the canonical checksum input.

- [x] Both `X` and `Z` represent characters stored as part of the canonical ID.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `is_canonical_prefixed_public_id` and `schemas/base_schema/50_functions/question_lineages.sql` `published_question_id_is_crockford_shape` validate the complete stored values.

- [x] `Z` is not a literal character or separate metadata.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `assign_public_id` appends the calculated checksum directly to the stored canonical ID.

- [x] Human-entered IDs may use lowercase Crockford characters.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` uppercase only explicit human-entry values before validation.

- [x] Human-entered IDs may use `O` or `o` for `0`.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` map the Crockford `O` alias to `0` before validation.

- [x] Human-entered IDs may use `I`, `i`, `L`, or `l` for `1`.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` map the Crockford `I` and `L` aliases to `1` before validation.

- [x] Normalize human-entered IDs to canonical form, then validate the canonical syntax and checksum at the human-input boundary.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` and `normalizeHumanEnteredPublicId` normalize explicit human entry and then invoke the generated exact validators.

- [ ] Store, transmit, display, copy, and generate only the canonical form.
  - Verification pending: strict generators, model parsers, SQL constraints, and browser validators are implemented, but every storage, transport, display, and copy surface has not been inventoried.

#### Public ID checksum
- [x] Calculate the checksum from the ASCII bytes of every other uppercase canonical-ID character.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` hashes caller-supplied canonical ASCII characters after typed constructors exclude separators and checksum positions.

- [x] Include type prefixes in the checksum input.
  - Evidence (source): `crates/question_model/src/public_route.rs` `impl_public_id` builds checksum input from the exact type prefix plus seven random characters.

- [x] Exclude only separators and the checksum position from the checksum input.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId::from_str` excludes the hyphen and checksum position, while `crates/question_model/src/public_route.rs` `impl_public_id` excludes only final `Z`.

- [x] `XXXX-ZXXX` has checksum input `XXXXXXX`.
  - Evidence (source): `crates/question_model/src/question_library.rs` `from_random_identifier` checksums the seven identity characters, then inserts the hyphen and checksum.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` checksums the first four characters plus the last three.

- [x] For `BPXXXXXXXZ`, `CIXXXXXXXZ`, `UXXXXXXXZ`, and `AXXXXXXXZ`, calculate the checksum from every preceding character.
  - Evidence (source): `crates/question_model/src/public_route.rs` `impl_public_id` hashes each literal prefix followed by its seven random Crockford characters.

- [x] Use public unsalted SHA-256 for the checksum.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` applies SHA-256 directly to the canonical checksum input.

- [x] Map the high five bits of SHA-256 digest byte 0 through the Crockford alphabet.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` shifts digest byte 0 by three bits and indexes `QUESTION_ID_ALPHABET`.

- [ ] Validate the public-ID syntax and embedded checksum before database lookup or resolution.
  - Verification pending: typed Rust and browser parsing plus canonical SQL predicates exist, but a complete lookup and resolution call-site inventory remains pending.

- [x] The embedded checksum detects typos.
  - Evidence (test): `crates/question_model/src/public_route.rs` `public_ids_are_exact_checksum_validated_values` accepts canonical vectors and rejects altered checksum characters for every current public-ID family.

- [x] The checksum adds no identity space.
  - Evidence (source): `crates/question_model/src/public_route.rs` `impl_public_id` derives the checksum deterministically from the prefix and seven-character random identity.

#### Public ID formats by object
- [x] Published Questions and Question Pools use the public `XXXX-ZXXX` format.
  - Evidence (source): `schemas/base_schema/10_types.sql` `question_family_id` stores both Published Question IDs and Question Pool IDs in that form.
  - Evidence (source): `crates/question_model/src/question_library.rs` `PublishedQuestionId` parses only that canonical form, and the same file defines `QuestionPoolId` with it.

- [x] Question IDs are the merged concept of Published Question IDs and Question Pool IDs
  - Evidence (source): `crates/question_model/src/question_library.rs` `QUESTION_ID_CANONICAL_LENGTH` is the shared canonical length for a Question or Pool ID.
  - Evidence (source): `schemas/base_schema/10_types.sql` `question_family_id` stores both Published Question IDs and Question Pool IDs.

- [x] The hyphen is specific to Question IDs. Other public IDs use a prefix without a hyphen.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` requires the hyphen, and `is_canonical_prefixed_public_id` accepts Account, Course Instance, Assessment, and Blueprint IDs with a prefix and no hyphen.

- [x] Published Questions and Question Pools share the same public-ID namespace.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `public_id_reservation` uses one primary key for both `published_question` and `question_pool` reservations.

- [x] An `XXXX-ZXXX` value identifies either a Published Question or a Question Pool, never both.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `reserve_public_id` rejects a second object-kind reservation for an already issued canonical value.

- [x] Blueprint Course IDs use `BPXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `BlueprintCourseId` and `schemas/base_schema/50_functions/blueprints.sql` `blueprint_course.blueprint_course_id` enforce the exact form.

- [x] Course Instance IDs use `CIXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `CourseInstanceId` and `schemas/base_schema/50_functions/course_core.sql` `course_instance.course_instance_id` enforce the exact form.

- [x] Assessment IDs use `AXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `AssessmentId` and `schemas/base_schema/50_functions/assessments.sql` `assessment.assessment_id` enforce the exact form.

- [x] Account IDs use `UXXXXXXXZ`.
  - Evidence (source): `crates/question_model/src/public_route.rs` `AccountId` and `schemas/base_schema/50_functions/accounts.sql` `account.account_id` enforce the exact form.

- [x] Each prefixed public ID uses seven cryptographically random Crockford Base32 characters and a final embedded checksum.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `assign_public_id` generates seven random characters, calculates the checksum over prefix plus random identity, and stores the result.

- [x] Each prefixed public-ID random namespace contains 32^7 = 34,359,738,368 values.
  - Evidence (source): `crates/question_model/src/public_route.rs` `PUBLIC_ID_RANDOM_LENGTH` fixes seven random positions over the 32-character `QUESTION_ID_ALPHABET`.

- [x] Account `UXXXXXXXZ` ids are look up values for Sysadmin support.
  - Evidence (source): `src/route_contract.ts` `instructorAccounts` opens `/sysadmin/instructor-accounts` only for the Sysadmin role.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `accountContent` shows the Account ID as the record title, with state and last successful sign-in.
  - Evidence (source): `crates/server/src/instructor_account.rs` `list_instructor_accounts` requires `sysadmin_session_hash` before returning those Account IDs.

- [x] Account `UXXXXXXXZ` ids are only exposed to Students or Instructors only on their profile page.
  - Evidence (source): `src/pages/profile_page.tsx` `ProfileAccountId` receives the signed-in `authenticatedAccountId` on Profile.
  - Evidence (source): `src/pages/profile_account_id.tsx` `validateCanonicalPublicId` shows and copies only a canonical Account ID.
  - Evidence (source): `src/route_contract.ts` `instructorAccounts` stays the Sysadmin support lookup, and `profile` is the Student and Instructor page.
  - Evidence (test): `tests/test_frontend_contract.mjs` `the profile page shows the signed-in Account ID as a labeled fact` rendered Account ID U0000035E and rendered nothing for a non-canonical value.

#### Database keys and clocks
- [x] An object with a public ID uses that public ID as its primary key and as the target of every
  foreign key to it.
  - Evidence (source): `schemas/base_schema/50_functions/course_core.sql` `course_instance.course_instance_id`, `schemas/base_schema/50_functions/assessments.sql` `assessment.assessment_id`, `schemas/base_schema/50_functions/blueprints.sql` `blueprint_course.blueprint_course_id`, `schemas/base_schema/50_functions/accounts.sql` `account.account_id`, and `schemas/base_schema/50_functions/question_pools.sql` `question_pool.question_pool_id` store the public ID as the primary key.

- [ ] An object without a public ID uses a native UUID primary key, or a composite natural key when
  it is owned by a parent (for example a Revision keyed by its lineage ID and Revision Number).
  - Reason: product decision still unclear
  - Question: Are Theme and provided-avatar vocabulary tokens objects that must use native UUID primary keys, or durable vocabulary keys that keep their text identifiers?
  - Mismatch: Question Revisions use the composite key published_question_id plus revision_number, and Attempts use a UUID. `schemas/base_schema/20_tables/theme.sql` `theme_id` and `schemas/base_schema/20_tables/profile_media.sql` `provided_avatar_id` are text vocabulary keys. `schemas/base_schema/20_tables/account.sql` `canonical_public_id` is the public-ID reservation key and is outside this bullet. One reading is that Theme and provided-avatar tokens are durable vocabulary keys, matching Theme IDs that stay stable when a display name or colors change. The other is that both are objects without a public ID and must use native UUID primary keys. Human Guidance states both the key rule and durable Theme IDs, and it does not say whether a vocabulary token is an object.

- [x] One value that identifies one object is an Id, such as a Public ID or UUID.
  - Evidence (source): `crates/question_model/src/question_library.rs` `PublishedQuestionId` is one value for one Published Question.
  - Evidence (source): `crates/question_model/src/public_route.rs` `AccountId` is one value for one Account.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `assessment_attempt_id` is one UUID for one Attempt.

- [x] Multiple values that together identify one exact object, state, or version are a Tuple.
  - Evidence (source): `crates/question_model/src/question_library.rs` `PublishedQuestionRevisionTuple` identifies one Question Revision from its Published Question ID and Revision Number.
  - Evidence (source): `crates/question_model/src/blueprint_operations/contracts.rs` `BlueprintRevisionTuple` identifies one Blueprint Revision from its Blueprint Course ID and Revision Number.

- [x] Tuple is the general cross-language term and suffix for a composite identity made from multiple values.
  - Evidence (source): `crates/question_model/src/question_library.rs` `PublishedQuestionRevisionTuple` serializes camelCase revisionNumber beside the Question ID.
  - Evidence (source): `src/pages/library_page_model.ts` `publishedQuestionRevisionTuple` reads that Tuple from library JSON.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `published_question_revision_tuple` stores that Tuple in Blueprint content.
  - Evidence (source): `src/api/course_instance.ts` `blueprintRevisionTuple` reads the Blueprint Tuple from Course JSON.

- [x] A Question Revision Tuple is one example: Question ID plus Question Revision Number.
  - Evidence (source): `crates/question_model/src/question_library.rs` `PublishedQuestionRevisionTuple` stores published_question_id and revision_number.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `revision_number` is the second column of the Question Revision primary key beside published_question_id.

- [x] A Blueprint Revision Tuple is another example: Blueprint Course ID plus Blueprint Revision Number.
  - Evidence (source): `crates/question_model/src/blueprint_operations/contracts.rs` `BlueprintRevisionTuple` stores blueprint_course_id and revision_number.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `blueprint_revision_number` is the second column of the Blueprint Revision primary key beside blueprint_course_id.

- [ ] Use Reference for a genuine indirect, scoped, or external locator.
  - Mismatch: shipped identity types use Id and Tuple. No current type uses the Reference suffix for an indirect, scoped, or external locator.

- [x] Do not name an Id or a Tuple as a Reference; "Reference" reads like a pointer, not a composite identity.
  - Evidence (source): `crates/question_model/src/question_library.rs` `published_question_revision_tuple_rejects_legacy_reference_json` refuses a reference wrapper around the Question ID and Revision Number.
  - Evidence (source): `crates/question_model/src/blueprint_operations/contracts.rs` `blueprint_revision_tuple_rejects_legacy_reference_json` refuses a reference wrapper around the Blueprint Course ID and Revision Number.

- [ ] Use the simplest term that accurately describes what the value represents.
  - Verification pending: Id and Tuple names follow the identity rules above. No repository check decides the simplest name for every remaining value.

- [x] An object without a public ID uses its UUID Id in routes and JSON. Secondary Ids are not allowed.
  - Evidence (source): `src/route_contract.ts` `ROUTE_CONTRACT` uses assessmentAttemptId, draftQuestionId, and proposalId for objects without a public ID.
  - Evidence (source): `src/navigation/route_params.ts` `routeScopeKey` accepts each of those route params only as one lowercase UUID.
  - Evidence (source): `src/api/decoders/assessment_attempt_history.ts` `decodeStudentAssessmentAttemptHistory` reads assessmentAttemptId as that UUID and attemptNumber as an ordinal.
  - Evidence (source): `src/api/decoders/blueprint_change_proposal.ts` `decodeBlueprintChangeProposalPageView` reads proposalId as that UUID.
  - Evidence (test): `tests/test_public_navigation.mjs` `objects without a public ID use their UUID in routes and reject a secondary Id` walked every ROUTE_CONTRACT path, accepted the UUID for assessmentAttemptId, draftQuestionId, and proposalId, and rejected a second token, R-1, D-50, W-40, and a public ID in those params. The Attempt scope returned only that UUID.
  - Evidence (test): `tests/test_assessment_attempt_history_decoder.mjs` `attempt history JSON uses the Assessment Attempt UUID and rejects a second attempt id` accepted assessmentAttemptId 00000000-0000-0000-0000-00000000000c with attemptNumber 2 and rejected attemptId, a UUID suffix, and attempt number 2 used as the identity.
  - Evidence (test): `tests/test_nested_identity_contracts.mjs` `change proposal JSON uses the proposal UUID and rejects a second proposal id` accepted proposalId 00000000-0000-0000-0000-00000000001e and rejected a second id field and a UUID suffix.

- [ ] Internal UUIDs never substitute for or appear as public identities.
  - Verification pending: owning tables store public IDs as primary keys, but every API, URL, export, log, and browser projection has not been inventoried.

- [ ] Table shape and clocks follow [DATABASE_STYLE.md](/docs/DATABASE_STYLE.md).
  - Mismatch: no transferred audit status after Human Guidance regeneration.

### Content classification
- [x] PLE uses one shared global content classification vocabulary for **Courses** and **Library
  Objects**.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_discipline` stores the Discipline vocabulary used by Courses and Library Objects.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subject` stores the Subject vocabulary used by Courses and Library Objects.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores a Discipline and Subject from that vocabulary on a Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores that same Discipline and Subject on a Course.

- [x] Content classification uses **Discipline** -> **Subject** -> **Topic** -> **Subtopic**.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subject_discipline` associates a Subject with a Discipline.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_topic` belongs to one Subject.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subtopic` belongs to one Topic.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores that Discipline, Subject, Topic, and Subtopic chain on a Published Question.

- [x] **Discipline** is the broad academic field, such as Biology, Chemistry, or Mathematics.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_discipline` stores the named Discipline above Subject, Topic, and Subtopic.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_discipline` stores a Sysadmin-named Discipline such as Biology.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` classifies a Published Question with that Discipline.

- [x] **Subject** identifies a global area associated with one or more Disciplines, such as Genetics,
  Biochemistry, or Ecology.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subject` stores one global Subject.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `add_content_subject_discipline` associates that Subject with another Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` returns that same Subject under each associated Discipline.

- [x] **Topic** identifies a major area within a Subject, such as Enzyme Inhibition or Chromosomal Inheritance.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_topic` stores a Topic under one Subject.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_topic` creates that Topic for a vetted Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_topics` returns that Topic for its Subject.

- [x] **Subtopic** provides a narrower classification within a Topic, such as Enzyme Catalysis Mechanisms or X-Linked Recessive Crosses.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subtopic` stores a Subtopic under one Topic.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subtopic` creates that Subtopic for a vetted Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subtopics` returns that Subtopic for its Topic.

- [x] Subjects have a global identity across PLE, and Subject names are unique across PLE.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subject` stores one Subject row for one global name.
  - Evidence (source): `schemas/base_schema/40_indexes.sql` `content_subject_global_name_unique` rejects a second Subject with the same name.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` inserts one Subject row for a new name and returns the existing Subject when that name already exists.

- [x] A Subject may be associated with one or more Disciplines.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subject_discipline` stores one association for each Subject and Discipline pair.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` records the Subject's first Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `add_content_subject_discipline` adds another Discipline and keeps the same Subject.

- [x] A Topic belongs to one Subject.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_topic` stores one content_subject_id for each Topic.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_topic` creates a Topic for one Subject.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_topics` returns that Topic under that Subject.

- [x] A Subtopic belongs to one Topic.
  - Evidence (source): `schemas/base_schema/20_tables/content_classification.sql` `content_subtopic` stores one content_topic_id for each Subtopic.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subtopic` creates a Subtopic for one Topic.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subtopics` returns that Subtopic under that Topic.

- [x] Every Course has exactly one **Discipline**.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `content_discipline_id` stores one Discipline on each Course.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` requires that Discipline and refuses a retired Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns that one Discipline.

- [x] **Subject**, **Topic**, and **Subtopic** are optional for Courses.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores a Course with Subject, Topic, and Subtopic absent, and stores another Course with all three present.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns the bare Course with those three absent and the other Course with all three present.

- [x] Every Library Object has exactly one **Discipline** and one **Subject**.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` stores one Discipline and one Subject for each Published Question.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores one Discipline and one Subject for each Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a Question without a Discipline or a Subject and stores one of each.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that Question's Discipline and Subject.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` copies that one Discipline and Subject onto the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool's Discipline and Subject.
  - Owner: Content classification (first occurrence).

- [x] **Topic** and **Subtopic** are optional for Library Objects.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` publishes a Question with Topic and Subtopic absent, and publishes another with both present.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns those Question classifications.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores a Question Pool with Topic and Subtopic absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool classification.
  - Owner: Content classification (first occurrence).

- [x] Courses retain the hierarchy because their classification supports Course organization, search,
  filtering, and discovery.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores a Course classification chain.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns those Courses in long-name order with their Discipline and Subject.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` shows that classification on each Course record.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` searches, filters, and sorts Public Blueprint Courses by classification.

- [x] Course and Library Object selections follow the hierarchy: the Subject is associated with the
  selected Discipline, the Topic belongs to that Subject, and the Subtopic belongs to that Topic.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` returns Subjects associated with the selected Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_topics` returns Topics that belong to the selected Subject.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subtopics` returns Subtopics that belong to the selected Topic.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores that chain and refuses a Subject, Topic, or Subtopic from another parent.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` requires the same parent chain on a Published Question.

- [x] Courses and Library Objects select from the same shared global vocabulary.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_disciplines` returns the active Discipline an Instructor selects for both a Published Question and a Course.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` returns the Subject associated with that Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores that Discipline and Subject on the Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores that same Discipline and Subject on the Course.

#### Classification vocabulary management
- [x] **Sysadmins** exclusively create and manage the Discipline vocabulary and its lifecycle.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_discipline` creates a Discipline only for an active Sysadmin.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `rename_content_discipline` changes that Discipline's name only for an active Sysadmin.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `retire_content_discipline` retires that Discipline and `restore_content_discipline` returns it to the active vocabulary.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_disciplines` omits a retired Discipline, and `get_content_discipline` still returns the same Discipline id.

- [x] Discipline is a stable vocabulary expected to change infrequently.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `rename_content_discipline` keeps one Discipline id across two renames, and an Instructor rename is refused.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `retire_content_discipline` keeps that id.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_disciplines` omits that Discipline while it is retired.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `get_content_discipline` returns that same id while it is retired.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `restore_content_discipline` returns that same id to the active vocabulary.

- [x] **Instructors** classify content by selecting from the Sysadmin-managed Disciplines.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_disciplines` returns the active Sysadmin Disciplines to an Instructor and omits a retired Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores the Discipline selected from that list and refuses a retired Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that selected Discipline on the Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores that selected Discipline on a new Course and refuses a retired Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns that selected Discipline on the Course.

- [x] **Instructors** may create new Subjects within a selected Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` creates a Subject and associates it with the selected Discipline for a vetted Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` returns that Subject for the Discipline.

- [x] When an Instructor attempts to create a Subject whose globally unique name already exists, PLE
  offers the existing Subject.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` returns the existing Subject for a duplicate name and does not add the other Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` omits that Subject from the other Discipline until acceptance.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `add_content_subject_discipline` associates that returned Subject only when the Instructor calls it.

- [x] PLE requires explicit Instructor acceptance before associating the existing Subject with the
  selected Discipline.
  - Evidence (source): `schemas/base_schema/40_indexes.sql` `content_subject_global_name_unique` keeps one global Subject name.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` inserts a new Subject and its first Discipline association. It does not attach an existing Subject to another Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `add_content_subject_discipline` associates that existing Subject with another Discipline only when the Instructor calls it.

- [x] **Instructors** may create new Topics within a Subject.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_topic` creates a Topic belonging to the selected Subject for a vetted Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_topics` returns that Topic for the Subject.

- [x] **Instructors** may create new Subtopics within a Topic.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subtopic` creates a Subtopic belonging to the selected Topic for a vetted Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subtopics` returns that Subtopic for the Topic.

- [x] Creating or selecting vocabulary should fit naturally into the classification workflow.
  - Evidence (source): `src/components/authoring_classification_level.tsx` `AuthoringClassificationLevel` places Subject, Topic, and Subtopic creation beside the classification selector.
  - Evidence (source): `crates/server/src/content_classification.rs` `create_subject` creates a Subject in the selected Discipline or offers an existing global name for acceptance.
  - Evidence (test): `tests/test_content_classification_client.mjs` `createSubject` posts a new Subject and accepts an existing Subject on the shipped client.
  - Evidence (test): `tests/playwright/ribbon_shell_contract.mjs` `Creating or selecting vocabulary should fit naturally into the classification workflow.` creates and selects Subject, Topic, and Subtopic on the Course classification form.

#### Classification selection and discovery
- [x] Classification selection, browsing, and filtering begin with Discipline.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` places Discipline first and uses that Discipline as the Subject parent.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `BlueprintSearchClassification` hides Subject until a Discipline is selected.
  - Evidence (source): `src/api/library_classification_filter.ts` `libraryClassificationFilter` rejects a Subject without a Discipline.
  - Evidence (source): `crates/question_model/src/question_search.rs` `normalized` rejects a Subject without a Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` rejects a Subject, Topic, Subtopic, or cross-Discipline option whose parent is missing.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns no rows when a Subject is set without a Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` rejects a Subject without a Discipline.

- [x] Course and Library Object classification follow Discipline -> Subject -> Topic -> Subtopic,
  progressively narrowing the available choices at each level.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` limits Subject choices to the selected Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_topics` limits Topic choices to the selected Subject.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subtopics` limits Subtopic choices to the selected Topic.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores one chain from those choices and refuses a choice from another parent.

- [x] Selecting a Discipline limits Subject choices to Subjects associated with that Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` associates a new Subject with the selected Discipline.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` returns that Subject for the selected Discipline and returns no Subject for another Discipline.

- [x] After selecting a Subject, search interfaces may offer an explicit option to include content
  associated with that Subject across its other Disciplines.
  - Evidence (source): `src/components/library_classification_search.tsx` `LibraryClassificationSearch` shows Include this Subject across Disciplines only after a Subject is selected.
  - Evidence (source): `src/pages/blueprint_course_search_classification.tsx` `BlueprintSearchClassification` shows that option only after a Subject is selected.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` returns the selected Discipline when the option is off and both Disciplines when it is on.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns the selected Discipline when the option is off and both Disciplines when it is on.

- [x] Classification supports searching, filtering, sorting, organization, and discovery wherever those capabilities are useful.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` filters Published Questions by classification and sorts that page.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` filters Question Pools by classification.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` filters and sorts Public Blueprint Courses by classification and name.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns Course classification in long-name order.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` shows that classification on the Course list.

#### Tags and classification names
- [x] **Tags** provide flexible labels outside the Discipline, Subject, Topic, and Subtopic hierarchy.
  - Evidence (source): `schemas/base_schema/15_table_check_functions.sql` `course_classification_tags_are_valid` accepts distinct trimmed labels and does not require a Discipline, Subject, Topic, or Subtopic.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `course_instance` stores those labels beside an optional Subject, Topic, and Subtopic.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores tags on a Course whose Subject, Topic, and Subtopic are absent.
  - Evidence (source): `schemas/base_schema/15_table_check_functions.sql` `question_metadata_tags_are_valid` accepts the same kind of label list for a Library Object.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores tags on a Published Question whose Topic and Subtopic are absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores a caller-chosen tag list that is not copied from the member Question.

- [x] Courses and Library Objects may have any number of Tags, including none.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores a Course with no tags and a Course with several tags.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns those Course tag lists.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores a Question with no tags and a Question with several tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns those Question tag lists.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores a Question Pool with no tags and a Question Pool with several tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns those Pool tag lists.

- [x] Subject, Topic, and Subtopic names must satisfy length limits and formatting requirements.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `normalize_content_classification_name` strips boundary whitespace, then rejects an empty name, a control character, or a name outside the caller's length limit.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` stores a Subject only after that normalization. `create_content_topic` and `create_content_subtopic` use the same check.

- [x] Length allowances increase from Subject to Topic to Subtopic, supporting more specific names as classification becomes narrower.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` limits a Subject name to 120 characters.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_topic` limits a Topic name to 240 characters.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subtopic` limits a Subtopic name to 480 characters.

- [x] Strip leading and trailing whitespace from Subject, Topic, and Subtopic names and validate the resulting names consistently.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `normalize_content_classification_name` removes leading and trailing whitespace before the shared length and control-character check.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_subject` stores that stripped Subject name. `create_content_topic` and `create_content_subtopic` store the stripped Topic and Subtopic names.

### Student and FERPA data
- [x] **Student** course data falls under FERPA; treat it as radioactive.
  - Evidence (source): `schemas/base_schema/20_tables/course_membership.sql` `student_record` is protected by RLS and has no PUBLIC privilege.

- [x] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Evidence (source): `schemas/base_schema/50_functions/course_roster.sql` `course_roster_profile` contains no duplicate Student email; ordinary roster is email-free and direct-Instructor-only.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` removes identifiable Course Student records while retaining Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 purge-preservation probe on 2026-09-15.
  - Owner: 02_accounts.md / Student role (first occurrence; identical requirement and status).

- [x] FERPA access should be scoped through exact Course membership and **Student** ownership.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_owns_student_record` allows the session only when that Account owns the Student record through an active Student membership in that Course.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history` returns a submitted Attempt only when that ownership holds.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `assert_current_student_assessment_attempt` refuses the saved response unless that ownership holds.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_access` returns only the session Student's Assessment view for that Course.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `ferpa_access_follows_course_membership_and_student_ownership` let USSV00009 read the submitted Attempt and saved response on CISV000007, hid that Attempt from same-Course Student USSV0001H, and refused non-member USSV00022 and Instructor UVSV0000A. Membership in CISV000016 did not authorize the first Student record.
  - Evidence (test): `tests/e2e/e2e_assessment_saved_response.sh` `ferpa_access_follows_course_membership_and_student_ownership` ran that oracle on PostgreSQL. No Live Demo stack was started.

- [x] **Sysadmins** receive only the FERPA access required for a specific administrative task.
  - Evidence (source): `crates/server/src/support_capability.rs` `support_capability_router` exposes only a scoped, revocable exact-record repair reader; it has no whole-Course roster route.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue` exercises named-record repair issuance, concealment, use, and revocation.

- [x] Student Accounts persist independently of Course data and Course retention.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account` is separate from course-scoped `student_record`.

- [x] Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` deletes private Attempt roots and Course-scoped Student records only after the archived state, while retaining Course teaching material and identity-free aggregate rows.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL actual-role proof with a released Assessment, issued Question, saved response, whole-Assessment submission, `question_response`, grading result, grading receipt, and aggregate; deletion removed the identifiable Student Work descendants at the stored expiry.

- [x] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` deletes Course-scoped Student records and private Student evidence without deleting Course, Assessment, Question, or configuration relations.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL actual-role proof retaining the global Student Account, Course, Assessment, Published Question, immutable source, settings, and an unrelated Course membership after the populated Student Work was deleted.
  - Owner: 06_data.md / Student and FERPA data (first occurrence; identical requirement and status).

- [x] **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_work_records` lists Assessment Attempts, Question Pool selections and selected items, Issued Questions, Question Attempts, saved responses, submissions, presentation bindings, grading results, and automated grading receipts for one Student Record, and `read_course_student_work_for_retention` reads Attempt rows from that collective.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `student_work_is_the_collective_term_for_course_records_that_keep_their_identities` assembled one Student Record's Attempt, Question Pool selection, Issued Question, Question Attempt, saved response, and grading result as Student Work. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student Work includes Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret that work after an Attempt is submitted.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history` joins Attempt, issued question, response, submission, grading, and receipt evidence.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies retained response finalization and resulting score evidence.

- [x] Student Work is an umbrella term; the underlying records retain their own identities and purposes.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_work_records` returns each record kind with that record's own id, member position, presentation response item id, or Question Image Asset id.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `student_work_is_the_collective_term_for_course_records_that_keep_their_identities` kept the Attempt, Question Pool selection, Issued Question, Question Attempt, and saved-response ids distinct and refused a foreign Student Record. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student retention removes identifiable Student evidence and leaves Question usage statistics
  unchanged.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` removed the Attempt, observation receipt, and Student record for Course CIA0000003 after the issued count was 1. A fresh read still returned issued 1. Instructor UXR5JYCDE and Student UBA7QPMZF remained. `start_assessment_attempt` issued 7A3M-V0A1. The response, submission, and grading row were inserted directly so the observation could record one answered use. No Question Backend was called. A disposable PostgreSQL session ran that check and was not kept.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_usage_statistics` returned issued 1, answered 1, and correct 1 both before `archive_course_student_records` and after deletion.

### Course retention and lifecycle
- [x] Course retention should follow Course Instance dates and its six-month Active lifetime rather than
  a fixed academic calendar.
  - Evidence (source): `schemas/base_schema/50_functions/course_core.sql` `enforce_course_instance_retention_schedule` sets active_until_at from created_at plus six months and rejects any other lifetime.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `course_instance` stores term_ends_on and requires that date to fall on or before the UTC date of active_until_at.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_core.sql` `enforce_course_instance_retention_schedule` stored summer CIB1000008 created 2024-06-03 with term end 2024-07-26 and active_until 2024-12-03, quarter CIB200000E created 2024-03-25 with term end 2024-06-07 and active_until 2024-09-25, and semester CIB300000B created 2024-01-16 with term end 2024-05-10 and active_until 2024-07-16. Each retention_starts_at matched that active_until. Supplying 2024-07-26 as the summer lifetime was refused, and a summer term ending 2024-12-04 was refused. The Courses were inserted directly. A disposable PostgreSQL session ran that check and was not kept.

- [x] The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA
  retention clock.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `retention_starts_at` must equal the latest Assessment Due date when that date is present, and otherwise the six-month active_until_at.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `synchronize_course_assessment_deadline` stores the current maximum Due date of unreleased and released Assessments and sets the active Course retention clock to that date.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `synchronize_course_assessment_deadline` moved Course CIK1000000, created 2025-01-01 with active_until 2025-07-01, from a retention clock equal to that lifetime to 2025-06-01. That instant was the Due date of released Assessment AK100000Z, and latest_assessment_due_at stored the same instant. The Course stayed active. The Course, Accounts, and Assessment were inserted directly. A disposable PostgreSQL session ran that check and was not kept. No Question Backend was called.

- [x] Creating or extending a later Assessment deadline may move those dates, but not beyond the
  six-month Active lifetime.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies` lock the Course first, reject a Due date after its immutable `active_until_at`, and invoke `ple_data.synchronize_course_assessment_deadline` after an accepted change.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` stores the current maximum Assessment Due date and moves the active Course retention anchor to that date, or to `active_until_at` when no Due date remains.
  - Evidence (runtime): accepted independent PostgreSQL actual-API proofs exercised `schemas/base_schema/50_functions/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies`, covering release, a cleared last Due date, cap rollback, stale CAS, wrong-Instructor denial, deterministic concurrent saves to two Assessments, an archive race, and frozen archived/deleted retention anchors.

- [x] Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `synchronize_course_assessment_deadline` writes the latest Due date and the retention clock and does not insert a notification or change archive, deletion, or Course lifecycle columns.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_notifications.sql` `claim_course_retention_notification` inserts a receipt only for a warn_inactive or notify_archive action that is already due.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `synchronize_course_assessment_deadline` moved Course CIK1000000 to retention clock 2025-06-01. course_lifecycle_state stayed active, retention_lifecycle_state stayed active, course_became_inactive_at stayed null, and student_data_archived_at and student_data_deleted_at stayed null. The Student record for UXVN6BDZW remained.
  - Evidence (runtime): `schemas/base_schema/50_functions/student_course_attempt_history.sql` `list_live_student_course_attempt_history` returned Attempt a5000000-0000-0000-0000-000000000011 for Student UXVN6BDZW before and after that clock move.
  - Evidence (runtime): `schemas/base_schema/50_functions/grading_access.sql` `read_course_gradebook` returned one row for Instructor U8QGEKSFF before and after that clock move. `schemas/base_schema/50_functions/course_retention.sql` `course_student_work_is_ordinarily_visible` stayed true.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_notifications.sql` `claim_course_retention_notification` at 2025-06-01 returned no receipt for Course CIK1000000, and no notification row was stored. No mail transport ran. The Course, Accounts, released Assessment, roster, and Assessment Attempt were inserted directly. A disposable PostgreSQL session ran that check and was not kept. No Question Backend was called.

- [x] The configured FERPA retention policy determines the later notice, archive, recovery, and
  permanent deletion transitions.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `retention_schedule` reads ple.retention_inactive_warning_lead_time, ple.retention_archive_notice_lead_time, ple.retention_archive_after_retention_start, and ple.retention_delete_after_archive.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` returns warn_inactive, notify_archive, archive, and delete from that schedule.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `archive_course_student_records` archives at retention_starts_at plus archive_after_retention_start, and delete_course_student_records deletes at that instant plus delete_after_archive.
  - Evidence (source): `schemas/base_schema/50_functions/archived_student_work_recovery.sql` `lock_archived_course_for_recovery` returns the live delete deadline, and select_archived_assessment_attempts_for_recovery raises when clock_timestamp is no longer before that deadline.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` for Course CIR100000V, Instructor U682DAX5X, active_until and retention_starts 2024-12-03 00:00:00+00, returned default warn 2024-11-19 00:00:00+00, notify 2025-01-02 00:00:00+00, archive 2025-03-13 00:00:00+00, and delete 2025-12-03 00:00:00+00. Setting those four intervals to 10 days, 40 days, 90 days, and 200 days moved the same four dues to 2024-11-23 00:00:00+00, 2025-01-22 00:00:00+00, 2025-03-03 00:00:00+00, and 2025-09-19 00:00:00+00. With archive after 90 days, archive_course_student_records refused one second before 2025-03-03 00:00:00+00 and returned true at that instant. With delete after 1000 days, lock_archived_course_for_recovery returned 2027-11-28 00:00:00+00. Shortening delete after to 1 day made select_archived_assessment_attempts_for_recovery raise 42501 while the Student record still existed, refused delete one second before 2025-03-04 00:00:00+00, and at 2025-03-04 00:00:00+00 delete_course_student_records returned true and left 0 Student records. The Course, Accounts, and Student record were inserted directly. A disposable PostgreSQL session ran that check and was not kept. No Question Backend was called.

- [x] PLE warns the **Instructors** before the Course Instance becomes Inactive six months after
  creation.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` schedules warn_inactive at active_until_at minus the configured lead while the Course is still active.
  - Evidence (source): `crates/server/src/composition.rs` `run_course_retention_process_from_env` selects SMTP delivery when both provider settings are present and then runs the retention worker.
  - Evidence (source): `crates/server/src/course_retention_notification_delivery.rs` `SmtpCourseRetentionNotificationDelivery` sends the fixed sign-in sentence and returns success only after the SMTP transport accepts the message.
  - Evidence (runtime): `crates/server/src/course_retention_worker.rs` `run_until_shutdown` ran as server_core --course-retention-worker on 2026-09-30. It claimed warn_inactive for still-active Course CIN100000A, active_until_at 2026-10-07 10:41:20.955465+00, and the SMTP provider accepted the sign-in sentence addressed to Instructor U0R8YSJWC at retention-warn@example.edu with idempotency key 43aaba1f-ff6a-4c34-84ca-d297ac50d577. The receipt stored provider_accepted_at and last_failure_kind stayed null. course_became_inactive_at stayed null, student_data_archived_at stayed null, and the Student record for U0EAQGJ45 remained. The message did not contain the Course id. The Course, Accounts, and roster were inserted directly. A disposable PostgreSQL session and a local SMTP listener ran that check and were not kept. No Question Backend was called. The SMTP 250 response is provider acceptance, not an inbox-delivery claim.

- [x] The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying
  FERPA retention and deletion.
  - Evidence (source): `schemas/base_schema/50_functions/course_core.sql` `ple_data.enforce_course_instance_retention_schedule` derives and preserves the immutable six-month `active_until_at`; `schemas/base_schema/50_functions/assessments.sql` `ple_data.save_assessment` and its sibling save functions reject every saved Due date beyond that cutoff.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` bounds the active retention anchor by the accepted current maximum Due date or that immutable cutoff and does not move an archived or deleted anchor.
  - Evidence (runtime): accepted independent PostgreSQL actual-API proofs exercised `schemas/base_schema/50_functions/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline`, rejecting over-cap saves without partial state, keeping concurrent current deadlines synchronized, and preserving the retention anchor after archive while later Assessment facts changed.

- [x] Course inactivity and FERPA deletion are separate transitions; becoming Inactive does not itself
  delete Student records.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `mark_course_instance_inactive` sets course_lifecycle_state to inactive and course_became_inactive_at to active_until_at.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` deletes Student records only from the archived retention state.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `mark_course_instance_inactive` refused 2023-07-14 for Course CIC100000D, then at 2023-07-15 returned true and a later call returned false. The Student record remained for Account UBVMVNJCV. retention_lifecycle_state stayed active, and student_data_archived_at and student_data_deleted_at stayed null. The Course, Accounts, and Student record were inserted directly. A disposable PostgreSQL session ran that check and was not kept. No Question Backend was called.

- [x] Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_core.sql` `enforce_course_instance_retention_schedule` applied created_at plus six months to summer CIB1000008, quarter CIB200000E, and semester CIB300000B. Their term ends stayed 2024-07-26, 2024-06-07, and 2024-05-10. The Courses were inserted directly. A disposable PostgreSQL session ran that check and was not kept.

- [x] PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` schedules notify_archive before the archive transition while student_data_archived_at is null.
  - Evidence (source): `crates/server/src/course_retention_worker.rs` `run_iteration` claims due notices before it applies mark, archive, or delete.
  - Evidence (source): `crates/server/src/course_retention_notification_delivery.rs` `deliver_claimed_course_retention_notification` records provider acceptance only after submit returns success.
  - Evidence (runtime): `crates/server/src/course_retention_worker.rs` `run_until_shutdown` ran as server_core --course-retention-worker on 2026-09-30. It claimed notify_archive for Course CIP100000T, retention_starts_at 2026-08-21 10:41:20.955522+00, while student_data_archived_at was null. The SMTP provider accepted the sign-in sentence addressed to Instructor UKC8GE644 at retention-archive@example.edu with idempotency key d24bc786-2d63-428a-9354-e57a56a403b9. The receipt stored provider_accepted_at and last_failure_kind stayed null. The Course stayed active, the Student record for UTMV62309 remained, and the message did not contain the Course id. The Course, Accounts, roster, and released Assessment were inserted directly, and synchronize_course_assessment_deadline set the retention clock from that Assessment Due date. A disposable PostgreSQL session and a local SMTP listener ran that check and were not kept. No Question Backend was called. The SMTP 250 response is provider acceptance, not an inbox-delivery claim.

- [x] Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `course_student_work_is_ordinarily_visible` is true only while retention_lifecycle_state is active.
  - Evidence (source): `schemas/base_schema/50_functions/grading_access.sql` `read_course_gradebook` reads a Course only while that retention state is active.
  - Evidence (source): `schemas/base_schema/50_functions/student_course_attempt_history.sql` `list_live_student_course_attempt_history` resolves the Student only while that retention state is active.
  - Evidence (source): `schemas/base_schema/50_functions/archived_student_work_recovery.sql` `select_archived_assessment_attempts_for_recovery` returns an archived Assessment Attempt for the Course Instructor and refuses a Course that is not archived.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `archive_course_student_records` refused 2026-06-08 for Course CIH100000P, then at 2026-09-01 returned true and a later call returned false. Before that archive, `list_live_student_course_attempt_history` as Student UTJE7MSCY returned one Attempt history row and `read_course_gradebook` as Instructor U193H1VV3 returned one row, while `select_archived_assessment_attempts_for_recovery` was refused. After it, the Student history call was refused, the gradebook returned no row, and the recovery read returned Assessment Attempt a3000000-0000-0000-0000-000000000011 for Assessment AH1000006. student_data_deleted_at stayed null and the Student record remained. The Course, Accounts, released Assessment, roster, and Assessment Attempt were inserted directly. A disposable PostgreSQL session ran that check and was not kept. No Question Backend was called.

- [x] FERPA-sensitive Student data should be permanently deleted when its retention period expires.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` is an archived-Course deletion transition and is repeat-safe after a deleted state.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` refused a premature transition, deleted populated Student Work at the stored due time, returned false on repeat, and serialized two concurrent executors as one true transition followed by one false reread in fresh PostgreSQL actual-role proof.

- [x] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` deletes Course-scoped Student records and private Student evidence without deleting Course, Assessment, Question, or configuration relations.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention_transitions.sql` `ple_api.delete_course_student_records` passed a fresh PostgreSQL actual-role proof retaining the global Student Account, Course, Assessment, Published Question, immutable source, settings, and an unrelated Course membership after the populated Student Work was deleted.
  - Owner: 06_data.md / Student and FERPA data (first occurrence; identical requirement and status).

- [x] FERPA retention intervals are operational configuration rather than separate product decisions.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `retention_schedule` reads ple.retention_archive_after_retention_start and the sibling retention interval settings, using defaults of 14 days, 70 days, 100 days, and 265 days.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` as ple_course_retention_executor scheduled summer CIB1000008 archive at 2025-03-13 under the 100-day default and at 2025-03-03 after ple.retention_archive_after_retention_start was set to 90 days. active_until stayed 2024-12-03. A disposable PostgreSQL session ran that check and was not kept.

### Course retention processing
- [x] A background process should periodically find Course Instances whose retention deadlines have passed.
  - Evidence (source): `crates/server/src/course_retention_worker.rs` `run_until_shutdown` starts a sweep immediately and repeats it every `RETENTION_SWEEP_INTERVAL`.
  - Evidence (source): `crates/learning-data-access/src/postgres/retention.rs` `list_due_course_retention_actions` reads `course_retention_due_actions` before the worker applies a transition.
  - Evidence (runtime): `crates/server/src/course_retention_worker.rs` `run_until_shutdown` ran as server_core --course-retention-worker on 2026-09-30. Its first sweep claimed passed warn_inactive for still-active Course CIN100000A and passed notify_archive for Course CIP100000T, whose retention_starts_at was 2026-08-21 10:41:20.955522+00 and whose Student data was not archived. The fixture had stored no notification receipts, so the worker created them. Both Courses stayed active, so that sweep did not apply mark, archive, or delete. The process logged shutdown 166 milliseconds after it selected SMTP delivery, before the 60-second repeat. The Courses, Accounts, roster, and released Assessment were inserted directly. A disposable PostgreSQL session and a local SMTP listener ran that check and were not kept. No Question Backend was called.

- [x] Retention decisions should come from stored Course dates and the Course Instance creation time.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` schedules mark_inactive at the stored active_until_at and schedules archive and delete from retention_starts_at plus the configured intervals.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` evaluated at 2026-01-01 returned summer CIB1000008 mark_inactive at 2024-12-03, archive at 2025-03-13, and delete at 2025-12-03. Those instants follow the Course created on 2024-06-03, not its 2024-07-26 term end. The read ran as ple_course_retention_executor. A disposable PostgreSQL session ran that check and was not kept.

- [x] The background process should execute retention policy rather than define when retention periods begin or end.
  - Evidence (source): `crates/server/src/course_retention_worker.rs` `run_iteration` dispatches mark_inactive, archive, and delete from the actions returned by list_due_course_retention_actions and does not compute retention intervals.
  - Evidence (source): `crates/learning-data-access/src/postgres/retention.rs` `list_due_course_retention_actions` reads course_retention_due_actions and calls the matching transition procedure.
  - Evidence (test): `crates/server/src/course_retention_worker.rs` `course_retention_delete_follows_the_course_not_the_account` passed. run_iteration executed the stored delete for one Course and did not mark that Course inactive or archive it. The test supplied that delete through a recording store and did not query PostgreSQL.

- [x] Running the retention process late should produce the same retention decision as running it on schedule.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` returns each unperformed action at its stored due_at once the evaluated instant reaches that due_at.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `mark_course_instance_inactive` stores course_became_inactive_at as the Course active_until_at.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_retention.sql` `course_retention_due_actions` for Course CIR100000V printed late_warn warn_inactive=2024-11-19 00:00:00+00 after reading that due instant and six days later, and late_mark warn_inactive=2024-11-19 00:00:00+00,mark_inactive=2024-12-03 00:00:00+00 after reading active_until 2024-12-03 00:00:00+00 and twelve days later. mark_course_instance_inactive at the later instant stored course_became_inactive_at 2024-12-03 00:00:00+00. The Course, Accounts, and Student record were inserted directly. A disposable PostgreSQL session ran that check and was not kept. No Question Backend was called.

- [x] The retention process should be safe to run repeatedly.
  - Evidence (source): `crates/server/src/course_retention_worker.rs` `run_iteration` claims due notices and then applies the mark, archive, and delete actions listed for that instant.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_notifications.sql` `claim_course_retention_notification` compares a due action as text and returns only an unaccepted notice receipt.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `mark_course_instance_inactive` returns false when the Course is already inactive, `archive_course_student_records` returns false when retention is archived or deleted, and `delete_course_student_records` returns false when retention is deleted.
  - Evidence (runtime): `crates/server/src/course_retention_worker.rs` `run_until_shutdown` ran twice as server_core --course-retention-worker on 2026-09-30 for Course CIS100000Z, Instructor U46SS8JCE. The first pass selected SMTP, submitted two notices, stored course_became_inactive_at 2024-12-03 00:00:00+00, and stored student_data_archived_at and student_data_deleted_at 2026-09-30 11:11:07.36+00. It left two accepted receipts, zero Student records, and no due action. The second pass selected SMTP at 2026-09-30 11:11:08.711784Z and logged shutdown at 2026-09-30 11:11:16.823664Z. It recorded no submission and no transition failure. The inactive, archive, and deletion timestamps, the two receipts, and the zero Student records stayed the same, and no due action remained. The Course, Accounts, roster, and verified Instructor email were inserted directly. A disposable PostgreSQL session and a local SMTP listener ran that check and were not kept. No Question Backend was called. The SMTP 250 response is provider acceptance, not an inbox-delivery claim.

### Common revision and history specifications
- [x] Be conservative about creating revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_question_revision` locks the Draft and immediate parent before it creates a successor; its `PQR01` source-checksum comparison rejects an unchanged `question_revision_source_binding` before any successor facts are written.
  - Decision: A fresh one-time PostgreSQL probe exercised `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_question_revision` and verified title and description metadata changes make no Revision, an unchanged source is rejected without a partial write, a changed source creates the next Revision, and two serialized sessions admit only one successor. Tags, subject, and topic have no persisted metadata fields yet; the probe asserts that present absence rather than inventing a field-level behavior. The probe is temporary and will be removed, not cited as permanent evidence.

- [x] Assessments, Course Instances, Draft Questions, and Question Pools use current state.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `save_assessment_inline` advanced Assessment A9FM0000C from Edit Number 1 to 2 and left one current Assessment row. A later Pool import advanced that same row again. No assessment_revision relation exists.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `update_course_classification` advanced Course Instance CI9F100006 from Edit Number 1 to 2 and left one current Course row. No course_instance_revision relation exists.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft` replaced one Draft Question, advanced its Edit Number from 1 to 2, and left that one Draft row. No draft revision relation exists.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stored Pool 8K3M-J9F1 at Edit Number 1. `schemas/base_schema/50_functions/assessment_pool_forks.sql` `append_assessment_question_pool_fork_members_for_course` advanced fork Pool 8K3M-09F2 from Edit Number 1 to 2, left one Pool row, and left the member pin on Question Revision 2. No question_pool_revision relation exists.

- [x] Published Questions and Blueprint Courses have immutable revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` refused an update of Question Revision 1 on 7K3M-V9F1 and left Revisions 1 and 2 stored.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_revision_integrity.sql` `blueprint_course_revision_is_immutable` refused an update of Blueprint Revision 1 on BP9F10000N and left Revisions 1 and 2 stored, including the original empty-module Revision 1.

- [x] Mutable working state uses a monotonic sequential Edit Number when needed for concurrency.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `save_assessment_inline` requires and advances `assessment_edit_number`.

- [x] An Edit Number is only a counter and does not identify a stored historical object.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `assessment_edit_number` is a current-state concurrency field rather than a revision foreign key.

- [x] Question and Blueprint Revision Numbers start at 1 and increase sequentially for each object.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stored Question Revision 1 for 7K3M-V9F1, and `publish_question_revision` stored Revision 2 on that same Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `create_blueprint_course` stored Blueprint Revision 1 for BP9F10000N, and `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `save_blueprint_course` stored Revision 2 for that same Blueprint Course.

- [x] A Revision Number identifies a specific immutable Revision stored by PLE.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` stored Revision 2 as its own question_revision row beside Revision 1 of Published Question 7K3M-V9F1. Blueprint Revisions 1 and 2 are separate rows of Blueprint Course BP9F10000N.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `question_pool_edit_number` is the current Pool counter. The same session found no revision_number column on question_pool and no question_pool_revision relation.

- [x] A new Revision keeps the same Published Question ID.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` stored Revision 2 of Published Question 7K3M-V9F1, and Revision 1 of that Question uses the same Published Question ID.

- [x] Forking a Published Question or Question Pool creates a new public ID.
  - Evidence (source): `crates/server/src/question_fork.rs` `fork_published_question` issues `forked_question_id` before the fork-to-Draft operation; `schemas/base_schema/50_functions/question_pools.sql` `ple_data.construct_question_pool_pin_fork` inserts the fork as a new Pool lineage with `p_public_question_pool_id`.

- [x] A Published Question fork starts at Revision 1 under its new ID.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `fork_published_question_to_draft` copied Revision 1 of 7K3M-V9F1 into a new Draft. `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stored Revision 1 under new Published Question 7K3M-89F2, and question_fork_source records that source Revision. The source Question still has Revisions 1 and 2.

- [x] A Question Pool fork starts at Edit Number 1 under its new ID; Pools have no Revision family.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork_for_ids` created Pool 8K3M-09F2 at Edit Number 1 from source Pool 8K3M-J9F1 and set source_question_pool_id to that source. question_pool keeps question_pool_edit_number, has no revision_number column, and has no question_pool_revision relation.

- [x] Student Work records the exact Assessment Attempt and Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempts.sql` `issued_question` records Attempt identity with `question_id` and `revision_number`.

- [x] Student Work records the Student's responses and the grading outcome returned by the Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history` returns retained responses and grading results.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies the finalization grading outcome.

- [x] For a Question served from a Question Pool, Student Work pins all four: the Published Question
  ID, its Revision Number, the Question Pool ID, and the Pool's Edit Number at selection time.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` stored fork Pool 8K3M-H0K4 at Edit Number 1 with Published Questions 7K3M-T0K1 and 7K3M-F0K2, both at Revision 1.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_attempt_pool_selection` returned 8K3M-H0K4:1:1:7K3M-T0K1:1 and 8K3M-H0K4:1:2:7K3M-F0K2:1, which is the Pool ID, that Edit Number, each Published Question ID, and Revision 1.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `append_assessment_question_pool_fork_members_for_course` then stored Pool Edit Number 2 and moved 7K3M-T0K1 to Revision 2. The stored selection stayed at Edit Number 1 and Revision 1. The Questions were inserted directly. The credit passed to commit was an input. No Question Backend was called.

- [x] Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `score_recorded_credit` calculates current points from retained `normalized_credit`.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `replay_score` changes points and verifies retained credit is replayed.

- [x] Changes to Assessment settings do not change the recorded history of completed Assessment Attempts.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history` deliberately interprets retained Attempt evidence rather than current Assessment content.

- [x] Immutable Question source and Question Image Assets use SHA-256 checksums where needed to verify their stored contents.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `source_object_checksum` binds immutable Question-source contents to SHA-256 object records.
  - Evidence (source): `schemas/base_schema/20_tables/question_images.sql` `public_object_checksum` binds immutable Question-asset contents to SHA-256 object records.

- [x] A public-ID checksum is one embedded character derived from other ID characters.
  - Evidence (source): `crates/question_model/src/question_library.rs` `public_id_checksum_character` derives one Crockford character from the canonical ID characters.

- [x] A stored-content checksum is a full SHA-256 value verifying exact bytes.
  - Evidence (source): `crates/question_model/src/student_work/source_object_checksum.rs` `SourceObjectChecksum` accepts exactly one 64-character lowercase SHA-256 hexadecimal value.

- [x] Public-ID checksums and stored-content checksums are not interchangeable.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId` embeds one Crockford checksum character, while `crates/question_model/src/student_work/source_object_checksum.rs` `SourceObjectChecksum` is a separate full-digest type with incompatible validation.

### Dates and time zones
- [x] Assessment deadlines are stored as instants.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `due_at` stores the Assessment deadline as `timestamptz`.

- [x] Instructor dates and times use the Instructor's IANA time zone.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account_time_zone_is_exact_iana` validates Instructor account zones against `pg_timezone_names`.

- [x] The Instructor's time zone is used to interpret dates and times the Instructor enters.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `resolve_in_account_time_zone` resolves entered release times with the account zone.

- [x] Changing an Instructor's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `LocalDateAndTime::from_activity_timestamp_in_account_time_zone` derives display values from stored timestamps and account zone.

- [x] Assessment deadlines are stored as absolute UTC instants.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `due_at` stores that deadline as `timestamptz`, which PostgreSQL keeps as an absolute UTC instant.

- [x] Students have their own IANA time zone for displaying dates and times.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account_time_zone_is_exact_iana` validates each Account's exact IANA time-zone preference.

- [x] A Student's time zone defaults to the Instructor's time zone during the invite phase.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `apply_student_invitation_time_zone_default` copies the Instructor preference while pending.

- [x] Changing a Student's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_access` returns stored deadlines and separately reads `display_time_zone`.

- [x] Changing a display time zone changes how a deadline is shown, not the deadline itself.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_attempt_context` returns `display_time_zone` separately from `expires_at_millis`.

- [x] I want times in my account formatted in the selected display zone, with the zone name shown only on
  Profile.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `useSelectedDisplayDateTimeFormatter` formats Watch activity from the signed-in Account zone. Blueprint Course detail, Change Proposals, and library discussion use that same formatter.
  - Evidence (source): `src/pages/profile_page.tsx` `profile-time-zone` shows the IANA zone name.
  - Evidence (source): `src/format_datetime.ts` `createDisplayDateTimeFormatter` formats that zone with date and time styles and no zone name.
  - Evidence (test): `tests/test_student_time_zone.mjs` `useSelectedDisplayDateTimeFormatter` renders 2026-01-15 18:30 UTC as Jan 15, 2026, 1:30 PM in America/New_York and 10:30 AM in America/Los_Angeles, and neither string contains the zone name.
## Question specifications
- [x] Questions are subject agnostic. Properly classified Published Questions from all subjects belong in
  the same Question Library.
  - Evidence (source): `crates/server/src/question_library/paging.rs` `QuestionSearchFilter` supplies the shared Library query filter without a subject partition.

- [x] Questions are strictly and deterministically automated; grading does not require an **Instructor**.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `record_direct_automated_grading_result` stores backend credit with no Instructor argument, and `reject_grading_evidence_change` keeps that result immutable.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` grades the compiled answer key and takes no Instructor.
  - Evidence (source): `crates/adapters/webwork/src/lib/grade.rs` `grade` sends the seeded Student response to the renderer and takes no Instructor.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `questions_are_strictly_and_deterministically_automated_grading_does_not_require_an_instructor` graded blue twice at credit 1 and red twice at credit 0.
  - Evidence (test): `crates/adapters/webwork/src/http_renderer/tests.rs` `grade_forwards_ordered_pairs_once_with_trusted_fields` mapped renderer scores 0, 0.5, and 1 to those credits with `isInstructor=0` and problem seed 7.

- [x] Questions have one canonical title. Compact interfaces may truncate that title.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` stores one lineage-level title.

- [x] Every Question stored by PLE has its own internal Question record.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question` owns the internal Question record.

- [x] Answer-choice randomization belongs to the Question.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_compile.rs` `compile_choices` maps the Question document randomizeChoices flag to NonceRandomized or Fixed for single-choice and multiple-answer responses.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `AssessmentActivityRules` stores authored or shuffled Question order in assessment_question_order_rule and has no answer-choice field.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` states that answer-choice order is configured on each Question.
  - Evidence (test): `crates/adapters/ple/src/question_json/tests.rs` `choice_randomization_is_choice_owned_and_defaults_when_omitted` compiled a single-choice document with randomizeChoices true to NonceRandomized and rejected that field on fill-in.
  - Evidence (test): `tests/test_ple_question_json_editor_model.mjs` `setChoiceRandomization` set randomizeChoices true and kept choice ids choice_a and choice_b.
  - Evidence (test): `tests/test_ple_question_json_multiple_answer_editor.mjs` `setMultipleAnswerChoiceRandomization` set randomizeChoices true and kept correct ids kinase and enzyme.

- [x] PLE-native Questions control their own answer-choice randomization.
  - Evidence (source): `crates/question_model/src/presentation/builder_items.rs` `pending_items` permutes multiple-choice items only when native_choice_order is NonceRandomized.
  - Evidence (source): `crates/question_model/src/presentation/choice_order.rs` `nonce_randomized_choices` ranks choices by the issued nonce and each stable choice id.
  - Evidence (test): `crates/question_model/src/presentation/tests.rs` `nonce_randomized_native_choices_reproduce_the_issued_binding_order` issued carboxyl, amine, then hydroxyl for nonce 0x31 and kept that order after the authored vector was reversed.

### Draft Question specifications
- [x] Draft Questions are private working content.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `ple_private.draft_question` stores draft state in the private schema.

- [x] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns accepted Published Question Revisions and does not read Draft Questions.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` searches those same Published Question Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `list_authoring_drafts` returns the Instructor's Draft Questions outside the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns published Question Pools and does not read Draft Questions.
  - Owner: 07_questions.md / Draft Question specifications (first occurrence; identical requirement and status).

- [x] Draft Questions use current state rather than immutable Revisions.
  - Evidence (source): `schemas/base_schema/20_tables/question_authoring.sql` `draft_question` stores one current Draft by Edit Number and has no Revision column.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft` updates that current Draft in place and does not insert a Question Revision.

- [x] Saving a Draft Question replaces its previous working state.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft` replaces the current draft aggregate values.

- [x] **Instructors** may delete Draft Questions they no longer need.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `delete_draft_question` resolves only the current Instructor-owned Draft, locks and compares its Edit Number, then deletes that private aggregate without considering the separate Published Question lineage.
  - Evidence (source): `crates/learning-data-access/src/postgres/authoring.rs` `delete_authoring_draft` carries the SQL compare-and-swap through the authenticated Store.
  - Evidence (source): `crates/server/src/authoring.rs` `delete_draft` requires the parsed `If-Match` Edit Number and maps a concurrent change to 412; `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` supplies explicit Keep/Delete confirmation.
  - Evidence (runtime): `crates/server/src/authoring.rs` `delete_draft` passed accepted isolated PostgreSQL/MinIO actual-server and focused browser proof: cancel, confirm, and list reload; valid-current-ETag collaborator, unrelated Instructor, Student, Sysadmin, and anonymous 404 denials while owner source/Edit Number remained unchanged; 428 missing, 400 malformed, and 412 stale preconditions; preserved parsed Published Question lineage and Revision JSON after a published-origin Draft deletion; and 404 repeat DELETE/PUT. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`.

- N/A PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
  - Reason: Automated abandoned-Draft cleanup is an explicitly optional future capability; HG sets no clock or durations.

- [x] A Draft Question must pass Question Publication Validation before becoming a Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` is the ordinary Draft publication path. It refuses a missing Discipline, Subject, license, or reviewed authorship before any insert, writes acceptance, authorship, license, source binding, and owner, then inserts the publication event. A fork-source license check runs only when that Draft has a fork source.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `validate_question_publication` rolls the publication transaction back when acceptance, contiguous authorship, license, exact source, or owner is missing.
  - Evidence (source): `crates/server/src/authoring.rs` `publish_draft` refuses a Draft whose source has no Question License before it builds the publication command.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `A Draft Question must pass Question Publication Validation before becoming a Published Question` posted a complete ordinary publication to `/publish` and withheld the request when Discipline, Subject, or reviewed authorship was missing.

- [x] Question Publication Validation requires Discipline, Subject, and all other required Question
  Library metadata before publication.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a missing Discipline or Subject and copies Title and Description from the Draft.
  - Evidence (source): `schemas/base_schema/20_tables/question_authoring.sql` `draft_question_metadata` requires Title and Description before publication copies them.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` requires Title, Description, one Discipline, and a Subject that belongs to that Discipline.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `publication sends shared Question Library metadata and refuses a missing Discipline or Subject` rejected a blank Title and Description and withheld publication when Discipline or Subject was omitted.

### Question formats and type specifications
- [x] PLE flat-question JSON is the canonical machine format for simple static Questions.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the PLE JSON source form.

- [x] QTI is for import, export, and archival interchange rather than the internal source model.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `workspace_import` treats `qti` as an import format, not a source binding.

- [x] A QTI ZIP, retained QTI archive, and extracted QTI image are interchange roles, not Question Image
  Assets.
  - Evidence (source): `crates/adapters/qti/src/model.rs` `QtiPackageArchive` and `QtiPackageExtractedImage` are import-only; `question_image_asset_id()` is derived at Question bind.

- [x] MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision` CHECK lists all eight types.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/scenarios_student_types.ts` `captureTypes` supplied current authorized Student delivery of each eight released native types at laptop and phone widths (18 unanswered captures including WeBWorK); exact issued Question Revision membership and permitted-response privacy checks passed. This is private presentation coverage, not an eight-type interaction matrix. Receipt: `/private/tmp/ple-resumed-types-20260916.md`.

- [x] Question Type is immutable author-declared educational metadata on a Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` protects `question_type` on a revision.

- [x] PLE uses Question Type for search, filtering, labeling, and presentation.
  - Evidence (source): `src/api/question_library_repository.ts` `questionSearchRequest` sends the selected Question Type as the Library search filter; `src/pages/library_page.tsx` `questionTypeLabel` supplies learner-facing type labels and the Question Type selector presents the type facets.

- [x] Question Type comes from the author rather than inference from backend controls.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `draft_question_source_binding` records authoring input independent of backend.

- [x] Question importers are transient translators from external formats into PLE-managed Question representations.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `workspace_import` stages external-format imports before committed PLE state.

### Native PLE JSON Question specifications
- [x] The native PLE JSON Question format is private, unversioned, and unpublished.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` accepts the unversioned internal source shape.

- [x] Stored native JSON Questions may be upgraded together when the internal format changes.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` is the single internal reader for stored PLE JSON.

- [x] The native PLE JSON Question format is a strictly validated internal source shape without an external API.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the internal source document.

- [x] Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
  - Evidence (source): `crates/question_model/src/generation.rs` `QuestionReproduction` distinguishes static source reproduction from the inseparable seeded generator pair; `crates/adapters/ple/src/lib/question_json_source.rs` `presentation` issues native PLE JSON with `QuestionReproduction::Static`.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempts.sql` `validate_issued_question_reproduction` rejects a seed for a `ple` source and requires one for renderer-backed sources.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_attempts.sql` `validate_issued_question_reproduction` passed in `/private/tmp/ple-native-seed-proof.sh --isolated --native-seed-http` against PostgreSQL: shuffled-position-2 native seed/hash were null, real WeBWorK retained numeric seed/64-character hash privately, public start/read/save/resume/restored payloads omitted both fields, resume retained the same issued Questions and saved native response, and invalid native seed insertion failed. Artifact: `/private/tmp/ple-native-seed-artifacts.KfY7Op`.

- [x] Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonResponse` defines all eight native types.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/scenarios_student_types.ts` `captureTypes` supplied current authorized Student delivery of each released native type at laptop and phone widths with exact published Revision checks. The 16 native captures establish presentation only; response interaction, save/reload, and grading remain separately scoped per type. Receipt: `/private/tmp/ple-resumed-types-20260916.md`.

- [x] External URLs used by native JSON Questions are explicitly recorded and reviewable.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` records the author-declared `externalResources` inventory as source metadata only, without fetching or browser permission; `validate_external_resources` bounds and de-duplicates recorded URLs.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `validate_external_resource_url` accepts only bounded, printable, absolute HTTPS URLs without user information.
  - Decision: A one-time parser proof accepted all five then-supported resource kinds and legacy omission, while rejecting invalid and duplicate URLs; remote script resources have since been removed from the closed kind set.

- [x] Recorded external URLs include links, images, stylesheets, and other resources.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResourceKind` is the closed Link, Image, Stylesheet, and Other category set for every `externalResources` entry; external script syntax is not supported.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResource` binds each recorded URL to exactly one reviewed category under `deny_unknown_fields` parsing.

#### Native Question response presentation
- [x] Native MATCH Questions should present prompts with a shared choice bank on laptop and desktop
  screens. Display the full set of choices once alongside the prompts.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` has one bank, prompt slots, assignment/replacement/Clear controls, same-bank drag, and filtered partial/reset serialization.
  - Evidence (runtime): `src/components/question_response_controls/matching.tsx` `MatchingResponse`, supplied `/private/tmp/ple-matching-saved-1280.png` and `/private/tmp/ple-attempt-compact-1280.png` show all four bank choices once beside the prompt slots. Independent source review `/private/tmp/ple-demo-ui-source-review.md` and bounded acceptance `/private/tmp/ple-ui-bounded-acceptance.md` support this shared-bank laptop/desktop presentation only; keyboard changing/clearing and whole-Attempt grading are separate requirements.

- [x] MATCH Questions should support drag-and-drop and an equally capable keyboard-only method for
  assigning, changing, and clearing matches.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` has one bank, prompt slots, assignment/replacement/Clear controls, same-bank drag, and filtered partial/reset serialization.
  - Evidence (runtime): `src/components/question_response_controls/matching.tsx` `MatchingResponse`, supplied parent 2026-09-16 actual current-demo Avery R-4 laptop proof (session 87294, exit 0): normal Tab traversal without programmatic focus plus Space/Enter cleared the first two saved matches, selected bank choices, swapped both assignments, then cleared/reassigned the original choices. All four original choice strings were restored exactly, and Tab/Enter Save was accepted. The existing native mouse drag and accepted Save receipt is independently accepted in `/private/tmp/ple-ui-bounded-acceptance.md`; fresh keyboard evidence is recorded in `/private/tmp/ple-latest-hg-checklist-reconciliation.md`. This closes assigning/changing/clearing parity only, not adapted grading or full pointer/touch bank reachability.

- [x] Question response layouts may adapt to available screen space while preserving the same content,
  response meaning, and grading behavior. Narrow layouts may repeat choices when that improves use.
  - Evidence (source): `src/components/question_response_control_styles.ts` `matching-layout` places the shared choice bank beside the prompts when the row is wide and stacks that same bank when the row is narrow.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` grades the saved Student response.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `Question response layouts may adapt to available screen space while preserving the same content, response meaning, and grading behavior` kept both prompts and both choices once, stacked the bank at a narrow width, and saved the same matches as the wide width.

- [x] MATCH Questions should make each prompt's assigned choice easy to recognize and keep the choice
  bank reachable while Students assign, change, and clear matches using keyboard, pointer, or touch.
  - Evidence (source): `src/components/question_response_controls/matching.tsx` `MatchingResponse` shows the assigned choice wording in the prompt slot and keeps every choice in the bank.
  - Evidence (test): `tests/playwright/test_student_assessment_presentation.mjs` `MATCH Questions should make each prompt's assigned choice easy to recognize and keep the choice bank reachable while Students assign, change, and clear matches using keyboard, pointer, or touch` assigned, changed, and cleared a match with pointer, keyboard, and touch while the bank stayed visible and the slot showed the choice wording.

#### Native PLE JSON Questions and JavaScript
- [x] Native JSON Questions may contain author-supplied JavaScript, including chemistry content using RDKit.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document/author_script.rs` `PleQuestionJsonAuthorScript` records the author source and the closed rdkit library without running JavaScript.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_codec.ts` `decodeAuthorScript` accepts that source and the rdkit library name.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `grading_and_correctness_decisions_remain_server_owned_and_independent_of_author_supplied_javascript` stored RDKit.get_mol("CCO") with library rdkit, issued that source, graded blue at credit 1, and graded red at credit 0.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `author script metadata stays closed and preserved without becoming an execution path` preserved an author script and the rdkit library and refused an unknown library.

- [x] Author-supplied JavaScript may provide client-side rendering or interaction without access to a random seed.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` installs the author source as a client script and does not receive a question seed.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` loads that document for the issued Question position.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_may_provide_client_side_rendering_or_interaction_without_access_to_a_random_seed` built the client document from author source, kept the script bytes and author-content root, and left question-seed-9f3c out of the document and headers.

- [x] Author-supplied JavaScript runs in an isolated browser environment.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` serves the author document with a script sandbox, `default-src 'none'`, and no `allow-same-origin`.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` loads that document in an iframe whose sandbox is `allow-scripts` and whose referrer policy is `no-referrer`.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_runs_in_an_isolated_browser_environment` built the client document, required `sandbox allow-scripts` without `allow-same-origin`, and kept the author script bytes in that document.

- [x] Author-supplied JavaScript is treated as untrusted content.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` carries the author source as base64 into a script text node.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_codec.ts` `decodeAuthorScript` accepts only source and libraries.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_is_treated_as_untrusted_content` built a document whose source was a script breakout and kept that text out of HTML parsing.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `author script metadata stays closed and preserved without becoming an execution path` preserved an author script and refused an execute field.

- [x] Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` serves the author document with `sandbox allow-scripts` and `connect-src 'none'`, and sets no cookie or authorization header.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` loads that document in an iframe whose sandbox is `allow-scripts`, whose permissions policy is empty, and whose source is only the attempt and position URL.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_is_isolated_from_ple_application_state_credentials_and_privileged_browser_context` built the client document, required `sandbox allow-scripts` and `connect-src 'none'`, and left cookie, authorization, storage APIs, and the session credential out of the document.
  - Evidence (test): `tests/test_assessment_attempt_navigation.mjs` `Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context` rendered the shipped frame with sandbox `allow-scripts`, an empty permissions policy, no-referrer, and an attempt/position document URL that omitted the session credential.

- [x] Author-supplied JavaScript is limited to client-side rendering and interaction.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `document_csp` limits the no-library author document to `sandbox allow-scripts` with `connect-src 'none'`, `form-action 'none'`, `frame-src 'none'`, and `worker-src 'none'`.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` grants that document only `sandbox="allow-scripts"` and an empty permissions policy.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_is_limited_to_client_side_rendering_and_interaction` built the client document, required the script sandbox and the none-source directives, and left fetch, XHR, WebSocket, and application paths out of the document.
  - Evidence (test): `tests/test_assessment_attempt_navigation.mjs` `Author-supplied JavaScript is limited to client-side rendering and interaction` rendered the shipped frame with sandbox exactly `allow-scripts` and without form, popup, navigation, download, or modal tokens.

- [x] Author-supplied JavaScript operates independently of PLE application APIs and privileged state.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `author_content_document_response` builds the author document from the author source and parent origin, with no API client or session.
  - Evidence (source): `src/components/author_content_frame.tsx` `AuthorContentFrame` reads only the document URL from the application client.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `author_supplied_javascript_operates_independently_of_ple_application_apis_and_privileged_state` built the document from author source alone, kept `connect-src 'none'`, and left application paths, the session credential, and storage APIs out of the document.
  - Evidence (test): `tests/test_assessment_attempt_navigation.mjs` `Author-supplied JavaScript operates independently of PLE application APIs and privileged state` rendered the shipped frame from a client that exposes only the document URL and left the session credential out of the frame.

- [x] Native interactive Question Types such as HOTSPOT use PLE-owned interaction code.
  - Evidence (source): `src/components/question_response_controls/question_response_control.tsx` `QuestionResponseControl` dispatches a delivered `hotspot` format to `HotspotResponse`; `src/components/question_response_controls/hotspot.tsx` `HotspotResponse` owns the image overlay, labeled native region controls, response serialization, and Save handoff.
  - Evidence (runtime): `tests/playwright/screenshot_corpus/hotspot_workflow.ts` `exerciseHotspot` passed unchanged for Avery's pointer input and Jack's keyboard Space input: each selected the PLE-owned region, saved, reloaded the exact issued Question ID and Revision with the selection intact, submitted the whole Attempt, and received `Marked correct.` from server grading. Receipt: `/private/tmp/ple-hotspot-connected-interaction-20260916.md`.

- [x] HOTSPOT content uses supported still images and SVG.
  - Evidence (source): `crates/server/src/draft_question_images.rs` `upload` accepts image/svg+xml and stores the WebP from prepare_question_image.
  - Evidence (test): `crates/objects/src/image_validation/svg_question_image.rs` `hotspot_content_uses_supported_still_images_and_svg` rewrote a red SVG into a 12 by 8 WebP, left a local file and a remote image unpainted, refused SVG text so labels cannot disappear, and refused DOCTYPE, entity, gzip, and a pixel flood.

- [x] Grading and correctness decisions remain server-owned and independent of author-supplied JavaScript.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` grades the compiled private answer key and does not pass author content into grading.
  - Evidence (source): `crates/grading/src/ple_question_json.rs` `evaluate` scores the response from the server answer key.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `grading_and_correctness_decisions_remain_server_owned_and_independent_of_author_supplied_javascript` issued an author script that returns red, kept that source and the rdkit library on the presentation, graded blue at credit 1, and graded red at credit 0.

- [x] Supported author JavaScript libraries are explicitly recorded and served by PLE.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document/recorded_javascript.rs` and `src/features/ple_question_json_authoring/recorded_javascript_dependencies.ts` retain the closed `rdkit` library registry; external script resources and CDN allowlists are absent.
  - Evidence (test): `crates/adapters/ple/src/question_json/tests.rs` `supported_author_javascript_libraries_are_explicitly_recorded_and_reviewable` parses an author Question that declares rdkit.

- N/A Approved external dependencies may initially load from recorded CDN sources.
  - Reason: Author Questions do not support remote script resources; the recorded rdkit dependency is served from PLE.

- [x] Supported external dependencies should eventually become PLE-owned and served locally.
  - Evidence (source): `crates/server/src/author_content_dependency_assets.rs` `author_content_dependency_asset_router` serves the current RDKit JavaScript and WASM from compiled-in local bytes at fixed PLE routes.
  - Evidence (source): `crates/server/src/author_content_dependency_registry_generated.rs` `RDKIT_FILES` records those two local files and names no CDN URL.
  - Evidence (source): `crates/server/src/author_content_document_route.rs` `document_csp` points an rdkit author document at that local WASM route and no other network host.
  - Evidence (test): `crates/server/src/author_content_document_route.rs` `supported_external_dependencies_should_eventually_become_ple_owned_and_served_locally` built the rdkit document, required the local JavaScript and WASM routes, rejected CDN hosts, and received both files from the PLE router with no redirect.
  - Evidence (test): `crates/server/src/author_content_dependency_assets.rs` `current_rdkit_assets_are_byte_exact_and_have_only_their_public_contract` served those routes and matched the compiled-in JavaScript and WASM bytes.

### Question Backend specifications
#### Supported Question Backends
- [x] WeBWorK is a PLE-managed Question Backend.
  - Evidence (source): `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` is the current PLE-managed WeBWorK integration boundary.

- [x] The initial primary Question Backends are PLE-native JSON and WeBWorK.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_presentation.sql` `backend IN ('ple', 'webwork')` is the delivered presentation boundary.

- N/A iMathAS and H5P are desired secondary Question Backends governed by Deferred product behavior.
  - Reason: Human Guidance explicitly defers both Backends, so they are desired product behavior rather than current implementation requirements. Current production Backends are PLE and WeBWorK.

- [x] PLE-native Questions use the PLE Question Backend.
  - Evidence (source): `schemas/base_schema/15_table_check_functions.sql` `question_source_binding_fields_are_valid` accepts native pleQuestionJson only with the ple Question Backend.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionBackend` names Ple as that first-party Question Backend.

- [x] WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/client.rs` `render` keeps the renderer HTML document, and `grade` returns only the renderer score.
  - Evidence (test): `crates/adapters/webwork/src/http_renderer/tests.rs` `webwork_owns_pg_pgml_rendering_controls_evaluators_partial_credit_and_feedback` forwarded PGML source unchanged, kept the renderer form control and feedback HTML, and graded the renderer score 0.5 as partial credit. No Live Demo stack was started. No PostgreSQL proof was run.

- N/A H5P owns its runtime, interactions, state, and scoring.
  - Reason: H5P is desired but explicitly deferred and is not a current implementation requirement.

- N/A iMathAS owns its rendering and evaluation.
  - Reason: iMathAS is desired but explicitly deferred and is not a current implementation requirement.

#### Question Backend responsibilities
- [x] PLE owns and stores the Question representation used for each Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `question_revision_source_binding` stores backend representations.

- N/A Imported backend source may be transformed into the form PLE stores and manages.
  - Reason: Optional transformation does not require a current backend-import behavior.

- [x] PLE preserves the information needed to reproduce the Question through its backend.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `question_revision_source_binding` retains backend selectors and source checksum.

- [x] PLE-managed Question representations participate in Question revision history.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `question_revision_source_binding` keys source bindings to immutable revisions.

- [x] Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `issue_question_json` renders the native prompt, choice control, and backend state, and `grade_question_json` grades the captured response.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `native_question_backend_owns_rendering_interaction_response_grading_feedback_and_state` issued the favorite-color prompt and choice control, graded blue at credit 1 and red at credit 0, projected the native incorrect feedback, and kept PLE backend state. `crates/adapters/webwork/src/http_renderer/tests.rs` `webwork_owns_pg_pgml_rendering_controls_evaluators_partial_credit_and_feedback` kept the renderer document, form control, answer field, partial-credit score, and feedback HTML. iMathAS is not a current production backend. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
  - Evidence (source): `crates/server/src/assessment_delivery.rs` `student_with_sessions` admits only a Student session before finalization preparation.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` selects the published question id, revision number, and database-derived finalization kind.
  - Evidence (source): `crates/objects/src/question_source.rs` `resolve` reads source bytes only for that exact Question Revision tuple.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `commit_assessment_attempt_finalization` stores the returned credit fraction only when the prepared finalization kind is still current.
  - Evidence (test): `crates/server/src/assessment_delivery/direct_finalization.rs` `ple_owns_authorization_question_id_revisions_persistence_lifecycle_and_stored_outcomes` concealed an Instructor before preparation, then posted the shipped submission route for a Student. The handler graded revision 4 of the prepared Question ID, committed Deadline with normalized credit 1, and opened zero renderer connections. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
  - Mismatch: `crates/question_model/src/question_library.rs` `QuestionBackend` is only an enum discriminator. Issuance and finalization branch separately on backend in `crates/server/src/assignment_delivery.rs` `issue_new_presentations` and `crates/server/src/assignment_delivery/direct_finalization.rs` `evaluate_one`; no common adapter interface covers every backend.

- [x] Each Question Backend adapter retains its backend-specific interaction knowledge.
  - Evidence (source): `crates/adapters/webwork/src/lib/grade.rs` `pg_source` sends the Question's PG source, PG path, and backend-owned response payload to the renderer.
  - Evidence (source): `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` keeps that WeBWorK interaction behind the adapter facade. Deferred iMathAS and H5P behavior is outside this production adapter.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `opaque_lifecycle_issues_and_grades_one_backend_owned_payload` issued one document and graded the opaque payload only after the shipped adapter passed the PG source, PG path, and payload through. `opaque_grading_refuses_native_ple_responses_without_a_renderer_call` refused a native PLE response before any renderer call. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Question Backends may support more complex interactions without requiring PLE to implement those interactions.
  - Evidence (source): `crates/adapters/webwork/src/lib/issue.rs` `QuestionResponseFormat::BackendOwned` issues an empty PLE prompt and a backend-owned response, so the PG form stays with the renderer.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `opaque_lifecycle_issues_and_grades_one_backend_owned_payload` checked that empty prompt and backend-owned response, then graded the opaque answer payload through the shipped adapter. Deferred iMathAS and H5P behavior stays outside this production backend. No Live Demo stack was started. No PostgreSQL proof was run.

#### Question Backend grading and feedback
- [x] Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` projects recorded native PLE feedback from the exact retained response and source, while the WeBWorK branch does not reconstruct or persist transient renderer feedback.
  - Evidence (runtime): the C910 isolated actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`, stopping the renderer after issuance and then submitting and reading exact WeBWorK Revision history without a backend-feedback field.

- [x] PLE does not extract or reconstruct transient feedback from Question Backend source or output.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` invokes recorded teaching-content projection only for the native PLE source variant; the WeBWorK source remains opaque.
  - Evidence (runtime): the C910 actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`; the history read succeeded after the renderer stopped and exposed no choice, correct, or incorrect feedback reconstructed from the PGML source or rendered output.

- [x] PLE-managed Hints, Question Feedback, and Worked Solutions remain separate from backend-generated
  interaction feedback.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns Revision Question Feedback, Hints, and Worked Solutions before backend teaching projection.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_teaching_feedback` assigns backend choice, correct, and incorrect feedback without writing Question Feedback, Hints, or Worked Solutions.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `ple_managed_support_stays_separate_from_backend_interaction_feedback` kept the PLE Hint, Question Feedback, and Worked Solution after backend choice, correct, and incorrect notes were applied. Those backend notes stayed out of the three PLE-managed fields. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` returns the compiled evaluation for one complete response.
  - Evidence (source): `crates/question_model/src/student_work/grading.rs` `QuestionEvaluation` keeps that credit fraction as the evaluation fact.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `backend_returned_credit_is_stored_as_the_immutable_grading_outcome` graded the correct color response at credit 1 and the same incorrect color response at credit 0 twice.
  - Owner: 07_questions.md / Question Backend grading and feedback (first occurrence; identical requirement and status).

- [x] PLE stores the immutable credit fraction as the grading outcome.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `commit_assessment_attempt_finalization` records each evaluated normalized credit through `record_direct_automated_grading_result`.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `grading_result_is_immutable` rejects a later change to the stored credit.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `backend_returned_credit_is_stored_as_the_immutable_grading_outcome` stored the backend credit 0 for the incorrect color response, refused to rewrite it, and refused a second grading result.

- [x] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` returns the compiled evaluation immediately.
  - Evidence (source): `crates/question_model/src/student_work/grading.rs` `QuestionEvaluation` records only correctness and a normalized credit fraction.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/grade.rs` `QuestionGradingOutcome::Evaluated` is the only grading result for a complete renderer score.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/client.rs` `WeBWorK uses stateless grading` refuses a lifecycle state on a grading request.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `questions_are_strictly_and_deterministically_automated_grading_does_not_require_an_instructor` graded complete responses to credits 1, 1, 0, and 0.
  - Evidence (test): `crates/adapters/webwork/src/http_renderer/tests.rs` `grade_forwards_ordered_pairs_once_with_trusted_fields` returned evaluated credits 0, 0.5, and 1.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `opaque_lifecycle_issues_and_grades_one_backend_owned_payload` graded one complete payload as an evaluated outcome and kept lifecycle state empty.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `issuance_refuses_renderer_lifecycle_state_for_stateless_webwork` refused renderer-issued lifecycle state for the stateless WeBWorK integration.

- [x] Assessment scores are calculated from stored credit fractions and current Question point values.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `score_recorded_credit` multiplies the retained credit fraction by the current Question point value.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` applies that calculation when an Attempt is already submitted.
  - Evidence (test): `crates/learning-data-access/tests/grading_rescore_postgres.rs` `current_point_values_recalculate_stored_credit_without_another_backend_grade` stored backend credit 1 as 2 of 2 and backend credit 0 as 0 of 2, then scored those same credits as 5 of 5 and 0 of 5 after the point value changed from 2 to 5.

- [x] Changing Question point values recalculates scores without another Question Backend interaction.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `save_assessment` writes the current Question point value.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `AlreadySubmitted` returns the prepared score before Question Backend evaluation.
  - Evidence (test): `crates/learning-data-access/tests/grading_rescore_postgres.rs` `current_point_values_recalculate_stored_credit_without_another_backend_grade` changed both point values through an Instructor save, and the next prepare returned already_submitted with no Question Attempt, response, backend, or source.

#### WeBWorK source and algorithmic Questions
- [x] Preserve the distinction between WeBWorK PG and PGML source. A Question should be identified as PGML only when its source is fully PGML-compliant; otherwise identify it as PG.
  - Evidence (source): `crates/project-tools/src/pilot_content.rs` `validated_question_format` maps only explicit PG or PGML declarations with matching extensions.
  - Evidence (test): `crates/project-tools/src/pilot_content/tests.rs` `pilot_publication_preserves_explicit_source_formats` exercises the format/extension refusals.
  - Evidence (runtime): `crates/project-tools/src/pilot_content/publication.rs` `existing_publication` passed accepted Pilot binding proof preserving source SHA, size, path, and exact explicit format through immutable replay; format/path refusal remains static-only. Artifact: `/private/tmp/ple-pilot-format-binding-artifacts.CKzka1`.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` passed selected ordinary-Instructor CLI proof publishing one canonical PGML source with exact bytes, checksum, and provenance; replay made no additional publication. Artifact: `/private/tmp/ple-canonical-family-artifacts.TOOlBJ`.

- [x] BiologyProblems.org imports should preserve whether the canonical algorithmic source is PG or PGML rather than treating both formats generically as PG/PGML.
  - Evidence (source): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` validates each explicit source format/path pair.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication reading all 42 canonical entries as explicit PGML source paths and ordinary WeBWorK Question lineages. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

- [x] When parameterized WeBWorK PG or PGML source exists, prefer it to importing static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `validate_loaded_content` requires direct Fixed Question entries.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication using its 42 canonical parameterized PGML sources as direct Fixed entries rather than generated static variants. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

- [x] Preserve backend-native algorithmic variation rather than expanding one algorithmic Question into static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content/parameterized_publication.rs` `publish_source` publishes a parameterized source without static expansion.
  - Evidence (runtime): `docs/active_plans/active/human_guidance_implementation_compliance_plan.md` `C839` accepts canonical source hashes with repeatable/reseeded renderer variation and deterministic grading.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/parameterized_publication.rs` `publish_source` passed fresh publication of 42 ordinary WeBWorK lineages, and exact replay made no mutation. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

- [x] One algorithmic Question remains one Published Question regardless of how many variants its Question Backend can generate.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `existing_source_revisions` resolves one ordinary lineage per canonical source.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted canonical installation creating 42 Question lineages and 42 Revision 1 records from 42 sources, with no variant expansion. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/parameterized_publication.rs` `publish_selected_with_context` and `publish_source` passed selected ordinary-Instructor CLI proof publishing one available Revision-1 Question from one canonical source, with no Pool or Blueprint; replay made no additional publication. Artifact: `/private/tmp/ple-canonical-family-artifacts.TOOlBJ`.

- [x] Use a Question Pool with algorithmic Questions only when the **Instructor** wants selection among distinct Questions, not to represent variants of one algorithmic Question.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` resolves selected Published Question Revisions and requires the Instructor's interchangeability attestation; `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects distinct immutable members without backend-specific variant expansion.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted fresh PostgreSQL/MinIO actual-server and private bundled-main HTTP-proxy browser proof: 42 canonical Genetics Questions installed with zero implicit Pools, then the Instructor visibly selected distinct DNA structure and nucleotide components Revision-1 PGML Questions, attested interchangeability, created a reusable Pool, and imported a distinct Assessment-owned fork with `selection_count=1`. Real WeBWorK rendering, radio-response save/resume, exact fork Pool/Question Revision, issued ID, seed/hash preservation, whole-Attempt submit, and fresh new-Attempt selection/issued IDs passed; a new Attempt may select the same Question and need not have different seeds. Artifact: `/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. Release used a 3600-second time limit and Correct answer Never; answer disclosure, full Live Demo/authentication/TLS, and all-backend acceptance are outside this receipt. Browser error arrays were empty after route teardown completed.

- [x] BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` validates canonical source pins before publication.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication importing all 42 C839-accepted canonical PGML sources (41 BiologyProblems.org sources plus HLA), preserving source pins and producing ordinary available WeBWorK Question lineages. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

- [x] Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `validate_loaded_content` rejects non-Fixed entries in the canonical Blueprint.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication creating one Revision-1 Question lineage per canonical source, 42 direct Fixed entries, and zero Pools; exact replay was unchanged and a same-short-name conflict made no mutation. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

### Published Question specifications
- [x] A Published Question is an immutable-revision Question available for reuse through the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` publishes one Revision from the current Draft.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` rejects a later change to that Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that available Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` returns that same Revision.

- [x] Published Questions are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` requires an active Instructor Account and exposes available Question summaries.

#### Published Question identity specifications
- [x] Published Questions receive a public `XXXX-ZXXX` Crockford Base32 ID.
  - Evidence (source): `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` mints the exact public form, and `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` stores only that hyphenated form.

- [x] Question IDs have the canonical form `XXXX-ZXXX`.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` accepts only four Crockford characters, a hyphen, a checksum character, and three Crockford characters.
  - Evidence (source): `crates/question_model/src/question_library.rs` `from_random_identifier` builds that same `XXXX-ZXXX` form.

- [x] The hyphen is part of the canonical ID and makes Question IDs immediately recognizable.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QUESTION_ID_HYPHEN_INDEX` requires the hyphen at the fifth character of the canonical Question ID.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` rejects a Question ID that omits that hyphen.

- [x] Human-entered Question IDs may omit the hyphen.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` accepts an eight-character explicit human-entry value and inserts the canonical hyphen before validation.

- [x] Normalize accepted human input to the canonical hyphenated form before validation and lookup.
  - Evidence (source): `src/question_id.ts` `normalizeHumanEnteredQuestionId` normalizes explicit human entry, inserts the hyphen, and invokes `validateCanonicalQuestionIdSyntax`.

- [x] PLE always stores, transmits, displays, and copies the canonical hyphenated form.
  - Evidence (test): `crates/question_model/src/question_library.rs` `question_and_pool_ids_generate_and_transmit_only_the_canonical_hyphenated_form` mints `ABCD-XEFG` from seven Crockford characters and JSON-transmits that hyphenated value for both Question and Pool IDs, rejecting an unhyphenated value, a lowercase value, and a bad checksum.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` accepts storage only when the hyphenated checksum form matches.
  - Evidence (test): `tests/test_frontend_contract.mjs` `PLE always stores, transmits, displays, and copies the canonical hyphenated form.` renders `ABCD-XEFG` in the Question ID code and refuses an unhyphenated display value before any copy control appears.
  - Evidence (test): `tests/playwright/record_list_contracts.mjs` `checkSemanticContent` clicks Copy Question ID `7K3M-79QP` and records that exact hyphenated clipboard value. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Seven Crockford Base32 characters are cryptographically random and provide the identity.
  - Evidence (source): `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` draws seven characters from operating-system randomness before `QuestionId` appends the checksum.

- [x] IDs never encode creation order, Question Type, ownership, subject, or other metadata.
  - Evidence (source): `crates/server/src/question_publication.rs` `RandomQuestionIdIssuer` derives the seven identity characters only from operating-system random bytes.

#### Published Question metadata
- [x] Published Questions have metadata specific to the individual Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` keys individual metadata to `question_id` and requires nonempty `question_title` and `question_description` independently of Course placement.

- [x] Published Question metadata includes Title and Description.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` keys individual metadata to `question_id` and requires nonempty `question_title` and `question_description` independently of Course placement.

- [x] Published Question metadata may include authorship, attribution, license, and source information.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `validate_question_publication` requires exact source, contiguous revision authorship and license records, keeping them associated with the Published Question Revision.

- [x] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft_general_feedback` stores optional `hint`, `worked_solution`, and `general_feedback` on the Draft, and leaves Hint and Worked Solution unchanged when they are omitted.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` copies Draft `hint`, `worked_solution`, and `general_feedback` onto the Published Question Revision.
  - Evidence (source): `crates/server/src/authoring.rs` `save_general_feedback` writes Hint and Worked Solution only when the request contains both keys.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `published questions include optional PLE-managed hint question feedback and worked solution` loaded the three texts and saved Hint, Question Feedback, and Worked Solution together. No Live Demo stack was started. No PostgreSQL proof was run.
  - Evidence (test): `crates/server/src/authoring.rs` `published_questions_include_optional_hint_feedback_and_worked_solution` kept omitted Hint and Worked Solution unchanged, replaced both when present, and rejected a body that sent only one of them.
  - Owner: 07_questions.md / Published Question metadata (first occurrence; identical requirement and status).

- [x] Published Questions also use the shared Question Library metadata required for publication.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a Published Question without a Discipline or a Subject and stores Discipline, Subject, Topic, Subtopic, and Tags.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` requires one Discipline and one Subject and stores optional Topic, Subtopic, and Tags.
  - Evidence (source): `crates/server/src/authoring.rs` `publish_draft` copies the Draft Question Tags and the requested Discipline, Subject, Topic, and Subtopic into publication.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `publication sends shared Question Library metadata and refuses a missing Discipline or Subject` sent that classification and withheld a request that omitted Discipline or Subject.

#### Published Question revisions, edits, and forks
- [x] **Published Questions** maintain immutable revision history.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.

- [x] Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_entry_question` stores the Assessment's exact Published Question Revision.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `issued_question` stores the Student Work Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends the next Revision and does not write Assessment entries or issued Questions.

- [x] Publishing a new Question Revision does not silently change existing Assessments or Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends the next Revision and does not write Assessment entries or issued Questions.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_entry_question` keeps the Assessment on the Revision recorded before publication.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `issued_question` keeps Student Work on the Revision recorded before publication.

- [x] The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends an owner-authored revision.

- [x] Changing Question source, answer content, grading rules, Hints, Question Feedback, Worked Solutions,
  or Question Image Assets creates a new Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` appends the next Revision unless `source_object_checksum`, Question Feedback `general_feedback`, Hint `hint`, Worked Solution `worked_solution`, and HOTSPOT image flag `v_question_image_unchanged` all match the parent. A different draft-owned Question Image Asset does not keep the current Revision.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `correct_choice` stores answer content in the Question source. `PleQuestionJsonNumericResponseTolerance` stores a grading rule. `PleQuestionJsonHotspotSurface` stores the Question Image Asset id and checksum in that same source.
  - Evidence (test): `crates/adapters/ple/src/question_json/tests.rs` `changing_source_answer_grading_or_question_image_changes_the_revision_source_checksum` changed the prompt, the correct choice, the numeric tolerance, and the HOTSPOT image tuple. Each change produced a different canonical source checksum. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Changes to the Question title, description, Discipline, Subject, Topic, Subtopic, Tags, or other
  search metadata update the Published Question metadata while preserving the current Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/published_question_metadata_operations.sql` `bulk_replace_published_question_metadata` stores each Question Title and Description and the shared Discipline, Subject, Topic, Subtopic, and Tags on published_question_metadata without inserting a Question Revision.
  - Evidence (source): `crates/learning-data-access/src/postgres/question_bulk_metadata.rs` `bulk_replace_published_question_metadata` sends that per-Question text with the shared metadata patch.
  - Evidence (source): `src/api/http_client/question_bulk_metadata.ts` `updateQuestionBulkMetadata` is the browser command for that metadata update.
  - Evidence (test): `tests/test_library_classification_search.mjs` `search metadata updates Title, Description, and classification without a Question Revision` sent two Question titles and descriptions with one shared classification and Tags and refused a blank Title before the request.

- [x] Search metadata belongs to the Published Question as a whole rather than to one Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `published_question_metadata` keys metadata to `question_id` only.

- [x] Any **Instructor** may fork a Published Question to create a separate Question with a new Question ID.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionForkControl` invokes the exact-Revision server command, which mints the separate Question ID and opens only the returned private Draft.
  - Evidence (runtime): `src/pages/question_detail_page.tsx` `QuestionForkControl` passed accepted C879 connected PostgreSQL/server/browser proof with two Instructors, exact source attribution, private cross-account denial, retry/concurrency, a distinct server-issued identity, and prevalidation-publication denial; exact canonical-ID proof remains required.

- [x] A fork starts as a private **Draft Question** with its own authorship and lineage.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_state.sql` `draft_question_fork_source` records a private draft fork source.
  - Evidence (runtime): `src/pages/question_detail_page.tsx` `QuestionForkControl` passed accepted C879 connected browser proof that opened only the returned private Draft for the invoking Instructor and denied the other Instructor.

- [x] A fork must pass Question Publication Validation before joining the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `validate_question_publication` guards publication.

- [x] Published forks retain source attribution.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `question_fork_source` records published fork provenance.

- [x] Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
  - Evidence (source): `schemas/base_schema/20_tables/corrections.sql` `forced_question_correction` and its immutable audit targets record correction actions.

- [x] Question authorship, contributor credit, history, attribution, and compatible CC licensing are preserved across Revisions and forks.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `question_revision_authorship` and `question_revision_license` preserve revision stewardship.
  - Evidence (runtime): `schemas/base_schema/50_functions/question_publication_operations.sql` `ple_private.publish_new_question_lineage` passed the accepted C879 3-by-3 PostgreSQL publication proof: each exact source Revision license was preserved across three supported compatible CC licenses and every mismatched requested license was rejected.

- [x] Watching a Published Question drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_revision_event` records a Published Question Revision, and `snapshot_library_watch_event_recipients` delivers that event to the Question's current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_fork_event` records a Published Question fork, and `enqueue_library_watch_thread_event` records an improvement thread for that watched Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for that watched Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `eventLabel` names New Revision, New public fork, Improvement thread activity, and Impact notice activity in the Watch inbox.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Published Question Revision, fork, improvement thread, and impact notice through the shipped Watch client. `crates/server/src/library_watch_notification.rs` `every_watch_activity_serializes_to_the_stable_wire_shape` serialized those four Question activities. No Live Demo stack was started. No PostgreSQL proof was run.

#### Published Question behavior specifications
- [x] Published Questions may include optional PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `save_authoring_draft_general_feedback` stores optional `hint`, `worked_solution`, and `general_feedback` on the Draft, and leaves Hint and Worked Solution unchanged when they are omitted.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_question_revision` copies Draft `hint`, `worked_solution`, and `general_feedback` onto the Published Question Revision.
  - Evidence (source): `crates/server/src/authoring.rs` `save_general_feedback` writes Hint and Worked Solution only when the request contains both keys.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `published questions include optional PLE-managed hint question feedback and worked solution` loaded the three texts and saved Hint, Question Feedback, and Worked Solution together. No Live Demo stack was started. No PostgreSQL proof was run.
  - Evidence (test): `crates/server/src/authoring.rs` `published_questions_include_optional_hint_feedback_and_worked_solution` kept omitted Hint and Worked Solution unchanged, replaced both when present, and rejected a body that sent only one of them.
  - Owner: 07_questions.md / Published Question metadata (first occurrence; identical requirement and status).

- [x] PLE-managed Hints, Question Feedback, and Worked Solutions are separate from Question Backend-generated content.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `hint` stores the PLE-managed Hint on the Question Revision. `worked_solution` stores the PLE-managed Worked Solution. `general_feedback` stores Question Feedback. Those columns are not the Question source binding.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `disclosed_revision_support` projects the Revision texts and does not take Question Backend-generated content.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_teaching_feedback` assigns a backend Question Answer and Question Answer Explanation without writing Question Feedback, Hints, or Worked Solutions.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `ple_managed_support_stays_separate_from_backend_generated_content` kept the PLE Hint, Question Feedback, and Worked Solution while the backend answer and explanation were disclosed in their own fields. The backend content stayed out of the three PLE-managed fields. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] WeBWorK Questions may use PLE-managed Hints, Question Feedback, and Worked Solutions even when similar material also exists in the WeBWorK source.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `hint` stores the optional PLE-managed Hint on the Question Revision, separate from the WeBWorK source. `worked_solution` stores the optional PLE-managed Worked Solution the same way. `general_feedback` stores Question Feedback.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` returns `hint` and `worked_solution` from the issued Question Revision for every backend.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `disclosed_revision_support` projects those Revision texts for every backend, including WeBWorK, and does not read the WeBWorK source. A native Question Hint is used only when the Revision Hint is absent.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `webwork_questions_keep_ple_managed_hints_and_worked_solutions` disclosed the PLE Hint and Worked Solution while a WeBWorK BEGIN_HINT and BEGIN_SOLUTION string stayed out of those blocks. Withheld timings hid both. A native Hint appeared only when the Revision Hint was absent. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Question Feedback is shown when its disclosure rules allow it.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `released_general_feedback` shows the Revision's Question Feedback when that text is present and omits it when absent.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns that feedback without using Assessment correct-answer disclosure.
  - Evidence (source): `src/pages/assessment_attempt_summary_page.tsx` `ReleasedBlocks` shows the General feedback block when the history carries it.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `question_feedback_is_shown_when_its_disclosure_rules_allow_it` kept "Keep the units." visible while a withheld answer decision hid the correct answer and still showed provided backend choice feedback. Absent Question Feedback stayed absent. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Hints and Worked Solutions use their own disclosure settings.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_disclosed_support` shows Hint blocks only when the Hints timing allows them and Worked Solution blocks only when the Worked Solutions timing allows them.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_disclosed_support` applies that projection from `project_released_content` on Attempt history. `disclosed_revision_support` uses the Revision Hint when it is stored and a native Question Hint only when the Revision Hint is absent. A Worked Solution stays absent until the Revision stores one.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `feedback_hints` stores the Hints timing separately from Question Answer disclosure. `feedback_worked_solutions` stores the Worked Solutions timing. Both default to Never.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `disclosureSummary` names the Hints timing and the Worked Solutions timing in the Student-facing sentence.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `hints_and_worked_solutions_use_their_own_disclosure_settings` released Hints during the Attempt while Worked Solutions stayed hidden, then released Worked Solutions after submit while Hints stayed hidden. The Quiz cohort gate kept the released Worked Solution and hid the Question Answer. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Student workflows remain complete when a Question has none of this optional support content.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AssessmentAttemptPage` presents the open Attempt without requiring Hints, Question Feedback, or Worked Solutions.
  - Evidence (source): `src/pages/assessment_attempt_summary_page.tsx` `ReleasedBlocks` shows a support block only when that block is present.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `general_feedback` may be absent; a NULL value means that optional fact is not stored.

### Question Pool specifications
- [x] A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` persists an ordered nonempty set of exact Published Question Revision members, and `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects from that Pool for Student delivery.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that selected an exact Pool member for Student Attempt 1, preserved it on resume, and selected again for Attempt 2. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.

- [x] Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` requires the creating Instructor's true `interchangeability_attested` value; it does not substitute an automatic pedagogical evaluator.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that required the Instructor's attestation before creating the ordered reusable Pool and before its later Assessment-owned reorder. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.
  - Evidence (runtime): `crates/server/src/question_pool_creation.rs` `create_question_pool` passed accepted actual-server proof that false or missing attestation returned 422 and left no Pool behind. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.

- [x] Question Pools may contain Questions from any Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores Published Questions from each production Question Backend in one Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_lineages.sql` `question_backend_is_supported_for_production` accepts the ple and webwork backends.

- [x] Question Pools are created from a Published Question and enter the Question Library immediately.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` requires each member to be an available Published Question and stores the Pool in that call.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that new Pool to the Instructor.

- [x] A Question Pool is an independently reusable Question Library object.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `current_pool` reads a Pool independently of any Assessment.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `current_pool` passed accepted actual-main Instructor proof: Pool `SBQR-N5RE` was created from the Question Library and its ordered member pins were read through `/api/question-pools/SBQR-N5RE`; separate actual-server proof then imported another reusable Pool into an Assessment.

- [x] Question Pools are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` and `read_current_published_question_pool` authorize active Instructors and project only public Pool/Revision/member facts.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `list_pools` passed accepted actual-server proof that a second vetted Instructor listed and read root Pool `1N6T-MZRD` and child Pool `J1BX-8V8F` with exact public member pins and no Course facts. A nonmember Assessment-fork PUT returned 404 without mutation; Student and anonymous Pool list/read calls returned no-store 404. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.

- [x] A Question Pool has its own public `XXXX-ZXXX` Crockford Base32 ID.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_id` is the Pool's own public ID column.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` requires that ID to keep the `XXXX-ZXXX` checksum form.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `reserve_public_id` records the ID for `question_pool` and rejects a second reservation of the same ID.

- [x] A Question Pool is a current ordered list of exact Published Question Revisions plus its
  metadata. Saving the list re-attests interchangeability and advances the Pool's Edit Number;
  no Revision is created. Removing ten Questions and saving once is one Edit.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_member` stores the current ordered Published Question Revision pins, and `question_pool` stores the Pool metadata with an Edit Number and no Revision column.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_members` re-attests interchangeability and advances `question_pool_edit_number` by one for one member-list save.

- [x] Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork` atomically creates a fresh child Pool Revision and Assessment Entry from an exact reusable source Revision without accepting raw member pins.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `import_fork` passed accepted actual-server proof that imported source Pool `P8H3-QYX9` into a direct Assessment and returned distinct fork `VFH9-CQKS`, Revision 1, at Assessment Edit 2.

- [x] The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `assessment_question_pool_fork` owns each child Pool through exactly one Assessment Entry, and `schemas/base_schema/50_functions/question_pools.sql` retains exact source-Revision provenance.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `append_fork_revision` passed accepted actual-server proof that appended the fork's Revision 2 with the two exact member pins reversed, then reread the reusable source unchanged at Revision 1 with its original order. Artifact: `/private/tmp/ple-course-empty-artifacts.BbKFFd`.

- [x] Forking a Question Pool preserves its list of Published Questions by their public `XXXX-ZXXX` IDs.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies each source member `published_question_id` onto the new Pool.
  - Evidence (source): `schemas/base_schema/10_types.sql` `is_canonical_question_family_id` requires that copied public ID to keep the `XXXX-ZXXX` checksum form.

- [x] Question Pools work the same way regardless of the Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `prepare_current_assessment_attempt_start` returns every member of one Pool with its source backend and does not split that Pool by backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` stores one Pool selection and both issued Questions, and keeps the source-backend reproduction seed.

- [x] **Instructors** choose the contents of a Question Pool and how many Questions are selected.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` submits the Instructor's ordered current Published Question Revisions with interchangeability attestation; `src/pages/assessment_workspace/assessment_pool_entry_editor.tsx` `AssessmentPoolEntryEditor` exposes the Assessment-owned fork's exact members and bounded selection count.
  - Evidence (runtime): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `updatePoolSelectionCount` passed accepted actual-main proof that created the reusable Pool from ordered Published Questions, then imported it, changed its selection count from 2 to 1, attested and reordered its exact members, and reloaded its Revision 2 while the source remained unchanged. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.

- [x] PLE selects from the Question Pool; the selected Question Backend controls the Question interaction.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` performs server-owned selection.

- [x] Question Pool selection and backend-native randomization are separate forms of variation.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `QuestionPoolSelectionEntropy` is separate from Question backend state.

- [x] Returning to an Attempt preserves the Question Pool selections already made.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `assessment_attempt_start_gate` returns an unfinished resumable Attempt before new issuance, while `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` returns its retained committed presentations rather than selecting again.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` passed accepted actual-server proof that returned Attempt 1 with `resumed: true`, the same selected pin, and the same presentation nonce after its first start. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.

- [x] Starting a new Attempt makes fresh selections from its Question Pools.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` stores a new Question Pool selection on a new Attempt and returns the open Attempt without writing another selection.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `prepare_current_assessment_attempt_start` returns that Pool's current members for the new selection.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `prepare_current_assessment_attempt_start_decision` returns no resumable Attempt once the open Attempt has passed its stored expiration.

- [x] Student Work pins the Published Question ID, its Revision Number, the Question Pool ID, and the
  Pool's Edit Number for every Question served from a Pool; the pinned Published Question
  Revision is all that later interpretation and grading need.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` stores the Pool ID, its Edit Number, and each selected Published Question ID and Revision Number on the Attempt.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_access.sql` `read_student_assessment_attempt_pool_selection` returns those stored pins.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` returns the issued Published Question ID, Revision Number, and that Revision's source binding.

- [x] Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` retains `question_id` and `revision_number`.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the exact issued linkage.

- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `question_pool_member` stores each exact Question Revision Tuple.

- [x] Question Pools contain only **Published Questions**; Question Pools cannot be members of Question Pools.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_member` stores each member as a Published Question ID and Revision Number.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `validate_question_pool_member_insert` requires that member to be a current production Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/public_ids.sql` `reserve_public_id` keeps one public ID from being both a Question Pool and a Published Question.

- [x] Watching a Question Pool drives in-app notifications for membership edits, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_members_changed_event` records a Pool membership edit for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_fork_event` records a Pool fork for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_thread_event` records an improvement thread for a watched Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for a watched Pool.
  - Evidence (source): `crates/learning-data-access/src/postgres/library_watch_notification.rs` `MembersChanged` reads that membership edit into the private Watch inbox and `require_pool_membership_target` refuses it for a Published Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `membersChanged` labels the inbox row Membership edit.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Question Pool membersChanged notification with Edit Number 5 through the shipped Watch client, and rejected a membership edit aimed at a Published Question. watch_activity_accepts_each_complete_variant accepted members_changed for a Pool and rejected it for a Question. every_watch_activity_serializes_to_the_stable_wire_shape serialized membersChanged. No Live Demo stack was started. No PostgreSQL proof was run.

#### Question Pool metadata
- [x] Question Pools have metadata specific to the individual Question Pool.
  - Evidence (runtime): `schemas/base_schema/50_functions/question_pools.sql` `question_pool` owns independent metadata. Accepted SQL/rollback/concurrency and final SQL, Rust/API, and browser reviews combine with root-supplied rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md`: independent metadata survives list/current reads and real Library UI creation/retry. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] Question Pool metadata includes Title and Description.
  - Evidence (runtime): Required independent Title/Description in `schemas/base_schema/50_functions/question_pools.sql` have accepted SQL and source review. Rebuilt `8147` proof at `/private/tmp/ple-pool-metadata-connected-report.md` rejects missing fields, retains exact list/current text, and preserves both fields after denied mixed-member UI creation. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] The first Published Question establishes the Question Pool's Discipline and Subject.
  - Evidence (runtime): Accepted actual-role SQL creation proof and final source reviews establish first-member classification. Rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md` retains exact ordered pins and first-member Discipline/Subject, rejects mixed Subject with `422` and unchanged public list, then creates after ordinary picker reselection. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] Every additional Published Question added to the Pool has the same Discipline and Subject as the Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_members` refuses a new member whose Discipline or Subject differs from the Pool and stores a new member that matches.

- [x] Published Questions retain their own Topic, Subtopic, Tags, and other Library Object metadata
  when included in a Question Pool.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` keeps Topic, Subtopic, Tags, Title, and Description on the Published Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_members` updates the Pool member list and does not write that Question metadata.

- [x] Question Pools may have their own authorship, attribution, license, and source information where
  appropriate.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_authorship` stores optional Pool-owned authors.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_license` stores an optional Pool-owned SPDX license.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_source` stores optional Pool-owned attribution and source information.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `save_question_pool_provenance` stores those facts, and a later empty save clears them without writing member Question authorship, license, citation, the Pool edit number, or source_question_pool_id.

- [x] Question Pool metadata describes the Pool rather than duplicating metadata from its member
  Published Questions.
  - Evidence (runtime): Accepted SQL/source proof establishes independent Title/Description, empty creation Tags, optional narrower hierarchy, classification retention after Question reclassification, and historical fork preservation. Rebuilt `8147` HTTP/browser proof at `/private/tmp/ple-pool-metadata-connected-report.md` confirms separately authored Pool text through creation, retry, list, and current reads. No historical Pool HTTP route is claimed. Source owner: `schemas/base_schema/50_functions/question_pools.sql` `question_pool`.

- [x] Question Pools may include optional PLE-managed **Hints**, **Question Feedback**, and
  **Worked Solutions**.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `hint` stores optional Pool Hint, Question Feedback, and Worked Solution text. NULL means absent. No PostgreSQL proof was run. No Live Demo stack was started.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_support.sql` `save_question_pool_ple_managed_support` replaces those three Pool texts and leaves the member-list Edit Number unchanged.
  - Evidence (source): `src/components/question_pool_support_editor.tsx` `QuestionPoolSupportEditor` edits the three Pool texts. Headless Chromium filled Hint, Question Feedback, and Worked Solution, saved them, and showed the fields at 1280 and 390 pixels. No Live Demo stack was started.
  - Evidence (test): `crates/server/src/question_pool_support.rs` `pools_keep_optional_ple_managed_support_without_copying_member_questions` saved the three texts, cleared blank text, and kept Edit Number 4.

- [x] Question Pools also use the shared Question Library metadata required for publication.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores one Discipline, one Subject, optional Topic and Subtopic, and Tags on each Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` copies the first member Discipline and Subject onto the Question Pool and stores its Tags.
  - Evidence (source): `src/api/decoders/question_pool_library.ts` `decodeQuestionPoolMetadata` requires that Discipline and Subject and accepts optional Topic, Subtopic, and Tags.
  - Evidence (test): `tests/test_question_pool_metadata.mjs` `Question Pools use the shared Question Library metadata required for publication` decoded that shared metadata and refused a Pool that omitted Discipline or Subject.

### Question Library specifications
- [x] Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
  - Evidence (source): `src/pages/library_route_page.tsx` `LibraryRoutePage` is the production Instructor Library surface.

- [x] The Question Library is one global collection of Published Questions and Question Pools.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` discovers Published Questions and Published Question Pools on one Question Library page.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` returns Published Questions for an Instructor or Sysadmin and left-joins Course membership only to mark course use.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns Question Pools from the global Question Pool table for an Instructor or Sysadmin without a Course filter.
  - Evidence (test): `tests/test_library_classification_search.mjs` `The Question Library is one global collection of Published Questions and Question Pools` requested `/api/questions/search` and `/api/question-pools` with no Course id.

- [x] Draft Questions are not part of the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns accepted Published Question Revisions and does not read Draft Questions.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` searches those same Published Question Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/question_authoring_operations.sql` `list_authoring_drafts` returns the Instructor's Draft Questions outside the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns published Question Pools and does not read Draft Questions.
  - Owner: 07_questions.md / Draft Question specifications (first occurrence; identical requirement and status).

- [x] **Published Questions** and Question Pools are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `create_instructor_account` creates an Instructor Account only after completed Instructor identity vetting.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_instructor` recognizes that active Instructor role.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` returns available Published Questions to an active Instructor or Sysadmin and does not filter by author or Course.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns Question Pools to an active Instructor or Sysadmin and does not filter by author or Course.
  - Evidence (source): `crates/server/src/question_library.rs` `library_reader_session_hash` admits an Instructor session to Question search and conceals a Student session.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `library_reader` admits an Instructor session to Question Pool discovery and conceals a Student session.
  - Evidence (test): `crates/server/src/question_library/tests.rs` `question_search_resolves_native_source_only_for_the_returned_page` searches as an Instructor and requires the author and Course filters to stay off.

- [x] **Students** access Question content through their Coursework rather than through the Question Library.
  - Evidence (source): `src/route_contract.ts` `ROUTE_CONTRACT` reserves both Question Library routes for Instructors, and `src/route_access_boundary.tsx` `withRouteAccessBoundary` fail-closes every protected route before its page component mounts.
  - Evidence (runtime): `src/route_access_boundary.tsx` `withRouteAccessBoundary` passed accepted actual-main Student proof that denied three Library routes without any Question Library API request, while the Student Ribbon allowed Coursework navigation to a Released Assessment. Earlier accepted native Student Attempt proof delivered Question content through that Assessment. Artifacts: `/private/tmp/ple-course-empty-artifacts.9s89JA` and `/private/tmp/ple-course-empty-artifacts.ZquNiI`.

- [x] Question Library content remains discoverable when used by a private **Course Instance**.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionSearchResult` is global and separately reports course use.

- [x] With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` is the Question Library collection surface and does not mount an archive control. `src/pages/question_detail_page.tsx` `QuestionArchiveControl` offers one Archive Published Question action for one available Published Question, and the danger zone appears only after that action is opened.
  - Evidence (test): `tests/test_question_library_archive_workflow.mjs` `renderLibrarySearch` rendered the shipped LibraryPage search surface with Search Question Library and Create Question Pool and without Archive. `renderQuestionArchive` rendered shipped QuestionArchiveControl as one Archive Published Question button, hid Danger Zone, and called getQuestionLineage without archiveQuestion. The render did not load 13,000 Questions. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Question Library workflows should support bulk operations because an **Instructor** may manage thousands of Questions.
  - Evidence (source): `crates/server/src/question_bulk_metadata.rs` `question_bulk_metadata_router` registers POST /api/questions/bulk-metadata. `bulk_replace_metadata` accepts one Instructor command for the selected Published Questions, and `decode_input` refuses a selection longer than 1000 before storage. `src/pages/library_page.tsx` `QuestionBulkMetadataEditor` mounts that command on the Question Library.
  - Evidence (test): `crates/server/src/question_bulk_metadata.rs` `question_library_workflows_support_bulk_operations_for_many_questions` posted two Published Questions through the shipped route, refused 1001 items before storage while the cap stayed 1000, and concealed a Student session without a store call. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] **Instructors** should be able to select many Library objects and update shared metadata such as
  Discipline, Subject, Topic, Subtopic, Tags, or other search fields together.
  - Evidence (source): `crates/server/src/question_bulk_metadata.rs` `question_bulk_metadata_router` updates Tags, Discipline, Subject, Topic, and Subtopic for many Published Questions. `crates/server/src/question_pool_bulk_metadata.rs` `question_pool_search_metadata_router` updates Topic, Subtopic, and Tags for many Question Pools, and `decode_patch` refuses Discipline and Subject. `schemas/base_schema/50_functions/question_pools.sql` `validate_question_pool_lineage_update` keeps the Discipline and Subject established by the first member and allows those Pool search fields only while Edit Number stays unchanged. `src/pages/library_pool_discovery.tsx` `updatePoolSelection` selects many Pools, and `src/components/question_pool_search_metadata_editor.tsx` `QuestionPoolSearchMetadataEditor` submits the Pool command.
  - Evidence (test): `crates/server/src/question_pool_bulk_metadata.rs` `instructors_select_many_library_objects_and_update_shared_search_metadata` posted two Question Pools, kept Edit Number 4, refused disciplineUuid before storage, concealed a Student session without a store call, and refused 1001 Pools while the cap stayed 1000. `crates/server/src/question_bulk_metadata.rs` `question_library_workflows_support_bulk_operations_for_many_questions` posted two Published Questions through the Question command. No Live Demo stack was started. No PostgreSQL proof was run.

- [ ] Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Mismatch: search, filters, and an accepted mock-transport browser metadata workflow exist, but connected HTTP and 13k practical-cleanup evidence remains pending.

#### Question Library metadata
- [x] **Library Objects** use shared metadata for organization, search, filtering, and discovery.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` filters and searches Published Questions by Discipline, Subject, Topic, Subtopic, and Tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` filters and searches Question Pools by that same shared metadata.
  - Evidence (source): `src/api/question_library_repository.ts` `questionSearchRequest` sends that metadata for Published Question discovery.
  - Evidence (source): `src/api/question_pool_library_filter.ts` `questionPoolLibraryFilter` sends that metadata for Question Pool discovery.
  - Evidence (test): `tests/test_library_classification_search.mjs` `Library Objects use shared metadata for organization, search, filtering, and discovery` sent the shared metadata through both discovery requests and refused a Subject without its Discipline.

- [x] Required Question Library metadata must be complete before a Library Object enters the Question Library.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a new Published Question when Discipline or Subject is absent and stores its Title, Description, language, Tags, and classification.
  - Evidence (source): `schemas/base_schema/20_tables/question_authoring.sql` `draft_question_metadata` requires that Title, Description, and language before publication copies them.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `question_library_entries` returns a Published Question only together with that Question Library metadata.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` requires a Title, Description, Discipline, and Subject.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` requires that Title and Description and copies the first member Discipline and Subject.
  - Evidence (test): `tests/test_ple_question_json_authoring.mjs` `publication sends shared Question Library metadata and refuses a missing Discipline or Subject` withheld publication when Discipline or Subject was omitted.

- [x] Library metadata should describe the Library Object rather than its location in a Course, Assessment, or textbook.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` stores the Published Question title, description, language, tags, and classification, and has no Course, Assessment, or textbook column.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores the Question Pool title, description, tags, and classification, and has no Course, Assessment, or textbook column.
  - Evidence (source): `src/api/decoders/question_bulk_metadata.ts` `decodeQuestionBulkMetadataCurrent` accepts only the Published Question identity, edit number, tags, and classification.
  - Evidence (source): `src/api/decoders/question_pool_library.ts` `decodeQuestionPoolMetadata` accepts only the Question Pool title, description, classification, and tags.
  - Evidence (test): `tests/test_library_classification_search.mjs` `Library metadata describes the Library Object rather than a Course Assessment or textbook` decoded that object metadata and refused a Course, Assessment, and textbook location.

- [x] Library Objects use the shared **Discipline**, **Subject**, **Topic**, **Subtopic**, and **Tag**
  vocabulary.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` stores content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, and tags on each Published Question.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, and tags on each Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Question Pool vocabulary and filters by Topic, Subtopic, and Tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` filters Published Questions by that same vocabulary.
  - Evidence (test): `tests/test_library_classification_search.mjs` `Library Objects share the Discipline Subject Topic Subtopic and Tag vocabulary` sent one Discipline, Subject, Topic, Subtopic, and the tag review through questionSearchRequest.

- [x] Every Library Object has exactly one **Discipline** and one **Subject**.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` stores one Discipline and one Subject for each Published Question.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool` stores one Discipline and one Subject for each Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a Question without a Discipline or a Subject and stores one of each.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that Question's Discipline and Subject.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` copies that one Discipline and Subject onto the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool's Discipline and Subject.
  - Owner: Content classification (first occurrence).

- [x] **Topic** and **Subtopic** are optional for Library Objects.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` publishes a Question with Topic and Subtopic absent, and publishes another with both present.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns those Question classifications.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores a Question Pool with Topic and Subtopic absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool classification.
  - Owner: Content classification (first occurrence).

- [x] Library Objects may have any number of **Tags**, including none.
  - Evidence (source): `schemas/base_schema/15_table_check_functions.sql` `question_metadata_tags_are_valid` accepts an empty tag list and any number of distinct trimmed tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` stores a Question tag list, including none, and stores several tags.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns those Question tag lists.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores a Question Pool with no tags and stores a different caller-chosen tag list.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns those Pool tag lists.

- [x] Question Publication Validation requires Discipline and Subject before publication.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a new lineage when Discipline or Subject is absent and stores both on the Published Question.

- [x] Library Object classification follows Discipline -> Subject -> Topic -> Subtopic.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` refuses a Subtopic without a Topic, refuses a Topic from another Subject, refuses a Subtopic from another Topic, and stores one valid chain.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `published_question_metadata` rejects a Topic or Subtopic outside that chain.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that Question's Discipline, Subject, Topic, and Subtopic.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `content_topic_id` belongs to the Pool's Subject, and a Subtopic belongs to that Topic.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` stores the member Question's Discipline and Subject with Topic and Subtopic absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns that Pool classification.

- [x] Questions and Question Pools retain their Library Object classification when used in an Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `save_assessment` pins a Published Question on an Assessment and does not rewrite its Library Object classification.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `list_question_library_entries` returns that Question's Discipline, Subject, Topic, and Subtopic after the pin.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork` places a Question Pool on an Assessment by forking it.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies the source Pool's Discipline, Subject, Topic, and Subtopic.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns the source Pool and the Assessment fork with that same classification.

- [x] Library classification supports searching, filtering, sorting, and bulk editing.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `search_question_library_entries` searches and filters by Discipline, Subject, Topic, and Subtopic, and sorts the matching Questions by title or publication time.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` filters Question Pools by classification and returns them in Pool ID order.
  - Evidence (source): `schemas/base_schema/50_functions/published_question_metadata_operations.sql` `bulk_replace_published_question_metadata` replaces Subject, Topic, and Subtopic for a selected set of Published Questions.

- [x] Published Questions and Question Pools may have PLE-managed **Hints**, **Question Feedback**, and **Worked Solutions**.
  - Evidence (test): `crates/server/src/authoring.rs` `published_questions_include_optional_hint_feedback_and_worked_solution` kept omitted Published Question Hint and Worked Solution unchanged and replaced both when present.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `general_feedback` stores optional Pool Question Feedback beside hint and worked_solution. No PostgreSQL proof was run. No Live Demo stack was started.
  - Evidence (test): `crates/server/src/question_pool_support.rs` `pools_keep_optional_ple_managed_support_without_copying_member_questions` saved the three Pool texts and kept Edit Number 4.

- [x] Support content may be attached at the level where it applies rather than duplicated across individual Questions.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_support.sql` `save_question_pool_ple_managed_support` updates ple_data.question_pool and does not write question_revision. No PostgreSQL proof was run. No Live Demo stack was started.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies hint, general_feedback, and worked_solution onto the new Pool.
  - Evidence (test): `crates/server/src/question_pool_support.rs` `pools_keep_optional_ple_managed_support_without_copying_member_questions` refused a body that named publishedQuestionId and recorded no member Question id.

#### Question Library object usage statistics
- [x] Published Questions and Question Pools keep privacy-safe aggregate usage statistics.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `capture_issued_question_statistics_observation` records one observation for each Issued Question when the Attempt is submitted.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `increment_question_revision_statistics` adds that observation to the Published Question aggregate.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `increment_question_pool_issue_statistics` adds the Question Pool issue count and the member selection count.

- [x] Statistics are kept separately for each Published Question Revision and for each Question
  Pool.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` stores counts for one Published Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_revision_usage_statistics` returns each Revision on its own, including a Revision that has not been issued.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_pool_statistics` stores the Question Pool issue count apart from those Revision rows.

- [x] Aggregate statistics contain counts and sums, never Student Attempts or identifiable Student
  records.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` stores issued, blank, answered, outcome, and credit sums.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_usage_statistics` returns those counts and sums.

- [x] Instructors should be able to judge how often a Question is used and how hard it is.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_usage_statistics` returns issued, correct, and credit sums for an Instructor.
  - Evidence (source): `src/pages/question_statistics_panel.tsx` `QuestionStatisticsPanel` shows each rate beside its observation count.

- [x] Removing Student names alone does not make statistics anonymous.
  - Evidence (source): `docs/FERPA_DATA_POLICY.md` `Question Library object usage statistics` keeps Course, Student, Account, Attempt, and response facts in Student Work.
  - Evidence (source): `schemas/base_schema/20_tables/statistics.sql` `question_revision_statistics` stores counts and credit sums without a Student name or Student record.

- [x] Shared statistics should be shown only when individual Students cannot reasonably be identified
  from the aggregate.
  - Evidence (source): `crates/question_model/src/question_library_statistics.rs` `into_shared_statistics` withholds a positive count below `DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE` and omits a small Revision or Pool cell.
  - Evidence (source): `crates/server/src/question_library/usage_statistics.rs` `question_detail_statistics` and `bulk_question_statistics` call `into_shared_statistics`.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `pool_evidence` calls `into_shared_statistics` for the Pool issue count.
  - Evidence (test): `crates/question_model/src/question_library_statistics.rs` `shared_statistics_withhold_counts_that_can_identify_one_student` withheld one observation and kept an empty count available.

- [x] Course-specific analysis remains FERPA-sensitive when individual Students could be inferred.
  - Evidence (source): `schemas/base_schema/50_functions/statistics.sql` `read_question_library_usage_statistics` returns global counts and sums with no Course or Student breakdown.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `disclose_course_class_statistics` omits a course average unless the feedback policy releases it and the cohort meets `DEFAULT_STATISTICS_MINIMUM_COHORT_SIZE`.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `student_history` returns the `project_history` projection.
  - Evidence (test): `crates/server/src/assessment_delivery/history.rs` `course_class_statistics_stay_omitted_when_a_student_could_be_inferred` omitted a cohort of 1 and a cohort one below the minimum, omitted a safe cohort while class statistics were Never, and disclosed that safe cohort after AfterSubmit.

- [x] Schema and increment rules live in [DESIGN_DECISIONS.md](../../../docs/DESIGN_DECISIONS.md) and
  [FERPA_DATA_POLICY.md](/docs/FERPA_DATA_POLICY.md).
  - Evidence (source): `docs/DESIGN_DECISIONS.md` `Library usage statistics are retained counters` states the stored counts and the submission increment.
  - Evidence (source): `docs/FERPA_DATA_POLICY.md` `Question Library object usage statistics` states those schema and increment rules.

#### Question Library Bloom classification metadata
- [x] Published Question Revisions and Question Pools can have a Bloom Cognitive Process and Bloom
  Knowledge Dimension.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `question_revision_bloom` stores one Cognitive Process and one Knowledge Dimension for a Published Question Revision.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_bloom` stores one Cognitive Process and one Knowledge Dimension for a Question Pool.
  - Evidence (source): `crates/learning-data-access/src/question_library.rs` `PublishedQuestionLibraryEntry` returns that Question pair when a classification row exists.
  - Evidence (source): `crates/learning-data-access/src/question_pool_library.rs` `PublishedQuestionPool` returns that Pool pair when a classification row exists.

- [x] The two Bloom dimensions are independent and together determine the object's Bloom Classification.
  - Evidence (source): `schemas/base_schema/10_types.sql` `bloom_cognitive_process` is a separate closed vocabulary from the Knowledge Dimension.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `validate_bloom_pair` accepts any Cognitive Process with any Knowledge Dimension.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_revision_bloom` changes one stored dimension and leaves the other stored dimension in place.
  - Evidence (source): `schemas/base_schema/20_tables/published_question.sql` `question_revision_bloom` stores those two dimensions as the classification and has no derived matrix column.

- [x] Bloom Classification supports Question Library search and Assessment item sorting.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `sortByBloomClassification` orders mixed fixed and Pool Entries from their exact Bloom pairs, and `save` sends that order through the current Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_live_page.tsx` `reloadAssessment` loads the saved Assessment again after a concurrent save conflict.
  - Evidence (source): `src/components/library_bloom_discovery.tsx` `LibraryBloomDiscovery` puts the selected Cognitive Process and Knowledge Dimension on the Question Library search.
  - Evidence (test): `tests/playwright/test_bloom_classification_workflow.mjs` `mountBloomAssessmentWorkflow` drove the shipped Assessment Question editor through sort, save, a concurrent-save conflict, and reload of the saved order. `mountBloomLibraryBrowse` drove shipped Question Library browse and recorded a search for Remember and Factual Knowledge. The browser called the pages' client methods through a test double. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] A Question Pool's Bloom Classification describes the intended cognitive work of the Pool as a whole.
  - Evidence (source): `schemas/base_schema/20_tables/question_pool.sql` `question_pool_bloom` stores the Pool pair by question_pool_id.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `attach_question_pool_bloom` stores that prepared Pool pair on the Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_pool_bloom` updates the Pool pair without reading a member Question's Bloom classification.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns the Pool pair.

- [x] Bloom Classification is left blank when a Published Question or Question Pool enters the Question
  Library, to be updated by AI later.
  - Evidence (source): `schemas/base_schema/50_functions/question_publication_operations.sql` `publish_new_question_lineage` publishes a Question Revision and does not insert a Bloom row.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` creates a Question Pool and does not insert a Bloom row.
  - Evidence (source): `schemas/base_schema/50_functions/question_library_operations.sql` `load_question_library_revision` returns a null Bloom pair when that row is absent.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `list_published_question_pools` returns a null Bloom pair when the Pool row is absent.

- N/A AI assigns the initial Bloom Classification using a daemon after publication.
  - Reason: Deferred product behavior overrides this implementation language. Initial Bloom Classification is deferred with the AI backend, AI-backed Bloom classification is deferred until a later release, and automated daemon backends are deferred until a final server location. Publication does not start a daemon.

- [x] An **Instructor** can correct either Bloom dimension without creating a new Published Question
  Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_revision_bloom` updates one stored dimension for an active Instructor and does not insert a Question Revision.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` `correct_question_pool_bloom` updates one stored Pool dimension and does not insert a Question Revision.

- [ ] Question Library search and reporting should make both Bloom dimensions useful to **Instructors**.
  - Evidence (source): `crates/question_model/src/question_search.rs` retains two independent exact Bloom filters, unchanged sorts, and normalized-query-bound cursors. `crates/learning-data-access/src/postgres/question_library.rs` applies them to the whole Library relation and computes all six plus all four guide-order counts; `src/pages/library_search_parameters.ts`, `src/pages/library_page.tsx`, and `src/components/library_bloom_discovery.tsx` retain URL/saved-search values, zeros, and empty results.
  - Verification pending: connected multi-page, role, and browser proof remains required. It stays open independently of the connected mixed-entry Assessment-sort/save/reload/concurrent-save proof required by the preceding row.

- [ ] Follow `docs/BLOOM_TAXONOMY_GUIDE.md` for Bloom classification and teaching interpretation.
  - Evidence (source): `schemas/base_schema/50_functions/question_bloom.sql` accepts only the guide's six Cognitive Process and four Knowledge Dimension spellings.
  - Verification pending: fresh PostgreSQL actual-role proof closes storage and publication-required attachment; classifier/provider semantics, Instructor-facing teaching interpretation, and connected search/reporting remain open.

#### Question Library stewardship specifications
- [ ] Question Library stewardship should use a GitHub-like model.
  - Evidence (runtime): `docs/archive/audits/sql_human_guidance_audit.md` records current Question/Pool Star and Watch SQL/LDA proof plus four-event private Watch delivery.
  - Verification pending: connected Question/Pool workflows and browser presentation remain open.

- [x] Published Questions and Question Pools can be starred and watched.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `set_current_question_star` records an Instructor Star, and `set_current_question_watch` records that Instructor's Question Watch.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `set_current_question_pool_star` records a Pool Star, and `set_current_question_pool_watch` records that Instructor's Pool Watch.
  - Evidence (source): `src/components/question_star_control.tsx` `QuestionStarControl` and `src/components/question_pool_star_control.tsx` `QuestionPoolStarControl` expose the Star actions.
  - Evidence (source): `src/components/question_watch_control.tsx` `QuestionWatchControl` and `src/components/question_pool_watch_control.tsx` `QuestionPoolWatchControl` expose the Watch actions.
  - Evidence (test): `tests/test_frontend_contract.mjs` `Published Questions and Question Pools can be starred and watched.` starred Question 7K3M-79QP and Pool 3S8B-24DZ, watched both through the shipped client, and rendered Star question and Star Pool. `tests/test_question_watch_client.mjs` `Watch means subscription.` rendered Watch and Unwatch for a Question and a Pool. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Star means favorite and visible endorsement.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `set_current_question_star` records an active Instructor's Star only for a Published Question; `src/components/question_star_control.tsx` `QuestionStarControl` provides the visible Star and count surface.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 with an active vetted Instructor's actual HTTP Star action and exact closed Star projection.

- [x] Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Published
  Question or Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `verified_instructor_display_name` returns the display name from the completed Instructor identity vetting decision.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `read_current_question_star` sets star_count to the active endorsers whose vetted display name is present, the same names the list returns.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `read_current_question_pool_star` sets star_count to the active endorsers whose vetted display name is present, the same names the list returns.
  - Evidence (source): `crates/server/src/question_stewardship.rs` `read_star` returns that Question projection only after `instructor_session_hash`.
  - Evidence (source): `crates/server/src/question_pool_stewardship.rs` `read_star` returns that Pool projection only after `instructor_session_hash`.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionStarControl` shows the count and names when `mayMutateLibrary` admits the Instructor role.
  - Evidence (source): `src/pages/library_page.tsx` `mayMutateLibrary` is the Instructor role and is passed to Pool discovery as `mayWatchPools`.
  - Evidence (source): `src/pages/library_pool_discovery.tsx` `QuestionPoolStarControl` shows the Pool count and names on that Instructor surface.
  - Evidence (test): `tests/test_frontend_contract.mjs` `Vetted Instructors can see the star count and which vetted Instructors starred a Published Question or Question Pool.` rendered star count 2 and the names Ada Lopez and Grace Hopper for Question 7K3M-79QP and Pool 3S8B-24DZ through the shipped Star controls and HTTP client.

- [x] Watch means subscription.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `set_current_question_watch` inserts or deletes only the current Instructor's Question Watch row.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `set_current_question_pool_watch` inserts or deletes only the current Instructor's Pool Watch row.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `snapshot_library_watch_event_recipients` copies those Question and Pool Watch rows as the notification recipients.
  - Evidence (source): `src/components/question_watch_control.tsx` `QuestionWatchControl` shows Watch or Unwatch for that Instructor's own boolean.
  - Evidence (source): `src/components/question_pool_watch_control.tsx` `QuestionPoolWatchControl` shows Watch or Unwatch for that Instructor's own boolean.
  - Evidence (source): `src/pages/question_detail_page.tsx` `QuestionWatchControl` is on the Instructor Question surface.
  - Evidence (source): `src/pages/library_pool_discovery.tsx` `QuestionPoolWatchControl` is on the Instructor Pool surface.
  - Evidence (test): `tests/test_question_watch_client.mjs` `Watch means subscription.` rendered Watch and Unwatch for Question 7K3M-79QP and Pool 3S8B-24DZ, subscribed and unsubscribed through the shipped Watch client, and rejected a Pool watcher count.

- [x] Watching a Published Question drives in-app notifications for new Revisions, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_revision_event` records a Published Question Revision, and `snapshot_library_watch_event_recipients` delivers that event to the Question's current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_watch_fork_event` records a Published Question fork, and `enqueue_library_watch_thread_event` records an improvement thread for that watched Question.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for that watched Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `eventLabel` names New Revision, New public fork, Improvement thread activity, and Impact notice activity in the Watch inbox.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Published Question Revision, fork, improvement thread, and impact notice through the shipped Watch client. `crates/server/src/library_watch_notification.rs` `every_watch_activity_serializes_to_the_stable_wire_shape` serialized those four Question activities. No Live Demo stack was started. No PostgreSQL proof was run.
  - Owner: earlier Human Guidance occurrence of this bullet.

- [x] Watching a Question Pool drives in-app notifications for membership edits, forks, improvement
  threads, and impact notices.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_members_changed_event` records a Pool membership edit for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_question_pool_watch_fork_event` records a Pool fork for current Watchers.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_thread_event` records an improvement thread for a watched Pool.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `enqueue_library_watch_impact_event` records an impact notice for a watched Pool.
  - Evidence (source): `crates/learning-data-access/src/postgres/library_watch_notification.rs` `MembersChanged` reads that membership edit into the private Watch inbox and `require_pool_membership_target` refuses it for a Published Question.
  - Evidence (source): `src/pages/library_watch_notifications_page.tsx` `membersChanged` labels the inbox row Membership edit.
  - Evidence (test): `tests/test_library_watch_notification_client.mjs` `Library Watch inbox returns every discriminated event shape privately` decoded a Question Pool membersChanged notification with Edit Number 5 through the shipped Watch client, and rejected a membership edit aimed at a Published Question. watch_activity_accepts_each_complete_variant accepted members_changed for a Pool and rejected it for a Question. every_watch_activity_serializes_to_the_stable_wire_shape serialized membersChanged. No Live Demo stack was started. No PostgreSQL proof was run.
  - Owner: earlier Human Guidance occurrence of this bullet.

- [x] An **Instructor's** watch list remains private.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `read_current_question_watch` returns only the current Instructor's watching boolean and does not enumerate other Instructors' watches.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `read_current_question_pool_watch` returns only the current Instructor's watching boolean and does not enumerate other Instructors' watches.
  - Evidence (source): `crates/server/src/question_watch.rs` `WatchResponse` serializes only that Instructor's watching boolean.
  - Evidence (source): `crates/server/src/question_pool_stewardship.rs` `WatchResponse` serializes only that Instructor's watching boolean.
  - Evidence (source): `src/api/decoders/question_watch.ts` `decodeQuestionWatchProjection` accepts only the watching field.
  - Evidence (source): `src/api/decoders/question_pool_stewardship.ts` `decodeQuestionPoolWatchProjection` accepts only the watching field.
  - Evidence (source): `src/components/question_watch_control.tsx` `QuestionWatchControl` shows Watch or Unwatch for the caller and does not list other Instructors.
  - Evidence (source): `src/components/question_pool_watch_control.tsx` `QuestionPoolWatchControl` shows Watch or Unwatch for the caller and does not list other Instructors.
  - Evidence (test): `tests/test_question_watch_client.mjs` `An Instructor's watch list remains private.` rejected a Question response that added watchList, a Pool response that added watchers, and an inbox response that added watchList, then rendered only the caller's Watch and Unwatch controls for Question 7K3M-79QP and Pool 3S8B-24DZ. The test does not open another Instructor's session.

- [x] **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
  - Evidence (source): `schemas/base_schema/50_functions/question_stewardship.sql` `read_current_question_star` refuses a Student or empty session before returning starred Instructor names. `read_current_question_watch` refuses those sessions before returning Watch information.
  - Evidence (source): `schemas/base_schema/50_functions/question_pool_stewardship.sql` `read_current_question_pool_star` refuses a Student or empty session before returning starred Instructor names. `read_current_question_pool_watch` refuses those sessions before returning Watch information.
  - Evidence (source): `schemas/base_schema/50_functions/question_watch_notifications.sql` `read_current_library_watch_notifications` refuses a Student or empty session before returning the Watch inbox.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `students_and_anonymous_users_do_not_receive_instructor_identity_lists_or_watch_information` refused Student USSV00009 and an empty session on Question SVR1-4XYZ and Pool SVP1-FABC, then let Instructor UVSV0000A open the Question Star list, Question Watch, and the Watch inbox.
  - Evidence (test): `tests/e2e/e2e_assessment_saved_response.sh` `students_and_anonymous_users_do_not_receive_instructor_identity_lists_or_watch_information` ran that oracle on PostgreSQL. No Live Demo stack was started.
## Course specifications
- [x] **Courses** organize reusable teaching content and its delivery to **Students**.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `blueprint_course` stores the reusable teaching design.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `course_instance` stores the teaching delivery of that design.
  - Evidence (source): `schemas/base_schema/10_types.sql` `membership_role` includes student on a Course Instance.

- [x] PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `blueprint_course` is the Blueprint Course form.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `course_instance` is the Course Instance form.
  - Evidence (source): `schemas/base_schema/10_types.sql` `course_source_kind` records whether a Course Instance started empty or was adopted from a Blueprint Course.

- [x] **Blueprint Courses** provide reusable course designs for **Course Instances**.
  - Evidence (source): `crates/learning-data-access/src/blueprint_course.rs` `BlueprintCourseStore` persists the reusable Blueprint design.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` creates a Course Instance from a Public Blueprint.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` adopted one Public Blueprint into two Course Instances. The disposable PostgreSQL database baseline required this ignored test and exited 0.

- [x] Course Instances may start independently with no parent Blueprint Course, or an **Instructor** may create them from a Blueprint Course.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CourseInstanceCreationSource` and `src/api/decoders/course_instance.ts` `decodeCreateCourseInstanceInput` accept strict Empty or exact Adopted source forms.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised against the actual server in bounded exact-main browser proof: Empty creation persisted without Blueprint-list requests; separate Public Blueprint exact-Revision adoption created a daughter Course and Unreleased Practice Assessment. Successful API responses were `no-store`. This creation-only row does not establish direct started-empty Assessment authoring or the full teaching lifecycle.

- [x] A Course can have multiple co-**Instructors** with equal teaching authority.
  - Evidence (source): `schemas/base_schema/10_types.sql` `membership_role` has only student and instructor.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `add_course_instructor` inserts that same instructor role for a later co-Instructor when `current_session_account_is_course_instructor` accepts the current Instructor.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `add_course_instructor` uses Course membership as the authority boundary.
  - Evidence (test): `crates/learning-data-access/tests/course_instance_postgres.rs` `empty_course_has_no_initial_content_and_current_instructors_are_peers` had the first Instructor add a co-Instructor, had that co-Instructor add a third Instructor, and rejected a non-member. The disposable PostgreSQL database baseline required this ignored test and exited 0.

- [x] **Sysadmins** can create Courses, but **Instructors** teach them.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminCourseCreation` is the Sysadmin home control that creates a Course for an Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` inserts the selected Instructor membership when the session is a Sysadmin.
  - Evidence (source): `src/components/sysadmin_course_creation.ts` `submitSysadminCourseCreation` sends assignedInstructorAccountId and leaves the Sysadmin off that membership.
  - Evidence (test): `tests/test_frontend_contract.mjs` `a Sysadmin creates a Course that an Instructor teaches` sent an empty Course for Instructor U0000035E and refused a blank Instructor Account ID before creation.

- [x] Every Course Instance must have at least one assigned **Instructor**.
  - Evidence (source): `schemas/base_schema/50_functions/course_membership.sql` `assert_assigned_instructor_membership` rejects a Course Instance with no current Instructor membership.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` records that Instructor membership when the Course Instance is created.

- [x] Creating a Course Instance establishes its first Instructor membership but does not give that Instructor greater Course authority than later co-Instructors.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` inserts the first membership with role instructor, and `add_course_instructor` inserts each later co-Instructor with that same role.
  - Evidence (source): `schemas/base_schema/50_functions/authorization.sql` `current_session_account_is_course_instructor` accepts any active instructor membership on that Course.
  - Evidence (test): `crates/learning-data-access/tests/course_instance_postgres.rs` `empty_course_has_no_initial_content_and_current_instructors_are_peers` started from one Instructor membership, then the later co-Instructor added another peer. The disposable PostgreSQL database baseline required this ignored test and exited 0.

- [x] **Adoption** connects a Blueprint Course and a Course Instance when an **Instructor** creates a new Course Instance from a Blueprint Course or creates a new Blueprint Course from an existing Course Instance's reusable structure.
  - Evidence (runtime): `schemas/base_schema/50_functions/course_blueprint_publication.sql` `ple_api.create_blueprint_from_course_instance` records the source Course as a distinct first Adoption; the existing Blueprint-to-Course path records daughters separately. Fresh PostgreSQL actual-role proof passed source preservation and Adoption-count checks.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` passed canonical HTTPS C420 browser proof: one child-route POST created Private Revision-1 Blueprint `BPJD8H28` from Course `CI0QR41X`, showed Adoption count 1, and left the source addressable and unchanged.

- [x] An Instructor may create a new Blueprint Course from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source, and the Course Instance remains the same teaching instance.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseInstancePage` exposes a metadata-only Create Blueprint from Course Instance dialog; `src/api/http_client/course_instance.ts` `createBlueprintFromCourseInstance` validates the canonical Course reference, sends only generated names/classification with one retry-safe idempotency key, requires `201 no-store`, and accepts only a new owner-visible Private Revision-1 Blueprint receipt.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` passed authorization/no-write, stale rollback, replay, exact reusable content/Pool pins, immutable source provenance, unchanged source state, first-Adoption/student counts, and lifecycle rollback on PostgreSQL.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` passed canonical HTTPS C420 browser proof: one child-route POST created actor-owned Private Revision-1 Blueprint `BPJD8H28` from Course `CI0QR41X`, showed Adoption count 1, and left the source addressable and unchanged.

- [x] A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/50_functions/course_core.sql` `course_instance` records Blueprint reference and Revision source columns.

### Course classification specifications
- [x] **Blueprint Courses** and **Course Instances** use the shared content classification system.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `content_discipline` stores a Blueprint Course on the shared Discipline vocabulary, with the same Subject, Topic, Subtopic, and Tag columns.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `content_discipline` stores a Course Instance on that same vocabulary.
  - Evidence (source): `crates/question_model/src/course_classification.rs` `CourseClassification` is the classification both Course forms save.

- [x] Course classification describes the Course as a whole.
  - Evidence (runtime): Accepted Course metadata SQL/Store/browser reviews and `/private/tmp/ple-course-classification-actual-role-result.log` establish independent Course-owned metadata without changing content Revisions or Question pins. Root's rebuilt `8147` ordinary Course/Blueprint editor proof (script `/private/tmp/ple-course-classification-no-workaround-20260916.mjs`, session 85118 exit 0) saves Tags-only metadata and preserves unsaved Blueprint names. Source owner: `crates/question_model/src/course_classification.rs` `CourseClassification`.

- [x] Every Blueprint Course and Course Instance has exactly one **Discipline**.
  - Evidence (runtime): Accepted required `CourseClassification` source and actual-role SQL proof at `/private/tmp/ple-course-classification-actual-role-result.log` reject missing/nonexistent Discipline. Rebuilt `8147` ordinary Course and Blueprint creation/editor proof selects Biology explicitly and hydrates it without reselection; session 85118 exited 0. Source owner: `crates/question_model/src/course_classification.rs` `CourseClassification`.

- [x] Courses may optionally have one **Subject**, one **Topic**, and one **Subtopic**.
  - Evidence (runtime): Accepted `CourseClassification` source and actual-role SQL proof at `/private/tmp/ple-course-classification-actual-role-result.log` validate optional hierarchy and parent constraints. Root's rebuilt `8147` ordinary Course/Blueprint creation and Tags-only saves succeed with only Biology and no narrower levels; session 85118 exited 0. Source owner: `crates/question_model/src/course_classification.rs` `CourseClassification`.

- [x] Courses may have any number of **Tags**, including none.
  - Evidence (runtime): Accepted actual-role SQL proof at `/private/tmp/ple-course-classification-actual-role-result.log` exercises empty Tags and 65 Tags without a count cap. Rebuilt `8147` ordinary Course/Blueprint Tags-only browser saves pass without Discipline reselection; session 85118 exited 0. Per-Tag validation remains bounded. Source owner: `crates/question_model/src/course_classification.rs` `CourseClassification`.

- [x] Course classification follows the shared Discipline -> Subject -> Topic -> Subtopic hierarchy.
  - Evidence (source): `schemas/base_schema/20_tables/course_instance.sql` `content_subject_discipline` requires a Course Subject to belong to its Discipline, and the Topic and Subtopic foreign keys continue that chain.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `content_subject_discipline` requires the same chain on a Blueprint Course.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `list_content_subjects` returns Subjects for the selected Discipline, and `list_content_topics` and `list_content_subtopics` continue that chain.

- [x] Course Discipline selection should provide a clear way to request a new Discipline when the needed
  Discipline is unavailable.
  - Evidence (source): `src/components/course_classification_fields.tsx` `DisciplineRequestControl` asks for a Discipline the Course selector does not offer.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `request_content_discipline` stores the requested name for an Instructor or Sysadmin.
  - Evidence (source): `src/pages/content_disciplines_page.tsx` `ContentDisciplinesPage` lists open requests with the requester Account ID.
  - Evidence (source): `src/components/discipline_request.ts` `createDisciplineFromRequest` calls createDiscipline and then resolveDisciplineRequest.
  - Evidence (test): `tests/test_frontend_contract.mjs` `Course Discipline selection requests a new Discipline without creating it` recorded trimmed Genetics through requestContentDiscipline.
  - Owner: Course interfaces (first occurrence).

- [x] **Sysadmins** exclusively create and manage Disciplines.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `create_content_discipline` creates a Discipline only for an active Sysadmin.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `rename_content_discipline` changes that name only for an active Sysadmin.
  - Evidence (source): `schemas/base_schema/50_functions/content_classification_operations.sql` `retire_content_discipline` retires a Discipline and `restore_content_discipline` returns it, both only for an active Sysadmin.
  - Evidence (source): `src/route_contract.ts` `contentDisciplines` admits only the sysadmin role, and `src/pages/content_disciplines_page.tsx` `ContentDisciplinesPage` is that page.

- [x] Course classification supports Course search, filtering, organization, and discovery where applicable.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `create_blueprint_course` stores the classification used for Public Blueprint discovery.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` searches, filters, and sorts those Public Blueprint Courses by classification.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns Course classification in long-name order.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` shows that classification on each Course record.

- [x] A Course Instance may have classification that differs from its Blueprint Course.
  - Evidence (runtime): Accepted actual-role SQL proof at `/private/tmp/ple-course-classification-actual-role-result.log` verifies fork/Instance classification independence. The earlier connected Course browser receipt verifies explicit daughter classification and source independence; its selector workaround is superseded only by rebuilt `8147` ordinary editor hydration/Tags-only proof (session 85118 exit 0), not by a new adoption journey. Source owner: `crates/question_model/src/course_classification.rs` `CourseClassification`.

### Blueprint Course specifications
- [x] **Blueprint Courses** are reusable course definitions for building **Course Instances**.
  - Evidence (source): `crates/learning-data-access/src/blueprint_course.rs` `BlueprintCourseStore` persists reusable Blueprint content and revisions.

- N/A Blueprint Courses are a similar concept as LibreTexts' ADAPT alpha courses.
  - Reason: This comparison provides human-oriented product context, not an implemented PLE behavior.

- [x] Blueprint Courses have no **Students**, deadlines, or other teaching-specific delivery settings.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `blueprint_course` stores the lineage and its Revision content, with no Student membership or deadline columns.
  - Evidence (source): `crates/question_model/src/blueprint_course/assessment_content.rs` `BlueprintAssessmentDefaults` stores reusable limits and feedback policy, not a roster or a deadline.

- [x] Blueprint Courses do not contain dates or relative schedules.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `validate_course_blueprint_adoption` accepts a new Course only when available_at, due_at, and closes_at are null.
  - Evidence (source): `crates/question_model/src/blueprint_course/assessment_content.rs` `BlueprintAssessmentDefaults` has no calendar date or relative schedule field.

- [x] Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` returns a Public Blueprint to any active Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` accepts that Public Blueprint as a daughter Course Instance source.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` had a non-owner Instructor list, load, and adopt the Public Blueprint. The disposable PostgreSQL database baseline required this ignored test and exited 0.

- [x] Blueprint Courses contain only **Published Questions** and published **Question Pools**.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `validate_blueprint_question_selection` refuses a new pin that is not an Available Question.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_pools.sql` `validate_blueprint_owned_pools` refuses a Pool that is not a fresh fork owned by the Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_pools.sql` `fork_blueprint_question_pool` copies the published Pool into that fork.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `save_blueprint_course` records the Revision that names the available Question and the fork.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `blueprint_courses_contain_only_published_questions_and_published_pools` pinned available Question SVR1-4XYZ, refused archived Question SVQ1-7ABC, refused published Pool SVP1-FABC, and stored fork SVB1-XABC of that Pool on Private Blueprint BPVP00000T Revision 2.
  - Evidence (test): `tests/e2e/e2e_assessment_saved_response.sh` `blueprint_courses_contain_only_published_questions_and_published_pools` ran that oracle on PostgreSQL.

- [x] An **Instructor** may create a new Blueprint Course from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseInstancePage` supplies the Course tools dialog; `src/api/http_client/course_instance.ts` `createBlueprintFromCourseInstance` supplies its strict, idempotent create request; `schemas/base_schema/50_functions/course_blueprint_publication.sql` `ple_api.create_blueprint_from_course_instance` owns the atomic source lock/copy/provenance boundary.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` passed immutable source provenance, unchanged Course state, replay, stale rollback, exact pins, and first-Adoption counts on PostgreSQL.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` passed canonical HTTPS C420 browser proof creating distinct Private Revision-1 Blueprint `BPJD8H28` from Course `CI0QR41X` while leaving the source unchanged.

- [x] Creating a Blueprint Course from a Course Instance copies the ordered Course Instance Assessment list as ordered Blueprint Assessments, preserving order.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_blueprint_publication.rs` `load_course_blueprint_publication_source` uses the existing visible Course order and `PostgresCourseBlueprintPublicationStore` preserves that vector under one deterministic `Assessments` wrapper with fresh Blueprint Assessment identities.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` passed canonical HTTPS C420 browser proof showing Private Revision-1 Blueprint `BPJD8H28` retained reusable structure copied from unchanged Course `CI0QR41X`.

#### Blueprint Course lifecycle specifications
- [x] Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Evidence (source): `schemas/base_schema/10_types.sql` `blueprint_availability` stores only private, public, and archived.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_model.ts` `blueprintLifecyclePresentation` gives each of those states its own editing and adoption controls.

- [x] New Blueprint Courses and forks start Private.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `availability` defaults to private, and creation does not accept another initial availability.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `fork_blueprint_course` inserts the new Blueprint with availability private.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` loaded a newly created Blueprint as Private. The disposable PostgreSQL database baseline required this ignored test and exited 0.

- [x] Private Blueprint Courses are visible only to their owning **Instructor**.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `list_blueprint_courses` includes a Private Blueprint only for its owner.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `load_blueprint_course` returns a Private Blueprint only to its owner.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/lifecycle_privacy.rs` `assert_private_blueprint_is_owner_only` kept the Private Blueprint on the owner list and returned NotFound for a non-owner list, load, and Revision load. The disposable PostgreSQL database baseline required the lifecycle test that calls this check and exited 0.

- [x] Instructors may develop and use Private Blueprint Courses without publishing them.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_model.ts` `blueprintLifecyclePresentation` permits the owner to edit a Private Blueprint and withholds adoption; Private is deliberately not a daughter-Course source.

- [x] Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` rejects a Blueprint whose availability is not public before it creates a Course Instance.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `load_course_instance_blueprint` returns Blueprint content only when availability is public.

- [x] Making a Blueprint Course Public adds it to the shared Blueprint Course collection.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.set_blueprint_availability` publishes owner content and `ple_api.list_blueprint_courses` includes Public Blueprints for active Instructors.

- [x] Public Blueprint Courses and their Revision history are visible to all vetted **Instructors**.
  - Evidence (runtime): `schemas/base_schema/50_functions/blueprint_history.sql` `ple_api.list_blueprint_history` uses ordinary visibility for Revision and metadata facts. Accepted Public-history proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json`.

- [x] Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` creates the daughter Course Instance from a Public Blueprint and calls `initialize_course_assessments`.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/lifecycle.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` had a non-owner Instructor adopt the Public Blueprint twice. The disposable PostgreSQL database baseline required this ignored test and exited 0.

- [x] A Public Blueprint Course with no adoptions may return to Private.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.set_blueprint_availability` permits this transition only before a daughter Course Instance exists; the accepted lifecycle runtime contract covers the rule.

- [x] A Public Blueprint Course with one or more adoptions remains Public.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.set_blueprint_availability` rejects Public-to-Private after an adoption; the accepted lifecycle runtime contract exercises the denial.

- [x] Blueprint Courses have no separate Draft state.
  - Evidence (source): `schemas/base_schema/10_types.sql` `blueprint_availability` has no Draft value.
  - Evidence (source): `crates/question_model/src/blueprint_operations/contracts.rs` `BlueprintAvailability` is Private, Public, or Archived.

#### Archived Blueprint Course specifications
- [x] Archived Blueprint Courses are read-only and no longer actively maintained.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_model.ts` `blueprintLifecyclePresentation` withholds content editing for an Archived Blueprint Course.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `save_blueprint_course` rejects an Archived Blueprint Course before it writes a Revision. `rename_blueprint_course` and `update_blueprint_classification` reject Archived name and classification writes.

- [x] Archived Blueprint Courses and their Revision history remain visible to all vetted **Instructors**.
  - Evidence (runtime): `schemas/base_schema/50_functions/blueprint_history.sql` `ple_api.list_blueprint_history` uses ordinary visibility for Archived Revision and metadata facts. Accepted Archived-history and discovery proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json` and `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json`.

- [x] Archived Blueprint Courses do not appear in normal discovery unless explicitly included.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.list_blueprint_courses` filters Public, owning Private, and only explicitly requested Archived records; `crates/server/src/blueprint_course.rs` `BlueprintCourseListQuery` accepts only the typed `includeArchived` boolean.
  - Evidence (runtime): `src/features/blueprint_course/blueprint_courses_workspace.tsx` `changeIncludeArchived`; `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json` records the actual compiled-main default-off, Include Archived, read-only Archived-detail, and return-to-off workflow with eight GETs and zero writes. Its companion HTTP receipt records default/false/true membership and strict invalid-query `400` results.

- [x] Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Blueprint availability `public` for exact-Revision adoption.

- [x] Archived Blueprint Courses can be forked.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_create.tsx` `canFork` offers the fork action for a Public or Archived Blueprint Course.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `fork_blueprint_course` accepts a Public or Archived source.

- [x] Forking an Archived Blueprint Course creates a new Private Blueprint Course.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `fork_blueprint_course` inserts the child Blueprint Course with availability private.
  - Evidence (source): `src/api/http_client/blueprint_course.ts` `forkBlueprintCourse` accepts only a response for a different Private Blueprint Course.

- [x] The owning **Instructor** can return an Archived Blueprint Course to Public.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` `restore_blueprint` sets availability to public.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `set_blueprint_availability` permits that change only for the owning Instructor, and only from archived to public.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `restoreBlueprintCourse` applies the returned Public availability and leaves the current Revision in place.

- [x] Blueprint Course visibility includes its content, Revision history, and recorded changes.
  - Evidence (runtime): `schemas/base_schema/50_functions/blueprint_history.sql` `ple_api.list_blueprint_history` provides separate, ordinary-visibility Revision and metadata-event pages; `src/features/blueprint_course/blueprint_history.tsx` `BlueprintHistory` presents both read-only. Accepted bounded proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json`.

- [x] Visibility does not grant editing authority.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `load_blueprint_course` returns a Public or Archived Blueprint to any active Instructor and reports ownership separately from that read.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `rename_blueprint_course` refuses a non-owner before changing the name.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `update_blueprint_classification` refuses a non-owner before changing classification.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `set_blueprint_availability` refuses a non-owner before changing availability.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `save_blueprint_course` refuses a non-owner before writing a Revision.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `visibility_does_not_grant_editing_authority` let a second Instructor read Public Blueprint BPVS00000W and its Revision, refused rename, classification, archive, and Save, and kept the owner no-op rename at Edit Number 1.
  - Evidence (test): `tests/e2e/e2e_assessment_saved_response.sh` `visibility_does_not_grant_editing_authority` ran that oracle on PostgreSQL.

#### Blueprint Course revision specifications
- [x] Blueprint Courses use immutable **Blueprint Revisions** for saved reusable content.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_revision_integrity.sql` `blueprint_course_revision_is_immutable` rejects Revision updates and deletes.

- [x] Blueprint Course content editing uses explicit Save.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `save_blueprint` is the explicit content-save route handler.

- [x] Saving changed Blueprint content creates the next Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `save_blueprint_course` inserts the next Blueprint Revision when the saved content differs from the current Revision.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `Saved Blueprint Revision` reports the Revision number returned after that Save.

- [x] Multiple content edits before Save become one Blueprint Revision.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `ReplaceBlueprintCourseContentInput` carries one complete replacement tree per Save.

- [x] Saving unchanged Blueprint content does not create another Revision.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `save_blueprint_course` keeps the expected Revision when the submitted content is not distinct.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_structure.tsx` `Save Blueprint Course` stays disabled until the editable content differs from the saved Revision.

- [x] Blueprint Course metadata can change without creating a Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.rename_blueprint_course` updates `blueprint_course` metadata without inserting a `blueprint_course_revision`.

- [x] Blueprint Course names are metadata and identify the Blueprint across Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `blueprint_course` owns names while `blueprint_course_revision` keys content by course reference and revision.

- [x] Changing a Blueprint Course name does not create a new Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `ple_api.rename_blueprint_course` updates names and metadata ETag without inserting a `blueprint_course_revision`.

#### Blueprint Course stewardship specifications
- [x] Blueprint Courses have a searchable boolean Promoted flag.
  - Evidence (source): `src/pages/blueprint_course_search_page.tsx` `draftPromotedOnly` keeps Promoted only on Public Blueprint Course search and sends it with the list request.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `p_promoted_only` keeps a course when the flag is off or the course promoted value is true.

- [x] Sysadmins exclusively control the Promoted flag.
  - Evidence (source): `crates/server/src/blueprint_course/promotion.rs` `sysadmin_session_hash` admits only a Sysadmin session before promotion load or set.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `set_blueprint_promotion` raises Blueprint promotion forbidden unless the session is a Sysadmin.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminHomePage` mounts the promotion control; Instructor and Student homes do not.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/promotion.rs` `promotion_boundary` denies promotion load and set for the Instructor owner token and a Student.

- [x] **Instructors** can Star or Watch Public and Archived Blueprint Courses.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_workspace.tsx` `BlueprintStewardship` mounts on a Public or Archived Blueprint Course and stays off a Private one.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_stewardship.sql` `set_current_blueprint_course_star` accepts only a Public or Archived Blueprint Course for an active Instructor, as does `set_current_blueprint_course_watch`.

- [x] A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
  - Evidence (source): `src/features/blueprint_course/blueprint_stewardship.tsx` `A Star is a visible endorsement` is the panel copy next to the Instructor's own Star control.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_stewardship.sql` `set_current_blueprint_course_star` inserts that Instructor's Star on the Blueprint Course.

- [x] Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_stewardship.sql` `verified_instructor_display_name` is required for the viewer and for each listed Instructor, and `read_current_blueprint_course_star` returns the Star count.
  - Evidence (source): `src/features/blueprint_course/blueprint_stewardship.tsx` `Instructors who Starred this Blueprint` lists those names beside the Star count.

- [x] Watching a Blueprint Course is private.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `A Watch is private` states there is no watcher count, list, or identity projection.
  - Evidence (source): `src/features/blueprint_course/blueprint_stewardship.tsx` `Your Watch preference and activity are private` is the visible copy, and Watch events render only under Your private Watch activity.

- [x] Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_stewardship.sql` `blueprint_revision_enqueues_watch_notifications` fans a new Revision out to active Instructor watchers, and `enqueue_blueprint_course_watch_lifecycle_change` fans out published, archived, and restored.
  - Evidence (source): `src/features/blueprint_course/blueprint_stewardship.tsx` `Your private Watch activity` labels revision, published, archived, and restored events for the current Instructor.

- [x] Forking or adopting a Blueprint Course does not automatically Star or Watch it.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `fork_blueprint_course` inserts the child Blueprint Course, Revision, fork row, and fork receipt, and does not insert a Star or Watch.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` inserts the Course Instance, origin, and membership, and does not insert a Star or Watch.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_stewardship.sql` `set_current_blueprint_course_star` is the insert into blueprint_course_star, and `set_current_blueprint_course_watch` is the insert into blueprint_course_watch.

- [x] Stars and Watches belong to the Blueprint Course across all of its Revisions.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `PRIMARY KEY (blueprint_course_id, instructor_account_id)` keys both blueprint_course_star and blueprint_course_watch by the Blueprint Course, with no Revision column.

#### Blueprint adoption and incorporation specifications
- [x] Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_instance.rs` `create_course_instance` obtains `creation_assignments` before atomic creation.

- [x] Course Instances pin the exact Blueprint Revision from which they were adopted.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CourseInstanceCreationSource` requires an exact immutable Blueprint Revision source for adoption.

- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

- [x] Routine Blueprint changes should be quick for an **Instructor** to review and incorporate.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

- [x] It should be obvious when a Course Instance is based on an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` carry the adopted and current Revision numbers and render the older-Revision notice with strict `bigint` comparisons.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was exercised by accepted independent actual-server/exact-main browser proof across empty, current, newer, and explicit synthetic Private-origin states. The newer state visibly showed its original adopted Revision and the current newer Revision; Student and unrelated-Instructor reads returned nonenumerating `404 no-store`, no extra Blueprint fetch or write occurred, and browser errors were empty. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This read-only notice makes a stale daughter obvious. It does not offer, review, approve, or apply a Blueprint update.

- [x] The **Instructor** decides which changes to existing Assessments to incorporate.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` offers each existing Assessment for separate review and does not apply the Course's updates together.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `applyBlueprintUpdate` applies only the Assessment the Instructor reviewed, using that source Revision and Edit Number.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_blueprint_updates.sql` `apply_assessment_blueprint_update` updates that one Assessment and leaves the other Course Assessments unchanged.
  - Evidence (test): `tests/test_assignment_client.mjs` `The Instructor decides which changes to existing Assessments to incorporate` posted one Assessment update and withheld a request that added another Assessment.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` validates the new-reference delta and inserts only new Assessments; it does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` changes a retained source Assessment title and proves existing daughter Assessment content, entries, and actual Student Work unchanged through Save/replay/no-op/stale operations. Accepted artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: This negative invariant remains separate from the verified Course-review workflow; it does not claim direct-Assessments or all Student Work.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (test): `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `newly_added_blueprint_assessments_are_the_daughter_append` keeps only the new Assessment and leaves release status and delivery dates off the daughter copy payload.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `append_course_assessments` inserts that Assessment without a release status or delivery dates.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_status` defaults to unreleased.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_append_projection` requires the appended daughter Assessment to be unreleased with null available, due, and close times.

#### Blueprint Course fork specifications
- [x] An **Instructor** can fork a Public or Archived **Blueprint Course** to create a new Private Blueprint Course.
  - Evidence (source): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` accepts one exact Public or Archived source Revision and delegates the actor-owned Private child to the lineage Store.
  - Evidence (runtime): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` is exercised by accepted actual HTTP evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json` and compiled-main browser evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json` that create and open a Private fork.

- [x] A fork is owned by the **Instructor** who created it.
  - Evidence (source): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` derives the actor from the attested session rather than accepting an owner or availability from the client.
  - Evidence (runtime): `crates/server/src/blueprint_course/fork.rs` `fork_blueprint` is exercised by accepted browser fixture state at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-fixture-state.json`, which records the created Private fork; the actual HTTP lineage receipt rejects concealed Private intermediates for other Instructors.

- [x] A fork records the source Blueprint Course and Blueprint Revision from which it was created.
  - Evidence (source): `crates/server/src/blueprint_course/fork.rs` `BlueprintForkSource` carries the source reference and Revision to the Store.

- [x] Forking a Blueprint Course creates new Blueprint Assessments.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.fork_blueprint_course` allocates the forked tree from the exact source Revision.
  - Evidence (runtime): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.fork_blueprint_course` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying fresh Assessment and Pool IDs with the same ordered Question Revision membership.

- [x] Published Questions in the new Blueprint Assessments retain the same Published Question IDs and exact Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.fork_blueprint_course` allocates the forked tree from the exact source Revision.
  - Evidence (runtime): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.fork_blueprint_course` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying fresh Assessment and Pool IDs with the same ordered Question Revision membership.

- [x] Question Pools in the new Blueprint Assessments are forked and receive new Question Pool IDs.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.fork_blueprint_course` allocates the forked tree from the exact source Revision.
  - Evidence (runtime): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.fork_blueprint_course` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying fresh Assessment and Pool IDs with the same ordered Question Revision membership.

- [x] Forked Question Pools initially contain the same Published Question IDs and exact Revisions as their source.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `inventory_question_ids` derives comparison relationships from the exact fixed and Pool member Question IDs.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `inventory_question_ids` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-owned-pool-http-proof.json`, verifying forked Pools retain exact ordered Question Revision membership under fresh Pool IDs.

- [x] Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.fork_blueprint_course` creates a separately editable fork tree.

- [x] Changes to a source Blueprint Course are never automatically applied to its forks.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_apply.rs` `apply_blueprint_fork` changes only explicit selections.

- [x] A Blueprint Course shows its known forks and the **Instructor** who owns each fork.
  - Evidence (runtime): accepted C881 actual-server proof at `/private/tmp/ple-fork-reader-artifacts.nRikDO` exercises `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks`, returns each fork's owner and recorded origin, and conceals unrelated Instructors with `404 no-store`.
  - Evidence (source): `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks` and `src/features/blueprint_forks/blueprint_fork_review.tsx` `BlueprintKnownForks` present authorized known-fork rows and owner names.

- [x] PLE should make newer source Revisions easy for the fork owner to discover and review.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_review.tsx` `BlueprintKnownForks` presents the source/fork review entry.

- [x] PLE should make newer Revisions in downstream forks visible from their source Blueprint Course.
  - Evidence (source): `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks` returns each visible child fork's current Revision for the source view.
  - Evidence (runtime): `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks` is exercised by accepted compiled-main browser evidence at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json`, which loads the source known-forks row before opening the pair review.

- [x] The fork owner decides whether to incorporate source changes into the fork.
  - Evidence (source): `crates/server/src/blueprint_course/fork_apply.rs` `apply_fork_update` passes explicit selected destinations and both source/fork Revision and metadata preconditions to the Store.
  - Evidence (runtime): `crates/server/src/blueprint_course/fork_apply.rs` `apply_fork_update` is exercised by accepted 84-request actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.vs0NCo/blueprint-local-id-apply-http-proof.json`, verifying owner selection, four denied preconditions, and zero-write denials.

- [x] PLE should make it easy for the fork owner to incorporate selected source changes.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_apply.tsx` `BlueprintForkApply` presents selected current-pair Apply choices.
  - Evidence (runtime): `src/features/blueprint_forks/blueprint_fork_apply.tsx` `BlueprintForkApply` is exercised by accepted compiled-main browser proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.wVCQ4m/comparison-browser.json`, completing selected Apply at desktop and narrow viewports.

#### Blueprint Course Change Proposal specifications
- [x] A **Blueprint Course Change Proposal** proposes changes from one Blueprint Course to another.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_workspace.tsx` `ProposalTargetTools` submits one saved source Blueprint to a different target Blueprint.

- [x] An **Instructor** can create a Change Proposal for a Blueprint Course they do not own.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `create_blueprint_change_proposal` rejects a proposal whose target owner is the acting Instructor.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_workspace.tsx` `ProposalTargetTools` offers the proposal action only when the reader is not the Blueprint owner.

- [x] A Change Proposal records the source Blueprint Course and exact Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `create_blueprint_change_proposal` stores the source Blueprint Course id and exact source Revision number.

- [x] A Change Proposal records the target Blueprint Course and exact Blueprint Revision used for comparison.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `create_blueprint_change_proposal` stores the target Blueprint Course id and the exact Revision used for comparison.

- [x] The proposed changes are represented using the canonical Blueprint Course JSON format.
  - Evidence (source): `crates/learning-data-access/src/blueprint_change_proposal.rs` `proposed_json` stores the proposed Revision as `CanonicalBlueprintCourse`.

- [x] PLE compares the proposed JSON with the target Blueprint Revision to determine the proposed changes.
  - Evidence (source): `crates/server/src/blueprint_course/change_proposal_view.rs` `compare_blueprint_courses` compares the pinned source and target Revisions.

- [x] A Change Proposal should present those changes in a human-readable interface rather than requiring
  the receiving **Instructor** to review raw JSON.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `ProposalReview` shows Question identifiers, Revision numbers, titles, and point values.

- [x] A Change Proposal may include any Blueprint Course content represented in its canonical JSON.
  - Evidence (source): `crates/question_model/src/blueprint_course/canonical_exchange.rs` `export` projects names, classification, modules, and complete Assessment content.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `ProposalReview` offers entire acceptance of that complete structure.

- [x] Changes may include Course names and metadata, Assessment names and settings, Assessment additions
  and removals, and Question membership changes.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_apply.tsx` `BlueprintSelectionEditor` offers source names, complete Assessment content, a new Assessment copy, and removal from the destination.

- [x] Question content changes belong to the Published Question and are not Blueprint Course changes.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` compares Question identifiers and does not read Question bodies or answers.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `ProposalReview` states that Question bodies and answers are not exposed.

- [x] PLE should present proposed changes in terms meaningful to Instructors rather than as raw JSON changes.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `ProposalReview` names the shared Question, the source-only Question, and the Assessment type in Instructor language.

- [x] The receiving **Instructor** can review proposed changes before changing the target Blueprint Course.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_workspace.tsx` `ChangeProposalDetailLivePage` loads the frozen proposal for review before acceptance.

- [x] The receiving Instructor decides which proposed changes to accept.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `ProposalReview` lets the receiving owner choose selected changes or the entire proposal before acceptance.

- [x] The receiving Instructor may accept the entire Change Proposal or selected proposed changes.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `finalize_blueprint_change_proposal_acceptance` accepts an entire decision or a selected decision.

- [x] Accepted changes are applied to the current target Blueprint Course and create a new Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `finalize_blueprint_change_proposal_acceptance` requires the accepted result to be the next target Revision.

- [x] The Change Proposal remains a record of what was proposed and what was accepted.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `reject_blueprint_proposal_evidence_change` keeps proposal evidence immutable.

- [x] If the target Blueprint Course changes after the proposal was created, PLE should show that the
  proposal was based on an older target Revision.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `ProposalReview` states that the frozen proposal used an older target basis.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `read_blueprint_change_proposal` marks the proposal stale when the target head no longer matches the pinned Revision.

- [x] PLE should not silently apply a proposal against a newer target Revision when the changes no longer
  apply cleanly.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `lock_blueprint_change_proposal_acceptance` refuses acceptance when the target Revision or edit number moved.

- [x] Change Proposals never directly change daughter Course Instances.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_change_proposals.sql` `finalize_blueprint_change_proposal_acceptance` saves the target Blueprint without the ordinary auto-daughter overload.

- [x] Daughter Course Instances receive accepted changes through the normal Blueprint incorporation workflow.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseBlueprintUpdateReviewList` offers a newer Blueprint Revision to the daughter Course.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `applyBlueprintUpdate` applies the reviewed Blueprint Revision to the daughter Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_blueprint_updates.sql` `apply_assessment_blueprint_update` keeps the Assessment's existing available, due, and close times.

#### Blueprint Course comparison specifications
- [x] Any **Instructor** can compare related Blueprint Courses in the same fork lineage when both are visible to that Instructor.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.load_blueprint_comparison_sources` authorizes an arbitrary related visible current pair before it is projected.
  - Evidence (runtime): `schemas/base_schema/50_functions/blueprint_lineage.sql` `ple_api.load_blueprint_comparison_sources` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`, covering visible sibling and transitive pairs in both orientations while concealing Private intermediates and denying unrelated pairs.

- [x] Fork comparison normally compares the newest Revision of the source Blueprint Course with the newest Revision of the fork.
  - Evidence (runtime): `src/api/decoders/blueprint_comparison.ts` `decodeBlueprintComparisonView` is exercised by accepted current-pair HTTP evidence, returning current source and fork names, ETags, and Revisions.
  - Evidence (source): `src/api/decoders/blueprint_comparison.ts` `decodeBlueprintComparisonView` requires the current source and fork Revision Tuples.

- [x] Older Revisions remain available through Blueprint history but are not the normal comparison workflow.
  - Evidence (runtime): `src/features/blueprint_course/blueprint_history.tsx` `BlueprintHistory` inspects exact older Revisions separately from current-head comparison. Accepted bounded proof is `/private/tmp/ple-blueprint-owned-pool-artifacts.sEJZUB/history-proof.json`.

- [x] Blueprint Course differences are calculated from canonical JSON when the Instructor requests the comparison.
  - Evidence (source): `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` requests C880's canonical comparison projection on demand rather than persisting comparison state.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` compares canonical Blueprint snapshots.
  - Evidence (runtime): C882's actual HTTP receipt exercises `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` as a real GET-only `no-store` review with zero `ple_data` mutations.

- [x] Shared Published Question IDs provide durable relationships between Published Questions across Blueprint Course forks.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` derives relationships from shared Question IDs only.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`, verifying Rev2-versus-Rev1 shared Question-ID relationships.

- [x] Blueprint Course comparison does not require Blueprint Assessment identity or history across forks.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `BlueprintComparisonAssessment` retains only side-local references while relationships carry shared Question IDs.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `BlueprintComparisonAssessment` is exercised by the accepted browser fixture at `/private/tmp/ple-blueprint-comparison-ui-proof/fixture.py`, which verifies source and fork Assessment IDs are disjoint before comparison and Apply.

- [x] Comparison should show shared, added, removed, and changed Assessments, Published Questions, and Question Pools.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_model.ts` `assessmentComparison` classifies a related Assessment as shared when its canonical fields match and as changed when they differ, and classifies a side-only Assessment as added or removed.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_model.ts` `publishedQuestionComparison` classifies Question IDs as shared, added, or removed, and a shared ID with a different Revision pin as changed.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_model.ts` `questionPoolComparison` classifies Question Pool IDs as shared, added, or removed, and the same Pool ID with different canonical entry content as changed.
  - Evidence (source): `src/features/blueprint_forks/blueprint_comparison_membership.tsx` `ComparisonMembershipSummary` shows those shared, added, removed, and changed rows on the Blueprint comparison.
  - Evidence (test): `tests/test_blueprint_course_model.mjs` `comparison shows shared, added, removed, and changed Assessments, Published Questions, and Question Pools` rendered Stable lab as shared, Added quiz as added, Removed quiz as removed, and Shared quiz as changed. It rendered CHANGED-Q and STABLE-Q as shared Published Questions, ADDED-Q as added, REMOVED-Q as removed, and CHANGED-Q as a changed Revision pin. It rendered STABLE-POOL as shared, ADDED-POOL as added, REMOVED-POOL as removed, and CHANGED-POOL as changed. No Live Demo stack was started.

- [x] Comparison should remain useful when Assessment names, order, or structure have changed.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` uses shared Question IDs instead of Assessment names, positions, or cross-Blueprint Assessment identity.
  - Evidence (runtime): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` is exercised by accepted actual HTTP proof at `/private/tmp/ple-blueprint-owned-pool-artifacts.pWOqCs/blueprint-lineage-pair-http-proof.json`, covering renamed, reordered, and split canonical content.

- [x] Comparison visibility follows Blueprint Course visibility rather than fork ownership.
  - Evidence (source): `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` uses ordinary Blueprint visibility for read-only direct-source review.
  - Evidence (runtime): C881/C882 accepted `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` ordinary-visibility direct-source review at `/private/tmp/ple-fork-reader-artifacts.nRikDO` and `/private/tmp/ple-fork-review-http-artifacts.LTUgsF` permits visible Public/Archived sides and conceals unauthorized Private sides; ownership restricts Apply, not comparison.

#### Blueprint Course JSON specifications
- [x] Blueprint Courses have a canonical JSON representation for comparison, import, export, and exchange.
  - Evidence (source): `crates/question_model/src/blueprint_course/canonical_exchange.rs` `CanonicalBlueprintCourse` defines the strict authority-free projection used by comparison and the authorized server export/import routes; the Instructor UI exposes download and validated import over those routes.

- [x] Canonical Blueprint JSON must contain enough information to fully recreate a Blueprint Course.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` exported a seeded Blueprint, imported it as a distinct actor-owned Private root at Revision 1, and deeply compared the imported re-export with the original canonical object.

- [x] Importing exported Blueprint JSON should reproduce the same Blueprint Course content and structure.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` preserved ordered modules, Assessments, entries, exact reusable references, and metadata while the source Blueprint remained unchanged.

- [x] Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
  - Evidence (source): `crates/question_model/src/blueprint_course/canonical_exchange.rs` `CanonicalBlueprintCourse` contains strict metadata and ordered module/Assessment/entry arrays; focused domain and client contracts reject unknown or malformed shapes.

- [x] Blueprint Assessments contain only reusable teaching settings.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `ple_data.blueprint_content_is_closed` allowlists reusable Assessment content and defaults without Course delivery dates or release state.

- [x] Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Evidence (source): `crates/question_model/src/blueprint_course/assessment_content.rs` `BlueprintAssessmentEntryInput` stores each Fixed Published Question or Question Pool in vector order.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `validate_blueprint_question_selection` rejects a new pin unless that Published Question is available.
  - Evidence (source): `src/features/question_pool_picker/question_pool_picker.tsx` `QuestionPoolPicker` selects one published Question Pool and previews its membership.
  - Owner: 08_courses.md / Blueprint Course JSON specifications (first occurrence; identical requirement and status).

- [x] Blueprint Assessments have no deadlines, release dates, Student data, or other Course Instance settings.
  - Evidence (source): `crates/question_model/src/blueprint_course/assessment_content.rs` `BlueprintAssessmentContentInput` stores the type, title, instructions, ordered entries, and reusable defaults, and denies unknown fields.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `blueprint_revision_assessment` records the Assessment identity and position only.

- [x] Blueprint Revisions can be compared through their canonical JSON representations.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` compares two saved Revision contents and records each Assessment in its canonical form.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_model.ts` `assessmentDifferenceLabels` reports which canonical fields differ.
  - Evidence (source): `src/features/blueprint_forks/blueprint_fork_review.tsx` `BlueprintForkReview` shows the latest saved Revision on each side.

- [x] Blueprint Course Change Proposals use canonical JSON to identify changes between Blueprint Revisions.
  - Evidence (source): `crates/learning-data-access/src/blueprint_change_proposal.rs` `proposed_json` stores the source Revision as canonical JSON, and `target_comparison_json` stores the pinned target Revision in the same form.
  - Evidence (source): `crates/server/src/blueprint_course/change_proposal_view.rs` `compare_blueprint_courses` identifies shared and one-sided Questions between those pinned Revisions and records each Assessment in canonical form.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_workspace.tsx` `ChangeProposalDetailLivePage` loads that frozen record for review.
  - Evidence (source): `src/features/blueprint_change_proposal/proposal_review.tsx` `ProposalReview` shows the frozen Revisions and the shared, source-only, and target-only Questions without raw JSON.

- N/A Canonical Blueprint JSON may support offline inspection or editing, even if it is not optimized for hand editing.
  - Reason: This explicitly optional future capability does not require implemented behavior.

- [x] Canonical Blueprint JSON is the complete exchange format, not the primary persistence model.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` proves authorized export/import/re-export semantic equality with fresh Blueprint, module, Assessment, and Pool identities while relational persistence remains authoritative.

### Course Instance specifications
#### Course Instance creation specifications
- [x] An **Instructor** can create a Course Instance from a Public Blueprint Course.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires a Public Blueprint at the selected exact Revision; `src/pages/course_list_page.tsx` `TeachingCourseListPage` exposes the Adopted source only after public Blueprint discovery.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised in private actual-HTTP and exact-main browser proof: an Instructor created and published a Blueprint through its API, selected its exact Public Revision, created a daughter Course Instance, and read its Unreleased Practice Assessment with finite Attempt limit and dates unset. This does not establish Pool copying or release/delivery workflows.

- [x] **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` defaults to Empty, activates Blueprint discovery only for Adopted, and sends the strict `source: { kind: "empty" }` wire through `src/api/http_client/course_instance.ts`.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised in a bounded authenticated actual-main browser and HTTP proof: an Instructor created an Empty Course Instance, then the resulting row and persisted Course read were observed, with zero Blueprint-list requests and `no-store` responses. Student creation denial was exercised at the HTTP boundary.

- [x] Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
  - Evidence (source): `schemas/base_schema/20_tables/course_membership.sql` `course_membership` stores a Student on a Course Instance when the membership role is student.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_policy_snapshot` stores available_at, due_at, and closes_at for a Course Assessment.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_status` stores the Assessment release state on the Course Instance Assessment.
  - Evidence (test): `tests/test_live_assignment_release_validation.mjs` `Course Instance delivery keeps Students, deadlines, release, and attempt settings` decoded a released Assessment with those deadlines and an attempt limit, and read an active Student from the Course roster.

- [x] Course Instances contain only **Published Questions** and published **Question Pools**.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `create_question_pool` creates an original published Pool only from available Published Questions, and refuses an archived Question.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `replace_assessment_entries` refuses a new pin that is not an Available Question and refuses a Pool that is not already the Assessment's immutable fork.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork_for_ids` stores a fork of the published Pool on the Course Assessment. The Assessment stores that fork, not the published Pool id.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `course_instance_contains_only_published_questions_and_published_pools` pinned available Question SVR1-4XYZ, refused archived Question SVQ1-7ABC, refused attaching published Pool SVP1-FABC, and stored fork SVF1-8ABC of that Pool on unreleased Assessment ASVR0002E.
  - Evidence (test): `tests/e2e/e2e_assessment_saved_response.sh` `course_instance_contains_only_published_questions_and_published_pools` ran that oracle on PostgreSQL.

- [x] Course Instances are visible only to their co-**Instructors** and enrolled **Students**.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `resolve_course_navigation` permits only an active Course Member.

- [x] Active Courses are current teaching Course Instances.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns the Instructor's Course and its lifecycle state.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `mark_course_instance_inactive` marks that Course Inactive at its Active cutoff.
  - Evidence (source): `src/pages/course_list_page.tsx` `coursesForMode` shows a Course only in the list for its lifecycle state.
  - Evidence (test): `crates/learning-data-access/tests/course_instance_postgres.rs` `inactive_course_keeps_metadata_after_student_data_deletion` listed PAST-1 as active for its Instructor, then the retention executor marked it inactive at the Active cutoff.

- [x] Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
  - Evidence (source): `schemas/base_schema/50_functions/course_retention_transitions.sql` `delete_course_student_records` removes the Course's Student records and Student memberships and leaves the Course row.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `list_course_instances` returns that Course's name after the deletion.
  - Evidence (test): `crates/learning-data-access/tests/course_instance_postgres.rs` `inactive_course_keeps_metadata_after_student_data_deletion` archived and deleted the Student record, left zero Student records and zero Student memberships, kept short name PAST-1, long name Past teaching Course, the term, the discipline, and one Instructor membership, and listed the Inactive Course.

- [x] An **Instructor** may create a new **Blueprint Course** from an existing Course Instance's reusable structure. The new Blueprint Course records that Course Instance as its source.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseInstancePage` offers the compact metadata-only Course tools action; its strict client retains source-Course identity and no delivery fields; `schemas/base_schema/50_functions/course_blueprint_publication.sql` `ple_api.create_blueprint_from_course_instance` records the immutable source without changing the Course.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` passed source preservation, first-Adoption counts, exact pins/Pool fork, replay, and stale rollback on PostgreSQL.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` passed canonical HTTPS C420 browser proof: one child-route POST created actor-owned Private Revision-1 Blueprint `BPJD8H28` from Course `CI0QR41X`; the source remained addressable and unchanged.

- [x] A new academic term uses a new Course Instance. Rollover is not a separate product model.
  - Evidence (source): `crates/question_model/src/course_term.rs` `CourseTerm` is input to each `CreateCourseInstanceInput`; no rollover model was found.

#### Blueprint adoption and daughter Course Instances
- [x] **Adoption** connects a Blueprint Course and a Course Instance through either Course creation workflow.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseInstancePage` calls C419 and explains that the unchanged source becomes the first Adoption.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` observed the immutable source relationship and Adoption count without creating a daughter relationship.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` passed canonical HTTPS C420 browser proof displaying Course `CI0QR41X` as Private Revision-1 Blueprint `BPJD8H28`'s first Adoption.

- [x] Creating a new Course Instance from a Blueprint Course establishes an Adoption and increases that Blueprint Course's **Adoption count** by one.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `create_course_instance` stores the source Blueprint on the new Course Instance.
  - Evidence (source): `schemas/base_schema/50_functions/blueprint_operations.sql` `total_adoptions` counts each Course Instance that records that Blueprint.

- [x] Creating a new Blueprint Course from an existing Course Instance's reusable structure establishes the originating Course Instance as that Blueprint Course's first Adoption, giving the new Blueprint Course an Adoption count of one.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CourseInstancePage` states the first-Adoption result and opens only the new Blueprint receipt after C419 accepts creation.
  - Evidence (runtime): `crates/learning-data-access/tests/blueprint_course_postgres/exchange.rs` `assert_actual_role_round_trip` observed the recorded source, `total_adoptions = 1`, distinct-student count, and denied Public-to-Private rollback.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` passed canonical HTTPS C420 browser proof displaying Adoption count 1 for actor-owned Private Revision-1 Blueprint `BPJD8H28`, created from Course `CI0QR41X`.

- [x] A Course Instance created from a Blueprint Course is a **daughter Course Instance** of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/50_functions/course_core.sql` `course_instance` records Blueprint reference and Revision source columns.

- [x] A daughter Course Instance records its parent Blueprint Course and the exact Blueprint Revision used to create it.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` includes `blueprint_course` and `blueprint_revision`.

- [x] A daughter Course Instance receives every Assessment from the selected Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` constructs the Course Assessments from selected Blueprint content.

- [x] Creating a daughter Course Instance copies the Blueprint Course's Assessments, Questions, Question Pools, and reusable settings.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies the source Pool members into the Course fork.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` checks those Question pins and Pool member copies.

- [x] Course Instance Assessments created from Blueprint Assessments start unreleased with dates unset.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` initializes adopted Assessments as unreleased with delivery dates unset.

- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

- [x] Routine Blueprint changes should be quick for an **Instructor** to review and incorporate.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

- [x] It should be obvious when a daughter Course Instance is based on an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/50_functions/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` use the authorized parent origin and exact adopted/current Revision projection for the same visible notice.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was covered by independently accepted actual-server/exact-main proof across empty, current, newer, and explicit synthetic Private-origin states; the visually inspected newer capture showed both Revision values and the stale notice. It preserved the original adoption pin, Assessment, and entries; its Work tables were empty, so this proof makes no populated-Student-Work claim. Unauthorized Student and unrelated-Instructor reads returned `404 no-store`. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This duplicate course-view indication does not implement the separate Blueprint update offer, review, approval, or apply workflow.

- [x] The **Instructor** decides which changes to existing Assessments to incorporate.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` offers each existing Assessment for separate review and does not apply the Course's updates together.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `applyBlueprintUpdate` applies only the Assessment the Instructor reviewed, using that source Revision and Edit Number.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_blueprint_updates.sql` `apply_assessment_blueprint_update` updates that one Assessment and leaves the other Course Assessments unchanged.
  - Evidence (test): `tests/test_assignment_client.mjs` `The Instructor decides which changes to existing Assessments to incorporate` posted one Assessment update and withheld a request that added another Assessment.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (test): `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `newly_added_blueprint_assessments_are_the_daughter_append` keeps only the new Assessment and leaves release status and delivery dates off the daughter copy payload.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `append_course_assessments` inserts that Assessment without a release status or delivery dates.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_status` defaults to unreleased.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_append_projection` requires the appended daughter Assessment to be unreleased with null available, due, and close times.
  - Owner: earlier Human Guidance occurrence of this bullet.

- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` validates the new-reference delta and inserts only new Assessments; it does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` changes a retained source Assessment title and proves existing daughter Assessment content, entries, and actual Student Work unchanged through Save/replay/no-op/stale operations. Accepted artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: This negative invariant remains separate from the verified Course-review workflow; it does not claim direct-Assessments or all Student Work.
  - Owner: 08_courses.md / Blueprint adoption and incorporation specifications (first occurrence; identical requirement and status).

### Course short and long name specifications
- [x] Blueprint Courses and Course Instances each have their own short name and long name.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `CreateBlueprintCourseInput` and `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` each own both names.

- [x] Short names are entered or chosen deliberately by **Instructors**.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `pub short_name: String` is a submitted validated field.

- [x] Short names are for compact navigation and should stay under about 16 characters when practical.
  - Evidence (source): `src/pages/course_list_page.tsx` `For compact navigation; about 16 characters when practical.` is the Course Instance creation guidance.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `blueprint-course-create-short-name-help` is the Blueprint creation guidance.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_detail_structure.tsx` `blueprint-course-detail-short-name-help` is the Blueprint name editing guidance.
  - Evidence (source): `src/pages/course_instance_page.tsx` `CompactShortNameField` is the Create Blueprint from Course Instance guidance and keeps maxlength 200.
  - Evidence (test): `tests/test_frontend_contract.mjs` `the Blueprint short name field states the compact-navigation guidance` rendered that sentence and kept a longer name.

- [x] Long names are descriptive names used for headings, breadcrumbs, and Course listings.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `pub long_name: String` is documented as the heading and breadcrumb name.

- N/A A Blueprint Course might be `Biochemistry` / `Upper-Level Introductory Biochemistry`.
  - Reason: This is an illustrative name example, not an implementation requirement.

- N/A A Course Instance might be `BCHM 355/455` / `BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)`.
  - Reason: This is an illustrative name example, not an implementation requirement.

- [x] Course Instance names are properties of the Course Instance and are not derived from Blueprint Course names.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` requires independently supplied `short_name` and `long_name`.
## Assessment specifications
- [x] **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `createLiveAssessment` creates one Course Assessment for the selected Type. `crates/question_model/src/assessment.rs` `AssessmentType` has no separate Assignment object.

- [x] PLE has **Blueprint Assessments** and **Course Instance Assessments**.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` is the reusable Blueprint Assessment aggregate; `schemas/base_schema/50_functions/assessments.sql` `ple_data.assessment` is the current Course Instance Assessment aggregate with a required `course_id`.

- [x] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains Type, title, instructions, ordered entries, and validated `BlueprintAssessmentDefaults`; `BlueprintCourseModuleContent` owns those Assessments in Blueprint Course content.
  - Owner: 09_assessments.md / Assessment specifications (first occurrence; identical requirement and status).

- [x] Course Instance Assessments deliver Questions to **Students**.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AssessmentAttemptPage` loads the Student Attempt and calls `getStudentAssessmentAttemptPresentation` for the current Question.
  - Evidence (test): `tests/playwright/test_assessment_attempt_delivery.mjs` `mountAssessmentAttemptDelivery` shows Membrane review and the prompt Which membrane lipid forms the bilayer? for Attempt 11111111-1111-4111-8111-666666666666. The presentation SQL was not executed against PostgreSQL in this pass.

- [x] All Assessments use the same underlying Assessment model.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` and `crates/learning-data-access/src/assessment_release.rs` `LiveAssessmentWorkspace` share canonical Assessment Type, title, instructions, activity rules, and Student feedback rules. `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `assessment_input` materializes `SaveLiveAssessmentInput` through `assessment_values_json` into ordinary `ple_data.assessment` rows; distinct reusable and delivery storage/lifecycle projections are not separate pedagogical models.
  - Evidence (runtime): accepted independent fresh PostgreSQL connected `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` passed 1 test with 0 ignored. Its `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` verified preserved Assessment Type, mixed ordered Pool/Fixed entries, nondefault points/scoring/retry/timing/activity/feedback rules, exact Revision pins, fresh independent daughter Pool IDs, and unset delivery dates. Supplemental read-only SQL verified two adopted Regular Assignments retained the source Type and were Unreleased with null dates. Artifact: `/private/tmp/ple-shared-assessment-adoption-artifacts.IkYuXY`. This architecture receipt does not establish every Type's Student delivery or completion.

- [x] **Assignment** is not a separate object or category. The word appears only in the names
  **Weekly Assignment**, **Unit Review Assignment**, and **Bonus Assignment**.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` is the browser source of the visible word Assignment, and only inside those three labels.

### Assessment content specifications
- [x] Assessments are organized by their Course and position within its ordered sequence.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `list_course_assessments` returns one Course Instance's Assessments ordered by due date, then Assessment ID.
  - Evidence (source): `src/pages/course_instance_page.tsx` `assessmentRows` numbers that Course sequence as Assessment 1, Assessment 2, and onward.

- [x] Assessments contain an ordered sequence of Published Questions and Question Pools.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` renders the mixed entry sequence; `schemas/base_schema/50_functions/assessments.sql` `assessment_entry_active_authored_position_key` enforces distinct current positions with a deferred constraint, allowing atomic swaps and retired-position reuse. `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.load_assessment_workspace_rows` projects only available current entries.
  - Evidence (runtime): accepted independent actual-server/private bundled-main browser proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` saved Fixed A, an imported Pool, and Fixed B, moved the top-level Pool across Fixed A, and reloaded exact ordered entry IDs and Revision pins. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`.

- [x] Each Published Question in an Assessment is identified by its Question ID and exact Revision.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_entry_content.ts` `blueprintAssessmentEntryContent` names each fixed Question by its Question ID and Revision number.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_selected_entry.ts` `selectedAssessmentEntryContent` names each Course Instance Question by its Question ID and Revision number.

- [x] Question Pools are copied by forking when added to another Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `import_assessment_question_pool_fork` creates the Assessment entry and calls `fork_question_pool` for a new Pool id.

- [x] A newly forked Question Pool initially contains the same Published Question IDs and exact Revisions
  as its source.
  - Evidence (source): `schemas/base_schema/50_functions/question_pools.sql` `construct_question_pool_fork` copies each source member's published_question_id and question_revision_number into the new Pool.

- [x] A forked Question Pool can be changed independently without changing its source Question Pool.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_pool_forks.sql` `append_assessment_question_pool_fork_members` saves members with `save_question_pool_members` on the fork Pool id, and only when that Pool has a source Pool.

- [x] Published Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` branches on `fixedQuestion` and `questionPool`; `schemas/base_schema/50_functions/assessments.sql` retains separate entry-kind storage and Assessment-owned Pool identity within one authored-position sequence.
  - Evidence (runtime): accepted mixed-entry proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` reloaded distinct Fixed Question and Pool entries with exact pins, then retained the retired Pool's old Revision while a reimport received a distinct fork entry ID and Pool ID. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. Existing connected adoption regression also passed 1 test with 0 ignored under the corrected SQL, with two supplemental Type projections retaining unchanged pins: `/private/tmp/ple-shared-assessment-adoption-artifacts.UmP416`.

- [x] **Instructors** can add, remove, and reorder Published Questions and Question Pools.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` supplies Question addition, Pool import, and mixed-entry move/remove controls; `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` reaches `schemas/base_schema/50_functions/assessments.sql` `ple_data.replace_assessment_entries` through authorized Instructor persistence.
  - Evidence (runtime): accepted actual-server/private bundled-main browser proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` added both kinds, reordered the Pool across a Fixed Question, removed Fixed B and then the Pool, and saved/reloaded their absence from current content. Private retired IDs and the old Pool Revision remained; source Pool JSON was unchanged, and reimport created distinct fork entry/Pool IDs. Student direct real HTTP returned 404 without current-state change; browser error arrays were empty. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. This does not establish Student Work history, full Live Demo, authentication/TLS, or WeBWorK delivery.

- [x] Assessment Question-order randomization is called **Randomize question order**.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` renders the exact accessible checkbox label **Randomize question order**.
  - Evidence (runtime): the earlier accepted part 04 actual-main/HTTP receipt saved and reloaded this visible checkbox through `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`; this is separate from the mixed-entry artifact. C64 owns persisted Question-order behavior. This label receipt does not claim C505's planned filename rename or whole-milestone completion.

### Assessment type specifications
- [x] PLE defines the available Assessment Types.
  - Evidence (source): `crates/question_model/src/assessment.rs` `AssessmentType` is the canonical closed model, and `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` carries it to the browser contract.

- [x] Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states the five purposes. `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults` leaves the Attempt limit unset for Weekly Assignment, Unit Review Assignment, and Bonus Assignment, and sets one Attempt for Quiz and Exam. `crates/question_model/src/assessment_activity_rules.rs` `for_assessment_type` releases a Unit Review correct answer after submit.
  - Evidence (test): `crates/question_model/src/assessment_template.rs` `regular_template_defaults_to_unlimited_attempts` passed. `crates/question_model/src/assessment_template.rs` `quiz_and_exam_templates_permit_exactly_one_attempt` passed. `crates/question_model/src/assessment_activity_rules.rs` `practice_defaults_release_the_answer_without_an_explanation` passed. This pass does not claim a collaboration control, learning-age inference, or an Instructor exam calendar. create_assessment was not executed against PostgreSQL in this pass.

- [x] Assessment Types are **Weekly Assignment**, **Unit Review Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps the five canonical values to those labels.
  - Evidence (source): `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` is the closed browser set those labels cover.

- [x] **Instructors** select an Assessment Type but cannot create new Assessment Types.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` requires one canonical Type before creation; `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog` does the same for reusable content, with no free-form Type path or silent default.
  - Evidence (test): `tests/test_blueprint_course_ui.mjs` `createdContent` proves the selected canonical Type reaches creation; the focused Blueprint UI/model lane passed 10/10.

- [x] Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `assessment_type` and `schemas/base_schema/50_functions/assessments.sql` `assessment_type` validate the same five values; `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `assessment_type` copies the selected Type during adoption.

- [x] **Instructors** can change Assessment settings independently of the defaults for its Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` resolves explicit Course Instance property overrides separately from Blueprint defaults and Assessment Type.
  - Evidence (runtime): accepted fresh PostgreSQL actual-Store receipt loaded an Instructor Exam, saved its policy settings, and retained its Type through `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore`.

- [x] Changing Assessment settings does not change its Assessment Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` does not expose Type as an editable property, while `crates/question_model/src/assessment.rs` `AssessmentType` remains part of Assessment identity.
  - Evidence (runtime): accepted fresh PostgreSQL actual-Store receipt saved Instructor Exam policy settings while retaining its Type through `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore`.

- [x] **Weekly Assignments** give **Students** regular practice applying course ideas outside class.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states practice applying course ideas outside class.

- [x] Weekly Assignments reinforce current learning and may also introduce new topics.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states that a Weekly Assignment reinforces current learning and can introduce new topics.

- [x] Weekly Assignments are designed as practice for learning, not merely as one-time assessments.
  - Evidence (source): `crates/question_model/src/assessment_template.rs` `for_assessment_type` leaves a Weekly Assignment Attempt limit unset.
  - Evidence (test): `crates/question_model/src/assessment_template.rs` `regular_template_defaults_to_unlimited_attempts` checks that unset limit.

- [x] **Unit Review Assignments** provide focused review or study-guide practice using material already covered.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states focused review or study-guide practice using material already covered.

- [x] Unit Review Assignments may be worth a small number of points or a small amount of extra credit.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `grade_contribution_points_possible` keeps authored points for a Unit Review Assignment unless the entry is extra credit or excluded. `src/features/blueprint_course/blueprint_course_model.ts` `updateReusableEntryScoring` accepts a decimal point value and extra credit.

- [x] Unit Review Assignments use the same whole-Attempt submission boundary as every other
  Assessment and show the correct answer immediately after that Assessment Attempt is submitted.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `finalize_assessment_attempt` finalizes the whole Assessment Attempt and does not branch on Assessment Type. `crates/question_model/src/assessment_activity_rules.rs` `for_assessment_type` releases the Unit Review correct answer after that Attempt is submitted.
  - Evidence (test): `crates/question_model/src/assessment_activity_rules.rs` `practice_defaults_release_the_answer_without_an_explanation` checks that after-submit answer.

- [x] **Bonus Assignments** provide optional extra credit.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states optional extra credit in the Bonus description at both Instructor creation surfaces; `schemas/base_schema/50_functions/grading.sql` `grade_contribution_points_possible` gives Bonus a zero grade denominator without discarding earned points.
  - Evidence (runtime): the separately accepted neighboring Bonus grading receipt returned and rendered `8 / 0` through `schemas/base_schema/50_functions/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments` and the M6 component. Optional is the Instructor-selected purpose, not a new completion/required-work policy field.

- [x] Bonus Assignments are worth zero points possible and add earned points directly to the grade.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `grade_contribution_points_possible` makes Bonus points possible zero while `score_recorded_credit` continues to supply earned points; `schemas/base_schema/50_functions/grading_access.sql` applies that contribution through the real Gradebook helper, and `schemas/base_schema/50_functions/student_assessment_landing.sql` projects the same selected contribution to the Student API.
  - Evidence (runtime): accepted actual PostgreSQL proofs exercised `schemas/base_schema/50_functions/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, returning earned Bonus points with a zero denominator; the Student row was `8 / 0`. The strict Student decoder accepts the complete pair, and compiled M6 component evidence rendered it without changing raw Assessment Attempt scoring.

- [x] **Quizzes** assess understanding of recent material.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states assessment of recent material in the Quiz description at both Instructor creation surfaces. The Instructor selects that purpose and content; PLE does not infer learning age.

- [ ] Quizzes may use more restrictive Attempt and collaboration settings than Weekly Assignments.
  - Reason: product decision still unclear
  - Question: Does this require a distinct collaboration control, or only the more restrictive Quiz Attempt limit?
  - Mismatch: One reading: a Quiz already defaults to one Assessment Attempt while a Weekly Assignment stays unlimited, and Human Guidance names no other collaboration control. The other reading: Quizzes need a separate collaboration setting that can be stricter than a Weekly Assignment.

- [x] **Exams** are individual assessments associated with scheduled exam periods.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states individual assessment associated with a scheduled exam period in the Exam description; `AssessmentWorkspaceCreatePage` and `BlueprintCourseCreateDialog` render it as an Instructor-selected purpose. This does not claim an Instructor exam calendar or infer scheduled periods.

- [x] Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` fixes Exam to one Attempt and exposes editable time limit, Available/Closes schedule, and six feedback-release controls including `never`. This is current UI capability evidence, not acceptance of every control's persistence/runtime enforcement.
  - Evidence (runtime): the separately accepted neighboring one-Attempt receipt covered Exam effective-one handling and expired-pending denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`; it does not expand this settings-capability receipt.

- [x] Quizzes and Exams allow one Assessment Attempt.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_policy_snapshot.sql` `ensure_assessment_policy_snapshot` rejects a Quiz or Exam unless the Assessment Attempt limit is 1.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `blueprint_content_is_closed` rejects Blueprint Quiz or Exam content unless that limit is 1.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `oneAttemptOnly` keeps a Course Instance Quiz or Exam at one Attempt.

### Blueprint Assessment specifications
- [x] A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintCourseModuleContent` contains ordered `BlueprintAssessmentContent`; `schemas/base_schema/50_functions/blueprints.sql` `blueprint_revision_assessment` records each stable Blueprint Assessment member of a Blueprint Course Revision.

- [x] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains Type, title, instructions, ordered entries, and validated `BlueprintAssessmentDefaults`; `BlueprintCourseModuleContent` owns those Assessments in Blueprint Course content.
  - Owner: 09_assessments.md / Assessment specifications (first occurrence; identical requirement and status).

- [x] Blueprint Assessments have an Assessment Type.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `assessment_type` requires the closed five-Type value; `src/api/decoders/blueprint_course.ts` `assessmentType` strictly requires it in both reusable-content input and view decoding without fallback.
  - Evidence (test): `tests/test_blueprint_course_client.mjs` `B1 client sends Revision and metadata validators to their separate routes` covers create/save/view Type round trips plus missing and unknown rejection; the focused Blueprint client lane passed 16/16.

- [x] Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Evidence (source): `crates/question_model/src/blueprint_course/assessment_content.rs` `BlueprintAssessmentEntryInput` stores each Fixed Published Question or Question Pool in vector order.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `validate_blueprint_question_selection` rejects a new pin unless that Published Question is available.
  - Evidence (source): `src/features/question_pool_picker/question_pool_picker.tsx` `QuestionPoolPicker` selects one published Question Pool and previews its membership.
  - Owner: 08_courses.md / Blueprint Course JSON specifications (first occurrence; identical requirement and status).

- [x] Blueprint Assessments define Question point values and points possible.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_model.ts` `updateReusableEntryScoring` stores a fixed Question's points possible and a Question Pool's points per Question, and leaves the content unchanged when the points are not canonical.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `BlueprintAssessmentContentEditor` shows those values as Points possible and Points per Question.
  - Evidence (source): `schemas/base_schema/50_functions/blueprints.sql` `blueprint_content_is_closed` requires points_possible on a fixed entry and points_per_item on a pool entry.

- [x] Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
  - Evidence (source): `crates/question_model/src/blueprint_course/assessment_content.rs` `BlueprintAssessmentContentInput` stores the type, title, instructions, ordered entries, and reusable defaults, and denies unknown fields.
  - Evidence (source): `schemas/base_schema/20_tables/blueprint_course.sql` `blueprint_revision_assessment` records the Assessment identity and position only.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_feedback_fields.tsx` `BlueprintAssessmentFeedbackFields` says due and close dates belong to the daughter Course Instance.

- [x] Blueprint Assessments do not use Assessment Templates.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_templates.sql` `ple_private.assessment_template` defines Templates as Instructor-owned private state outside Courses and Blueprints with no Blueprint or source field.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt covered Template create, save, read, and direct Course Assessment copy without a Blueprint relationship through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
  - Owner: 09_assessments.md / Blueprint Assessment specifications (first occurrence; identical requirement and status).

- [x] Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_instance.rs` `create_course_instance` sends every Blueprint Assessment from `creation_assessments` to `ple_api.create_course_instance`, and `schemas/base_schema/50_functions/course_operations.sql` `initialize_course_assessments` copies that payload into the new Course Instance.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `append_course_assessments` inserts one adopted Assessment for each member, including its Published Questions.
  - Evidence (test): `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `daughter_creation_copies_every_blueprint_assessment` materializes both Blueprint Assessments and each Published Question, with null delivery dates and no release status on the copy payload. `initialize_course_assessments` and `append_course_assessments` were not executed against PostgreSQL in this pass.

### Course Instance Assessment specifications
- [x] A **Course Instance Assessment** is an Assessment in a **Course Instance**.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `ple_data.assessment` requires `course_id` referencing `ple_data.course_instance`; `crates/question_model/src/assessment.rs` `AssessmentOrigin` names direct creation in a Course Instance and adopted creation from an exact Blueprint Assessment.

- [x] Course Instance Assessments are the Assessments delivered to **Students**.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `getLiveAssessmentAccess` loads one Course Instance Assessment for the signed-in Student, and `schemas/base_schema/20_tables/assessment.sql` `course_instance_id` stores that Assessment on its Course Instance.

- [x] Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `assessment_type` adds the closed Type to the existing Assessment content, Pool, and points model; `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `assessment_type` preserves it during adoption.
  - Evidence (source): `crates/learning-data-access/tests/assessment_policies_postgres.rs` `policy_save_is_isolated_conflict_checked_and_reports_unreleased_invalid_dates` supplies the current adopted Quiz fixture: it changes instructions and a 300-second time limit to 600 seconds through a Type-free Properties input.
  - Decision: The prior Quiz Attempt-limit 3-to-5 runtime wording is retired. The current ignored PostgreSQL fixture still needs an actual acceptance rerun; this source evidence does not manufacture one.

- [x] Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` edits Available date, Due date, and Closes date, and releases the Assessment. `src/pages/assessment_workspace/assessment_workspace_live_page.tsx` `AssessmentWorkspaceIdentity` shows the current release status.
  - Evidence (test): `tests/playwright/test_course_instance_delivery.mjs` `a course instance assessment has due date release status and student availability` opens Membrane review as Unreleased, shows Due date 2026-10-15, Available date, Closes date, Release assessment, and Open Student View.

- [x] Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `initialize_course_assessments` creates a fresh Course Assessment with immutable adopted provenance; `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` updates that Course Assessment through its Course-owned reference.
  - Evidence (runtime): accepted C503 PostgreSQL evidence covered adopted Assessment load/save with exact Blueprint Course, Revision, and Assessment provenance retained; `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore` exercised the adopted load/save production mapper. The standalone C503 proof did not rerun the full publisher-backed installation-data seed.

- [x] Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` `save_trusted_content` passes `newly_added_assessments` to `ple_api.save_blueprint_course` for every daughter.
  - Evidence (source): `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `append_new_blueprint_assessments` accepts only Assessments added on the saved Revision and calls `append_course_assessments`.
  - Evidence (source): `schemas/base_schema/20_tables/assessment.sql` `assessment_status` defaults to unreleased, and the adoption insert does not set a release status or delivery dates.
  - Evidence (test): `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `newly_added_blueprint_assessments_are_the_daughter_append` keeps only the new Assessment and leaves release status off that copy payload. `append_new_blueprint_assessments` was not executed against PostgreSQL in this pass.

### Assessment Template specifications
- [x] An **Assessment Template** is a reusable set of settings for creating Course Instance Assessments.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_templates.sql` `ple_private.assessment_template` stores reusable settings, and `schemas/base_schema/50_functions/assessment_template_copy.sql` `ple_api.create_assessment_from_template` makes a direct Course Assessment from them.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt passed the complete Template round trip and by-value Course Assessment copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.

- [x] Assessment Templates are separate from Assessment Types.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_templates.sql` `ple_private.assessment_template` has a private UUID, owner, name, and settings while its required `assessment_type` is one closed Type field.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt passed Template create, save, read, and by-value copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.

- [x] Every Assessment Template has one of the five Assessment Types.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_templates.sql` `ple_private.assessment_template` constrains `assessment_type` to the five canonical values.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt passed Template create, save, read, and by-value copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.

- [x] **Instructors** can create and change their own Assessment Templates.
  - Evidence (source): `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` supplies Instructor CRUD; owner authorization is enforced by the Template Store.
  - Evidence (runtime): accepted C515 actual-Store proof covered owner/nonowner and inactive authorization, stale CAS, and settings round trip through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
  - Evidence (runtime): separate actual-component proof covered browser Template CRUD at `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface`; it is not connected-server evidence.

- [x] Assessment Templates provide defaults for settings such as Attempts, timing, scoring, and disclosure.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_templates.sql` `ple_private.assessment_template` stores instructions, Attempt/time limits, late-work, seven activity rules, and six feedback-release timings including grade rule.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt passed Template settings round trip and copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.

- [x] Creating a Course Instance Assessment from a Template copies its settings into the new Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_template_copy.sql` `ple_api.create_assessment_from_template` reads the owner-visible Template once and passes every portable setting by value to `ple_data.create_assessment_from_template_values`.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence and actual-component UI proofs passed at `schemas/base_schema/50_functions/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt passed Template copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.

- [x] The new Course Instance Assessment can be changed independently after it is created.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_template_copy.sql` `ple_api.create_assessment_from_template` creates a direct Course Assessment with copied values and no Template link.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence proof passed at `schemas/base_schema/50_functions/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.

- [x] Changing an Assessment Template does not change Assessments previously created from it.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_template_copy.sql` `ple_api.create_assessment_from_template` copies settings by value into the new Course Assessment and stores no Template identity.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence proof passed at `schemas/base_schema/50_functions/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.

- [x] Assessment Templates do not contain Questions or Question Pools.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_templates.sql` `ple_private.assessment_template` defines identity, owner, Type, and reusable settings with no Question, Pool, content, point, or source field.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt passed Template creation and empty direct Course Assessment copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.

- [x] Blueprint Assessments do not use Assessment Templates.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_templates.sql` `ple_private.assessment_template` defines Templates as Instructor-owned private state outside Courses and Blueprints with no Blueprint or source field.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt covered Template create, save, read, and direct Course Assessment copy without a Blueprint relationship through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
  - Owner: 09_assessments.md / Blueprint Assessment specifications (first occurrence; identical requirement and status).

### Course Instance Assessment release and defaults
#### Assessment release validation
- [x] Course Instance Assessments start unreleased.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `assessment_status` defaults to `unreleased`; `schemas/base_schema/50_functions/assessment_creation.sql` `ple_data.create_assessment` creates direct rows without overriding it, `schemas/base_schema/50_functions/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` owns the same initial status for adopted rows, and `schemas/base_schema/50_functions/assessment_template_copy.sql` `create_assessment_from_template_values` flows through direct `create_assessment` before copying policy values.
  - Evidence (runtime): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` was exercised in accepted private actual-main/HTTP proof: a newly Empty Course's direct Practice Assessment was Unreleased at creation, before its later validated release. Separate accepted `src/pages/course_list_page.tsx` `TeachingCourseListPage` Public Blueprint adoption produced an Unreleased Practice Assessment at creation. This initial-state fact does not establish Student delivery or all release rules.

- [x] Releasing a Course Instance Assessment requires an automated and interactive **Assessment Release Validation** process.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.validate_assessment_release` projects the trusted release issues only to an authorized Course Instructor; `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` invokes it from the unreleased Assessment Properties workflow.
  - Evidence (runtime): accepted fresh PostgreSQL actual-API receipt proved the authorized validation/release boundary and rollback behavior at `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.validate_assessment_release`; the accepted actual Properties component receipt exercised the interactive readiness flow at `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`.

- [x] Assessment Release Validation checks the Assessment settings and data required for release.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` `assessment_release_issues` checks an available Question or Pool, the 250 delivered-Question limit, Pool selection against Pool membership, and the due-date conditions.

- [x] Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` `assessment_release_issues` emits questions_required, question_count_exceeded, and question_pool_insufficient_items. `crates/learning-data-access/src/postgres/assessment_release.rs` `validate_live_assessment_release` maps those codes to the Instructor issue names. `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` states what to fix for each name.
  - Evidence (test): `tests/playwright/test_course_instance_delivery.mjs` `mountAssessmentReleaseQuestionIssues` shows the missing-Question, 250-Question, and unavailable-Question sentences. The Rust mapper was not executed in this pass.

- [x] Release Validation should require a due date at least 24 hours in the future and no later than the
  Course Instance's six-month Active limit.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` `ple_data.assessment_release_issues` rejects a missing Due date, a Due date less than 24 hours ahead at release, and a Due date after `course_instance.active_until_at`.
  - Evidence (runtime): accepted fresh PostgreSQL actual-API receipt exercised the missing-Due, 24-hour, and Course Active-limit boundaries from `schemas/base_schema/50_functions/assessment_release_validation.sql` `ple_data.assessment_release_issues`.

- [x] Release Validation should check that release, due, and other dates occur in a valid order.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` `ple_data.assessment_release_issues` rejects Available after Due and Due after Closes.
  - Evidence (runtime): accepted fresh PostgreSQL actual-API receipt exercised both invalid orderings and the corrected valid ordering from `schemas/base_schema/50_functions/assessment_release_validation.sql` `ple_data.assessment_release_issues`.

- [x] Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` reaches `schemas/base_schema/50_functions/assessments.sql` `ple_data.replace_assessment_entries`, which rejects point values outside `0` through `1000000000.9999` or four decimal places; table checks retain that bound for alternate writers. The Assessment table permits only null or positive whole-Assessment Attempt/time limits and requires one Attempt for Quiz and Exam.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` `ple_data.assessment_release_issues` separately requires an Assessment Attempt time limit before release.
  - Evidence (runtime): accepted fresh PostgreSQL actual-API receipt for `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` atomically rejected `1000000001` and `1000000000.99999`; exact `1000000000.9999` saved and released.

- [x] Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` `assessment_release_issues` requires an available Question or Pool, rejects more than 250 delivered Questions, and rejects a Pool selection count above that Pool's member count.

- [x] The **Instructor** should be able to correct validation problems and run Release Validation again.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` keeps Assessment Properties editable, provides issue-specific save-and-rerun instructions, and exposes `Check release readiness` again after correction.
  - Evidence (runtime): accepted actual Properties component receipt exercised invalid dates, correction, a second readiness check, and release through `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`.

- [x] An Assessment can be released only after Release Validation passes.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` `ple_data.validate_assessment_release` raises on every issue, and `schemas/base_schema/50_functions/assessment_operations.sql` `ple_data.release_assessment` invokes that hard gate before changing release state.
  - Evidence (runtime): accepted fresh PostgreSQL actual-API receipt proved unauthorized access is denied, invalid release rolls back, and corrected release succeeds through `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.release_assessment`.

- [x] Releasing an Assessment makes it available to **Students** according to its dates and access settings.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `assessment_status_gate` opens only a released Assessment, and `assessment_start_decision` then returns not yet available, may start, late work refused, or closed from the available, due, close, and late-work facts.
  - Evidence (test): `crates/domain/src/effective_assessment_properties/tests.rs` `default_reject_policy_opens_released_work_until_the_due_date` denies an unreleased Assessment, refuses a start before the available time and after close, and allows an on-time start inside the window.

- [x] Student Work begins when a **Student** starts an Assessment Attempt.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `startLiveAssessment` runs only from the Start or Resume action, and `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` inserts the Assessment Attempt at that start.

#### Assessment submission defaults
- [x] New Course Instance Assessments default to accepting submissions only through the due date.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_creation.sql` `create_assessment` stores late work as reject, `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` sets the Attempt expiry to the due time for that rule, and `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `save_student_assessment_attempt_response` returns expired at that expiry. `crates/question_model/src/assessment/teaching_settings_local.rs` `derive_instructor_assessment_availability` closes a released Assessment at the due time when late work is reject.
  - Evidence (test): `crates/domain/src/effective_assessment_properties/tests.rs` `default_reject_policy_opens_released_work_until_the_due_date` keeps the default reject rule, closes availability at the due time, and leaves an accept rule available at that same time. The creation, start, and save SQL was not executed against PostgreSQL in this pass.

- [x] New Course Instance Assessments default to starting new Attempts only through the due date.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `assessment_start_decision` returns late_work_refused after the due time when the rule is reject, and returns may start for accept or mark late.
  - Evidence (test): `crates/domain/src/effective_assessment_properties/tests.rs` `default_reject_policy_opens_released_work_until_the_due_date` refuses a new Attempt after the due time for the default reject rule and still allows an accepted-late start when the rule is accept.

- [x] Late work defaults to rejected.
  - Evidence (source): `crates/question_model/src/assessment.rs` `BaseAssessmentPolicy` defaults late work to reject, `crates/question_model/src/assessment_template.rs` `for_assessment_type` copies that default, and `schemas/base_schema/50_functions/assessment_creation.sql` `create_assessment` stores reject for a new Course Instance Assessment.
  - Evidence (test): `crates/domain/src/effective_assessment_properties/tests.rs` `default_reject_policy_opens_released_work_until_the_due_date` checks the default and every Assessment Type template.

#### Assessment answer and feedback disclosure
- [x] Assessment disclosure settings remain separate and independently configurable.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` gives score, correctness, submitted response, correct answer, answer explanation, and class statistics independent timing fields; `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` exposes those six fields. Question Feedback is shown when provided and has no delayed-release state.
  - Evidence (test): accepted actual-component proof exercised `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog`, switching Type both ways while preserving title, entries, and the resulting Type defaults.

- [x] **Weekly Assignments** and **Bonus Assignments** should rarely show the correct answer.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `for_assessment_type` leaves the correct answer at never for Weekly Assignment and Bonus Assignment. `schemas/base_schema/50_functions/assessment_creation.sql` `create_assessment` stores that same never default unless the Type is Unit Review, Quiz, or Exam. `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults` uses that never timing and still lets the Instructor change it.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `weekly_and_bonus_keep_the_correct_answer_hidden_after_submission` keeps the correct answer hidden after submission while showing whether the response was correct.

- [x] Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `create_assessment` defaults `submitted_response` and `per_item_correctness` to `after_submit`; `crates/server/src/assessment_delivery/history.rs` `project_history` projects those fields independently through the existing server-redacted summary.
  - Evidence (test): accepted actual-component proof exercised `src/pages/assessment_attempt_page.tsx` `submitAttempt`, navigating an accepted whole submission to that summary while preserving existing failure behavior on the Attempt page.

- [x] **Unit Review Assignments** show correct answers immediately after Assessment Attempt
  submission.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `for_assessment_type` releases the Unit Review correct answer after that Attempt is submitted. `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults` uses the same after-submit timing.
  - Evidence (test): `crates/question_model/src/assessment_activity_rules.rs` `practice_defaults_release_the_answer_without_an_explanation` checks that after-submit answer.

- [x] **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_history.sql` `current_student_cohort_completed_assessment` is true only when the Assessment exists and every current Student has a submission. `crates/server/src/assessment_delivery/history.rs` `project_released_content` offers a WeBWorK correct-answer review only when `history_decision` releases the answer. `src/pages/assessment_attempt_summary_page.tsx` `OpaqueWebworkPreviewFrame` shows that review.

- [x] A Quiz or Exam Attempt is complete when the **Student** submits it or its time limit expires and
  PLE submits it automatically.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` inserts one submitted-Attempt record for `student` or `deadline` finalization.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt covered Quiz Student submission, generic deadline finalization, and expired-pending Exam denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
  - Decision: The accepted composition uses the type-independent submission authority. Quiz/Exam worker finalization was not directly run.

- [x] Assessment Attempt completion does not depend on correctness or score.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` resolves finalization from Student-versus-deadline state; correctness and score are not completion conditions.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt passed zero/partial whole submission and deadline finalization through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.

- [x] Until then, Quizzes and Exams do not disclose correct answers.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `gate_quiz_exam_answers_for_current_cohort` withholds Quiz and Exam answers while any current Student is unfinished. `crates/server/src/webwork_document_route.rs` `permitted_answer_revision_tuple` returns no answer document unless that decision releases the answer. `src/pages/assessment_attempt_summary_page.tsx` `OpaqueWebworkPreviewFrame` renders only when review is available.

- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_provided_feedback_is_shown` verifies provided native feedback without answer disclosure.

- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` has no Question Feedback timing field; `crates/domain/src/student_feedback_release.rs` `project_student_feedback` projects supplied feedback independently while `StudentFeedbackReleaseDecision` continues to gate correct answer and explanation.

#### Assessment unrelease
- [x] Unreleasing is the destructive reversal of releasing a Course Instance Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/unrelease.sql` `unrelease_assessment` requires a released Assessment, then sets its status to unreleased and deletes that Assessment's Attempts. `schemas/base_schema/50_functions/assessments.sql` `release_assessment` is the release transition this reverses.

- [x] Unreleasing permanently deletes all Student Work for that Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/unrelease.sql` `unrelease_assessment` deletes `assessment_attempt` rows for that Assessment, and saved responses, submissions, and grading results reference those Attempts with ON DELETE CASCADE.

- [x] Student Work deletion includes Assessment Attempts, saved responses, submissions, and grading outcomes.
  - Evidence (source): `schemas/base_schema/50_functions/unrelease.sql` `unrelease_assessment` counts Attempts, finalized saved responses, submissions, and grading results, then deletes the Assessment Attempt root.

- [x] Unreleasing removes the Assessment from Student availability and returns it to a pre-release state.
  - Evidence (source): `schemas/base_schema/50_functions/unrelease.sql` `unrelease_assessment` sets `assessment_status` to unreleased. `schemas/base_schema/50_functions/assessment_attempt_start.sql` `assessment_attempt_start_gate` refuses a start unless that status is released.

- [x] The Assessment itself, its Questions, settings, and other Instructor-created content remain.
  - Evidence (source): `schemas/base_schema/50_functions/unrelease.sql` `unrelease_assessment` updates release status and deletes Assessment Attempts. It does not delete the Assessment, its entries, its policy snapshot, or the Question Revision.

- [x] The Instructor can edit the unreleased Assessment normally after Student Work is deleted.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `save_assessment_inline` saves an Instructor title and due date when the edit number matches, including while the Assessment is unreleased.

- [x] Releasing the Assessment again follows the normal Assessment Release Validation process.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_operations.sql` `validate_assessment_release` projects `assessment_release_issues` for an unreleased Assessment. `schemas/base_schema/50_functions/assessments.sql` `release_assessment` calls `validate_assessment_release` before setting released.

- [x] A later release starts with no Student Work or Assessment Attempts from the earlier release.
  - Evidence (source): `schemas/base_schema/50_functions/unrelease.sql` `unrelease_assessment` deletes that Assessment's Attempts before a later release. `schemas/base_schema/50_functions/assessments.sql` `release_assessment` changes status only and does not restore deleted Attempts.

### Assessment Attempt specifications
- [x] An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempts.sql` `ple_private.assessment_attempt` requires both a `student_record_id` and an `assessment_id`; that Assessment ID references the Course-owned `ple_data.assessment` aggregate.

- [x] Blueprint Assessments do not have Assessment Attempts.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains only reusable content and defaults; `schemas/base_schema/50_functions/assessment_attempts.sql` `ple_private.assessment_attempt` permits Attempts only through `ple_data.assessment`, the Course Instance Assessment aggregate.

- [x] Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `save_student_assessment_attempt_response` stores the response on the Question Attempt. `schemas/base_schema/50_functions/assessment_attempt_operations_api.sql` `read_student_assessment_attempt_saved_response` reads that Attempt row and takes no browser session.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `save_student_assessment_attempt_response` saved a response, and `read_student_assessment_attempt_saved_response` returned it. `tests/e2e/e2e_assessment_saved_response.sh` `save_student_assessment_attempt_response` ran that oracle on PostgreSQL. The proof is one database session and does not open a second browser.

- [x] **Instructors** control the number of permitted Assessment Attempts.
  - Evidence (source): `schemas/base_schema/50_functions/assessments.sql` `ple_data.save_assessment` and `ple_data.save_assessment_policies` accept `assessment_attempt_limit` only after `current_session_account_is_course_instructor`; issuance applies that saved value subject to Quiz/Exam effective-one.
  - Evidence (runtime): accepted independent fresh PostgreSQL actual-Store receipt covered unlimited retries, Quiz Attempt-2 denial, and expired-unlimited new-Attempt behavior through `crates/learning-data-access/src/postgres/assessment_attempt.rs` `PostgresAssessmentAttemptStore`.

- [x] Weekly Assignments default to unlimited Attempts.
  - Evidence (source): `crates/question_model/src/assessment_template.rs` `for_assessment_type` leaves a Weekly Assignment attempt limit unset, and `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` applies a cap only when that limit is present.
  - Evidence (test): `crates/question_model/src/assessment_template.rs` `regular_template_defaults_to_unlimited_attempts` checks that the Weekly Assignment default has no attempt limit. `start_assessment_attempt` was not executed against PostgreSQL in this pass.

- [x] **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `assessment_start_decision` allows another Attempt when the saved limit is absent or the prior Attempt count is still below it, and that decision does not read a score. `schemas/base_schema/50_functions/assessment_attempt_access.sql` `assessment_start_decision` applies the same count-against-limit rule, and `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` offers Start while the decision is may_start.
  - Evidence (test): `crates/domain/src/effective_assessment_properties/tests.rs` `student_may_repeat_while_the_saved_attempt_limit_allows_it` checks an unset limit after seven prior Attempts, one remaining Attempt under a limit of 3, and refusal at that limit.

- [x] When an Assessment permits multiple Attempts, the highest Assessment Attempt score is used as the
  Student's Assessment score.
  - Evidence (source): `schemas/base_schema/50_functions/grading_access.sql` `read_assessment_gradebook_evidence` independently selects the highest grading-complete submitted Attempt by earned points, then uses the latest Attempt only when no score is established; `ple_api.read_course_gradebook` consumes that private answer-free helper.
  - Evidence (runtime): accepted actual PostgreSQL evidence exercised `schemas/base_schema/50_functions/grading_access.sql` `ple_api.read_course_gradebook`, proving an earlier higher earned score beats a later lower score and a later unfinished or pending Attempt does not replace it. Current Question points recalculated the selected score from `8` to `16`; a Bonus contribution retained a zero possible denominator; and a latest unscored expired Attempt remained the fallback when no completed score existed.
  - Evidence (source): `schemas/base_schema/50_functions/student_assessment_landing.sql` `ple_private.read_student_released_assessment_landing_evidence` keeps progress, completion, and resume state on the latest Attempt but obtains the Assessment score from the selected highest Attempt and applies that Attempt's copied disclosure timing. `src/api/decoders/live_student_course_landing.ts` `decodeAssessmentSummary` requires direct `assessmentScore` and rejects the retired `score` alias; `src/pages/student_course_landing_page.tsx` labels it `Assessment score`.
  - Evidence (runtime): accepted actual PostgreSQL evidence exercised `schemas/base_schema/50_functions/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, preserving an earlier higher score across later lower, unfinished, and pending Attempts, including inverse selected-Attempt/latest-Attempt disclosure cases. The focused decoder/presentation lane passed 8/8, and compiled M6 component evidence passed.

- [x] Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `commit_student_assessment_attempt_finalization` inserts the submission and calls `record_direct_automated_grading_result` in that same Student finalization. The Instructor is not an actor.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `commit_student_assessment_attempt_finalization` recorded one Student submission and one grading result at 1 of 1 points. `tests/e2e/e2e_assessment_saved_response.sh` `commit_student_assessment_attempt_finalization` ran that oracle on PostgreSQL. The supplied credit was the already-returned evaluation. This proof did not call a Question Backend.

- [x] Automatic grading does not require a separate Student or **Instructor** grading workflow.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `record_direct_automated_grading_result` writes the grading result during `commit_student_assessment_attempt_finalization`. There is no separate grading queue.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `commit_student_assessment_attempt_finalization` wrote that grading result while the session was the Student. `tests/e2e/e2e_assessment_saved_response.sh` `commit_student_assessment_attempt_finalization` ran that oracle on PostgreSQL.

### Assessment response and submission specifications
- [x] The Student submission action submits the whole Assessment Attempt.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` creates one Assessment submission and finalizes every open issued Question in that Attempt.
  - Evidence (runtime): accepted C525 actual-Store evidence exercised whole zero- and partial-response submissions through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.

- [x] A Question either has a complete saved response or has no saved response.
  - Evidence (source): `src/pages/assessment_attempt_finish.ts` `saveCompleteResponseBeforeAttemptSubmission` saves only when the response is complete. `schemas/base_schema/20_tables/assessment_attempt.sql` `assessment_attempt_saved_response` keeps one response row for a Question Attempt.
  - Evidence (test): `tests/test_backend_owned_bridge.mjs` `saveCompleteResponseBeforeAttemptSubmission` leaves an incomplete response unsaved. `tests/e2e/assessment_saved_response_oracle.sql` `save_student_assessment_attempt_response` kept one saved row after a replacement. The bridge test ran with node --import tsx.

- [x] PLE saves complete Question responses as the **Student** works.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` saves an ordinary Student response and asserts its persisted `saved` state.

- [x] The Student may change a saved response while the Assessment Attempt remains open.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `save_student_assessment_attempt_response` replaces `student_response` while the Attempt has no submission.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `save_student_assessment_attempt_response` replaced bilayer with cholesterol, then a save after submission left cholesterol stored. `tests/e2e/e2e_assessment_saved_response.sh` `save_student_assessment_attempt_response` ran that oracle on PostgreSQL.

- [x] Submitting the Assessment Attempt finalizes all saved Question responses together as Student Work.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` verifies every saved response before inserting the Assessment submission and its finalized Question responses.
  - Evidence (runtime): accepted C525 actual-Store evidence exercised whole partial-response submission and history through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.

- [x] Questions without a saved response remain visibly unanswered when the Attempt is submitted.
  - Evidence (source): `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` labels closed issued Questions with no submission as **Unanswered** and reserves unavailable-response wording for submitted Questions whose saved response is not released.
  - Evidence (runtime): `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent`, accepted authenticated Avery R-4 submitted/expired history at `/private/tmp/ple-unanswered-history-fixed-1280.png` and `/private/tmp/ple-unanswered-history-fixed-390.png`, shows Q1, Q3, and Q4 as **Unanswered**, incorrect `0 / 1`; Q2 retains all four exact MATCH pairs and correct `1 / 1`; total is `1 / 4`.

- [x] An unanswered Question receives zero credit and counts as incorrect without being sent to the
  Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `ple_private.score_recorded_credit` treats null retained credit as unanswered zero earned points while retaining current points possible; evaluated zero credit remains a distinct grading outcome.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_attempt_operations_api.sql` `ple_api.prepare_student_assessment_attempt_finalization` was invoked by `/private/tmp/ple-unanswered-connected-proof/run.sh --isolated` running the non-versioned temporary fixture `/private/tmp/ple-unanswered-connected-proof/proof.sql` against fresh PostgreSQL; its ordinary prepare/commit, history, and Gradebook calls observed no unanswered submission/result, one evaluated-zero result/receipt, incorrect history, `0 / 8` versus `8 / 8`, then `0 / 13` versus `13 / 13`; artifact `/private/tmp/ple-unanswered-connected-artifacts.vUexkI/proof.log` (exit 0). This is not a permanent test and does not exercise Attempt expiry.

- [x] PLE treats an incomplete Question response as unsaved, although the Question interface may keep the Student's unfinished input while they work.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `saveCurrentResponseBeforeAttemptSubmission` keeps an incomplete draft local and does not replace an earlier saved response. `src/pages/assessment_attempt_finish.ts` `saveCompleteResponseBeforeAttemptSubmission` returns without saving when the response is incomplete.
  - Evidence (test): `tests/test_backend_owned_bridge.mjs` `saveCompleteResponseBeforeAttemptSubmission` does not call save for incomplete input, including when an earlier save has already succeeded. The test ran with node --import tsx.

- [x] A Question Backend may evaluate a response before Assessment submission when needed for its interaction.
  - Decision: Human Guidance permits early evaluation when a Question Backend interaction needs it. Current native Questions and WeBWorK do not need evaluation before whole-Attempt submission. WeBWorK hides check-answers, and a complete native response is graded only when submission asks for an outcome. Saving acknowledges the response without a credit fraction.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `save_selected_response` stores one complete response and returns `SavedResponseAcknowledgement` with response state saved. It does not call `evaluate_saved_responses`.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `finalize_assessment_attempt` calls `evaluate_saved_responses` only while submitting the Attempt.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/protocol.rs` `hideCheckAnswersButton` keeps the WeBWorK check-answers control off, so that interaction does not need a pre-submission grade.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` returns the credit fraction when submission asks the native backend to evaluate.
  - Evidence (test): `crates/server/src/assessment_delivery/submission.rs` `a_question_backend_may_evaluate_a_response_before_assessment_submission_when_needed` put the shipped save route for a two-choice response, received responseState saved, stored the durable choice, and left finalization prepare and commit at zero. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` returns the compiled evaluation immediately.
  - Evidence (source): `crates/question_model/src/student_work/grading.rs` `QuestionEvaluation` records only correctness and a normalized credit fraction.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/grade.rs` `QuestionGradingOutcome::Evaluated` is the only grading result for a complete renderer score.
  - Evidence (source): `crates/adapters/webwork/src/http_renderer/client.rs` `WeBWorK uses stateless grading` refuses a lifecycle state on a grading request.
  - Evidence (test): `crates/adapters/ple/src/lib/question_json_source_tests.rs` `questions_are_strictly_and_deterministically_automated_grading_does_not_require_an_instructor` graded complete responses to credits 1, 1, 0, and 0.
  - Evidence (test): `crates/adapters/webwork/src/http_renderer/tests.rs` `grade_forwards_ordered_pairs_once_with_trusted_fields` returned evaluated credits 0, 0.5, and 1.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `opaque_lifecycle_issues_and_grades_one_backend_owned_payload` graded one complete payload as an evaluated outcome and kept lifecycle state empty.
  - Evidence (test): `crates/adapters/webwork/src/lib/tests.rs` `issuance_refuses_renderer_lifecycle_state_for_stateless_webwork` refused renderer-issued lifecycle state for the stateless WeBWorK integration.
  - Owner: 07_questions.md / Question Backend grading and feedback (first occurrence; identical requirement and status).

- [x] The **Student** does not see the grading outcome until the Assessment Attempt is submitted.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AssessmentAttemptPage` has no grading-outcome display. `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` shows the score and marked result on the submitted summary.
  - Evidence (test): `tests/playwright/test_assessment_attempt_delivery.mjs` `mountAssessmentAttemptDelivery` shows the open Attempt without Marked correct, Your score, Correct answer, or points earned.

### Assessment Attempt timing and expiration specifications
- [x] Each Assessment Attempt has a time limit.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` refuses a start unless `assessment_effective_duration_seconds` is finite, then stores that limit as `expires_at`. `src/pages/assessment_attempt_page.tsx` `AssessmentAttemptPage` shows the remaining time and the automatic-submission deadline.
  - Evidence (test): `tests/playwright/test_assessment_attempt_delivery.mjs` `an assessment attempt has a time limit` supplies 15 minutes remaining for Membrane review, shows a timer that is not Untimed, and shows the automatic-submission deadline. `start_assessment_attempt` was not executed against PostgreSQL in this pass.

- [x] Each Assessment may contain at most 250 Questions.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_release_validation.sql` defines `ple_data.assessment_delivered_question_count`; `schemas/base_schema/50_functions/assessment_pool_forks.sql` `ple_data.import_assessment_question_pool_fork` calls it before advancing the parent Assessment Edit Number.
  - Evidence (runtime): accepted fresh PostgreSQL proof exercised `schemas/base_schema/50_functions/assessment_pool_forks.sql` `ple_data.import_assessment_question_pool_fork`, imported to 250, rejected 251 with `23514`, and verified atomic rollback of the child Pool, Revision, Entry, ownership association, and parent Edit Number. Artifact: `/private/tmp/ple-finite-pool-bound-artifacts.hoI0on/proof.log`. This does not establish browser or general timing behavior.

- [x] A Question Pool counts as the number of Questions selected from it for the Assessment Question
  limit and default time calculation; selecting 3 of 199 Questions counts as 3.
  - Evidence (source): `src/pages/assessment_workspace/assessment_pool_entry_editor.tsx` `AssessmentPoolEntryEditor` states the selected count of the pinned Questions. `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `deliveredAssessmentQuestionCount` counts each available Pool as its `selectionCount` toward the 250 Question limit. `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessmentDurationDefaultDescription` uses that same selected count for the default time. `schemas/base_schema/50_functions/assessment_release_validation.sql` `assessment_delivered_question_count` sums `selection_count`.

- [x] The default time limit is 1.5 minutes per Question, rounded up to the nearest whole minute.
  - Evidence (source): `src/assessment_duration.ts` `calculatedAssessmentDurationMinutes` rounds 1.5 minutes per Question up to a whole minute, and `schemas/base_schema/50_functions/assessment_release_validation.sql` `assessment_effective_base_duration_seconds` uses that same whole-minute default when no override is stored.
  - Evidence (test): `tests/test_assessment_duration.mjs` `calculatedAssessmentDurationMinutes` checks 2 minutes for 1 Question, 3 minutes for 2 Questions, and 375 minutes for 250 Questions. `assessment_effective_base_duration_seconds` was not executed against PostgreSQL in this pass.

- [x] Instructors can override the default time limit up to 12 hours.
  - Evidence (source): `src/assessment_duration.ts` `assessmentDurationOverrideSecondsFromMinutesDraft` accepts a whole-minute override through 12 hours, and `schemas/base_schema/20_tables/assessment.sql` `assessment_attempt_time_limit_seconds` stores only 1 to 43200 seconds.
  - Evidence (test): `tests/test_assessment_duration.mjs` `assessmentDurationOverrideSecondsFromMinutesDraft` accepts 720 minutes as 43200 seconds and rejects 721 minutes. The table check was not executed against PostgreSQL in this pass.

- [x] The interface should show the calculated default time limit and provide a specific Instructor override.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` shows `assessmentDurationDefaultDescription` and saves the Instructor's minutes through `assessmentDurationOverrideSecondsFromMinutesDraft`.
  - Evidence (test): `tests/playwright/test_course_instance_delivery.mjs` `assessment properties show the calculated time limit and save an instructor override` shows Calculated default: 2 minutes for 1 Questions, saves 20 minutes as 1200 seconds, reloads that override, and clears it so the calculated default remains. The policy SQL was not executed against PostgreSQL in this pass.

- [x] Time limits must support individual **Students** with accommodations, such as 1.5X or 2X time.
  - Evidence (source): `src/pages/assessment_workspace/assessment_student_time_accommodations.tsx` `AssessmentStudentTimeAccommodations` offers 1.5X and 2X for one active Student and saves that choice through `saveAssessmentStudentTimeAccommodation`. `schemas/base_schema/50_functions/assessment_student_time_accommodation.sql` `student_assessment_time_configuration` stores `p_time_multiplier`.
  - Evidence (test): `tests/playwright/test_assessment_student_time.mjs` `an instructor saves 1.5X and 2X time for one student` opens Assessment Properties, offers only active Student AVERY, and saves multipliers 1.5 and 2. `student_assessment_time_configuration` was not executed against PostgreSQL in this pass. The stub keeps the effective duration at the base 120 seconds.

- [x] Student accommodations are applied after the Assessment time limit and may extend that Student's effective time limit
  up to 24 hours.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_operations.sql` `assessment_effective_duration_seconds` applies the Student multiplier after the stored Assessment time limit and returns 86400 seconds when that extension reaches 24 hours.
  - Evidence (test): `tests/e2e/e2e_assessment_duration_accommodation.sh` `assessment_effective_duration_seconds` called that function on PostgreSQL. A stored 1800-second limit stayed 1800 with no multiplier, became 2700 at 1.5X and 3600 at 2X, and became 86400 at 100X. A stored 101-second limit became 152 at 1.5X and 86400 at 856X. student_assessment_time_configuration was not executed in this pass.

- N/A Attempt time limits help **Students** develop an accurate sense of expected working speed.
  - Reason: audited pedagogical purpose, not a separately testable PLE behavior or demonstrated learning effect. The preceding time-limit/default/override/accommodation requirements remain binding implementation requirements.

- [x] Assessment Attempts use wall-clock time.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` observes database `clock_timestamp()` reach the persisted expiry before rejecting late work.

- [x] The server owns the Attempt start and expiration times.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` reads the persisted server `expires_at` against database `clock_timestamp()`.

- [x] Attempt time continues while the **Student** is disconnected or the browser is closed.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_background_expiry` proves the generic worker finalizes an expired Attempt without a further Student interaction.

- [x] A **Student** may reconnect, reload, or use another browser session to resume the same active Attempt.
  - Evidence (source): `schemas/base_schema/50_functions/authentication.sql` `resolve_and_install_session` installs one authenticated session, including its session id, before Student work.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` returns the open Attempt for that Student record and does not write a new expires_at on that path.
  - Evidence (test): `tests/e2e/assessment_resume_expiration_oracle.sql` `resolve_and_install_session` installed sessions 72000000-0000-0000-0000-0000000000a1 and 72000000-0000-0000-0000-0000000000a2 for Student USRS0000T, and each session resumed Attempt 72000000-0000-0000-0000-000000000001 with the same expires_at and one issued Question.
  - Evidence (test): `tests/e2e/e2e_assessment_resume_expiration.sh` `from_distinct_browser_sessions` ran that oracle on PostgreSQL. This proof installs two authenticated sessions and does not open a browser page.

- [x] Resuming an Attempt does not reset or extend its expiration time.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` returns the open Attempt before any new-Attempt insert, and that resume path does not write `expires_at`.
  - Evidence (test): `tests/e2e/assessment_resume_expiration_oracle.sql` `start_assessment_attempt` created one Attempt, then a second call with a different proposed Attempt id returned that same Attempt and the same expires_at, with one issued Question. `tests/e2e/e2e_assessment_resume_expiration.sh` `start_assessment_attempt` ran that oracle on PostgreSQL. This proof uses one database session and does not open a second browser.

- [x] Attempt expiration is checked whenever a **Student** interacts with the Attempt.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` proves a save rechecks the server clock after lock waiting and returns `expired`.

- [x] Background processing ensures expired Attempts are submitted even when the **Student** is no longer connected.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_background_expiry` waits only for generic-worker immutable evidence, then proves the expired Attempt is submitted.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_expired_student_assessment_attempt_finalization` is invoked under the real expiry-worker SQL role by fresh `/private/tmp/ple-attempt-timing-artifacts.7HGbyP/proof.log` after a wait against the original deadline with advancing `clock_timestamp()`, without further Student interaction. This supplementary proof does not exercise the deployed worker polling loop or actual browser disconnect.

- [x] When an Attempt expires, PLE submits the whole Attempt and finalizes its saved responses.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `commit_expired_student_assessment_attempt_finalization` submits the expired Attempt as deadline finalization and finalizes its saved responses. `prepare_expired_student_assessment_attempt_finalizations` selects only Attempts whose expires_at has passed.
  - Evidence (test): `tests/e2e/assessment_attempt_expiry_oracle.sql` `commit_expired_student_assessment_attempt_finalization` recorded one submission with no authorizing account and finalized the saved response after the Attempt's 5-second limit elapsed. `tests/e2e/e2e_assessment_attempt_expiry.sh` `commit_expired_student_assessment_attempt_finalization` ran that oracle on PostgreSQL. The supplied credit was the already-returned evaluation. This proof did not call a Question Backend.

- [x] Unanswered Questions remain visibly unanswered, receive zero credit, and count as incorrect.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `ple_private.score_recorded_credit` assigns unanswered work zero earned points while retaining its current denominator.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_expired_student_assessment_attempt_finalization`, accepted fresh PostgreSQL expiry proof, retains an unanswered Question without submission/grading evidence and reports it incorrect at `0 / 8`. Artifact: `/private/tmp/ple-attempt-timing-artifacts.7HGbyP/proof.log`.
  - Evidence (runtime): `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent`, accepted authenticated Avery R-4 expired history at `/private/tmp/ple-unanswered-history-fixed-1280.png` and `/private/tmp/ple-unanswered-history-fixed-390.png`, visibly labels Q1, Q3, and Q4 **Unanswered** and incorrect `0 / 1`; Q2 retains four exact MATCH pairs and correct `1 / 1`; total is `1 / 4`.

- [x] Unanswered Questions are not sent to the Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` returns only saved responses, so an unanswered Question is absent from the prepared work set.
  - Evidence (source): `crates/server/src/assessment_delivery/direct_finalization.rs` `evaluate_saved_responses` grades each saved response through the Question Backend and returns one evaluation per saved response.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `evaluate_saved_responses` sends `preparation.saved_responses` for a Student submission.
  - Evidence (source): `crates/server/src/worker.rs` `evaluate_saved_responses` sends that same saved-response list when an Attempt expires.
  - Evidence (test): `crates/server/src/assessment_delivery/direct_finalization.rs` `unanswered_questions_are_not_sent_to_the_question_backend` graded the saved blue response to credit 1 and returned no evaluation when the saved-response list was empty.
  - Evidence (test): `tests/e2e/assessment_saved_response_oracle.sql` `unanswered_question_omitted_from_backend_work` issued two Questions, saved one, and prepared one ready row for that saved Question Attempt with zero rows for the unanswered Question Attempt.
  - Evidence (test): `tests/e2e/e2e_assessment_saved_response.sh` `unanswered_question_omitted_from_backend_work` ran that oracle on PostgreSQL.

### Student Work specifications
- [x] Student Work keeps the exact Published Question Revision delivered to the **Student**.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempts.sql` `issued_question_is_immutable` stores issued Question revision identity under foreign-key protection.

- [x] For a Question Pool, Student Work keeps the Question Pool ID, the Pool's Edit Number at
  selection, and the exact Published Question Revision selected.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_start.sql` `start_assessment_attempt` copies the Pool ID, the Pool edit number, and the selected member revision onto the Attempt.

- [x] Student Work keeps each saved response as finalized with the submitted Attempt and the grading outcome returned by the Question Backend.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` checks every saved-response snapshot, inserts the submitted Attempt and exact saved responses in one transaction, and supplies each evaluated normalized credit to `schemas/base_schema/50_functions/grading.sql` `ple_private.record_direct_automated_grading_result` for retained grading evidence.
  - Decision: This is current persistence-source evidence, not a fresh connected/backend receipt. The retired SQL oracle and legacy Assignment transport shell are not current runtime proof.

- [x] Changes to Assessment content do not replace Question evidence already delivered in existing Attempts.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempts.sql` `issued_question_is_immutable` rejects a rewrite of an issued Question. `schemas/base_schema/50_functions/assessment_attempt_start.sql` `ensure_assessment_entry_snapshot` stores the delivered entry facts on that Question.

- [x] PLE should retain only the additional historical Student Work data needed to interpret or grade that work correctly.
  - Evidence (source): `schemas/base_schema/20_tables/assessment_attempt.sql` `assessment_entry_snapshot` stores entry kind, scoring rule, points, question or pool identity, and attempt limits. The issued Question stores the revision reference. The grading result stores normalized credit, and its comment says PLE retains only that outcome.
  - Evidence (source): `schemas/base_schema/50_functions/grading_access.sql` `read_assessment_gradebook_evidence` applies current question points, then current pool points, then the snapshot points.
  - Evidence (test): `tests/test_course_retention_ferpa.py` `PLE should retain only the additional historical Student Work data needed to interpret or grade that work correctly.` reads those shipped definitions and does not execute PostgreSQL.

### Assessment scoring specifications
- [x] Blueprint Assessments and Course Instance Assessments assign point values to Questions.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `changeScoring` stores Points possible on a Blueprint Assessment Question and Points per Question on a Blueprint Pool.
  - Evidence (source): `src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx` `withFixedQuestionPointValues` saves a Course Instance Assessment fixed Question point value through the Assessment workspace.

- [x] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Evidence (source): `crates/adapters/ple/src/lib/question_json_source.rs` `grade_question_json` returns the compiled evaluation for one complete response.
  - Evidence (source): `crates/question_model/src/student_work/grading.rs` `QuestionEvaluation` keeps that credit fraction as the evaluation fact.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `backend_returned_credit_is_stored_as_the_immutable_grading_outcome` graded the correct color response at credit 1 and the same incorrect color response at credit 0 twice.
  - Owner: 07_questions.md / Question Backend grading and feedback (first occurrence; identical requirement and status).

- [x] PLE stores the credit fraction as the Question grading outcome.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `ple_private.record_direct_automated_grading_result` inserts the supplied credit fraction into `ple_private.grading_result.normalized_credit`, constrained to the inclusive unit interval and retained under `grading_result_is_immutable`.
  - Evidence (runtime): `schemas/base_schema/50_functions/grading.sql` `ple_private.record_direct_automated_grading_result` is invoked by the accepted private PostgreSQL production-SQL lifecycle fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) retained the submitted `0.5` fraction while rescoring current points from `8` to `13`. This SQL-only fixture simulates the initial synchronous Backend credit and does not establish Backend transport or HTTP.

- [x] Course Instance Assessment scores are calculated from stored credit fractions and current Question point values.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` joins retained grading credit to the current Assessment entry point value and calls `schemas/base_schema/50_functions/grading.sql` `ple_private.score_recorded_credit`; current scoring treatment remains an explicit input.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` is invoked by the accepted private PostgreSQL production-SQL lifecycle fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) first recorded `0.5` as `4 / 8`, then authorized an expected-current Instructor save to points `13` and observed `6.5 / 13` for the answered Question, `0 / 13` unanswered, and `6.5 / 26` through replay, history, Student landing, and Instructor Gradebook reads. This does not establish HTTP, rendering, or Backend transport.

- [x] An unanswered Question contributes zero points to the Assessment score and counts as incorrect.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `ple_private.score_recorded_credit` distinguishes null retained credit from an evaluated zero-credit result, returning zero earned points for unanswered work even under `full_credit` while retaining the current denominator.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_attempt_operations_api.sql` `ple_api.prepare_student_assessment_attempt_finalization` was invoked by `/private/tmp/ple-unanswered-connected-proof/run.sh --isolated` running the non-versioned temporary fixture `/private/tmp/ple-unanswered-connected-proof/proof.sql` against fresh PostgreSQL; its ordinary prepare/commit calls verified `0 / 8` unanswered versus `8 / 8` evaluated zero, no unanswered submission/result, one evaluated result/receipt and Assessment submission across both replays, incorrect history, `0 / 13`, `13 / 13`, and Gradebook `13 / 26`; artifact `/private/tmp/ple-unanswered-connected-artifacts.vUexkI/proof.log` (exit 0). This is not a permanent test and does not exercise Attempt expiry.

- [x] When an Assessment has multiple submitted Attempts, the highest Assessment Attempt score is the
  Student's Assessment score.
  - Evidence (source): `schemas/base_schema/50_functions/grading_access.sql` `read_assessment_gradebook_evidence` selects the highest grading-complete submitted Attempt by earned points; `schemas/base_schema/50_functions/student_assessment_landing.sql` consumes that same helper while retaining latest-Attempt progress separately. The retired configurable grade-rule enum, field, SQL columns, and editor choices are absent from the production model and UI.
  - Evidence (runtime): accepted independent fresh PostgreSQL lifecycle proof exercised `schemas/base_schema/50_functions/grading_access.sql` `ple_api.read_course_gradebook` and the Student landing helper across four submitted or issued Attempts with individual scores `12`, `4`, `16`, and `NULL`. Instructor Gradebook and Student landing both projected `12 / 16`, `12 / 16`, and `16 / 16`; the fourth in-progress Attempt did not replace the selected `16 / 16` score, while Student landing continued to show latest-Attempt state. Artifact: `/private/tmp/ple-highest-score-proof/artifacts.rQxVs8/proof.log` (exit 0). This privileged fixture and simulated backend proof does not establish HTTP, rendering, actual backend grading, or unlimited-Attempt eligibility.

- [x] PLE uses Question point values directly to calculate Assessment scores.
  - Evidence (test): `crates/question_model/src/student_work/grading.rs` `current_points_recalculate_without_changing_recorded_credit` tests current point values rescale recorded credit directly.

- [x] PLE does not use separate Question weights, Grade Categories, weighted categories, Course Grade
  Schemes, or Course percentage calculations.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `score_recorded_credit` returns points_earned and points_possible from the retained credit and the Question point value.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `grade_contribution_points_possible` returns the authored points, or zero for a Bonus Assignment, extra credit, or an excluded entry.
  - Evidence (source): `crates/server/src/live_gradebook/export.rs` `COLUMNS` names roster, Assessment, status, points_earned, and points_possible.

- [x] For the pilot, grade export uses CSV or TSV only and exports point-based Assessment scores.
  - Evidence (source): `crates/server/src/live_gradebook/export.rs` `encode` writes quoted CSV or TSV using only the seven point-grade fields in `COLUMNS`. `src/pages/gradebook_page.tsx` `GradebookCoursePage` offers Download CSV and Download TSV.
  - Evidence (test): `crates/server/src/live_gradebook/export_tests.rs` `point_exports_preserve_states_points_order_and_empty_courses` checks both formats, including earned and possible points and a header-only empty course.

- [x] The Instructor handles Course-level weighting or percentage calculations in the home LMS.
  - Evidence (source): `src/pages/gradebook_page.tsx` `GradebookCoursePage` tells the Instructor to handle Course weighting and percentages in the home LMS. `crates/server/src/live_gradebook/export.rs` `COLUMNS` exports point fields and has no weight, percentage, or Course-total column.
  - Evidence (test): `crates/server/src/live_gradebook/export_tests.rs` `point_exports_preserve_states_points_order_and_empty_courses` checks that the exported columns are the point fields.

- [x] Changing Question point values recalculates affected Assessment scores.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.prepare_assessment_attempt_finalization` reads an already-submitted Attempt by joining immutable credit to current Assessment entry points; `schemas/base_schema/50_functions/grading.sql` `ple_private.score_recorded_credit` applies those current points at read time.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` is invoked by the accepted private PostgreSQL production-SQL fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) changed both current entry values from `8` to `13` and advanced the expected-current edit number, then observed `0.5` rescored from `4 / 8` to `6.5 / 13` and the Assessment from `4 / 16` to `6.5 / 26` across three replays and the history, landing, and Gradebook projections. This does not establish HTTP or rendering.

- [x] Score recalculation does not require another Question Backend interaction.
  - Evidence (source): `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `prepare_assessment_attempt_finalization` returns already_submitted and the current score before source resolution or backend-evaluation preparation.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `score_recorded_credit` calculates that score from retained credit and the current Question point value.
  - Evidence (source): `crates/server/src/assessment_delivery/submission.rs` `finalize_assessment_attempt` returns AlreadySubmitted before evaluate_saved_responses.
  - Evidence (test): `crates/server/src/assessment_delivery/submission.rs` `score_recalculation_does_not_require_another_question_backend_interaction` posted that shipped route for an already-submitted Attempt, received submitted, and observed one prepare, zero finalization commits, and zero renderer connections. No Live Demo stack was started. No PostgreSQL proof was run.

- [x] Score recalculation does not change the stored Question grading outcome.
  - Evidence (source): `schemas/base_schema/50_functions/grading.sql` `ple_private.score_recorded_credit` only calculates a score and performs no writes; `grading_result_is_immutable` rejects updates through `ple_private.reject_grading_evidence_change`. `schemas/base_schema/50_functions/assessment_attempt_finalization.sql` `ple_private.prepare_assessment_attempt_finalization` reads existing grading outcomes without replacing them.
  - Evidence (runtime): `schemas/base_schema/50_functions/assessment_operations.sql` `ple_api.save_assessment` is invoked by the accepted private PostgreSQL fixture at `/private/tmp/ple-current-rescore-proof/proof.sql`; its artifact `/private/tmp/ple-current-rescore-artifacts.xDwFxD/proof.log` (exit 0) retained the `0.5` fraction and matched before/after JSON hashes for every grading-result, finalized-Question-response, Assessment-submission, and automated-grading-receipt row after the authorized points edit and replay. This does not establish HTTP, rendering, or Backend transport.
