-- Disposable acceptance driver for independently owned PostgreSQL behavior receipts.
-- psql variables from the first transaction remain available to later transactions.
\set ON_ERROR_STOP on
\ir assessment_saved_response/01_saved_response_lifecycle.sql
\ir assessment_saved_response/02_blueprint_visibility.sql
\ir assessment_saved_response/03_course_pool_forks.sql
\ir assessment_saved_response/04_blueprint_pool_forks.sql
\ir assessment_saved_response/05_student_history_privacy.sql
\ir assessment_saved_response/06_question_watch_access.sql
