-- A Question Source is exact immutable stored data, never an inline fallback.
--
-- Question Backend and backend_locator describe how the source is interpreted
-- or located. The Source Object Reference and Source Object Checksum identify
-- the authored or imported bytes themselves.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.question_source
    DROP CONSTRAINT question_source_stores_data_or_an_object,
    DROP CONSTRAINT question_source_object_reference_is_complete,
    DROP COLUMN source_data,
    DROP COLUMN source_checksum,
    ALTER COLUMN source_object_id SET NOT NULL,
    ALTER COLUMN source_object_checksum SET NOT NULL;

COMMENT ON COLUMN ple_private.question_source.source_object_id IS
    'Source Object Reference: immutable Object Record that identifies the exact Question Source bytes.';
COMMENT ON COLUMN ple_private.question_source.source_object_checksum IS
    'Source Object Checksum: SHA-256 verification value for the exact Question Source bytes.';
COMMENT ON COLUMN ple_private.question_source.backend IS
    'Question Backend: PLE, WeBWorK, QTI, or iMathAS interpretation of the Question Source.';
COMMENT ON COLUMN ple_private.question_source.backend_locator IS
    'Question Backend locator details, such as a WeBWorK PG Path or iMathAS Item Reference.';

RESET ROLE;
