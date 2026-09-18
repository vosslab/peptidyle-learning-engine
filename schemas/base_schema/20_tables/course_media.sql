-- course_media tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.course_banner (
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    course_banner_id uuid NOT NULL,
    source_object_id uuid NOT NULL REFERENCES ple_private.object_record,
    source_object_checksum bytea NOT NULL CHECK (octet_length(source_object_checksum) = 32),
    source_byte_length bigint NOT NULL CHECK (source_byte_length BETWEEN 1 AND 8388608),
    source_media_type text NOT NULL CHECK (source_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    -- These are the server-inspected, orientation-corrected dimensions of the
    -- immutable source object.  A Course banner is deliberately exact 5:1;
    -- there is no arbitrary minimum beyond the independent byte-size limits.
    source_width integer NOT NULL CHECK (source_width > 0),
    source_height integer NOT NULL CHECK (source_height > 0),
    CHECK (source_width::bigint = source_height::bigint * 5),
    PRIMARY KEY (course_id, course_banner_id),
    UNIQUE (course_id, course_banner_id, source_object_id)
);

CREATE TABLE ple_data.course_banner_rendition (
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    rendition_kind text NOT NULL CHECK (rendition_kind = 'banner'),
    object_id uuid NOT NULL REFERENCES ple_private.object_record,
    rendition_width integer NOT NULL CHECK (rendition_width = 1280),
    rendition_height integer NOT NULL CHECK (rendition_height = 256),
    PRIMARY KEY (course_id, course_banner_id, rendition_kind),
    UNIQUE (course_id, course_banner_id, rendition_kind, object_id),
    FOREIGN KEY (course_id, course_banner_id) REFERENCES ple_data.course_banner
);

CREATE TABLE ple_data.course_banner_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    rendition_kind text NOT NULL CHECK (rendition_kind = 'banner'),
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id),
    FOREIGN KEY (course_id, course_banner_id, rendition_kind, object_id)
        REFERENCES ple_data.course_banner_rendition
            (course_id, course_banner_id, rendition_kind, object_id)
);

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.course_banner_upload (
    course_banner_upload_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    account_id uuid NOT NULL REFERENCES ple_private.account,
    object_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    canonical_media_type text NOT NULL CHECK (canonical_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    byte_length bigint NOT NULL CHECK (byte_length BETWEEN 1 AND 8388608),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    width integer NOT NULL CHECK (width > 0),
    height integer NOT NULL CHECK (height > 0),
    CHECK (width::bigint = height::bigint * 5),
    expires_at timestamptz NOT NULL,
    promoted_at timestamptz,
    created_at timestamptz NOT NULL,
    CHECK (expires_at > created_at)
);

CREATE TABLE ple_private.course_banner_storage_subject (
    course_banner_storage_subject_id uuid PRIMARY KEY,
    subject_kind text NOT NULL CHECK (subject_kind IN ('upload', 'source')),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    course_banner_upload_id uuid REFERENCES ple_private.course_banner_upload,
    course_banner_id uuid,
    object_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    expected_sha256 bytea NOT NULL CHECK (octet_length(expected_sha256) = 32),
    expected_size_bytes bigint NOT NULL CHECK (expected_size_bytes BETWEEN 1 AND 8388608),
    expected_media_type text NOT NULL CHECK (expected_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    storage_area text NOT NULL CHECK (storage_area IN ('temp-processing', 'private-content')),
    UNIQUE (course_banner_storage_subject_id, object_id),
    CHECK ((subject_kind = 'upload' AND course_banner_upload_id IS NOT NULL
            AND course_banner_id IS NULL AND storage_area = 'temp-processing')
        OR (subject_kind = 'source' AND course_banner_upload_id IS NULL
            AND course_banner_id IS NOT NULL AND storage_area = 'private-content')),
    FOREIGN KEY (course_id, course_banner_id) REFERENCES ple_data.course_banner
);

CREATE TABLE ple_private.course_banner_work (
    course_banner_work_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance,
    course_banner_id uuid,
    operation_kind text NOT NULL CHECK (operation_kind IN
        ('put-upload', 'put-source', 'put-rendition', 'delete-upload', 'delete-source', 'delete-rendition')),
    object_id uuid NOT NULL,
    state text NOT NULL CHECK (state IN ('pending', 'completed', 'repair-required')),
    course_banner_storage_subject_id uuid REFERENCES ple_private.course_banner_storage_subject,
    delivery_id uuid REFERENCES ple_data.object_delivery,
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    CHECK ((course_banner_storage_subject_id IS NULL) <> (delivery_id IS NULL)),
    FOREIGN KEY (course_banner_storage_subject_id, object_id)
        REFERENCES ple_private.course_banner_storage_subject (course_banner_storage_subject_id, object_id),
    FOREIGN KEY (delivery_id, object_id)
        REFERENCES ple_data.object_delivery (delivery_id, object_id)
);

CREATE TABLE ple_private.course_banner_prepared_presentation (
    course_id uuid NOT NULL,
    course_banner_id uuid NOT NULL,
    alternative_kind text NOT NULL CHECK (alternative_kind IN ('decorative', 'informative')),
    alternative_text text,
    PRIMARY KEY (course_id, course_banner_id),
    FOREIGN KEY (course_id, course_banner_id) REFERENCES ple_data.course_banner,
    CHECK ((alternative_kind = 'decorative' AND alternative_text IS NULL)
        OR (alternative_kind = 'informative' AND char_length(btrim(alternative_text)) BETWEEN 1 AND 160))
);

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.course_banner IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_rendition IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_delivery IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.course_banner_upload IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_storage_subject IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_work IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_prepared_presentation IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

