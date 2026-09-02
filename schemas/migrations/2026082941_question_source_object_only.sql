-- A Question Source is exact immutable stored data, never an inline fallback.
--
-- Question Backend and its exact backend-owned fields describe how the source
-- is interpreted. The Source Object Reference and Source Object Checksum identify
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
COMMENT ON COLUMN ple_private.question_source.webwork_pg_path IS
    'WeBWorK PG Path for a WeBWorK Question Backend.';
COMMENT ON COLUMN ple_private.question_source.qti_package_item_identifier IS
    'QTI package item identifier for a QTI Question Backend.';
COMMENT ON COLUMN ple_private.question_source.workspace_import_id IS
    'Workspace Import ID for a draft QTI Question Source only.';
COMMENT ON COLUMN ple_private.question_source.imathas_deployment_reference IS
    'iMathAS Deployment Reference for an iMathAS Question Backend.';
COMMENT ON COLUMN ple_private.question_source.imathas_item_reference IS
    'iMathAS Item Reference for an iMathAS Question Backend.';
COMMENT ON COLUMN ple_private.question_source.imathas_profile IS
    'Pinned iMathAS Profile for a published iMathAS Question Source only.';

RESET ROLE;
