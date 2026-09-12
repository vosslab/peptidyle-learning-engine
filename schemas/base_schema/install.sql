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
\ir question_stewardship.sql
\ir object_records.sql
\ir question_authoring_state.sql
\ir question_authoring_operations.sql
\ir question_assets.sql
\ir blueprints.sql
\ir course_core.sql
\ir course_membership.sql
\ir course_roster.sql
\ir course_operations.sql
\ir course_media.sql
\ir profile_media.sql
\ir assignments.sql
\ir assignment_operations.sql
\ir attempts.sql
\ir attempt_interaction.sql
\ir attempt_presentation.sql
\ir attempt_operations.sql
\ir attempt_access.sql
\ir delivery_backends.sql
\ir delivery.sql
\ir jobs.sql
\ir question_asset_operations.sql
\ir grading.sql
\ir student_assignment_landing.sql
\ir attempt_history.sql
\ir statistics.sql
\ir corrections.sql
\ir cross_domain_constraints.sql
\ir unrelease.sql
\ir api_compatibility.sql
