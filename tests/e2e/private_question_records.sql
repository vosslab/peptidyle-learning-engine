-- Private Question record terminology and integrity oracle for the fresh baseline.
DO $$
DECLARE
    table_name text;
    trigger_name text;
    expected_tables text[] := ARRAY[
        'draft_question_answer_key',
        'draft_question_feedback',
        'draft_question_answer_explanation',
        'draft_question_grading_input',
        'question_revision_answer_key',
        'question_revision_feedback',
        'question_revision_answer_explanation',
        'question_revision_grading_input',
        'workspace_import_grading_input'
    ];
BEGIN
    IF to_regclass('ple_private.draft_question_grading_material') IS NOT NULL
        OR to_regclass('ple_private.published_flat_question_grading') IS NOT NULL
        OR to_regclass('ple_private.published_qti_question_grading') IS NOT NULL
        OR to_regclass('ple_private.workspace_qti_import_grading') IS NOT NULL THEN
        RAISE EXCEPTION 'generic private Question grading records remain in the baseline';
    END IF;

    FOREACH table_name IN ARRAY expected_tables LOOP
        IF NOT EXISTS (
            SELECT 1
            FROM pg_class AS relation
            JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
            WHERE namespace.nspname = 'ple_private'
              AND relation.relname = table_name
              AND relation.relrowsecurity
              AND relation.relforcerowsecurity
        ) THEN
            RAISE EXCEPTION 'private Question record % does not enforce RLS', table_name;
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM pg_constraint AS table_constraint
            WHERE table_constraint.conrelid = format('ple_private.%I', table_name)::regclass
              AND pg_get_constraintdef(table_constraint.oid) LIKE '%sha256%'
        ) THEN
            RAISE EXCEPTION 'private Question record % lacks a checksum constraint', table_name;
        END IF;
    END LOOP;

    FOREACH table_name IN ARRAY ARRAY[
        'draft_question_answer_key',
        'draft_question_feedback',
        'draft_question_answer_explanation',
        'draft_question_grading_input'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_constraint AS table_constraint
            WHERE table_constraint.conrelid = format('ple_private.%I', table_name)::regclass
              AND table_constraint.contype = 'f'
              AND table_constraint.confrelid = 'ple_private.draft_question_revision'::regclass
        ) THEN
            RAISE EXCEPTION 'Draft Question record % is not revision-bound', table_name;
        END IF;
    END LOOP;

    FOREACH table_name IN ARRAY ARRAY[
        'question_revision_answer_key',
        'question_revision_feedback',
        'question_revision_answer_explanation',
        'question_revision_grading_input'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_constraint AS table_constraint
            WHERE table_constraint.conrelid = format('ple_private.%I', table_name)::regclass
              AND table_constraint.contype = 'f'
              AND table_constraint.confrelid = 'ple_data.published_question_version'::regclass
        ) THEN
            RAISE EXCEPTION 'Question Revision record % is not revision-bound', table_name;
        END IF;
    END LOOP;

    FOREACH trigger_name IN ARRAY ARRAY[
        'draft_question_answer_key_is_immutable',
        'draft_question_feedback_is_immutable',
        'draft_question_answer_explanation_is_immutable',
        'draft_question_grading_input_is_immutable',
        'question_revision_answer_key_is_immutable',
        'question_revision_feedback_is_immutable',
        'question_revision_answer_explanation_is_immutable',
        'question_revision_grading_input_is_immutable',
        'workspace_import_grading_input_is_immutable_after_commit'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_trigger
            WHERE tgname = trigger_name AND NOT tgisinternal
        ) THEN
            RAISE EXCEPTION 'private Question record immutability trigger % is missing', trigger_name;
        END IF;
    END LOOP;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint AS table_constraint
        WHERE table_constraint.conrelid = 'ple_private.workspace_import_grading_input'::regclass
          AND table_constraint.contype = 'f'
          AND table_constraint.confrelid = 'ple_private.workspace_qti_import'::regclass
    ) THEN
        RAISE EXCEPTION 'Workspace Import Question Grading Input is not import-bound';
    END IF;
END
$$;
