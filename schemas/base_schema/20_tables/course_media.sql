-- course_media tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.course_banner (
    course_instance_id ple_data.course_instance_id NOT NULL REFERENCES ple_data.course_instance,
    course_banner_id uuid NOT NULL,
    source_object_record_id uuid NOT NULL REFERENCES ple_private.object_record,
    source_object_checksum bytea NOT NULL CHECK (octet_length(source_object_checksum) = 32),
    source_byte_length bigint NOT NULL CHECK (source_byte_length BETWEEN 1 AND 8388608),
    source_media_type ple_data.media_type NOT NULL,
    -- These are the server-inspected, orientation-corrected dimensions of the
    -- immutable source object.  A Course banner is deliberately exact 5:1;
    -- there is no arbitrary minimum beyond the independent byte-size limits.
    source_width integer NOT NULL CHECK (source_width > 0),
    source_height integer NOT NULL CHECK (source_height > 0),
    CHECK (source_width::bigint = source_height::bigint * 5),
    PRIMARY KEY (course_instance_id, course_banner_id),
    UNIQUE (course_instance_id, course_banner_id, source_object_record_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



CREATE TABLE ple_data.course_banner_rendition (
    course_instance_id ple_data.course_instance_id NOT NULL,
    course_banner_id uuid NOT NULL,
    rendition_kind text NOT NULL DEFAULT 'banner',
    object_record_id uuid NOT NULL REFERENCES ple_private.object_record,
    rendition_width integer NOT NULL CHECK (rendition_width = 1280),
    rendition_height integer NOT NULL CHECK (rendition_height = 256),
    PRIMARY KEY (course_instance_id, course_banner_id, rendition_kind),
    UNIQUE (course_instance_id, course_banner_id, rendition_kind, object_record_id),
    FOREIGN KEY (course_instance_id, course_banner_id) REFERENCES ple_data.course_banner,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_data.course_banner_delivery (
    object_delivery_id uuid PRIMARY KEY,
    object_record_id uuid NOT NULL,
    course_instance_id ple_data.course_instance_id NOT NULL,
    course_banner_id uuid NOT NULL,
    rendition_kind text NOT NULL DEFAULT 'banner',
    FOREIGN KEY (object_delivery_id, object_record_id) REFERENCES ple_data.object_delivery (object_delivery_id, object_record_id),
    FOREIGN KEY (course_instance_id, course_banner_id, rendition_kind, object_record_id)
        REFERENCES ple_data.course_banner_rendition
            (course_instance_id, course_banner_id, rendition_kind, object_record_id),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.course_banner_upload (
    course_banner_upload_reference uuid PRIMARY KEY,
    course_instance_id ple_data.course_instance_id NOT NULL REFERENCES ple_data.course_instance,
    account_id ple_data.account_id NOT NULL REFERENCES ple_private.account,
    object_record_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    canonical_media_type ple_data.media_type NOT NULL,
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
    subject_kind ple_data.banner_subject_kind NOT NULL,
    course_instance_id ple_data.course_instance_id NOT NULL REFERENCES ple_data.course_instance,
    course_banner_upload_reference uuid REFERENCES ple_private.course_banner_upload,
    course_banner_id uuid,
    object_record_id uuid NOT NULL UNIQUE REFERENCES ple_private.object_record,
    expected_sha256 bytea NOT NULL CHECK (octet_length(expected_sha256) = 32),
    expected_size_bytes bigint NOT NULL CHECK (expected_size_bytes BETWEEN 1 AND 8388608),
    expected_media_type ple_data.media_type NOT NULL,
    storage_area ple_data.object_storage_area NOT NULL,
    UNIQUE (course_banner_storage_subject_id, object_record_id),
    CHECK ((subject_kind = 'upload' AND course_banner_upload_reference IS NOT NULL
            AND course_banner_id IS NULL AND storage_area = 'temp-processing')
        OR (subject_kind = 'source' AND course_banner_upload_reference IS NULL
            AND course_banner_id IS NOT NULL AND storage_area = 'private-content')),
    FOREIGN KEY (course_instance_id, course_banner_id) REFERENCES ple_data.course_banner,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



CREATE TABLE ple_private.course_banner_work (
    course_banner_work_id uuid PRIMARY KEY,
    course_instance_id ple_data.course_instance_id NOT NULL REFERENCES ple_data.course_instance,
    course_banner_id uuid,
    operation_kind ple_data.banner_work_operation NOT NULL,
    object_record_id uuid NOT NULL,
    state ple_data.lease_state NOT NULL,
    lease_token uuid,
    lease_expires_at timestamptz,
    course_banner_storage_subject_id uuid REFERENCES ple_private.course_banner_storage_subject,
    object_delivery_id uuid REFERENCES ple_data.object_delivery,
    created_at timestamptz NOT NULL,
    completed_at timestamptz,
    CHECK (ple_private.work_lease_pair_is_valid(lease_token, lease_expires_at, created_at)),
    CHECK ((course_banner_storage_subject_id IS NULL) <> (object_delivery_id IS NULL)),
    FOREIGN KEY (course_banner_storage_subject_id, object_record_id)
        REFERENCES ple_private.course_banner_storage_subject (course_banner_storage_subject_id, object_record_id),
    FOREIGN KEY (object_delivery_id, object_record_id)
        REFERENCES ple_data.object_delivery (object_delivery_id, object_record_id)
);


CREATE TABLE ple_private.course_banner_prepared_presentation (
    course_instance_id ple_data.course_instance_id NOT NULL,
    course_banner_id uuid NOT NULL,
    alternative_kind ple_data.alternative_kind NOT NULL,
    alternative_text text,
    PRIMARY KEY (course_instance_id, course_banner_id),
    FOREIGN KEY (course_instance_id, course_banner_id) REFERENCES ple_data.course_banner,
    CHECK ((alternative_kind = 'decorative' AND alternative_text IS NULL)
        OR (alternative_kind = 'informative' AND char_length(btrim(alternative_text)) BETWEEN 1 AND 160)),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.course_banner IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_rendition IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_delivery IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_banner_upload IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_storage_subject IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_work IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_token IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_expires_at.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_expires_at IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_token.';
COMMENT ON COLUMN ple_private.course_banner_work.completed_at IS 'NULL means this optional fact is absent.';

COMMENT ON TABLE ple_private.course_banner_prepared_presentation IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.course_banner IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_rendition IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_delivery IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_banner_upload IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_storage_subject IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_work IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_token IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_expires_at.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_expires_at IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_token.';
COMMENT ON COLUMN ple_private.course_banner_work.completed_at IS 'NULL means this optional fact is absent.';

COMMENT ON TABLE ple_private.course_banner_prepared_presentation IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.course_banner IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_rendition IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_delivery IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_banner_upload IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_storage_subject IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_work IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_token IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_expires_at.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_expires_at IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_token.';
COMMENT ON COLUMN ple_private.course_banner_work.completed_at IS 'NULL means this optional fact is absent.';

COMMENT ON TABLE ple_private.course_banner_prepared_presentation IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.course_banner IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_rendition IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_data.course_banner_delivery IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_banner_upload IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_storage_subject IS 'role: current state, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

COMMENT ON TABLE ple_private.course_banner_work IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_token IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_expires_at.';
COMMENT ON COLUMN ple_private.course_banner_work.lease_expires_at IS 'NULL means this optional fact is absent. Shared work-lease pair with lease_token.';
COMMENT ON COLUMN ple_private.course_banner_work.completed_at IS 'NULL means this optional fact is absent.';

COMMENT ON TABLE ple_private.course_banner_prepared_presentation IS 'role: event, deleted by Course delete and object cleanup. HUMAN_GUIDANCE.md Course appearance.';

SET LOCAL ROLE ple_private_owner;
COMMENT ON COLUMN ple_private.course_banner_upload.promoted_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_banner_storage_subject.course_banner_upload_reference IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_banner_storage_subject.course_banner_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_banner_work.course_banner_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_banner_work.course_banner_storage_subject_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_banner_work.object_delivery_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_banner_work.completed_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_private.course_banner_prepared_presentation.alternative_text IS 'NULL means this optional fact is absent.';

