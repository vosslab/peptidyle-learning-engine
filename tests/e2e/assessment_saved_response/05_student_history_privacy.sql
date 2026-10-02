BEGIN;
SET LOCAL ROLE ple_private_owner;
SELECT 'USSV0001' || ple_private.crockford_checksum_character('USSV0001') AS other_student_id \gset
SELECT 'USSV0002' || ple_private.crockford_checksum_character('USSV0002') AS nonmember_student_id \gset
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES
    (:'other_student_id', 'student', pg_catalog.transaction_timestamp()),
    (:'nonmember_student_id', 'student', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_api_owner;
SELECT 'CISV00001' || ple_private.crockford_checksum_character('CISV00001') AS other_course_id \gset
INSERT INTO ple_data.course_instance (
    course_instance_id, source_kind, course_short_name, course_long_name,
    content_discipline_id, tags, term_starts_on, term_ends_on, created_at
) VALUES (
    :'other_course_id', 'empty', 'SVR-2', 'Other Course',
    '73000000-0000-0000-0000-00000000cc01',
    ARRAY[]::text[], current_date, current_date + 1,
    pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, joined_at
) VALUES (
    '73000000-0000-0000-0000-000000000304',
    :'other_course_id', :'instructor_id', 'instructor', pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.student_record (
    student_record_id, course_instance_id, student_account_id, created_at
) VALUES
    (
        '73000000-0000-0000-0000-000000000202',
        :'course_id', :'other_student_id', pg_catalog.transaction_timestamp()
    ),
    (
        '73000000-0000-0000-0000-000000000203',
        :'other_course_id', :'student_id', pg_catalog.transaction_timestamp()
    );
INSERT INTO ple_data.course_membership (
    course_membership_id, course_instance_id, account_id, role, student_record_id, joined_at
) VALUES
    (
        '73000000-0000-0000-0000-000000000305',
        :'course_id', :'other_student_id', 'student',
        '73000000-0000-0000-0000-000000000202', pg_catalog.transaction_timestamp()
    ),
    (
        '73000000-0000-0000-0000-000000000306',
        :'other_course_id', :'student_id', 'student',
        '73000000-0000-0000-0000-000000000203', pg_catalog.transaction_timestamp()
    );

RESET ROLE;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT set_config('ple.ferpa_owner_id', :'student_id', true);
SELECT set_config('ple.ferpa_other_student_id', :'other_student_id', true);
SELECT set_config('ple.ferpa_nonmember_id', :'nonmember_student_id', true);
SELECT set_config('ple.ferpa_instructor_id', :'instructor_id', true);
SELECT set_config('ple.ferpa_course_id', :'course_id', true);
SELECT set_config('ple.ferpa_other_course_id', :'other_course_id', true);
SELECT set_config('ple.ferpa_assessment_id', :'assessment_id', true);
SELECT set_config('ple.ferpa_attempt_id', :'attempt_id', true);
DO $$
DECLARE
    owner_id text := current_setting('ple.ferpa_owner_id');
    course_id text := current_setting('ple.ferpa_course_id');
    other_course_id text := current_setting('ple.ferpa_other_course_id');
    assessment_id text := current_setting('ple.ferpa_assessment_id');
    attempt_id uuid := current_setting('ple.ferpa_attempt_id')::uuid;
    owned_record uuid := '73000000-0000-0000-0000-000000000201';
    other_course_record uuid := '73000000-0000-0000-0000-000000000203';
    owned_course text;
    owned_state text;
    saved_response jsonb;
    previous_attempts jsonb;
    denial_count integer := 0;
BEGIN
    SELECT history.course_instance_id, history.state
      INTO owned_course, owned_state
      FROM ple_api.read_student_assessment_attempt_history(attempt_id) AS history;
    IF NOT FOUND OR owned_course IS DISTINCT FROM course_id OR owned_state IS DISTINCT FROM 'submitted' THEN
        RAISE EXCEPTION 'owning Student could not read the submitted Attempt';
    END IF;
    SELECT response.student_response INTO saved_response
      FROM ple_api.read_student_assessment_attempt_saved_response(attempt_id, 1) AS response;
    IF saved_response::text NOT LIKE '%cholesterol%' THEN
        RAISE EXCEPTION 'owning Student could not read the saved response';
    END IF;
    SELECT access.previous_assessment_attempts INTO previous_attempts
      FROM ple_api.read_student_assessment_access(course_id, assessment_id) AS access;
    IF previous_attempts::text NOT LIKE '%' || attempt_id::text || '%' THEN
        RAISE EXCEPTION 'owning Student could not see the Attempt on the Course Assessment';
    END IF;
    IF NOT ple_api.current_session_account_is_course_member(course_id)
       OR NOT ple_api.current_session_account_owns_student_record(course_id, owned_record)
       OR NOT ple_api.current_session_account_owns_student_record(other_course_id, other_course_record)
       OR ple_api.current_session_account_owns_student_record(other_course_id, owned_record)
       OR ple_api.current_session_account_owns_student_record(course_id, other_course_record) THEN
        RAISE EXCEPTION 'Student ownership did not follow the exact Course record';
    END IF;

    BEGIN
        PERFORM * FROM ple_api.read_student_assessment_access(other_course_id, assessment_id);
        RAISE EXCEPTION 'the other Course opened this Assessment';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Assessment is unavailable' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    PERFORM set_config('ple.session_account_id', current_setting('ple.ferpa_other_student_id'), true);
    IF NOT ple_api.current_session_account_is_course_member(course_id)
       OR ple_api.current_session_account_owns_student_record(course_id, owned_record) THEN
        RAISE EXCEPTION 'the other Student was not a Course member without this Student record';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assessment_attempt_history(attempt_id)
    ) THEN
        RAISE EXCEPTION 'another Student in the Course read the Attempt';
    END IF;
    BEGIN
        PERFORM * FROM ple_api.read_student_assessment_attempt_saved_response(attempt_id, 1);
        RAISE EXCEPTION 'another Student in the Course read the saved response';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Assessment Attempt is unavailable' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;
    SELECT access.previous_assessment_attempts INTO previous_attempts
      FROM ple_api.read_student_assessment_access(course_id, assessment_id) AS access;
    IF NOT FOUND OR previous_attempts::text LIKE '%' || attempt_id::text || '%' THEN
        RAISE EXCEPTION 'another Student in the Course saw this Student Attempt';
    END IF;

    PERFORM set_config('ple.session_account_id', current_setting('ple.ferpa_nonmember_id'), true);
    IF ple_api.current_session_account_is_course_member(course_id)
       OR ple_api.current_session_account_owns_student_record(course_id, owned_record) THEN
        RAISE EXCEPTION 'a non-member was treated as a Course Student';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assessment_attempt_history(attempt_id)
    ) THEN
        RAISE EXCEPTION 'a non-member read the Attempt';
    END IF;
    BEGIN
        PERFORM * FROM ple_api.read_student_assessment_attempt_saved_response(attempt_id, 1);
        RAISE EXCEPTION 'a non-member read the saved response';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Assessment Attempt is unavailable' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;
    BEGIN
        PERFORM * FROM ple_api.read_student_assessment_access(course_id, assessment_id);
        RAISE EXCEPTION 'a non-member read the Course Assessment';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Assessment is unavailable' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    PERFORM set_config('ple.session_account_id', current_setting('ple.ferpa_instructor_id'), true);
    IF NOT ple_api.current_session_account_is_course_member(course_id)
       OR ple_api.current_session_account_owns_student_record(course_id, owned_record) THEN
        RAISE EXCEPTION 'the Instructor was not a Course member without the Student record';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_api.read_student_assessment_attempt_history(attempt_id)
    ) THEN
        RAISE EXCEPTION 'the Course Instructor read the Student Attempt';
    END IF;
    BEGIN
        PERFORM * FROM ple_api.read_student_assessment_attempt_saved_response(attempt_id, 1);
        RAISE EXCEPTION 'the Course Instructor read the saved response';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Assessment Attempt is unavailable' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;
    BEGIN
        PERFORM * FROM ple_api.read_student_assessment_access(course_id, assessment_id);
        RAISE EXCEPTION 'the Course Instructor read the Student Assessment';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Assessment is unavailable' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    IF denial_count <> 6 THEN
        RAISE EXCEPTION 'FERPA denial count %', denial_count;
    END IF;
    IF owner_id IS NULL THEN
        RAISE EXCEPTION 'owning Student was missing';
    END IF;
    RAISE NOTICE 'ferpa_access_follows_course_membership_and_student_ownership';
END $$;
COMMIT;
