-- object_record tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_private_owner;

-- Immutable object identities, delivery ownership, and factual storage receipts.
-- External storage remains outside PostgreSQL.  These rows retain only the
-- exact facts that PostgreSQL can authorize and later reconcile.
CREATE TABLE ple_private.object_record (
    object_id uuid PRIMARY KEY,
    object_address jsonb NOT NULL CHECK (jsonb_typeof(object_address) = 'object'),
    object_storage_area text NOT NULL CHECK (object_storage_area IN
        ('public-assets', 'private-content', 'student-records', 'temp-processing')),
    object_data_class text NOT NULL CHECK (object_data_class IN
        ('authoring-content', 'question-source', 'question-asset', 'question-render',
         'course-appearance', 'profile-image', 'student-record',
         'temporary-processing')),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    size_bytes bigint NOT NULL CHECK (size_bytes >= 0),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 255),
    created_at timestamptz NOT NULL,
    UNIQUE (object_address)
);

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.object_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    media_type text NOT NULL CHECK (char_length(btrim(media_type)) BETWEEN 1 AND 200),
    byte_length bigint NOT NULL CHECK (byte_length >= 0),
    delivery_state text NOT NULL CHECK (delivery_state IN ('pending', 'available', 'retired')),
    registered_at timestamptz NOT NULL,
    UNIQUE (delivery_id, object_id)
);

CREATE TABLE ple_data.course_object_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    course_id uuid NOT NULL,
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id)
);

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.object_storage_check (
    object_storage_check_id uuid PRIMARY KEY,
    delivery_id uuid REFERENCES ple_data.object_delivery (delivery_id),
    -- A check applies to one immutable storage anchor.  Course Media owns the
    -- banner subject relation and the late cross-domain foreign key; this
    -- generic record owns the complete anchor shape from its initial DDL.
    course_banner_storage_subject_id uuid,
    expected_sha256 bytea NOT NULL CHECK (octet_length(expected_sha256) = 32),
    check_result text NOT NULL CHECK (check_result IN ('verified', 'missing', 'mismatched')),
    checked_at timestamptz NOT NULL,
    CHECK ((delivery_id IS NOT NULL) <> (course_banner_storage_subject_id IS NOT NULL))
);

CREATE TABLE ple_private.object_cleanup_manifest (
    object_cleanup_manifest_id uuid PRIMARY KEY,
    object_storage_check_id uuid NOT NULL REFERENCES ple_private.object_storage_check,
    authorized_at timestamptz NOT NULL,
    permitted_disposition text NOT NULL CHECK (permitted_disposition IN ('deleted', 'already_absent', 'retained')),
    UNIQUE (object_cleanup_manifest_id, permitted_disposition)
);

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.object_delivery_access_event (
    event_id uuid PRIMARY KEY,
    delivery_id uuid NOT NULL REFERENCES ple_data.object_delivery,
    account_id uuid NOT NULL REFERENCES ple_private.account,
    access_decision text NOT NULL CHECK (access_decision IN ('allowed', 'denied')),
    accessed_at timestamptz NOT NULL,
    UNIQUE (event_id, delivery_id)
);

CREATE TABLE ple_audit.object_storage_check_event (
    event_id uuid PRIMARY KEY,
    object_storage_check_id uuid NOT NULL REFERENCES ple_private.object_storage_check,
    check_result text NOT NULL CHECK (check_result IN ('verified', 'missing', 'mismatched')),
    recorded_at timestamptz NOT NULL,
    object_storage_check_event_checksum bytea NOT NULL CHECK (octet_length(object_storage_check_event_checksum) = 32),
    UNIQUE (object_storage_check_id, check_result, object_storage_check_event_checksum)
);

CREATE TABLE ple_audit.object_cleanup_receipt (
    object_cleanup_receipt_id uuid PRIMARY KEY,
    object_cleanup_manifest_id uuid NOT NULL,
    disposition text NOT NULL,
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (object_cleanup_manifest_id, disposition)
        REFERENCES ple_private.object_cleanup_manifest (object_cleanup_manifest_id, permitted_disposition)
);

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.object_record IS 'role: current state, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.object_delivery IS 'role: current state, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

COMMENT ON TABLE ple_data.course_object_delivery IS 'role: current state, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.object_storage_check IS 'role: event, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

COMMENT ON TABLE ple_private.object_cleanup_manifest IS 'role: event, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

SET LOCAL ROLE ple_audit_owner;

COMMENT ON TABLE ple_audit.object_delivery_access_event IS 'role: event, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

COMMENT ON TABLE ple_audit.object_storage_check_event IS 'role: event, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

COMMENT ON TABLE ple_audit.object_cleanup_receipt IS 'role: event, deleted by object cleanup after the last delivery is gone. HUMAN_GUIDANCE.md Object storage.';

