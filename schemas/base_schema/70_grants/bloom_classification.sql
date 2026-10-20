-- The API process owns automatic classification; these functions have no HTTP route.
SET LOCAL ROLE ple_private_owner;
REVOKE ALL ON FUNCTION ple_private.bloom_classification_candidate(text, text, integer),
    ple_private.save_bloom_classification(text, text, integer, bigint, bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.bloom_classification_candidate(text, text, integer),
    ple_private.save_bloom_classification(text, text, integer, bigint, bigint, text, text) TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
REVOKE ALL ON FUNCTION ple_api.bloom_classification_candidate(text, text, integer),
    ple_api.save_bloom_classification(text, text, integer, bigint, bigint, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.bloom_classification_candidate(text, text, integer),
    ple_api.save_bloom_classification(text, text, integer, bigint, bigint, text, text) TO ple_app;
