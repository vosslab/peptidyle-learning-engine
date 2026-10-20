-- Behavior checks on the disposable baseline; all fixtures roll back.
BEGIN;
SET CONSTRAINTS ALL DEFERRED;
SET LOCAL ROLE ple_private_owner;
INSERT INTO ple_private.account(account_id, user_role, created_at)
VALUES ('U00000009', 'instructor', transaction_timestamp())
RETURNING account_id AS bloom_owner \gset
SELECT 'B100-' || ple_private.crockford_checksum_character('B100ABC') || 'ABC' AS q1,
       'B200-' || ple_private.crockford_checksum_character('B200ABC') || 'ABC' AS q2,
       'B300-' || ple_private.crockford_checksum_character('B300ABC') || 'ABC' AS q3,
       'B400-' || ple_private.crockford_checksum_character('B400ABC') || 'ABC' AS pool_id \gset
SELECT set_config('ple.test_bloom_q1', :'q1', true), set_config('ple.test_bloom_pool', :'pool_id', true);

SET LOCAL ROLE ple_data_owner;
INSERT INTO ple_data.content_discipline(content_discipline_id, name)
VALUES ('b1000000-0000-0000-0000-000000000001', 'Bloom test discipline');
INSERT INTO ple_data.content_subject(content_subject_id, name)
VALUES ('b1000000-0000-0000-0000-000000000002', 'Bloom test subject');
INSERT INTO ple_data.content_subject_discipline(content_subject_id, content_discipline_id)
VALUES ('b1000000-0000-0000-0000-000000000002', 'b1000000-0000-0000-0000-000000000001');
INSERT INTO ple_data.published_question(published_question_id, created_at)
SELECT id, transaction_timestamp() FROM unnest(ARRAY[:'q1', :'q2', :'q3']) AS id;
INSERT INTO ple_data.question_revision(published_question_id, revision_number, backend, published_at)
SELECT id, 1, 'ple', transaction_timestamp() FROM unnest(ARRAY[:'q1', :'q2', :'q3']) AS id;
INSERT INTO ple_data.question_revision_metadata(published_question_id, revision_number,
    question_title, question_description, question_type, content_discipline_id, content_subject_id,
    created_at, updated_at)
SELECT id, 1, 'Bloom test', 'Bloom classification test question', 'multipleChoice',
    'b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000002',
    transaction_timestamp(), transaction_timestamp() FROM unnest(ARRAY[:'q1', :'q2', :'q3']) AS id;
INSERT INTO ple_data.question_revision_license(published_question_id, revision_number, spdx_expression)
SELECT id, 1, 'CC-BY-4.0' FROM unnest(ARRAY[:'q1', :'q2', :'q3']) AS id;
INSERT INTO ple_data.question_pool(question_pool_id, owner_account_id, question_pool_edit_number,
    question_type, backend, license, title, description, content_discipline_id, content_subject_id, created_at)
VALUES (:'pool_id', :'bloom_owner', 1, 'multipleChoice', 'ple', 'CC-BY-4.0', 'Bloom pool',
    'Bloom modal classification', 'b1000000-0000-0000-0000-000000000001',
    'b1000000-0000-0000-0000-000000000002', transaction_timestamp());
INSERT INTO ple_data.question_pool_member(question_pool_id, published_question_id, question_revision_number, created_at)
SELECT :'pool_id', id, 1, transaction_timestamp() FROM unnest(ARRAY[:'q1', :'q2', :'q3']) AS id;

SET LOCAL ROLE ple_app;
DO $$
DECLARE q text := current_setting('ple.test_bloom_q1');
BEGIN
    IF ple_api.save_bloom_classification('question', q, 1, 99, 0, 'Apply', 'Procedural Knowledge') THEN
        RAISE EXCEPTION 'stale Question metadata accepted';
    END IF;
    IF NOT ple_api.save_bloom_classification('question', q, 1, 1, 0, 'Apply', 'Procedural Knowledge') THEN
        RAISE EXCEPTION 'initial Question pair not saved';
    END IF;
    IF ple_api.save_bloom_classification('question', q, 1, 2, 0, 'Create', 'Factual Knowledge') THEN
        RAISE EXCEPTION 'assigned Question overwritten';
    END IF;
    BEGIN
        PERFORM ple_api.save_bloom_classification('question', q, 1, 2, 0, 'Apply', NULL);
        RAISE EXCEPTION 'incomplete JEV pair accepted';
    EXCEPTION WHEN invalid_parameter_value THEN NULL;
    END;
END
$$;

SET LOCAL ROLE ple_data_owner;
UPDATE ple_data.question_revision_metadata SET bloom_cognitive_process = 'Remember',
    bloom_knowledge_dimension = 'Conceptual Knowledge', metadata_edit_number = metadata_edit_number + 1
WHERE published_question_id = :'q2';
UPDATE ple_data.question_revision_metadata SET bloom_cognitive_process = 'Remember',
    metadata_edit_number = metadata_edit_number + 1 WHERE published_question_id = :'q3';
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_data.question_revision_metadata
        WHERE published_question_id = current_setting('ple.test_bloom_q1')
        AND bloom_cognitive_process = 'Apply' AND bloom_knowledge_dimension = 'Procedural Knowledge'
        AND metadata_edit_number = 2) THEN RAISE EXCEPTION 'Question pair or Edit Number differs'; END IF;
END
$$;

SET LOCAL ROLE ple_app;
DO $$
DECLARE p text := current_setting('ple.test_bloom_pool'); c jsonb;
BEGIN
    SELECT * INTO STRICT c FROM ple_api.bloom_classification_candidate('pool', p, 0);
    IF c->>'cognitive' <> 'Remember' OR c->>'knowledge' <> 'Conceptual Knowledge' THEN
        RAISE EXCEPTION 'independent modes, tie order, or NULL exclusion differs';
    END IF;
    IF NOT ple_api.save_bloom_classification('pool', p, 0,
        (c->>'edit_number')::bigint, (c->>'membership_edit_number')::bigint,
        c->>'cognitive', c->>'knowledge') THEN RAISE EXCEPTION 'Pool modes not saved'; END IF;
    IF ple_api.save_bloom_classification('pool', p, 0, 1, 1, 'Apply', 'Procedural Knowledge') THEN
        RAISE EXCEPTION 'late Pool result overwrote newer metadata';
    END IF;
END
$$;

SET LOCAL ROLE ple_data_owner;
DELETE FROM ple_data.question_pool_member WHERE question_pool_id = :'pool_id' AND published_question_id <> :'q1';
UPDATE ple_data.question_pool SET question_pool_edit_number = 2 WHERE question_pool_id = :'pool_id';
SET LOCAL ROLE ple_app;
DO $$
DECLARE p text := current_setting('ple.test_bloom_pool'); c jsonb;
BEGIN
    IF ple_api.save_bloom_classification('pool', p, 0, 2, 1, 'Remember', 'Factual Knowledge') THEN
        RAISE EXCEPTION 'stale membership accepted';
    END IF;
    SELECT * INTO STRICT c FROM ple_api.bloom_classification_candidate('pool', p, 0);
    IF c->>'cognitive' <> 'Apply' OR c->>'knowledge' <> 'Procedural Knowledge' THEN
        RAISE EXCEPTION 'membership change did not change modes';
    END IF;
    IF NOT ple_api.save_bloom_classification('pool', p, 0, 2, 2,
        c->>'cognitive', c->>'knowledge') THEN RAISE EXCEPTION 'edited Pool modes not saved'; END IF;
END
$$;
ROLLBACK;
