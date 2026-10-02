BEGIN;
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'student_id', true);
SELECT set_config('ple.identity_student_id', :'student_id', true);
SELECT set_config('ple.identity_instructor_id', :'instructor_id', true);
SELECT set_config('ple.identity_question_id', :'question_id', true);
SELECT set_config('ple.identity_pool_id', :'published_pool_id', true);
DO $$
DECLARE
    question_id text := current_setting('ple.identity_question_id');
    pool_id text := current_setting('ple.identity_pool_id');
    session_id text;
    sessions text[] := ARRAY[current_setting('ple.identity_student_id'), ''];
    denial_count integer := 0;
BEGIN
    FOREACH session_id IN ARRAY sessions LOOP
        PERFORM set_config('ple.session_account_id', session_id, true);
        BEGIN
            PERFORM * FROM ple_api.read_current_question_star(question_id);
            RAISE EXCEPTION 'a Student or anonymous session received the Question Star list';
        EXCEPTION WHEN insufficient_privilege THEN
            IF SQLERRM <> 'Question Star requires an active Instructor Account' THEN RAISE; END IF;
            denial_count := denial_count + 1;
        END;
        BEGIN
            PERFORM * FROM ple_api.read_current_question_pool_star(pool_id);
            RAISE EXCEPTION 'a Student or anonymous session received the Question Pool Star list';
        EXCEPTION WHEN insufficient_privilege THEN
            IF SQLERRM <> 'Question Pool Star requires an active Instructor Account' THEN RAISE; END IF;
            denial_count := denial_count + 1;
        END;
        BEGIN
            PERFORM * FROM ple_api.read_current_question_watch(question_id);
            RAISE EXCEPTION 'a Student or anonymous session received Question Watch information';
        EXCEPTION WHEN insufficient_privilege THEN
            IF SQLERRM <> 'Question Watch requires an active Instructor Account' THEN RAISE; END IF;
            denial_count := denial_count + 1;
        END;
        BEGIN
            PERFORM * FROM ple_api.read_current_question_pool_watch(pool_id);
            RAISE EXCEPTION 'a Student or anonymous session received Question Pool Watch information';
        EXCEPTION WHEN insufficient_privilege THEN
            IF SQLERRM <> 'Question Pool Watch requires an active Instructor Account' THEN RAISE; END IF;
            denial_count := denial_count + 1;
        END;
        BEGIN
            PERFORM * FROM ple_api.read_current_library_watch_notifications(10);
            RAISE EXCEPTION 'a Student or anonymous session received Watch notifications';
        EXCEPTION WHEN insufficient_privilege THEN
            IF SQLERRM <> 'Library Watch inbox requires an active Instructor Account' THEN RAISE; END IF;
            denial_count := denial_count + 1;
        END;
    END LOOP;
    IF denial_count <> 10 THEN
        RAISE EXCEPTION 'identity disclosure denial count %', denial_count;
    END IF;

    PERFORM set_config('ple.session_account_id', current_setting('ple.identity_instructor_id'), true);
    BEGIN
        PERFORM * FROM ple_api.read_current_question_star(question_id);
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE EXCEPTION 'Instructor was refused the Question Star list';
    END;
    BEGIN
        PERFORM * FROM ple_api.read_current_question_watch(question_id);
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE EXCEPTION 'Instructor was refused Question Watch';
    END;
    BEGIN
        PERFORM * FROM ple_api.read_current_library_watch_notifications(10);
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE EXCEPTION 'Instructor was refused the Watch inbox';
    END;
    RAISE NOTICE 'students_and_anonymous_users_do_not_receive_instructor_identity_lists_or_watch_information';
END $$;
COMMIT;
