-- question_assets tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Exact Question Revision asset publication.  A publisher claims only an
-- already-bound job and can make one Pending-to-Ready transition.
CREATE TABLE ple_data.question_asset_delivery (
    object_delivery_id uuid PRIMARY KEY,
    object_record_id uuid NOT NULL,
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    asset_id uuid NOT NULL,
    FOREIGN KEY (object_delivery_id, object_record_id) REFERENCES ple_data.object_delivery (object_delivery_id, object_record_id),
    FOREIGN KEY (published_question_id, revision_number) REFERENCES ple_data.question_revision (published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.question_asset_publication (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    asset_id uuid NOT NULL,
    source_object_record_id uuid NOT NULL REFERENCES ple_private.object_record,
    source_object_checksum bytea NOT NULL CHECK (octet_length(source_object_checksum) = 32),
    public_object_id uuid NOT NULL UNIQUE,
    public_object_checksum bytea NOT NULL CHECK (octet_length(public_object_checksum) = 32),
    public_byte_length bigint NOT NULL CHECK (public_byte_length >= 0),
    verified_media_type ple_data.media_type NOT NULL,
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    object_delivery_id uuid NOT NULL UNIQUE,
    -- The late integration layer binds this UUID to the sole
    -- publish_public_assets/public_asset_publication Job after jobs.sql.
    job_id uuid NOT NULL UNIQUE,
    publication_state ple_data.publication_state NOT NULL,
    PRIMARY KEY (published_question_id, revision_number, asset_id),
    FOREIGN KEY (published_question_id, revision_number) REFERENCES ple_data.question_revision,
    FOREIGN KEY (object_delivery_id, public_object_id) REFERENCES ple_data.object_delivery (object_delivery_id, object_record_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_asset_delivery IS 'role: current state, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_asset_publication IS 'role: event, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_asset_delivery IS 'role: current state, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_asset_publication IS 'role: event, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_asset_delivery IS 'role: current state, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_asset_publication IS 'role: event, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.question_asset_delivery IS 'role: current state, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.question_asset_publication IS 'role: event, deleted by Unrelease and object cleanup. HUMAN_GUIDANCE.md Question assets.';

