-- course_instance tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.course_theme (
    course_theme_id text PRIMARY KEY CHECK (
        course_theme_id = btrim(course_theme_id)
        AND course_theme_id ~ '^[a-z][a-z0-9-]{0,31}$'
    ),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.course_theme (course_theme_id) VALUES
    ('tundra'), ('forest'), ('desert'), ('grass'), ('arctic'),
    ('ocean'), ('tropical'), ('coral-reef'), ('swamp'), ('underground'),
    ('salt-marsh'), ('wetland'), ('sea-floor'), ('magma'), ('beach');
SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.course_theme IS
    'role: vocabulary, authored Course appearance palettes. Deleted by: none.';

-- Current Course Instance truth and immutable source history.
CREATE TABLE ple_data.course_instance (
    course_instance_id ple_data.course_instance_id PRIMARY KEY,
    source_kind ple_data.course_source_kind NOT NULL,
    blueprint_course_id ple_data.blueprint_course_id,
    blueprint_revision_number integer CHECK (blueprint_revision_number > 0),
    course_short_name text NOT NULL CHECK (
        course_short_name = btrim(course_short_name)
        AND char_length(course_short_name) BETWEEN 1 AND 200
    ),
    course_long_name text NOT NULL CHECK (
        course_long_name = btrim(course_long_name)
        AND char_length(course_long_name) BETWEEN 1 AND 200
    ),
    content_discipline_id uuid NOT NULL REFERENCES ple_data.content_discipline(content_discipline_id),
    content_subject_id uuid,
    content_topic_id uuid,
    content_subtopic_id uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    course_edit_number bigint NOT NULL DEFAULT 1 CHECK (course_edit_number > 0),
    CHECK (content_topic_id IS NULL OR content_subject_id IS NOT NULL),
    CHECK (content_subtopic_id IS NULL OR content_topic_id IS NOT NULL),
    FOREIGN KEY (content_subject_id, content_discipline_id)
        REFERENCES ple_data.content_subject_discipline(content_subject_id, content_discipline_id),
    FOREIGN KEY (content_subject_id, content_topic_id)
        REFERENCES ple_data.content_topic(content_subject_id, content_topic_id),
    FOREIGN KEY (content_topic_id, content_subtopic_id)
        REFERENCES ple_data.content_subtopic(content_topic_id, content_subtopic_id),
    term_starts_on date NOT NULL,
    term_ends_on date NOT NULL CHECK (term_ends_on >= term_starts_on),
    course_theme_id text NOT NULL DEFAULT 'grass'
        REFERENCES ple_data.course_theme (course_theme_id),
    created_at timestamp with time zone NOT NULL,
    active_until_at timestamp with time zone NOT NULL,
    course_lifecycle_state ple_data.course_lifecycle_state NOT NULL DEFAULT 'active',
    course_became_inactive_at timestamp with time zone,
    -- Assessment saves keep this current fact synchronized with the latest
    -- Unreleased or Released Assessment due instant.
    latest_assessment_due_at timestamp with time zone,
    retention_starts_at timestamp with time zone NOT NULL,
    retention_lifecycle_state ple_data.retention_lifecycle_state NOT NULL DEFAULT 'active',
    student_data_archived_at timestamp with time zone,
    student_data_deleted_at timestamp with time zone,
    -- Identity-free discovery evidence, captured once at permanent deletion.
    purged_students_ever_enrolled bigint CHECK (purged_students_ever_enrolled >= 0),
    CHECK (
        (retention_lifecycle_state = 'deleted' AND purged_students_ever_enrolled IS NOT NULL)
        OR (retention_lifecycle_state <> 'deleted' AND purged_students_ever_enrolled IS NULL)
    ),
    CHECK (
        (source_kind = 'empty'
            AND blueprint_course_id IS NULL
            AND blueprint_revision_number IS NULL)
        OR (source_kind = 'adopted'
            AND blueprint_course_id IS NOT NULL
            AND blueprint_revision_number IS NOT NULL)
    ),
    CHECK (active_until_at = (
        (created_at AT TIME ZONE 'UTC') + INTERVAL '6 months'
    ) AT TIME ZONE 'UTC'),
    -- Course terms use inclusive calendar dates. Their final date may be the
    -- UTC calendar date containing the immutable Active cutoff, but never a
    -- later date. The exact cutoff instant still controls Active status.
    CHECK (term_ends_on <= (active_until_at AT TIME ZONE 'UTC')::date),
    CHECK (
        (course_lifecycle_state = 'active' AND course_became_inactive_at IS NULL)
        OR (
            course_lifecycle_state = 'inactive'
            AND course_became_inactive_at >= created_at
            AND course_became_inactive_at <= active_until_at
        )
    ),
    CHECK (latest_assessment_due_at IS NULL OR (
        latest_assessment_due_at >= created_at
        AND latest_assessment_due_at <= active_until_at
    )),
    CHECK (
        (retention_lifecycle_state = 'active'
            AND retention_starts_at = COALESCE(latest_assessment_due_at, active_until_at)
            AND student_data_archived_at IS NULL
            AND student_data_deleted_at IS NULL)
        OR (retention_lifecycle_state = 'archived'
            AND retention_starts_at BETWEEN created_at AND active_until_at
            AND student_data_archived_at >= retention_starts_at
            AND student_data_deleted_at IS NULL)
        OR (retention_lifecycle_state = 'deleted'
            AND retention_starts_at BETWEEN created_at AND active_until_at
            AND student_data_archived_at >= retention_starts_at
            AND student_data_deleted_at >= student_data_archived_at)
    ),
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



CREATE TABLE ple_data.course_origin (
    course_origin_id uuid PRIMARY KEY,
    course_instance_id ple_data.course_instance_id NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_instance_id),
    source_kind ple_data.course_source_kind NOT NULL,
    blueprint_course_id ple_data.blueprint_course_id,
    blueprint_revision_number integer CHECK (blueprint_revision_number > 0),
    source_course_instance_id ple_data.course_instance_id REFERENCES ple_data.course_instance (course_instance_id),
    created_at timestamp with time zone NOT NULL,
    CHECK (
        (source_kind = 'empty'
            AND blueprint_course_id IS NULL
            AND blueprint_revision_number IS NULL
            AND source_course_instance_id IS NULL)
        OR (source_kind = 'adopted'
            AND blueprint_course_id IS NOT NULL
            AND blueprint_revision_number IS NOT NULL
            AND source_course_instance_id IS NULL)
    ),
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number)
);


SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_instance_creation_event (
    course_instance_creation_event_id uuid PRIMARY KEY,
    course_instance_id ple_data.course_instance_id NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_instance_id),
    source_kind ple_data.course_source_kind NOT NULL,
    blueprint_course_id ple_data.blueprint_course_id,
    blueprint_revision_number integer CHECK (blueprint_revision_number > 0),
    assigned_instructor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    created_by_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    CHECK (
        (source_kind = 'empty'
            AND blueprint_course_id IS NULL
            AND blueprint_revision_number IS NULL)
        OR (source_kind = 'adopted'
            AND blueprint_course_id IS NOT NULL
            AND blueprint_revision_number IS NOT NULL)
    ),
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number)
);


SET LOCAL ROLE ple_data_owner;

-- This relation is source provenance, not daughter-Course adoption provenance.
-- A Course Instance remains unchanged and never acquires a parent Blueprint
-- when its reusable structure is copied into a new lineage.
CREATE TABLE ple_data.blueprint_course_instance_source (
    blueprint_course_instance_source_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    blueprint_course_id ple_data.blueprint_course_id NOT NULL UNIQUE
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    source_course_instance_id ple_data.course_instance_id NOT NULL REFERENCES ple_data.course_instance (course_instance_id),
    recorded_at timestamp with time zone NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


COMMENT ON TABLE ple_data.blueprint_course_instance_source IS 'role: current state, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.course_instance IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course Instances.';

COMMENT ON TABLE ple_data.course_origin IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course Instances.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_instance_creation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course Instances.';

SET LOCAL ROLE ple_data_owner;
COMMENT ON COLUMN ple_data.course_instance.course_instance_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.blueprint_course_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.blueprint_revision_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.content_subject_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.content_topic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.content_subtopic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.course_became_inactive_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.latest_assessment_due_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.student_data_archived_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.student_data_deleted_at IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_instance.purged_students_ever_enrolled IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_origin.blueprint_course_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_origin.blueprint_revision_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.course_origin.source_course_instance_id IS 'NULL means this optional fact is absent.';
SET LOCAL ROLE ple_audit_owner;
COMMENT ON COLUMN ple_audit.course_instance_creation_event.blueprint_course_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_audit.course_instance_creation_event.blueprint_revision_number IS 'NULL means this optional fact is absent.';

