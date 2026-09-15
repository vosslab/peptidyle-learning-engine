# Human Guidance compliance summary

Temporary closeout report for the corpus-wide documentation reconciliation.

## Authority and boundary

[HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md) is the authority for current PLE product intent.
This pass changes only files below `docs/`. Source code, schemas, tests, configuration,
migrations, and generators were not changed.

## Review method and evidence

The pass combined a complete file inventory with semantic searches and manual comparison of the
current product, architecture, authorization, data, Assessment, Question, UI, operations, plan, and
example documents. It reviewed meaning rather than spelling alone. Compatible detail remains;
historical and generated evidence is explicitly subordinate instead of being rewritten as current
product intent.

## Corpus review outcome

- 295 files inventoried.
- 167 files received substantive documentation changes.
- 73 files reviewed with no content change needed.
- 55 generated files or visual artifacts remain source-owned follow-up work; two of their text
  files received authority notices outside the generated body.
- The remaining product area without a locked-in design is recorded in
  [UNRESOLVED_OR_AMBIGUOUS_ITEMS.md](UNRESOLVED_OR_AMBIGUOUS_ITEMS.md).

Changed documents now use the Human Guidance Assessment, Blueprint, role, retention, Question,
backend, submission, grading, and interface models. The whole Assessment Attempt is the only
submission target; saving changes a working response, and Attempt submission finalizes all saved
responses together. The three active plans and the active Assessment Type decision were reconciled
directly. Changelog content carries an authority notice; archive content remains historical evidence
by its path and inventory classification. Human-readable dated audits, workstreams, and retained
reports now carry file-local evidence notices. Loose source-note and completed-plan files that could
otherwise look current carry their own authority notices.

A KISS triage then resolved the manufactured product ambiguities. Assessment is the generic object
and has exactly five Assessment Types; Assignment is not an object or parent category. Practice
Question Assignments use the ordinary whole-Attempt boundary and disclose correct answers
immediately after submission. Optional Question Feedback does not use correct-answer disclosure
settings. Unanswered Questions receive zero credit and count as incorrect without being sent to a
backend, and the highest submitted Attempt score is used. PLE calculates point-based Assessment scores without a
separate weighting or Course-grade model; pilot export is CSV or TSV. Pool Revisions are explicit,
new terms use new Course Instances, backend evaluation has no deferred result state, and no permanent
Account-closure workflow is currently defined. Course Instances represent one teaching period,
remain Active for at most six months from creation, and permit bulk roster import for adding
Students. Student removal is individual; PLE has no bulk-removal workflow. The latest Assessment
deadline starts a separate FERPA retention clock but does not itself archive or remove Student data;
the configured policy determines the later notice, archive, recovery, and deletion transitions. The
six-month Active limit caps deadline movement so Course reuse cannot indefinitely delay that FERPA
path, while becoming Inactive does not itself delete Student records.

Course banners use one responsive 5:1 geometry, with 1280 by 256 pixels as the recommended authoring
size; the former 6:1 hero, 5:2 card crop, and dual-rendition design are superseded.

## Independent audit follow-up

Six fresh reviewers completed independent Plan, Test, Style, Documentation, Legacy, and Comment
passes after the initial closeout. The Plan pass reported no finding. The other passes found and
corrected stale authentication status, ambiguous submission targets, then-unsupported unanswered-score
and retry claims, overbroad backend evaluation and score-retention wording, universal adapter
snapshot machinery, non-clickable report references, and ambiguous generated-artifact statuses.

Two audit risks remain explicit:

- These temporary reports still use a nested folder and SCREAMING_SNAKE_CASE filenames rather than
  the active-plan folder's snake_case prefix convention. Correcting that requires a coordinated
  `git mv` and link update; the Git index was read-only during this pass.
- The complete inventory proves that every file received a status classification, but it is not a
  per-file semantic concern matrix. The categorized reports and six-pass audit provide review
  evidence, not a machine-verifiable proof of meaning. No reviewer recommended a permanent test for
  this one-time corpus state.

## Inventory status legend

- **Changed:** The file was created or received substantive edits in this docs-only pass.
- **Reviewed - no change needed:** The content is compatible, out of product scope, or
  deliberately historical as classified in this report.
- **Changed notice; generated body needs source-owned refresh:** A docs-side authority notice was added outside
  generated content that still needs a source-owned refresh.
- **Follow-up:** The artifact is generated from source/UI ownership outside `docs/` and still
  needs a source-owned refresh.

## Complete corpus inventory

| File | Status |
| --- | --- |
| [docs/ACTIVITY_MODEL.md](../../../ACTIVITY_MODEL.md) | Changed |
| [docs/ADAPTER_DEVELOPMENT.md](../../../ADAPTER_DEVELOPMENT.md) | Changed |
| [docs/API_CONTRACTS.md](../../../API_CONTRACTS.md) | Changed |
| [docs/ASSESSMENT_LIFECYCLE.md](../../../ASSESSMENT_LIFECYCLE.md) | Changed |
| [docs/ASSESSMENT_PAYLOAD_DESIGN.md](../../../ASSESSMENT_PAYLOAD_DESIGN.md) | Changed |
| [docs/AUTHORIZATION_CONTRACTS.md](../../../AUTHORIZATION_CONTRACTS.md) | Changed |
| [docs/AUTHORS.md](../../../AUTHORS.md) | Reviewed - no change needed |
| [docs/BLOOM_TAXONOMY_GUIDE.md](../../../BLOOM_TAXONOMY_GUIDE.md) | Changed |
| [docs/CACHING_AND_PREFETCH.md](../../../CACHING_AND_PREFETCH.md) | Changed |
| [docs/CHANGELOG-2026-08a.md](../../../CHANGELOG-2026-08a.md) | Changed |
| [docs/CHANGELOG-2026-08b.md](../../../CHANGELOG-2026-08b.md) | Changed |
| [docs/CHANGELOG-2026-08c.md](../../../CHANGELOG-2026-08c.md) | Changed |
| [docs/CHANGELOG-2026-08d.md](../../../CHANGELOG-2026-08d.md) | Changed |
| [docs/CHANGELOG-2026-09a.md](../../../CHANGELOG-2026-09a.md) | Changed |
| [docs/CHANGELOG-2026-09b.md](../../../CHANGELOG-2026-09b.md) | Changed |
| [docs/CHANGELOG-2026-09c.md](../../../CHANGELOG-2026-09c.md) | Changed |
| [docs/CHANGELOG-2026-09d.md](../../../CHANGELOG-2026-09d.md) | Changed |
| [docs/CHANGELOG-2026-09e.md](../../../CHANGELOG-2026-09e.md) | Changed |
| [docs/CHANGELOG-2026-09f.md](../../../CHANGELOG-2026-09f.md) | Changed |
| [docs/CHANGELOG-2026-09g.md](../../../CHANGELOG-2026-09g.md) | Changed |
| [docs/CHANGELOG.md](../../../CHANGELOG.md) | Changed |
| [docs/CLAUDE_HOOK_USAGE_GUIDE.md](../../../CLAUDE_HOOK_USAGE_GUIDE.md) | Reviewed - no change needed |
| [docs/CODEX_SPARK_SUBAGENTS.md](../../../CODEX_SPARK_SUBAGENTS.md) | Reviewed - no change needed |
| [docs/CODE_ARCHITECTURE.md](../../../CODE_ARCHITECTURE.md) | Changed |
| [docs/COLOR_CONTRAST_ACCESSIBILITY.md](../../../COLOR_CONTRAST_ACCESSIBILITY.md) | Reviewed - no change needed |
| [docs/CONCURRENCY_CONTRACTS.md](../../../CONCURRENCY_CONTRACTS.md) | Changed |
| [docs/CONTAINER_PORT_MAPPING.md](../../../CONTAINER_PORT_MAPPING.md) | Reviewed - no change needed |
| [docs/CONTRACTS.md](../../../CONTRACTS.md) | Changed |
| [docs/COOKBOOK.md](../../../COOKBOOK.md) | Changed |
| [docs/DATABASE_AUTHORIZATION.md](../../../DATABASE_AUTHORIZATION.md) | Changed |
| [docs/DATABASE_STRUCTURE.md](../../../DATABASE_STRUCTURE.md) | Changed |
| [docs/DATA_CLASSIFICATION.md](../../../DATA_CLASSIFICATION.md) | Changed |
| [docs/DATA_CONTRACTS.md](../../../DATA_CONTRACTS.md) | Changed |
| [docs/DESIGN_DECISIONS.md](../../../DESIGN_DECISIONS.md) | Changed |
| [docs/DESIGN_DECISIONS_OPERATIONS.md](../../../DESIGN_DECISIONS_OPERATIONS.md) | Changed |
| [docs/DETERMINISM_CONTRACT.md](../../../DETERMINISM_CONTRACT.md) | Changed |
| [docs/DEVELOPMENT.md](../../../DEVELOPMENT.md) | Changed |
| [docs/E2E_TESTS.md](../../../E2E_TESTS.md) | Reviewed - no change needed |
| [docs/ENROLLMENT_DESIGN.md](../../../ENROLLMENT_DESIGN.md) | Changed |
| [docs/FAILURE_RECOVERY.md](../../../FAILURE_RECOVERY.md) | Changed |
| [docs/FAQ.md](../../../FAQ.md) | Changed |
| [docs/FILE_STRUCTURE.md](../../../FILE_STRUCTURE.md) | Changed |
| [docs/FRONTEND_ARCHITECTURE.md](../../../FRONTEND_ARCHITECTURE.md) | Changed |
| [docs/FUN_VIBES_DESIGN_STYLE.md](../../../FUN_VIBES_DESIGN_STYLE.md) | Reviewed - no change needed |
| [docs/GMAIL_EMAIL_DELIVERY_BACKEND.md](../../../GMAIL_EMAIL_DELIVERY_BACKEND.md) | Changed |
| [docs/GRAPHIFY.md](../../../GRAPHIFY.md) | Changed notice; generated body needs source-owned refresh |
| [docs/GRAPHIFY_map.svg](../../../GRAPHIFY_map.svg) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md) | Changed |
| [docs/IDENTITY_CONTRACTS.md](../../../IDENTITY_CONTRACTS.md) | Changed |
| [docs/INPUT_FORMATS.md](../../../INPUT_FORMATS.md) | Changed |
| [docs/INSTALL.md](../../../INSTALL.md) | Changed |
| [docs/INSTRUCTOR_GUIDE.md](../../../INSTRUCTOR_GUIDE.md) | Changed |
| [docs/INSTRUCTOR_PAGE_VISUALS.md](../../../INSTRUCTOR_PAGE_VISUALS.md) | Changed |
| [docs/INTERFACE_TERMINOLOGY.md](../../../INTERFACE_TERMINOLOGY.md) | Changed |
| [docs/LIVE_DEMO_SPEC.md](../../../LIVE_DEMO_SPEC.md) | Changed |
| [docs/LOCAL_STACK_ARCHITECTURE.md](../../../LOCAL_STACK_ARCHITECTURE.md) | Changed |
| [docs/LOCAL_STACK_OPERATIONS.md](../../../LOCAL_STACK_OPERATIONS.md) | Changed |
| [docs/MACOS_PODMAN.md](../../../MACOS_PODMAN.md) | Reviewed - no change needed |
| [docs/MARKDOWN_STYLE.md](../../../MARKDOWN_STYLE.md) | Reviewed - no change needed |
| [docs/MASTERY_ASSIGNMENT_DESIGN.md](../../../MASTERY_ASSIGNMENT_DESIGN.md) | Changed |
| [docs/MULTI_SERVER_SETUP.md](../../../MULTI_SERVER_SETUP.md) | Changed |
| [docs/NAMING_CONVENTIONS.md](../../../NAMING_CONVENTIONS.md) | Changed |
| [docs/NEWS.md](../../../NEWS.md) | Changed |
| [docs/NO_MOUSE_ACCESSIBILITY_CONTRACT.md](../../../NO_MOUSE_ACCESSIBILITY_CONTRACT.md) | Changed |
| [docs/OBJECT_STORAGE.md](../../../OBJECT_STORAGE.md) | Changed |
| [docs/PALETTE_CONTRAST_AUDIT.md](../../../PALETTE_CONTRAST_AUDIT.md) | Reviewed - no change needed |
| [docs/PILOT_CONTENT.md](../../../PILOT_CONTENT.md) | Changed |
| [docs/PLAYFUL_TRAINING_GAME_STYLE.md](../../../PLAYFUL_TRAINING_GAME_STYLE.md) | Reviewed - no change needed |
| [docs/PLAYWRIGHT_TEST_STYLE.md](../../../PLAYWRIGHT_TEST_STYLE.md) | Reviewed - no change needed |
| [docs/PLAYWRIGHT_USAGE.md](../../../PLAYWRIGHT_USAGE.md) | Reviewed - no change needed |
| [docs/PORTSWIGGER_SECURITY_REVIEW_REFERENCE.md](../../../PORTSWIGGER_SECURITY_REVIEW_REFERENCE.md) | Reviewed - no change needed |
| [docs/PYTEST_AUTHORING_GUIDE.md](../../../PYTEST_AUTHORING_GUIDE.md) | Reviewed - no change needed |
| [docs/PYTEST_STYLE.md](../../../PYTEST_STYLE.md) | Reviewed - no change needed |
| [docs/PYTHON_STYLE.md](../../../PYTHON_STYLE.md) | Reviewed - no change needed |
| [docs/QTI-JSON_OBJECT_FORMAT.md](../../../QTI-JSON_OBJECT_FORMAT.md) | Changed |
| [docs/QUESTION_BACKEND_CONTRACTS.md](../../../QUESTION_BACKEND_CONTRACTS.md) | Changed |
| [docs/QUESTION_ID_SPEC.md](../../../QUESTION_ID_SPEC.md) | Changed |
| [docs/QUESTION_MODEL.md](../../../QUESTION_MODEL.md) | Changed |
| [docs/RELATED_PROJECTS.md](../../../RELATED_PROJECTS.md) | Changed |
| [docs/RELEASE_HISTORY.md](../../../RELEASE_HISTORY.md) | Changed |
| [docs/REPO_STYLE.md](../../../REPO_STYLE.md) | Reviewed - no change needed |
| [docs/RETENTION_POLICY.md](../../../RETENTION_POLICY.md) | Changed |
| [docs/ROADMAP.md](../../../ROADMAP.md) | Changed |
| [docs/RUST_PYO3_STYLE.md](../../../RUST_PYO3_STYLE.md) | Reviewed - no change needed |
| [docs/RUST_STYLE.md](../../../RUST_STYLE.md) | Reviewed - no change needed |
| [docs/RUST_WASM_STYLE.md](../../../RUST_WASM_STYLE.md) | Reviewed - no change needed |
| [docs/SCREENSHOT_ATLAS.md](../../../SCREENSHOT_ATLAS.md) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/SCREENSHOT_CONTRACT.md](../../../SCREENSHOT_CONTRACT.md) | Changed |
| [docs/SECURITY_MODEL.md](../../../SECURITY_MODEL.md) | Changed |
| [docs/SOLID_MODEL.md](../../../SOLID_MODEL.md) | Changed |
| [docs/STORAGE_CONSISTENCY.md](../../../STORAGE_CONSISTENCY.md) | Changed |
| [docs/STUDENT_GUIDE.md](../../../STUDENT_GUIDE.md) | Changed |
| [docs/STUDENT_PAGE_VISUALS.md](../../../STUDENT_PAGE_VISUALS.md) | Changed |
| [docs/TERMINOLOGY_CONTRACT.md](../../../TERMINOLOGY_CONTRACT.md) | Changed |
| [docs/TEST_EVIDENCE_MODEL.md](../../../TEST_EVIDENCE_MODEL.md) | Changed |
| [docs/TODO.md](../../../TODO.md) | Changed |
| [docs/TROUBLESHOOTING.md](../../../TROUBLESHOOTING.md) | Reviewed - no change needed |
| [docs/TYPESCRIPT_STYLE.md](../../../TYPESCRIPT_STYLE.md) | Reviewed - no change needed |
| [docs/UI_DESIGN_GUIDE.md](../../../UI_DESIGN_GUIDE.md) | Changed |
| [docs/UI_DESIGN_REVIEW.md](../../../UI_DESIGN_REVIEW.md) | Changed |
| [docs/USAGE.md](../../../USAGE.md) | Changed |
| [docs/USER_ROLES.md](../../../USER_ROLES.md) | Changed |
| [docs/WEBWORK_PG_RENDERER_API_USAGE.md](../../../WEBWORK_PG_RENDERER_API_USAGE.md) | Changed |
| [docs/active_plans/2026-09-09-notes.txt](../../2026-09-09-notes.txt) | Changed |
| [docs/active_plans/active/production_release_readiness_phase_plan-v2.md](../../active/production_release_readiness_phase_plan-v2.md) | Changed |
| [docs/active_plans/active/student_decisions_and_grading_trust_plan.md](../../active/student_decisions_and_grading_trust_plan.md) | Changed |
| [docs/active_plans/active/webwork_opaque_backend_plan.md](../../active/webwork_opaque_backend_plan.md) | Changed |
| [docs/active_plans/audits/course_appearance_six_pass_review.md](../../audits/course_appearance_six_pass_review.md) | Changed |
| [docs/active_plans/audits/instructor_student_walkthrough_plan_review.md](../../audits/instructor_student_walkthrough_plan_review.md) | Changed |
| [docs/active_plans/audits/m3_arrangement_integration_review.md](../../audits/m3_arrangement_integration_review.md) | Changed |
| [docs/active_plans/audits/m5_retained_pagination_blocker_review.md](../../audits/m5_retained_pagination_blocker_review.md) | Changed |
| [docs/active_plans/audits/pagination_course_assignments_review.md](../../audits/pagination_course_assignments_review.md) | Changed |
| [docs/active_plans/audits/pagination_gradebook_review.md](../../audits/pagination_gradebook_review.md) | Changed |
| [docs/active_plans/audits/pagination_product_final_review.md](../../audits/pagination_product_final_review.md) | Changed |
| [docs/active_plans/audits/pagination_walkthrough_integration_review.md](../../audits/pagination_walkthrough_integration_review.md) | Changed |
| [docs/active_plans/audits/peptidyle_walkthrough_final_review.md](../../audits/peptidyle_walkthrough_final_review.md) | Changed |
| [docs/active_plans/audits/pilot_final_retained_live_review.md](../../audits/pilot_final_retained_live_review.md) | Changed |
| [docs/active_plans/audits/pilot_identity_startup_review.md](../../audits/pilot_identity_startup_review.md) | Changed |
| [docs/active_plans/audits/ui_walkthrough_plan_consistency_audit.md](../../audits/ui_walkthrough_plan_consistency_audit.md) | Changed |
| [docs/active_plans/audits/walkthrough_plan_reconciliation_review.md](../../audits/walkthrough_plan_reconciliation_review.md) | Changed |
| [docs/active_plans/audits/wp_a2_assignment_arrangement_review.md](../../audits/wp_a2_assignment_arrangement_review.md) | Changed |
| [docs/active_plans/audits/wp_e1_v2_report_review.md](../../audits/wp_e1_v2_report_review.md) | Changed |
| [docs/active_plans/audits/wp_e2_walkthrough_library_architecture_review.md](../../audits/wp_e2_walkthrough_library_architecture_review.md) | Changed |
| [docs/active_plans/audits/wp_g1_harness_independence_independent_review.md](../../audits/wp_g1_harness_independence_independent_review.md) | Changed |
| [docs/active_plans/audits/wp_g2_walked_journey_baseline_review.md](../../audits/wp_g2_walked_journey_baseline_review.md) | Changed |
| [docs/active_plans/audits/wp_i1_course_creation_review.md](../../audits/wp_i1_course_creation_review.md) | Changed |
| [docs/active_plans/audits/wp_i2_local_roster_backend_review.md](../../audits/wp_i2_local_roster_backend_review.md) | Changed |
| [docs/active_plans/audits/wp_i2_local_roster_hci_review.md](../../audits/wp_i2_local_roster_hci_review.md) | Changed |
| [docs/active_plans/audits/wp_i4_instructor_setup_hci_review.md](../../audits/wp_i4_instructor_setup_hci_review.md) | Changed |
| [docs/active_plans/audits/wp_o1_python_runner_review.md](../../audits/wp_o1_python_runner_review.md) | Changed |
| [docs/active_plans/audits/wp_o1_walkthrough_runner_review.md](../../audits/wp_o1_walkthrough_runner_review.md) | Changed |
| [docs/active_plans/audits/wp_o2_live_playwright_review.md](../../audits/wp_o2_live_playwright_review.md) | Changed |
| [docs/active_plans/audits/wp_rc8_onboarding_preflight_evidence_review.md](../../audits/wp_rc8_onboarding_preflight_evidence_review.md) | Changed |
| [docs/active_plans/audits/wp_s1_catalog_binding_hci_review.md](../../audits/wp_s1_catalog_binding_hci_review.md) | Changed |
| [docs/active_plans/audits/wp_s1_catalog_binding_security_review.md](../../audits/wp_s1_catalog_binding_security_review.md) | Changed |
| [docs/active_plans/audits/wp_s1_student_repeat_hci_review.md](../../audits/wp_s1_student_repeat_hci_review.md) | Changed |
| [docs/active_plans/audits/wp_v1_deterministic_decisions_review.md](../../audits/wp_v1_deterministic_decisions_review.md) | Changed |
| [docs/active_plans/audits/wp_v2_visible_outcome_report_review.md](../../audits/wp_v2_visible_outcome_report_review.md) | Changed |
| [docs/active_plans/audits/wp_v3_failure_triage_review.md](../../audits/wp_v3_failure_triage_review.md) | Changed |
| [docs/active_plans/audits/wp_w1_first_keyboard_journey_review.md](../../audits/wp_w1_first_keyboard_journey_review.md) | Changed |
| [docs/active_plans/audits/wp_w2_frontend_refresh_review.md](../../audits/wp_w2_frontend_refresh_review.md) | Changed |
| [docs/active_plans/audits/wp_w2_native_retry_lifecycle_review.md](../../audits/wp_w2_native_retry_lifecycle_review.md) | Changed |
| [docs/active_plans/audits/wp_w2_receipt_authoritative_completion_review.md](../../audits/wp_w2_receipt_authoritative_completion_review.md) | Changed |
| [docs/active_plans/audits/wp_w2_response_lifecycle_review.md](../../audits/wp_w2_response_lifecycle_review.md) | Changed |
| [docs/active_plans/audits/wp_w2_retry_until_correct_hci_review.md](../../audits/wp_w2_retry_until_correct_hci_review.md) | Changed |
| [docs/active_plans/audits/wp_w2_retry_until_correct_report_review.md](../../audits/wp_w2_retry_until_correct_report_review.md) | Changed |
| [docs/active_plans/audits/wp_w3_leave_return_review.md](../../audits/wp_w3_leave_return_review.md) | Changed |
| [docs/active_plans/audits/wp_w6_exam_copy_fix_review.md](../../audits/wp_w6_exam_copy_fix_review.md) | Changed |
| [docs/active_plans/audits/wp_w6_policy_contrast_review.md](../../audits/wp_w6_policy_contrast_review.md) | Changed |
| [docs/active_plans/decisions/assessment_type_terminology.md](../../decisions/assessment_type_terminology.md) | Changed |
| [docs/active_plans/decisions/assets/course_appearance_banner_ratio_specimen.svg](../../decisions/assets/course_appearance_banner_ratio_specimen.svg) | Changed |
| [docs/active_plans/decisions/course_appearance_banner_storage_and_sizing.md](../../decisions/course_appearance_banner_storage_and_sizing.md) | Changed |
| [docs/active_plans/phase_1_plan_goal.md](../../phase_1_plan_goal.md) | Changed |
| [docs/active_plans/reports/fall_genetics_walkthrough_2026_09.md](../fall_genetics_walkthrough_2026_09.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/ARCHITECTURE_AND_IMPLEMENTATION_CHANGES.md](ARCHITECTURE_AND_IMPLEMENTATION_CHANGES.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/AUTHORIZATION_AND_FERPA_CHANGES.md](AUTHORIZATION_AND_FERPA_CHANGES.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/COMPLIANCE_SUMMARY.md](COMPLIANCE_SUMMARY.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/PRODUCT_CONFLICTS.md](PRODUCT_CONFLICTS.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/QUESTION_AND_ASSESSMENT_CHANGES.md](QUESTION_AND_ASSESSMENT_CHANGES.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/TERMINOLOGY_AND_MODEL_CHANGES.md](TERMINOLOGY_AND_MODEL_CHANGES.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/UI_AND_WORKFLOW_CHANGES.md](UI_AND_WORKFLOW_CHANGES.md) | Changed |
| [docs/active_plans/reports/human_guidance_compliance/UNRESOLVED_OR_AMBIGUOUS_ITEMS.md](UNRESOLVED_OR_AMBIGUOUS_ITEMS.md) | Changed |
| [docs/active_plans/reports/webwork_opaque_e2e_findings.md](../webwork_opaque_e2e_findings.md) | Changed |
| [docs/active_plans/reports/webwork_opaque_render_findings.md](../webwork_opaque_render_findings.md) | Changed |
| [docs/active_plans/review-the-screenshots-in-glistening-acorn.md](../../review-the-screenshots-in-glistening-acorn.md) | Changed |
| [docs/active_plans/screenshot_llm_thoughts.md](../../screenshot_llm_thoughts.md) | Changed |
| [docs/active_plans/virtual-doodling-honey.md](../../virtual-doodling-honey.md) | Changed |
| [docs/active_plans/walked_journey_baseline.json](../../walked_journey_baseline.json) | Reviewed - no change needed |
| [docs/active_plans/walked_journey_baseline_v2.json](../../walked_journey_baseline_v2.json) | Reviewed - no change needed |
| [docs/active_plans/workstreams/m3_arrangement_integration.md](../../workstreams/m3_arrangement_integration.md) | Changed |
| [docs/active_plans/workstreams/wp_a1_retry_corpus.md](../../workstreams/wp_a1_retry_corpus.md) | Changed |
| [docs/active_plans/workstreams/wp_a2_assignment_arrangement.md](../../workstreams/wp_a2_assignment_arrangement.md) | Changed |
| [docs/active_plans/workstreams/wp_e1_corrected_v2_report.md](../../workstreams/wp_e1_corrected_v2_report.md) | Changed |
| [docs/active_plans/workstreams/wp_e2_walkthrough_runner_library.md](../../workstreams/wp_e2_walkthrough_runner_library.md) | Changed |
| [docs/active_plans/workstreams/wp_g2_walked_journey_baseline.md](../../workstreams/wp_g2_walked_journey_baseline.md) | Changed |
| [docs/active_plans/workstreams/wp_i1_course_creation.md](../../workstreams/wp_i1_course_creation.md) | Changed |
| [docs/active_plans/workstreams/wp_i3_assignment_creation.md](../../workstreams/wp_i3_assignment_creation.md) | Changed |
| [docs/active_plans/workstreams/wp_i4_instructor_setup.md](../../workstreams/wp_i4_instructor_setup.md) | Changed |
| [docs/active_plans/workstreams/wp_m5_shared_integration.md](../../workstreams/wp_m5_shared_integration.md) | Changed |
| [docs/active_plans/workstreams/wp_rc8_onboarding_preflight_evidence.md](../../workstreams/wp_rc8_onboarding_preflight_evidence.md) | Changed |
| [docs/active_plans/workstreams/wp_s1_student_repeat.md](../../workstreams/wp_s1_student_repeat.md) | Changed |
| [docs/active_plans/workstreams/wp_v1_deterministic_decisions.md](../../workstreams/wp_v1_deterministic_decisions.md) | Changed |
| [docs/active_plans/workstreams/wp_v2_visible_outcome_report.md](../../workstreams/wp_v2_visible_outcome_report.md) | Changed |
| [docs/active_plans/workstreams/wp_v3_failure_triage.md](../../workstreams/wp_v3_failure_triage.md) | Changed |
| [docs/active_plans/workstreams/wp_w1_first_keyboard_journey.md](../../workstreams/wp_w1_first_keyboard_journey.md) | Changed |
| [docs/active_plans/workstreams/wp_w2_retry_until_correct.md](../../workstreams/wp_w2_retry_until_correct.md) | Changed |
| [docs/active_plans/workstreams/wp_w3_leave_return.md](../../workstreams/wp_w3_leave_return.md) | Changed |
| [docs/active_plans/workstreams/wp_w4_instructor_gradebook.md](../../workstreams/wp_w4_instructor_gradebook.md) | Changed |
| [docs/active_plans/workstreams/wp_w6_exam_copy_fix.md](../../workstreams/wp_w6_exam_copy_fix.md) | Changed |
| [docs/active_plans/workstreams/wp_w6_policy_contrast.md](../../workstreams/wp_w6_policy_contrast.md) | Changed |
| [docs/active_plans/wp_inst_s4_tomorrow_start_checklist.md](../../wp_inst_s4_tomorrow_start_checklist.md) | Reviewed - no change needed |
| [docs/archive/account_creation_security_hardening.md](../../../archive/account_creation_security_hardening.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/blueprint_revision_only_lifecycle_plan.md](../../../archive/blueprint_revision_only_lifecycle_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/codebase_and_interaction_review.md](../../../archive/codebase_and_interaction_review.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/cryptic_foraging_hennessy.md](../../../archive/cryptic_foraging_hennessy.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/customer-spec.md](../../../archive/customer-spec.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/database_baseline_and_revision_model_reset.md](../../../archive/database_baseline_and_revision_model_reset.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/delegated-percolating-steele.md](../../../archive/delegated-percolating-steele.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/durable_live_demo_screenshot_corpus.md](../../../archive/durable_live_demo_screenshot_corpus.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/grading_retry_drift_reset_plan.md](../../../archive/grading_retry_drift_reset_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/hashed-crafting-minsky.md](../../../archive/hashed-crafting-minsky.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/implementation_plan.md](../../../archive/implementation_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/implementation_status.md](../../../archive/implementation_status.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/instructor_safety_truthful_ui_plan.md](../../../archive/instructor_safety_truthful_ui_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/interface_cleanup_2026_09.md](../../../archive/interface_cleanup_2026_09.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/live_delivery_convergence_plan.md](../../../archive/live_delivery_convergence_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/local_stack_controller_implementation.md](../../../archive/local_stack_controller_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/plan-velvet-brewing-llama-terminology-updates-needed.md](../../../archive/plan-velvet-brewing-llama-terminology-updates-needed.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/plan-velvet-brewing-llama-updated.md](../../../archive/plan-velvet-brewing-llama-updated.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/plan-velvet-brewing-llama.md](../../../archive/plan-velvet-brewing-llama.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/ple_question_json_editor_hci_brief.md](../../../archive/ple_question_json_editor_hci_brief.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/ple_question_json_editor_implementation.md](../../../archive/ple_question_json_editor_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/ple_question_json_package_implementation.md](../../../archive/ple_question_json_package_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/production_release_readiness_2026_09_12.md](../../../archive/production_release_readiness_2026_09_12.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/production_release_readiness_phase_plan.md](../../../archive/production_release_readiness_phase_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/project_status_report_2026-08-10.md](../../../archive/project_status_report_2026-08-10.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_author_ui_implementation.md](../../../archive/qti_author_ui_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_blackboard_parser_implementation.md](../../../archive/qti_blackboard_parser_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_ordered_xml_foundation.md](../../../archive/qti_ordered_xml_foundation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_profile_contract_implementation.md](../../../archive/qti_profile_contract_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_profile_fixture_corpus_implementation.md](../../../archive/qti_profile_fixture_corpus_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_provenance_schema_implementation.md](../../../archive/qti_provenance_schema_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_server_routes_implementation.md](../../../archive/qti_server_routes_implementation.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/qti_shared_safety_extraction.md](../../../archive/qti_shared_safety_extraction.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/release_completion_plan.md](../../../archive/release_completion_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/restore_live_demo.md](../../../archive/restore_live_demo.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/reviewer_commments_2.md](../../../archive/reviewer_commments_2.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/reviewer_commments_3.md](../../../archive/reviewer_commments_3.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/revision_concerns.txt](../../../archive/revision_concerns.txt) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/ribbon_application_shell.md](../../../archive/ribbon_application_shell.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/ribbon_chrome_compaction.md](../../../archive/ribbon_chrome_compaction.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/student_activation_mailer.md](../../../archive/student_activation_mailer.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/terminology_boundary_review.md](../../../archive/terminology_boundary_review.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/user_thoughts.md](../../../archive/user_thoughts.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/vocabulary_final_audit_candidate_2026-09-04.md](../../../archive/vocabulary_final_audit_candidate_2026-09-04.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/wire_naming_contract_migration_plan.md](../../../archive/wire_naming_contract_migration_plan.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/archive/wp_rc8_docs_closeout_review.md](../../../archive/wp_rc8_docs_closeout_review.md) | Reviewed - no change needed; historical evidence remains subordinate to Human Guidance |
| [docs/how-to-reduce-impact-of-bot-traffic.md](../../../how-to-reduce-impact-of-bot-traffic.md) | Reviewed - no change needed |
| [docs/screenshots/current_capture_manifest.json](../../../screenshots/current_capture_manifest.json) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/current_capture_receipt.json](../../../screenshots/current_capture_receipt.json) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/assignment_creation_laptop.png](../../../screenshots/instructor/assignment_creation_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/assignment_release_draft_laptop.png](../../../screenshots/instructor/assignment_release_draft_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/assignment_release_preview_laptop.png](../../../screenshots/instructor/assignment_release_preview_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/assignment_release_released_laptop.png](../../../screenshots/instructor/assignment_release_released_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/assignments_due_soon_empty_laptop.png](../../../screenshots/instructor/assignments_due_soon_empty_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/blueprint_course_detail_laptop.png](../../../screenshots/instructor/blueprint_course_detail_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/blueprint_courses_laptop.png](../../../screenshots/instructor/blueprint_courses_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/blueprint_question_picker_laptop.png](../../../screenshots/instructor/blueprint_question_picker_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/course_assignment_workspace_laptop.png](../../../screenshots/instructor/course_assignment_workspace_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/course_list_laptop.png](../../../screenshots/instructor/course_list_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/course_roster_active_laptop.png](../../../screenshots/instructor/course_roster_active_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/course_roster_pending_invitation_laptop.png](../../../screenshots/instructor/course_roster_pending_invitation_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/draft_editor_saved_laptop.png](../../../screenshots/instructor/draft_editor_saved_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/gradebook_laptop.png](../../../screenshots/instructor/gradebook_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/profile_default_laptop.png](../../../screenshots/instructor/profile_default_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/publication_review_laptop.png](../../../screenshots/instructor/publication_review_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/published_question_detail_laptop.png](../../../screenshots/instructor/published_question_detail_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/published_question_result_laptop.png](../../../screenshots/instructor/published_question_result_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/question_drafts_laptop.png](../../../screenshots/instructor/question_drafts_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/question_library_filtered_laptop.png](../../../screenshots/instructor/question_library_filtered_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/instructor/question_library_laptop.png](../../../screenshots/instructor/question_library_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/public/session_renewal_laptop.png](../../../screenshots/public/session_renewal_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/public/sign_in_laptop.png](../../../screenshots/public/sign_in_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/public/sign_in_phone.png](../../../screenshots/public/sign_in_phone.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/assignment_attempt_resumed_tablet.png](../../../screenshots/student/assignment_attempt_resumed_tablet.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/assignment_attempt_saved_laptop.png](../../../screenshots/student/assignment_attempt_saved_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/assignment_attempt_submitted_phone.png](../../../screenshots/student/assignment_attempt_submitted_phone.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/assignment_attempt_summary_laptop.png](../../../screenshots/student/assignment_attempt_summary_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/assignment_overview_history_laptop.png](../../../screenshots/student/assignment_overview_history_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/assignment_overview_unanswered_laptop.png](../../../screenshots/student/assignment_overview_unanswered_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/assignment_overview_unanswered_tablet.png](../../../screenshots/student/assignment_overview_unanswered_tablet.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/authorization_denial_laptop.png](../../../screenshots/student/authorization_denial_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/authorization_denial_phone.png](../../../screenshots/student/authorization_denial_phone.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/course_completed_laptop.png](../../../screenshots/student/course_completed_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/course_in_progress_laptop.png](../../../screenshots/student/course_in_progress_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/course_list_laptop.png](../../../screenshots/student/course_list_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/course_list_phone.png](../../../screenshots/student/course_list_phone.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/course_not_started_laptop.png](../../../screenshots/student/course_not_started_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/course_not_started_phone.png](../../../screenshots/student/course_not_started_phone.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/invitation_accepted_laptop.png](../../../screenshots/student/invitation_accepted_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/invitation_detail_laptop.png](../../../screenshots/student/invitation_detail_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/student/invitation_index_laptop.png](../../../screenshots/student/invitation_index_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/sysadmin/course_list_laptop.png](../../../screenshots/sysadmin/course_list_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/sysadmin/instructor_account_created_laptop.png](../../../screenshots/sysadmin/instructor_account_created_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/sysadmin/instructor_account_deactivated_laptop.png](../../../screenshots/sysadmin/instructor_account_deactivated_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/sysadmin/instructor_account_validation_laptop.png](../../../screenshots/sysadmin/instructor_account_validation_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/sysadmin/instructor_accounts_initial_laptop.png](../../../screenshots/sysadmin/instructor_accounts_initial_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/sysadmin/scoped_support_entry_laptop.png](../../../screenshots/sysadmin/scoped_support_entry_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/screenshots/sysadmin/scoped_support_roster_laptop.png](../../../screenshots/sysadmin/scoped_support_roster_laptop.png) | Follow-up - generated implementation evidence needs a source-owned refresh |
| [docs/ux/COURSE_APPEARANCE_ACCESSIBILITY_AUDIT.md](../../../ux/COURSE_APPEARANCE_ACCESSIBILITY_AUDIT.md) | Changed |
| [docs/ux/FRONTEND_CAPABILITY_INTEGRATION.md](../../../ux/FRONTEND_CAPABILITY_INTEGRATION.md) | Changed |
| [docs/ux/RIBBON_DESTINATION_LEDGER.md](../../../ux/RIBBON_DESTINATION_LEDGER.md) | Changed notice; generated body needs source-owned refresh |
| [docs/ux/RIBBON_RETIREMENT_RESPONSIBILITY_INVENTORY.md](../../../ux/RIBBON_RETIREMENT_RESPONSIBILITY_INVENTORY.md) | Changed |
| [docs/ux/RIBBON_TASK_MODEL.md](../../../ux/RIBBON_TASK_MODEL.md) | Changed |
| [docs/ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md](../../../ux/STUDENT_KEYBOARD_ACCESSIBILITY_AUDIT.md) | Changed |
