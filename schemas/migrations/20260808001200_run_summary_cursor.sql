-- MOD-RUN summary outcome keyset. The route orders exactly by this tuple;
-- the index prevents a large completed run from degrading into OFFSET scans.
CREATE INDEX question_attempt_run_summary_cursor_idx
    ON question_attempt (tenant_id, run_id, assignment_position, attempt_id);
