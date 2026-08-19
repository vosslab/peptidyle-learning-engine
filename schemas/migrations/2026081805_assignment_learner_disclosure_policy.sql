-- WP-PROF-S4: assignment-owned learner disclosure policy.
--
-- This is a pre-production direct cutover.  Disclosure is current assignment
-- policy; feedback_release remains immutable audit evidence and is not an
-- unlock authority.  New application writes must choose every timing value.

BEGIN;

ALTER TABLE public.assignment
    ADD COLUMN score_disclosure text NOT NULL DEFAULT 'after_submit'
        CONSTRAINT assignment_score_disclosure_check
        CHECK (score_disclosure IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )),
    ADD COLUMN per_item_correctness_disclosure text NOT NULL DEFAULT 'after_submit'
        CONSTRAINT assignment_per_item_correctness_disclosure_check
        CHECK (per_item_correctness_disclosure IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )),
    ADD COLUMN feedback_text_disclosure text NOT NULL DEFAULT 'after_submit'
        CONSTRAINT assignment_feedback_text_disclosure_check
        CHECK (feedback_text_disclosure IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )),
    ADD COLUMN solution_disclosure text NOT NULL DEFAULT 'after_submit'
        CONSTRAINT assignment_solution_disclosure_check
        CHECK (solution_disclosure IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        )),
    ADD COLUMN class_statistics_disclosure text NOT NULL DEFAULT 'never'
        CONSTRAINT assignment_class_statistics_disclosure_check
        CHECK (class_statistics_disclosure IN (
            'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
        ));

ALTER TABLE public.assignment
    ALTER COLUMN score_disclosure DROP DEFAULT,
    ALTER COLUMN per_item_correctness_disclosure DROP DEFAULT,
    ALTER COLUMN feedback_text_disclosure DROP DEFAULT,
    ALTER COLUMN solution_disclosure DROP DEFAULT,
    ALTER COLUMN class_statistics_disclosure DROP DEFAULT;

-- The S4 columns are the one current disclosure authority.  These legacy
-- snapshots carried the retired coarse policy and must not survive as a
-- competing reader/writer surface.  Dropping their dependent CHECK constraints
-- is PostgreSQL's normal ALTER TABLE dependency behavior.
ALTER TABLE public.assignment
    DROP COLUMN feedback_disclosure;

ALTER TABLE public.question_attempt
    DROP COLUMN issued_feedback_disclosure;

ALTER TABLE public.submission_receipt_snapshot
    DROP COLUMN feedback_disclosure;

COMMIT;
