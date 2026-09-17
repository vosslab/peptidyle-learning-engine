-- Course, roster, and narrowly scoped support operations. Structures live in
-- the preceding Course modules so this file can resolve their exact roots.

SET LOCAL ROLE ple_data_owner;

CREATE POLICY course_instance_data_owner_instructor_read ON ple_data.course_instance
    FOR SELECT TO ple_data_owner
    USING (ple_api.current_session_account_is_course_instructor(course_id));
CREATE POLICY course_instance_member_read ON ple_data.course_instance
    FOR SELECT TO ple_app USING (ple_api.current_session_account_is_course_member(course_id));
CREATE POLICY course_membership_instructor_or_self_read ON ple_data.course_membership
    FOR SELECT TO ple_app USING (
        ple_api.current_session_account_is_course_instructor(course_id)
        OR ple_api.current_session_account_owns_course_membership(course_id, membership_id)
    );
CREATE POLICY student_record_instructor_or_self_read ON ple_data.student_record
    FOR SELECT TO ple_app USING (
        ple_api.current_session_account_is_course_instructor(course_id)
        OR ple_api.current_session_account_owns_student_record(course_id, student_record_id)
    );

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_course_theme(p_course_id uuid)
RETURNS TABLE(course_theme text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course_theme FROM ple_data.course_instance
     WHERE course_id = p_course_id AND ple_api.current_session_account_is_course_member(p_course_id)
$$;

CREATE FUNCTION ple_api.update_course_theme(p_course_id uuid, p_course_theme text)
RETURNS TABLE(course_theme text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    UPDATE ple_data.course_instance SET course_theme = p_course_theme
     WHERE course_id = p_course_id AND ple_api.current_session_account_is_course_instructor(p_course_id)
 RETURNING course_theme
$$;

CREATE FUNCTION ple_api.resolve_course_navigation(p_public_reference text)
RETURNS TABLE(course_id uuid) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course_id FROM ple_data.course_instance
     WHERE public_reference = p_public_reference
       AND ple_api.current_session_account_is_course_member(course_id)
$$;

CREATE FUNCTION ple_api.read_course_summary(p_course_id uuid)
RETURNS TABLE(course_id uuid, public_reference text, short_name text, long_name text,
              term_starts_on date, term_ends_on date, membership_role text,
              discipline_uuid uuid, subject_uuid uuid, topic_uuid uuid, subtopic_uuid uuid, tags text[])
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_id, course.public_reference, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, membership.role,
           course.discipline_uuid, course.subject_uuid, course.topic_uuid, course.subtopic_uuid, course.tags
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND ple_data.course_membership_is_active(membership.membership_id)
     WHERE course.course_id = p_course_id
       AND ple_api.current_session_account_is_course_member(p_course_id)
$$;

CREATE FUNCTION ple_api.create_course_instance(
    p_course_id uuid, p_origin_id uuid, p_membership_id uuid, p_event_id uuid,
    p_source_kind text, p_blueprint_reference text, p_blueprint_revision bigint,
    p_short_name text, p_long_name text, p_term_start date, p_term_end date,
    p_assigned_instructor_reference text, p_assessments jsonb,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE(public_reference text, short_name text, long_name text, term_starts_on date,
              term_ends_on date, discipline_uuid uuid, subject_uuid uuid, topic_uuid uuid,
              subtopic_uuid uuid, tags text[], metadata_etag uuid)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    actor uuid;
    assigned uuid;
    course_reference_number bigint;
    course_public_reference text;
    now_at timestamptz;
    blueprint ple_data.blueprint_course%ROWTYPE;
    expected_sources uuid[];
    supplied_sources uuid[];
BEGIN
    IF p_course_id IS NULL OR p_origin_id IS NULL OR p_membership_id IS NULL OR p_event_id IS NULL
       OR p_source_kind IS NULL OR p_source_kind NOT IN ('empty', 'adopted')
       OR (p_source_kind = 'empty'
           AND (p_blueprint_reference IS NOT NULL OR p_blueprint_revision IS NOT NULL))
       OR (p_source_kind = 'adopted'
           AND (p_blueprint_reference IS NULL OR p_blueprint_revision IS NULL
                OR p_blueprint_revision <= 0))
       OR p_short_name IS NULL OR p_short_name <> btrim(p_short_name)
       OR char_length(p_short_name) NOT BETWEEN 1 AND 200
       OR p_long_name IS NULL OR p_long_name <> btrim(p_long_name)
       OR char_length(p_long_name) NOT BETWEEN 1 AND 200
       OR p_term_start IS NULL OR p_term_end IS NULL OR p_term_end < p_term_start THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Instance creation arguments are invalid';
    END IF;
    actor := ple_api.current_session_account_id();
    IF ple_api.current_session_account_is_instructor() THEN
        assigned := actor;
        IF p_assigned_instructor_reference IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_private.account AS account
             WHERE account.account_id = actor
               AND account.public_reference = p_assigned_instructor_reference
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '42501',
                MESSAGE = 'an Instructor may create a Course Instance only for self';
        END IF;
    ELSIF ple_api.current_session_account_is_sysadmin() THEN
        SELECT account.account_id INTO assigned FROM ple_private.account AS account
         WHERE account.public_reference = p_assigned_instructor_reference
           AND account.product_role = 'instructor';
        IF NOT FOUND THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'the selected Assigned Instructor is unavailable';
        END IF;
    ELSE
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instance creation requires an active Instructor or Sysadmin Account';
    END IF;
    now_at := pg_catalog.transaction_timestamp();
    -- ASVS 2.2.2 and 2.3.2: the trusted creation boundary compares the
    -- Instructor-supplied calendar date with the immutable server clock. A
    -- term may include the UTC calendar date containing the six-month cutoff,
    -- but cannot name a later date; the cutoff instant remains authoritative.
    IF p_term_end > (((now_at AT TIME ZONE 'UTC') + INTERVAL '6 months')::date) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instance term cannot extend past its Active lifetime';
    END IF;
    IF jsonb_typeof(p_assessments) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Instance assessments are invalid';
    END IF;
    IF p_source_kind = 'empty' THEN
        IF jsonb_array_length(p_assessments) <> 0 THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'an Empty Course Instance accepts no initial Assessments';
        END IF;
    ELSE
        -- C49 remains the lifecycle prerequisite: serialize against Blueprint
        -- Save and archive before comparing the submitted Revision with the head.
        SELECT source_blueprint.* INTO blueprint
          FROM ple_data.blueprint_course AS source_blueprint
         WHERE source_blueprint.public_reference = p_blueprint_reference
         FOR UPDATE OF source_blueprint;
        IF NOT FOUND OR blueprint.availability <> 'public' THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Instance source is unavailable';
        END IF;
        IF blueprint.current_blueprint_revision_number <> p_blueprint_revision THEN
            RAISE EXCEPTION USING ERRCODE = '40001',
                MESSAGE = 'Blueprint Revision precondition is stale';
        END IF;
        SELECT array_agg(source.blueprint_assessment_reference ORDER BY source.blueprint_assessment_reference)
          INTO expected_sources FROM ple_data.blueprint_revision_assessment AS source
         WHERE source.blueprint_course_reference_number = blueprint.reference_number
           AND source.blueprint_revision_number = p_blueprint_revision;
        SELECT array_agg((item ->> 'source')::uuid ORDER BY (item ->> 'source')::uuid)
          INTO supplied_sources FROM jsonb_array_elements(p_assessments) AS item;
        IF expected_sources IS NULL OR supplied_sources IS DISTINCT FROM expected_sources THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Blueprint adoption requires every Assessment from its exact Revision';
        END IF;
    END IF;
    INSERT INTO ple_data.course_instance (
        course_id, source_kind, blueprint_course_reference_number, blueprint_revision_number,
        assigned_instructor_account_id, course_short_name, course_long_name,
        term_starts_on, term_ends_on, created_at,
        discipline_uuid, subject_uuid, topic_uuid, subtopic_uuid, tags
    ) VALUES (
        p_course_id, p_source_kind, blueprint.reference_number, p_blueprint_revision, assigned,
        p_short_name, p_long_name, p_term_start, p_term_end, now_at,
        p_discipline, p_subject, p_topic, p_subtopic, p_tags
    ) RETURNING ple_data.course_instance.reference_number, ple_data.course_instance.public_reference
      INTO course_reference_number, course_public_reference;
    INSERT INTO ple_data.course_origin (
        course_origin_id, course_id, source_kind, blueprint_course_reference_number,
        blueprint_revision_number, source_course_id, created_at
    ) VALUES (p_origin_id, p_course_id, p_source_kind, blueprint.reference_number,
              p_blueprint_revision, NULL, now_at);
    INSERT INTO ple_data.course_membership (membership_id, course_id, account_id, role, joined_at)
    VALUES (p_membership_id, p_course_id, assigned, 'instructor', now_at);
    IF p_source_kind = 'adopted' THEN
        PERFORM ple_data.initialize_course_assessments(
            p_course_id, blueprint.reference_number, p_blueprint_revision, p_assessments
        );
    END IF;
    PERFORM ple_audit.record_course_instance_creation_event(
        p_event_id, p_course_id, course_reference_number, p_source_kind,
        blueprint.reference_number, p_blueprint_revision, assigned, actor, now_at
    );
    RETURN QUERY SELECT course.public_reference, course.course_short_name, course.course_long_name,
        course.term_starts_on, course.term_ends_on, course.discipline_uuid, course.subject_uuid,
        course.topic_uuid, course.subtopic_uuid, course.tags, course.metadata_etag
        FROM ple_data.course_instance AS course WHERE course.course_id = p_course_id;
END
$$;

-- A current Instructor may extend only that Course's teaching team.  The
-- assigned Instructor is historical accountability, never an authority rank.
CREATE FUNCTION ple_api.add_course_instructor(
    p_membership_id uuid, p_course_reference text, p_instructor_reference text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course uuid;
    instructor uuid;
BEGIN
    IF p_membership_id IS NULL OR p_course_reference IS NULL OR p_instructor_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instructor membership arguments are invalid';
    END IF;
    SELECT course_id INTO course
      FROM ple_data.course_instance
     WHERE public_reference = p_course_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Instructor membership is unavailable';
    END IF;
    SELECT account.account_id INTO instructor
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS state_event ON state_event.state = 'active'
     WHERE account.public_reference = p_instructor_reference
       AND account.product_role = 'instructor';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'the requested Instructor is unavailable';
    END IF;
    INSERT INTO ple_data.course_membership (
        membership_id, course_id, account_id, role, joined_at
    ) VALUES (
        p_membership_id, course, instructor, 'instructor', pg_catalog.transaction_timestamp()
    );
END
$$;

CREATE FUNCTION ple_api.list_course_instances()
RETURNS TABLE(public_reference text, short_name text, long_name text, term_starts_on date,
              term_ends_on date, course_theme text, discipline_uuid uuid, subject_uuid uuid,
              topic_uuid uuid, subtopic_uuid uuid, tags text[], metadata_etag uuid)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.public_reference, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, course.course_theme,
           course.discipline_uuid, course.subject_uuid, course.topic_uuid,
           course.subtopic_uuid, course.tags, course.metadata_etag
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
     WHERE ple_api.current_session_account_is_instructor()
     ORDER BY course.course_long_name, course.public_reference
$$;

CREATE FUNCTION ple_api.load_course_instance(p_reference text)
RETURNS TABLE(public_reference text, short_name text, long_name text, term_starts_on date,
              term_ends_on date, course_theme text,
              active_instructor_count bigint, blueprint_reference text,
              adopted_blueprint_revision bigint, current_blueprint_revision bigint,
              discipline_uuid uuid, subject_uuid uuid, topic_uuid uuid, subtopic_uuid uuid,
              tags text[], metadata_etag uuid)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.public_reference, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, course.course_theme,
           (SELECT count(*) FROM ple_data.course_membership AS teammate
             WHERE teammate.course_id = course.course_id AND teammate.role = 'instructor'
               AND ple_data.course_membership_is_active(teammate.membership_id)),
           readable_blueprint.public_reference,
           CASE WHEN readable_blueprint.public_reference IS NOT NULL
                THEN course.blueprint_revision_number END,
           readable_blueprint.current_blueprint_revision_number,
           course.discipline_uuid, course.subject_uuid, course.topic_uuid,
           course.subtopic_uuid, course.tags, course.metadata_etag
      FROM ple_data.course_instance AS course
      LEFT JOIN ple_data.blueprint_course AS parent_blueprint
        ON parent_blueprint.reference_number = course.blueprint_course_reference_number
      -- ASVS 8.2.3 and 8.3.1: Course access never grants source metadata access;
      -- reuse the Blueprint reader's current actor and lifecycle capability.
      LEFT JOIN LATERAL ple_api.load_blueprint_course(parent_blueprint.public_reference)
        AS readable_blueprint ON true
      JOIN ple_data.course_membership AS membership
        ON membership.course_id = course.course_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.membership_id)
     WHERE course.public_reference = p_reference
       AND ple_api.current_session_account_is_instructor()
$$;

CREATE FUNCTION ple_api.update_course_classification(
    p_reference text, p_expected_metadata_etag uuid,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE(metadata_etag uuid, changed boolean)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    v_course ple_data.course_instance%ROWTYPE;
    v_next uuid;
BEGIN
    -- ASVS 8.2.1/8.2.2: every current co-Instructor has equal scoped authority.
    SELECT course.* INTO v_course FROM ple_data.course_instance AS course
     WHERE course.public_reference = p_reference
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Course Instance is not available' USING ERRCODE = '42501';
    END IF;
    -- ASVS 2.3.3: only classification writes advance this bounded CAS validator.
    IF p_expected_metadata_etag IS DISTINCT FROM v_course.metadata_etag THEN
        RAISE EXCEPTION 'Course metadata ETag is stale' USING ERRCODE = '40001';
    END IF;
    IF ROW(v_course.discipline_uuid, v_course.subject_uuid, v_course.topic_uuid,
           v_course.subtopic_uuid, v_course.tags)
       IS NOT DISTINCT FROM ROW(p_discipline, p_subject, p_topic, p_subtopic, p_tags) THEN
        RETURN QUERY SELECT v_course.metadata_etag, false;
        RETURN;
    END IF;
    v_next := pg_catalog.gen_random_uuid();
    UPDATE ple_data.course_instance AS course SET
        discipline_uuid = p_discipline, subject_uuid = p_subject, topic_uuid = p_topic,
        subtopic_uuid = p_subtopic, tags = p_tags, metadata_etag = v_next
     WHERE course.course_id = v_course.course_id;
    RETURN QUERY SELECT v_next, true;
END
$$;

REVOKE ALL ON FUNCTION ple_api.update_course_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.update_course_classification(text, uuid, uuid, uuid, uuid, uuid, text[]) TO ple_app;

CREATE FUNCTION ple_api.list_course_creation_instructors()
RETURNS TABLE(public_reference text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT public_reference FROM ple_private.account
     WHERE product_role = 'instructor' AND ple_api.current_session_account_is_sysadmin()
     ORDER BY public_reference
$$;

CREATE FUNCTION ple_api.list_course_roster(p_reference text)
RETURNS TABLE(roster_id text, roster_name text, state text)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE selected_course uuid; retention_state text;
BEGIN
    SELECT course_id INTO selected_course FROM ple_data.course_instance
     WHERE public_reference = p_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(selected_course) THEN
        RETURN;
    END IF;
    SELECT retention_lifecycle_state INTO retention_state FROM ple_data.course_instance
     WHERE course_id = selected_course FOR UPDATE;
    IF retention_state IS DISTINCT FROM 'active'
       OR NOT ple_api.current_session_account_is_course_instructor(selected_course) THEN
        RETURN;
    END IF;
    RETURN QUERY SELECT profile.roster_id, profile.roster_name,
           CASE WHEN EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = course.course_id
                  AND membership.account_id = profile.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           ) THEN 'active_student' ELSE 'invitation_pending' END
      FROM ple_data.course_instance AS course
      JOIN ple_private.course_roster_profile AS profile ON profile.course_id = course.course_id
     WHERE course.public_reference = p_reference
       AND (
           EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_id = course.course_id
                  AND membership.account_id = profile.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.membership_id)
           )
           OR EXISTS (
               SELECT 1 FROM ple_private.course_invitation AS invitation
                WHERE invitation.course_id = course.course_id
                  AND invitation.target_account_id = profile.student_account_id
                  AND invitation.membership_role = 'student'
                  AND invitation.expires_at > pg_catalog.clock_timestamp()
                  AND NOT EXISTS (
                      SELECT 1 FROM ple_private.course_invitation_event AS event
                       WHERE event.invitation_id = invitation.invitation_id
                  )
           )
       )
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY profile.roster_id
    ;
END
$$;

-- C26: this is an exact, task-scoped Student-record projection, not a Course
-- browser or a generic support reader.  The support reference is canonical
-- here (not at C25's deliberately opaque request boundary).  The helper locks
-- revocation and writes the immutable `used` receipt in this same transaction;
-- this operation never writes a Course membership or Account role.
CREATE FUNCTION ple_api.read_course_roster_entry_repair_support(
    p_capability_id uuid, p_course_public_reference text, p_roster_id text
)
RETURNS TABLE(roster_id text, state text)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_data, ple_private AS $$
DECLARE course uuid; student uuid; canonical_reference text;
BEGIN
    IF p_capability_id IS NULL
       OR p_course_public_reference IS NULL
       OR p_roster_id IS NULL OR p_roster_id !~ '^[A-Za-z0-9._-]+$'
       OR NOT ple_api.current_session_account_has_platform_administration() THEN
        RETURN;
    END IF;
    SELECT course_instance.course_id INTO course
      FROM ple_data.course_instance AS course_instance
     WHERE course_instance.public_reference = p_course_public_reference;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    SELECT profile.student_account_id INTO student
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_id = course
       AND profile.roster_id = p_roster_id;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    canonical_reference := format(
        'course-instance/%s/roster/%s', p_course_public_reference, p_roster_id
    );
    PERFORM ple_api.record_support_repair_capability_use(
        p_capability_id, 'student', canonical_reference
    );
    IF NOT FOUND THEN
        RETURN;
    END IF;
    RETURN QUERY
    SELECT profile.roster_id,
           CASE
               WHEN EXISTS (
                   SELECT 1 FROM ple_data.course_membership AS membership
                    WHERE membership.course_id = course
                      AND membership.account_id = student
                      AND membership.role = 'student'
                      AND ple_data.course_membership_is_active(membership.membership_id)
               ) THEN 'active_student'
               WHEN EXISTS (
                   SELECT 1 FROM ple_private.course_invitation AS invitation
                    WHERE invitation.course_id = course
                      AND invitation.target_account_id = student
                      AND invitation.membership_role = 'student'
                      AND invitation.expires_at > pg_catalog.clock_timestamp()
                      AND NOT EXISTS (
                          SELECT 1 FROM ple_private.course_invitation_event AS event
                           WHERE event.invitation_id = invitation.invitation_id
                      )
               ) THEN 'invitation_pending'
               ELSE 'removed'
           END
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_id = course
       AND profile.student_account_id = student
       AND profile.roster_id = p_roster_id;
END
$$;

CREATE FUNCTION ple_api.list_live_student_course_landing()
RETURNS TABLE(course_public_reference text, course_short_name text, course_long_name text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT course.public_reference, course.course_short_name, course.course_long_name
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN ple_data.course_membership AS membership
        ON membership.account_id = account.account_id
       AND membership.role = 'student'
       AND ple_data.course_membership_is_active(membership.membership_id)
      JOIN ple_data.course_instance AS course ON course.course_id = membership.course_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_long_name, course.public_reference
$$;

CREATE FUNCTION ple_api.list_pending_student_course_invitations()
RETURNS TABLE(course_public_reference text, course_short_name text, course_long_name text)
LANGUAGE sql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT course.public_reference, course.course_short_name, course.course_long_name
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN LATERAL (
          SELECT DISTINCT ON (invitation.course_id) invitation.course_id
            FROM ple_private.course_invitation AS invitation
           WHERE invitation.target_account_id = account.account_id
             AND invitation.membership_role = 'student'
             AND invitation.expires_at > pg_catalog.clock_timestamp()
             AND NOT EXISTS (
                 SELECT 1 FROM ple_private.course_invitation_event AS event
                  WHERE event.invitation_id = invitation.invitation_id
             )
             AND NOT EXISTS (
                 SELECT 1 FROM ple_data.course_membership AS membership
                  WHERE membership.course_id = invitation.course_id
                    AND membership.account_id = account.account_id
                    AND membership.role = 'student'
                    AND ple_data.course_membership_is_active(membership.membership_id)
             )
           ORDER BY invitation.course_id, invitation.issued_at DESC, invitation.invitation_id DESC
      ) AS pending_invitation ON true
      JOIN ple_data.course_instance AS course
        ON course.course_id = pending_invitation.course_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_long_name, course.public_reference
$$;

CREATE FUNCTION ple_api.export_pending_course_invitations(
    p_course_public_reference text
)
RETURNS TABLE(roster_email text, roster_id text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_course_id uuid;
BEGIN
    IF p_course_public_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.public_reference = p_course_public_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;
    RETURN QUERY
    -- Deliberate disclosure: delivery email is read only for a still-pending
    -- invitation export, after exact Instructor Course authorization.  The
    -- Course roster itself stores no duplicate email and never projects one.
    SELECT ple_private.pending_course_invitation_delivery_email(
               profile.course_id, profile.student_account_id
           ), profile.roster_id
      FROM ple_private.course_roster_profile AS profile
      JOIN LATERAL (
          SELECT invitation.invitation_id
            FROM ple_private.course_invitation AS invitation
           WHERE invitation.course_id = profile.course_id
             AND invitation.target_account_id = profile.student_account_id
             AND invitation.membership_role = 'student'
             AND invitation.expires_at > pg_catalog.clock_timestamp()
             AND NOT EXISTS (
                 SELECT 1 FROM ple_private.course_invitation_event AS event
                  WHERE event.invitation_id = invitation.invitation_id
             )
           ORDER BY invitation.issued_at DESC, invitation.invitation_id DESC
           LIMIT 1
      ) AS pending_invitation ON true
     WHERE profile.course_id = v_course_id
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_id = profile.course_id
              AND membership.account_id = profile.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.membership_id)
       )
     ORDER BY profile.roster_id;
END
$$;

CREATE FUNCTION ple_api.load_invitation_export_course(
    p_course_public_reference text
)
RETURNS TABLE(course_name text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_course_id uuid;
BEGIN
    IF p_course_public_reference IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;
    SELECT course.course_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.public_reference = p_course_public_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;
    RETURN QUERY
    SELECT course.course_long_name
      FROM ple_data.course_instance AS course
     WHERE course.course_id = v_course_id;
END
$$;

CREATE FUNCTION ple_api.import_course_roster(
    p_reference text, p_normalized text[], p_delivery text[], p_roster text[], p_names text[]
)
RETURNS TABLE(roster_id text, roster_name text, state text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course uuid; actor uuid; student uuid; item integer; now_at timestamptz;
    retention_state text;
BEGIN
    IF p_reference IS NULL OR coalesce(cardinality(p_normalized), 0) NOT BETWEEN 1 AND 50
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_delivery)
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_roster)
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_names) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import arguments are invalid';
    END IF;
    SELECT course_id INTO course FROM ple_data.course_instance WHERE public_reference = p_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Roster Import requires a current Instructor Course Membership';
    END IF;
    actor := ple_api.current_session_account_id();
    -- ASVS 8.2.2/15.4.2/15.4.3: authorize before taking an untrusted target
    -- lock, then serialize Course-local creation with the Course-first purge.
    SELECT retention_lifecycle_state INTO retention_state
      FROM ple_data.course_instance WHERE course_id = course FOR UPDATE;
    IF NOT FOUND OR retention_state IS DISTINCT FROM 'active'
       OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Roster Import requires an available Instructor Course Membership';
    END IF;
    now_at := pg_catalog.transaction_timestamp();
    -- Validate the complete reviewed batch before any Account resolution.
    FOR item IN 1..cardinality(p_normalized) LOOP
        IF p_normalized[item] IS NULL OR p_delivery[item] IS NULL OR p_roster[item] IS NULL
           OR char_length(p_roster[item]) NOT BETWEEN 1 AND 64
           OR p_roster[item] !~ '^[A-Za-z0-9._-]+$'
           OR p_names[item] IS NULL OR char_length(p_names[item]) NOT BETWEEN 1 AND 200
           OR p_names[item] <> btrim(p_names[item],
               U&'\0009\000A\000B\000C\000D\0020\0085\00A0\1680\2000\2001\2002\2003\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000')
           OR p_names[item] ~ U&'[\0001-\001F\007F-\009F]' THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import row is invalid';
        END IF;
    END LOOP;
    IF (SELECT count(DISTINCT value) FROM unnest(p_normalized) AS value) <> cardinality(p_normalized)
       OR (SELECT count(DISTINCT value) FROM unnest(p_roster) AS value) <> cardinality(p_roster) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import row is repeated';
    END IF;
    FOR item IN 1..cardinality(p_normalized) LOOP
        student := ple_private.resolve_or_create_student_account(p_normalized[item], p_delivery[item]);
        -- The authentication email is resolved exactly once in the global
        -- Account boundary.  A Course needs only its local roster identifier.
        IF EXISTS (SELECT 1 FROM ple_private.course_roster_profile AS profile
            WHERE profile.course_id = course AND profile.student_account_id = student
              AND profile.roster_id <> p_roster[item]) THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import identity does not match';
        END IF;
        INSERT INTO ple_private.course_roster_profile
            (course_roster_profile_id, course_id, student_account_id, roster_id, roster_name, created_at)
        VALUES (pg_catalog.gen_random_uuid(), course, student, p_roster[item], p_names[item], now_at)
        ON CONFLICT (course_id, student_account_id) DO NOTHING;
        UPDATE ple_private.course_roster_profile AS profile SET roster_name = p_names[item]
         WHERE profile.course_id = course AND profile.student_account_id = student
           AND profile.roster_name IS DISTINCT FROM p_names[item];
        SELECT profile.roster_id, profile.roster_name INTO roster_id, roster_name
          FROM ple_private.course_roster_profile AS profile
         WHERE profile.course_id = course AND profile.student_account_id = student;
        IF EXISTS (
            SELECT 1 FROM ple_data.course_membership AS membership
             WHERE membership.course_id = course AND membership.account_id = student
               AND membership.role = 'student' AND ple_data.course_membership_is_active(membership.membership_id)
        ) THEN
            state := 'active_student';
            RETURN NEXT; CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM ple_private.course_invitation AS invitation
             WHERE invitation.course_id = course AND invitation.target_account_id = student
               AND invitation.membership_role = 'student'
               AND invitation.expires_at > pg_catalog.clock_timestamp()
               AND NOT EXISTS (
                   SELECT 1 FROM ple_private.course_invitation_event AS event
                    WHERE event.invitation_id = invitation.invitation_id
               )
        ) THEN
            INSERT INTO ple_private.course_invitation (
                invitation_id, course_id, target_account_id, membership_role,
                inviting_instructor_account_id, inviting_instructor_role, issued_at, expires_at
            ) VALUES (
                pg_catalog.gen_random_uuid(), course, student, 'student', actor, 'instructor',
                now_at, now_at + interval '7 days'
            );
            PERFORM ple_audit.record_course_roster_event(course, student, actor, 'invitation_created');
        END IF;
        state := 'invitation_pending';
        RETURN NEXT;
    END LOOP;
END
$$;

CREATE FUNCTION ple_api.claim_course_invitation(
    p_student_record uuid, p_membership uuid, p_event uuid, p_reference text
)
RETURNS TABLE(active_student_membership boolean)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course uuid; student uuid; invitation uuid; inviter uuid; existing_record uuid; now_at timestamptz;
    retention_state text;
BEGIN
    student := ple_api.current_session_account_id();
    SELECT course_id INTO course FROM ple_data.course_instance WHERE public_reference = p_reference;
    IF NOT FOUND OR student IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    -- ASVS 8.2.2: establish target-bound authority before locking a Course
    -- supplied by the caller. Revalidate the owning path after the root lock.
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.course_membership
         WHERE course_id = course AND account_id = student AND role = 'student'
           AND ple_data.course_membership_is_active(membership_id)
    ) AND NOT EXISTS (
        SELECT 1 FROM ple_private.course_invitation
         WHERE course_id = course AND target_account_id = student AND membership_role = 'student'
           AND expires_at > pg_catalog.clock_timestamp()
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.course_invitation_event AS event
                WHERE event.invitation_id = course_invitation.invitation_id
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    -- ASVS 15.4.2/15.4.3: Course -> Invitation/Student children matches purge;
    -- require ordinary retention access before the active-membership fast path.
    SELECT retention_lifecycle_state INTO retention_state
      FROM ple_data.course_instance WHERE course_id = course FOR UPDATE;
    IF NOT FOUND OR retention_state <> 'active' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.course_membership
         WHERE course_id = course AND account_id = student AND role = 'student'
           AND ple_data.course_membership_is_active(membership_id)
    ) THEN
        RETURN QUERY SELECT true;
        RETURN;
    END IF;
    SELECT invitation_id, inviting_instructor_account_id INTO invitation, inviter
      FROM ple_private.course_invitation
     WHERE course_id = course AND target_account_id = student AND membership_role = 'student'
       AND expires_at > pg_catalog.clock_timestamp()
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.invitation_id = course_invitation.invitation_id
       )
     ORDER BY issued_at DESC LIMIT 1 FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    now_at := pg_catalog.transaction_timestamp();
    -- ASVS 4.2.1: only the authenticated invitation target can reach this
    -- one-time default. A prior Student choice clears the pending flag and wins.
    PERFORM ple_private.apply_student_invitation_time_zone_default(student, inviter);
    INSERT INTO ple_data.student_record VALUES (p_student_record, course, student, now_at)
    ON CONFLICT (course_id, student_account_id) DO NOTHING;
    SELECT student_record_id INTO existing_record FROM ple_data.student_record
     WHERE course_id = course AND student_account_id = student;
    INSERT INTO ple_data.course_membership
    VALUES (p_membership, course, student, 'student', existing_record, now_at);
    INSERT INTO ple_private.course_invitation_event
    VALUES (p_event, invitation, 'accepted', student, now_at, 'student accepted Course Invitation');
    PERFORM ple_audit.record_course_roster_event(course, student, student, 'invitation_claimed');
    RETURN QUERY SELECT true;
END
$$;

CREATE FUNCTION ple_api.revoke_course_roster_entry(
    p_event uuid, p_reference text, p_roster_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course uuid; student uuid; membership uuid; invitation uuid; actor uuid; now_at timestamptz;
BEGIN
    SELECT course_id INTO course FROM ple_data.course_instance WHERE public_reference = p_reference;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    -- ASVS 15.4.2/15.4.3: serialize with claim and purge at the Course root
    -- before reading membership or pending-event state. An Invitation lock
    -- alone can wait after those reads and leave their eligibility stale.
    PERFORM 1 FROM ple_data.course_instance WHERE course_id = course FOR UPDATE;
    -- ASVS 8.2.2: authorization may change while the Course lock waits.
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    SELECT student_account_id INTO student FROM ple_private.course_roster_profile
     WHERE course_id = course AND roster_id = p_roster_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    -- A transaction may have begun before the claim it waited behind; date
    -- the revocation after serialization, not before that membership began.
    actor := ple_api.current_session_account_id(); now_at := pg_catalog.clock_timestamp();
    SELECT membership_id INTO membership FROM ple_data.course_membership
     WHERE course_id = course AND account_id = student AND role = 'student'
       AND ple_data.course_membership_is_active(membership_id) LIMIT 1;
    IF FOUND THEN
        INSERT INTO ple_data.course_membership_event
        VALUES (p_event, membership, 'ended', now_at, 'Instructor revoked Student course access');
        PERFORM ple_audit.record_course_roster_event(course, student, actor, 'student_access_revoked');
        RETURN;
    END IF;
    SELECT invitation_id INTO invitation FROM ple_private.course_invitation
     WHERE course_id = course AND target_account_id = student AND membership_role = 'student'
       AND expires_at > now_at
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.invitation_id = course_invitation.invitation_id
       )
     ORDER BY issued_at DESC LIMIT 1 FOR UPDATE;
    IF FOUND THEN
        INSERT INTO ple_private.course_invitation_event
        VALUES (p_event, invitation, 'revoked', actor, now_at, 'Instructor revoked pending Course Invitation');
        PERFORM ple_audit.record_course_roster_event(course, student, actor, 'student_access_revoked');
    END IF;
END
$$;

REVOKE ALL ON FUNCTION ple_api.read_course_theme(uuid), ple_api.update_course_theme(uuid, text),
    ple_api.resolve_course_navigation(text), ple_api.read_course_summary(uuid),
    ple_api.create_course_instance(uuid, uuid, uuid, uuid, text, text, bigint, text, text, date, date, text, jsonb, uuid, uuid, uuid, uuid, text[]),
    ple_api.add_course_instructor(uuid, text, text),
    ple_api.list_course_instances(), ple_api.load_course_instance(text),
    ple_api.list_course_creation_instructors(), ple_api.list_course_roster(text),
    ple_api.read_course_roster_entry_repair_support(uuid, text, text),
    ple_api.list_live_student_course_landing(),
    ple_api.list_pending_student_course_invitations(),
    ple_api.export_pending_course_invitations(text),
    ple_api.load_invitation_export_course(text),
    ple_api.import_course_roster(text, text[], text[], text[], text[]),
    ple_api.claim_course_invitation(uuid, uuid, uuid, text),
    ple_api.revoke_course_roster_entry(uuid, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_course_theme(uuid), ple_api.update_course_theme(uuid, text),
    ple_api.resolve_course_navigation(text), ple_api.read_course_summary(uuid),
    ple_api.create_course_instance(uuid, uuid, uuid, uuid, text, text, bigint, text, text, date, date, text, jsonb, uuid, uuid, uuid, uuid, text[]),
    ple_api.add_course_instructor(uuid, text, text),
    ple_api.list_course_instances(), ple_api.load_course_instance(text),
    ple_api.list_course_creation_instructors(), ple_api.list_course_roster(text),
    ple_api.read_course_roster_entry_repair_support(uuid, text, text),
    ple_api.list_live_student_course_landing(),
    ple_api.list_pending_student_course_invitations(),
    ple_api.export_pending_course_invitations(text),
    ple_api.load_invitation_export_course(text),
    ple_api.import_course_roster(text, text[], text[], text[], text[]),
    ple_api.claim_course_invitation(uuid, uuid, uuid, text),
    ple_api.revoke_course_roster_entry(uuid, text, text) TO ple_app;

RESET ROLE;
