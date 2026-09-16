# Human Guidance implementation compliance checklist

Source: `docs/HUMAN_GUIDANCE.md`. Human Guidance remains authoritative. This file records
implementation status only.

- [x] Verified: implemented behavior matches the bullet. Evidence follows.
- [ ] Unverified, or implementation differs. Mismatch follows.
- N/A: audited and not an implementation requirement. Reason follows.

# Human guidance

## How to use this guidance

Implementation status: N/A

Reason: This section gives rules for writing and maintaining Human Guidance. It does not specify
PLE product or code behavior.

- N/A Guidance bullets should start with the subject when practical, making them easier to scan.
- N/A Guidance should stay terse and in my own words.
- N/A Uncertainty should remain when I have not made a final decision.
- N/A This document uses GitHub Flavored Markdown (GFM).
- N/A Bullet duplication is acceptable because many agents only skim read one section at a time.

## Development principles

### Agent working principles

- N/A Read and learn the core principles in docs/REPO_STYLE.md
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Apply the Keep It Simple, Stupid (KISS) philosophy aggressively.
  - Reason: agent instruction, not implemented PLE product behavior.
- [ ] Prefer the smallest coherent design that meets actual requirements and known failure modes.
  - Mismatch: Current implementation choices have not been audited against this KISS constraint.
- [ ] Complexity must earn its place.
  - Mismatch: Current implementation choices have not been audited against this KISS constraint.
- N/A Time should be used efficiently. Agents and tokens are cheap; wall time is not.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Hard work should be broken into small, independently completable tasks.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Write plans in plain, concrete language. Use technical terms when they add precision.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Prioritize positive prompting. Avoid naming unneeded tools. Positive prompting plus omission is better.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Small LMs mishandle negative prompting and flip negative instructions producing poor code and egregious results.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Classify one-time checks separately from permanent tests.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Finish the obvious. Continue while the next safe step is defined by the plan, implied by the current task.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Robust means the software continues to function despite imperfect inputs, data, state, or behavior.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Treat tests as liabilities as well as assets. Keep only requirements and gates grounded in actual needs.
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

- [x] Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
  - Evidence (source): `tests/test_source_file_line_limit.py` `test_source_file_line_limit` enforces the exclusive `LINE_LIMIT = 1000` across tracked authored source; the current longest tracked `src/` source, `src/style.css`, is 994 lines.
  - Evidence (source): `src/styles/product_role.css` `.ple-app-ribbon__product-role[data-product-role]` owns the extracted shared role-color selectors; `src/index.html` `styles/product_role.css` loads that focused stylesheet after `style.css`.
  - Evidence (source): `pipeline/build.mjs` `STATIC_STYLESHEETS` copies and fingerprints `styles/product_role.css` into the production build.
  - Evidence (test): `tests/e2e/e2e_ribbon_production_styles.mjs` `production CSS includes the Ribbon root rule` rebuilds and inspects the emitted production CSS.
- [ ] PLE is pre-production with no users. Fix the design directly rather than preserving legacy behavior.
  - Evidence (source): `crates/project-tools/src/database_coordinator.rs` `run` rejects migration while the base release is pre-production and requires direct base-schema correction.
  - Mismatch: this database-only guard does not prove that every live alternate reader, writer, route, parser, DTO, client, fallback, alias, or migration path has been removed. The obsolete Assessment route layer and tsgen retired-header migration are gone, but live CI/A/U/BP client guards and receipt formats remain under audit. One Unrelease mutation path was found; no duplicate-current-path claim is made.
- [x] Use SQL directly to create the initial PostgreSQL database.
  - Evidence (source): `schemas/base_schema/install.sql` ordered `psql` installation manifest creates the base database schema.
- [x] Before production, edit the main database design directly as the design changes.
  - Evidence (source): `crates/project-tools/src/database_coordinator.rs` `run` reports that the base schema must be updated directly until production freeze.
- [x] After production, update existing databases without rebuilding them from scratch.
  - Evidence (source): `local_stack_control/lifecycle_migrations.py` `database_operation_for` selects `migrate` after initial installation.

- [ ] PLE is pre-production with no users or durable production data. Improve the design directly.
  - Evidence (source): `crates/project-tools/src/database_coordinator.rs` `run` makes direct base-schema correction the pre-production path.
  - Mismatch: this database-only guard does not prove that every live alternate reader, writer, route, parser, DTO, client, fallback, alias, or migration path has been removed. The obsolete Assessment route layer and tsgen retired-header migration are gone, but live CI/A/U/BP client guards and receipt formats remain under audit. One Unrelease mutation path was found; no duplicate-current-path claim is made.
- [x] Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
  - Evidence (source): `devel/development_conformance_audit.py` `current_source_paths` inventories tracked and untracked current-worktree source files without opening deleted paths; `source_name_violations` enforces readable snake_case names.
  - Evidence (source): `devel/development_conformance_audit.py` `load_allowlist` permits only exact documented external-name exceptions owned by approved authority sections.
  - Decision: The one-time adversarial and current-worktree proof was removed after validation; the durable audit command is the regression boundary.
- N/A Adaptability should be a focus so the software can evolve as requirements and insights change.
  - Reason: future development-direction guidance; it makes no current PLE product behavior claim.
- [x] Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
  - Evidence (source): `devel/dependency_freshness_audit.py` `check_cargo`, `check_node`, and `check_pypi` fail closed unless every direct manifest dependency matches the dated primary-registry snapshot.
  - Evidence (source): `devel/dependency_freshness_snapshot.json` `"snapshot_date": "2026-09-14"` records every direct dependency. `cargo_exceptions` is empty; the only range exception is TypeScript's documented upstream compatibility cap.
  - Evidence (test): `tests/test_crate_boundaries.py` `test_registry_dependencies_use_open_latest_first_requirements` retains Cargo's open latest-first manifest contract.
  - Decision: PyPI declarations use one `>=` floor each, and `aws-sdk-s3` resolves the recorded current 1.147.0 release; no frozen AWS release is permitted. This declaration-and-resolution audit does not duplicate the separate workspace build gate, whose upstream Smithy failure remains an open build report. TypeScript retains its documented temporary upstream compatibility cap.
- N/A If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
  - Reason: conditional future implementation option; it makes no current PLE product behavior claim.
- [x] Do not create or leave placeholder database tables, states, APIs, workers, or compatibility scaffolding before the feature has an approved product design.
  - Evidence (source): `devel/development_conformance_audit.py` `scaffolding_violations` rejects declared placeholder and compatibility scaffolding unless `load_allowlist` records an exact approved design owner.
  - Evidence (source): `devel/development_conformance_audit.py` `all_python_declarations` uses the Python AST and `masked_non_python_source` masks comments and literals before bounded TS/JS, Rust, and SQL declaration checks; `current_source_paths` includes CSS and HTML in the current-worktree filename inventory.
  - Decision: The one-time adversarial and current-worktree proof was removed after validation; the durable audit command is the regression boundary.

### PLE development rules

- [x] A fresh production installation includes the complete Live Demo by default.
  - Evidence (source): `local_stack_control/lifecycle.py` `provision_ready_installation_data` provisions installation data after readiness.
  - Evidence (test): `tests/test_local_stack_demo_provisioning.py` `test_ready_installation_data_uses_one_canonical_migrator_command_after_readiness` verifies default provisioning.
- [x] Treat the initial course content as shipped examples.
  - Evidence (source): `schemas/installation_data/live_demo.sql` `ple_data.course_instance` is seeded as installation-owned Live Demo teaching data.
- [x] BiologyProblems.org content is free and open source.
  - Evidence (source): `content/genetics/ATTRIBUTION.md` `CC BY 4.0` records the bundled Biology Problems OER content license.
- [x] The Genetics Blueprint Course from BiologyProblems.org ships as the example course.
  - Evidence (source): `content/genetics/manifest.yaml` `short_name` and `long_name` define the bundled Genetics Blueprint.
  - Evidence (source): `local_stack_control/lifecycle.py` `require_bundled_genetics_without_live_demo` verifies bundled Genetics installation data.
- N/A All Podman content on the Mac-Studio-36G machine belongs to this project.
  - Reason: human ownership statement about a named machine, not implemented PLE behavior.
- N/A Neil pre-approves pruning Podman images, volumes, and containers on Mac-Studio-36G as needed.
  - Reason: human authorization statement, not implemented PLE behavior.
- N/A The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
  - Reason: human-owned project priority, not implemented PLE behavior.
- [x] PLE should use one global installation with no institution boundaries.
  - Evidence (source): `schemas/base_schema/accounts.sql` `ple_private.account` has no institution column or foreign key; product roles are global account data.
- [x] Project images and simulated live-stack data are disposable acceptance infrastructure.
  - Evidence (source): `local_stack_control/disposable_stack_adapter.py` `disposable_target` creates a closed disposable Compose target for acceptance infrastructure.
- [x] `./launchers/run_live_demo.sh` is the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && python3 local_stack.py`.
  - Evidence (source): `launchers/run_live_demo.sh` launcher delegates the normal Live Demo start to `local_stack.py`.
  - Evidence (source): `local_stack.py` `main` is the direct local-stack controller entry point.

## Product vocabulary

- [ ] **Blueprint Course**: A reusable course used to create **Course Instances**. It has no enrolled **Students** or deadlines.
  - Mismatch: `schemas/base_schema/blueprints.sql` `ple_data.blueprint_course` establishes reusable Blueprint storage, but direct evidence has not established every stated absence or Course Instance creation behavior together.
- [x] **Blueprint Revision**: A fixed version of a **Blueprint Course** preserved so its content cannot change.
  - Evidence (source): `schemas/base_schema/blueprint_revision_integrity.sql` `blueprint_course_revision_is_immutable` invokes `reject_blueprint_revision_change`.
- [ ] **Course Instance**: A course used for teaching. It has **Students**, deadlines, releases, and other course settings. It may be created from a Blueprint Course or started empty.
  - Evidence (source): `schemas/base_schema/course_core.sql` `ple_data.course_instance` stores teaching Courses, and `crates/learning-data-access/src/course_instance.rs` accepts Empty or exact Adopted creation sources.
  - Evidence (runtime): Bounded private actual HTTP and exact-main browser proof created and read an Empty Instructor Course Instance without Blueprint discovery; Student creation was denied and successful API responses were `no-store`.
  - Mismatch: The combined Student membership, deadline, release, Adopted content, and direct started-empty Assessment delivery lifecycle has not been verified.
- [ ] **Published Question**: A validated question in the global **Question Library**, available to vetted **Instructors**.
  - Mismatch: `schemas/base_schema/question_authoring_operations.sql` `ple_api.list_question_library_entries` lists published entries, but the audited evidence does not establish validation and vetted-Instructor availability together.
- [ ] **Draft Question**: A private question being developed by an **Instructor**. It must pass validation before publication.
  - Mismatch: `crates/server/src/question_publication.rs` `QuestionPublicationService::publish` verifies the stored Draft Question source record, but no cited direct evidence establishes content validation before publication; `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` alone does not establish private Instructor-only persistence.
- [ ] **Question Library**: The global collection of Published Questions and published Question Pools available to vetted **Instructors**.
  - Mismatch: `schemas/base_schema/question_authoring_operations.sql` `question_library_entries` and `src/api/question_library_repository.ts` `QuestionLibraryRepository` establish published Question Library entries and their search contract, but do not establish published Question Pool inclusion or availability only to vetted Instructors.

- [x] **User Roles**:
  - Evidence (source): `schemas/base_schema/accounts.sql` `product_role` limits the product role to student, instructor, or sysadmin.
  - [ ] **Sysadmin**: A PLE administrator who manages the system, approves **Instructors**, creates accounts, and helps manage courses.
    - Mismatch: `schemas/base_schema/accounts.sql` `ple_api.create_instructor_account` proves a Sysadmin actor can create an Instructor account, but it does not establish system management, Instructor approval or identity vetting, or course-management help.
  - [ ] **Instructor**: An approved user who teaches courses and can browse, reuse, create, fork, and publish Questions.
    - Mismatch: `crates/server/src/question_publication.rs` `publish_new_question` proves only the authorized publication boundary; it does not directly verify the approved Instructor's browse, reuse, create, and fork capabilities.
  - [ ] **Student**: A user enrolled in a **Course Instance** who completes Assessments and other course activities.
    - Mismatch: browser routes and DTOs still call the graded object `assignment`, rather than the required Assessment terminology.

- [ ] **Assessment Question Editor**: The **Instructor** editor for selecting, adding, removing, and ordering Questions in an Assessment.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` and its routes call this an Assignment workspace, not the required Assessment Question Editor.
- [ ] **Assessment Properties Editor**: The **Instructor** editor for settings that apply to the whole Assessment, such as dates, scoring, attempts, late work, and what **Students** can see.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` `AssignmentWorkspacePoliciesPage` presents the editor as `Policies`, not Assessment Properties Editor.
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
  - Evidence (source): `schemas/base_schema/course_roster.sql` `course_roster_profile` retains Course-local roster ID and Account link without duplicating Student email.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `list_course_roster` is direct-Instructor-only, while `export_pending_course_invitations` is the sole pending-delivery email projection.
  - Evidence (source): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` removes Course-scoped identifiable records but preserves the global Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 probe on 2026-09-15: the dedicated executor purged Student record, membership, and roster profile while preserving the Account and Course and recording deletion.
  - Decision: Existing category and operation boundaries are the simplest HG-consistent implementation; no field-policy engine is needed.
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
## Interface design

### General interface design

- [x] **Sysadmin** uses tomato red as its role color.
  - Evidence (source): `src/styles/product_role.css` `[data-product-role="sysadmin"]` defines `--ple-role-accent: #ff6347`.
- [x] **Instructor** uses teal green as its role color.
  - Evidence (source): `src/styles/product_role.css` `[data-product-role="instructor"]` defines `--ple-role-accent: #168575`.
- [x] **Student** uses lavender /purple as its role color.
  - Evidence (source): `src/styles/product_role.css` `[data-product-role="student"]` defines `--ple-role-accent: #8861b5`.
- [x] Role colors should be used consistently in role labels and other appropriate interface cues.
  - Evidence (source): `src/styles/product_role.css` `.live-demo-persona-action[data-product-role]` and `.ple-app-ribbon__product-role[data-product-role]` consume the shared role tokens.
- [x] Demo role selection should clearly state both the user's role and name.
  - Evidence (test): `tests/playwright/e2e_live_demo_authoring_browser.mjs` selects `Assume the role of Instructor Dr. Elena Rivera`.
- [x] Instructor and **Sysadmin** workflows should work well in a 1280 by 800 desktop browser viewport.
  - Evidence (source): `tests/playwright/ui_corpus_manifest.ts` `RIBBON_RESPONSIVE_PROFILES` and `SYSADMIN_DESKTOP_CONTEXT_OPTIONS` declare 1280 by 800 desktop contexts for both staff roles.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` verifies the Instructor desktop shell and `assertSysadminDesktopRibbon` verifies the Sysadmin Ribbon has no overflow with Instructor Accounts and Scoped Support visible.
  - Evidence (source): `src/pages/role_home_pages.tsx` `SysadminHomePage` presents the backed Instructor Accounts and Scoped Support operations reached by the checked Sysadmin desktop model.
  - Decision: A one-time real-shell keyboard/page probe for Sysadmin Instructor Accounts and Scoped Support passed and was removed rather than retained as a permanent page-script test. The permanent responsive evidence is role/viewport behavior, not a fixed page sequence.
- [ ] Design around what users need to find and do.
  - Mismatch: No repository-wide behavioral or usability evidence establishes this broad design outcome.
- [ ] Important information should stand out from supporting information.
  - Mismatch: Local visual hierarchy exists, but no system-wide implementation evidence verifies this outcome.
- [ ] Related information should be visually grouped and aligned.
  - Mismatch: No repository-wide visual audit verifies this across PLE pages.
- [ ] Similar pages should place similar controls in consistent locations.
  - Mismatch: No cross-page implementation evidence verifies the whole-product requirement.
- [ ] Primary actions should be easy to find and appear near the content or workflow they affect.
  - Mismatch: No whole-product browser or usability evidence verifies this broad requirement.
- [ ] Avoid scattering related actions across page headers, menus, navigation, and content areas.
  - Mismatch: Current top-bar Sign Out contradicts the specified Profile-menu location.
- [x] PLE often presents large collections where users need to find a few relevant items.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders the Question Library collection surface.
- [ ] Optimize large collections for scanning, searching, filtering, and comparison.
  - Mismatch: The broad all-collection outcome lacks complete implementation evidence.
- [ ] Show enough useful information at once to support comparison without excessive scrolling.
  - Mismatch: No viewport-based collection comparison evidence verifies this requirement.
- [ ] Search and filters should help users quickly narrow large collections.
  - Mismatch: Existing controls do not establish the stated quick-narrowing outcome across all large collections.
- [ ] Dense pages should remain easy to scan.
  - Mismatch: No usability or whole-product visual evidence establishes scanability.
- [ ] Use spacing to separate meaningful groups rather than simply making pages spacious.
  - Mismatch: No systematic visual evidence verifies the required rationale across pages.
- [ ] Prefer alignment, typography, and dividers over unnecessary cards, boxes, borders, and nested containers.
  - Mismatch: Current pages include cards and borders; no audit establishes the preference is followed.
- [ ] Keep the visual design compact, flat, information dense, and consistent across PLE.
  - Mismatch: No complete rendered-product audit verifies all four whole-product attributes.
- [ ] Dream big on the UI. Choose one visual philosophy and carry it through the entire interface.
  - Mismatch: No repository evidence can verify this whole-product qualitative outcome.
- [ ] Use drag-and-drop where it makes reordering faster and more natural.
  - Mismatch: No implemented drag-and-drop reordering surface was found in the audited shell evidence.
- [x] Reordering must also have a precise keyboard-accessible method.
  - Evidence (source): `src/features/blueprint_course/blueprint_assessment_content_editor.tsx` `moveEntry` and `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `move` back their labelled native-button Move earlier and Move later controls.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_choice_list.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multiple_answer_editor.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multi_fill_in_editor.tsx` `onMoveBlank`, `src/features/ple_question_json_authoring/question_json_matching_editor.tsx` `onMoveItem`, and `src/features/ple_question_json_authoring/question_json_ordering_editor.tsx` `onMoveItem` give every native JSON reorderer the same precise buttons.
  - Evidence (test): `tests/test_blueprint_course_model.mjs` `reusable entries preserve fixed and Question Pool interleaving`, `tests/test_ple_question_json_editor_model.mjs` `choice edits retain semantic IDs and enforce choices and correct-answer invariants`, `tests/test_ple_question_json_multiple_answer_editor.mjs` `multiple-answer text edits and reordering retain choice IDs and exact correct IDs`, and `tests/test_ple_question_json_multi_fill_ordering_authoring.mjs` `ORDER treats Ordering Items as the source of truth and derives correctOrder after movement` protect the stable reorder results.
  - Decision: The one-time seven-surface keyboard-control inventory passed and was removed rather than becoming a permanent implementation-inventory test. It does not select drag-and-drop surfaces, which remains the separate Human Guidance product question.
- [x] Themes should use biome and habitat names.
  - Evidence (source): `src/features/course_appearance/course_theme_registry.ts` `COURSE_THEME_REGISTRY` retains stored ID `grass` and its unchanged anchors while presenting `Grassland`; the same closed registry presents Forest, Ocean, Desert, and the remaining habitat names.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `COURSE_THEME_OPTIONS` renders each visible theme label from `option.tokens.name`, not its stored ID.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `Grassland uses the Roosevelt-inspired anchors and accessible derived actions` verifies the `grass` ID presents Grassland without changing its reviewed palette; `every reviewed theme resolves to complete, contrast-safe course tokens` covers the closed registry.
- [ ] Implement the themes as specified in `docs/BIOME_THEME_PALETTES.md`
  - Mismatch: `src/features/course_appearance/course_theme_registry.ts` still implements the current 15-ID, three-light-anchor registry, while `docs/BIOME_THEME_PALETTES.md` specifies a proposed 25-theme light/dark four-color registry and records six unresolved product decisions that block an atomic runtime cutover.
- [ ] UUIDs should never appear in visible content, navigation URLs, or copyable links.
  - Mismatch: `tests/test_public_navigation.mjs` `human route references are compact, typed, and bounded` checks route references only; it does not establish the absence of UUIDs from visible content or copyable links.
- [x] Use [Atkinson Hyperlegible Next](https://www.brailleinstitute.org/freefont/) as the main PLE font.
  - Evidence (source): `src/style.css` `:root` sets `Atkinson Hyperlegible Next` as the first font family.
- [x] Use [Atkinson Hyperlegible Mono](https://www.brailleinstitute.org/freefont/) for code and other monospace text.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-mono` applies the local `Atkinson Hyperlegible Mono` family to `code`, `kbd`, `pre`, and `samp` through normal and italic `@font-face` declarations.
  - Evidence (source): `pipeline/build.mjs` `BROWSER_FONT_BUNDLES` copies and verifies the Mono assets in the production `dist` output.
  - Decision: One-time normal-and-italic computed-style proof passed and was removed; the behavior does not retain a permanent implementation-coupled test.
- [x] Prefer the official Braille Institute font files and include the needed weights locally with PLE.
  - Evidence (source): `src/styles/browser_fonts.css` `@font-face` loads local Atkinson Hyperlegible Next variable font files.
- [x] When a narrow font is needed, use `IBM Plex Sans Condensed` for long unbreakable strings such as URLs.
  - Evidence (source): `src/styles/browser_fonts.css` `--ple-font-narrow` declares the local `IBM Plex Sans Condensed` face and applies it only to the PLE Question JSON editor's Citation URL input; `pipeline/build.mjs` `BROWSER_FONT_BUNDLES` copies and verifies that same-origin asset.
  - Evidence (runtime): `src/features/ple_question_json_authoring/question_json_editor_styles.ts` `PLE_QUESTION_JSON_EDITOR_STYLES`: a one-time Chromium fixture imported the actual injected editor CSS against production-built local font assets on 2026-09-15; at 360 and 1280 CSS pixels in light and dark OS preferences it requested the local font, computed the narrow family on the Citation URL input, and retained normal input value, horizontal-scroll, and overflow behavior.
- [x] With `IBM Plex Sans Condensed`, try `font-variant-numeric: slashed-zero` to better distinguish `0` from `O`.
  - Evidence (source): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero` applies it to that narrow Citation URL input; `src/assets/fonts/ibm_plex_sans_condensed/provenance.txt` records the locally retained IBM Plex Sans Condensed Regular asset, OFL provenance, and its verified OpenType `zero` GSUB feature.
  - Evidence (runtime): `src/styles/browser_fonts.css` `font-variant-numeric: slashed-zero`: that same one-time Chromium fixture computed `slashed-zero` on the local IBM face without a fallback; it was removed rather than retained as a permanent implementation-coupled test.
- [ ] Question Backend-rendered content may use its own fonts when needed for correct display.
  - Mismatch: No Question Backend font-isolation implementation evidence was found in the shell audit.
- [ ] Students should have no upload capabilities. Instructor-created content should use text boxes.
  - Mismatch: Student upload denial is not sufficient to verify the universal Instructor text-box requirement.

### Ribbon and page layout

- [x] The top Ribbon is the persistent navigation area for signed-in PLE pages.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` renders `AppRibbon` inside the persistent shell.
- [x] The Ribbon should remain in the same location and use the same overall structure while navigating.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` verifies declared Ribbon rows across route-model changes.
- [x] Navigation choices should remain in predictable locations as users move between related pages.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` and `RIBBON_TASK_CATALOG` own fixed navigation identities.
- [x] Changing a Ribbon selection changes the content below the Ribbon without moving the main content area up or down.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` and the `application data does not move the content origin` assertion verify the content origin.
- [x] Ribbon rows should keep their space when needed so changing selections does not make the content area jump.
  - Evidence (test): `tests/playwright/ribbon_geometry_evidence.mjs` `chromeAboveContent` verifies reserved row tokens and shell track geometry.
- [x] Page actions should appear near the content they affect rather than changing the Ribbon layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` renders only catalog navigation and Sign Out in `AppRibbon`; task content stays in `ApplicationShell` content.
- [x] See **User top bar** and **Breadcrumbs** for the persistent elements that make up the top of the page.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` composes `AppRibbon` and `BreadcrumbPrelude`.

### User top bar

- [x] All signed-in users share the same basic top bar layout.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `AppRibbon` renders one top-bar structure from each role model.
- [x] The top bar remains in a consistent location as users navigate.
  - Evidence (test): `tests/playwright/ribbon_m9_responsive_evidence.mjs` `assertResponsiveRows` measures the persistent top row across model changes.
- [x] The PLE logo and product name appear at the upper left and link to the user's home dashboard.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__brand` is the leading `href="/"` Peptidyle home link.
- [x] Each Product Role has its own home dashboard and navigation.
  - Evidence (source): `src/route_contract.ts` `productRoleHomeRouteId` declares one role-scoped home route for Instructor, Student, and Sysadmin; `src/ribbon/ribbon_contract.ts` `hrefFor` directs each Product Role's selected Courses navigation to that route.
  - Evidence (test): `tests/test_ribbon_route_contract.mjs` `each Product Role has an explicit selected Courses home route` verifies the role-scoped route, selected Courses navigation, and matching link for every Product Role.
  - Generated evidence stale: `docs/screenshots/current_capture_manifest.json` marks the three new role-home routes `deferred`; it is not visual proof until a fresh Live Demo capture.
- [x] Product Role appears once next to the PLE name.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__product-role` occurs once in the shared identity block beside `ple-app-ribbon__brand`.
- [x] Role-specific navigation appears between the product identity and Profile.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` places `ple-app-ribbon__tabs` after identity and before account controls.
- [x] Profile appears at the far right as an icon-only avatar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `ple-app-ribbon__profile-endcap` renders the shared icon-only Profile button after the navigation region; `src/ribbon/app_ribbon.css` `ple-app-ribbon__profile-endcap` anchors that endcap at the inline end.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in Product Role has one accessible generic Profile end control` verifies the one accessible, text-free Profile control for Student, Instructor, and Sysadmin.
  - Decision: A one-time real-shell probe verified the isolated Profile control at 1280 and 320 CSS pixels with a coarse pointer, including thumbnail-request isolation; it was removed rather than retained as a permanent browser test.
- [x] Clicking the Profile avatar opens the Profile menu.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `openProfileMenu` controls the Profile trigger's `profileMenuOpen` state and renders the labelled `ple-profile-menu` menu.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` protects pointer and keyboard opening, focus, dismissal, and action dispatch.
- [ ] The Profile menu contains Profile settings, account settings, and Sign Out.
  - Mismatch: No Profile menu is implemented.
- [x] Sign Out belongs in the Profile menu rather than the main top bar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `data-ribbon-action={props.model.context.signOutAction.id}` renders Sign Out as a Profile-menu item and closes that menu after dispatch.
  - Evidence (test): `tests/playwright/ribbon_profile_menu_contract.mjs` `Ribbon Profile menu contract: PASS` verifies no top-bar Sign Out button and one dispatched Profile-menu Sign Out action.
- [x] The Profile avatar uses a generic user avatar until the user selects another avatar.
  - Evidence (source): `src/ribbon/app_ribbon.tsx` `renderProfileAvatar` defaults to the generic `RibbonIcon` and records `data-ribbon-profile-avatar="generic"`; `src/application_shell.tsx` `renderProfileAvatar` supplies the selected-avatar renderer only when the application shell has one.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `every signed-in Product Role has one accessible generic Profile end control` verifies the generic circle-user fallback for all signed-in Product Roles.
- [ ] **Students** select avatars from a PLE-provided collection and cannot upload Profile images.
  - Mismatch: No Student avatar collection or selection UI was found.
- [ ] Student avatar selection should be visual and playful, similar to choosing a LEGO avatar.
  - Mismatch: No Student avatar selection UI was found.
- [ ] **Instructors** and **Sysadmins** may select a provided avatar or add their own Profile image.
  - Mismatch: Instructor image upload exists, but Sysadmin profile/avatar support and provided-avatar selection were not found.
- [ ] The current avatar appears consistently anywhere PLE represents that user.
  - Mismatch: No cross-surface all-role avatar consistency evidence was found.
- [x] Instructor Profile includes the Instructor's time zone and profile image.
  - Evidence (source): `src/pages/profile_page.tsx` `ProfilePage` renders the time-zone value and Profile image controls.
- [x] Profile images may use any reasonable aspect ratio and are cropped to a consistent rounded square.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-app-ribbon__profile img` uses `object-fit: cover` within the fixed rounded profile box.
- [x] See **Ribbon and page layout** for the overall navigation and page-position rules.
  - Evidence (source): `src/application_shell.tsx` `ApplicationShell` is the shared shell that composes the top bar and content region.

### Breadcrumbs

- [ ] All signed-in users have a permanent breadcrumb row below the top Ribbon.
  - Mismatch: `src/application_shell.tsx` `BreadcrumbPrelude` renders only when `breadcrumbPreludeReserved` is true.
- [ ] The breadcrumb row remains in the same location and keeps the same space as users navigate.
  - Mismatch: Breadcrumb space is conditional on `breadcrumbPreludeReserved`, not permanent for every signed-in route.
- [ ] Breadcrumbs show the path from the user's home dashboard to the current page.
  - Mismatch: No audit evidence proves every route's breadcrumb path begins at the role home dashboard.
- [ ] Each breadcrumb level links back to its corresponding page.
  - Mismatch: `src/application_shell.tsx` allows a breadcrumb without `href` to render as a span.
- [x] Breadcrumbs use human-readable names rather than internal identifiers.
  - Evidence (source): `src/application_shell.tsx` `BreadcrumbPrelude` renders `RibbonBreadcrumbModel.label`, not an ID.
- [x] Course and Assessment breadcrumbs preserve the current Course context.
  - Evidence (source): `src/ribbon/route_scope_controller.ts` `createRouteScopeController` resolves route labels while retaining course scope.
- [x] Keeping the breadcrumb row in place prevents the main content from moving up or down as breadcrumb depth changes.
  - Evidence (test): `tests/playwright/ribbon_m10_shell_evidence.mjs` `label resolution preserves the reserved breadcrumb-prelude geometry` verifies stable shell geometry through deferred resolution.
- [x] See **Ribbon and page layout** for the overall page-position rules.
  - Evidence (source): `src/ribbon/app_ribbon.css` `.ple-shell__breadcrumb-prelude` documents and implements the shell-owned stable prelude.
### Instructor interface

- [ ] The Instructor interface should make frequent teaching tasks fast and easy to find.
  - Mismatch: the main Instructor task areas still contain deferred destinations and no end-to-end usability evidence establishes this broad workflow claim.
- [ ] The Instructor menu has **Courses**, **Questions**, and **Assessments** in one dense top bar.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` labels the third tab "Assignments," not the required "Assessments."
- [x] Instructor Profile uses a generic user icon until the **Instructor** adds a Profile image.
  - Evidence (source): `src/features/profile_avatar/ribbon_account_avatar.tsx` `RibbonAccountAvatar` falls back to `RibbonIcon` `circle-user` when no Profile image or provided avatar exists.
- [ ] All required ribbon choices remain visible even when their collection is empty.
  - Mismatch: several required choices have `future` destinations in `src/ribbon/ribbon_catalog.ts` and are not admitted as usable controls.
- [x] A working navigation destination remains visible when its collection is empty.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` retains the courses route and renders an empty state.
- [x] A future or unavailable capability should not appear as a usable control until its workflow exists.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `RibbonDestination` distinguishes `route` from `future` destinations.
- [ ] Empty collection pages should explain what the collection is for and provide an obvious action to create or add the first item when the user can do so.
  - Mismatch: `src/pages/library_page.tsx` empty Question Library results offer no action to add or create a first item.
- [ ] Similar pages should place similar actions in consistent locations.
  - Mismatch: no cross-page layout contract or test verifies consistent action placement.
- [ ] Instructor pages should be composed around the teaching task rather than collections of padded components.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_live_page.tsx` presents Assignment-named tasks, and no behavior or rendered-layout evidence verifies this broad Instructor-interface judgment.
- [ ] Instructor Course and Assessment lists should be dense and easy to scan, more like a spreadsheet than cards.
  - Mismatch: `src/pages/course_list_page.tsx` has a dense Course Instance row and `src/pages/assessments_due_soon_page.tsx` has an Assessment list, but no accepted visual evidence establishes the shared spreadsheet-like scanning judgment.
- [ ] Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_student_view_page.tsx` `AssessmentWorkspaceStudentViewPage` renders the server-authorized answer-free Assessment projection, loads only the manifest and selected Question, disables native response controls, and provides no Student Work, Assessment Attempt, submission, or grade action.
  - Evidence (source): `crates/server/src/assessment_student_view.rs` `assessment_student_view_router`, `crates/learning-data-access/src/postgres/assessment_student_view.rs` `PostgresInstructorStudentViewStore`, and `schemas/base_schema/assessment_student_view.sql` `load_instructor_student_view_question_source` implement the authorized no-write server, Store, and SQL boundaries.
  - Evidence (runtime): a fresh PostgreSQL proof exercised the real Store through the API roles with a nonempty Ready Asset rendition and verified read-only SQLSTATE `25006` plus zero writes to Student-state tables.
  - Evidence (test): an independently reviewed Chromium component proof with mock transport covered native and WeBWorK presentations, navigation, disabled controls, stale and error recovery, and no mutation requests; it was not connected or live-stack acceptance.
  - Mismatch: connected live-HTTP acceptance is still missing; the unchanged full server compile is blocked in the AWS dependency graph; and production iMathAS Student View integration remains deferred outside the pilot. Independent final server source review passed, but it does not establish runtime behavior.

#### Courses

- [ ] The **Courses** ribbon must include: My Blueprint Courses, My Active Courses, My Inactive Courses, Search Public Blueprint Courses.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` declares all four labels, but three are `future` destinations rather than working ribbon destinations.
- [x] Course lists should support scanning and comparison without opening each Course.
  - Evidence (source): `src/pages/course_list_page.tsx` `CourseInstanceRow` renders each Course Instance name, term, theme, and open action in the list.
- [ ] The Course Editor should show the Course structure and its ordered Assessments without showing every Question at once.
  - Mismatch: `src/pages/course_instance_page.tsx` is an Assignment list rather than the required Course Editor and uses the retired Assignment product term.
- [ ] Selecting an Assessment in the Course Editor opens that Assessment for editing.
  - Mismatch: Course Instance rows link to Assignment routes; no Assessment-named Course Editor exists.
- [ ] Assessment content and Assessment properties should remain separate editing tasks.
  - Mismatch: the implemented separation is for the retired Assignment object in `src/pages/assignment_workspace/`, not the HG Assessment object.
- [ ] My Active Courses and My Inactive Courses should both be available from the Courses area.
  - Mismatch: both controls are `future` destinations in `src/ribbon/ribbon_catalog.ts`.

##### Blueprint Courses

- [x] **My Blueprint Courses** should emphasize reusable course design rather than teaching activity.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCoursesWorkspace` presents reusable Blueprint Course content and adoption information.
- [ ] **Search Public Blueprint Courses** helps Instructors find a Blueprint Course they already have in mind.
  - Mismatch: `searchPublicBlueprintCourses` is a `future` destination in `src/ribbon/ribbon_catalog.ts`.
- [ ] Public Blueprint Course search should support quickly narrowing a large collection.
  - Mismatch: no Public Blueprint Course search route or query controls exist.
- [x] A **Blueprint Course** should provide an obvious action for creating a **Course Instance** from it.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` renders "Create Course Instance from this Blueprint."

###### Blueprint Course editing

- [ ] Blueprint Course editing should follow Course Editor -> Blueprint Assessment Editor.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` opens a Course Editor but names the selected editor a Blueprint Assignment editor, not a Blueprint Assessment Editor.
- [x] The Course Editor should show the Blueprint Course structure without editing every Question on one page.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` lists modules and assignments before selecting an editor.
- [ ] Selecting a Blueprint Assessment in the Course Editor opens the editor for that Blueprint Assessment.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` uses `setSelectedAssignment` and a Blueprint Assignment editor rather than the required Blueprint Assessment.
- [ ] Only the selected Blueprint Assessment's Questions should appear in its editor.
  - Mismatch: the selected scope in `src/features/blueprint_course/blueprint_course_workspace.tsx` is a `BlueprintAssignmentContentEditor`, not a Blueprint Assessment editor.
- [ ] **Blueprint Assessment Question Editor**: Selects, adds, removes, and orders Questions in a Blueprint Assessment.
  - Mismatch: the implemented editor calls the product object a Blueprint Assignment, not a Blueprint Assessment.
- [ ] **Blueprint Assessment Properties Editor**: Controls scoring, attempts, late work, and what **Students** can see.
  - Mismatch: `src/features/blueprint_course/blueprint_assignment_content_editor.tsx` supplies reusable defaults but not the required separate Blueprint Assessment Properties Editor.
- [x] Blueprint Courses should not contain Assessment dates or relative Assessment schedules.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `BlueprintCourseDetailWorkspace` describes reusable structure without deadlines or course delivery settings.

###### Blueprint Course lifecycle

- [ ] Blueprint Courses follow the lifecycle **Private -> Public -> Archived**.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` exposes only available/archive states, not the required Private/Public lifecycle.
- [ ] New and forked Blueprint Courses start **Private**.
  - Mismatch: no verified fork workflow or Private initial availability appears in the Blueprint Course UI.
- [ ] Private Blueprint Courses are visible only to their owner.
  - Mismatch: source evidence identifies `blueprint_course_owner` read access but does not verify the required Private visibility behavior.
- [ ] Instructors may develop and use Private Blueprint Courses without publishing them.
  - Mismatch: no Private availability workflow is implemented in the visible Blueprint Course controls.
- [ ] Making a Blueprint Course **Public** adds it to the shared Blueprint Course collection.
  - Mismatch: no Public transition or shared Public collection is implemented.
- [ ] Public Blueprint Courses are visible to all **Instructors**.
  - Mismatch: no verified Public Blueprint Course interface or shared-collection workflow exists.
- [ ] A Public Blueprint Course with no adoptions may return to **Private**.
  - Mismatch: no Public-to-Private transition is implemented.
- [ ] A Public Blueprint Course with one or more adoptions remains **Public**.
  - Mismatch: no Public availability model is implemented.
- [x] Archived Blueprint Courses leave normal discovery but remain available where needed for history.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `archive_blueprint` and discovery query comments retain historical pins while excluding archived discovery.
- [ ] Archived Blueprint Courses remain viewable when accessed directly or through their history.
  - Mismatch: no accepted direct-or-history UI workflow verifies Archived Blueprint Course visibility.
- [ ] Blueprint Courses do not have a separate Draft state.
  - Mismatch: no source-level lifecycle test establishes the absence of a Draft state across Blueprint Course APIs and UI.
- [ ] Public Blueprint Courses and their Revision history are visible to all **Instructors**.
  - Verification pending: ordinary current Public detail reads are verified; full Revision history visibility is not established.
- [ ] Archived Blueprint Courses and their Revision history remain visible to all **Instructors**, but
  do not appear in normal Blueprint Course discovery.
  - Verification pending: accepted Archived discovery/detail proof establishes default exclusion and known-detail visibility, not full Revision history.
- [ ] Blueprint Course visibility includes its content, Revision history, and recorded changes.
  - Verification pending: current content reads are verified, but complete Revision history and recorded-change presentation remain unverified.
- [ ] The owning **Instructor** controls changes to a Blueprint Course; visibility does not grant
  editing authority.
  - Verification pending: prior C883 owner/nonowner Apply denials are contributor evidence; complete owner mutation boundaries and connected workflow need current-authority verification.

###### Blueprint Course forks and changes

- [ ] Instructors may fork a Public Blueprint Course to continue development privately.
  - Mismatch: no Public Blueprint Course fork action or Private fork lifecycle is implemented.
- [ ] Forking a Blueprint Course creates an independent Private Blueprint Course owned by the Instructor who created the fork.
  - Mismatch: no authorized fork creation workflow or persisted fork-owner presentation is verified.
- [ ] Forking a Blueprint Course creates new Blueprint Assessments populated with the same Published
  Question IDs and forks of the source Question Pools.
  - Mismatch: Independent Question Pool forks are not yet implemented; historical pin-preserving evidence does not verify this fork behavior.
- [ ] Forked Question Pools preserve the Published Question IDs contained in their source Question Pools.
  - Mismatch: Independent Question Pool forks are not yet implemented; historical pin-preserving evidence does not verify this behavior.
- [ ] A Blueprint Course fork records the Blueprint Course and Revision it was forked from.
  - Mismatch: no accepted lineage read or fork-creation evidence verifies the recorded source Course and Revision.
- [ ] Blueprint Course forks develop independently after they are created.
  - Mismatch: no created-fork workflow verifies independent later Revision development.
- [x] A Blueprint Course shows its known forks and the **Instructor** who owns each fork.
  - Evidence (runtime): accepted C881 actual-server proof at `/private/tmp/ple-fork-reader-artifacts.nRikDO` exercises `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks`, returns each fork's owner and recorded origin, and conceals unrelated Instructors with `404 no-store`.
  - Evidence (source): `crates/server/src/blueprint_course/known_forks.rs` `list_known_forks` and `src/features/blueprint_forks/blueprint_fork_review.tsx` `BlueprintKnownForks` present authorized known-fork rows and owner names.
- [ ] A fork does not automatically receive later changes from its source Blueprint Course.
  - Mismatch: no created-fork runtime evidence verifies that later source changes leave the fork unchanged.
- [ ] PLE should make it clear when a source Blueprint Course has newer Revisions than its forks.
  - Mismatch: current-head review evidence does not verify an obvious source-newer indication in the fork workflow.
- [ ] PLE should make newer Revisions in downstream forks visible from their source Blueprint Course.
  - Mismatch: known-fork rows expose current Revision numbers, but no accepted newer-downstream indication proof establishes this behavior.
- [ ] The fork owner decides whether to incorporate source changes into the fork.
  - Verification pending: C883's explicit owner-authorized selective-save backend is contributor-verified; connected Instructor Apply evidence remains open under C884/C413.
- [ ] PLE should make it easy for the fork owner to incorporate selected source changes into the fork.
  - Verification pending: C883 backend evidence does not establish an easy connected Instructor selection and Apply workflow.

###### Blueprint Course comparison

- [ ] Any **Instructor** can compare related Blueprint Courses in the same fork lineage when those
  Blueprint Courses are visible to that Instructor.
  - Mismatch: `crates/server/src/blueprint_course/fork_review.rs` `load_fork_review` reviews a fork with its direct source only; accepted C882 proof does not cover other visible related pairs in the same lineage.
  - Evidence (runtime): accepted C882 actual HTTP proof at `/private/tmp/ple-fork-review-http-artifacts.LTUgsF` remains useful direct-source evidence: ordinary visibility, GET-only `no-store`, zero `ple_data` mutations, lazy review and retry at 1280 by 800 and initially 390px.
- [x] Fork comparison normally compares the newest Revision of the source Blueprint Course with the
  newest Revision of the fork.
  - Evidence (runtime): C881 records corrected current heads, including source equal to origin; C882 exercises `src/api/decoders/blueprint_course.ts` `decodeBlueprintComparisonView` to return current source and fork names, ETags, and revisions.
  - Evidence (source): `src/api/decoders/blueprint_course.ts` `decodeBlueprintComparisonView` requires the current source and fork Revision references.
- [ ] Older Revisions remain available through Blueprint history but are not the normal comparison
  workflow.
  - Mismatch: current-head review exists, but no accepted Blueprint history workflow establishes older-Revision availability.
- [x] Blueprint Course differences are calculated from canonical JSON when the Instructor requests
  the comparison.
  - Evidence (source): `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` requests C880's canonical comparison projection on demand rather than persisting comparison state.
  - Evidence (source): `crates/question_model/src/blueprint_course/fork_comparison.rs` `compare_blueprint_courses` compares canonical Blueprint snapshots.
  - Evidence (runtime): C882's actual HTTP receipt exercises `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` as a real GET-only `no-store` review with zero `ple_data` mutations.
- [ ] Shared Question IDs provide the durable content relationships between compared Blueprint Courses.
  - Mismatch: C880's `compare_blueprint_fork` matches Blueprint Assessments by internal Assessment IDs; retained-ID evidence is not proof of Question-ID content relationships.
- [ ] Blueprint Assessments are matched by the shared Question IDs they contain.
  - Mismatch: `crates/question_model/src/blueprint_course/fork_comparison.rs` uses internal Assessment IDs instead of matching Assessments by shared Question IDs.
- [ ] Blueprint Course comparison does not require Blueprint Assessment identity or history across
  forks; Assessment relationships are determined from the shared Question IDs they contain.
  - Verification pending: the new Assessment and Pool fork pair needs proof that comparison uses shared Published Question IDs without Blueprint Assessment identity or history across forks.
- [ ] Comparison should show shared, added, and removed Assessments and Question IDs, plus changed
  content where those differences can be determined from canonical JSON.
  - Mismatch: accepted C880/C882 evidence covers internal-ID source-only/fork-only changes, not the required shared-Question-ID Assessment and Question correspondence.
- [ ] Comparison should remain useful when Assessment names, order, or structure have changed.
  - Verification pending: previous module-parent, name and order evidence remains useful, but does not verify shared-Question-ID matching after those changes or independently assigned Assessment IDs.
- [x] Comparison visibility follows Blueprint Course visibility rather than fork ownership.
  - Evidence (source): `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` uses ordinary Blueprint visibility for read-only direct-source review.
  - Evidence (runtime): C881/C882 accepted `crates/server/src/blueprint_course/fork_review.rs` `load_comparison` ordinary-visibility direct-source review at `/private/tmp/ple-fork-reader-artifacts.nRikDO` and `/private/tmp/ple-fork-review-http-artifacts.LTUgsF` permits visible Public/Archived sides and conceals unauthorized Private sides; ownership restricts Apply, not comparison.

##### Course Instances

- [ ] **My Active Courses** should emphasize Course Instances the Instructor is currently teaching.
  - Mismatch: `myActiveCourses` is a `future` Ribbon destination.
- [ ] Active Course Instances should make upcoming Assessments and important course activity easy to find.
  - Mismatch: no active-only Course Instance view exists and the UI uses Assignment rather than Assessment.
- [ ] **My Inactive Courses** should keep past Course Instances available without competing with active Course Instances.
  - Mismatch: `myInactiveCourses` is a `future` Ribbon destination.
- [ ] Creating a Course Instance from a Blueprint Course preserves its Assessments, Questions, pools, and settings.
  - Mismatch: the creation path and `tests/e2e/e2e_live_demo_course_instance.sh` adopt Blueprint Assignments; they do not verify the required Assessment-named contents and settings contract.
- [ ] Assessments created from a Blueprint Course start unreleased with dates unset.
  - Mismatch: `tests/e2e/e2e_live_demo_course_instance.sh` verifies an adopted Assignment, not the required Assessment product behavior.
- [ ] A Course Instance represents one teaching period and remains Active for at most six months from
  creation.
  - Mismatch: `crates/question_model/src/course_term.rs` `CourseTerm::new` only requires an end date not before the start date; no implementation establishes a six-month Active limit from Course Instance creation.
- [ ] Course banners use a 5:1 aspect ratio.
  - Mismatch: `src/features/course_appearance/course_entry_identity.tsx` `COURSE_ENTRY_IDENTITY_STYLES` declares `aspect-ratio: 6 / 1`, not 5:1.
- [ ] 1280 by 256 pixels is the recommended Course banner authoring size.
  - Mismatch: the checked banner presentation in `src/features/course_appearance/course_entry_identity.tsx` uses a 1200-by-200 6:1 rendition; no 1280-by-256 authoring recommendation exists.
- [ ] Higher-resolution 5:1 Course banner images are supported.
  - Mismatch: the cited `tests/e2e/e2e_course_appearance.sh` does not contain `course_banner_upload` or an assertion that a higher-resolution 5:1 upload is accepted.
- [ ] PLE responsively scales Course banners while preserving their aspect ratio.
  - Mismatch: `tests/playwright/e2e/course_appearance_propagation.spec.ts` verifies upload, persistence, and visibility at one 1280px viewport, not responsive scaling or aspect-ratio preservation across viewports.
- [ ] Course banners appear as small centered banners rather than full-width page heroes.
  - Mismatch: `src/features/course_appearance/course_entry_identity.tsx` has a centered 6:1 entry banner, but no rendered evidence verifies the required visual comparison with a full-width page hero.
- [x] An Instructor can upload a Course banner and select a three-color theme.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `AppearanceBannerEditor` contains `saveBanner`, which calls `uploadCourseBanner` and `setCourseBanner`; `AppearanceThemeEditor` selects a `COURSE_THEME_OPTIONS` theme.
  - Evidence (source): `src/features/course_appearance/course_theme_registry.ts` `ThemeAnchors` defines each theme's canvas, secondary, and accent colors.
  - Evidence (test): `tests/playwright/e2e/course_appearance_propagation.spec.ts` `Instructor saves both properties and enrolled Student receives only that Course appearance` independently saves, reloads, and displays a Course theme and banner.
- [ ] Course Instance Assessments have two editors:
  - Mismatch: the UI names the object Assignment, not Assessment.
  - [ ] **Assessment Question Editor**: Selects, adds, removes, and orders Questions in an Assessment.
    - Mismatch: the implemented `AssignmentWorkspaceQuestionsPage` is not an Assessment-named editor.
  - [ ] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and what **Students** can see.
    - Mismatch: the implemented `AssignmentWorkspacePoliciesPage` is not an Assessment-named properties editor.

#### Questions

- [ ] The **Questions** ribbon must include: My Questions, My Draft Questions, Starred, Watched, Search Question Library, Browse Question Library.
  - Mismatch: My Questions, Starred, and Watched are `future` destinations in `src/ribbon/ribbon_catalog.ts`.
- [ ] **My Questions** should make the Instructor's Published Questions easy to find and manage.
  - Mismatch: `myQuestions` is a `future` Ribbon destination.
- [x] **My Draft Questions** should emphasize Questions that still need work before publication.
  - Evidence (source): `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` presents the current Instructor's drafts.
- [ ] **Starred** should provide a quick personal collection of Questions the Instructor wants to keep handy.
  - Mismatch: `starred` is a `future` Ribbon destination.
- [ ] **Watched** should help Instructors follow Questions where changes or activity matter to them.
  - Mismatch: `watched` is a `future` Ribbon destination.

##### Search Question Library

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
- [x] Search results should switch to a dense, information-rich layout.
  - Evidence (source): `src/pages/library_page.tsx` `question-library-row` renders title, summary, authors, identifier, and action per result.
- [x] Results should make it easy to scan many Questions quickly.
  - Evidence (source): `src/pages/library_page.tsx` `questionLibraryBrowseVirtualWindow` virtualizes dense result rows.
- [x] Results should show the information needed to judge relevance without opening each Question.
  - Evidence (source): `src/pages/library_page.tsx` `question-library-row` shows title, summary, authors, and identifier.
- [x] Search results should support filters for narrowing the Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies author, backend, tag, Question Type, license, and capability filters.
- [x] Filters should update the current search rather than start a separate workflow.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` resets one `QuestionLibraryBrowseSession` with the updated query.
- [x] Search should support Google-like syntax for more precise queries.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `pub(super) struct QuestionTextQuery` accepts ordinary AND words, quoted phrases, minus exclusions, PLE field tags, and exact Question IDs with filters through its parser.
  - Evidence (runtime): accepted one-time private PostgreSQL 17 and actual-server HTTP proof exercised `crates/server/src/question_library.rs` `search_questions` with ordinary AND words, a quoted phrase, minus exclusion, and all five PLE fields under an active vetted Instructor; anonymous and Student requests remained concealed and responses were `no-store`. The temporary proof is retained outside Git through documentation acceptance.
- [x] Quoted text should search for an exact phrase.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `term_value` retains quoted text as one exact phrase term.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `term_value` and returned only `Alpha enzyme kinetics` for the quoted phrase `"enzyme kinetics"`.
- [x] A minus sign should exclude matching terms.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `exclusion_prefix` records a leading minus as an excluded search term.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `exclusion_prefix` and returned `Alpha enzyme kinetics` for `enzyme -inhibitor` while excluding the matching inhibitor Question.
- [x] Search should support PubMed-like field tags such as `topic:genetics`.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `field_prefix` recognizes `topic` and `SearchTerm::matches` applies it to Question metadata.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `field_prefix`, composed `subject:`, `topic:`, `tags:`, `type:`, and `author:` in one request, and returned the exact expected Question.
- [x] Field tags should use PLE concepts and vocabulary.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` limits field tags to PLE terms: `subject`, `topic`, `tags`, `type`, and `author`.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `SearchField` with only the closed PLE field vocabulary and preserved the production route's strict query boundary.
- [x] Useful fields may include subject, topic, tags, Question Type, and author.
  - Evidence (source): `crates/server/src/question_library/search_query.rs` `SearchField` and `SearchTerm::matches` support `subject`, `topic`, `tags`, `type`, and `author`.
  - Evidence (runtime): the accepted C58 actual-server HTTP proof exercised `crates/server/src/question_library/search_query.rs` `impl SearchTerm` for subject, topic, tags, Question Type, and author together through the ordinary `text` query parameter.
- [x] Simple and advanced searches should use the same search box.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` provides one Search input, and `crates/server/src/question_library.rs` passes its optional `text` query value to `QuestionTextQuery::parse` before matching.
  - Evidence (runtime): accepted C59 component proof exercised `src/pages/library_page.tsx` `LibraryPage` and confirmed the visible Search box and its normal-flow Search tips.
  - Evidence (runtime): the accepted C58 component and actual-server HTTP evidence exercised `src/pages/library_page.tsx` `LibraryPage` and `crates/server/src/question_library.rs` `search_questions` through the same visible Search input and `text` transport for ordinary words and advanced grammar.
- [x] Instructors should not need to learn search syntax to use Search Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` exposes ordinary search and labeled filter controls without syntax requirements.
- [x] The interface should make useful search syntax discoverable when needed.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders `question-library-search-tips` as a native disclosure beside the ordinary Search box with words, quotes, minus, PLE fields, and examples.
  - Evidence (runtime): accepted corrected desktop `src/pages/library_page.tsx` `LibraryPage` component proof opened Search tips without obscuring filters or bulk controls; the full `./check_codebase.sh` gate passed.
- [ ] Search syntax should help expert users quickly narrow a very large Question Library.
  - Evidence (runtime): accepted C58 actual-server HTTP evidence proves the grammar through the production Store and route across a bounded 69-Question fixture.
  - Verification pending: the bounded proof does not establish usability or performance for a very large production Question Library.
- [x] Search terms and active filters should remain visible while reviewing results.
  - Evidence (source): `src/pages/library_page.tsx` `query` signal remains bound to the search input and filter selects while rows render.
- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
- [x] Opening a result and returning should preserve the Instructor's search and position.
  - Evidence (source): `src/pages/library_page_model.ts` `saveQuestionLibraryReturnState` and `takeQuestionLibraryReturnState` retain one session-bound, single-use in-document snapshot; `src/pages/library_page.tsx` `LibraryPage` restores its query, server-validated loaded rows, filters, and clamped scroll position.
  - Evidence (runtime): one-time accepted compiled-browser exercise of `src/pages/library_page.tsx` `LibraryPage` restored `genetics`, the `ple` filter, 80 loaded rows, and exact virtual-list scroll position through visible detail return and browser Back; a changed session returned to the empty landing. The temporary harness and screenshots were removed after the accepted proof.

##### Browse Question Library

- [x] **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
  - Evidence (source): `src/route_contract.ts` `libraryBrowse` is a distinct Instructor route and `src/pages/library_page.tsx` `LibraryPage` provides an overview-first grouped Browse mode without requiring Search text.
  - Evidence (runtime): accepted private full-app browser evidence exercised `src/main.tsx` `render` against the actual server through overview-first Browse, Biology, Enzymes, and focused Search without starting from Search text; every actual search response was `no-store`.
- [x] Browse should help Instructors understand what the Question Library contains.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` explains that counts cover all authorized matching Questions and presents subject, topic, tag, and Question Type groups.
  - Evidence (runtime): accepted C60 component evidence rendered `src/pages/library_page.tsx` `LibraryPage` overview groups; actual-server HTTP evidence exercised `crates/server/src/question_library.rs` `search_questions` over the full authorized matched snapshot.
- [x] Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` renders dedicated Subjects, Topics, Tags, and Question Types groups with explicit free-text truncation notices.
  - Evidence (runtime): accepted component and actual-server HTTP evidence covered `crates/server/src/question_library/facets.rs` `facets` for all four groups, including truthful 64-value truncation flags.
- [x] Browse should make moving from broad subjects to narrower topics easy.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` clears the prior topic on subject selection and requests exact subject filtering before presenting `Topics in this subject`.
  - Evidence (runtime): accepted routed component evidence exercised `src/pages/library_page.tsx` `changeQuery`, selected Biochemistry then Enzymes, and preserved both exact filters.
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

#### Assessments

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
- [ ] Instructors should be able to inspect a Question before adding it to an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `questionRevisionInspectionPath` addresses the candidate's exact Revision, and `src/api/client.ts` `getQuestionRevisionDetails` exposes the strict client operation. The server resolves the Instructor-authorized private source and checksum through the existing opaque WeBWorK adapter, which supplies a hardened iframe.
  - Evidence (runtime): accepted private PostgreSQL 17/MinIO and unchanged-renderer HTTP evidence returned preview 200 with hardened headers and concealed missing Revision, Student, and anonymous requests. Exact-main browser evidence visibly rendered the prompt and five choices. This isolated preview path left all five Student Work counts at zero before and after: Assessment Attempts, Question Attempts, saved responses, submissions, and grading results.
  - Verification pending: the renderer JavaScript dereferences `window.frameElement.id` when the hardened sandbox has no same-origin frame element, then errors before focus, popover, and parent telemetry. Do not loosen the iframe sandbox or rewrite the sibling renderer HTML. Successful embed behavior and return without losing Assessment state remain required for closure.
- [x] Assessment Properties should group related settings so important settings are easy to find.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` groups Assessment and delivery controls separately from Student feedback, and the owning CSS uses a two-column desktop grid that collapses to one column below 60rem.
  - Evidence (runtime): accepted private exact-main browser evidence rendered `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` at 1280 by 800; computed-style checks verified the two-column desktop and one-column 720px layouts, restored panel/control styling, and persisted edited instructions through actual HTTP and reload.
- [x] Instructors can randomize Question order for an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `updateOrder` binds the current Assessment Properties checkbox to authored or shuffled Question order. The Student start transaction persists that rule with the Attempt and shuffles the complete fixed-and-Pool issued vector only for `shuffled`.
  - Evidence (runtime): accepted private actual-HTTP evidence exercised `crates/learning-data-access/src/postgres/assessment_delivery_start.rs` `start_current_assessment_attempt`: it persisted authored and shuffled rules, observed a concurrent Instructor save blocked behind the Student-start lock, issued exact fixed-and-Pool pins, and resumed the immutable shuffled Attempt after a current-rule edit. Accepted exact-main browser evidence saved and reloaded `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s visible **Randomize question order** checkbox through actual HTTP.
- [x] Answer-choice randomization belongs to the Question, not the Assessment.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_editor_model.ts` `setChoiceRandomization` and the strict Question codec own `randomizeChoices`; `crates/adapters/ple/src/question_json/source_document.rs` `compile_choices` compiles it to `NativeChoiceOrder`, and `crates/question_model/src/presentation/builder.rs` `pending_items` applies its nonce-derived choice permutation. The closed Assessment activity rules contain Question-order policy but no answer-choice override.
  - Evidence (runtime): accepted exact-main browser evidence preserved `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`'s explicit Question-owned choice-order explanation while saving and reloading the independent Assessment Question-order rule. This verifies the ownership boundary without claiming a runtime matrix of every native choice permutation.
- [x] **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `AssessmentsDueSoonPage` calls `listAssessmentsDueSoon` and states its across-Courses scope.
  - Evidence (runtime): accepted private actual-server evidence exercised `schemas/base_schema/assessment_operations.sql` `list_assessments_due_soon` across two owned Courses and one outsider Course; each Instructor saw only their own Course rows, while anonymous and Student requests received the same concealed response.
- [x] Assessments Due Soon shows the Course and due time for each Assessment.
  - Evidence (source): `src/pages/assessments_due_soon_page.tsx` `DueSoonAssessmentRow` renders `courseLongName` and a formatted due time.
  - Evidence (runtime): accepted private full-app evidence exercised `src/pages/assessments_due_soon_page.tsx` `formatDueTime`; both visible Course names and Due values matched the actual HTTP instants formatted in the response's Account time zone.

#### High-consequence actions

- [ ] Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Mismatch: `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` implements Assessment Unrelease and `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` implements Archive Blueprint Course, but no current interface exposes Archive Published Question in a Danger Zone; the browser API in `src/api/question_availability.ts` alone is not an Instructor workflow.
- [x] Danger Zone should be visually separate from ordinary editing actions.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-workspace-unrelease-danger-zone` is the sole current Danger Zone consumer, and `src/pages/assessment_workspace/assessment_workspace.css` applies the distinct danger border, background, spacing, and responsive layout to that exact class while preserving its shared `assessment-editor-field` child.
  - Evidence (test): an independently reviewed temporary Chromium proof rendered `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-workspace-unrelease-danger-zone` with current CSS and verified the destructive panel remained visually distinct with a readable confirmation action at 1280px and 600px; it was component evidence, not connected-app acceptance.
- [x] Assessment Unrelease should explain that Student work will be deleted.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `assessment-unrelease-confirmation-help` says Unrelease permanently deletes the represented Student Work, and the action is labeled "Unrelease and delete Student Work."
- [x] Assessment Unrelease should require typing the Assessment title before confirmation.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `confirmationTitle` disables Unrelease unless the entered value exactly matches the current Assessment title, while `schemas/base_schema/unrelease.sql` `ple_api.unrelease_assessment` independently rejects a nonmatching confirmation title.
- [ ] Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Mismatch: `src/features/blueprint_course/blueprint_course_lifecycle_controls.tsx` `Archive Blueprint Course` explains removal from new selection and requires the long name, but no current Archive Published Question interface provides the corresponding explanation and confirmation; `src/api/question_availability.ts` `archiveQuestion` is only a browser transport contract.
- [x] Restore actions should use ordinary availability controls.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `restore` is presented under Course names and availability rather than the archive confirmation control.
### Student interface

- [x] The Student interface should focus on current Courses, Coursework, and work that needs attention.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` lists current courses; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` lists assigned work.
- [ ] **Coursework** is the Student-facing collective term for Regular Assignments, Practice Question Assignments, Bonus Assignments, Quizzes, and Exams.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` labels the collective section and its loading and empty states "Coursework," while each item receives one canonical Type from the typed projection.
  - Mismatch: the actual PostgreSQL landing Store proved the same closed Type column with Regular Assignment and the compiled SolidJS/mock-API browser evidence was accepted, but the connected Student HTTP workflow was not run; this broad Student-facing terminology row remains runtime-unverified.
- [ ] Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` renders the specific Type label and uses it in the individual item's open action; `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` uses it in start, resume, and availability language.
  - Mismatch: the actual PostgreSQL access and landing Stores projected the specific Type and accepted compiled SolidJS/mock-API browser evidence rendered it, but the connected Student HTTP workflow and other Student-facing surfaces were not exercised; this broad interface row remains open.
- [x] The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` labels the Student-only tab "Coursework" and `RIBBON_TASK_CATALOG` labels its return task "Back to Coursework"; `src/ribbon/ribbon_contract.ts` uses "Before you start" when the individual Coursework title is not yet available, "Attempt" for the Student Attempt task area and current breadcrumb, and "Attempt history" for the history breadcrumb. Instructor Assessment labels remain separate.
  - Evidence (test): `tests/test_ribbon_contract.mjs` `Student Coursework navigation retains collective labels` and its canonical breadcrumb projections passed with the focused 15-test Ribbon contract lane.
  - Evidence (runtime): `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` and `src/ribbon/ribbon_contract.ts` `deriveRibbonModel` were exercised by accepted isolated actual-server/exact-main Student browser receipts `/private/tmp/ple-course-empty-artifacts.r7S1T6` and `/private/tmp/ple-course-empty-artifacts.ONrLSK`. They showed the Coursework Ribbon tab, "Before you start" overview breadcrumb, specific Practice Question Assignment Type, and the submitted Attempt history route/breadcrumb after a real native Student Attempt. The source catalog keeps the return task "Back to Coursework"; the complete Student Ribbon task layout remains separately unlocked.
- N/A Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**, **Bonus Assignments**, **Quizzes**, and **Exams**.
  - Reason: Optional permission does not require current product behavior.
- [x] Each Coursework item should clearly show its Assessment Type using its label and Type icon.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` always renders `typePresentation().label` beside the guaranteed bundled `typePresentation().icon`; semantic Type color is supplementary.
- [x] The Student interface should make the next useful action easy to find.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` presents the primary "Open Assessment" action.
- [ ] The Student menu is simpler than the Instructor menu.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` `RIBBON_TASK_CATALOG` does not define a complete Student menu for comparison.
- [ ] Student workflows should work well on laptops, portrait tablets, narrow phones, and square displays.
  - Mismatch: needs runtime evidence for the four required Student viewport classes; `tests/playwright/student_course_entry_m6_evidence.mjs` does not cover them.
- [ ] Every Student browser action should be usable with the keyboard alone.
  - Mismatch: needs keyboard-only journey evidence; `src/pages/assignment_attempt_page.tsx` has keyboard-operable controls but no complete Student journey test.
- [x] Student pages should use names meaningful to Students.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` uses "Your courses" and "Open assigned work".
- [ ] Student navigation and pages should contain only Student interfaces and capabilities.
  - Mismatch: `src/route_contract.ts` `studentCourseLanding` restricts that one route to Students, but source inspection is not evidence that every Student navigation and page exposes only Student capabilities; the required authorization/runtime check has not been recorded.
- [x] Students enrolled in one active Course should go directly into that Course.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` redirects the one-entry `courses()` result to its Course reference.
- [x] Students should be able to see their active Courses and Coursework from the main navigation.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` provides the current-Course index; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` provides its work.
- [x] Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
  - Evidence (source): `src/pages/student_coursework_presentation.ts` `studentCourseworkDisplay` maps the server-owned start decision, completion, and resumability to upcoming, available, in-progress, completed, or missed learner states.
  - Evidence (test): `tests/test_student_coursework_presentation.mjs` `Coursework display distinguishes resumable, non-resumable unfinished, and completed work` exercises the pure state projection.
  - Evidence (runtime): `crates/learning-data-access/tests/assessment_access_postgres.rs` `access_reader_projects_one_authoritative_decision_and_effective_policy` passed on a fresh PostgreSQL 17 database, proving the landing Store projects scheduled, expired unfinished, active resumable, Attempt-limit-reached resumable, and late-work-refused resumable states; accepted compiled SolidJS/mock-API browser evidence proved their visible presentation. This does not claim a connected HTTP-server run.
- [x] Coursework lists should make due dates, Type, and completion status easy to scan.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` presents visible Type, due, completion, and state fields from the typed Student projection.
  - Evidence (runtime): `crates/learning-data-access/tests/assessment_access_postgres.rs` `access_reader_projects_one_authoritative_decision_and_effective_policy` passed on a fresh PostgreSQL 17 database and projected the Regular Assignment Type, due/availability facts, completion, and resumability through the actual landing Store; accepted compiled SolidJS/mock-API browser evidence proved the fields are scannable. This does not claim a connected HTTP-server run.
- [x] Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` presents the title, specific Type, Question count, points possible, time limit, and previous Attempts before start.
  - Evidence (source): `src/components/student_assessment_presentation.tsx` `StudentAssessmentStartFacts` owns the compact Question, points, and time-limit facts.
  - Evidence (runtime): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` was exercised by accepted isolated actual-server/exact-main Student proof `/private/tmp/ple-course-empty-artifacts.JCFLm9` after a real roster import/claim. It opened a Released direct Practice Assessment before Start and showed title, Practice Type, one Question, one point, the exact one-hour limit, and an explicit zero-previous-Attempt state; an Unreleased sibling was omitted and an outsider received 404. A separate accepted native Student HTTP/browser run `/private/tmp/ple-course-empty-artifacts.ONrLSK` whole-submitted a real graded 1/1 Attempt, then reopened the overview before starting another. The same six facts included an actual "Previous attempts" Attempt 1 Submitted link; its clicked history showed recorded PKU response and 1/1 score. The overview's previous-Attempt score is optional under the current DTO, so this row does not require that optional value or claim every Student viewport.
- [x] Students see one Question at a time while completing Coursework.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders one keyed current presentation in one `article.question-card`.
- [x] While completing Coursework, navigation should show every Question, its saved status, and allow Students to jump directly between Questions.
  - Evidence (source): `src/components/student_assessment_attempt_navigation.tsx` `StudentAssessmentAttemptNavigation` renders every position, saved-status label, and position button.
  - Evidence (test): `tests/test_student_assessment_attempt_navigation.mjs` `Student Question navigation renders ordered, answer-free states with one current Question`.
- [x] Leaving a Question and returning should preserve its saved response.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `activatePosition` saves before changing position; `src/pages/assessment_attempt_page.tsx` `loadPresentation` restores the persisted `savedResponse` when the Student returns.
- [x] The current Question and overall progress should remain easy to see.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders "Question" with the current position and overall question count immediately before the Question navigation.
- [x] The timer should be subtle and keep the focus on the Questions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` places the `calm-status` timer in the Assessment Attempt header, outside the Question card.
- [x] For timed Coursework, the remaining time should stay visible while moving between Questions.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders `remainingMilliseconds` in the persistent header while keyed Question presentations change below it.
- [x] Submission status should be obvious and use plain language.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` renders "Submitting Assessment...", "Your answers were accepted", and plain-language save or submission errors from the submission state.
- [x] Scores and feedback should appear where the Coursework settings allow them.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_history` applies the server-owned feedback-release decision before projecting scores and per-Question feedback; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` renders only the released fields present in that projection.
- [x] Completed Coursework should remain easy to find and review.
  - Evidence (source): `src/pages/assessment_overview_page.tsx` `AssessmentOverviewPage` lists and links previous Attempts; `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptHistoryContent` presents the selected Attempt's score and recorded work.
- [x] Student content entry should use the response controls provided by Questions and other Student activities.
  - Evidence (source): `src/pages/assessment_attempt_page.tsx` `AttemptExperience` passes the current Question presentation's response format to `QuestionPresentationResponseControl`.
- [ ] The complete Student Ribbon task layout does not have a locked-in design yet.
  - Reason: HG: no locked-in design.
  - Mismatch: no complete Student Ribbon task layout can be verified until the design is locked.

### Sysadmin interface

- [x] The Sysadmin interface should focus on system administration.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` labels its workspace "System administration" and manages Instructor Accounts.
- [ ] The Sysadmin menu should make Accounts, Instructors, Courses, and system configuration easy to find.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` defines Instructor Accounts and Scoped Support only; it has no Sysadmin Courses or system configuration destinations.
- [ ] Sysadmins should be able to find users quickly by name or email.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` has no name or email search.
- [ ] Account lists should support searching, filtering, and scanning large numbers of users.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` lists all Instructor Accounts without search, filters, or large-list pagination.
- [ ] User pages should clearly show role, account status, and other important administrative information.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` displays only Instructor Account reference, state, and sign-in time; no user detail page exists.
- [x] Sysadmins create accounts and manage account access.
  - Evidence (source): `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` provides Instructor Account creation, deactivation, and reactivation actions.
- [ ] Sysadmins approve Instructors before they receive Instructor capabilities.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `createAccount` creates an active Instructor Account directly; there is no approval state.
- [ ] Instructor approval status should be easy to find and change.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `InstructorAccountsPage` exposes active/deactivated/closed account lifecycle state, but no Instructor approval status exists to find or change.
- [ ] Sysadmins should be able to find and inspect Courses across the installation.
  - Mismatch: `src/pages/support_roster_page.tsx` `SupportRosterPage` inspects only one Instructor-issued exact-capability roster, not installation-wide Courses.
- [ ] Course administration should show the Instructor and important Course status information.
  - Mismatch: no Sysadmin Course administration page or Course status projection exists in `src/pages/`.
- [ ] Sysadmins should manage Courses through Sysadmin interfaces and capabilities.
  - Mismatch: `src/route_contract.ts` has no Sysadmin Course-management route.
- [ ] System-wide settings should have their own area, separate from user and Course administration.
  - Reason: product decision still unclear
  - Question: Which implemented installation-wide settings must Sysadmins view or change, and which source-of-truth boundary owns each?
  - Mismatch: `src/ribbon/ribbon_catalog.ts` has no system-settings destination. The guidance could require a page for actual platform settings, or no page until implemented system-owned settings exist; current evidence cannot select between those readings.
- [x] Everyday navigation should emphasize frequently used administrative tasks.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `instructorAccounts` is a primary critical task while `supportRoster` is supporting normal priority.
- [ ] Rare installation and configuration tasks should remain available through secondary navigation.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` `TAB_CATALOG` contains only the primary Instructor Accounts Sysadmin destination; no rare installation or configuration task is available through secondary Sysadmin navigation.
- [ ] High-consequence administrative actions should have a visually distinct area.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `Deactivate Instructor Account` uses the ordinary `quiet-action` styling with no distinct high-consequence area.
- [ ] Confirmation for destructive actions should clearly state what will happen.
  - Mismatch: `src/pages/instructor_accounts_page.tsx` `deactivate` executes immediately after a reason is entered; no confirmation step states the consequence.
- [ ] The complete Sysadmin Ribbon task layout does not have a locked-in design yet.
  - Reason: HG: no locked-in design.
  - Mismatch: no complete Sysadmin Ribbon task layout can be verified until the design is locked.
## Data and history

- [ ] Answers, keys, grading, and correctness decisions should stay on the server, out of reach of **Students**.
  - Mismatch: The cited history reader authorizes one completed-Attempt projection and the expiry test finalizes grading; neither proves every answer, key, grading, and correctness path remains server-only and unreachable to Students.
- [ ] Public data should stay separate from private, answer-bearing, identifying, or radioactive FERPA data.
  - Mismatch: The aggregate-statistics tables and private receipts show one separation boundary, not a complete audit of public data and every private, answer-bearing, identifying, or FERPA-sensitive path.
- [ ] Human-readable titles and identifiers should be used wherever people must recognize, copy, or enter them.
  - Mismatch: `assignment_title` in one Student-history projection does not prove that all recognition, copy, and entry surfaces use human-readable titles and identifiers.
- [ ] FERPA-sensitive Student data should not become ordinary logs, analytics, URLs, or long-lived browser storage.
  - Mismatch: `src/log.ts` only describes browser console logging and explicitly leaves redaction for later; it does not prove that all logs, analytics, URLs, and browser storage exclude FERPA-sensitive Student data.
- [x] Opaque IDs remain FERPA-sensitive when they link a Student to Course activity.
  - Evidence (source): `schemas/base_schema/authorization.sql` `current_session_account_owns_student_record` authorizes a Student record only through its Course and active membership.

### Human-facing reference IDs

- [ ] Human-facing reference IDs should be short, opaque, easy to communicate, and should not reveal creation order, counts, database keys, ownership, or other object metadata.
  - Evidence (source): `crates/question_model/src/public_route.rs` defines opaque typed read/use formats and `src/navigation/public_route.ts` accepts only their typed forms at browser route boundaries.
  - Mismatch: no connected creation/allocation proof establishes cryptographically random, nonsequential references across all required objects.
- [ ] Blueprint Course IDs use `BP`, Course Instance IDs use `CI`, Assessment IDs use `A`, and Account IDs use `U`, followed directly by a common cryptographically random Crockford Base32 reference format.
  - Evidence (source): `crates/question_model/src/public_route.rs` and generated/browser route contracts read and use the `BP`, `CI`, `A`, and `U` typed formats; authenticated server paths parse those formats before Store authorization.
  - Mismatch: creation and allocation boundaries have not received connected proof of the common cryptographically random format.
- [ ] ID generation enforces uniqueness and retries random collisions.
  - Mismatch: no connected common human-reference allocator/retry collision proof exists.
- [ ] Give an internal object a human-facing reference ID when a useful workflow needs to display, search, communicate, or support it.
  - Mismatch: Current private route-token inventory needs a workflow-by-workflow audit before a reference is exposed or retained as human-facing.
- [ ] Account `U` references are Sysadmin support references and are not automatically exposed to Students or Instructors.
  - Evidence (source): accepted source review found `U` parsing/use limited to authenticated Account-management paths; no Student or Instructor projection was identified in that review.
  - Mismatch: full cross-route authorization and creation-boundary proof remains outstanding.
- [ ] Published Questions and published Question Pools retain their existing public `AAAA-ZBBB` IDs.
  - Mismatch: Published Question IDs use `AAAA-ZBBB`, but published Question Pool identities are not complete.

### Student and FERPA data

- [x] **Student** course data falls under FERPA; treat it as radioactive.
  - Evidence (source): `schemas/base_schema/course_membership.sql` `student_record` is protected by RLS and has no PUBLIC privilege.
- [x] **Student** data should be collected reluctantly, used deliberately, and purged predictably.
  - Owner: `docs/active_plans/audits/hg_checklist_parts/02_accounts.md`
  - Evidence (source): `schemas/base_schema/course_roster.sql` `course_roster_profile` contains no duplicate Student email; ordinary roster is email-free and direct-Instructor-only.
  - Evidence (source): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` removes identifiable Course Student records while retaining Account and Course teaching material.
  - Evidence (runtime): `schemas/base_schema/course_retention_transitions.sql` `delete_course_student_records` passed a self-owned disposable PG17 purge-preservation probe on 2026-09-15.
- [x] FERPA access should be scoped through exact Course membership and **Student** ownership.
  - Evidence (source): `schemas/base_schema/authorization.sql` `current_session_account_owns_student_record` requires the exact Course, Student Record, authenticated Student Account, and active Student membership before Student Work access is allowed.
  - Evidence (test): `crates/learning-data-access/tests/assessment_access_postgres.rs` `access_reader_projects_one_authoritative_decision_and_effective_policy` uses a real `ple_auth` to `ple_app` session to allow the owner and deny a same-Course other Student, nonmember, same Account with another Course record, and ordinary Sysadmin.
  - Decision: This permanent behavior-level BOLA/FERPA oracle protects a stable high-impact outcome. If its baseline gate fails, this record returns to `[ ]` while the session installation, exact-membership predicate, or ownership boundary is repaired and the same gate rerun.
- [x] **Sysadmins** receive only the FERPA access required for a specific administrative task.
  - Evidence (source): `crates/server/src/support_capability.rs` `support_capability_router` exposes only a scoped, revocable exact-record repair reader; it has no whole-Course roster route.
  - Evidence (test): `tests/e2e/e2e_live_demo_support_capability.sh` `prove_issue` exercises named-record repair issuance, concealment, use, and revocation.
- [x] Student Accounts persist independently of Course data and Course retention.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account` is separate from course-scoped `student_record`.
- [ ] Course work, Attempts, submissions, grades, and other FERPA-sensitive data follow the Course retention policy.
  - Mismatch: No Course retention policy or deletion transition is present to govern these records.
- [ ] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Mismatch: Student-data deletion and its preservation boundary are not implemented.
- [ ] **Student Work** is the collective term for FERPA-sensitive records created by a Student in a Course Instance.
  - Mismatch: The Attempt table links a Student record and Assessment, but the implementation does not establish `Student Work` as the collective product term for all such records.
- [x] Student Work includes Assessment Attempts, saved Question responses, grading outcomes, and the evidence needed to interpret that work after an Attempt is submitted.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history` joins Attempt, issued question, response, submission, grading, and receipt evidence.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies retained response finalization and resulting score evidence.
- [ ] Student Work is an umbrella term; the underlying records retain their own identities and purposes.
  - Mismatch: Distinct Attempt, issued-Question, and Question-Pool-selection records show separate identities, but no implemented collective `Student Work` term establishes the required umbrella relationship.
- [ ] Student retention removes identifiable Student evidence, not privacy-safe aggregate Question statistics.
  - Mismatch: Retention deletion is not implemented, so this separation has not been realized.
- [ ] Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
  - Mismatch: Aggregate statistics exist, but no implemented Student-record deletion transition proves their retention behavior.
- [ ] Aggregate Question statistics must not identify or allow reconstruction of individual Student activity.
  - Mismatch: `question_revision_statistics` omits direct identity fields, but the implementation has no demonstrated disclosure or small-cohort rule preventing aggregate counts from reconstructing an individual Student's activity.
- [x] Published Question statistics retain accepted graded Attempt count and correct count.
  - Evidence (source): `schemas/base_schema/statistics.sql` `question_revision_statistics` has `accepted_graded_attempt_count` and `correct_count`.
- [x] Eligible Question Types may also retain aggregate answer-choice counts.
  - Evidence (source): `schemas/base_schema/statistics.sql` `selected_count` stores aggregate choice counts.
- [x] Question statistics are version-specific first, with clearly labeled Question-level rollups when appropriate.
  - Evidence (source): `schemas/base_schema/statistics.sql` `question_revision_statistics` primary key is `(question_id, revision_number)`.

### Course retention

- [ ] Course retention should follow Course Instance dates and its six-month Active lifetime rather than a fixed academic calendar.
  - Mismatch: Course Instance storage has no six-month Active lifetime or retention deadline.
- [ ] The latest Assessment deadline ends normal teaching and starts the Course Instance's FERPA retention clock.
  - Mismatch: Assessment deadlines exist, but no Course FERPA retention clock is derived from them.
- [x] Creating or extending a later Assessment deadline may move those dates, but not beyond the six-month Active lifetime.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies` lock the Course first, reject a Due date after its immutable `active_until_at`, and invoke `ple_data.synchronize_course_assessment_deadline` after an accepted change.
  - Evidence (source): `schemas/base_schema/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` stores the current maximum Assessment Due date and moves the active Course retention anchor to that date, or to `active_until_at` when no Due date remains.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proofs exercised `schemas/base_schema/assessments.sql` `ple_data.save_assessment`, `ple_data.save_assessment_inline`, and `ple_data.save_assessment_policies`, covering release, a cleared last Due date, cap rollback, stale CAS, wrong-Instructor denial, deterministic concurrent saves to two Assessments, an archive race, and frozen archived/deleted retention anchors.
- [ ] Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
  - Mismatch: The FERPA retention-clock transition is absent.
- [ ] The configured FERPA retention policy determines the later notice, archive, recovery, and permanent deletion transitions.
  - Mismatch: No configured FERPA retention policy or its transitions exists.
- [ ] PLE warns the **Instructors** before the Course Instance becomes Inactive six months after creation.
  - Mismatch: No six-month inactivity transition or Instructor warning exists.
- [x] The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying FERPA retention and deletion.
  - Evidence (source): `schemas/base_schema/course_core.sql` `ple_data.enforce_course_instance_retention_schedule` derives and preserves the immutable six-month `active_until_at`; `schemas/base_schema/assessments.sql` `ple_data.save_assessment` and its sibling save functions reject every saved Due date beyond that cutoff.
  - Evidence (source): `schemas/base_schema/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline` bounds the active retention anchor by the accepted current maximum Due date or that immutable cutoff and does not move an archived or deleted anchor.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proofs exercised `schemas/base_schema/assessment_deadline_sync.sql` `ple_data.synchronize_course_assessment_deadline`, rejecting over-cap saves without partial state, keeping concurrent current deadlines synchronized, and preserving the retention anchor after archive while later Assessment facts changed.
- [ ] Course inactivity and FERPA deletion are separate transitions; becoming Inactive does not itself delete Student records.
  - Mismatch: Neither Course inactivity nor FERPA deletion transition is implemented.
- [ ] Retention should work equally for semesters, quarters, summer Courses, and other academic calendars.
  - Mismatch: No Course retention processing exists for any calendar.
- [ ] PLE should notify the **Instructor** before FERPA-sensitive Student data is archived.
  - Mismatch: No Student-data archiving or associated notification exists.
- [ ] Archived Student data should leave normal Instructor and Student interfaces but remain recoverable during the retention period.
  - Mismatch: No archive/recovery state or interface exclusion exists.
- [ ] FERPA-sensitive Student data should be permanently deleted when its retention period expires.
  - Mismatch: No retention-period expiry deletion exists.
- [ ] Course metadata, Assessment definitions, Questions, settings, and other teaching material remain after Student data is deleted.
  - Mismatch: No Student-data deletion transition exists to establish this preservation behavior.
  - Owner: A6
- [ ] FERPA retention intervals are operational configuration rather than separate product decisions.
  - Mismatch: No operational FERPA retention interval configuration exists.

### Retention processing

- [ ] A background process should periodically find Course Instances whose retention deadlines have passed.
  - Mismatch: `worker` only sweeps expired Assignment Attempts, not Course retention deadlines.
- [ ] Retention decisions should come from stored Course dates and the Course Instance creation time.
  - Mismatch: No stored Course retention decision or worker implementation exists.
- [ ] The background process should execute retention policy rather than define when retention periods begin or end.
  - Mismatch: No Course retention processor or policy exists.
- [ ] Running the retention process late should produce the same retention decision as running it on schedule.
  - Mismatch: No Course retention process exists to test schedule independence.
- [ ] The retention process should be safe to run repeatedly.
  - Mismatch: No Course retention process exists to establish repeat safety.

### Revisions and history

- [x] Be conservative about creating revisions.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` `ple_private.publish_question_revision` locks the Draft and immediate parent before it creates a successor; its `PQR01` source-checksum comparison rejects an unchanged `question_revision_source_binding` before any successor facts are written.
  - Decision: A fresh one-time PostgreSQL 17 probe exercised `schemas/base_schema/question_publication_operations.sql` `ple_private.publish_question_revision` and verified title and description metadata changes make no Revision, an unchanged source is rejected without a partial write, a changed source creates the next Revision, and two serialized sessions admit only one successor. Tags, subject, and topic have no persisted metadata fields yet; the probe asserts that present absence rather than inventing a field-level behavior. The probe is temporary and will be removed, not cited as permanent evidence.
- [x] Assessments, Course Instances, and Draft Questions use current state.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` updates current Assessment state and advances its `assessment_edit_number`.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.update_course_theme` updates the current `ple_data.course_instance` row.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_private.save_authoring_draft` updates the current `ple_private.draft_question`, metadata, and source-binding rows.
- [ ] Published Questions, Question Pools, and Blueprint Courses have immutable revisions.
  - Mismatch: Published Question and Blueprint revision storage exists, but Question Pools are current Assignment configuration and have no immutable Question Pool Revision.
- [x] Mutable working state uses a monotonic sequential Edit Number when needed for concurrency.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `save_assessment_inline` requires and advances `assessment_edit_number`.
- [x] An Edit Number is only a counter and does not identify a stored historical object.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `assessment_edit_number` is a current-state concurrency field rather than a revision foreign key.
- [ ] Question, Question Pool, and Blueprint Revision Numbers start at 1 and increase sequentially for each object.
  - Mismatch: Question and Blueprint revisions have positive sequential numbers, but Question Pools have no Revision Number.
- [ ] A Revision Number identifies a specific immutable Revision stored by PLE.
  - Mismatch: Question and Blueprint Revision Numbers identify immutable rows, but the absent Question Pool Revision leaves this general Revision Number behavior incomplete.
- [x] Student Work records the exact Assessment Attempt and Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `issued_question` records Attempt identity with `question_id` and `revision_number`.
- [x] Student Work records the Student's responses and the grading outcome returned by the Question Backend.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history` returns retained responses and grading results.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `first_score` verifies the finalization grading outcome.
- [ ] Student Work records the Question Pool Revision and selected Published Question Revision for each response.
  - Mismatch: `question_pool_selected_item` retains the selected Published Question revision but no immutable Question Pool Revision identity.
- [x] Changes to Question point values recalculate scores from the stored grading outcome without changing the outcome.
  - Evidence (source): `schemas/base_schema/grading.sql` `score_recorded_credit` calculates current points from retained `normalized_credit`.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `replay_score` changes points and verifies retained credit is replayed.
- [x] Changes to Assessment settings do not change the recorded history of completed Assessment Attempts.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history` deliberately interprets retained Attempt evidence rather than current Assessment content.
- [x] Immutable Question source and Question assets use SHA-256 checksums where needed to verify their stored contents.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `source_object_checksum` binds immutable Question-source contents to SHA-256 object records.
  - Evidence (source): `schemas/base_schema/question_assets.sql` `public_object_checksum` binds immutable Question-asset contents to SHA-256 object records.

### Dates and time zones

- [x] Assessment deadlines are stored as instants.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment.due_at` is `timestamptz`.
- [x] Instructor dates and times use the Instructor's IANA time zone.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account_time_zone_is_exact_iana` validates Instructor account zones against `pg_timezone_names`.
- [x] The Instructor's time zone is used to interpret dates and times the Instructor enters.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `resolve_in_account_time_zone` resolves entered release times with the account zone.
- [x] Changing an Instructor's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `crates/learning-data-access/src/postgres/assessment_release.rs` `LocalDateAndTime::from_activity_timestamp_in_account_time_zone` derives display values from stored timestamps and account zone.
- [x] Assessment deadlines are stored as absolute UTC instants.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment.due_at` uses PostgreSQL `timestamptz`.
- [x] Students have their own IANA time zone for displaying dates and times.
  - Evidence (source): `schemas/base_schema/accounts.sql` `account_time_zone_is_exact_iana` validates each Account's exact IANA time-zone preference.
- [x] A Student's time zone defaults to the Instructor's time zone during the invite phase.
  - Evidence (source): `schemas/base_schema/accounts.sql` `apply_student_invitation_time_zone_default` copies the Instructor preference while pending.
- [x] Changing a Student's time zone changes how existing deadlines are displayed without changing the deadlines.
  - Evidence (source): `schemas/base_schema/assessment_attempt_access.sql` `read_student_assessment_access` returns stored deadlines and separately reads `display_time_zone`.
- [x] Changing a display time zone changes how a deadline is shown, not the deadline itself.
  - Evidence (source): `schemas/base_schema/assessment_attempt_access.sql` `read_student_assessment_attempt_context` returns `display_time_zone` separately from `expires_at_millis`.
## Questions

- [x] Questions are subject agnostic. Properly tagged Questions from all subjects belong in the same Question Library.
  - Evidence (source): `crates/server/src/question_library/paging.rs` `QuestionSearchFilter` supplies the shared Library query filter without a subject partition.
- [ ] Questions are strictly and deterministically automated; grading does not require an **Instructor**.
  - Mismatch: needs runtime grading evidence for every supported backend.
- [x] Questions have one canonical title. Compact interfaces may truncate that title.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` stores one lineage-level title.
- [x] Every Question stored by PLE has its own internal Question record.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question` owns the internal Question record.

### Draft Questions

- [x] Draft Questions are private working content.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `ple_private.draft_question` stores draft state in the private schema.
- [x] Draft Questions use current state rather than immutable Revisions.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `draft_question_edit_number` is current-state concurrency data, separate from `question_revision`.
- [x] Saving a Draft Question replaces its previous working state.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `save_authoring_draft` replaces the current draft aggregate values.
- [x] Instructors may delete Draft Questions they no longer need.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `delete_draft_question` resolves only the current Instructor-owned Draft, locks and compares its Edit Number, then deletes that private aggregate without considering the separate Published Question lineage.
  - Evidence (source): `crates/learning-data-access/src/postgres/authoring.rs` `delete_authoring_draft` carries the SQL compare-and-swap through the authenticated Store.
  - Evidence (source): `crates/server/src/authoring.rs` `delete_draft` requires the parsed `If-Match` Edit Number and maps a concurrent change to 412; `src/pages/question_drafts_page.tsx` `QuestionDraftsPage` supplies explicit Keep/Delete confirmation.
  - Evidence (runtime): `crates/server/src/authoring.rs` `delete_draft` passed accepted isolated PostgreSQL 17/MinIO actual-server and focused browser proof: cancel, confirm, and list reload; valid-current-ETag collaborator, unrelated Instructor, Student, Sysadmin, and anonymous 404 denials while owner source/Edit Number remained unchanged; 428 missing, 400 malformed, and 412 stale preconditions; preserved parsed Published Question lineage and Revision JSON after a published-origin Draft deletion; and 404 repeat DELETE/PUT. Artifact: `/private/tmp/ple-draft-delete-artifacts.km9ybM`.
- N/A PLE may clean up abandoned Draft Questions after an appropriate warning and recovery period.
  - Reason: Automated abandoned-Draft cleanup is an explicitly optional future capability; HG sets no clock or durations.

### Question formats and types

- [x] PLE flat-question JSON is the canonical machine format for simple static Questions.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the PLE JSON source form.
- [x] QTI is for import, export, and archival interchange rather than the internal source model.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `workspace_import` treats `qti` as an import format, not a source binding.
- [x] MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT Question Types should be supported.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision` CHECK lists all eight types.
- [x] Question Type is immutable author-declared educational metadata on a Published Question Revision.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision_is_immutable` protects `question_type` on a revision.
- [x] PLE uses Question Type for search, filtering, labeling, and presentation.
  - Evidence (source): `src/api/question_library_repository.ts` `questionSearchRequest` sends the selected Question Type as the Library search filter; `src/pages/library_page.tsx` `questionTypeLabel` supplies learner-facing type labels and the Question Type selector presents the type facets.
- [x] Question Type comes from the author rather than inference from backend controls.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `draft_question_source_binding` records authoring input independent of backend.
- [x] Question importers are transient translators from external formats into PLE-managed Question representations.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `workspace_import` stages external-format imports before committed PLE state.

### Native PLE JSON Questions

- [x] The native PLE JSON Question format is private, unversioned, and unpublished.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` accepts the unversioned internal source shape.
- [x] Stored native JSON Questions may be upgraded together when the internal format changes.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` is the single internal reader for stored PLE JSON.
- [x] The native PLE JSON Question format is a strictly validated internal source shape without an external API.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` validates the internal source document.
- [x] Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
  - Evidence (source): `crates/question_model/src/generation.rs` `QuestionReproduction` distinguishes static source reproduction from the inseparable seeded generator pair; `crates/adapters/ple/src/lib/question_json_source.rs` `presentation` issues native PLE JSON with `QuestionReproduction::Static`.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `validate_issued_question_reproduction` rejects a seed for a `ple` source and requires one for renderer-backed sources.
  - Evidence (runtime): `schemas/base_schema/assessment_attempts.sql` `validate_issued_question_reproduction` passed in `/private/tmp/ple-native-seed-proof.sh --isolated --native-seed-http` against PostgreSQL 17: shuffled-position-2 native seed/hash were null, real WeBWorK retained numeric seed/64-character hash privately, public start/read/save/resume/restored payloads omitted both fields, resume retained the same issued Questions and saved native response, and invalid native seed insertion failed. Artifact: `/private/tmp/ple-native-seed-artifacts.KfY7Op`.
- [x] Native PLE JSON supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonResponse` defines all eight native types.
- [x] External URLs used by native JSON Questions are explicitly recorded and reviewable.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonDocumentBody` records the author-declared `externalResources` inventory as source metadata only, without fetching or browser permission; `validate_external_resources` bounds and de-duplicates recorded URLs.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `validate_external_resource_url` accepts only bounded, printable, absolute HTTPS URLs without user information.
  - Decision: A one-time parser proof accepted all five resource kinds and legacy omission, while rejecting invalid and duplicate URLs; it is temporary evidence and will be removed rather than retained as a permanent implementation-inventory test.
- [x] Recorded external URLs include links, images, scripts, stylesheets, and other resources.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResourceKind` is the closed Link, Image, Script, Stylesheet, and Other category set for every `externalResources` entry.
  - Evidence (source): `crates/adapters/ple/src/question_json/source_document.rs` `PleQuestionJsonExternalResource` binds each recorded URL to exactly one reviewed category under `deny_unknown_fields` parsing.

#### Native PLE JSON Questions and JavaScript

- [ ] Native JSON Questions may contain author-supplied JavaScript, including chemistry content using RDKit.
  - Mismatch: no author JavaScript field or RDKit integration was found in the native JSON schema.
- [ ] Author-supplied JavaScript may provide client-side rendering or interaction without access to a random seed.
  - Mismatch: author-supplied JavaScript is not implemented.
- [ ] Author-supplied JavaScript runs in an isolated browser environment.
  - Mismatch: no isolated author-script runtime was found.
- [ ] Author-supplied JavaScript is treated as untrusted content.
  - Mismatch: author-supplied JavaScript is not implemented.
- [ ] Author-supplied JavaScript is isolated from PLE application state, credentials, and privileged browser context.
  - Mismatch: no sandbox boundary for author JavaScript was found.
- [ ] Author-supplied JavaScript is limited to client-side rendering and interaction.
  - Mismatch: author-supplied JavaScript is not implemented.
- [ ] Author-supplied JavaScript operates independently of PLE application APIs and privileged state.
  - Mismatch: author-supplied JavaScript is not implemented.
- [ ] Native interactive Question Types such as HOTSPOT use PLE-owned interaction code.
  - Mismatch: HOTSPOT source editing exists, but delivered interaction evidence was not found.
- [ ] HOTSPOT content uses supported static assets such as images and SVG.
  - Mismatch: no delivery validation for HOTSPOT static assets was found.
- [ ] Grading and correctness decisions remain server-owned and independent of author-supplied JavaScript.
  - Mismatch: author JavaScript is absent; no runtime proof covers this interaction boundary.
- [ ] External JavaScript dependencies and CDN domains are explicitly recorded and reviewable.
  - Mismatch: no external JavaScript dependency inventory was found.
- [ ] Approved external dependencies may initially load from recorded CDN sources.
  - Mismatch: no approval or recorded-CDN mechanism was found.
- [ ] Supported external dependencies should eventually become PLE-owned and served locally.
  - Mismatch: no dependency-localization workflow was found.

### Question Backends

- [ ] WeBWorK, iMathAS, and H5P are PLE-managed Question Backends.
  - Mismatch: C870 removed every current H5P source, import, adapter, and runtime seam; H5P is not a delivered backend.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] The initial primary Question Backends are PLE-native JSON and WeBWorK.
  - Evidence (source): `schemas/base_schema/assessment_attempt_presentation.sql` `backend IN ('ple', 'webwork')` is the delivered presentation boundary.
- [ ] iMathAS and H5P are supported secondary Question Backends.
  - Mismatch: iMathAS has a launch boundary, while C870 leaves no current H5P source, import, or delivered backend seam.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] PLE owns and stores the Question representation used for each Question Backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_revision_source_binding` stores backend representations.
- N/A Imported backend source may be transformed into the form PLE stores and manages.
  - Reason: Optional transformation does not require a current backend-import behavior.
- [x] PLE preserves the information needed to reproduce the Question through its backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_revision_source_binding` retains backend selectors and source checksum.
- [x] PLE-managed Question representations participate in Question revision history.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_revision_source_binding` keys source bindings to immutable revisions.
- [ ] Question Backends own rendering, interaction, response, grading, feedback, and backend-specific state.
  - Mismatch: needs backend render-and-grade runtime evidence; current native and WeBWorK boundaries are not proof for all supported backends.
- [ ] PLE owns authorization, Question ID, revisions, persistence, lifecycle, and stored outcomes.
  - Mismatch: schema ownership is evidence for records, but no connected runtime or test evidence proves the complete authorization and stored-outcome boundary.
- [ ] PLE uses the same basic interface for every Question Backend, each backend handles its own internal details.
  - Mismatch: `crates/question_model/src/question_library.rs` `QuestionBackend` is only an enum discriminator. Issuance and finalization branch separately on backend in `crates/server/src/assignment_delivery.rs` `issue_new_presentations` and `crates/server/src/assignment_delivery/direct_finalization.rs` `evaluate_one`; no common adapter interface covers every backend.
- [ ] Each Question Backend adapter retains its backend-specific interaction knowledge.
  - Mismatch: `crates/adapters/webwork/src/lib.rs` `WebworkAdapter` establishes an opaque WeBWorK boundary, and `crates/adapters/imathas/src/imathas_question_backend.rs` defines an iMathAS seam, but C870 leaves no current H5P adapter or runtime seam. Evidence from WeBWorK alone cannot establish this claim for each backend.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] PLE-native Questions use the PLE Question Backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_source_binding_fields_are_valid` maps `ple` to `pleQuestionJson`.
- [ ] WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Mismatch: the isolated opaque adapter proves renderer documents, ordered pairs, score, partial credit, and stateless state. Connected live-ownership proof remains required; PLE is not required to capture historic renderer feedback.
- [x] Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` projects recorded native PLE feedback from the exact retained response and source, while the WeBWorK branch does not reconstruct or persist transient renderer feedback.
  - Evidence (runtime): the C910 isolated actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`, stopping the renderer after issuance and then submitting and reading exact WeBWorK Revision history without a backend-feedback field.
- [x] PLE does not extract or reconstruct transient feedback from Question Backend source or output.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `project_released_content` invokes recorded teaching-content projection only for the native PLE source variant; the WeBWorK source remains opaque.
  - Evidence (runtime): the C910 actual-HTTP proof exercised `crates/server/src/assessment_delivery/history.rs` `student_history`; the history read succeeded after the renderer stopped and exposed no choice, correct, or incorrect feedback reconstructed from the PGML source or rendered output.
- [x] Questions may have PLE-managed general feedback that remains separate from backend-generated interaction feedback.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_api.save_authoring_draft_general_feedback` stores immutable Revision `general_feedback` separately from the private source binding; `crates/server/src/assessment_delivery/history.rs` `project_released_content` assigns it independently of backend teaching-content projection.
  - Evidence (runtime): the C910 actual Student start, bodyless submission, history, and exact-main browser proof exercised `src/pages/assessment_attempt_summary_page.tsx` `AssessmentAttemptSummaryPage`, rendering the exact Revision marker as General feedback with all six disclosure timings `Never` while response, score, correctness, answer, and explanation remained absent.
- [ ] H5P owns its runtime, interactions, state, and scoring.
  - Mismatch: C870 leaves no current H5P source, import, adapter, or runtime seam, so H5P cannot yet own delivered runtime behavior.
  - Question: Which H5P content type(s) are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [ ] iMathAS owns its rendering and evaluation.
  - Mismatch: needs runtime rendering and evaluation proof for iMathAS.
- [ ] Question Backends may support more complex interactions without requiring PLE to implement those interactions.
  - Mismatch: incomplete secondary backends leave the general capability unverified.
- [ ] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Mismatch: needs test or runtime evidence for backend evaluation and immutable outcome creation.
- [ ] PLE stores the immutable credit fraction as the grading outcome.
  - Mismatch: needs test or runtime evidence linking backend credit to stored outcome.
- [ ] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Mismatch: needs runtime grading evidence for the no-deferred-state contract.
- [ ] Assessment scores are calculated from stored credit fractions and current Question point values.
  - Mismatch: needs test or runtime scoring evidence.
- [ ] Changing Question point values recalculates scores without another Question Backend interaction.
  - Mismatch: needs test or runtime rescoring evidence.
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
- [x] Use a Question Pool with algorithmic Questions only when the Instructor wants selection among distinct Questions, not to represent variants of one algorithmic Question.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` resolves selected Published Question Revisions and requires the Instructor's interchangeability attestation; `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects distinct immutable members without backend-specific variant expansion.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted fresh PostgreSQL 17/MinIO actual-server and private bundled-main HTTP-proxy browser proof: 42 canonical Genetics Questions installed with zero implicit Pools, then the Instructor visibly selected distinct DNA structure and nucleotide components Revision-1 PGML Questions, attested interchangeability, created a reusable Pool, and imported a distinct Assessment-owned fork with `selection_count=1`. Real WeBWorK rendering, radio-response save/resume, exact fork Pool/Question Revision, issued ID, seed/hash preservation, whole-Attempt submit, and fresh new-Attempt selection/issued IDs passed; a new Attempt may select the same Question and need not have different seeds. Artifact: `/private/tmp/ple-algorithmic-pool-artifacts.K2Kk6Z`. Release used a 3600-second time limit and Correct answer Never; answer disclosure, full Live Demo/authentication/TLS, and all-backend acceptance are outside this receipt. Browser error arrays were empty after route teardown completed.
- [x] BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
  - Evidence (source): `crates/project-tools/src/curriculum_content.rs` `validate_selected_parameterized_manifest` validates canonical source pins before publication.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication importing all 42 C839-accepted canonical PGML sources (41 BiologyProblems.org sources plus HLA), preserving source pins and producing ordinary available WeBWorK Question lineages. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.
- [x] Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.
  - Evidence (source): `crates/project-tools/src/curriculum_content/publication.rs` `validate_loaded_content` rejects non-Fixed entries in the canonical Blueprint.
  - Evidence (runtime): `crates/project-tools/src/curriculum_content/publication.rs` `publish_with_context` passed accepted fresh Genetics publication creating one Revision-1 Question lineage per canonical source, 42 direct Fixed entries, and zero Pools; exact replay was unchanged and a same-short-name conflict made no mutation. Artifact: `/private/tmp/ple-fresh-genetics-artifacts.5ERV83`.

### Question Pools

- [x] A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `create_question_pool` persists an ordered nonempty set of exact Published Question Revision members, and `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` selects from that Pool for Student delivery.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that selected an exact Pool member for Student Attempt 1, preserved it on resume, and selected again for Attempt 2. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `create_question_pool` requires the creating Instructor's true `interchangeability_attested` value; it does not substitute an automatic pedagogical evaluator.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that required the Instructor's attestation before creating the ordered reusable Pool and before its later Assessment-owned reorder. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.
  - Evidence (runtime): `crates/server/src/question_pool_creation.rs` `create_question_pool` passed accepted actual-server proof that false or missing attestation returned 422 and left no Pool behind. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.
- [ ] Question Pools may contain Questions from any Question Backend.
  - Mismatch: C885 supplies backend-neutral Pool membership, but no completed Instructor Pool workflow proves this behavior.
- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool_revision_member` stores each exact Published Question revision reference.
- [x] Question Pools are always published and have no draft or unpublished state.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool` and `question_pool_revision` model only a stable published lineage and immutable Revisions, with no draft, publication-status, or unpublished state.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main Instructor proof: it created a reusable Pool from two Published Questions and immediately read its server-issued Revision 1; the UI and API expose no draft or publish transition.
- [x] A Question Pool is an independently reusable Question Library object.
  - Evidence (source): `crates/server/src/question_pool_library.rs` `current_pool` reads a Pool independently of any Assessment.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `current_pool` passed accepted actual-main Instructor proof: Pool `SBQR-N5RE` was created from the Question Library and its ordered member pins were read through `/api/question-pools/SBQR-N5RE`; separate actual-server proof then imported another reusable Pool into an Assessment.
- [x] A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID and immutable revisions.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool` stores the unique compact public Pool ID, while `question_pool_revision` and `question_pool_revision_member` have immutable update/delete triggers and ordered exact member pins.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that returned canonical Pool ID `SBQR-N5RE`, Revision 1, then read the same identity and exact ordered Question Revision pins.
- [x] Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Evidence (source): `schemas/base_schema/assessment_pool_forks.sql` `import_assessment_question_pool_fork` atomically creates a fresh child Pool Revision and Assessment Entry from an exact reusable source Revision without accepting raw member pins.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `import_fork` passed accepted actual-server proof that imported source Pool `P8H3-QYX9` into a direct Assessment and returned distinct fork `VFH9-CQKS`, Revision 1, at Assessment Edit 2.
- [x] The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment_question_pool_fork` owns each child Pool through exactly one Assessment Entry, and `schemas/base_schema/question_pools.sql` retains exact source-Revision provenance.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `append_fork_revision` passed accepted actual-server proof that appended the fork's Revision 2 with the two exact member pins reversed, then reread the reusable source unchanged at Revision 1 with its original order. Artifact: `/private/tmp/ple-course-empty-artifacts.BbKFFd`.
- [x] Forking a Question Pool preserves its Published Questions by their public `AAAA-ZBBB` IDs.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool_revision_member` pins each ordered public Question identity and Revision, and `import_assessment_question_pool_fork` copies those exact immutable source members.
  - Evidence (runtime): `crates/server/src/assessment_pool_fork.rs` `import_fork` passed accepted actual-server proof that returned both source Question IDs and Revision 1 pins unchanged and in order in the fresh fork; subsequent Student selection retained one exact member pin.
- [ ] Question Pools work the same way regardless of the Question Backend.
  - Mismatch: incomplete secondary backend delivery leaves this unverified.
- [x] **Instructors** choose the contents of a Question Pool and how many Questions are selected.
  - Evidence (source): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` submits the Instructor's ordered current Published Question Revisions with interchangeability attestation; `src/pages/assessment_workspace/assessment_pool_entry_editor.tsx` `AssessmentPoolEntryEditor` exposes the Assessment-owned fork's exact members and bounded selection count.
  - Evidence (runtime): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `updatePoolSelectionCount` passed accepted actual-main proof that created the reusable Pool from ordered Published Questions, then imported it, changed its selection count from 2 to 1, attested and reordered its exact members, and reloaded its Revision 2 while the source remained unchanged. Artifacts: `/private/tmp/ple-course-empty-artifacts.bzwXEa` and `/private/tmp/ple-course-empty-artifacts.lgyOMK`.
- [x] PLE selects from the Question Pool; the selected Question Backend controls the Question interaction.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` performs server-owned selection.
- [x] Question Pool selection and backend-native randomization are separate forms of variation.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `QuestionPoolSelectionEntropy` is separate from Question backend state.
- [x] Returning to an Attempt preserves the Question Pool selections already made.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `assessment_attempt_start_gate` returns an unfinished resumable Attempt before new issuance, while `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` returns its retained committed presentations rather than selecting again.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `issue_native_assessment_batch` passed accepted actual-server proof that returned Attempt 1 with `resumed: true`, the same selected pin, and the same presentation nonce after its first start. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Starting a new Attempt makes fresh selections from its Question Pools.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `assessment_attempt_start_gate` has no prior-Pool-selection reuse branch; after a submitted Attempt it authorizes a new Attempt, whose new selection payload is persisted by `start_assessment_attempt`.
  - Evidence (runtime): `crates/server/src/assessment_delivery.rs` `start` passed accepted actual-server proof that submitted Attempt 1, then started Attempt 2 with `resumed: false`, a distinct Pool selection ID, and a new presentation nonce. The same selected member remained valid with a two-member Pool. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Student Work preserves the exact Question Pool Revision and Published Question Revision delivered.
  - Evidence (source): `crates/question_model/src/student_work.rs` `QuestionPoolSelection` retains issued Question revision references.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the issued revision link.
- [x] Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` retains `question_id` and `revision_number`.
  - Evidence (test): `crates/question_model/src/student_work/model_tests.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the exact issued linkage.
- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool_revision_member` stores each exact Published Question revision reference.
  - Owner: The first Question Pools occurrence owns this duplicate status.
- [ ] Question Pools contain only **Published Questions**; Question Pools cannot be members of Question Pools.
  - Verification pending: Source-contributor audit must confirm only exact Published Question Revision members and no Pool-member input; broad runtime evidence remains pending.

### Question Library

- [x] Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
  - Evidence (source): `src/pages/library_route_page.tsx` `LibraryRoutePage` is the production Instructor Library surface.
- [x] The Question Library is one global collection of published Question content.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question` is not course-scoped.
- [x] **Published Questions** are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/question_library_operations.sql` `question_library_entries` requires an active Instructor Account and exposes available Question summaries.
- [x] Published Question Pools are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `list_published_question_pools` and `read_current_published_question_pool` authorize active Instructors and project only public Pool/Revision/member facts.
  - Evidence (runtime): `crates/server/src/question_pool_library.rs` `list_pools` passed accepted actual-server proof that a second vetted Instructor listed and read root Pool `1N6T-MZRD` and child Pool `J1BX-8V8F` with exact public member pins and no Course facts. A nonmember Assessment-fork PUT returned 404 without mutation; Student and anonymous Pool list/read calls returned no-store 404. Artifact: `/private/tmp/ple-course-empty-artifacts.hvS4KT`.
- [x] **Students** access Question content through their Coursework rather than through the Question Library.
  - Evidence (source): `src/route_contract.ts` `ROUTE_CONTRACT` reserves both Question Library routes for Instructors, and `src/route_access_boundary.tsx` `withRouteAccessBoundary` fail-closes every protected route before its page component mounts.
  - Evidence (runtime): `src/route_access_boundary.tsx` `withRouteAccessBoundary` passed accepted actual-main Student proof that denied three Library routes without any Question Library API request, while the Student Ribbon allowed Coursework navigation to a Released Assessment. Earlier accepted native Student Attempt proof delivered Question content through that Assessment. Artifacts: `/private/tmp/ple-course-empty-artifacts.9s89JA` and `/private/tmp/ple-course-empty-artifacts.ZquNiI`.
- [x] Published content remains discoverable when used by a private **Course Instance**.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionSearchResult` is global and separately reports course use.
- [ ] With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
  - Mismatch: no product test or design enforcement establishes archive as non-primary at this scale.
- [ ] Question Library workflows should support bulk operations because an Instructor may manage thousands of Questions.
  - Evidence (test): temporary compiled Chromium component and strict-client proof accepted sorted selection/Edit Numbers, closed replace/clear patches, virtualization, busy controls, blank-replace rejection, pre-fetch canonical-ID rejection, stale/ambiguous refresh, denial, filter clearing, no page errors, and zero critical/serious axe findings; the mock/injected transport was not server-connected and the proof was removed.
  - Mismatch: connected HTTP and practical-scale workflow evidence remains pending.
- [x] Instructors should be able to select many Questions and update shared metadata such as tags, subject, topic, or other search fields together.
  - Evidence (source): `crates/question_model/src/question_library.rs` `PublishedQuestionSharedMetadata` is the closed shared-metadata DTO; generated contracts bound its collection to 1,000 items.
  - Evidence (source): `crates/server/src/question_library/shared_metadata.rs` `load_current_shared_metadata` owns the no-store current-metadata read; `crates/server/src/question_bulk_metadata.rs` owns the bounded all-or-none update command and validates canonical IDs before Store access.
  - Evidence (runtime): `crates/server/src/question_bulk_metadata.rs` `bulk_replace_metadata` passed fresh PostgreSQL 17 SQL/API proofs, independently rerun, covering read/write/read, stale all-or-none denial, clear, unauthorized, unvetted, archived, missing, and duplicate concealment, unchanged Revision count, and private-helper denial. Source review establishes once-only `PLE authoring`/`Pilot` initial tags for native publication and an empty WebWork start; separate PostgreSQL proof uses explicit initial tags, rejects null elements without database/publication/object side effects, and preserves an intentional empty clear in a successor Revision.
  - Evidence (test): `src/components/question_bulk_metadata_editor.tsx` `QuestionBulkMetadataEditor` passed temporary compiled Chromium component and strict-client proof for the selected metadata workflow, including stale and ambiguous refresh with no automatic second write; its mock/injected transport was not server-connected and the proof was removed.
  - Evidence (runtime): `src/pages/library_page.tsx` `QuestionBulkMetadataEditor` passed accepted isolated actual-server HTTP and private exact-main browser proof: selected Published Questions read current metadata/Edit Numbers, replaced tags and subject, cleared subject, and observed the search projection. An authorized second Instructor advanced one selected Question; a stale two-Question update returned 412 with no partial write, and the browser refreshed current values without automatically writing again. HTTP responses were `no-store`; anonymous and Student calls were denied. Source, Revision, and availability remained unchanged. The three-Question fixture does not establish thousands-Question or 13k cleanup practicality, backend rendering, or a canonical screenshot corpus; C366 and the practical-scale rows remain open.
- [ ] Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Mismatch: search, filters, and an accepted mock-transport browser metadata workflow exist, but connected HTTP and 13k practical-cleanup evidence remains pending.

#### Published Question identity

- [x] Published Questions receive a public `AAAA-ZBBB` Crockford Base32 ID.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId` defines and displays the canonical `AAAA-ZBBB` public Question ID; `crates/server/src/question_publication.rs` `NewQuestionLineagePublisher` issues it for a new Published Question lineage.
  - Evidence (runtime): `crates/server/src/question_publication.rs` `NewQuestionLineagePublisher` passed accepted actual-server proof that published two native Questions, whose exact public IDs then formed a reusable Pool's members. Artifact: `/private/tmp/ple-course-empty-artifacts.JTjOJ3`.
- [x] Published Questions and published Question Pools have public Crockford Base32 IDs.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId` is the common public ID model, while `schemas/base_schema/question_pools.sql` `question_pool` stores a unique public Pool ID.
  - Evidence (runtime): `src/components/question_pool_create_dialog.tsx` `QuestionPoolCreateDialog` passed accepted actual-main proof that displayed a server-issued public Pool ID alongside ordered exact public Question IDs. Artifact: `/private/tmp/ple-course-empty-artifacts.bzwXEa`.
- [x] Public IDs use the form `AAAA-ZBBB`.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionId` documents and formats the canonical `AAAA-ZBBB` display form, including the middle validation character.
  - Evidence (runtime): `crates/server/src/question_pool_creation.rs` `create_question_pool` passed accepted actual-server proof that returned a canonical public Pool ID at Revision 1. Artifact: `/private/tmp/ple-course-empty-artifacts.BhKHDp`.
- [x] Seven Crockford Base32 characters are cryptographically random and provide the identity.
  - Evidence (source): `crates/server/src/question_publication.rs` `question_id_from_random_bytes` derives the identifier from random bytes.
- [x] The middle character is an HMAC-derived check character calculated from the seven identity characters.
  - Evidence (source): `crates/server/src/question_publication.rs` `question_id_validation_character` derives the validation character with HMAC.
- [x] The check character detects mistyped or malformed IDs; it is not a security boundary.
  - Evidence (source): `crates/server/src/question_publication.rs` `validates_question_id` validates syntax/check character separately from authorization.
- [ ] ID generation enforces database uniqueness and retries when a random collision occurs.
  - Mismatch: database uniqueness exists, but collision retry behavior was not found in the issuer or publication store.
- [x] IDs never encode creation order, Question Type, ownership, subject, or other metadata.
  - Evidence (source): `crates/server/src/question_publication.rs` `question_id_from_random_bytes` uses random bytes and a secret only.

#### Published Question revisions, edits, and forks

- [x] **Published Questions** maintain immutable revision history.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `question_revision_is_immutable` trigger protects revision rows.
- [ ] Assessments and Student Work remain pinned to exact immutable Published Question Revisions.
  - Mismatch: exact revision columns are source evidence only; no connected test verifies an Assessment and Student Work stay pinned across a later publication.
- [x] Publishing a new Question Revision does not silently change existing Assessments or Student Work.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` publication appends `next_revision_number` rather than rewriting prior rows.
- [x] The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` `publish_question_revision` appends an owner-authored revision.
- [x] Changing Question source, answer content, grading rules, feedback, or Question assets creates a new Question Revision.
  - Evidence (source): `schemas/base_schema/question_publication_operations.sql` `publish_question_revision` persists a new source binding keyed to a new revision.
- [x] Changing the Question title, description, tags, subject, topic, or other search metadata does not create a new Question Revision.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` is separate from `question_revision`.
- [x] Search metadata belongs to the Published Question as a whole rather than to one Revision.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question_metadata` keys metadata to `question_id` only.
- [ ] Any **Instructor** may fork a Published Question to create a separate Question with a new Question ID.
  - Mismatch: draft-fork source support is source evidence only; no authorization or behavior test verifies any eligible Instructor can publish a separate ID.
- [x] A fork starts as a private **Draft Question** with its own authorship and lineage.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `draft_question_fork_source` records a private draft fork source.
- [x] A fork must pass Question Publication Validation before joining the Question Library.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `validate_question_publication` guards publication.
- [x] Published forks retain source attribution.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `question_fork_source` records published fork provenance.
- [x] Forced corrections are audited **Sysadmin** actions reserved for critical flaws.
  - Evidence (source): `schemas/base_schema/corrections.sql` `forced_question_correction` and its immutable audit targets record correction actions.
- [x] Question authorship, contributor credit, history, attribution, and compatible CC licensing are preserved across Revisions and forks.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `question_revision_authorship` and `question_revision_license` preserve revision stewardship.

#### Question stewardship

- [ ] Question stewardship should use a GitHub-like model.
  - Mismatch: star/watch and improvement-thread workflows are not all delivered.
- [ ] **Published Questions** can be starred and watched, similar to GitHub.
  - Mismatch: no Question star or watch persistence model was found.
- [x] Star means favorite and visible endorsement.
  - Evidence (source): `schemas/base_schema/question_stewardship.sql` `set_current_question_star` records an active Instructor's Star only for a Published Question; `src/components/question_star_control.tsx` `QuestionStarControl` provides the visible Star and count surface.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 with an active vetted Instructor's actual HTTP Star action and exact closed Star projection.
- [x] Vetted **Instructors** can see the star count and which vetted **Instructors** starred a Question.
  - Evidence (test): `tests/e2e/e2e_question_star_name_privacy.sh` `Question Star name privacy E2E` passed on 2026-09-15 against isolated PostgreSQL 17, MinIO, and `server_core`: the active vetted Instructor received only `{starCount, viewerHasStarred, starredInstructors:[{displayName}]}`; anonymous, Student, inactive-Instructor, and HMAC-valid non-Published requests were concealed with `404`.
  - Evidence (runtime): one-time Chromium proof for `src/components/question_star_control.tsx` `QuestionStarredInstructorList` passed on 2026-09-15. It rendered the exact server-shaped display-name list with an accessible name and no link, button, or avatar; the fixture was removed after review because it is implementation proof, not a stable product contract.
- [ ] Watch means subscription.
  - Mismatch: Question watches are not implemented.
- [ ] Watching drives in-app notifications for revisions, forks, improvement threads, and impact notices.
  - Mismatch: source-bound Watch events exist for revisions and forks, but improvement threads and impact notices have no product-defined model or private delivery behavior.
  - Question: For improvement threads, who may create/read/reply/edit/resolve them, which identity/attachments/linkage/notification/retention rules apply; and for impact notices, who may create them, under what condition, with what text/category/severity/manual-or-derived/linkage/audience/update/cancel rules?
- [ ] An **Instructor's** watch list remains private.
  - Mismatch: Question watches are not implemented.
- [ ] **Students** and anonymous users do not receive **Instructor** identity lists or watch information.
  - Mismatch: Question watch access controls are not implemented.

#### Question statistics

- [ ] Privacy-safe aggregate Question statistics remain after the underlying Student records are deleted.
  - Mismatch: retention transition needs runtime or connected-oracle proof.
  - Owner: `docs/active_plans/audits/hg_checklist_parts/06_data.md`
- [ ] Question statistics are kept separately for each Published Question Revision.
  - Mismatch: private aggregate capture exists, but no released Question Statistics surface establishes this product behavior.
- [ ] Each Published Question Revision may retain aggregate counts of correct, incorrect, partial-credit, and unanswered results.
  - Mismatch: private aggregate capture exists, but no released Question Statistics surface establishes this product behavior.
- [x] Eligible Question Types may also retain aggregate answer-choice counts.
  - Evidence (source): `schemas/base_schema/statistics.sql` `selected_count` stores aggregate choice counts.
  - Owner: `docs/active_plans/audits/hg_checklist_parts/06_data.md`
- [ ] Question-level statistics may combine Revisions when clearly labeled and privacy thresholds are met.
  - Mismatch: `QuestionStatistics` is currently `Unavailable`; no labeled combined-revision view exists.
- [ ] Question statistics contain aggregate counts rather than Student Attempts or identifiable Student records.
  - Mismatch: private aggregate evidence exists, but no released Question Statistics surface establishes the required product behavior.
- [ ] Student data retention removes the underlying Student evidence without removing approved aggregate Question statistics.
  - Mismatch: needs runtime or connected-oracle evidence for retention and aggregate preservation.
- [ ] Removing Student names alone does not make statistics anonymous.
  - Mismatch: no released Question Statistics policy establishes this behavior.
- [ ] Shared Question statistics should be shown only when individual Students cannot reasonably be identified from the aggregate.
  - Mismatch: `QuestionStatistics` is currently `Unavailable`; no shared-view privacy threshold exists.
- [ ] Course-specific Question analysis remains FERPA-sensitive when individual Students could be inferred.
  - Mismatch: aggregate analysis structures exist, but no complete FERPA-sensitive product workflow was verified.

#### Question behavior

- [ ] Answer-choice randomization belongs to the Question.
  - Mismatch: native answer-choice randomization ownership has not been verified.
- [ ] PLE-native Questions control their own answer-choice randomization.
  - Mismatch: no native answer-choice randomization implementation was found.
- [x] Question writers may add optional Question Feedback when it helps.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_feedback_fields.tsx` `PleQuestionJsonFeedbackFields` edits optional feedback.
- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_provided_feedback_is_shown` verifies provided feedback without answer disclosure.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` omits Question Feedback from the six-field disclosure timing rule; `crates/domain/src/student_feedback_release.rs` `project_student_feedback` always projects feedback supplied at the submitted-history boundary while gating Question Answer separately.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_provided_feedback_is_shown` verifies that separation.
- [ ] Student workflows remain complete whether or not Students read Question Feedback.
  - Verification pending: no end-to-end Student workflow proof covers completion when a Question provides feedback, when it provides none, and when the Student does or does not read the supplied feedback.
## Courses

- [ ] **Courses** organize reusable teaching content and its delivery to **Students**.
  - Mismatch: The current Course model does not establish the complete stated product boundary.
- [ ] PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
  - Mismatch: The current paths implement related records but do not verify the complete product distinction.
- [ ] **Blueprint Courses** provide reusable course designs for creating Course Instances.
  - Mismatch: Adoption is implemented only for the current stored Blueprint shape.
- [x] Course Instances may be created from a Blueprint Course or started empty.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CourseInstanceCreationSource` and `src/api/decoders/course_instance.ts` `decodeCreateCourseInstanceInput` accept strict Empty or exact Adopted source forms.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised against the actual server in bounded exact-main browser proof: Empty creation persisted without Blueprint-list requests; separate Public Blueprint exact-Revision adoption created a daughter Course and Unreleased Practice Assessment. Successful API responses were `no-store`. This creation-only row does not establish direct started-empty Assessment authoring or the full teaching lifecycle.
- [ ] A Course can have multiple co-**Instructors** with equal teaching authority.
  - Mismatch: The schema has an assigned Instructor distinction, not verified equal co-Instructor authority.
- [ ] **Sysadmins** can create Courses, but **Instructors** teach them.
  - Mismatch: Sysadmin creation selection exists, but role behavior is not fully verified.
- [x] Every Course Instance must have at least one assigned **Instructor**.
  - Evidence (source): `schemas/base_schema/course_membership.sql` `assert_assigned_instructor_membership` rejects a Course Instance without a current assigned Instructor membership.
- [ ] Creating a Course Instance establishes its first Instructor membership but does not give that Instructor greater Course authority than later co-Instructors.
  - Mismatch: `CourseInstanceView.is_assigned_instructor` exposes a special authority distinction.

### Blueprint Courses

- [x] **Blueprint Courses** are reusable course definitions for building **Course Instances**.
  - Evidence (source): `crates/learning-data-access/src/blueprint_course.rs` `BlueprintCourseStore` persists reusable Blueprint content and revisions.
- N/A Blueprint Courses are a similar concept as LibreTexts' ADAPT alpha courses.
  - Reason: This comparison provides human-oriented product context, not an implemented PLE behavior.
- [x] Blueprint Courses have no **Students**, deadlines, or other teaching-specific delivery settings.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course_revision` stores reusable content without Course Instance delivery fields.
- [x] Blueprint Courses do not contain dates or relative schedules.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course_revision` has no date or schedule columns.
- [ ] Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` lists Public Blueprints to active Instructors.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the full vetted-Instructor visibility and reusability claim.
- [ ] Blueprint Courses contain only **Published Questions** and published **Question Pools**.
  - Mismatch: Current pin validation covers Question revisions but not the required published Pool behavior.
- [ ] An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.
  - Mismatch: No Course Instance-to-Blueprint publishing route or store operation was found.

#### Blueprint Course lifecycle

- [ ] Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course.availability` permits `private`, `public`, and `archived`.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against the complete state claim.
- [ ] New Blueprint Courses and forks start Private.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.create_blueprint_course` and `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` create Private lineages.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against both creation paths.
- [ ] Private Blueprint Courses are visible only to their owning **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` and `ple_api.load_blueprint_course` limit Private access to the owner.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the owner-only claim.
- [ ] Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Public availability.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle receipt against the Private-adoption denial.
- [ ] Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` lists Public Blueprints to active Instructors.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the full vetted-Instructor visibility and reusability claim.
  - Owner: Same implementation finding as the earlier Public Blueprint Courses bullet.
- [ ] Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Public availability for adoption.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against the complete Public-adoption workflow.
- [x] Archived Blueprint Courses are read-only and no longer actively maintained.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course` and `ple_api.rename_blueprint_course` lock the owner-visible Blueprint and reject `archived` before replay, CAS, or no-op handling.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` covers denied replay, no-op, changed Save, and rename without changing the Blueprint state, then restored writes.
  - Evidence (runtime): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course`; `/private/tmp/ple-daughter-revision-notice-artifacts.JhV6aj/archived-blueprint-http-proof.json` records five `409` denials with unchanged state and preserved Private/Public/restored writes.
- [x] Archived Blueprint Courses remain visible by every vetted **Instructor**.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` defaults `p_include_archived` to false and returns Archived records when that explicit parameter is true.
  - Evidence (runtime): `crates/server/src/blueprint_course.rs` `list_blueprints`; `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-http-proof.json` records owner and nonowner active-Instructor default/false/true lists: only true includes the Archived Blueprint, while the Private Blueprint remains owner-only. The same receipt records nonowner Archived detail `200`, Private detail `404`, Student list/detail `404`, and invalid query values `400`.
- [x] Archived Blueprint Courses are excluded from normal search results unless the search explicitly includes them.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` filters Public, owning Private, and only explicitly requested Archived records; `crates/server/src/blueprint_course.rs` `BlueprintCourseListQuery` accepts only the typed `includeArchived` boolean.
  - Evidence (runtime): `src/features/blueprint_course/blueprint_course_workspace.tsx` `changeIncludeArchived`; `/private/tmp/ple-archived-discovery-artifacts.1q5ste/archived-discovery-browser-proof.json` records the actual compiled-main default-off, Include Archived, read-only Archived-detail, and return-to-off workflow with eight GETs and zero writes. Its companion HTTP receipt records default/false/true membership and strict invalid-query `400` results.
- [x] Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires Blueprint availability `public` for exact-Revision adoption.
- [ ] Archived Blueprint Courses can be forked but not adopted.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` accepts Public or Archived sources, while adoption requires Public availability.
  - Mismatch: `src/api/blueprint_course.ts` has no Instructor fork client method, and `BlueprintCourseLifecycleControls` has no fork action.
- [ ] The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` `restore_blueprint` sets availability to `public`.
  - Verification pending: Reconcile the existing connected Blueprint lifecycle and actual HTTP receipts against restore followed by adoption.
- [ ] Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
  - Evidence (source): `schemas/base_schema/blueprint_lineage.sql` `ple_api.fork_blueprint_course` accepts Archived sources and creates a Private child owned by the actor.
  - Mismatch: `src/api/blueprint_course.ts` has no Instructor fork client method, and `BlueprintCourseLifecycleControls` has no fork action.
- [x] Blueprint Courses have no separate draft state.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `CHECK (availability IN ('private', 'public', 'archived'))` defines the complete Blueprint availability state.

#### Blueprint Course revisions

- [x] Blueprint Courses use immutable **Blueprint Revisions** for saved reusable content.
  - Evidence (source): `schemas/base_schema/blueprint_revision_integrity.sql` `blueprint_course_revision_is_immutable` rejects Revision updates and deletes.
- [x] Blueprint Course content editing uses explicit Save.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `save_blueprint` is the explicit content-save route handler.
- [x] Saving changed Blueprint content creates the next Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course` inserts the next `blueprint_course_revision` when `changed` is true.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts one changed Save creates one new Revision.
- [x] Multiple content edits before Save become one Blueprint Revision.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `ReplaceBlueprintCourseContentInput` carries one complete replacement tree per Save.
- [x] Saving unchanged Blueprint content does not create another Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.save_blueprint_course` returns the expected Revision without inserting when `changed` is false.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts a canonical no-op Save returns Revision 2 with `changed` false.
- [x] Blueprint Course metadata can change without creating a Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.rename_blueprint_course` updates `blueprint_course` metadata without inserting a `blueprint_course_revision`.
- [x] Blueprint Course names are metadata and identify the Blueprint across Revisions.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course` owns names while `blueprint_course_revision` keys content by course reference and revision.
- [x] Changing a Blueprint Course name does not create a new Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.rename_blueprint_course` updates names and metadata ETag without inserting a `blueprint_course_revision`.

#### Blueprint Course stewardship

- [ ] **Instructors** can Star or Watch Public and Archived Blueprint Courses.
  - Mismatch: No Blueprint Star or Watch model, route, or store operation was found.
- [ ] A Star is a visible endorsement and helps **Instructors** save useful Blueprint Courses.
  - Mismatch: No Star model or presentation was found.
- [ ] Vetted **Instructors** can see who Starred a Blueprint Course and its Star count.
  - Mismatch: No Star model or presentation was found.
- [ ] Watching a Blueprint Course is private.
  - Mismatch: No Watch model or presentation was found.
- [ ] Watchers are notified about new Blueprint Revisions and other important Blueprint changes.
  - Mismatch: No Watch or notification implementation was found.
- [ ] Forking or adopting a Blueprint Course does not automatically Star or Watch it.
  - Mismatch: No Star, Watch, or fork implementation exists to verify this invariant.
- [ ] Stars and Watches belong to the Blueprint Course across all of its Revisions.
  - Mismatch: No Star or Watch persistence exists.

#### Blueprint adoption and updates

- [x] Blueprint adoption copies every Assessment from the Blueprint Course into the Course Instance.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_instance.rs` `create_course_instance` obtains `creation_assignments` before atomic creation.
- [x] Course Instances pin the exact Blueprint Revision from which they were adopted.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CourseInstanceCreationSource` requires an exact immutable Blueprint Revision source for adoption.
- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] It should be obvious when a Course Instance is using an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` carry the adopted and current Revision numbers and render the older-Revision notice with strict `bigint` comparisons.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was exercised by accepted independent actual-server/exact-main browser proof across empty, current, newer, and explicit synthetic Private-origin states. The newer state visibly showed its original adopted Revision and the current newer Revision; Student and unrelated-Instructor reads returned nonenumerating `404 no-store`, no extra Blueprint fetch or write occurred, and browser errors were empty. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This read-only notice makes a stale daughter obvious. It does not offer, review, approve, or apply a Blueprint update.
- [x] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` discovers each adopted Assessment from one current parent Revision; `src/api/assessment_release.ts` `LiveAssessmentReleaseClient` defines the existing detail Apply with source-Revision and daughter-Edit CAS. No persisted offer, receipt, comparison baseline, or new update table is introduced.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: changed, matching, removed-source, and Type-mismatch existing Assessments were classified before the changed one was explicitly applied. The Course-summary flow is distinct from the earlier per-Assessment proof of exact pins, stale-CAS/no-op/invalid-Released rollback, dates, status, origin, and one populated Assessment Attempt hash at `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`; neither artifact claims all Student Work. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` validates the new-reference delta and inserts only new Assessments; it does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` changes a retained source Assessment title and proves existing daughter Assessment content, entries, and actual Student Work unchanged through Save/replay/no-op/stale operations. Accepted artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: This negative invariant remains separate from the verified Course-review workflow; it does not claim direct-Assessments or all Student Work.
- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (source): `crates/learning-data-access/src/postgres/blueprint_course.rs` Save and `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` atomically append only newly added Assessments to daughters with fresh Course-owned Pool identities and unset dates.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` passed connected PostgreSQL 17 proof, preserving exact pins/settings, existing Assessment content and actual Student Work, and the original adoption Revision pin across two daughters including an inactive Course; an unrelated empty Course remained unchanged. Replay/no-op/stale saves made no duplicate append. Artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8` also accepted temporary bad-payload rollback proof. Existing connected adoption lifecycle regression passed 1 test with 0 ignored: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: Only automatic-new Assessment propagation is verified, not C410 existing-Assessment update offers or the whole Course milestone.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` now invokes the `append.rs` helper `assert_new_assessment_save_preserves_daughter_work`; the existing permanent lifecycle regression passed 1 test with 0 ignored in `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. No separate seed-sharing test was retained.

#### Blueprint Course forks and Change Proposals

- [ ] An **Instructor** can fork a **Blueprint Course** to create a new independent Blueprint Course.
  - Mismatch: No Blueprint fork operation was found.
- [ ] A fork records the source Blueprint Course and Blueprint Revision from which it was created.
  - Mismatch: No fork lineage persistence exists.
- [ ] Forked Blueprint Courses develop independently and have their own Blueprint Revisions.
  - Mismatch: No fork implementation exists.
- [ ] Changes to a source Blueprint Course are never automatically applied to its forks.
  - Mismatch: No fork implementation exists to enforce this invariant.
- [ ] A fork should make newer changes from its source Blueprint Course easy to discover and review.
  - Mismatch: No fork update UI or runtime proof exists.
- [ ] An **Instructor** can selectively bring changes from a source Blueprint Course into their fork.
  - Mismatch: No selective fork-update operation exists.
- [ ] An **Instructor** can create a **Blueprint Course Change Proposal** to propose changes to another Blueprint Course.
  - Mismatch: No Change Proposal model, route, or store operation was found.
- [ ] A Change Proposal shows added, removed, and changed Assessments and Question content.
  - Mismatch: No Change Proposal comparison implementation was found.
- [ ] The receiving **Instructor** decides which proposed changes to accept.
  - Mismatch: No Change Proposal acceptance operation exists.
- [ ] Accepted changes create a new Blueprint Revision of the receiving Blueprint Course.
  - Mismatch: No Change Proposal acceptance operation exists.
- [ ] Change Proposals never directly change daughter Course Instances.
  - Mismatch: No Change Proposal implementation exists to verify this invariant.
- [ ] Daughter Course Instances receive accepted changes through the normal Blueprint update workflow.
  - Mismatch: No Change Proposal or Blueprint update workflow exists.

#### Blueprint Course JSON

- [ ] Blueprint Courses have a canonical JSON representation for comparison, import, export, and exchange.
  - Mismatch: Current JSON is internal stored content; no canonical import/export exchange surface was found.
- [ ] Canonical Blueprint JSON must contain enough information to fully recreate a Blueprint Course.
  - Mismatch: No complete export/import round trip was found.
- [ ] Importing exported Blueprint JSON should reproduce the same Blueprint Course content and structure.
  - Mismatch: No Blueprint JSON import or export operation exists.
- [ ] Blueprint JSON contains Blueprint metadata and an ordered list of Blueprint Assessments.
  - Mismatch: Stored revision JSON does not demonstrate the required complete canonical exchange shape.
- [x] Blueprint Assessments contain only reusable teaching settings.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_data.blueprint_content_is_closed` allowlists reusable Assessment content and defaults without Course delivery dates or release state.
- [ ] Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Mismatch: Current stored content pins Questions but does not verify published Pool support.
- [x] Blueprint Assessments have no deadlines, release dates, Student data, or other Course Instance settings.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course_revision` and its children have no Student or delivery-date fields.
- [ ] Blueprint Revisions can be compared through their canonical JSON representations.
  - Mismatch: No canonical JSON comparison surface was found.
- [ ] Blueprint Course Change Proposals use canonical JSON to identify changes between Blueprint Revisions.
  - Mismatch: No Change Proposal implementation exists.
- N/A Canonical Blueprint JSON may support offline inspection or editing, even if it is not optimized for hand editing.
  - Reason: This explicitly optional future capability does not require implemented behavior.
- [ ] Canonical Blueprint JSON is the complete exchange format, not the primary persistence model.
  - Mismatch: No canonical exchange format implementation exists.

### Course Instances

#### Course Instance creation

- [x] An **Instructor** can create a Course Instance from a Public Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.load_course_instance_blueprint` requires a Public Blueprint at the selected exact Revision; `src/pages/course_list_page.tsx` `TeachingCourseListPage` exposes the Adopted source only after public Blueprint discovery.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised in private actual-HTTP and exact-main browser proof: an Instructor created and published a Blueprint through its API, selected its exact Public Revision, created a daughter Course Instance, and read its Unreleased Practice Assessment with finite Attempt limit and dates unset. This does not establish Pool copying or release/delivery workflows.
- [x] **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
  - Evidence (source): `src/pages/course_list_page.tsx` `TeachingCourseListPage` defaults to Empty, activates Blueprint discovery only for Adopted, and sends the strict `source: { kind: "empty" }` wire through `src/api/http_client/course_instance.ts`.
  - Evidence (runtime): `src/pages/course_list_page.tsx` `TeachingCourseListPage` was exercised in a bounded authenticated actual-main browser and HTTP proof: an Instructor created an Empty Course Instance, then the resulting row and persisted Course read were observed, with zero Blueprint-list requests and `no-store` responses. Student creation denial was exercised at the HTTP boundary.
- [ ] Course Instances have **Students**, deadlines, releases, and other delivery-specific settings.
  - Mismatch: This audit has not found the complete Course Instance delivery model in A8 paths.
- [ ] Course Instances contain only **Published Questions** and published **Question Pools**.
  - Mismatch: Current adoption validates Question pins but not required published Pool behavior.
- [x] Course Instances are visible only to their co-**Instructors** and enrolled **Students**.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `resolve_course_navigation` permits only an active Course Member.
- [ ] Active Courses are current teaching Course Instances.
  - Mismatch: No active/inactive Course Instance lifecycle model was found.
- [ ] Inactive Courses are past Course Instances and retain Course metadata, including after
  FERPA-sensitive Student data is removed.
  - Mismatch: No inactive Course lifecycle and retention linkage was verified in A8 paths.
- [ ] An **Instructor** may deliberately publish reusable Course Instance structure as a new **Blueprint Course**.
  - Mismatch: No Course Instance-to-Blueprint publishing operation exists.
- [x] A new academic term uses a new Course Instance. Rollover is not a separate product model.
  - Evidence (source): `crates/question_model/src/course_term.rs` `CourseTerm` is input to each `CreateCourseInstanceInput`; no rollover model was found.

#### Blueprint adoption and daughter Course Instances

- [x] An **adoption** occurs when an **Instructor** creates a Course Instance from a Blueprint Course.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_instance.rs` `create_course_instance` consumes Blueprint source inputs during creation.
- [x] Blueprint Courses track how many Course Instances have been created from them as their adoption count.
  - Evidence (source): `schemas/base_schema/blueprint_operations.sql` `ple_api.list_blueprint_courses` computes `total_adoptions` by counting Course Instances with each Blueprint reference.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts the adopted Blueprint summary has `total_adoptions` equal to 1.
- [x] A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance` records Blueprint reference and Revision source columns.
- [x] A daughter Course Instance records its parent Blueprint Course and the exact Blueprint Revision used to create it.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` includes `blueprint_course` and `blueprint_revision`.
- [x] Creating a Course Instance from a Blueprint Course counts as an adoption of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance_creation_event` records the Blueprint reference and Revision at creation.
- [x] The new Course Instance receives every Assessment from the selected Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` constructs the Course Assessments from selected Blueprint content.
- [ ] Creating a Course Instance from a Blueprint Course copies its Assessments, Questions, Question Pools, and reusable settings.
  - Mismatch: Current adoption evidence does not verify published Pool copying.
- [x] Course Instance Assessments created from a Blueprint Course start unreleased with dates unset.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` initializes adopted Assessments as unreleased with delivery dates unset.
- [x] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` lazily obtains the authorized current-parent Course summary and offers each adopted Assessment for review; `src/api/assessment_release.ts` `CourseBlueprintUpdateReview` excludes direct local Assessments and carries matching, removed-source, Type-mismatch, changed, and automatically-added correspondences.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: each Course-summary read returned five coherent rows (changed, matching, removed, Type mismatch, automatically added) after lazy open/reopen at 1280 by 900 and 390 by 844. The changed Assessment then reviewed and applied with exact source Revision 2 and daughter Edit CAS; the Course refresh showed the applied match. Student and unrelated reads returned `404 no-store`; a private parent was concealed from another Instructor in the privileged-availability fixture; Archived review remained available and new adoption was denied. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: Same verified Course-review workflow as the earlier Blueprint update offer bullet.
- [x] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` supplies one Course-level Review action, clear per-Assessment status labels, Refresh, and links to the existing Assessment detail Review/Apply workflow.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in compiled-main browser proof at 1280 by 900 and 390 by 844: lazy open/reopen GET behavior produced the five-row Course summary and the Course-to-Assessment detail review. Cancel issued zero POST requests; Apply used exact source Revision 2 plus daughter Edit CAS and a returning Course refresh showed the match. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: Same verified Course-review workflow as the earlier routine Blueprint updates bullet.
- [x] It should be obvious when a daughter Course Instance is using an older Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_operations.sql` `ple_api.load_course_instance`, `crates/learning-data-access/src/postgres/course_instance.rs` `decode_view`, `src/api/decoders/course_instance.ts` `decodeCourseInstanceView`, and `src/pages/course_instance_page.tsx` `CourseInstancePage` use the authorized parent origin and exact adopted/current Revision projection for the same visible notice.
  - Evidence (runtime): `src/pages/course_instance_page.tsx` `CourseInstancePage` was covered by independently accepted actual-server/exact-main proof across empty, current, newer, and explicit synthetic Private-origin states; the visually inspected newer capture showed both Revision values and the stale notice. It preserved the original adoption pin, Assessment, and entries; its Work tables were empty, so this proof makes no populated-Student-Work claim. Unauthorized Student and unrelated-Instructor reads returned `404 no-store`. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.u1qUyY`.
  - Decision: This duplicate course-view indication does not implement the separate Blueprint update offer, review, approval, or apply workflow.
- [x] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Evidence (source): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` discovers each adopted Assessment from one current parent Revision; `src/api/assessment_release.ts` `LiveAssessmentReleaseClient` defines the existing detail Apply with source-Revision and daughter-Edit CAS. No persisted offer, receipt, comparison baseline, or new update table is introduced.
  - Evidence (runtime): `src/pages/course_blueprint_update_review.tsx` `CourseBlueprintUpdateReviewList` was accepted in actual-server and compiled-main proof: changed, matching, removed-source, and Type-mismatch existing Assessments were classified before the changed one was explicitly applied. The Course-summary flow is distinct from the earlier per-Assessment proof of exact pins, stale-CAS/no-op/invalid-Released rollback, dates, status, origin, and one populated Assessment Attempt hash at `/private/tmp/ple-daughter-revision-notice-artifacts.kE8MnT`; neither artifact claims all Student Work. Artifact: `/private/tmp/ple-daughter-revision-notice-artifacts.zVOyqd`.
  - Owner: Same verified Course-review workflow as the earlier existing-Assessment update bullet.
- [x] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` and the PostgreSQL Blueprint Store Save implement the same automatic-new append boundary documented in the earlier identical row.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` accepted connected proof and existing adoption lifecycle regression preserve exact pins/settings, distinct daughter Pool IDs, existing Student Work, original adoption pin, Unreleased state, and unset dates. Artifacts: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8` and `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Owner: Same implementation finding as the earlier newly added Blueprint Assessments bullet.
- [x] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` inserts only validated newly added Assessments and does not update existing daughter Assessments.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` proves the same negative invariant after changing retained source content, preserving daughter content/entries/actual Student Work through Save/replay/no-op/stale operations. Artifact: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Owner: Same implementation finding as the earlier non-silent Blueprint changes bullet.

### Course names

- [x] Blueprint Courses and Course Instances each have their own short name and long name.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `CreateBlueprintCourseInput` and `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` each own both names.
- [x] Short names are entered or chosen deliberately by **Instructors**.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `pub short_name: String` is a submitted validated field.
- [ ] Short names are for compact navigation and should stay under about 16 characters when practical.
  - Mismatch: Current name validation permits up to 200 or 500 characters without the stated compact guidance.
- [x] Long names are descriptive names used for headings, breadcrumbs, and Course listings.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `pub long_name: String` is documented as the heading and breadcrumb name.
- N/A A Blueprint Course might be `Biochemistry` / `Upper-Level Introductory Biochemistry`.
  - Reason: This is an illustrative name example, not an implementation requirement.
- N/A A Course Instance might be `BCHM 355/455` / `BCHM 355/455 Section 20 Biochemistry (Roosevelt U; Spring 2026)`.
  - Reason: This is an illustrative name example, not an implementation requirement.
- [x] Course Instance names are properties of the Course Instance and are not derived from Blueprint Course names.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` requires independently supplied `short_name` and `long_name`.
## Assessments

- [ ] **Assessment** is the PLE object for organizing Questions into a graded or practice activity.
  - Evidence (source): `schemas/base_schema/assessments.sql` defines the Course aggregate as `ple_data.assessment`; current Type, release, Attempt, and API boundaries use that name.
  - Mismatch: Source naming is current, but the complete graded-or-practice product behavior requires the remaining delivery and content verification below.
- [x] PLE has **Blueprint Assessments** and **Course Instance Assessments**.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` is the reusable Blueprint Assessment aggregate; `schemas/base_schema/assessments.sql` `ple_data.assessment` is the current Course Instance Assessment aggregate with a required `course_id`.
- [x] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains Type, title, instructions, ordered entries, and validated `BlueprintAssessmentDefaults`; `BlueprintCourseModuleContent` owns those Assessments in Blueprint Course content.
- [ ] Course Instance Assessments deliver Questions to **Students**.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` requires a released Course Assessment and current Student Course record before issuing Questions.
  - Mismatch: Source establishes the delivery boundary, but complete Student delivery acceptance remains separately open.
- [x] All Assessments use the same underlying Assessment model.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` and `crates/learning-data-access/src/assessment_release.rs` `LiveAssessmentWorkspace` share canonical Assessment Type, title, instructions, activity rules, and Student feedback rules. `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `assessment_input` materializes `SaveLiveAssessmentInput` through `assessment_values_json` into ordinary `ple_data.assessment` rows; distinct reusable and delivery storage/lifecycle projections are not separate pedagogical models.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 connected `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` passed 1 test with 0 ignored. Its `crates/learning-data-access/tests/blueprint_course_postgres/adoption.rs` `assert_adoption_projection` verified preserved Assessment Type, mixed ordered Pool/Fixed entries, nondefault points/scoring/retry/timing/activity/feedback rules, exact Revision pins, fresh independent daughter Pool IDs, and unset delivery dates. Supplemental read-only SQL verified two adopted Regular Assignments retained the source Type and were Unreleased with null dates. Artifact: `/private/tmp/ple-shared-assessment-adoption-artifacts.IkYuXY`. This architecture receipt does not establish every Type's Student delivery or completion.
- [ ] **Assignment** is not a separate object or category. The word appears only in the names
  **Regular Assignment**, **Practice Question Assignment**, and **Bonus Assignment**.
  - Evidence (source): `schemas/base_schema/assessments.sql`, Assessment Attempt SQL, and browser APIs use `assessment` generally; the closed Type set retains Assignment only in the three specified Type names.
  - Mismatch: A complete title/reference inventory and legacy-consumer cutover verification remain open.

### Assessment content

- [x] Assessments contain an ordered sequence of Published Questions and Question Pools.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` renders the mixed entry sequence; `schemas/base_schema/assessments.sql` `assessment_entry_active_authored_position_key` enforces distinct current positions with a deferred constraint, allowing atomic swaps and retired-position reuse. `schemas/base_schema/assessment_operations.sql` `ple_api.load_assessment_workspace_rows` projects only available current entries.
  - Evidence (runtime): accepted independent actual-server/private bundled-main browser proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` saved Fixed A, an imported Pool, and Fixed B, moved the top-level Pool across Fixed A, and reloaded exact ordered entry IDs and Revision pins. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`.
- [ ] Published Questions stay references to the same Question ID and exact Revision.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.replace_assessment_entries` preserves retained fixed-entry Question IDs and Revision Numbers and requires both values for saved entries.
  - Mismatch: The Assessment authoring input still permits ID-only Question selection; require an exact Revision at that input boundary before this invariant can be verified.
- [ ] Question Pools are copied by forking when added to another Assessment.
  - Verification pending: Source-contributor audit must confirm the import path creates an Assessment-owned fork rather than a reusable Pool copy; broad runtime evidence remains pending.
- [ ] A newly forked Question Pool initially contains the same Published Question IDs and exact Revisions as its source.
  - Verification pending: Source-contributor audit must confirm the import path carries every exact source Question ID and Revision into the new fork; broad runtime evidence remains pending.
- [ ] A forked Question Pool can be changed independently without changing its source Question Pool.
  - Verification pending: Source-contributor audit must confirm independent fork revision writes and source preservation; broad runtime evidence remains pending.
- [x] Published Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` branches on `fixedQuestion` and `questionPool`; `schemas/base_schema/assessments.sql` retains separate entry-kind storage and Assessment-owned Pool identity within one authored-position sequence.
  - Evidence (runtime): accepted mixed-entry proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` reloaded distinct Fixed Question and Pool entries with exact pins, then retained the retired Pool's old Revision while a reimport received a distinct fork entry ID and Pool ID. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. Existing connected adoption regression also passed 1 test with 0 ignored under the corrected SQL, with two supplemental Type projections retaining unchanged pins: `/private/tmp/ple-shared-assessment-adoption-artifacts.UmP416`.
- [x] **Instructors** can add, remove, and reorder Published Questions and Question Pools.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` supplies Question addition, Pool import, and mixed-entry move/remove controls; `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` reaches `schemas/base_schema/assessments.sql` `ple_data.replace_assessment_entries` through authorized Instructor persistence.
  - Evidence (runtime): accepted actual-server/private bundled-main browser proof at `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` added both kinds, reordered the Pool across a Fixed Question, removed Fixed B and then the Pool, and saved/reloaded their absence from current content. Private retired IDs and the old Pool Revision remained; source Pool JSON was unchanged, and reimport created distinct fork entry/Pool IDs. Student direct real HTTP returned 404 without current-state change; browser error arrays were empty. Artifact: `/private/tmp/ple-assessment-mixed-entries-artifacts.CogOX1`. This does not establish Student Work history, full Live Demo, authentication/TLS, or WeBWorK delivery.
- [x] Assessment Question-order randomization is called **Randomize question order**.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` renders the exact accessible checkbox label **Randomize question order**.
  - Evidence (runtime): the earlier accepted part 04 actual-main/HTTP receipt saved and reloaded this visible checkbox through `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`; this is separate from the mixed-entry artifact. C64 owns persisted Question-order behavior. This label receipt does not claim C505's planned filename rename or whole-milestone completion.

### Assessment types

- [x] PLE defines the available Assessment Types.
  - Evidence (source): `crates/question_model/src/assessment.rs` `AssessmentType` is the canonical closed model, and `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` carries it to the browser contract.
- [ ] Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states all five Instructor-selected purposes; `AssessmentWorkspaceCreatePage` and `BlueprintCourseCreateDialog` render the selected description. `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment`, `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults`, `StudentFeedbackReleaseRule::for_assessment_type`, and the curriculum publisher apply Type-aware Attempt/disclosure defaults.
  - Mismatch: Purpose classification and the accepted Attempt/disclosure subsets are established, but complete appropriate-default coverage remains owned by this broad row. This receipt does not claim every Type's persisted/runtime settings or Student delivery, enforce collaboration policy, infer learning age, or supply an Instructor exam calendar.
- [x] Assessment Types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
  - Evidence (source): `crates/question_model/src/assessment.rs` `AssessmentType::ALL` contains exactly the five HG values; schema constraints and `generated/api/AssessmentType.ts` `ASSESSMENT_TYPE_VALUES` use the same closed set.
- [x] **Instructors** select an Assessment Type but cannot create new Assessment Types.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` requires one canonical Type before creation; `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog` does the same for reusable content, with no free-form Type path or silent default.
  - Evidence (test): `tests/test_blueprint_course_ui.mjs` `createdContent` proves the selected canonical Type reaches creation; the focused Blueprint UI/model lane passed 10/10.
- [x] Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `assessment_type` and `schemas/base_schema/assessments.sql` `assessment_type` validate the same five values; `schemas/base_schema/course_blueprint_adoption.sql` `assessment_type` copies the selected Type during adoption.
- [x] **Instructors** can change Assessment settings independently of the defaults for its Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` resolves explicit Course Instance property overrides separately from Blueprint defaults and Assessment Type.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-Store receipt loaded an Instructor Exam, saved its policy settings, and retained its Type through `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore`.
- [x] Changing Assessment settings does not change its Assessment Type.
  - Evidence (source): `crates/domain/src/effective_assessment_properties.rs` `EffectiveAssessmentPolicy` does not expose Type as an editable property, while `crates/question_model/src/assessment.rs` `AssessmentType` remains part of Assessment identity.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-Store receipt saved Instructor Exam policy settings while retaining its Type through `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore`.
- [x] **Regular Assignments** give **Students** regular practice applying course ideas outside class.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states practice applying course ideas outside class in the Regular description; `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` and `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog` render that Instructor-selected purpose at both real creation entry points.
- [x] Regular Assignments reinforce current learning and may also introduce new topics.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states both reinforcement of current learning and introduction of new topics in the Regular description; both real creation surfaces render it. The Instructor chooses content and its pedagogical classification; PLE does not infer material age.
- [x] Regular Assignments are designed as practice for learning, not merely as one-time assessments.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` identifies learning practice in the Regular description. `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` and `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults` leave Regular Attempt limits unset, unlike Quiz/Exam's one-Attempt rule; this is a purpose/default receipt, not complete Student delivery acceptance.
- [x] **Practice Question Assignments** provide focused review or study-guide practice using material already covered.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states focused review/study-guide practice of covered material in the Practice description; `AssessmentWorkspaceCreatePage` and `BlueprintCourseCreateDialog` render that Instructor-selected purpose. PLE does not infer when material was taught.
- [x] Practice Question Assignments may be worth a small number of points or a small amount of extra credit.
  - Evidence (source): `src/pages/assessment_workspace/assessment_fixed_question_points_editor.tsx` `AssessmentFixedQuestionPointsEditor` edits nonnegative Question point values independently of Type; `src/pages/assessment_workspace/assessment_workspace_questions_page.tsx` `AssessmentWorkspaceQuestionsPage` offers `extraCredit` entry scoring. `schemas/base_schema/grading.sql` `score_recorded_credit` retains earned points while `grade_contribution_points_possible` excludes extra-credit points from the denominator for Practice as well as other Types. The Instructor chooses the amount; no arbitrary numeric definition of small is imposed.
- [ ] Practice Question Assignments use the same whole-Attempt submission boundary as every other
  Assessment and show the correct answer immediately after that Assessment Attempt is submitted.
  - Evidence (runtime): accepted PostgreSQL 17 proof through the ordinary start, whole-submission, and history APIs returned zero history response-source rows before submission and one after submission; the native PLE summary then preserved the disclosed correct answer.
  - Mismatch: Backend-owned answers currently project as absent. Opaque WeBWorK post-submit answer disclosure remains unimplemented and requires renderer/adapter work without answer extraction, so the cross-backend row remains open.
- [x] **Bonus Assignments** provide optional extra credit.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states optional extra credit in the Bonus description at both Instructor creation surfaces; `schemas/base_schema/grading.sql` `grade_contribution_points_possible` gives Bonus a zero grade denominator without discarding earned points.
  - Evidence (runtime): the separately accepted neighboring Bonus grading receipt returned and rendered `8 / 0` through `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments` and the M6 component. Optional is the Instructor-selected purpose, not a new completion/required-work policy field.
- [x] Bonus Assignments are worth zero points possible and add earned points directly to the grade.
  - Evidence (source): `schemas/base_schema/grading.sql` `grade_contribution_points_possible` makes Bonus points possible zero while `score_recorded_credit` continues to supply earned points; `schemas/base_schema/grading_access.sql` applies that contribution through the real Gradebook helper, and `schemas/base_schema/student_assessment_landing.sql` projects the same selected contribution to the Student API.
  - Evidence (runtime): accepted actual PostgreSQL 17 proofs exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, returning earned Bonus points with a zero denominator; the Student row was `8 / 0`. The strict Student decoder accepts the complete pair, and compiled M6 component evidence rendered it without changing raw Assessment Attempt scoring.
- [x] **Quizzes** assess understanding of recent material.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states assessment of recent material in the Quiz description at both Instructor creation surfaces. The Instructor selects that purpose and content; PLE does not infer learning age.
- [x] Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
  - Evidence (source): `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` and `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults` select one Attempt for Quiz versus unset for Regular. `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` fixes Quiz to one Attempt and supplies editable Student instructions for Instructor-stated collaboration restrictions. This capability receipt does not claim a formal collaboration-policy field or automatic collaboration enforcement.
  - Evidence (runtime): the separately accepted neighboring one-Attempt receipt covered Quiz resume/submission and Attempt-2 denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [x] **Exams** are individual assessments associated with scheduled exam periods.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` states individual assessment associated with a scheduled exam period in the Exam description; `AssessmentWorkspaceCreatePage` and `BlueprintCourseCreateDialog` render it as an Instructor-selected purpose. This does not claim an Instructor exam calendar or infer scheduled periods.
- [x] Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` fixes Exam to one Attempt and exposes editable time limit, Available/Closes schedule, and six feedback-release controls including `never`. This is current UI capability evidence, not acceptance of every control's persistence/runtime enforcement.
  - Evidence (runtime): the separately accepted neighboring one-Attempt receipt covered Exam effective-one handling and expired-pending denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`; it does not expand this settings-capability receipt.
- [x] Quizzes and Exams allow one Assessment Attempt.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` resolves Quiz and Exam to an effective limit of `1` before issue or resume.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered Quiz resume/submission and Attempt-2 denial, Exam effective-one handling, and expired-pending Exam denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.

### Assessment type appearance

- [x] Each Assessment Type has its own PLE-defined Font Awesome icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` assigns one required Font Awesome icon name and bundled glyph to every canonical Type.
- [x] Assessment Type icons remain consistent across PLE themes.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` owns the theme-independent Type-to-icon mapping.
- [x] Each Assessment Type also has its own theme-defined color.
  - Evidence (source): `src/styles/assessment_types.css` `--ple-assessment-type-regular-assignment` defines the five semantic Type color properties at the root and Course-theme scope.
- [x] Themes may change Assessment Type colors but preserve the meaning of each Type.
  - Evidence (source): `src/styles/assessment_types.css` `course-theme-scope` overrides the five semantic Type properties by Type identity; an accepted nested-theme fixture verified the actual scoped cascade.
- [x] Assessment Type should never be communicated by color alone.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssessmentCard` always renders the Type label and its genuine bundled glyph; color is supplementary.
- [x] Icons and labels should remain sufficient to identify the Assessment Type without color.
  - Evidence (source): `src/assessment_type_presentation.ts` `assessmentTypePresentation` returns the canonical Type label and bundled icon metadata independently of color.
- [x] **Regular Assignment** uses the Font Awesome `pen-to-square` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Regular Assignment to `pen-to-square` and the genuine bundled glyph.
- [x] **Practice Question Assignment** uses the Font Awesome `arrows-spin` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Practice Question Assignment to `arrows-spin` and the genuine bundled glyph.
- [x] **Bonus Assignment** uses the Font Awesome `star` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Bonus Assignment to `star` and the genuine bundled glyph.
- [x] **Quiz** uses the Font Awesome `circle-question` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Quiz to `circle-question` and the genuine bundled glyph.
- [x] **Exam** uses the Font Awesome `file-signature` icon.
  - Evidence (source): `src/assessment_type_presentation.ts` `ASSESSMENT_TYPE_PRESENTATIONS` maps Exam to `file-signature` and the genuine bundled glyph.

### Blueprint Assessments

- [x] A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintCourseModuleContent` contains ordered `BlueprintAssessmentContent`; `schemas/base_schema/blueprints.sql` `blueprint_revision_assessment` records each stable Blueprint Assessment member of a Blueprint Course Revision.
- [x] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` validates reusable content and `BlueprintAssessmentDefaults` before a Blueprint Course Revision is constructed.
  - Owner: Same implementation finding as the earlier Assessments bullet.
- [x] Blueprint Assessments have an Assessment Type.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `assessment_type` requires the closed five-Type value; `src/api/decoders/blueprint_course.ts` `assessmentType` strictly requires it in both reusable-content input and view decoding without fallback.
  - Evidence (test): `tests/test_blueprint_course_client.mjs` `B1 client sends Revision and metadata validators to their separate routes` covers create/save/view Type round trips plus missing and unknown rejection; the focused Blueprint client lane passed 16/16.
- [ ] Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Mismatch: Ordered entries exist, but published Question and Pool Assessment behavior is not verified.
  - Owner: Same implementation finding as the earlier Courses bullet.
- [ ] Blueprint Assessments define Question point values and points possible.
  - Mismatch: Point values exist in Assignment source, but Blueprint Assessment behavior is not verified.
- [ ] Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
  - Mismatch: Blueprint Assignment source does not establish this full absence contract.
- [x] Blueprint Assessments do not use Assessment Templates.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` defines Templates as Instructor-owned private state outside Courses and Blueprints with no Blueprint or source field.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered Template create, save, read, and direct Course Assessment copy without a Blueprint relationship through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [ ] Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.
  - Mismatch: Copy behavior is outside this source-only verification and uses Assignment terminology.

### Course Instance Assessments

- [x] A **Course Instance Assessment** is an Assessment in a **Course Instance**.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.assessment` requires `course_id` referencing `ple_data.course_instance`; `crates/question_model/src/assessment.rs` `AssessmentOrigin` names direct creation in a Course Instance and adopted creation from an exact Blueprint Assessment.
- [ ] Course Instance Assessments are the Assessments delivered to **Students**.
  - Mismatch: Delivery source implements Assignments rather than the HG Assessment model.
- [x] Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment_type` adds the closed Type to the existing Assessment content, Pool, and points model; `schemas/base_schema/course_blueprint_adoption.sql` `assessment_type` preserves it during adoption.
  - Evidence (source): `crates/learning-data-access/tests/assessment_policies_postgres.rs` `policy_save_is_isolated_conflict_checked_and_reports_unreleased_invalid_dates` supplies the current adopted Quiz fixture: it changes instructions and a 300-second time limit to 600 seconds through a Type-free Properties input.
  - Decision: The prior Quiz Attempt-limit 3-to-5 runtime wording is retired. The current ignored PostgreSQL fixture still needs an actual acceptance rerun; this source evidence does not manufacture one.
- [ ] Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
  - Mismatch: Assignment delivery settings exist, but the complete HG Assessment behavior is not verified.
- [x] Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `initialize_course_assessments` creates a fresh Course Assessment with immutable adopted provenance; `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` updates that Course Assessment through its Course-owned reference.
  - Evidence (runtime): accepted C503 PostgreSQL 17 evidence covered adopted Assessment load/save with exact Blueprint Course, Revision, and Assessment provenance retained; `crates/learning-data-access/src/postgres/assessment_release.rs` `PostgresLiveAssessmentStore` exercised the adopted load/save production mapper. The standalone C503 proof did not rerun the full publisher-backed installation-data seed.
- [x] Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `ple_api.append_new_blueprint_assessments` materializes only newly added Assessments through `ple_data.append_course_assessments`; PostgreSQL Blueprint Store Save invokes the transaction boundary.
  - Evidence (test): connected `crates/learning-data-access/tests/blueprint_course_postgres/append.rs` `assert_new_assessment_save_preserves_daughter_work` passed exact settings/Revision-pin and distinct daughter Pool-ID checks, inactive daughter handling, unrelated empty-Course nonmutation, actual existing Student Work preservation, unchanged adoption pin, and replay/no-op/stale refusal without duplicate copies. Accepted temporary malformed-payload rollback is supplemental: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. Existing connected adoption lifecycle regression passed 1 test with 0 ignored: `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`.
  - Decision: This closes only automatic-new copying, not existing-Assessment update offers, full Student delivery, or the whole Assessment milestone.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` was extended to invoke the `append.rs` helper above; its final integrated connected receipt and malformed-daughter rollback both passed in `/private/tmp/ple-blueprint-append-proof-artifacts.LZU0K8`. This is extension of an existing permanent test, not a separate new test.

### Assessment Templates

- [x] An **Assessment Template** is a reusable set of settings for creating Course Instance Assessments.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` stores reusable settings, and `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` makes a direct Course Assessment from them.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed the complete Template round trip and by-value Course Assessment copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Assessment Templates are separate from Assessment Types.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` has a private UUID, owner, name, and settings while its required `assessment_type` is one closed Type field.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template create, save, read, and by-value copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Every Assessment Template has one of the five Assessment Types.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` constrains `assessment_type` to the five canonical values.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template create, save, read, and by-value copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] **Instructors** can create and change their own Assessment Templates.
  - Evidence (source): `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface` supplies Instructor CRUD; owner authorization is enforced by the Template Store.
  - Evidence (runtime): accepted C515 actual-Store proof covered owner/nonowner and inactive authorization, stale CAS, and settings round trip through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
  - Evidence (runtime): separate actual-component proof covered browser Template CRUD at `src/pages/assessment_templates_page.tsx` `AssessmentTemplatesSurface`; it is not connected-server evidence.
- [x] Assessment Templates provide defaults for settings such as Attempts, timing, scoring, and disclosure.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` stores instructions, Attempt/time limits, late-work, seven activity rules, and six feedback-release timings including grade rule.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template settings round trip and copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Creating a Course Instance Assessment from a Template copies its settings into the new Assessment.
  - Evidence (source): `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` reads the owner-visible Template once and passes every portable setting by value to `ple_data.create_assessment_from_template_values`.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence and actual-component UI proofs passed at `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] The new Course Instance Assessment can be changed independently after it is created.
  - Evidence (source): `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` creates a direct Course Assessment with copied values and no Template link.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence proof passed at `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.
- [x] Changing an Assessment Template does not change Assessments previously created from it.
  - Evidence (source): `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template` copies settings by value into the new Course Assessment and stores no Template identity.
  - Evidence (runtime): accepted C516 SQL full-settings/copy-independence proof passed at `schemas/base_schema/assessment_template_copy.sql` `ple_api.create_assessment_from_template`.
- [x] Assessment Templates do not contain Questions or Question Pools.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` defines identity, owner, Type, and reusable settings with no Question, Pool, content, point, or source field.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template creation and empty direct Course Assessment copy through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
- [x] Blueprint Assessments do not use Assessment Templates.
  - Evidence (source): `schemas/base_schema/assessment_templates.sql` `ple_private.assessment_template` defines Templates outside Courses and Blueprints; the by-value path creates only direct Course Assessments.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed Template copy to a direct Course Assessment without a Blueprint relationship through `crates/learning-data-access/src/postgres/assessment_template.rs` `PostgresAssessmentTemplateStore`.
  - Owner: Same implementation finding as the earlier Assessment Templates bullet.

### Course Instance Assessment release and defaults

- [x] Course Instance Assessments start unreleased.
  - Evidence (source): `schemas/base_schema/assessments.sql` `assessment_status` defaults to `unreleased`; `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` creates direct rows without overriding it, `schemas/base_schema/course_blueprint_adoption.sql` `ple_data.initialize_course_assessments` owns the same initial status for adopted rows, and `schemas/base_schema/assessment_template_copy.sql` `create_assessment_from_template_values` flows through direct `create_assessment` before copying policy values.
  - Evidence (runtime): `src/pages/assessment_workspace/assessment_workspace_create_page.tsx` `AssessmentWorkspaceCreatePage` was exercised in accepted private actual-main/HTTP proof: a newly Empty Course's direct Practice Assessment was Unreleased at creation, before its later validated release. Separate accepted `src/pages/course_list_page.tsx` `TeachingCourseListPage` Public Blueprint adoption produced an Unreleased Practice Assessment at creation. This initial-state fact does not establish Student delivery or all release rules.
- [x] Releasing a Course Instance Assessment requires an automated and interactive **Assessment Release Validation** process.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `ple_api.validate_assessment_release` projects the trusted release issues only to an authorized Course Instructor; `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` invokes it from the unreleased Assessment Properties workflow.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt proved the authorized validation/release boundary and rollback behavior at `schemas/base_schema/assessment_operations.sql` `ple_api.validate_assessment_release`; the accepted actual Properties component receipt exercised the interactive readiness flow at `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`.
- [ ] Assessment Release Validation checks the Assessment settings and data required for release.
  - Mismatch: The canonical `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` checks the current release conditions, but complete required-setting coverage remains unverified; see the separate valid-range and Question-validity rows.
- [ ] Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
  - Mismatch: The accepted evidence demonstrates five actionable date issues (missing Due, 24-hour and Course Active-limit boundaries, and two date-order violations), correction, and rerun through `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` and `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`. It does not establish missing, invalid, or unreasonable validation beyond dates; the required-setting valid-range and Question-validity rows remain open.
- [x] Release Validation should require a due date at least 24 hours in the future and no later than the
  Course Instance's six-month Active limit.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` rejects a missing Due date, a Due date less than 24 hours ahead at release, and a Due date after `course_instance.active_until_at`.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt exercised the missing-Due, 24-hour, and Course Active-limit boundaries from `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues`.
- [x] Release Validation should check that release, due, and other dates occur in a valid order.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` rejects Available after Due and Due after Closes.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt exercised both invalid orderings and the corrected valid ordering from `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues`.
- [x] Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Evidence (source): `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` reaches `schemas/base_schema/assessments.sql` `ple_data.replace_assessment_entries`, which rejects point values outside `0` through `1000000000.9999` or four decimal places; table checks retain that bound for alternate writers. The Assessment table permits only null or positive whole-Assessment Attempt/time limits and requires one Attempt for Quiz and Exam.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.assessment_release_issues` separately requires an Assessment Attempt time limit before release.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt for `schemas/base_schema/assessment_operations.sql` `ple_api.save_assessment` atomically rejected `1000000001` and `1000000000.99999`; exact `1000000000.9999` saved and released.
- [ ] Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
  - Mismatch: No verified Assessment Question validation was found.
- [x] The **Instructor** should be able to correct validation problems and run Release Validation again.
  - Evidence (source): `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage` keeps Assessment Properties editable, provides issue-specific save-and-rerun instructions, and exposes `Check release readiness` again after correction.
  - Evidence (runtime): accepted actual Properties component receipt exercised invalid dates, correction, a second readiness check, and release through `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` `AssessmentWorkspacePoliciesPage`.
- [x] An Assessment can be released only after Release Validation passes.
  - Evidence (source): `schemas/base_schema/assessment_release_validation.sql` `ple_data.validate_assessment_release` raises on every issue, and `schemas/base_schema/assessment_operations.sql` `ple_data.release_assessment` invokes that hard gate before changing release state.
  - Evidence (runtime): accepted fresh PostgreSQL 17 actual-API receipt proved unauthorized access is denied, invalid release rolls back, and corrected release succeeds through `schemas/base_schema/assessment_operations.sql` `ple_api.release_assessment`.
- [ ] Releasing an Assessment makes it available to **Students** according to its dates and access settings.
  - Mismatch: Assignment delivery source exists, but live access behavior needs runtime evidence.
- [ ] Student Work begins when a **Student** starts an Assessment Attempt.
  - Mismatch: Attempt issuance source exists, but live Student Work behavior needs runtime evidence.
- [x] New Course Instance Assessments default to accepting submissions only through the due date.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.create_assessment` applies the `reject` late-work default; `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` pins the immutable expiration to the effective Due date for that rule and `ple_private.save_student_assessment_attempt_response` rejects an expired save. Explicit `accept` and `mark_late` overrides remain valid and do not use Due as expiration.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-Student-API proofs exercised `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.save_student_assessment_attempt_response` and the ordinary finalization API, rejecting post-Due save and commit for the default rule while the expiry worker retained both accepted pre-Due saved responses; explicit `accept` and `mark_late` cases remained open through Due and used Closes.
- [x] New Course Instance Assessments default to starting new Attempts only through the due date.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.create_assessment` applies the `reject` late-work default; `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` rejects a post-Due start and uses the effective accommodated Due date when present. Explicit `accept` and `mark_late` overrides remain valid.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-Student-API proofs exercised `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt`, rejecting a post-Due start under `reject`, honoring an accommodated Due date, and keeping explicit `accept` and `mark_late` cases available through Closes.
- [x] Late work defaults to rejected.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.create_assessment`, `src/features/blueprint_course/blueprint_course_model.ts` `defaultDefaults`, and `crates/project-tools/src/curriculum_content/publication.rs` `blueprint_input` apply `reject` at direct and reusable-content default creation boundaries while preserving explicit Instructor overrides.
  - Evidence (runtime): accepted independent PostgreSQL 17 actual-API proof exercised `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` and `ple_private.save_student_assessment_attempt_response` through immutable Due expiry and post-Due start/save/commit denial, while confirming that explicit `accept` and `mark_late` remain valid alternatives.
- [x] Assessment disclosure settings remain separate and independently configurable.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` gives score, correctness, submitted response, correct answer, answer explanation, and class statistics independent timing fields; `src/pages/assessment_workspace/assessment_workspace_policies_page.tsx` exposes those six fields. Question Feedback is shown when provided and has no delayed-release state.
  - Evidence (test): accepted actual-component proof exercised `src/features/blueprint_course/blueprint_course_create_dialog.tsx` `BlueprintCourseCreateDialog`, switching Type both ways while preserving title, entries, and the resulting Type defaults.
- [x] **Regular Assignments** and **Bonus Assignments** should rarely show the correct answer.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule::for_assessment_type` defaults `question_answer` to `never` for Regular and Bonus while leaving the setting independently configurable.
- [x] Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
  - Evidence (source): `schemas/base_schema/assessments.sql` `create_assessment` defaults `submitted_response` and `per_item_correctness` to `after_submit`; `crates/server/src/assessment_delivery/history.rs` `project_history` projects those fields independently through the existing server-redacted summary.
  - Evidence (test): accepted actual-component proof exercised `src/pages/assessment_attempt_page.tsx` `submitAttempt`, navigating an accepted whole submission to that summary while preserving existing failure behavior on the Attempt page.
- [ ] **Practice Question Assignments** show correct answers immediately after Assessment Attempt
  submission.
  - Evidence (source): the Rust, SQL, Blueprint, and curriculum-publication creation boundaries default Practice `question_answer` to `after_submit` while Question Feedback and answer explanation remain independent.
  - Evidence (runtime): accepted PostgreSQL 17 ordinary start and whole-submit proof found zero response-source rows before submission and one after; the native PLE summary preserved the disclosed correct answer.
  - Mismatch: Backend-owned answers currently project as absent, and opaque WeBWorK answer disclosure remains unimplemented. This universal row stays open pending safe backend-owned disclosure without answer extraction.
- [ ] **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `ple_private.current_student_cohort_completed_assessment` derives the current active Student cohort from immutable submission evidence without a snapshot or latch. `crates/server/src/assessment_delivery/history.rs` `history_decision` calls `gate_quiz_exam_answers_for_current_cohort`, and `project_released_content` applies that gate.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed the two-current-Student cohort transition through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
  - Evidence (runtime): accepted independent PostgreSQL 17 installed-predicate proof with administrator-inserted synthetic fixtures verified never-started blocking, pending-invitation exclusion, joined-current-membership blocking, Account-deactivation membership preservation, Course-end noncompletion, ended-episode exit/new-episode rejoin, and retained submission behavior: `/private/tmp/ple-assessment-cohort-transition-artifacts.nWdHzT`.
  - Mismatch: Opaque WeBWorK answer display remains unimplemented.
  - Verification pending: Connected HTTP correct-answer display/release must verify that the current gate controls the projection.
- [x] A Quiz or Exam Attempt is complete when the **Student** submits it or its time limit expires and
  PLE submits it automatically.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` inserts one submitted-Attempt record for `student` or `deadline` finalization.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered Quiz Student submission, generic deadline finalization, and expired-pending Exam denial through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
  - Decision: The accepted composition uses the type-independent submission authority. Quiz/Exam worker finalization was not directly run.
- [x] Assessment Attempt completion does not depend on correctness or score.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` resolves finalization from Student-versus-deadline state; correctness and score are not completion conditions.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed zero/partial whole submission and deadline finalization through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [ ] Until then, Quizzes and Exams do not disclose correct answers.
  - Evidence (source): `crates/server/src/assessment_delivery/history.rs` `history_decision` calls `gate_quiz_exam_answers_for_current_cohort`, and `project_released_content` applies the resulting decision before projecting released content.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed the two-current-Student cohort transition through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
  - Evidence (runtime): accepted independent PostgreSQL 17 installed-predicate proof with administrator-inserted synthetic fixtures verified never-started blocking, pending-invitation exclusion, joined-current-membership blocking, Account-deactivation membership preservation, Course-end noncompletion, ended-episode exit/new-episode rejoin, and retained submission behavior: `/private/tmp/ple-assessment-cohort-transition-artifacts.nWdHzT`.
  - Mismatch: Opaque WeBWorK answer display remains unimplemented.
  - Verification pending: Connected HTTP answer-withholding must verify the current gate; the SQL proof is not whole-submit-pipeline acceptance.
- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_provided_feedback_is_shown` verifies provided native feedback without answer disclosure.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/question_model/src/assessment_activity_rules.rs` `StudentFeedbackReleaseRule` has no Question Feedback timing field; `crates/domain/src/student_feedback_release.rs` `project_student_feedback` projects supplied feedback independently while `StudentFeedbackReleaseDecision` continues to gate correct answer and explanation.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Unreleasing a Course Instance Assessment permanently deletes its Student Work and returns to a pre-release state.
  - Evidence (test): `tests/e2e/e2e_unrelease_connected.sh` `psql_admin` runs the connected unrelease deletion and pre-release-state oracle.

### Assessment Attempts

- [x] An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `ple_private.assessment_attempt` requires both a `student_record_id` and an `assessment_id`; that Assessment ID references the Course-owned `ple_data.assessment` aggregate.
- [x] Blueprint Assessments do not have Assessment Attempts.
  - Evidence (source): `crates/question_model/src/blueprint_operations.rs` `BlueprintAssessmentContent` contains only reusable content and defaults; `schemas/base_schema/assessment_attempts.sql` `ple_private.assessment_attempt` permits Attempts only through `ple_data.assessment`, the Course Instance Assessment aggregate.
- [ ] Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
  - Mismatch: Saved-response persistence is tested after a browser reload, but no evidence establishes persistence across a distinct browser session.
- [x] **Instructors** control the number of permitted Assessment Attempts.
  - Evidence (source): `schemas/base_schema/assessments.sql` `ple_data.save_assessment` and `ple_data.save_assessment_policies` accept `assessment_attempt_limit` only after `current_session_account_is_course_instructor`; issuance applies that saved value subject to Quiz/Exam effective-one.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt covered unlimited retries, Quiz Attempt-2 denial, and expired-unlimited new-Attempt behavior through `crates/learning-data-access/src/postgres/assessment_attempt.rs` `PostgresAssessmentAttemptStore`.
- [x] Regular Assignments default to unlimited Attempts.
  - Evidence (source): `schemas/base_schema/assessment_creation.sql` `ple_data.create_assessment` defaults to a nullable Attempt limit for unlimited Attempts; only Quiz and Exam override it to one at issuance.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed unlimited retries after perfect and nonperfect submissions through `crates/learning-data-access/src/postgres/assessment_attempt.rs` `PostgresAssessmentAttemptStore`.
- [x] **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
  - Evidence (source): `schemas/base_schema/assessment_attempt_operations.sql` `ple_private.start_assessment_attempt` counts issued Attempts only with an effective finite limit; `NULL` permits another.
  - Evidence (runtime): accepted independent fresh PostgreSQL 17 actual-Store receipt passed unlimited perfect/nonperfect retries and expired-unlimited new Attempt through `crates/learning-data-access/src/postgres/assessment_attempt.rs` `PostgresAssessmentAttemptStore`. A separate isolated actual-server native Practice proof published the checked-in PKU Question through Draft authoring, enrolled the Student through roster import/claim, saved and submitted its correct response for a disclosed 1/1 score, then issued a distinct second Assessment Attempt despite that perfect score. This proves this one unlimited Practice transport case, not the Regular Assignment default or a complete Student browser journey.
- [x] When an Assessment permits multiple Attempts, the highest Assessment Attempt score is used as the
  Student's Assessment score.
  - Evidence (source): `schemas/base_schema/grading_access.sql` `read_assessment_gradebook_evidence` independently selects the highest grading-complete submitted Attempt by earned points, then uses the latest Attempt only when no score is established; `ple_api.read_course_gradebook` consumes that private answer-free helper.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/grading_access.sql` `ple_api.read_course_gradebook`, proving an earlier higher earned score beats a later lower score and a later unfinished or pending Attempt does not replace it. Current Question points recalculated the selected score from `8` to `16`; a Bonus contribution retained a zero possible denominator; and a latest unscored expired Attempt remained the fallback when no completed score existed.
  - Evidence (source): `schemas/base_schema/student_assessment_landing.sql` `ple_private.read_student_released_assessment_landing_evidence` keeps progress, completion, and resume state on the latest Attempt but obtains the Assessment score from the selected highest Attempt and applies that Attempt's copied disclosure timing. `src/api/decoders/live_student_course_landing.ts` `decodeAssessmentSummary` requires direct `assessmentScore` and rejects the retired `score` alias; `src/pages/student_course_landing_page.tsx` labels it `Assessment score`.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, preserving an earlier higher score across later lower, unfinished, and pending Attempts, including inverse selected-Attempt/latest-Attempt disclosure cases. The focused decoder/presentation lane passed 8/8, and compiled M6 component evidence passed.
- [ ] Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.
  - Mismatch: `schemas/base_schema/assessment_attempt_finalization.sql` has a current automatic finalization path, but no current runtime receipt establishes the complete submission-and-grading row after the retired oracle was removed.
- [ ] Automatic grading does not require a separate Student or **Instructor** grading workflow.
  - Mismatch: The current finalization source records direct grading, but no current runtime receipt establishes the complete no-workflow behavior after the retired oracle was removed.

### Assessment responses and submission

- [x] The Student submission action submits the whole Assessment Attempt.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` creates one Assessment submission and finalizes every open issued Question in that Attempt.
  - Evidence (runtime): accepted C525 actual-Store evidence exercised whole zero- and partial-response submissions through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [ ] A Question either has a complete saved response or has no saved response.
  - Mismatch: Complete-response contract was not verified.
- [x] PLE saves complete Question responses as the **Student** works.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` saves an ordinary Student response and asserts its persisted `saved` state.
- [ ] The Student may change a saved response while the Assessment Attempt remains open.
  - Mismatch: Current response persistence source exists, but no current runtime receipt establishes saved-response replacement after the retired oracle was removed.
- [x] Submitting the Assessment Attempt finalizes all saved Question responses together as Student Work.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` verifies every saved response before inserting the Attempt submission and Question submissions.
  - Evidence (runtime): accepted C525 actual-Store evidence exercised whole partial-response submission and history through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [ ] Questions without a saved response remain visibly unanswered when the Attempt is submitted.
  - Mismatch: Visible unanswered-state behavior needs runtime evidence.
- [x] An unanswered Question receives zero credit and counts as incorrect without being sent to the
  Question Backend.
  - Evidence (source): `schemas/base_schema/assessment_attempt_finalization.sql` `ple_private.commit_assessment_attempt_finalization` marks unanswered Questions `closed_unanswered` and scores missing credit as zero.
  - Evidence (runtime): accepted C525 actual-Store deadline evidence observed `closed_unanswered` with no invented response through `crates/learning-data-access/src/postgres/assessment_delivery.rs` `LiveAssessmentDeliveryStore`.
- [ ] PLE treats an incomplete Question response as unsaved, although the Question interface may keep the Student's unfinished input while they work.
  - Mismatch: Incomplete response behavior was not verified.
- [ ] A Question Backend may evaluate a response before Assessment submission when needed for its interaction.
  - Mismatch: No verified pre-submission backend evaluation behavior was found.
- [ ] When PLE requests a grading outcome, the Question Backend returns it without a deferred grading
  state.
  - Mismatch: No test/runtime proof of immediate backend grading outcome was recorded.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [ ] The **Student** does not see the grading outcome until the Assessment Attempt is submitted.
  - Mismatch: Student grading-outcome timing needs runtime evidence.

### Assessment Attempt timing and expiration

- [ ] Each Assessment Attempt has a time limit.
  - Mismatch: Assignment time limit is optional rather than required for each attempt.
- [ ] Attempt time limits help **Students** develop an accurate sense of expected working speed.
  - Mismatch: This pedagogical effect is not implemented as verifiable product behavior.
- [x] Timed Assessment Attempts use wall-clock time.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` observes database `clock_timestamp()` reach the persisted expiry before rejecting late work.
- [x] The server owns the Attempt start and expiration times.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` reads the persisted server `expires_at` against database `clock_timestamp()`.
- [x] Attempt time continues while the **Student** is disconnected or the browser is closed.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_background_expiry` proves the generic worker finalizes an expired Attempt without a further Student interaction.
- [ ] A **Student** may reconnect, reload, or use another browser session to resume the same active Attempt.
  - Mismatch: `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_start` repeats start with the same cookie; it does not prove reconnect, reload, or a distinct browser session.
- [ ] Resuming an Attempt does not reset, pause, or extend its time limit.
  - Mismatch: Resume source returns the active Attempt, but no test or runtime observation verifies that its time limit remains unchanged.
- [x] Attempt expiration is checked whenever a **Student** interacts with the Attempt.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` proves a save rechecks the server clock after lock waiting and returns `expired`.
- [x] Background processing ensures expired Attempts are submitted even when the **Student** is no longer connected.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_background_expiry` waits only for generic-worker immutable evidence, then proves the expired Attempt is submitted.
- [x] When an Attempt expires, PLE submits the whole Attempt, finalizing its saved responses. Other
  Questions remain visibly unanswered, receive zero credit, and count as incorrect without being
  sent to the Question Backend.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_expired_student_assignment_attempt_finalization` checks deadline finalization of saved work and no invented result for unanswered Questions.

### Student Work

- [x] Student Work keeps the exact Published Question Revision delivered to the **Student**.
  - Evidence (source): `schemas/base_schema/assessment_attempts.sql` `issued_question_is_immutable` stores issued Question revision identity under foreign-key protection.
- [ ] For a Question Pool, Student Work keeps the exact Question Pool Revision and Published Question Revision selected.
  - Mismatch: Pool selection retention is not verified.
- [x] Student Work keeps each saved response as finalized with the submitted Attempt and the grading outcome returned by the Question Backend.
  - Evidence (test): `tests/e2e/e2e_live_demo_assignment_attempt.sh` `prove_webwork_submission` submits a live WeBWorK-backed response and checks its renderer-derived stored credit against the submitted score.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` proves the saved response, submitted Attempt, stored credit, and automated receipt commit together without later replacement.
- [ ] Changes to Assessment content do not replace Question evidence already delivered in existing Attempts.
  - Mismatch: Historical evidence isolation is not verified.
- [ ] PLE should retain only the additional historical Student Work data needed to interpret or grade that work correctly.
  - Mismatch: No minimal-retention contract was verified.

### Assessment scoring

- [ ] Blueprint Assessments and Course Instance Assessments assign point values to Questions.
  - Mismatch: Assignment point values exist, but the HG Assessment model is not implemented.
- [ ] A Question Backend returns an immutable credit fraction for each complete response it evaluates.
  - Mismatch: The live WeBWorK submission check observes stored credit, but no direct adapter test proves a Question Backend returns that fraction as an immutable outcome.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] PLE stores the credit fraction as the Question grading outcome.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` reads the immutable stored normalized credit after finalization.
- [x] Course Instance Assessment scores are calculated from stored credit fractions and current Question point values.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` changes current entry points and proves the same stored 0.67 credit rescales the score.
- [x] An unanswered Question contributes zero points to the Assessment score and counts as incorrect.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_expired_student_assignment_attempt_finalization` asserts an all-unanswered expired Attempt remains in the denominator at zero credit.
- [x] When an Assessment has multiple submitted Attempts, the highest Assessment Attempt score is the
  Student's Assessment score.
  - Evidence (source): `schemas/base_schema/grading_access.sql` `read_assessment_gradebook_evidence` independently selects the highest grading-complete submitted Attempt by earned points, then uses the latest Attempt only when no score is established; `ple_api.read_course_gradebook` consumes that private answer-free helper.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/grading_access.sql` `ple_api.read_course_gradebook`, proving an earlier higher earned score beats a later lower score and a later unfinished or pending Attempt does not replace it. Current Question points recalculated the selected score from `8` to `16`; a Bonus contribution retained a zero possible denominator; and a latest unscored expired Attempt remained the fallback when no completed score existed.
  - Evidence (source): `schemas/base_schema/student_assessment_landing.sql` `ple_private.read_student_released_assessment_landing_evidence` keeps progress, completion, and resume state on the latest Attempt but obtains the Assessment score from the selected highest Attempt and applies that Attempt's copied disclosure timing. `src/api/decoders/live_student_course_landing.ts` `decodeAssessmentSummary` requires direct `assessmentScore` and rejects the retired `score` alias; `src/pages/student_course_landing_page.tsx` labels it `Assessment score`.
  - Evidence (runtime): accepted actual PostgreSQL 17 evidence exercised `schemas/base_schema/student_assessment_landing.sql` `ple_api.list_released_live_student_assessments`, preserving an earlier higher score across later lower, unfinished, and pending Attempts, including inverse selected-Attempt/latest-Attempt disclosure cases. The focused decoder/presentation lane passed 8/8, and compiled M6 component evidence passed.
  - Owner: Same implementation finding as the earlier Assessment Attempts bullet.
- [x] PLE uses Question point values directly to calculate Assessment scores.
  - Evidence (test): `crates/question_model/src/student_work/grading.rs` `current_points_recalculate_without_changing_recorded_credit` tests current point values rescale recorded credit directly.
- [ ] PLE does not use separate Question weights, Grade Categories, weighted categories, Course Grade
  Schemes, or Course percentage calculations.
  - Mismatch: Absence of every prohibited model was not verified.
- [ ] For the pilot, grade export uses CSV or TSV only and exports point-based Assessment scores.
  - Mismatch: No grade export implementation matching this contract was found.
- [ ] The Instructor handles Course-level weighting or percentage calculations in the home LMS.
  - Mismatch: No product boundary or export guidance establishing this behavior was verified.
- [x] Changing Question point values recalculates affected Assessment scores.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` changes entry points from two to three and proves score recalculation.
- [x] Score recalculation does not require another Question Backend interaction.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` replays finalization with an empty result array after changing current points and proves the stored credit rescores without another supplied backend outcome.
- [x] Score recalculation does not change the stored Question grading outcome.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` proves rescoring retains stored normalized credit and the original receipt.
