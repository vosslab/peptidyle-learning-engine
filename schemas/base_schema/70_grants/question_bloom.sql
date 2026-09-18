-- Privileges from question_bloom.sql.

SET LOCAL ROLE ple_data_owner;

GRANT USAGE ON TYPE ple_data.bloom_cognitive_process,
    ple_data.bloom_knowledge_dimension TO ple_private_owner, ple_api_owner;

REVOKE ALL ON TABLE ple_data.question_revision_bloom FROM PUBLIC;

GRANT SELECT, INSERT ON TABLE ple_data.question_revision_bloom TO ple_private_owner;

GRANT UPDATE (cognitive_process, knowledge_dimension, classification_edit_number)
    ON TABLE ple_data.question_revision_bloom TO ple_private_owner;

REVOKE ALL ON TABLE ple_data.question_pool_bloom FROM PUBLIC;

GRANT SELECT, INSERT ON TABLE ple_data.question_pool_bloom TO ple_private_owner;

GRANT UPDATE (cognitive_process, knowledge_dimension, classification_edit_number)
    ON TABLE ple_data.question_pool_bloom TO ple_private_owner;

GRANT SELECT (
    question_pool_id, cognitive_process,
    knowledge_dimension, classification_edit_number
)
    ON TABLE ple_data.question_pool_bloom TO ple_api_owner;

SET LOCAL ROLE ple_private_owner;

GRANT USAGE ON TYPE ple_private.bloom_preparation_target_kind
    TO ple_data_owner, ple_api_owner;

REVOKE ALL ON TABLE ple_private.bloom_preparation_receipt FROM PUBLIC;

GRANT SELECT, INSERT, DELETE ON TABLE ple_private.bloom_preparation_receipt TO ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.validate_bloom_pair(text, text) FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.question_revision_bloom_candidate_fingerprint(text),
    ple_private.question_pool_bloom_candidate_fingerprint(text, text, text[], integer[]) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.question_revision_bloom_candidate_fingerprint(text),
    ple_private.question_pool_bloom_candidate_fingerprint(text, text, text[], integer[])
    TO ple_data_owner, ple_api_owner;

REVOKE ALL ON FUNCTION ple_private.prepare_bloom_classification(
    uuid, ple_private.bloom_preparation_target_kind, bytea, text, text),
    ple_private.consume_bloom_classification(
        uuid, ple_private.bloom_preparation_target_kind, bytea),
    ple_private.attach_question_revision_bloom(uuid, text, integer, text),
    ple_private.attach_question_pool_bloom(uuid, text, text, text, text[], integer[])
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.prepare_bloom_classification(
    uuid, ple_private.bloom_preparation_target_kind, bytea, text, text)
    TO ple_data_owner, ple_api_owner;

GRANT EXECUTE ON FUNCTION ple_private.consume_bloom_classification(
    uuid, ple_private.bloom_preparation_target_kind, bytea)
    TO ple_data_owner, ple_private_owner;

GRANT EXECUTE ON FUNCTION ple_private.attach_question_revision_bloom(uuid, text, integer, text),
    ple_private.attach_question_pool_bloom(uuid, text, text, text, text[], integer[])
    TO ple_data_owner, ple_private_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.prepare_question_revision_bloom_classification(
    uuid, text, text, text),
    ple_api.prepare_question_pool_bloom_classification(
        uuid, text, text, text[], integer[], text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.prepare_question_revision_bloom_classification(
    uuid, text, text, text),
    ple_api.prepare_question_pool_bloom_classification(
        uuid, text, text, text[], integer[], text, text) TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.correct_question_revision_bloom(
    text, integer, bigint, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.correct_question_revision_bloom(
    text, integer, bigint, text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.correct_question_revision_bloom(
    text, integer, bigint, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.correct_question_revision_bloom(
    text, integer, bigint, text, text) TO ple_app;

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON FUNCTION ple_private.correct_question_pool_bloom(
    text, bigint, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.correct_question_pool_bloom(
    text, bigint, text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL ON FUNCTION ple_api.correct_question_pool_bloom(
    text, bigint, text, text) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.correct_question_pool_bloom(
    text, bigint, text, text) TO ple_app;
