-- Catalog oracle for immutable released Assignment Revision entry snapshots.

DO $$
BEGIN
    IF to_regclass('ple_data.assignment_revision_entry') IS NULL
       OR to_regclass('ple_data.assignment_revision_fixed_question') IS NULL
       OR to_regclass('ple_data.assignment_revision_question_pool') IS NULL
       OR to_regclass('ple_data.assignment_revision_question_pool_item') IS NULL THEN
        RAISE EXCEPTION 'released Assignment Revision entry snapshot tables are missing';
    END IF;
    IF EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE (table_schema, table_name, column_name) IN (
            ('ple_private', 'issued_question', 'issued_question_id'),
            ('ple_private', 'question_attempt', 'issued_question_id'),
            ('ple_audit', 'forced_question_correction_issued_question_target', 'issued_question_id')
         )
           AND data_type <> 'uuid'
    ) OR (SELECT count(*) FROM information_schema.columns
        WHERE (table_schema, table_name, column_name) IN (
            ('ple_private', 'issued_question', 'issued_question_id'),
            ('ple_private', 'question_attempt', 'issued_question_id'),
            ('ple_audit', 'forced_question_correction_issued_question_target', 'issued_question_id')
        ) AND data_type = 'uuid') <> 3 THEN
        RAISE EXCEPTION 'Issued Question identity must use UUID storage at every relational boundary';
    END IF;
    IF (SELECT count(*) FROM information_schema.columns
        WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision_entry'
        AND column_name IN (
            'assignment_revision_id', 'assignment_entry_id', 'assignment_content_entry_index',
            'entry_kind', 'availability', 'scoring_rule', 'point_value'
        ) AND is_nullable = 'NO') <> 7
       OR (SELECT count(*) FROM information_schema.columns
        WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision_entry'
        AND column_name IN (
            'question_attempt_limit', 'question_attempt_time_limit_seconds',
            'question_attempt_time_limit_grace_seconds'
        ) AND is_nullable = 'YES') <> 3 THEN
        RAISE EXCEPTION 'Assignment Revision Entry does not retain its exact released facts';
    END IF;
    IF (SELECT count(*) FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_entry'::regclass
          AND pg_get_constraintdef(oid) LIKE '%question_attempt_limit > 0%') <> 1
       OR (SELECT count(*) FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_entry'::regclass
          AND pg_get_constraintdef(oid) LIKE '%question_attempt_time_limit_seconds > 0%') <> 1
       OR (SELECT count(*) FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_entry'::regclass
          AND pg_get_constraintdef(oid) LIKE '%question_attempt_time_limit_grace_seconds >= 0%') <> 1
       OR (SELECT count(*) FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_entry'::regclass
          AND pg_get_constraintdef(oid) LIKE '%question_attempt_time_limit_seconds IS NULL%'
          AND pg_get_constraintdef(oid) LIKE '%question_attempt_time_limit_grace_seconds IS NULL%') <> 1 THEN
        RAISE EXCEPTION 'Assignment Revision Entry Question Attempt controls are not constrained';
    END IF;
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'ple_data'
          AND table_name = 'assignment_revision_question_pool_item'
          AND column_name = 'entry_index'
    ) OR (SELECT count(*) FROM information_schema.columns
        WHERE table_schema = 'ple_data' AND table_name = 'assignment_revision_question_pool_item'
        AND column_name IN (
            'assignment_revision_id', 'assignment_entry_id', 'question_pool_item_id',
            'question_pool_item_index', 'question_id', 'revision_number', 'availability'
        ) AND is_nullable = 'NO') <> 7 THEN
        RAISE EXCEPTION 'Question Pool Item snapshots do not retain exact Item facts';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_entry'::regclass
          AND conname = 'assignment_revision_entry_revision_matches'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_fixed_question'::regclass
          AND conname = 'assignment_revision_fixed_question_entry_matches'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_question_pool'::regclass
          AND conname = 'assignment_revision_question_pool_assignment_entry_matches'
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'ple_data.assignment_revision_question_pool_item'::regclass
          AND conname = 'assignment_revision_question_pool_item_pool_matches'
    ) THEN
        RAISE EXCEPTION 'released Assignment Revision entry snapshot relationships are incomplete';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'ple_data'
          AND relation.relname IN (
              'assignment_revision_entry', 'assignment_revision_fixed_question',
              'assignment_revision_question_pool', 'assignment_revision_question_pool_item'
          )
          AND (NOT relation.relrowsecurity OR NOT relation.relforcerowsecurity)
    ) THEN
        RAISE EXCEPTION 'released Assignment Revision entry snapshots require forced RLS';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        CROSS JOIN LATERAL aclexplode(COALESCE(relation.relacl, acldefault('r', relation.relowner))) AS privilege
        WHERE namespace.nspname = 'ple_data'
          AND relation.relname IN (
              'assignment_revision_entry', 'assignment_revision_fixed_question',
              'assignment_revision_question_pool', 'assignment_revision_question_pool_item'
          )
          AND privilege.grantee = 0
    ) THEN
        RAISE EXCEPTION 'PUBLIC retains a released Assignment Revision entry snapshot privilege';
    END IF;
    IF (SELECT count(*) FROM pg_trigger
        WHERE tgrelid IN (
            'ple_data.assignment_revision_entry'::regclass,
            'ple_data.assignment_revision_fixed_question'::regclass,
            'ple_data.assignment_revision_question_pool'::regclass,
            'ple_data.assignment_revision_question_pool_item'::regclass
        )
          AND tgname IN (
              'assignment_revision_entry_is_immutable',
              'assignment_revision_fixed_question_is_immutable',
              'assignment_revision_question_pool_is_immutable',
              'assignment_revision_question_pool_item_is_immutable',
              'assignment_revision_entry_has_exact_shape',
              'assignment_revision_fixed_question_matches_entry_kind',
              'assignment_revision_question_pool_matches_entry_kind',
              'assignment_revision_question_pool_item_count_is_sufficient'
          )
          AND NOT tgisinternal) <> 8 THEN
        RAISE EXCEPTION 'released Assignment Revision entry snapshot immutability or shape validation is missing';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgrelid = 'ple_private.question_pool_selection'::regclass
          AND tgname = 'question_pool_selection_matches_released_pool_assignment_entry'
          AND NOT tgisinternal
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgrelid = 'ple_private.question_pool_selected_item'::regclass
          AND tgname = 'question_pool_selected_item_matches_released_pool_item'
          AND NOT tgisinternal
    ) OR NOT EXISTS (
        SELECT 1 FROM pg_trigger
        WHERE tgrelid = 'ple_private.issued_question'::regclass
          AND tgname = 'issued_question_matches_released_assignment_entry'
          AND NOT tgisinternal
    ) THEN
        RAISE EXCEPTION 'Student Work does not validate its exact released Assignment Entry snapshot';
    END IF;
    IF to_regprocedure('ple_api.start_assignment_attempt(uuid,uuid,uuid,jsonb,jsonb)') IS NULL
       OR to_regprocedure('ple_private.start_assignment_attempt(uuid,uuid,uuid,jsonb,jsonb)') IS NULL
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.start_assignment_attempt(uuid,uuid,uuid,jsonb,jsonb)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'ple_api.start_assignment_attempt(uuid,uuid,uuid,jsonb,jsonb)',
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'authenticated Assignment Attempt start is not restricted to the application role';
    END IF;
END
$$;

-- Authorized start creates one exact fixed Question; a second start resumes it.
INSERT INTO ple_private.account (account_id, product_role, created_at) VALUES
    ('00000000-0000-0000-0000-000000000101', 'student', '2026-01-01 00:00:00+00'),
    ('00000000-0000-0000-0000-000000000102', 'instructor', '2026-01-01 00:00:00+00'),
    ('00000000-0000-0000-0000-000000000104', 'instructor', '2026-01-01 00:00:00+00'),
    ('00000000-0000-0000-0000-000000000204', 'instructor', '2026-01-01 00:00:00+00');
INSERT INTO ple_private.account_state_event (
    event_id, account_id, state, occurred_at
) VALUES (
    '00000000-0000-0000-0000-000000000114',
    '00000000-0000-0000-0000-000000000101',
    'active',
    pg_catalog.clock_timestamp()
);
INSERT INTO ple_data.published_question (question_id, created_at)
VALUES ('ABC-DEF0', '2026-01-01 00:00:00+00');
INSERT INTO ple_data.question_revision (
    question_id, revision_number, backend, published_at
) VALUES ('ABC-DEF0', 1, 'ple', '2026-01-01 00:00:00+00');
INSERT INTO ple_data.published_question_metadata (
    question_id, question_title, question_description, created_at, updated_at
) VALUES (
    'ABC-DEF0', 'Assignment Attempt fixture question',
    'Instructor-facing Assignment Attempt fixture question.',
    '2026-01-01 00:00:00+00', '2026-01-01 00:00:00+00'
);
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.published_question (question_id, created_at)
        VALUES ('ABC-DEF1', '2026-01-01 00:00:00+00');
        INSERT INTO ple_data.published_question_metadata (
            question_id, question_title, question_description, created_at, updated_at
        ) VALUES (
            'ABC-DEF1', 'Question with blank description', '   ',
            '2026-01-01 00:00:00+00', '2026-01-01 00:00:00+00'
        );
        RAISE EXCEPTION 'Published Question Metadata accepted a blank Question Description';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
INSERT INTO ple_data.blueprint_course (
    blueprint_id, reference_number, blueprint_course_owner_account_id, created_at
) OVERRIDING SYSTEM VALUE
VALUES (
    '00000000-0000-0000-0000-000000000103', 7,
    '00000000-0000-0000-0000-000000000102', '2026-01-01 00:00:00+00'
);
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.blueprint_course (
            blueprint_id, reference_number, blueprint_course_owner_account_id, created_at
        ) OVERRIDING SYSTEM VALUE
        VALUES (
            '00000000-0000-0000-0000-000000000198', 0,
            '00000000-0000-0000-0000-000000000102', '2026-01-01 00:00:00+00'
        );
        RAISE EXCEPTION 'Blueprint Course accepted an invalid Blueprint Course Reference number';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
INSERT INTO ple_data.blueprint_course_revision (
    blueprint_course_reference_number, blueprint_revision_number, title,
    blueprint_course_content, created_at
) VALUES
    (7, 1, 'Assignment Attempt fixture', '{}'::jsonb, '2026-01-01 00:00:00+00'),
    (7, 2, 'Draft collaboration fixture', '{}'::jsonb, '2026-01-01 00:00:00+00');
DO $$
DECLARE
    identity_columns text[];
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'ple_data'
           AND table_name = 'blueprint_course'
           AND column_name = 'blueprint_course_owner_account_id'
           AND is_nullable = 'NO'
    ) OR EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'ple_data'
           AND table_name = 'blueprint_course'
           AND column_name = 'owner_account_id'
    ) THEN
        RAISE EXCEPTION 'Blueprint Course does not retain its exact Blueprint Course Owner relationship';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_schema = 'ple_data'
           AND (
               column_name IN ('blueprint_revision_id', 'source_blueprint_revision_id')
               OR (
                   table_name = 'blueprint_course_revision'
                   AND column_name IN ('blueprint_id', 'revision')
               )
           )
    ) THEN
        RAISE EXCEPTION 'Blueprint Revision persistence retained a redundant UUID or Blueprint ID identity';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM (
              VALUES
                  ('blueprint_course', 'reference_number'),
                  ('blueprint_course_revision', 'blueprint_course_reference_number'),
                  ('blueprint_course_revision', 'blueprint_revision_number'),
                  ('course_instance', 'blueprint_course_reference_number'),
                  ('course_instance', 'blueprint_revision_number'),
                  ('course_origin', 'blueprint_course_reference_number'),
                  ('course_origin', 'blueprint_revision_number'),
                  ('assignment', 'source_blueprint_course_reference_number'),
                  ('assignment', 'source_blueprint_revision_number'),
                  ('blueprint_publication_event', 'blueprint_course_reference_number'),
                  ('blueprint_publication_event', 'blueprint_revision_number'),
                  ('blueprint_collaborator_event', 'blueprint_course_reference_number'),
                  ('blueprint_collaborator_event', 'blueprint_revision_number'),
                  ('blueprint_revision_availability_event', 'blueprint_course_reference_number'),
                  ('blueprint_revision_availability_event', 'blueprint_revision_number')
          ) AS expected(table_name, column_name)
         WHERE NOT EXISTS (
             SELECT 1
               FROM information_schema.columns AS actual
              WHERE actual.table_schema = 'ple_data'
                AND actual.table_name = expected.table_name
                AND actual.column_name = expected.column_name
         )
    ) THEN
        RAISE EXCEPTION 'Blueprint Revision Reference columns are incomplete';
    END IF;

    SELECT array_agg(attribute.attname ORDER BY key_column.ordinality)
      INTO identity_columns
      FROM pg_constraint AS constraint_record
      CROSS JOIN LATERAL unnest(constraint_record.conkey)
          WITH ORDINALITY AS key_column(attribute_number, ordinality)
      JOIN pg_attribute AS attribute
        ON attribute.attrelid = constraint_record.conrelid
       AND attribute.attnum = key_column.attribute_number
     WHERE constraint_record.conrelid = 'ple_data.blueprint_course_revision'::regclass
       AND constraint_record.contype = 'p';
    IF identity_columns IS DISTINCT FROM ARRAY[
        'blueprint_course_reference_number', 'blueprint_revision_number'
    ]::text[] THEN
        RAISE EXCEPTION 'Blueprint Revision primary key is not its exact Reference pair';
    END IF;
END
$$;
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.blueprint_publication_event (
            blueprint_publication_event_id, blueprint_course_reference_number,
            blueprint_revision_number, published_by_account_id, occurred_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000190', 7, 1,
            '00000000-0000-0000-0000-000000000104', '2026-01-02 00:00:00+00'
        );
        RAISE EXCEPTION 'another Instructor published as the Blueprint Course Owner';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
INSERT INTO ple_data.blueprint_publication_event (
    blueprint_publication_event_id, blueprint_course_reference_number,
    blueprint_revision_number, published_by_account_id, occurred_at
) VALUES (
    '00000000-0000-0000-0000-000000000191', 7, 1,
    '00000000-0000-0000-0000-000000000102', '2026-01-02 00:00:00+00'
);
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.blueprint_revision_availability_event (
            blueprint_revision_availability_event_id, blueprint_course_reference_number,
            blueprint_revision_number, recorded_by_account_id, event_kind, occurred_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000192', 7, 1,
            '00000000-0000-0000-0000-000000000104', 'available',
            '2026-01-03 00:00:00+00'
        );
        RAISE EXCEPTION 'another Instructor changed Blueprint Revision Availability as the owner';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
INSERT INTO ple_data.blueprint_revision_availability_event (
    blueprint_revision_availability_event_id, blueprint_course_reference_number,
    blueprint_revision_number, recorded_by_account_id, event_kind, occurred_at
) VALUES (
    '00000000-0000-0000-0000-000000000193', 7, 1,
    '00000000-0000-0000-0000-000000000102', 'available',
    '2026-01-03 00:00:00+00'
);
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.blueprint_collaborator_event (
            blueprint_collaborator_event_id, blueprint_course_reference_number,
            blueprint_revision_number, collaborator_account_id, recorded_by_account_id,
            event_kind, occurred_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000194', 7, 2,
            '00000000-0000-0000-0000-000000000204',
            '00000000-0000-0000-0000-000000000104', 'granted',
            '2026-01-03 00:00:00+00'
        );
        RAISE EXCEPTION 'another Instructor granted Blueprint collaboration as the owner';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
INSERT INTO ple_data.blueprint_collaborator_event (
    blueprint_collaborator_event_id, blueprint_course_reference_number,
    blueprint_revision_number, collaborator_account_id, recorded_by_account_id,
    event_kind, occurred_at
) VALUES (
    '00000000-0000-0000-0000-000000000195', 7, 2,
    '00000000-0000-0000-0000-000000000204',
    '00000000-0000-0000-0000-000000000102', 'granted',
    '2026-01-03 00:00:00+00'
);
BEGIN;
INSERT INTO ple_data.course_instance (
    course_id, blueprint_course_reference_number, blueprint_revision_number,
    assigned_instructor_account_id, assigned_instructor_role, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000105', 7, 1,
    '00000000-0000-0000-0000-000000000102',
    'instructor', '2026-01-01 00:00:00+00'
);
INSERT INTO ple_data.student_record (
    student_record_id, course_id, student_account_id, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000106', '00000000-0000-0000-0000-000000000105',
    '00000000-0000-0000-0000-000000000101', '2026-01-01 00:00:00+00'
);
INSERT INTO ple_data.course_membership (
    membership_id, course_id, account_id, role, joined_at, student_record_id
) VALUES
    (
        '00000000-0000-0000-0000-000000000107', '00000000-0000-0000-0000-000000000105',
        '00000000-0000-0000-0000-000000000102', 'instructor', '2026-01-01 00:00:00+00', NULL
    ),
    (
        '00000000-0000-0000-0000-000000000108', '00000000-0000-0000-0000-000000000105',
        '00000000-0000-0000-0000-000000000101', 'student', '2026-01-01 00:00:00+00',
        '00000000-0000-0000-0000-000000000106'
    );
COMMIT;
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_data.course_instance (
            course_id, blueprint_course_reference_number, blueprint_revision_number,
            assigned_instructor_account_id, assigned_instructor_role, created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000000199', 7, 3,
            '00000000-0000-0000-0000-000000000102',
            'instructor', '2026-01-01 00:00:00+00'
        );
        RAISE EXCEPTION 'Course Instance accepted a nonexistent Blueprint Revision Number';
    EXCEPTION WHEN foreign_key_violation THEN NULL;
    END;
END
$$;
INSERT INTO ple_data.course_schedule_revision (
    course_schedule_revision_id, course_id, revision_number, term_starts_on, term_ends_on,
    course_time_zone, created_at
) VALUES (
    '00000000-0000-0000-0000-000000000109', '00000000-0000-0000-0000-000000000105', 1,
    '2026-01-01', '2026-12-31', 'America/Chicago', '2026-01-01 00:00:00+00'
);
INSERT INTO ple_data.assignment (
    assignment_id, course_id, source_blueprint_course_reference_number,
    source_blueprint_revision_number, created_at, updated_at,
    assignment_edit_number, assignment_title, assignment_instructions,
    available_at, due_at, closes_at, assignment_attempt_time_limit_seconds, attempt_limit,
    late_work_rule, assignment_deadline_rule, assignment_completion_rule,
    assignment_completion_score_threshold, assignment_attempt_grade_rule,
    assignment_attempt_continuation_rule, max_additional_assignment_attempts,
    question_pool_reuse_rule, question_variation_rule, assignment_attempt_resume_rule,
    assignment_question_display_rule, assignment_navigation_rule,
    assignment_question_order_rule, assignment_status, released_assignment_revision_id
) VALUES (
    '00000000-0000-0000-0000-000000000110', '00000000-0000-0000-0000-000000000105',
    7, 1, '2026-01-01 00:00:00+00',
    '2026-01-01 00:00:00+00', 1, 'Assignment Attempt fixture', '',
    NULL, NULL, NULL, NULL, NULL, 'accept', 'auto_submit', 'answer_all', NULL,
    'highest', 'unlimited', NULL, 'reuse_selection', 'new_variation', 'resumable',
    'all_questions', 'free_navigation', 'authored_order', 'unreleased', NULL
);
INSERT INTO ple_data.assignment_revision (
    assignment_revision_id, assignment_id, course_id, course_schedule_revision_id, revision_number,
    assignment_title, assignment_instructions, available_at, due_at, closes_at,
    assignment_attempt_time_limit_seconds, attempt_limit, late_work_rule, assignment_deadline_rule,
    assignment_completion_rule, assignment_completion_score_threshold, assignment_attempt_grade_rule,
    assignment_attempt_continuation_rule, max_additional_assignment_attempts,
    question_pool_reuse_rule, question_variation_rule, assignment_attempt_resume_rule,
    assignment_question_display_rule, assignment_navigation_rule, assignment_question_order_rule,
    created_at
) VALUES (
    '00000000-0000-0000-0000-000000000111', '00000000-0000-0000-0000-000000000110',
    '00000000-0000-0000-0000-000000000105', '00000000-0000-0000-0000-000000000109', 1,
    'Assignment Attempt fixture', '', NULL, NULL, NULL, NULL, NULL, 'accept',
    'auto_submit', 'answer_all', NULL, 'highest', 'unlimited', NULL, 'reuse_selection',
    'new_variation', 'resumable', 'all_questions', 'free_navigation', 'authored_order',
    '2026-01-01 00:00:00+00'
);
UPDATE ple_data.assignment
   SET assignment_status = 'released',
       released_assignment_revision_id = '00000000-0000-0000-0000-000000000111'
 WHERE assignment_id = '00000000-0000-0000-0000-000000000110';
BEGIN;
INSERT INTO ple_data.assignment_revision_entry (
    assignment_revision_id, assignment_entry_id, assignment_content_entry_index, entry_kind, availability,
    scoring_rule, point_value, question_attempt_limit, question_attempt_time_limit_seconds,
    question_attempt_time_limit_grace_seconds
) VALUES (
    '00000000-0000-0000-0000-000000000111', '00000000-0000-0000-0000-000000000112', 0,
    'fixed_question', 'available', 'normal', 1, 2, 300, 0
);
INSERT INTO ple_data.assignment_revision_fixed_question (
    assignment_revision_id, assignment_entry_id, question_id, revision_number
) VALUES (
    '00000000-0000-0000-0000-000000000111', '00000000-0000-0000-0000-000000000112',
    'ABC-DEF0', 1
);
INSERT INTO ple_data.assignment_revision_entry (
    assignment_revision_id, assignment_entry_id, assignment_content_entry_index, entry_kind, availability,
    scoring_rule, point_value, question_attempt_limit, question_attempt_time_limit_seconds,
    question_attempt_time_limit_grace_seconds
) VALUES (
    '00000000-0000-0000-0000-000000000111', '00000000-0000-0000-0000-000000000117', 1,
    'question_pool', 'available', 'normal', 1, 3, 600, 30
);
INSERT INTO ple_data.assignment_revision_question_pool (
    assignment_revision_id, assignment_entry_id, selection_count, selected_question_order
) VALUES (
    '00000000-0000-0000-0000-000000000111', '00000000-0000-0000-0000-000000000117', 1,
    'question_pool_order'
);
INSERT INTO ple_data.assignment_revision_question_pool_item (
    assignment_revision_id, assignment_entry_id, question_pool_item_id, question_pool_item_index,
    question_id, revision_number, availability
) VALUES (
    '00000000-0000-0000-0000-000000000111', '00000000-0000-0000-0000-000000000117',
    '00000000-0000-0000-0000-000000000118', 0, 'ABC-DEF0', 1, 'available'
);
COMMIT;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.assignment_revision_entry
        WHERE assignment_revision_id = '00000000-0000-0000-0000-000000000111'
          AND assignment_entry_id = '00000000-0000-0000-0000-000000000112'
          AND entry_kind = 'fixed_question'
          AND question_attempt_limit = 2
          AND question_attempt_time_limit_seconds = 300
          AND question_attempt_time_limit_grace_seconds = 0
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_data.assignment_revision_entry
        WHERE assignment_revision_id = '00000000-0000-0000-0000-000000000111'
          AND assignment_entry_id = '00000000-0000-0000-0000-000000000117'
          AND entry_kind = 'question_pool'
          AND question_attempt_limit = 3
          AND question_attempt_time_limit_seconds = 600
          AND question_attempt_time_limit_grace_seconds = 30
    ) THEN
        RAISE EXCEPTION 'Released Assignment Revision Entry did not retain limited Question Attempt controls';
    END IF;
    BEGIN
        INSERT INTO ple_data.assignment_revision_entry (
            assignment_revision_id, assignment_entry_id, assignment_content_entry_index, entry_kind,
            availability, scoring_rule, point_value, question_attempt_limit,
            question_attempt_time_limit_seconds, question_attempt_time_limit_grace_seconds
        ) VALUES (
            '00000000-0000-0000-0000-000000000111', '00000000-0000-0000-0000-000000000119', 2,
            'fixed_question', 'available', 'normal', 1, 1, 60, NULL
        );
        RAISE EXCEPTION 'Assignment Revision Entry accepted an unpaired Question Attempt time/grace limit';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
INSERT INTO ple_private.authenticated_session (
    session_id, account_id, product_role, token_hash, created_at, expires_at, revoked_at
) VALUES (
    '00000000-0000-0000-0000-000000000113', '00000000-0000-0000-0000-000000000101',
    'student', decode(repeat('ab', 32), 'hex'), pg_catalog.clock_timestamp(),
    pg_catalog.clock_timestamp() + interval '1 hour', NULL
);

BEGIN;
SET ROLE ple_auth;
SELECT session_id FROM ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET ROLE ple_app;
DO $$
DECLARE
    first_attempt uuid;
    resumed_attempt uuid;
    first_number integer;
    resumed_number integer;
    first_resumed boolean;
    second_resumed boolean;
BEGIN
    SELECT assignment_attempt_id, attempt_number, resumed
      INTO first_attempt, first_number, first_resumed
      FROM ple_api.start_assignment_attempt(
          '00000000-0000-0000-0000-000000000114',
          '00000000-0000-0000-0000-000000000106',
          '00000000-0000-0000-0000-000000000110',
          jsonb_build_array(jsonb_build_object(
              'question_pool_selection_id', '00000000-0000-0000-0000-000000000120',
              'assignment_entry_id', '00000000-0000-0000-0000-000000000117',
              'reused_from_question_pool_selection_id', NULL,
              'selected_items', jsonb_build_array(jsonb_build_object(
                  'question_pool_item_id', '00000000-0000-0000-0000-000000000118',
                  'selection_position', 0,
                  'question_id', 'ABC-DEF0',
                  'revision_number', 1
              ))
          )),
          jsonb_build_array(
              jsonb_build_object(
                  'issued_question_id', '00000000-0000-5000-8000-000000000115',
                  'assignment_entry_id', '00000000-0000-0000-0000-000000000112',
                  'issued_position', 0,
                  'question_id', 'ABC-DEF0',
                  'revision_number', 1,
                  'question_pool_selection_id', NULL,
                  'question_pool_item_id', NULL
              ),
              jsonb_build_object(
                  'issued_question_id', '00000000-0000-5000-8000-000000000121',
                  'assignment_entry_id', '00000000-0000-0000-0000-000000000117',
                  'issued_position', 1,
                  'question_id', 'ABC-DEF0',
                  'revision_number', 1,
                  'question_pool_selection_id', '00000000-0000-0000-0000-000000000120',
                  'question_pool_item_id', '00000000-0000-0000-0000-000000000118'
              )
          )
      );
    SELECT assignment_attempt_id, attempt_number, resumed
      INTO resumed_attempt, resumed_number, second_resumed
      FROM ple_api.start_assignment_attempt(
          '00000000-0000-0000-0000-000000000116',
          '00000000-0000-0000-0000-000000000106',
          '00000000-0000-0000-0000-000000000110',
          '[]'::jsonb, '[]'::jsonb
      );
    IF first_attempt <> '00000000-0000-0000-0000-000000000114'
       OR first_number <> 1 OR first_resumed OR resumed_attempt <> first_attempt
       OR resumed_number <> 1 OR NOT second_resumed THEN
        RAISE EXCEPTION 'authenticated Assignment Attempt start/resume did not retain one exact Attempt';
    END IF;
END
$$;
SET ROLE ple_private_owner;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_private.issued_question
        WHERE assignment_attempt_id = '00000000-0000-0000-0000-000000000114'
          AND point_value = 1 AND scoring_rule = 'normal' AND question_statistics_eligibility
    ) THEN
        RAISE EXCEPTION 'Issued Question did not derive its released scoring facts';
    END IF;
END
$$;
COMMIT;
RESET ROLE;
