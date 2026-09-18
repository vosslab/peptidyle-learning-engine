-- Row security policies from assessment_attempts.sql.

SET LOCAL ROLE ple_data_owner;



-- The Student Work table owner needs this one relational fact while inserting
-- an Assessment Attempt.  The data owner has this narrow RLS read only to evaluate the
-- invariant.  Neither private nor application roles receive a general Student
-- record read grant.
CREATE POLICY student_record_data_owner_assessment_scope_check ON ple_data.student_record
    FOR SELECT TO ple_data_owner USING (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.student_assessment_accommodation ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.student_assessment_accommodation FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.assessment_attempt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.assessment_attempt FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_pool_selection ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_pool_selection FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_pool_selected_item ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.question_pool_selected_item FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.issued_question ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.issued_question FORCE ROW LEVEL SECURITY;

CREATE POLICY student_assessment_accommodation_private_owner_access ON ple_private.student_assessment_accommodation FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY assessment_attempt_private_owner_access ON ple_private.assessment_attempt FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_selection_private_owner_access ON ple_private.question_pool_selection FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY question_pool_selected_item_private_owner_access ON ple_private.question_pool_selected_item FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY issued_question_private_owner_access ON ple_private.issued_question FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

