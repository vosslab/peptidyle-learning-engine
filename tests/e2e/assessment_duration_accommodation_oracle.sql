-- Disposable proof that a Student time multiplier is applied after the stored
-- Assessment time limit and stops at 24 hours. The integers below are the
-- Human Guidance outcomes. The calculation is
-- ple_private.assessment_effective_duration_seconds.
\set ON_ERROR_STOP on

BEGIN;
SET CONSTRAINTS ALL DEFERRED;

SET LOCAL ROLE ple_data_owner;
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Thirty minute limit', '', NULL, NULL, NULL, 1800, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
    'regular_assignment'
), 'hex') AS override_snapshot_id \gset
SELECT encode(ple_private.ensure_assessment_policy_snapshot(
    'Fractional limit', '', NULL, NULL, NULL, 101, NULL,
    'accept', 'reuse_variation', 'authored_order',
    'after_submit', 'after_submit', 'after_submit', 'after_submit', 'after_submit',
    'regular_assignment'
), 'hex') AS fractional_snapshot_id \gset

INSERT INTO ple_data.content_discipline (content_discipline_id, name)
VALUES ('21000000-0000-0000-0000-00000000cc01', 'Duration fixture discipline');
SELECT 'DRT1-' || ple_private.crockford_checksum_character('DRT1XYZ') || 'XYZ' AS question_id \gset
INSERT INTO ple_data.published_question (published_question_id, created_at)
VALUES (:'question_id', pg_catalog.transaction_timestamp());
INSERT INTO ple_data.question_revision (
    published_question_id, revision_number, backend, question_type, published_at
) VALUES (:'question_id', 1, 'ple', 'multipleChoice', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.course_instance (
    course_instance_id, source_kind, course_short_name, course_long_name,
    content_discipline_id, tags, term_starts_on, term_ends_on, created_at
) VALUES (
    'CIDRT0000' || ple_private.crockford_checksum_character('CIDRT0000'),
    'empty', 'DRT-1', 'Duration accommodation Course',
    '21000000-0000-0000-0000-00000000cc01',
    ARRAY[]::text[], current_date, current_date + 1,
    pg_catalog.transaction_timestamp()
) RETURNING course_instance_id AS course_id \gset

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id
) VALUES (
    'ADRT1800' || ple_private.crockford_checksum_character('ADRT1800'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'override_snapshot_id', 'hex')
) RETURNING assessment_id AS override_assessment_id \gset
INSERT INTO ple_data.assessment (
    assessment_id, course_instance_id, origin_kind, created_at, updated_at,
    assessment_type, assessment_policy_snapshot_id
) VALUES (
    'ADRT0101' || ple_private.crockford_checksum_character('ADRT0101'),
    :'course_id', 'direct',
    pg_catalog.transaction_timestamp(), pg_catalog.transaction_timestamp(),
    'regular_assignment', decode(:'fractional_snapshot_id', 'hex')
) RETURNING assessment_id AS fractional_assessment_id \gset
INSERT INTO ple_data.assessment_entry (
    assessment_entry_id, assessment_id, authored_position, entry_kind, availability, scoring_rule
) VALUES
    ('41000000-0000-0000-0000-000000000011', :'override_assessment_id', 0, 'fixed_question', 'available', 'normal'),
    ('41000000-0000-0000-0000-000000000012', :'fractional_assessment_id', 0, 'fixed_question', 'available', 'normal');
INSERT INTO ple_data.assessment_entry_question (
    assessment_entry_id, assessment_id, published_question_id, question_revision_number, points_possible
) VALUES
    ('41000000-0000-0000-0000-000000000011', :'override_assessment_id', :'question_id', 1, 1),
    ('41000000-0000-0000-0000-000000000012', :'fractional_assessment_id', :'question_id', 1, 1);
SELECT set_config('ple.duration_override_assessment', :'override_assessment_id', true);
SELECT set_config('ple.duration_fractional_assessment', :'fractional_assessment_id', true);

SET LOCAL ROLE ple_api_owner;
DO $duration$
DECLARE
    override_id text := current_setting('ple.duration_override_assessment');
    fractional_id text := current_setting('ple.duration_fractional_assessment');
    standard_seconds integer;
    one_and_half_seconds integer;
    two_times_seconds integer;
    capped_seconds integer;
    fractional_seconds integer;
    fractional_cap_seconds integer;
BEGIN
    standard_seconds := ple_private.assessment_effective_duration_seconds(override_id, NULL);
    IF standard_seconds IS DISTINCT FROM 1800 THEN
        RAISE EXCEPTION 'standard time expected the 1800-second Assessment limit, got %', standard_seconds;
    END IF;
    one_and_half_seconds := ple_private.assessment_effective_duration_seconds(override_id, 1.5);
    IF one_and_half_seconds IS DISTINCT FROM 2700 THEN
        RAISE EXCEPTION '1.5X after 1800 seconds expected 2700, got %', one_and_half_seconds;
    END IF;
    two_times_seconds := ple_private.assessment_effective_duration_seconds(override_id, 2);
    IF two_times_seconds IS DISTINCT FROM 3600 THEN
        RAISE EXCEPTION '2X after 1800 seconds expected 3600, got %', two_times_seconds;
    END IF;
    capped_seconds := ple_private.assessment_effective_duration_seconds(override_id, 100);
    IF capped_seconds IS DISTINCT FROM 86400 THEN
        RAISE EXCEPTION '100X after 1800 seconds expected the 86400-second cap, got %', capped_seconds;
    END IF;
    fractional_seconds := ple_private.assessment_effective_duration_seconds(fractional_id, 1.5);
    IF fractional_seconds IS DISTINCT FROM 152 THEN
        RAISE EXCEPTION '1.5X after 101 seconds expected 152 rounded up, got %', fractional_seconds;
    END IF;
    fractional_cap_seconds := ple_private.assessment_effective_duration_seconds(fractional_id, 856);
    IF fractional_cap_seconds IS DISTINCT FROM 86400 THEN
        RAISE EXCEPTION '856X after 101 seconds expected the 86400-second cap, got %', fractional_cap_seconds;
    END IF;
    RAISE NOTICE 'assessment_effective_duration_seconds standard % one_and_half % two_times % capped % fractional % fractional_cap %',
        standard_seconds, one_and_half_seconds, two_times_seconds, capped_seconds,
        fractional_seconds, fractional_cap_seconds;
END
$duration$;
COMMIT;
