-- Pool member Save is available to its owning Instructor and platform Sysadmins.
BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SELECT 'UVSV0004' || ple_private.crockford_checksum_character('UVSV0004')
    AS m09_other_instructor_id \gset
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES (:'m09_other_instructor_id', 'instructor', pg_catalog.transaction_timestamp());
INSERT INTO ple_private.instructor_profile (
    account_id, first_name, last_name, affiliation, created_at, updated_at
) VALUES (
    :'m09_other_instructor_id', 'Other', 'Instructor', 'Test University',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp()
);

RESET ROLE;
SELECT 'M09P-' || ple_private.crockford_checksum_character('M09PABC') || 'ABC'
    AS m09_created_pool_id \gset
SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'instructor_id', true);
SELECT question_pool_id AS m09_pool_id, question_pool_edit_number AS m09_created_pool_edit
  FROM ple_api.create_question_pool(
      :'m09_created_pool_id',
      ARRAY[:'question_id', :'replacement_question_id'], ARRAY[1, 1],
      'Pool Save permissions', 'Isolated Pool Save permissions oracle', ARRAY[]::text[]
  ) \gset
SET LOCAL ROLE ple_api_owner;
SELECT set_config('ple.m09_pool_id', :'m09_pool_id', true);
SELECT set_config('ple.m09_created_pool_edit', :'m09_created_pool_edit', true);
SELECT set_config('ple.m09_owner_id', :'instructor_id', true);
SELECT set_config('ple.m09_other_instructor_id', :'m09_other_instructor_id', true);
SELECT set_config('ple.m09_student_id', :'student_id', true);
SELECT set_config('ple.m09_sysadmin_id', :'sysadmin_id', true);
SELECT set_config('ple.m09_question_id', :'question_id', true);
SELECT set_config('ple.m09_replacement_question_id', :'replacement_question_id', true);

RESET ROLE;
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM set_config('ple.session_account_id', current_setting('ple.m09_owner_id'), true);
    PERFORM * FROM ple_api.save_question_pool_members(
        current_setting('ple.m09_pool_id'),
        current_setting('ple.m09_created_pool_edit')::bigint,
        ARRAY[current_setting('ple.m09_question_id'), current_setting('ple.m09_replacement_question_id')],
        ARRAY[1, 1]
    ) AS result;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'ple_app no-op Save returned no result row';
    END IF;
END $$;
RESET ROLE;
SET LOCAL ROLE ple_api_owner;

DO $$
DECLARE
    pool_id text := current_setting('ple.m09_pool_id');
    current_edit bigint;
    saved_edit bigint;
    actual_member_ids text[];
    actual_revision_numbers integer[];
    expected_member_tuples text[];
    actual_member_tuples text[];
BEGIN
    PERFORM set_config('ple.session_account_id', current_setting('ple.m09_owner_id'), true);
    SELECT pool.question_pool_edit_number INTO current_edit
      FROM ple_data.question_pool AS pool WHERE pool.question_pool_id = pool_id;
    SELECT array_agg(member.published_question_id::text || ':' || member.question_revision_number::text
                     ORDER BY member.published_question_id)
      INTO actual_member_tuples
      FROM ple_data.question_pool_member AS member
     WHERE member.question_pool_id = pool_id;
    SELECT array_agg(expected.question_id || ':' || expected.revision_number::text
                     ORDER BY expected.question_id)
      INTO expected_member_tuples
      FROM unnest(
          ARRAY[current_setting('ple.m09_question_id'),
                current_setting('ple.m09_replacement_question_id')],
          ARRAY[1, 1]
      ) AS expected(question_id, revision_number);
    IF actual_member_tuples IS DISTINCT FROM expected_member_tuples THEN
        RAISE EXCEPTION 'ple_app no-op Save changed the exact Pool member tuple set';
    END IF;

    SELECT result.question_pool_edit_number INTO saved_edit
      FROM ple_data.save_question_pool_members(
          pool_id, current_edit,
          ARRAY[current_setting('ple.m09_question_id')], ARRAY[1]
      ) AS result;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'owning Instructor Save returned no result row';
    END IF;
    IF saved_edit IS DISTINCT FROM current_edit + 1 THEN
        RAISE EXCEPTION 'owning Instructor Save did not advance the Pool Edit Number';
    END IF;

    PERFORM set_config('ple.session_account_id', current_setting('ple.m09_other_instructor_id'), true);
    BEGIN
        PERFORM * FROM ple_data.save_question_pool_members(
            pool_id, saved_edit,
            ARRAY[current_setting('ple.m09_replacement_question_id')], ARRAY[1]
        );
        RAISE EXCEPTION 'another Instructor saved a Pool they do not own';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
    SELECT pool.question_pool_edit_number,
           array_agg(member.published_question_id::text ORDER BY member.published_question_id),
           array_agg(member.question_revision_number ORDER BY member.published_question_id)
      INTO current_edit, actual_member_ids, actual_revision_numbers
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_member AS member USING (question_pool_id)
     WHERE pool.question_pool_id = pool_id
     GROUP BY pool.question_pool_edit_number;
    IF current_edit <> saved_edit
       OR actual_member_ids IS DISTINCT FROM ARRAY[current_setting('ple.m09_question_id')]
       OR actual_revision_numbers IS DISTINCT FROM ARRAY[1] THEN
        RAISE EXCEPTION 'another Instructor denial changed Pool members or Edit Number';
    END IF;

    PERFORM set_config('ple.session_account_id', current_setting('ple.m09_student_id'), true);
    BEGIN
        PERFORM * FROM ple_data.save_question_pool_members(
            pool_id, saved_edit,
            ARRAY[current_setting('ple.m09_replacement_question_id')], ARRAY[1]
        );
        RAISE EXCEPTION 'Student saved Question Pool members';
    EXCEPTION WHEN insufficient_privilege THEN
        NULL;
    END;
    SELECT pool.question_pool_edit_number,
           array_agg(member.published_question_id::text ORDER BY member.published_question_id),
           array_agg(member.question_revision_number ORDER BY member.published_question_id)
      INTO current_edit, actual_member_ids, actual_revision_numbers
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_member AS member USING (question_pool_id)
     WHERE pool.question_pool_id = pool_id
     GROUP BY pool.question_pool_edit_number;
    IF current_edit <> saved_edit
       OR actual_member_ids IS DISTINCT FROM ARRAY[current_setting('ple.m09_question_id')]
       OR actual_revision_numbers IS DISTINCT FROM ARRAY[1] THEN
        RAISE EXCEPTION 'Student denial changed Pool members or Edit Number';
    END IF;

    PERFORM set_config('ple.session_account_id', current_setting('ple.m09_sysadmin_id'), true);
    SELECT result.question_pool_edit_number INTO current_edit
      FROM ple_data.save_question_pool_members(
          pool_id, saved_edit,
          ARRAY[current_setting('ple.m09_question_id'), current_setting('ple.m09_replacement_question_id')],
          ARRAY[1, 1]
      ) AS result;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Sysadmin Save returned no result row';
    END IF;
    IF current_edit IS DISTINCT FROM saved_edit + 1 THEN
        RAISE EXCEPTION 'Sysadmin Save did not advance the Pool Edit Number';
    END IF;

    PERFORM set_config('ple.session_account_id', current_setting('ple.m09_owner_id'), true);
    BEGIN
        PERFORM * FROM ple_data.save_question_pool_members(
            pool_id, saved_edit,
            ARRAY[current_setting('ple.m09_replacement_question_id')], ARRAY[1]
        );
        RAISE EXCEPTION 'stale Pool Edit Number was accepted';
    EXCEPTION WHEN serialization_failure THEN
        NULL;
    END;
    SELECT pool.question_pool_edit_number,
           array_agg(member.published_question_id::text ORDER BY member.published_question_id),
           array_agg(member.question_revision_number ORDER BY member.published_question_id)
      INTO saved_edit, actual_member_ids, actual_revision_numbers
      FROM ple_data.question_pool AS pool
      JOIN ple_data.question_pool_member AS member USING (question_pool_id)
     WHERE pool.question_pool_id = pool_id
     GROUP BY pool.question_pool_edit_number;
    IF saved_edit <> current_edit
       OR actual_member_ids IS DISTINCT FROM ARRAY(
           SELECT member.question_id
             FROM unnest(ARRAY[
                 current_setting('ple.m09_question_id'), current_setting('ple.m09_replacement_question_id')
             ]) AS member(question_id)
            ORDER BY member.question_id
       )
       OR actual_revision_numbers IS DISTINCT FROM ARRAY[1, 1] THEN
        RAISE EXCEPTION 'stale Pool Edit Number rejection changed Pool members or Edit Number';
    END IF;

    RAISE NOTICE 'question_pool_member_save_enforces_owner_and_sysadmin_authority';
END $$;

COMMIT;
