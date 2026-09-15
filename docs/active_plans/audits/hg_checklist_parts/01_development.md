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
