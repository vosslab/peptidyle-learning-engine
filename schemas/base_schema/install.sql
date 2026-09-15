-- Canonical PLE structural installation manifest.
--
-- The administrator invokes this file with PostgreSQL 17 `psql -X`,
-- `--set=ON_ERROR_STOP=1`, and `--single-transaction`.  Keep this manifest
-- limited to ordered includes: each module owns its current DDL directly.
\set ON_ERROR_STOP on

\ir foundation_roles.sql
\ir accounts.sql
\ir authentication.sql
\ir authorization.sql
\ir question_lineages.sql
\ir question_pools.sql
\ir question_stewardship.sql
\ir question_watch_notifications.sql
\ir object_records.sql
\ir question_authoring_state.sql
\ir question_publication_operations.sql
\ir question_library_operations.sql
\ir question_authoring_operations.sql
\ir published_question_metadata_operations.sql
\ir question_assets.sql
\ir blueprints.sql
\ir blueprint_operations.sql
\ir blueprint_lineage.sql
\ir blueprint_fork_sync.sql
\ir blueprint_stewardship.sql
\ir blueprint_revision_integrity.sql
\ir course_core.sql
\ir course_retention.sql
\ir course_membership.sql
\ir course_roster.sql
\ir support_repair_capability.sql
\ir course_operations.sql
\ir course_media.sql
\ir profile_media.sql
\ir assessments.sql
\ir assessment_pool_forks.sql
\ir assessment_pool_selection.sql
\ir course_blueprint_adoption.sql
\ir assessment_operations.sql
\ir assessment_student_view.sql
\ir assessment_attempts.sql
\ir assessment_attempt_interaction.sql
\ir assessment_attempt_presentation.sql
\ir assessment_attempt_operations.sql
\ir assessment_attempt_finalization.sql
\ir assessment_attempt_operations_api.sql
\ir assessment_attempt_access.sql
\ir delivery_backends.sql
\ir delivery.sql
\ir jobs.sql
\ir question_asset_operations.sql
\ir grading.sql
\ir grading_access.sql
\ir student_assessment_landing.sql
\ir assessment_attempt_history.sql
\ir statistics.sql
\ir course_retention_transitions.sql
\ir course_retention_notifications.sql
\ir corrections.sql
\ir cross_domain_constraints.sql
\ir unrelease.sql
\ir api_compatibility.sql
