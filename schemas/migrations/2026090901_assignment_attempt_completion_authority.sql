-- Keep Assignment Attempt completion under the ple_private schema's Database
-- Schema Owner Role, ple_private_owner, regardless of which authorized grading
-- writer inserts a Grading Result.

SET LOCAL ROLE ple_private_owner;

-- ASVS 2.3.3, 8.2.1, and 8.3.1: the trigger owns this invariant update inside
-- the grading transaction. Its fixed search path and private owner avoid
-- widening API or worker table privileges.
ALTER FUNCTION ple_private.complete_assignment_attempt_after_grading()
    SECURITY DEFINER
    SET search_path = pg_catalog, ple_data, ple_private;

REVOKE ALL PRIVILEGES ON FUNCTION
    ple_private.complete_assignment_attempt_after_grading() FROM PUBLIC;

COMMENT ON FUNCTION ple_private.complete_assignment_attempt_after_grading() IS
    'ple_private_owner trigger that applies the released Assignment Completion Rule after an authorized Grading Result insert.';

RESET ROLE;
