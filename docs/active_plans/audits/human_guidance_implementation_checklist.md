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
  - Mismatch: `schemas/base_schema/course_core.sql` `ple_data.course_instance` establishes teaching-course storage, but current direct evidence does not establish all listed lifecycle, membership, deadline, release, and empty-start behaviors.
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
  - Evidence (source): `src/features/blueprint_course/blueprint_assignment_content_editor.tsx` `moveEntry` and `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` `move` back their labelled native-button Move earlier and Move later controls.
  - Evidence (source): `src/features/ple_question_json_authoring/question_json_choice_list.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multiple_answer_editor.tsx` `onMoveChoice`, `src/features/ple_question_json_authoring/question_json_multi_fill_in_editor.tsx` `onMoveBlank`, `src/features/ple_question_json_authoring/question_json_matching_editor.tsx` `onMoveItem`, and `src/features/ple_question_json_authoring/question_json_ordering_editor.tsx` `onMoveItem` give every native JSON reorderer the same precise buttons.
  - Evidence (test): `tests/test_blueprint_course_model.mjs` `reusable entries preserve fixed and Question Pool interleaving`, `tests/test_ple_question_json_editor_model.mjs` `choice edits retain semantic IDs and enforce choices and correct-answer invariants`, `tests/test_ple_question_json_multiple_answer_editor.mjs` `multiple-answer text edits and reordering retain choice IDs and exact correct IDs`, and `tests/test_ple_question_json_multi_fill_ordering_authoring.mjs` `ORDER treats Ordering Items as the source of truth and derives correctOrder after movement` protect the stable reorder results.
  - Decision: The one-time seven-surface keyboard-control inventory passed and was removed rather than becoming a permanent implementation-inventory test. It does not select drag-and-drop surfaces, which remains the separate Human Guidance product question.
- [x] Themes should use biome and habitat names, such as Forest, Grassland, Ocean, and Desert.
  - Evidence (source): `src/features/course_appearance/course_theme_registry.ts` `COURSE_THEME_REGISTRY` retains stored ID `grass` and its unchanged anchors while presenting `Grassland`; the same closed registry presents Forest, Ocean, Desert, and the remaining habitat names.
  - Evidence (source): `src/pages/course_appearance_page.tsx` `COURSE_THEME_OPTIONS` renders each visible theme label from `option.tokens.name`, not its stored ID.
  - Evidence (test): `tests/test_course_theme_scope.mjs` `Grassland uses the Roosevelt-inspired anchors and accessible derived actions` verifies the `grass` ID presents Grassland without changing its reviewed palette; `every reviewed theme resolves to complete, contrast-safe course tokens` covers the closed registry.
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
  - Evidence (source): `src/pages/instructor_profile_page.tsx` `InstructorProfilePage` renders Profile image and time-zone controls.
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
  - Evidence (source): `src/features/instructor_profile/ribbon_profile_avatar.tsx` `RibbonProfileAvatar` falls back to `RibbonIcon` `circle-user` when no thumbnail URL exists.
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
  - Mismatch: `src/pages/course_list_page.tsx` has a dense Course Instance row, but no Assessment-named list exists; `src/pages/assignments_due_soon_page.tsx` still presents Assignments.
- [ ] Instructor **Student View** is an answer-free preview and does not create Student Work, Assessment Attempts, submissions, or grades.
  - Mismatch: `src/pages/assignment_preview_page.tsx` implements an Assignment preview, and its display text alone does not verify the required Assessment projection and no-write behavior.

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
- [ ] A Public Blueprint Course with no adoptions may return to **Private**.
  - Mismatch: no Public-to-Private transition is implemented.
- [ ] A Public Blueprint Course with one or more adoptions remains **Public**.
  - Mismatch: no Public availability model is implemented.
- [x] Archived Blueprint Courses leave normal discovery but remain available where needed for history.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `archive_blueprint` and discovery query comments retain historical pins while excluding archived discovery.
- [ ] Instructors may fork a Public Blueprint Course to continue development privately.
  - Mismatch: no Public Blueprint Course fork action or Private fork lifecycle is implemented.
- [ ] Blueprint Courses do not have a separate Draft state.
  - Mismatch: no source-level lifecycle test establishes the absence of a Draft state across Blueprint Course APIs and UI.

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
- [ ] Search should begin with a prominent search box, similar to Google Search.
  - Mismatch: `src/pages/library_page.tsx` places `Search published questions` before filters, but no rendered UX evidence establishes the required prominence or Google-like presentation.
- [ ] The initial Search page should stay simple and focus attention on entering a search.
  - Mismatch: `src/pages/library_page.tsx` presents seven filter controls alongside the initial search field.
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
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` supplies author, backend, tag, Question Type, license, Course-use, and capability filters.
- [x] Filters should update the current search rather than start a separate workflow.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` resets one `QuestionLibraryBrowseSession` with the updated query.
- [ ] Search should support Google-like syntax for more precise queries.
  - Mismatch: `src/pages/library_page_model.ts` exposes plain `search` text and no parsed advanced query syntax.
- [ ] Quoted text should search for an exact phrase.
  - Mismatch: no exact-phrase query parser or test exists.
- [ ] A minus sign should exclude matching terms.
  - Mismatch: no exclusion query parser or test exists.
- [ ] Search should support PubMed-like field tags such as `topic:genetics`.
  - Mismatch: no field-tag query parser or `topic` filter exists.
- [ ] Field tags should use PLE concepts and vocabulary.
  - Mismatch: field tags are not implemented.
- [ ] Useful fields may include subject, topic, tags, Question Type, and author.
  - Mismatch: the UI has tags, Question Type, and author filters but lacks subject and topic fields and field-tag search.
- [ ] Simple and advanced searches should use the same search box.
  - Mismatch: advanced search syntax is not implemented.
- [x] Instructors should not need to learn search syntax to use Search Question Library.
  - Evidence (source): `src/pages/library_page.tsx` `LibraryPage` exposes ordinary search and labeled filter controls without syntax requirements.
- [ ] The interface should make useful search syntax discoverable when needed.
  - Mismatch: no search syntax exists or is documented in the interface.
- [ ] Search syntax should help expert users quickly narrow a very large Question Library.
  - Mismatch: no advanced search syntax exists.
- [x] Search terms and active filters should remain visible while reviewing results.
  - Evidence (source): `src/pages/library_page.tsx` `query` signal remains bound to the search input and filter selects while rows render.
- [x] Clearing or changing part of a search should be quick.
  - Evidence (source): `src/pages/library_page.tsx` `changeQuery` updates the search session on each input or selection change.
- [ ] Opening a result and returning should preserve the Instructor's search and position.
  - Mismatch: `src/pages/library_page.tsx` keeps query and scroll state only in the mounted component; no route-return persistence exists.

##### Browse Question Library

- [ ] **Browse Question Library** helps Instructors explore Questions without knowing what to search for.
  - Mismatch: `browseQuestionLibrary` and Search share the same `library` route with no distinct browse workflow.
- [ ] Browse should help Instructors understand what the Question Library contains.
  - Mismatch: no browse landing surface explains the library's content.
- [ ] Browse should emphasize subjects, topics, tags, Question Types, and other useful groupings.
  - Mismatch: `src/pages/library_page.tsx` has some filters but no subject/topic browse hierarchy.
- [ ] Browse should make moving from broad subjects to narrower topics easy.
  - Mismatch: no subject-to-topic browse hierarchy exists.
- [ ] Browse should show useful counts where they help Instructors choose where to explore.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` routes Browse and Search to the same undifferentiated Library page; count-bearing Search filters do not establish a Browse workflow.
- [ ] Browse results should use the same dense Question presentation used by Search where practical.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` maps Browse and Search to one undifferentiated route, so no Browse result presentation exists to compare with Search.
- [ ] Instructors should be able to move from browsing into a more focused search.
  - Mismatch: no separate Browse state or transition to focused Search exists.
- [ ] Search and Browse are different paths into the same **Question Library**.
  - Mismatch: both Ribbon controls route directly to the same undifferentiated `library` route.

#### Assessments

- [ ] The **Assessments** ribbon must include: Assessments Due Soon, My Assessment Templates.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` uses "Assignments Due Soon" and "My Assignment Templates"; templates are a `future` destination.
- [ ] **Assessments Due Soon** should emphasize Assessments that may need the Instructor's attention.
  - Mismatch: `src/pages/assignments_due_soon_page.tsx` implements the retired Assignment term rather than Assessments.
- [ ] Assessment lists should make Course, release status, due date, and other important state easy to scan.
  - Mismatch: `src/pages/assignments_due_soon_page.tsx` implements an Assignment list rather than an Assessment list.
- [ ] **My Assessment Templates** should emphasize reusable Assessment design rather than Course activity.
  - Mismatch: `assignmentTemplates` is a `future` Ribbon destination.
- [ ] Assessment editing has two editors:
  - Mismatch: current editors are Assignment-named.
  - [ ] **Assessment Question Editor**: Selects, adds, removes, and orders Questions.
    - Mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` implements an Assignment Questions editor.
  - [ ] **Assessment Properties Editor**: Controls dates, scoring, attempts, late work, and other Assessment settings.
    - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` implements Assignment Policies.
- [ ] The two Assessment editors should remain clearly distinct.
  - Mismatch: two Assignment editors exist, but the required Assessment terminology and editors do not.
- [ ] The Assessment Question Editor should make Question order easy to understand at a glance.
  - Mismatch: Question ordering is implemented for Assignment, not Assessment, editor terminology.
- [ ] Adding Questions should provide direct paths to Search and Browse Question Library.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` provides available Questions but no direct Search and Browse Question Library paths.
- [ ] Instructors should be able to inspect a Question before adding it to an Assessment.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_questions_page.tsx` provides no Question inspection action before Add Question.
- [ ] Assessment Properties should group related settings so important settings are easy to find.
  - Mismatch: the implemented UI is Assignment Policies, not the required Assessment Properties surface.
- [ ] Instructors can randomize Question order for an Assessment.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` applies randomization to an Assignment, not an Assessment.
- [ ] Answer-choice randomization belongs to the Question, not the Assessment.
  - Mismatch: the implemented explanatory text uses Question and Assignment, not the required Assessment terminology.
- [ ] **Assessments Due Soon** shows upcoming Assessments across the Courses an **Instructor** teaches.
  - Mismatch: the route implements upcoming Assignments, not Assessments.
- [ ] Assessments Due Soon shows the Course and due time for each Assessment.
  - Mismatch: `src/pages/assignments_due_soon_page.tsx` displays Course and due time for Assignments, not Assessments.

#### High-consequence actions

- [ ] Danger Zone contains **Assessment Unrelease**, **Archive Published Question**, and **Archive Blueprint Course**.
  - Mismatch: the implemented UI names the first action "Unrelease assignment," not Assessment Unrelease.
- [x] Danger Zone should be visually separate from ordinary editing actions.
  - Evidence (source): `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` `assignment-workspace-unrelease-danger-zone` is a separate danger section.
- [ ] Assessment Unrelease should explain that Student work will be deleted.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` explains Assignment unrelease, not Assessment Unrelease.
- [ ] Assessment Unrelease should require typing the Assessment title before confirmation.
  - Mismatch: `src/pages/assignment_workspace/assignment_workspace_policies_page.tsx` requires an Assignment title, not an Assessment title.
- [ ] Archive actions should explain the effect on shared availability and require a clear confirmation.
  - Mismatch: `src/features/blueprint_course/blueprint_course_workspace.tsx` verifies that behavior only for Archive Blueprint Course; it does not establish the required behavior for every Archive action, including Archive Published Question.
- [x] Restore actions should use ordinary availability controls.
  - Evidence (source): `src/features/blueprint_course/blueprint_course_workspace.tsx` `restore` is presented under Course names and availability rather than the archive confirmation control.
### Student interface

- [x] The Student interface should focus on current Courses, Coursework, and work that needs attention.
  - Evidence (source): `src/pages/student_courses_page.tsx` `StudentCoursesPage` lists current courses; `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` lists assigned work.
- [ ] **Coursework** is the Student-facing collective term for Regular Assignments, Practice Question Assignments, Bonus Assignments, Quizzes, and Exams.
  - Mismatch: `src/pages/student_course_landing_page.tsx` `StudentCourseLandingPage` calls the collection "Assignments" and has no Coursework terminology or listed types.
- [ ] Student-facing interfaces should use the specific Assessment Type when referring to an individual item rather than calling it an Assessment.
  - Mismatch: `src/pages/student_course_landing_page.tsx` `AssignmentCard` has no Assessment Type label.
- [ ] The Student Ribbon should use familiar Student language rather than internal PLE terms such as Assessment.
  - Mismatch: `src/ribbon/ribbon_catalog.ts` `studentAssignments` provides only an "Assignments" control; no complete Student Ribbon is implemented.
- N/A Coursework lists may provide filters for **Regular Assignments**, **Practice Question Assignments**, **Bonus Assignments**, **Quizzes**, and **Exams**.
  - Reason: Optional permission does not require current product behavior.
- [ ] Each Coursework item should clearly show its Assessment Type using its label and Type icon.
  - Mismatch: `src/pages/student_course_landing_page.tsx` `AssignmentCard` has no type label or icon.
- [x] The Student interface should make the next useful action easy to find.
  - Evidence (source): `src/pages/student_course_landing_page.tsx` `AssignmentCard` presents the primary "Open Assignment" action.
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
- [ ] Course pages should make upcoming, available, completed, and missed Coursework easy to distinguish.
  - Mismatch: `src/pages/student_course_landing_page.tsx` `progressLabel` covers completed, in-progress, and not-started only; it has no upcoming or missed state.
- [ ] Coursework lists should make due dates, Type, and completion status easy to scan.
  - Mismatch: `src/pages/student_course_landing_page.tsx` `AssignmentCard` lacks visible due date and Assessment Type fields.
- [ ] Before starting Coursework, Students should see its title, Type, Question count, points possible, time limit, and previous Attempts.
  - Mismatch: `src/components/student_assignment_presentation.tsx` `StudentAssignmentStartFacts` has some delivery facts, but does not establish the complete required Type and previous-Attempts presentation.
- [x] Students see one Question at a time while completing Coursework.
  - Evidence (source): `src/pages/assignment_attempt_page.tsx` `AssignmentAttemptPage` renders one keyed `currentPresentation` question card.
  - Evidence (test): `tests/playwright/e2e_live_demo_course_seed_browser.mjs` `expectAttempt` asserts one `article.question-card`.
- [x] While completing Coursework, navigation should show every Question, its saved status, and allow Students to jump directly between Questions.
  - Evidence (source): `src/components/student_assignment_attempt_navigation.tsx` `StudentAssignmentAttemptNavigation` renders every position, saved-status label, and position button.
  - Evidence (test): `tests/test_student_assignment_attempt_navigation.mjs` `Student Question navigation renders ordered, answer-free states with one current Question`.
- [x] Leaving a Question and returning should preserve its saved response.
  - Evidence (source): `src/pages/assignment_attempt_page.tsx` `activatePosition` saves before loading another position and `loadPresentation` restores `savedResponse`.
  - Evidence (test): `tests/playwright/e2e_live_demo_webwork_submission_browser.mjs` reload assertion verifies persisted `savedResponse`.
- [x] The current Question and overall progress should remain easy to see.
  - Evidence (source): `src/pages/assignment_attempt_page.tsx` `AssignmentAttemptPage` renders the current position and total question count beside the navigation.
- [x] The timer should be subtle and keep the focus on the Questions.
  - Evidence (source): `src/pages/assignment_attempt_page.tsx` `assignment-attempt-header` keeps `calm-status` timer in the header outside the question card.
- [x] For timed Coursework, the remaining time should stay visible while moving between Questions.
  - Evidence (source): `src/pages/assignment_attempt_page.tsx` `remainingMilliseconds` is header state independent of `currentPresentation`.
- [x] Submission status should be obvious and use plain language.
  - Evidence (source): `src/pages/assignment_attempt_page.tsx` `submissionState` renders "Your answers were accepted" and saved-response submission text.
- [x] Scores and feedback should appear where the Coursework settings allow them.
  - Evidence (source): `src/components/student_assignment_presentation.tsx` `StudentAssignmentPresentation` conditions scores and feedback on `studentFeedbackReleaseRule`.
- [x] Completed Coursework should remain easy to find and review.
  - Evidence (source): `src/pages/assignment_attempt_summary_page.tsx` `AssignmentAttemptSummaryPage` presents previous attempt score and recorded work.
- [x] Student content entry should use the response controls provided by Questions and other Student activities.
  - Evidence (source): `src/pages/assignment_attempt_page.tsx` `QuestionPresentationResponseControl` receives the Question presentation response format.
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
- [x] Rare installation and configuration tasks should remain available through secondary navigation.
  - Evidence (source): `src/ribbon/ribbon_catalog.ts` `supportRoster` is the supporting `Scoped Support` administrative task.
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
- [ ] Creating or extending a later Assessment deadline may move those dates, but not beyond the six-month Active lifetime.
  - Mismatch: No Active lifetime cap or retention-date recalculation exists.
- [ ] Starting the FERPA retention clock does not itself notify, archive, hide, or delete Student data.
  - Mismatch: The FERPA retention-clock transition is absent.
- [ ] The configured FERPA retention policy determines the later notice, archive, recovery, and permanent deletion transitions.
  - Mismatch: No configured FERPA retention policy or its transitions exists.
- [ ] PLE warns the **Instructors** before the Course Instance becomes Inactive six months after creation.
  - Mismatch: No six-month inactivity transition or Instructor warning exists.
- [ ] The six-month Active limit prevents Course reuse or deadline extensions from indefinitely delaying FERPA retention and deletion.
  - Mismatch: No six-month Active limit exists.
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
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `ple_private.publish_question_revision` locks the Draft and immediate parent before it creates a successor; its `PQR01` source-checksum comparison rejects an unchanged `question_revision_source_binding` before any successor facts are written.
  - Decision: A fresh one-time PostgreSQL 17 probe verified title and description metadata changes make no Revision, an unchanged source is rejected without a partial write, a changed source creates the next Revision, and two serialized sessions admit only one successor. Tags, subject, and topic have no persisted metadata fields yet; the probe asserts that present absence rather than inventing a field-level behavior. The probe is temporary and will be removed, not cited as permanent evidence.
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
- [ ] Instructors may delete Draft Questions they no longer need.
  - Mismatch: draft creation and save operations exist, but no owned draft deletion operation was found.
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
- [ ] Native JSON Questions are static, not algorithmic nor random, and receive no random seed.
  - Mismatch: `crates/server/src/assignment_delivery.rs` `issue_new_presentations` requires and persists a `QuestionSeed` for every native PLE issue, and `crates/adapters/ple/src/lib/question_json_source.rs` `PleQuestionBackend::issue_question_json` accepts it. The source is static, but the delivered native Question still receives a seed. `randomizeChoices` is separately compiled to server-only `NativeChoiceOrder::NonceRandomized` in `crates/adapters/ple/src/question_json/source_document.rs` `compile_choices`; `crates/question_model/src/presentation/choice_order.rs` `nonce_randomized_choices` uses the durable presentation nonce only to permute stable authored choice IDs. That presentation behavior is not algorithmic Question generation, but it does not cure the seed mismatch.
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
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] The initial primary Question Backends are PLE-native JSON and WeBWorK.
  - Evidence (source): `schemas/base_schema/assessment_attempt_presentation.sql` `backend IN ('ple', 'webwork')` is the delivered presentation boundary.
- [ ] iMathAS and H5P are supported secondary Question Backends.
  - Mismatch: iMathAS has a launch boundary, while C870 leaves no current H5P source, import, or delivered backend seam.
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
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
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
- [x] PLE-native Questions use the PLE Question Backend.
  - Evidence (source): `schemas/base_schema/question_authoring_state.sql` `question_source_binding_fields_are_valid` maps `ple` to `pleQuestionJson`.
- [ ] WeBWorK owns PG/PGML rendering, controls, answer evaluators, partial credit, and feedback.
  - Mismatch: the isolated opaque adapter proves renderer documents, ordered pairs, score, partial credit, and stateless state. Connected live-ownership proof remains required; PLE is not required to capture historic renderer feedback.
- [ ] Question Backend feedback is transient unless the backend provides a robust way for PLE to preserve it.
  - Evidence (runtime): the C910 fresh-PG17 procedure persisted only author-managed general feedback while retaining the same opaque `webworkPgml` source binding through two immutable Published Revisions. It did not capture renderer feedback.
  - Mismatch: authorized Student HTTP delivery remains unverified because the server build is blocked by the current AWS Smithy dependency incompatibility.
- [ ] PLE does not extract or reconstruct transient feedback from Question Backend source or output.
  - Evidence (runtime): the C910 procedure changed only author-managed general feedback and retained the PGML binding path and checksum unchanged; it did not extract feedback from source or output.
  - Mismatch: a connected authorized-delivery boundary proof remains unavailable while the server build is blocked by the current AWS Smithy dependency incompatibility.
- [ ] Questions may have PLE-managed general feedback that remains separate from backend-generated interaction feedback.
  - Evidence (runtime): `schemas/base_schema/question_authoring_operations.sql` stores `general_feedback` on immutable Question Revisions separately from the opaque source binding. The accepted fresh-PG17 procedure created an explicit PGML Draft/binding, saved feedback, published Revision 1, then saved feedback only and published Revision 2; old and new feedback read back immutably with the same format, path, and checksum.
  - Mismatch: the authorized Student HTTP projection/release behavior remains unverified because the server build is blocked by the current AWS Smithy dependency incompatibility.
- [ ] H5P owns its runtime, interactions, state, and scoring.
  - Mismatch: C870 leaves no current H5P source, import, adapter, or runtime seam, so H5P cannot yet own delivered runtime behavior.
  - Question: Which H5P content type(s) and exact pinned library versions are supported first; for each which terminal xAPI event/score semantics are authoritative; are scoreless activities non-assessment only?
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
- [ ] Preserve the distinction between WeBWorK PG and PGML source. A Question should be identified as PGML only when its source is fully PGML-compliant; otherwise identify it as PG.
  - Evidence (runtime): the accepted canonical-source inventory records 42 parameterized BiologyProblems.org sources with explicit `pgml` format and matching `.pgml` paths; C910 also proved an explicit `webworkPgml` Draft binding persists across immutable feedback-only publication.
  - Mismatch: the remaining bundled static families have not completed canonical import, publication, and catalog migration, so the product-wide classification is unverified.
- [ ] BiologyProblems.org imports should preserve whether the canonical algorithmic source is PG or PGML rather than treating both formats generically as PG/PGML.
  - Evidence (source): `content/genetics/manifest.yaml` now registers 42 accepted canonical parameterized sources with explicit PGML paths and format metadata.
  - Mismatch: the current static-bank import/catalog migration remains incomplete, so this end-to-end import behavior is unverified.
- [ ] When parameterized WeBWorK PG or PGML source exists, prefer it to importing static variants.
  - Mismatch: no selection policy enforcement or test was found.
- [ ] Preserve backend-native algorithmic variation rather than expanding one algorithmic Question into static variants.
  - Mismatch: `content/genetics` still contains generated static WeBWorK expansions; C824--C841 own the forward replacement.
- [ ] One algorithmic Question remains one Published Question regardless of how many variants its Question Backend can generate.
  - Mismatch: no completed per-family publication and catalog transition proves this lineage boundary.
- [ ] Use a Question Pool with algorithmic Questions only when the Instructor wants selection among distinct Questions, not to represent variants of one algorithmic Question.
  - Mismatch: C885 supplies backend-neutral Pool membership, but no completed Instructor workflow proves the distinct-Question purpose and preserves independent backend variation.
- [ ] BiologyProblems.org WeBWorK problems should be imported from their canonical algorithmic PG or PGML source rather than from generated static variants.
  - Evidence (runtime): C839 accepted 42 canonical PGML sources (41 official biologyproblems-website sources plus HLA) with provenance, format/path, representative render/lint, and deterministic grading evidence.
  - Mismatch: redundant static source files were removed, but ordinary publication and catalog reconciliation remain unverified; C840--C841 own that work.
- [ ] Multiple static BiologyProblems.org questions generated from one algorithmic source represent one Published Question, not separate Published Questions or a Question Pool.
  - Evidence (runtime): C839's 42-source acceptance establishes candidate canonical sources, not a Published-Question lineage.
  - Mismatch: source removal does not prove a per-family Published-Question lineage or catalog migration; C840--C841 remain open.

### Question Pools

- [ ] A **Question Pool** is a set of interchangeable **Published Questions** from which PLE selects for a Student.
  - Mismatch: current pools are Assignment entries, not independent published Question Library objects.
- [ ] Pool contents should represent reasonably interchangeable assessments of the intended learning.
  - Mismatch: no interchangeability validation was found.
- [ ] Question Pools may contain Questions from any Question Backend.
  - Mismatch: C885 supplies backend-neutral Pool membership, but no completed Instructor Pool workflow proves this behavior.
- [x] Each member of a Question Pool is a **Published Question**.
  - Evidence (source): `schemas/base_schema/question_pools.sql` `question_pool_revision_member` stores each exact Published Question revision reference.
- [ ] Question Pools are always published and have no draft or unpublished state.
  - Mismatch: current Question Pools are editable Assignment content rather than published library lineages.
- [ ] A Question Pool is an independently reusable Question Library object.
  - Mismatch: current pools are Assignment entries rather than Library objects.
- [ ] A Question Pool has its own public `AAAA-ZBBB` Crockford Base32 ID and immutable revisions.
  - Mismatch: no published Question Pool lineage or public Pool ID exists.
- [ ] Importing a Question Pool into a new Assessment automatically forks the Question Pool.
  - Mismatch: no independent Pool import-and-fork operation was found.
- [ ] The fork belongs to the new Assessment and can be changed without changing the source Question Pool.
  - Mismatch: no independent Question Pool fork model was found.
- [ ] Forking a Question Pool preserves its Published Questions by their public `AAAA-ZBBB` IDs.
  - Mismatch: no Question Pool fork model exists; current Question IDs use a different display grouping.
- [ ] Question Pools work the same way regardless of the Question Backend.
  - Mismatch: incomplete secondary backend delivery leaves this unverified.
- [x] **Instructors** choose the contents of a Question Pool and how many Questions are selected.
  - Evidence (source): `src/pages/assessment_pool_editor.tsx` `AssessmentPoolEditor` edits item IDs and selection count.
- [x] PLE selects from the Question Pool; the selected Question Backend controls the Question interaction.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `select_question_pool_items` performs server-owned selection.
- [x] Question Pool selection and backend-native randomization are separate forms of variation.
  - Evidence (source): `crates/domain/src/question_pool_selection.rs` `QuestionPoolSelectionEntropy` is separate from Question backend state.
- [x] Returning to an Attempt preserves the Question Pool selections already made.
  - Evidence (source): `schemas/base_schema/assessment_attempt_access.sql` `read_reusable_question_pool_selection` reads durable selections.
  - Evidence (test): `crates/question_model/src/student_work.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks exact retained selections.
- [ ] Starting a new Attempt makes fresh selections from its Question Pools.
  - Mismatch: `crates/learning-data-access/src/postgres/assignment_delivery_start.rs` `current_attempt_start_from_rows` calls `reusable_pool_selection` when the persisted `question_pool_reuse_rule` is `reuse_selection`; `schemas/base_schema/attempt_access.sql` `read_reusable_question_pool_selection` returns the latest prior selection for the same Student and Assignment. `schemas/base_schema/attempts.sql` `question_pool_reuse_rule` permits that mode, so a new Attempt can reuse rather than freshly select its pool membership.
- [x] Student Work preserves the exact Question Pool Revision and Published Question Revision delivered.
  - Evidence (source): `crates/question_model/src/student_work.rs` `QuestionPoolSelection` retains issued Question revision references.
  - Evidence (test): `crates/question_model/src/student_work.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the issued revision link.
- [x] Grading and historical evidence follow the exact Published Question Revision delivered to the Student.
  - Evidence (source): `schemas/base_schema/assessment_attempt_history.sql` `read_student_assessment_attempt_history_response_sources` retains `question_id` and `revision_number`.
  - Evidence (test): `crates/question_model/src/student_work.rs` `question_pool_selection_retains_exact_entries_and_issued_question_link` checks the exact issued linkage.

### Question Library

- [x] Question sharing, discovery, and reuse are a high-priority **Instructor** workflow.
  - Evidence (source): `src/pages/library_route_page.tsx` `LibraryRoutePage` is the production Instructor Library surface.
- [x] The Question Library is one global collection of published Question content.
  - Evidence (source): `schemas/base_schema/question_lineages.sql` `published_question` is not course-scoped.
- [x] **Published Questions** are available to all vetted **Instructors**.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `question_library_entries` requires an active Instructor Account and exposes available Question summaries.
- [ ] Published Question Pools are available to all vetted **Instructors**.
  - Mismatch: published Question Pool library objects do not exist.
- [ ] **Students** access Question content through their Coursework rather than through the Question Library.
  - Mismatch: the Student landing component is source evidence only; no browser or behavior test verifies that Students cannot reach the Question Library.
- [x] Published content remains discoverable when used by a private **Course Instance**.
  - Evidence (source): `crates/question_model/src/question_library.rs` `QuestionSearchResult` is global and separately reports course use.
- [ ] With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.
  - Mismatch: no product test or design enforcement establishes archive as non-primary at this scale.
- [ ] Question Library workflows should support bulk operations because an Instructor may manage thousands of Questions.
  - Mismatch: bounded browsing does not implement the required bulk operations.
- [ ] Instructors should be able to select many Questions and update shared metadata such as tags, subject, topic, or other search fields together.
  - Mismatch: no bulk Question metadata update operation was found.
- [ ] Question Library search, filters, sorting, and bulk editing should make large imports practical to clean up.
  - Mismatch: search and filters exist, but bulk editing is absent.

#### Published Question identity

- [ ] Published Questions receive a public `AAAA-ZBBB` Crockford Base32 ID.
  - Mismatch: `QuestionId` renders `AAA-BBBB`, not the HG-required `AAAA-ZBBB` grouping.
- [ ] Published Questions and published Question Pools have public Crockford Base32 IDs.
  - Mismatch: published Question Pools and their public IDs do not exist.
- [ ] Public IDs use the form `AAAA-ZBBB`.
  - Mismatch: current `QuestionId` uses `AAA-BBBB`, not the HG-required `AAAA-ZBBB` grouping.
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
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` publication appends `next_revision_number` rather than rewriting prior rows.
- [x] The Question owner may publish corrections, wording changes, accessibility improvements, answer changes, grading changes, and other updates as a new Revision.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `publish_question_revision` appends an owner-authored revision.
- [x] Changing Question source, answer content, grading rules, feedback, or Question assets creates a new Question Revision.
  - Evidence (source): `schemas/base_schema/question_authoring_operations.sql` `publish_question_revision` persists a new source binding keyed to a new revision.
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
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `feedback_projection_allowlists_each_released_field` verifies released Question Feedback fields.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `StudentFeedbackReleaseDecision` has a separate Question Feedback decision.
  - Evidence (test): `crates/domain/src/student_feedback_release/tests.rs` `withheld_question_answer_is_absent_while_authorized_feedback_still_releases` verifies separate feedback and answer disclosure.
- [ ] Student workflows remain complete whether or not Students read Question Feedback.
  - Mismatch: release projection is optional, but no end-to-end Student workflow test covers completion with feedback withheld and read.
## Courses

- [ ] **Courses** organize reusable teaching content and its delivery to **Students**.
  - Mismatch: The current Course model does not establish the complete stated product boundary.
- [ ] PLE has two Course forms: **Blueprint Courses** and **Course Instances**.
  - Mismatch: The current paths implement related records but do not verify the complete product distinction.
- [ ] **Blueprint Courses** provide reusable course designs for creating Course Instances.
  - Mismatch: Adoption is implemented only for the current stored Blueprint shape.
- [ ] Course Instances may be created from a Blueprint Course or started empty.
  - Mismatch: `CreateCourseInstanceInput` requires a Blueprint source; no empty creation exists.
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
  - Mismatch: Current `available` availability has no verified vetted-Instructor public-read contract.
- [ ] Blueprint Courses contain only **Published Questions** and published **Question Pools**.
  - Mismatch: Current pin validation covers Question revisions but not the required published Pool behavior.
- [ ] An **Instructor** may deliberately publish an existing Course Instance structure as a new Blueprint Course.
  - Mismatch: No Course Instance-to-Blueprint publishing route or store operation was found.

#### Blueprint Course lifecycle

- [ ] Blueprint Courses have three lifecycle states: **Private**, **Public**, and **Archived**.
  - Mismatch: `blueprint_course.availability` permits only `available` and `archived`.
- [ ] New Blueprint Courses and forks start Private.
  - Mismatch: New records use `available`; no fork implementation was found.
- [ ] Private Blueprint Courses are visible only to their owning **Instructor**.
  - Mismatch: No Private lifecycle state exists.
- [ ] Private Blueprint Courses cannot be adopted to create daughter **Course Instances**.
  - Mismatch: No Private lifecycle state exists.
- [ ] Public Blueprint Courses are visible and reusable by every vetted **Instructor**.
  - Mismatch: No Public lifecycle state or vetted-Instructor contract exists.
  - Owner: Same implementation finding as the earlier Public Blueprint Courses bullet.
- [ ] Public Blueprint Courses can be adopted to create daughter Course Instances.
  - Mismatch: Adoption does not prove the required Public lifecycle gate.
- [ ] Archived Blueprint Courses are read-only and no longer actively maintained.
  - Mismatch: `schemas/base_schema/blueprints.sql` `ple_api.save_blueprint_course` checks ownership but does not reject an Archived Blueprint Course, so its owner can still save changed content.
- [ ] Archived Blueprint Courses remain visible by every vetted **Instructor**.
  - Mismatch: No vetted-Instructor archived visibility evidence was found.
- [ ] Archived Blueprint Courses are excluded from normal search results unless the search explicitly includes them.
  - Mismatch: Blueprint listing has no include-archived search option.
- [x] Archived Blueprint Courses cannot be adopted to create new daughter Course Instances.
  - Evidence (source): `crates/learning-data-access/src/postgres/course_blueprint_adoption.rs` `creation_assignments` resolves an available exact Blueprint Revision before creation.
- [ ] Archived Blueprint Courses can be forked but not adopted.
  - Mismatch: No fork operation exists.
- [ ] The owning **Instructor** can return an Archived Blueprint Course to Public before adopting it again.
  - Mismatch: `restore_blueprint` returns `available`, not the required Public state.
- [ ] Other **Instructors** can fork an Archived Blueprint Course to create a new Private Blueprint Course.
  - Mismatch: No fork operation or Private state exists.
- [x] Blueprint Courses have no separate draft state.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `CHECK (availability IN ('private', 'public', 'archived'))` defines the complete Blueprint availability state.

#### Blueprint Course revisions

- [x] Blueprint Courses use immutable **Blueprint Revisions** for saved reusable content.
  - Evidence (source): `schemas/base_schema/blueprint_revision_integrity.sql` `blueprint_course_revision_is_immutable` rejects Revision updates and deletes.
- [x] Blueprint Course content editing uses explicit Save.
  - Evidence (source): `crates/server/src/blueprint_course.rs` `save_blueprint` is the explicit content-save route handler.
- [x] Saving changed Blueprint content creates the next Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.save_blueprint_course` inserts the next `blueprint_course_revision` when `changed` is true.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts one changed Save creates one new Revision.
- [x] Multiple content edits before Save become one Blueprint Revision.
  - Evidence (source): `crates/question_model/src/blueprint_course/blueprint_children.rs` `ReplaceBlueprintCourseContentInput` carries one complete replacement tree per Save.
- [x] Saving unchanged Blueprint content does not create another Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.save_blueprint_course` returns the expected Revision without inserting when `changed` is false.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts a canonical no-op Save returns Revision 2 with `changed` false.
- [x] Blueprint Course metadata can change without creating a Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.rename_blueprint_course` updates `blueprint_course` metadata without inserting a `blueprint_course_revision`.
- [x] Blueprint Course names are metadata and identify the Blueprint across Revisions.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_course` owns names while `blueprint_course_revision` keys content by course reference and revision.
- [x] Changing a Blueprint Course name does not create a new Blueprint Revision.
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.rename_blueprint_course` updates names and metadata ETag without inserting a `blueprint_course_revision`.

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
- [ ] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Mismatch: No Blueprint update offer, review, or approval operation was found.
- [ ] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Mismatch: No applicable UI or runtime proof exists for this usability behavior.
- [ ] It should be obvious when a Course Instance is using an older Blueprint Revision.
  - Mismatch: No stale-Revision indicator or runtime proof was found.
- [ ] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Mismatch: No Blueprint update workflow exists.
- [ ] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Mismatch: No update workflow exists to verify the non-silent behavior.
- [ ] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Mismatch: No daughter-update implementation exists.

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
  - Evidence (source): `schemas/base_schema/blueprints.sql` `blueprint_revision_assignment` stores Revision-owned reusable assignment positions without delivery settings.
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

- [ ] An **Instructor** can create a Course Instance from a Public Blueprint Course.
  - Mismatch: Creation accepts an exact Blueprint Revision but has no Public-state gate.
- [ ] **Instructors** can also create a new empty Course Instance without a parent Blueprint Course.
  - Mismatch: `CreateCourseInstanceInput` requires Blueprint Course and Revision fields.
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
  - Evidence (source): `schemas/base_schema/blueprints.sql` `ple_api.list_blueprint_courses` computes `total_adoptions` by counting Course Instances with each Blueprint reference.
  - Evidence (test): `crates/learning-data-access/tests/blueprint_course_postgres.rs` `revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` asserts the adopted Blueprint summary has `total_adoptions` equal to 1.
- [x] A Course Instance created from a Blueprint Course is a daughter Course Instance of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance` records Blueprint reference and Revision source columns.
- [x] A daughter Course Instance records its parent Blueprint Course and the exact Blueprint Revision used to create it.
  - Evidence (source): `crates/learning-data-access/src/course_instance.rs` `CreateCourseInstanceInput` includes `blueprint_course` and `blueprint_revision`.
- [x] Creating a Course Instance from a Blueprint Course counts as an adoption of that Blueprint Course.
  - Evidence (source): `schemas/base_schema/course_core.sql` `course_instance_creation_event` records the Blueprint reference and Revision at creation.
- [x] The new Course Instance receives every Assessment from the selected Blueprint Revision.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `initialize_course_assignments` constructs the Course assignments from selected Blueprint content.
- [ ] Creating a Course Instance from a Blueprint Course copies its Assessments, Questions, Question Pools, and reusable settings.
  - Mismatch: Current adoption evidence does not verify published Pool copying.
- [x] Course Instance Assessments created from a Blueprint Course start unreleased with dates unset.
  - Evidence (source): `schemas/base_schema/course_blueprint_adoption.sql` `initialize_course_assignments` initializes adopted assignments as unreleased with delivery dates unset.
- [ ] New Blueprint Revisions are offered to daughter Course Instances for **Instructor** review and approval.
  - Mismatch: No daughter-update implementation exists.
  - Owner: Same implementation finding as the earlier Blueprint update offer bullet.
- [ ] Routine Blueprint updates should be quick for an **Instructor** to review and approve.
  - Mismatch: No applicable UI or runtime proof exists for this usability behavior.
  - Owner: Same implementation finding as the earlier routine Blueprint updates bullet.
- [ ] It should be obvious when a daughter Course Instance is using an older Blueprint Revision.
  - Mismatch: No stale-Revision indicator or runtime proof was found.
- [ ] Changes to existing Assessments follow the Blueprint Revision update workflow.
  - Mismatch: No Blueprint update workflow exists.
  - Owner: Same implementation finding as the earlier Assessment update-workflow bullet.
- [ ] Newly added Blueprint Assessments are automatically added to daughter Course Instances as unreleased Assessments.
  - Mismatch: No daughter-update implementation exists.
  - Owner: Same implementation finding as the earlier newly added Blueprint Assessments bullet.
- [ ] Blueprint changes to existing Assessments are never silently applied to daughter Course Instances.
  - Mismatch: No update workflow exists to verify the non-silent behavior.
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
  - Mismatch: The implemented product calls this object an Assignment.
- [ ] PLE has **Blueprint Assessments** and **Course Instance Assessments**.
  - Mismatch: No Assessment model with these two product categories was found.
- [ ] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Mismatch: Blueprint Assignment source exists, but does not establish the HG Assessment model.
- [ ] Course Instance Assessments deliver Questions to **Students**.
  - Mismatch: Delivery source uses Assignment terminology and contract.
- [ ] All Assessments use the same underlying Assessment model.
  - Mismatch: No shared Assessment model was found.
- [ ] **Assignment** is not a separate object or category. The word appears only in the names
  **Regular Assignment**, **Practice Question Assignment**, and **Bonus Assignment**.
  - Mismatch: Assignment is the current general object name throughout the product.

### Assessment content

- [ ] Assessments contain an ordered sequence of Questions and Question Pools.
  - Mismatch: Assignment entry source was found, but the HG Assessment behavior is not verified.
- [ ] **Instructors** can add, remove, and reorder Questions and Question Pools.
  - Mismatch: Source was not verified through the complete instructor behavior.
- [ ] Questions and Question Pools remain distinct even though both can occupy positions in an Assessment.
  - Mismatch: Current Assignment entry types do not establish the named Assessment behavior.
- [ ] Assessment Question-order randomization is called **Randomize question order**.
  - Mismatch: No matching product label was found.

### Assessment types

- [ ] PLE defines the available Assessment Types.
  - Mismatch: No Assessment Type model was found.
- [ ] Assessment Type describes the pedagogical purpose of an Assessment and provides appropriate defaults.
  - Mismatch: No Assessment Type defaults were found.
- [ ] Assessment Types are **Regular Assignment**, **Practice Question Assignment**, **Bonus Assignment**, **Quiz**, and **Exam**.
  - Mismatch: The five required types are not implemented as a model.
- [ ] **Instructors** select an Assessment Type but cannot create new Assessment Types.
  - Mismatch: No instructor selection-only Assessment Type workflow was found.
- [ ] Blueprint Assessments and Course Instance Assessments use the same Assessment Types.
  - Mismatch: No shared Assessment Type implementation was found.
- [ ] **Instructors** can change Assessment settings independently of the defaults for its Type.
  - Mismatch: No type-default override behavior was verified.
- [ ] Changing Assessment settings does not change its Assessment Type.
  - Mismatch: No Assessment Type identity behavior was found.
- [ ] **Regular Assignments** give **Students** regular practice applying course ideas outside class.
  - Mismatch: No Regular Assignment type behavior was found.
- [ ] Regular Assignments reinforce current learning and may also introduce new topics.
  - Mismatch: No Regular Assignment type behavior was found.
- [ ] Regular Assignments are designed as practice for learning, not merely as one-time assessments.
  - Mismatch: No Regular Assignment type behavior was found.
- [ ] **Practice Question Assignments** provide focused review or study-guide practice using material already covered.
  - Mismatch: No Practice Question Assignment type behavior was found.
- [ ] Practice Question Assignments may be worth a small number of points or a small amount of extra credit.
  - Mismatch: No Practice Question Assignment type behavior was found.
- [ ] Practice Question Assignments use the same whole-Attempt submission boundary as every other
  Assessment and show the correct answer immediately after that Assessment Attempt is submitted.
  - Mismatch: Whole-attempt source exists, but type-specific disclosure is not verified.
- [ ] **Bonus Assignments** provide optional extra credit.
  - Mismatch: No Bonus Assignment type behavior was found.
- [ ] Bonus Assignments are worth zero points possible and add earned points directly to the grade.
  - Mismatch: No Bonus Assignment scoring behavior was found.
- [ ] **Quizzes** assess understanding of recent material.
  - Mismatch: No Quiz type behavior was found.
- [ ] Quizzes may use more restrictive Attempt and collaboration settings than Regular Assignments.
  - Mismatch: No type-specific settings behavior was found.
- [ ] **Exams** are individual assessments associated with scheduled exam periods.
  - Mismatch: No Exam type behavior was found.
- [ ] Exams may use more restrictive Attempt, timing, availability, and feedback settings.
  - Mismatch: No type-specific settings behavior was found.

### Assessment type appearance

- [ ] Each Assessment Type has its own PLE-defined Font Awesome icon.
  - Mismatch: No Assessment Type icon mapping was found.
- [ ] Assessment Type icons remain consistent across PLE themes.
  - Mismatch: No Assessment Type icon mapping was found.
- [ ] Each Assessment Type also has its own theme-defined color.
  - Mismatch: No Assessment Type color mapping was found.
- [ ] Themes may change Assessment Type colors but preserve the meaning of each Type.
  - Mismatch: No Assessment Type theme contract was found.
- [ ] Assessment Type should never be communicated by color alone.
  - Mismatch: No implemented Assessment Type presentation was found.
- [ ] Icons and labels should remain sufficient to identify the Assessment Type without color.
  - Mismatch: No implemented Assessment Type presentation was found.
- [ ] **Regular Assignment** uses the Font Awesome `pen-to-square` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Practice Question Assignment** uses the Font Awesome `arrows-spin` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Bonus Assignment** uses the Font Awesome `sparkles` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Quiz** uses the Font Awesome `square-q` icon.
  - Mismatch: No required icon mapping was found.
- [ ] **Exam** uses the Font Awesome `file-signature` icon.
  - Mismatch: No required icon mapping was found.

### Blueprint Assessments

- [ ] A **Blueprint Assessment** is an Assessment in a **Blueprint Course**.
  - Mismatch: Blueprint Assignment source does not implement the HG object naming/model.
- [ ] Blueprint Assessments define reusable Assessment content and teaching settings.
  - Mismatch: Blueprint Assignment source does not establish the HG Assessment model.
  - Owner: Same implementation finding as the earlier Assessments bullet.
- [ ] Blueprint Assessments have an Assessment Type.
  - Mismatch: No Assessment Type field was found.
- [ ] Blueprint Assessments contain ordered **Published Questions** and published **Question Pools**.
  - Mismatch: Ordered entries exist, but published Question and Pool Assessment behavior is not verified.
  - Owner: Same implementation finding as the earlier Courses bullet.
- [ ] Blueprint Assessments define Question point values and points possible.
  - Mismatch: Point values exist in Assignment source, but Blueprint Assessment behavior is not verified.
- [ ] Blueprint Assessments have no **Students**, Student Work, due dates, release dates, or other Course Instance delivery settings.
  - Mismatch: Blueprint Assignment source does not establish this full absence contract.
- [ ] Blueprint Assessments do not use Assessment Templates.
  - Mismatch: No Assessment Template model was found.
- [ ] Creating a daughter Course Instance from a Blueprint Course copies its Blueprint Assessments into the Course Instance.
  - Mismatch: Copy behavior is outside this source-only verification and uses Assignment terminology.

### Course Instance Assessments

- [ ] A **Course Instance Assessment** is an Assessment in a **Course Instance**.
  - Mismatch: The implemented object is an Assignment.
- [ ] Course Instance Assessments are the Assessments delivered to **Students**.
  - Mismatch: Delivery source implements Assignments rather than the HG Assessment model.
- [ ] Course Instance Assessments have an Assessment Type, Questions, Question Pools, point values, and points possible.
  - Mismatch: No Assessment Type model was found.
- [ ] Course Instance Assessments also have delivery settings such as due dates, release status, and Student availability.
  - Mismatch: Assignment delivery settings exist, but the complete HG Assessment behavior is not verified.
- [ ] Course Instance Assessments copied from a Blueprint Assessment can be changed for the needs of that Course Instance.
  - Mismatch: No verified HG Assessment copy-and-edit behavior was found.
- [ ] Newly added Blueprint Assessments are automatically copied to daughter Course Instances as unreleased Course Instance Assessments.
  - Mismatch: No verified automatic Assessment propagation behavior was found.

### Assessment Templates

- [ ] An **Assessment Template** is a reusable set of settings for creating Course Instance Assessments.
  - Mismatch: No Assessment Template model was found.
- [ ] Assessment Templates are separate from Assessment Types.
  - Mismatch: Neither model was found.
- [ ] Every Assessment Template has one of the five Assessment Types.
  - Mismatch: No Assessment Template or five-type model was found.
- [ ] **Instructors** can create and change their own Assessment Templates.
  - Mismatch: No Assessment Template workflow was found.
- [ ] Assessment Templates provide defaults for settings such as Attempts, timing, scoring, and disclosure.
  - Mismatch: No Assessment Template defaults behavior was found.
- [ ] Creating a Course Instance Assessment from a Template copies its settings into the new Assessment.
  - Mismatch: No Assessment Template creation behavior was found.
- [ ] The new Course Instance Assessment can be changed independently after it is created.
  - Mismatch: No Assessment Template creation behavior was found.
- [ ] Changing an Assessment Template does not change Assessments previously created from it.
  - Mismatch: No Assessment Template behavior was found.
- [ ] Assessment Templates do not contain Questions or Question Pools.
  - Mismatch: No Assessment Template model was found.
- [ ] Blueprint Assessments do not use Assessment Templates.
  - Mismatch: No Assessment Template model was found.
  - Owner: Same implementation finding as the earlier Assessment Templates bullet.

### Course Instance Assessment release and defaults

- [ ] Course Instance Assessments start unreleased.
  - Mismatch: Assignment release exists, but Course Instance Assessment behavior is not verified.
- [ ] Releasing a Course Instance Assessment requires an automated and interactive **Assessment Release Validation** process.
  - Mismatch: Assignment release validation exists, but the HG Assessment workflow is not verified.
- [ ] Assessment Release Validation checks the Assessment settings and data required for release.
  - Mismatch: Current validation is Assignment-specific.
- [ ] Validation should catch missing, invalid, or unreasonable values and explain what the **Instructor** needs to fix.
  - Mismatch: Complete instructor-facing validation behavior was not verified.
- [ ] Release Validation should require a due date at least 24 hours in the future and no later than the
  Course Instance's six-month Active limit.
  - Mismatch: No verified 24-hour and six-month release rule was found.
- [ ] Release Validation should check that release, due, and other dates occur in a valid order.
  - Mismatch: No verified complete date-order validation was found.
- [ ] Release Validation should check required settings such as point values, Attempt limits, and time limits for valid ranges.
  - Mismatch: No verified complete required-setting validation was found.
- [ ] Release Validation should check that the Assessment contains Questions and that required Question settings are valid.
  - Mismatch: No verified Assessment Question validation was found.
- [ ] The **Instructor** should be able to correct validation problems and run Release Validation again.
  - Mismatch: No verified correction-and-rerun workflow was found.
- [ ] An Assessment can be released only after Release Validation passes.
  - Mismatch: Existing release endpoint does not establish the HG Assessment gate.
- [ ] Releasing an Assessment makes it available to **Students** according to its dates and access settings.
  - Mismatch: Assignment delivery source exists, but live access behavior needs runtime evidence.
- [ ] Student Work begins when a **Student** starts an Assessment Attempt.
  - Mismatch: Attempt issuance source exists, but live Student Work behavior needs runtime evidence.
- [ ] New Course Instance Assessments default to accepting submissions only through the due date.
  - Mismatch: No verified default was found.
- [ ] New Course Instance Assessments default to starting new Attempts only through the due date.
  - Mismatch: No verified default was found.
- [ ] Late work defaults to rejected.
  - Mismatch: No verified default was found.
- [ ] Assessment disclosure settings remain separate and independently configurable.
  - Mismatch: No Assessment disclosure model was found.
- [ ] **Regular Assignments** and **Bonus Assignments** should rarely show the correct answer.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] Regular and Bonus Assignments show the **Student's** response and whether it was correct or incorrect.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] **Practice Question Assignments** show correct answers immediately after Assessment Attempt
  submission.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] **Quizzes** and **Exams** show correct answers after all **Students** in the Course have completed the Assessment.
  - Mismatch: No type-specific disclosure behavior was found.
- [ ] Until then, Quizzes and Exams do not disclose correct answers.
  - Mismatch: No type-specific disclosure behavior was found.
- [x] Optional Question Feedback is shown when the Question Backend provides it.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `project_student_feedback` releases backend-provided feedback.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Question Feedback does not use Assessment correct-answer disclosure settings.
  - Evidence (source): `crates/domain/src/student_feedback_release.rs` `StudentFeedbackReleaseDecision` has a separate question-feedback policy field.
  - Owner: Same implementation finding as the earlier Questions bullet.
- [x] Unreleasing a Course Instance Assessment permanently deletes its Student Work and returns to a pre-release state.
  - Evidence (test): `tests/e2e/e2e_unrelease_connected.sh` `psql_admin` runs the connected unrelease deletion and pre-release-state oracle.

### Assessment Attempts

- [ ] An **Assessment Attempt** is one Student attempt at a Course Instance Assessment.
  - Mismatch: The implemented object is an Assignment Attempt.
- [ ] Blueprint Assessments do not have Assessment Attempts.
  - Mismatch: No Blueprint Assessment model was found.
- [ ] Question responses are saved as the **Student** works and remain part of the Attempt across browser sessions.
  - Mismatch: Saved-response persistence is tested after a browser reload, but no evidence establishes persistence across a distinct browser session.
- [ ] **Instructors** control the number of permitted Assessment Attempts.
  - Mismatch: Assignment attempt limit exists, but instructor behavior is not verified.
- [ ] Regular Assignments default to unlimited Attempts.
  - Mismatch: No Regular Assignment type default was found.
- [ ] **Students** may repeat an Assessment as often as its settings allow, including practicing toward a perfect score.
  - Mismatch: Repeat behavior needs runtime evidence.
- [x] When an Assessment permits multiple Attempts, the highest Assessment Attempt score is used as the
  Student's Assessment score.
  - Evidence (test): `crates/domain/src/scoring.rs` `hand_computed_fixture_agrees_for_batch_and_incremental_scoring` proves `AssignmentAttemptGradeRule::Highest` selects the highest completed Attempt score.
- [x] Assessment Attempt submission and grading are fully automatic and require no **Instructor** action.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` commits ordinary Student finalization and checks immutable automated grading evidence.
- [x] Automatic grading does not require a separate Student or **Instructor** grading workflow.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` proves the Student finalization API creates the grading result directly.

### Assessment responses and submission

- [x] The Student submission action submits the whole Assessment Attempt.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` commits one ordinary Student Attempt finalization and asserts one Assignment submission.
- [ ] A Question either has a complete saved response or has no saved response.
  - Mismatch: Complete-response contract was not verified.
- [x] PLE saves complete Question responses as the **Student** works.
  - Evidence (test): `crates/learning-data-access/tests/grading_lifecycle_postgres.rs` `late_save_and_commit_recheck_the_clock_after_waiting_on_their_locks` saves an ordinary Student response and asserts its persisted `saved` state.
- [x] The Student may change a saved response while the Assessment Attempt remains open.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` replaces saved response A with response B before finalization and rejects the stale A snapshot.
- [x] Submitting the Assessment Attempt finalizes all saved Question responses together as Student Work.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_student_assignment_attempt_finalization` commits the prepared saved response into immutable submission and grading evidence.
- [ ] Questions without a saved response remain visibly unanswered when the Attempt is submitted.
  - Mismatch: Visible unanswered-state behavior needs runtime evidence.
- [x] An unanswered Question receives zero credit and counts as incorrect without being sent to the
  Question Backend.
  - Evidence (test): `tests/e2e/attempt_expiry_connected_oracle.sql` `commit_expired_student_assignment_attempt_finalization` verifies an expired all-unanswered Attempt has no invented backend result and zero-credit scoring.
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
  - Evidence (source): `schemas/base_schema/attempts.sql` `issued_question_is_immutable` stores issued Question revision identity under foreign-key protection.
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
  - Evidence (test): `crates/domain/src/scoring.rs` `hand_computed_fixture_agrees_for_batch_and_incremental_scoring` proves `AssignmentAttemptGradeRule::Highest` selects the highest completed Attempt score.
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
