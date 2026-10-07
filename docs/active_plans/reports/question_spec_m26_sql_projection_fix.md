# M26 confirmed roster projection type correction

## Checkpoint: October 6, 2026

The confirmed Sysadmin roster reader declares `course_instance_id` and `student_account_id` as
plain `text`, while `ple_private.course_roster_profile` stores them as the
`ple_data.course_instance_id` and `ple_data.account_id` domains over `text`. PostgreSQL's
`RETURN QUERY` checks its result row descriptor against the declared `RETURNS TABLE` descriptor;
these two domain-valued expressions therefore need explicit `::text` casts.

The projection now casts those two expressions. Its other five expressions already match their
declared types: `roster_id` and `roster_name` are `text`; the status `CASE` resolves to `text`;
`recorded_event_id` is `uuid`; and the floored epoch expression is explicitly `bigint`. The casts
change only the public projection's SQL types. They leave confirmation, Sysadmin authorization,
exact-record selection, same-transaction audit recording, privacy, and Course membership behavior
unchanged.

This is a source-backed diagnosis of the failed `RETURN QUERY`, not live confirmation of the exact
PostgreSQL error. The exclusive fresh-database gate owner will verify the corrected function on a
fresh install. Runtime acceptance remains pending until that gate passes.

## Verification

- The existing `database_baseline_security_catalog.sql` fixture checks denied Instructor access,
  unconfirmed access, the complete confirmed record and audit receipt, unchanged Course
  membership, and application execute privilege.
- `source ./source_me.sh && schema_style/check_schema_style.py`: clean.
- `git diff --check -- schemas/base_schema/50_functions/course_operations.sql docs/active_plans/reports/question_spec_m26_sql_projection_fix.md`: passed.
- No database or live stack operation was performed in this lane.
