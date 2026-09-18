-- course_instance tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Current Course Instance truth and immutable source history.
CREATE TABLE ple_data.course_instance (
    course_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    public_reference text NOT NULL UNIQUE CHECK (
        ple_private.is_canonical_prefixed_public_id(public_reference, 'CI')
    ),
    -- Empty is a real Course source variant, not a sentinel Blueprint.
    source_kind text NOT NULL CHECK (source_kind IN ('empty', 'adopted')),
    blueprint_course_reference_number bigint,
    blueprint_revision_number bigint CHECK (blueprint_revision_number > 0),
    assigned_instructor_account_id uuid NOT NULL,
    assigned_instructor_role text NOT NULL DEFAULT 'instructor'
        CHECK (assigned_instructor_role = 'instructor'),
    course_short_name text NOT NULL CHECK (
        course_short_name = btrim(course_short_name)
        AND char_length(course_short_name) BETWEEN 1 AND 200
    ),
    course_long_name text NOT NULL CHECK (
        course_long_name = btrim(course_long_name)
        AND char_length(course_long_name) BETWEEN 1 AND 200
    ),
    discipline_uuid uuid NOT NULL REFERENCES ple_data.content_discipline(discipline_uuid),
    subject_uuid uuid,
    topic_uuid uuid,
    subtopic_uuid uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    metadata_etag uuid NOT NULL DEFAULT gen_random_uuid(),
    CHECK (topic_uuid IS NULL OR subject_uuid IS NOT NULL),
    CHECK (subtopic_uuid IS NULL OR topic_uuid IS NOT NULL),
    FOREIGN KEY (subject_uuid, discipline_uuid)
        REFERENCES ple_data.content_subject_discipline(subject_uuid, discipline_uuid),
    FOREIGN KEY (subject_uuid, topic_uuid)
        REFERENCES ple_data.content_topic(subject_uuid, topic_uuid),
    FOREIGN KEY (topic_uuid, subtopic_uuid)
        REFERENCES ple_data.content_subtopic(topic_uuid, subtopic_uuid),
    term_starts_on date NOT NULL,
    term_ends_on date NOT NULL CHECK (term_ends_on >= term_starts_on),
    course_theme text NOT NULL DEFAULT 'grass' CHECK (course_theme IN (
        'tundra', 'forest', 'desert', 'grass', 'arctic', 'ocean', 'tropical',
        'coral-reef', 'swamp', 'underground', 'salt-marsh', 'wetland',
        'sea-floor', 'magma', 'beach'
    )),
    created_at timestamp with time zone NOT NULL,
    active_until_at timestamp with time zone NOT NULL,
    course_lifecycle_state text NOT NULL DEFAULT 'active'
        CHECK (course_lifecycle_state IN ('active', 'inactive')),
    course_became_inactive_at timestamp with time zone,
    -- Assessment saves keep this current fact synchronized with the latest
    -- Unreleased or Released Assessment due instant.
    latest_assessment_due_at timestamp with time zone,
    retention_starts_at timestamp with time zone NOT NULL,
    retention_lifecycle_state text NOT NULL DEFAULT 'active'
        CHECK (retention_lifecycle_state IN ('active', 'archived', 'deleted')),
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
            AND blueprint_course_reference_number IS NULL
            AND blueprint_revision_number IS NULL)
        OR (source_kind = 'adopted'
            AND blueprint_course_reference_number IS NOT NULL
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
    FOREIGN KEY (assigned_instructor_account_id, assigned_instructor_role)
        REFERENCES ple_private.account (account_id, product_role),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE TABLE ple_data.course_origin (
    course_origin_id uuid PRIMARY KEY,
    course_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_id),
    source_kind text NOT NULL CHECK (source_kind IN ('empty', 'adopted')),
    blueprint_course_reference_number bigint,
    blueprint_revision_number bigint CHECK (blueprint_revision_number > 0),
    source_course_id uuid REFERENCES ple_data.course_instance (course_id),
    created_at timestamp with time zone NOT NULL,
    CHECK (
        (source_kind = 'empty'
            AND blueprint_course_reference_number IS NULL
            AND blueprint_revision_number IS NULL
            AND source_course_id IS NULL)
        OR (source_kind = 'adopted'
            AND blueprint_course_reference_number IS NOT NULL
            AND blueprint_revision_number IS NOT NULL
            AND source_course_id IS NULL)
    ),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_instance_creation_event (
    course_instance_creation_event_id uuid PRIMARY KEY,
    course_id uuid NOT NULL UNIQUE REFERENCES ple_data.course_instance (course_id),
    course_reference_number bigint NOT NULL UNIQUE,
    source_kind text NOT NULL CHECK (source_kind IN ('empty', 'adopted')),
    blueprint_course_reference_number bigint,
    blueprint_revision_number bigint CHECK (blueprint_revision_number > 0),
    assigned_instructor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    CHECK (
        (source_kind = 'empty'
            AND blueprint_course_reference_number IS NULL
            AND blueprint_revision_number IS NULL)
        OR (source_kind = 'adopted'
            AND blueprint_course_reference_number IS NOT NULL
            AND blueprint_revision_number IS NOT NULL)
    ),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.course_instance IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course Instances.';

COMMENT ON TABLE ple_data.course_origin IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course Instances.';

SET LOCAL ROLE ple_audit_owner;

COMMENT ON TABLE ple_audit.course_instance_creation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course Instances.';

