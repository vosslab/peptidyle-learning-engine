-- Current Course Instance truth and immutable source history.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.course_instance (
    course_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE
        CHECK (reference_number BETWEEN 1 AND 2147483647),
    public_reference text NOT NULL UNIQUE CHECK (public_reference ~ '^CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$'),
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

CREATE TRIGGER course_instance_human_reference_is_minted
BEFORE INSERT ON ple_data.course_instance
FOR EACH ROW EXECUTE FUNCTION ple_private.assign_human_reference('CI');

CREATE FUNCTION ple_data.enforce_course_instance_retention_schedule()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        NEW.active_until_at := COALESCE(
            NEW.active_until_at,
            ((NEW.created_at AT TIME ZONE 'UTC') + INTERVAL '6 months') AT TIME ZONE 'UTC'
        );
        NEW.retention_lifecycle_state := 'active';
        NEW.retention_starts_at := COALESCE(
            NEW.latest_assessment_due_at,
            NEW.active_until_at
        );
    ELSIF NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.active_until_at IS DISTINCT FROM OLD.active_until_at THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'a Course Instance creation time and Active lifetime are immutable';
    END IF;
    IF NEW.active_until_at IS DISTINCT FROM (
        ((NEW.created_at AT TIME ZONE 'UTC') + INTERVAL '6 months') AT TIME ZONE 'UTC'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'a Course Instance Active lifetime is six months from creation';
    END IF;
    IF TG_OP = 'UPDATE' THEN
        IF OLD.course_lifecycle_state = 'inactive' AND (
            NEW.course_lifecycle_state IS DISTINCT FROM OLD.course_lifecycle_state
            OR NEW.course_became_inactive_at IS DISTINCT FROM OLD.course_became_inactive_at
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course Instance cannot return from Inactive';
        END IF;
        IF (OLD.retention_lifecycle_state = 'active'
               AND NEW.retention_lifecycle_state NOT IN ('active', 'archived'))
           OR (OLD.retention_lifecycle_state = 'archived'
               AND NEW.retention_lifecycle_state NOT IN ('archived', 'deleted'))
           OR (OLD.retention_lifecycle_state = 'deleted'
               AND NEW.retention_lifecycle_state <> 'deleted') THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course retention lifecycle cannot regress';
        END IF;
        IF OLD.retention_lifecycle_state IN ('archived', 'deleted')
           AND NEW.retention_starts_at IS DISTINCT FROM OLD.retention_starts_at THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course retention start is immutable';
        END IF;
        IF OLD.student_data_archived_at IS NOT NULL
           AND NEW.student_data_archived_at IS DISTINCT FROM OLD.student_data_archived_at THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course Student-data archive time is immutable';
        END IF;
        IF OLD.student_data_deleted_at IS NOT NULL
           AND NEW.student_data_deleted_at IS DISTINCT FROM OLD.student_data_deleted_at THEN
            RAISE EXCEPTION USING ERRCODE = '55000',
                MESSAGE = 'a Course Student-data deletion time is immutable';
        END IF;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER course_instance_retention_schedule_is_consistent
BEFORE INSERT OR UPDATE ON ple_data.course_instance
FOR EACH ROW EXECUTE FUNCTION ple_data.enforce_course_instance_retention_schedule();

CREATE FUNCTION ple_data.reject_course_instance_source_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    IF NEW.source_kind IS DISTINCT FROM OLD.source_kind
       OR NEW.blueprint_course_reference_number
          IS DISTINCT FROM OLD.blueprint_course_reference_number
       OR NEW.blueprint_revision_number IS DISTINCT FROM OLD.blueprint_revision_number THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'a Course Instance source is immutable';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER course_instance_source_is_immutable
BEFORE UPDATE OF source_kind, blueprint_course_reference_number, blueprint_revision_number
ON ple_data.course_instance
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_instance_source_change();

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

CREATE FUNCTION ple_data.reject_course_origin_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'a Course Origin is immutable';
END
$$;

CREATE TRIGGER course_origin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.course_origin
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_origin_change();

ALTER TABLE ple_data.course_instance ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_instance FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_origin ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_origin FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_data.course_instance, ple_data.course_origin FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.reject_course_origin_change(),
    ple_data.enforce_course_instance_retention_schedule(),
    ple_data.reject_course_instance_source_change() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_api_owner;
GRANT SELECT, INSERT ON ple_data.course_instance, ple_data.course_origin TO ple_api_owner;
GRANT UPDATE (discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags, metadata_etag)
    ON ple_data.course_instance TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner, ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance TO ple_private_owner, ple_audit_owner;
CREATE POLICY course_instance_api_owner_access ON ple_data.course_instance
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_instance_data_owner_deadline_schedule ON ple_data.course_instance
    FOR UPDATE TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY course_origin_api_owner_access ON ple_data.course_origin
    FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);

RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT REFERENCES ON TABLE ple_data.blueprint_course_revision TO ple_audit_owner;
RESET ROLE;

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

CREATE FUNCTION ple_audit.reject_course_instance_creation_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'a Course Instance Creation Event is immutable';
END
$$;

CREATE TRIGGER course_instance_creation_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_audit.course_instance_creation_event
FOR EACH ROW EXECUTE FUNCTION ple_audit.reject_course_instance_creation_event_change();

ALTER TABLE ple_audit.course_instance_creation_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.course_instance_creation_event FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_audit.course_instance_creation_event FROM PUBLIC;
CREATE POLICY course_instance_creation_event_owner_write
    ON ple_audit.course_instance_creation_event FOR INSERT TO ple_audit_owner WITH CHECK (true);

CREATE FUNCTION ple_audit.record_course_instance_creation_event(
    p_event_id uuid, p_course_id uuid, p_reference bigint, p_source_kind text,
    p_blueprint_reference bigint, p_blueprint_revision bigint,
    p_assigned_instructor uuid, p_creator uuid,
    p_occurred_at timestamp with time zone
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_audit
AS $$
BEGIN
    IF p_event_id IS NULL OR p_course_id IS NULL OR p_reference NOT BETWEEN 1 AND 2147483647
       OR p_source_kind IS NULL OR p_source_kind NOT IN ('empty', 'adopted')
       OR (p_source_kind = 'empty'
           AND (p_blueprint_reference IS NOT NULL OR p_blueprint_revision IS NOT NULL))
       OR (p_source_kind = 'adopted'
           AND (p_blueprint_reference IS NULL OR p_blueprint_revision IS NULL
                OR p_blueprint_reference NOT BETWEEN 1 AND 2147483647 OR p_blueprint_revision <= 0))
       OR p_assigned_instructor IS NULL OR p_creator IS NULL OR p_occurred_at IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance Creation Event arguments are invalid';
    END IF;
    INSERT INTO ple_audit.course_instance_creation_event
    VALUES (p_event_id, p_course_id, p_reference, p_source_kind,
            p_blueprint_reference, p_blueprint_revision,
            p_assigned_instructor, p_creator, p_occurred_at);
END
$$;

REVOKE ALL ON FUNCTION ple_audit.reject_course_instance_creation_event_change(),
    ple_audit.record_course_instance_creation_event(
        uuid, uuid, bigint, text, bigint, bigint, uuid, uuid, timestamp with time zone
    ) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_audit.record_course_instance_creation_event(
    uuid, uuid, bigint, text, bigint, bigint, uuid, uuid, timestamp with time zone
) TO ple_api_owner;

RESET ROLE;
