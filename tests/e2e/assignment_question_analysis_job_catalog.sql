-- Assignment Question Analysis Job target oracle. The main catalog oracle
-- creates the Course Assignment records referenced here.

-- No other Job Kind may target a Course Assignment.
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_private.job (
            job_id, job_kind, job_target_kind, course_id, assignment_id, generation,
            payload, state, available_at, max_attempts, created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000002007',
            'recalculate_assignment', 'course_assignment',
            '00000000-0000-0000-0000-000000000105',
            '00000000-0000-0000-0000-000000000110', 1,
            '{}'::jsonb, 'ready', '2026-01-01 00:00:00+00', 1,
            '2026-01-01 00:00:00+00'
        );
        RAISE EXCEPTION 'a non-analysis Job Kind accepted a Course Assignment target';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;

-- Recalculation is a Course Assignment Job; Question Submission targets must
-- be rejected.
INSERT INTO ple_private.job (
    job_id, job_kind, job_target_kind, course_id, assignment_id, generation,
    payload, state, available_at, max_attempts, created_at
) VALUES (
    '00000000-0000-0000-0000-000000002004',
    'recalculate_assignment_question_analysis', 'course_assignment',
    '00000000-0000-0000-0000-000000000105',
    '00000000-0000-0000-0000-000000000110', 1,
    '{}'::jsonb, 'ready', '2026-01-01 00:00:00+00', 1,
    '2026-01-01 00:00:00+00'
);
DO $$
BEGIN
    BEGIN
        INSERT INTO ple_private.job (
            job_id, job_kind, job_target_kind, question_submission_id, generation,
            payload, state, available_at, max_attempts, created_at
        ) VALUES (
            '00000000-0000-0000-0000-000000002005',
            'recalculate_assignment_question_analysis', 'question_submission',
            '00000000-0000-0000-0000-000000002006', 1,
            '{}'::jsonb, 'ready', '2026-01-01 00:00:00+00', 1,
            '2026-01-01 00:00:00+00'
        );
        RAISE EXCEPTION 'Assignment Question Analysis recalculation accepted a Question Submission target';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END
$$;
