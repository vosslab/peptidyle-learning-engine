## Development principles

### Agent working principles

- [ ] The stack takes a long time to rebuild. Use `./launchers/run_fast_checks.sh` for faster
  interface checks.
  - Evidence (source): `launchers/run_fast_checks.sh` `run_fast_checks` runs the offline aggregate gates, including schema style, Rust, and TypeScript, without starting the live stack.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A Read and learn the core principles in docs/REPO_STYLE.md
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Apply the Keep It Simple, Stupid (KISS) philosophy aggressively.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Prefer the smallest coherent design that meets actual requirements and known failure modes.
  - Reason: Not separately testable or independently closable product behavior; this remains a binding design and implementation-review constraint.
- [ ] Keep configuration simple. Add options, parameters, modes, overrides, and extension points only
  for demonstrated needs. Prefer sensible fixed behavior for internal implementation choices.
  - Verification pending: Current configuration choices have not been audited against this new guidance.
- [ ] When callers need different behavior, first consider whether the shared design can handle it
  automatically or whether the tasks are genuinely different. When in doubt, use the simpler shared design.
  - Verification pending: Current shared designs have not been audited against this new guidance.
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
- [ ] Python code uses the current interpreter's defaults; `from __future__ import ...` belongs nowhere
  in this repo. `tests/test_no_future_imports.py` enforces it.
  - Evidence (test): `tests/test_no_future_imports.py` `test_python_sources_do_not_import_from_future` fails when a tracked Python file imports from __future__.
  - Evidence (source): `devel/change_scope.py` `decide` and `devel/capture_screenshots.py` `print_step` are ordinary Python 3.12 modules with no __future__ import.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A Long local operations must be robust and informative: keep going through imperfect state where
  useful, recover gracefully, and tell me what is happening while I wait. I am impatient.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Classify one-time checks separately from permanent tests.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Finish the obvious. Continue while the next safe step is defined by the plan, implied by the current task.
  - Reason: agent instruction, not implemented PLE product behavior.
- N/A Robust software continues to function despite imperfect inputs, data, state, or behavior. Handle
  imperfections according to their context and impact, recovering gracefully and preserving useful
  operation whenever possible.
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

- [ ] Language-native casing is the right system: SQL stays account_id, Rust/TS types stay AccountId, JSON stays accountId.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `account_id` is the PostgreSQL Account column.
  - Evidence (source): `crates/question_model/src/public_route.rs` `AccountId` is the Rust and generated TypeScript Account type.
  - Evidence (source): `src/api/course_instance.ts` `accountId` is the JSON Account field.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Use `source ./source_me.sh && ./launchers/run_fast_checks.sh` for a quicker compliance check
  - Evidence (source): `launchers/run_fast_checks.sh` `run_fast_checks` is the offline compliance subset, and `source_me.sh` `source_me` prepares the command environment.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Use `source ./source_me.sh && ./launchers/all_test.sh` for a complete compliance check
  - Evidence (source): `launchers/all_test.sh` `check_schema_style` runs schema style, Rust, the codebase check, pytest, and local-stack acceptance after `source_me.sh` `source_me`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Use `source ./source_me.sh && ./devel/capture_screenshots.sh` for UI work to capture fresh screenshots
  - Evidence (source): `devel/capture_screenshots.sh` `capture_screenshots` runs the screenshot driver after `source_me.sh` `source_me`.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Every source file should stay below 1000 lines. Split complete capabilities into focused modules.
  - Evidence (test): `tests/test_source_file_line_limit.py` `test_source_file_line_limit` fails when a tracked authored source file reaches 1000 lines. The current tree passed; three files remain under that limit and warned.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] PLE is pre-production with no users or durable production data. Fix the design directly;
  there is no legacy behavior to preserve.
  - Evidence (source): `crates/project-tools/src/database_coordinator.rs` `base_is_pre_production` refuses database migrate and tells the operator to update the base schema directly until production freeze.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A Use the pre-production state to improve foundational schemas, contracts, and abstractions
  whenever that produces a stronger long-term system.
  - Reason: repository process guidance; it makes no current PLE product behavior claim.
- [ ] Use SQL directly to create the initial PostgreSQL database structure. Insertions have more flexibility.
  - Evidence (source): `schemas/base_schema/install.sql` `00_roles.sql` installs the structural schema as ordered SQL includes.
  - Evidence (source): `schemas/installation_data/live_demo.sql` `course_roster_profile` inserts Live Demo rows separately from that structural manifest.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Before production, edit the main database design directly as the design changes.
  - Evidence (source): `crates/project-tools/src/database_coordinator.rs` `run` reports that the base schema must be updated directly until production freeze.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] After production, update existing databases without rebuilding them from scratch.
  - Evidence (source): `local_stack_control/lifecycle_migrations.py` `database_operation_for` selects `migrate` after initial installation.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Use readable `snake_case` whenever possible; see [NAMING_CONVENTIONS.md](/docs/NAMING_CONVENTIONS.md) for details.
  - Evidence (source): `devel/development_conformance_audit.py` `current_source_paths` inventories tracked and untracked current-worktree source files without opening deleted paths; `source_name_violations` enforces readable snake_case names.
  - Evidence (source): `devel/development_conformance_audit.py` `load_allowlist` permits only exact documented external-name exceptions owned by approved authority sections.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A Give variables for distinct concepts distinct names. For example, `qti_package_upload_file`,
  `qti_package_archive`, `qti_package_extracted_image`, `question_image_asset`,
  `question_image_rendition`, and `object_id` name different roles.
  - Reason: authoring guidance with examples; it makes no separately closable product behavior claim.
- N/A Use the clearest variable name, including a longer name when it better expresses the value; name length has no runtime cost.
  - Reason: authoring guidance; it makes no separately closable product behavior claim.
- N/A Adaptability should be a focus so the software can evolve as requirements and insights change.
  - Reason: future development-direction guidance; it makes no current PLE product behavior claim.
- [ ] Cargo, Node, and PyPI dependencies should use the latest versions to include security fixes.
  - Evidence (source): `devel/dependency_freshness_audit.py` `check_cargo`, `check_node`, and `check_pypi` fail closed unless every direct manifest dependency matches the dated primary-registry snapshot.
  - Evidence (source): `devel/dependency_freshness_snapshot.json` `"snapshot_date": "2026-09-14"` records every direct dependency. `cargo_exceptions` is empty; the only range exception is TypeScript's documented upstream compatibility cap.
  - Evidence (test): `tests/test_crate_boundaries.py` `test_registry_dependencies_use_open_latest_first_requirements` retains Cargo's open latest-first manifest contract.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A If an interface is measured as too slow, consider moving the slow code to Rust/WebAssembly.
  - Reason: conditional future implementation option; it makes no current PLE product behavior claim.
- [ ] All fields, identifiers, domain concepts, and terminology in the PostgreSQL database structure, Rust code,
  TypeScript code, JSON/API contracts, Terminology Contract, and Human Guidance are in alignment.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.

### PLE development rules

- [ ] A fresh production installation includes the complete Live Demo by default.
  - Evidence (source): `crates/project-tools/src/installation_data.rs` `parse_arguments` selects Live Demo provisioning when the command is `provision` without `--without-live-demo`.
  - Evidence (test): `tests/e2e/e2e_fresh_install_without_live_demo_launcher.sh` `fresh_install_includes_live_demo_and_genetics` ran that default command on a fresh schema and found Course BCHM 301, Biochemistry 301: Proteins and Peptides. Disposable Postgres, object storage, and the API were removed. The Live Demo launcher was not started.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Treat the initial course content as shipped examples.
  - Evidence (source): `schemas/installation_data/live_demo.sql` `ple_data.course_instance` is seeded as installation-owned Live Demo teaching data.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] BiologyProblems.org content is free and open source.
  - Evidence (source): `content/genetics/ATTRIBUTION.md` `CC BY 4.0` records the bundled Biology Problems OER content license.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] The Genetics Blueprint Course from BiologyProblems.org ships as the example course.
  - Evidence (source): `content/genetics/manifest.yaml` `short_name` and `long_name` name the bundled Genetics example, and `crates/project-tools/src/installation_data.rs` `publish_bundled_genetics` publishes it through the ordinary owner API.
  - Evidence (test): `tests/e2e/e2e_fresh_install_without_live_demo_launcher.sh` `fresh_install_includes_live_demo_and_genetics` found Genetics|Fall Genetics|public after the default provision command. Disposable Postgres, object storage, and the API were removed. The Live Demo launcher was not started.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- N/A All Podman content on the Mac-Studio-36G machine belongs to this project.
  - Reason: human ownership statement about a named machine, not implemented PLE behavior.
- N/A Neil pre-approves pruning Podman images, volumes, and containers on Mac-Studio-36G as needed and
  has provided the script `./devel/prune_podman.sh` for doing the pruning
  - Reason: human authorization statement, not implemented PLE behavior.
- N/A The polished PLE Live Demo is the top priority; see [LIVE_DEMO_SPEC.md](/docs/LIVE_DEMO_SPEC.md).
  - Reason: human-owned project priority, not implemented PLE behavior.
- [ ] PLE should use one global installation with no institution boundaries.
  - Evidence (source): `schemas/base_schema/50_functions/accounts.sql` `ple_private.account` has no institution column or foreign key; user roles are global account data.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] Project images and simulated test data are disposable and can be recreated for testing.
  - Evidence (source): `local_stack_control/disposable_stack_adapter.py` `disposable_target` creates a closed disposable Compose target for acceptance infrastructure.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
- [ ] `./launchers/run_live_demo.sh` is the normal local-stack entry point. For direct controller
  diagnostics, use `source source_me.sh && python3 local_stack.py`.
  - Evidence (source): `launchers/run_live_demo.sh` launcher delegates the normal Live Demo start to `local_stack.py`.
  - Evidence (source): `local_stack.py` `main` is the direct local-stack controller entry point.
  - Generated evidence stale: Evidence retained from the pre-interview audit; it has not been revalidated against current HG in this pass.
  - Verification pending: Current implementation audit; see docs/active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md for fresh findings and scope.
