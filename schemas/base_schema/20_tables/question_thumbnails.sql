-- Question Thumbnail tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

-- One stored WebP preview names one exact immutable Published Question
-- Revision. Object Record remains the single authority for checksum, byte
-- count, media type, and creation time.
CREATE TABLE ple_private.question_revision_thumbnail (
    published_question_id ple_data.question_family_id NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    object_record_id uuid NOT NULL UNIQUE,
    intrinsic_width integer NOT NULL CHECK (intrinsic_width = 256),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height = 256),
    PRIMARY KEY (published_question_id, revision_number),
    -- Object Record owns when the bytes were created. This records only when
    -- this immutable revision-to-artifact binding was registered.
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

COMMENT ON TABLE ple_private.question_revision_thumbnail IS 'role: revision, immutable private WebP thumbnail for one exact Published Question Revision. HUMAN_GUIDANCE.md Question thumbnails.';
