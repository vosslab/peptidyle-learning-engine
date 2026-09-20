-- Functions, triggers, and views from course_operations.sql.

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.read_course_theme(p_course_instance_id text)
RETURNS TABLE(course_theme text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course_theme_id AS course_theme FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id AND ple_api.current_session_account_is_course_member(p_course_instance_id)
$$;

CREATE FUNCTION ple_api.update_course_theme(p_course_instance_id text, p_course_theme text)
RETURNS TABLE(course_theme text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    UPDATE ple_data.course_instance SET course_theme_id = p_course_theme
     WHERE course_instance_id = p_course_instance_id AND ple_api.current_session_account_is_course_instructor(p_course_instance_id)
 RETURNING course_theme_id AS course_theme
$$;

CREATE FUNCTION ple_api.resolve_course_navigation(p_course_instance_id text)
RETURNS TABLE(course_instance_id text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data
AS $$
    SELECT course_instance_id FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_member(course_instance_id)
$$;

CREATE FUNCTION ple_api.read_course_summary(p_course_instance_id text)
RETURNS TABLE(course_instance_id text, short_name text, long_name text,
              term_starts_on date, term_ends_on date, membership_role text,
              content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, tags text[])
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_instance_id, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, membership.role,
           course.content_discipline_id, course.content_subject_id, course.content_topic_id, course.content_subtopic_id, course.tags
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_instance_id = course.course_instance_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND ple_data.course_membership_is_active(membership.course_membership_id)
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_member(p_course_instance_id)
$$;

CREATE FUNCTION ple_api.create_course_instance(
    p_course_instance_id text, p_origin_id uuid, p_membership_id uuid, p_event_id uuid,
    p_source_kind text, p_blueprint_course_id text, p_blueprint_revision_number bigint,
    p_short_name text, p_long_name text, p_term_start date, p_term_end date,
    p_assigned_instructor_account_id text, p_assessments jsonb,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE(course_instance_id text, short_name text, long_name text, term_starts_on date,
              term_ends_on date, content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid,
              content_subtopic_id uuid, tags text[], course_edit_number bigint, course_lifecycle_state text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE
    actor text;
    assigned text;
    now_at timestamptz;
    blueprint ple_data.blueprint_course%ROWTYPE;
    expected_sources uuid[];
    supplied_sources uuid[];
BEGIN
    IF p_course_instance_id IS NULL OR p_origin_id IS NULL OR p_membership_id IS NULL OR p_event_id IS NULL
       OR p_source_kind IS NULL OR p_source_kind NOT IN ('empty', 'adopted')
       OR (p_source_kind = 'empty'
           AND (p_blueprint_course_id IS NOT NULL OR p_blueprint_revision_number IS NOT NULL))
       OR (p_source_kind = 'adopted'
           AND (p_blueprint_course_id IS NULL OR p_blueprint_revision_number IS NULL
                OR p_blueprint_revision_number <= 0))
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
        IF p_assigned_instructor_account_id IS NOT NULL AND NOT EXISTS (
            SELECT 1 FROM ple_private.account AS account
             WHERE account.account_id = actor
               AND account.account_id = p_assigned_instructor_account_id
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '42501',
                MESSAGE = 'an Instructor may create a Course Instance only for self';
        END IF;
    ELSIF ple_api.current_session_account_is_sysadmin() THEN
        SELECT account.account_id INTO assigned FROM ple_private.account AS account
         WHERE account.account_id = p_assigned_instructor_account_id
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
         WHERE source_blueprint.blueprint_course_id = p_blueprint_course_id
         FOR UPDATE OF source_blueprint;
        IF NOT FOUND OR blueprint.availability <> 'public' THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Instance source is unavailable';
        END IF;
        IF blueprint.current_blueprint_revision_number <> p_blueprint_revision_number THEN
            RAISE EXCEPTION USING ERRCODE = '40001',
                MESSAGE = 'Blueprint Revision precondition is stale';
        END IF;
        SELECT array_agg(source.blueprint_assessment_id ORDER BY source.blueprint_assessment_id)
          INTO expected_sources FROM ple_data.blueprint_revision_assessment AS source
         WHERE source.blueprint_course_id = blueprint.blueprint_course_id
           AND source.blueprint_revision_number = p_blueprint_revision_number;
        SELECT array_agg((item ->> 'source')::uuid ORDER BY (item ->> 'source')::uuid)
          INTO supplied_sources FROM jsonb_array_elements(p_assessments) AS item;
        IF expected_sources IS NULL OR supplied_sources IS DISTINCT FROM expected_sources THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Blueprint adoption requires every Assessment from its exact Revision';
        END IF;
    END IF;
    -- Adoption may preserve only its locked source Blueprint's exact retired
    -- Discipline. Every empty Course and every caller-selected replacement
    -- remains an active-only choice.
    IF p_source_kind <> 'adopted'
       OR p_discipline IS DISTINCT FROM blueprint.content_discipline_id
       OR NOT EXISTS (
           SELECT 1 FROM ple_api.get_content_discipline(blueprint.content_discipline_id) AS discipline
            WHERE discipline.content_discipline_id = blueprint.content_discipline_id
              AND discipline.is_retired
       ) THEN
        PERFORM ple_api.require_active_content_discipline(p_discipline);
    END IF;
    INSERT INTO ple_data.course_instance (
        course_instance_id, source_kind, blueprint_course_id, blueprint_revision_number,
        course_short_name, course_long_name,
        term_starts_on, term_ends_on, created_at, active_until_at, retention_starts_at,
        content_discipline_id, content_subject_id, content_topic_id, content_subtopic_id, tags
    ) VALUES (
        p_course_instance_id, p_source_kind, blueprint.blueprint_course_id, p_blueprint_revision_number,
        p_short_name, p_long_name, p_term_start, p_term_end, now_at,
        ((now_at AT TIME ZONE 'UTC') + INTERVAL '6 months') AT TIME ZONE 'UTC',
        ((now_at AT TIME ZONE 'UTC') + INTERVAL '6 months') AT TIME ZONE 'UTC',
        p_discipline, p_subject, p_topic, p_subtopic, p_tags
    );
    INSERT INTO ple_data.course_origin (
        course_origin_id, course_instance_id, source_kind, blueprint_course_id,
        blueprint_revision_number, source_course_instance_id, created_at
    ) VALUES (p_origin_id, p_course_instance_id, p_source_kind, blueprint.blueprint_course_id,
              p_blueprint_revision_number, NULL, now_at);
    INSERT INTO ple_data.course_membership (course_membership_id, course_instance_id, account_id, role, joined_at)
    VALUES (p_membership_id, p_course_instance_id, assigned, 'instructor', now_at);
    IF p_source_kind = 'adopted' THEN
        PERFORM ple_data.initialize_course_assessments(
            p_course_instance_id, blueprint.blueprint_course_id, p_blueprint_revision_number, p_assessments
        );
    END IF;
    PERFORM ple_audit.record_course_instance_creation_event(
        p_event_id, p_course_instance_id, p_source_kind,
        blueprint.blueprint_course_id, p_blueprint_revision_number, assigned, actor, now_at
    );
    RETURN QUERY SELECT course.course_instance_id, course.course_short_name, course.course_long_name,
        course.term_starts_on, course.term_ends_on, course.content_discipline_id, course.content_subject_id,
        course.content_topic_id, course.content_subtopic_id, course.tags, course.course_edit_number,
        course.course_lifecycle_state
        FROM ple_data.course_instance AS course WHERE course.course_instance_id = p_course_instance_id;
END
$$;



-- A current Instructor may extend only that Course's teaching team.  The
-- assigned Instructor is historical accountability, never an authority rank.
CREATE FUNCTION ple_api.add_course_instructor(
    p_membership_id uuid, p_course_instance_id text, p_instructor_account_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    course text;
    instructor text;
BEGIN
    IF p_membership_id IS NULL OR p_course_instance_id IS NULL OR p_instructor_account_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Course Instructor membership arguments are invalid';
    END IF;
    SELECT course_instance_id INTO course
      FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id;
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
     WHERE account.account_id = p_instructor_account_id
       AND account.product_role = 'instructor';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'the requested Instructor is unavailable';
    END IF;
    INSERT INTO ple_data.course_membership (
        course_membership_id, course_instance_id, account_id, role, joined_at
    ) VALUES (
        p_membership_id, course, instructor, 'instructor', pg_catalog.transaction_timestamp()
    );
END
$$;

CREATE FUNCTION ple_api.list_course_instances()
RETURNS TABLE(course_instance_id text, short_name text, long_name text, term_starts_on date,
              term_ends_on date, course_theme text, content_discipline_id uuid, content_subject_id uuid,
              content_topic_id uuid, content_subtopic_id uuid, tags text[], course_edit_number bigint,
              course_lifecycle_state text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_instance_id, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, course.course_theme_id AS course_theme,
           course.content_discipline_id, course.content_subject_id, course.content_topic_id,
           course.content_subtopic_id, course.tags, course.course_edit_number, course.course_lifecycle_state
      FROM ple_data.course_instance AS course
      JOIN ple_data.course_membership AS membership
        ON membership.course_instance_id = course.course_instance_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.course_membership_id)
     WHERE ple_api.current_session_account_is_instructor()
     ORDER BY course.course_long_name, course.course_instance_id
$$;

CREATE FUNCTION ple_api.load_course_instance(p_course_instance_id text)
RETURNS TABLE(course_instance_id text, short_name text, long_name text, term_starts_on date,
              term_ends_on date, course_theme text,
              active_instructor_count bigint, blueprint_course_id text,
              adopted_blueprint_revision_number bigint, current_blueprint_revision_number bigint,
              content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
              tags text[], course_edit_number bigint, course_lifecycle_state text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT course.course_instance_id, course.course_short_name, course.course_long_name,
           course.term_starts_on, course.term_ends_on, course.course_theme_id AS course_theme,
           (SELECT count(*) FROM ple_data.course_membership AS teammate
             WHERE teammate.course_instance_id = course.course_instance_id AND teammate.role = 'instructor'
               AND ple_data.course_membership_is_active(teammate.course_membership_id)),
           readable_blueprint.blueprint_course_id,
           CASE WHEN readable_blueprint.blueprint_course_id IS NOT NULL
                THEN course.blueprint_revision_number END,
           readable_blueprint.current_blueprint_revision_number,
           course.content_discipline_id, course.content_subject_id, course.content_topic_id,
           course.content_subtopic_id, course.tags, course.course_edit_number, course.course_lifecycle_state
      FROM ple_data.course_instance AS course
      LEFT JOIN ple_data.blueprint_course AS parent_blueprint
        ON parent_blueprint.blueprint_course_id = course.blueprint_course_id
      -- ASVS 8.2.3 and 8.3.1: Course access never grants source metadata access;
      -- reuse the Blueprint reader's current actor and lifecycle capability.
      LEFT JOIN LATERAL ple_api.load_blueprint_course(parent_blueprint.blueprint_course_id)
        AS readable_blueprint ON true
      JOIN ple_data.course_membership AS membership
        ON membership.course_instance_id = course.course_instance_id
       AND membership.account_id = ple_api.current_session_account_id()
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.course_membership_id)
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_instructor()
$$;

CREATE FUNCTION ple_api.update_course_classification(
    p_course_instance_id text, p_expected_course_edit_number bigint,
    p_discipline uuid, p_subject uuid, p_topic uuid, p_subtopic uuid, p_tags text[]
)
RETURNS TABLE(course_edit_number bigint, changed boolean)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE
    v_course ple_data.course_instance%ROWTYPE;
    v_next_course_edit_number bigint;
BEGIN
    -- ASVS 8.2.1/8.2.2: every current co-Instructor has equal scoped authority.
    SELECT course.* INTO v_course FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Course Instance is not available' USING ERRCODE = '42501';
    END IF;
    -- ASVS 2.3.3: only classification writes advance this bounded CAS validator.
    IF p_expected_course_edit_number IS DISTINCT FROM v_course.course_edit_number THEN
        RAISE EXCEPTION 'Course metadata ETag is stale' USING ERRCODE = '40001';
    END IF;
    IF ROW(v_course.content_discipline_id, v_course.content_subject_id, v_course.content_topic_id,
           v_course.content_subtopic_id, v_course.tags)
       IS NOT DISTINCT FROM ROW(p_discipline, p_subject, p_topic, p_subtopic, p_tags) THEN
        RETURN QUERY SELECT v_course.course_edit_number, false;
        RETURN;
    END IF;
    v_next_course_edit_number := v_course.course_edit_number + 1;
    -- An existing retired Discipline remains valid when this update retains
    -- its UUID; only a replacement needs an active new selection.
    IF v_course.content_discipline_id IS DISTINCT FROM p_discipline THEN
        PERFORM ple_api.require_active_content_discipline(p_discipline);
    END IF;
    UPDATE ple_data.course_instance AS course SET
        content_discipline_id = p_discipline, content_subject_id = p_subject, content_topic_id = p_topic,
        content_subtopic_id = p_subtopic, tags = p_tags, course_edit_number = v_next_course_edit_number
     WHERE course.course_instance_id = v_course.course_instance_id;
    RETURN QUERY SELECT v_next_course_edit_number, true;
END
$$;

CREATE FUNCTION ple_api.list_course_creation_instructors()
RETURNS TABLE(account_id text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT account_id FROM ple_private.account
     WHERE product_role = 'instructor' AND ple_api.current_session_account_is_sysadmin()
     ORDER BY account_id
$$;

CREATE FUNCTION ple_api.list_course_roster(p_course_instance_id text)
RETURNS TABLE(roster_id text, roster_name text, state text)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE selected_course text; retention_state text;
BEGIN
    SELECT course_instance_id INTO selected_course FROM ple_data.course_instance
     WHERE course_instance_id = p_course_instance_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(selected_course) THEN
        RETURN;
    END IF;
    SELECT retention_lifecycle_state INTO retention_state FROM ple_data.course_instance
     WHERE course_instance_id = selected_course FOR UPDATE;
    IF retention_state IS DISTINCT FROM 'active'
       OR NOT ple_api.current_session_account_is_course_instructor(selected_course) THEN
        RETURN;
    END IF;
    RETURN QUERY SELECT profile.roster_id, profile.roster_name,
           CASE WHEN EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_instance_id = course.course_instance_id
                  AND membership.account_id = profile.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.course_membership_id)
           ) THEN 'active_student' ELSE 'invitation_pending' END
      FROM ple_data.course_instance AS course
      JOIN ple_private.course_roster_profile AS profile ON profile.course_instance_id = course.course_instance_id
     WHERE course.course_instance_id = p_course_instance_id
       AND (
           EXISTS (
               SELECT 1 FROM ple_data.course_membership AS membership
                WHERE membership.course_instance_id = course.course_instance_id
                  AND membership.account_id = profile.student_account_id
                  AND membership.role = 'student'
                  AND ple_data.course_membership_is_active(membership.course_membership_id)
           )
           OR EXISTS (
               SELECT 1 FROM ple_private.course_invitation AS invitation
                WHERE invitation.course_instance_id = course.course_instance_id
                  AND invitation.target_account_id = profile.student_account_id
                  AND invitation.membership_role = 'student'
                  AND invitation.expires_at > pg_catalog.clock_timestamp()
                  AND NOT EXISTS (
                      SELECT 1 FROM ple_private.course_invitation_event AS event
                       WHERE event.course_invitation_id = invitation.course_invitation_id
                  )
           )
       )
       AND ple_api.current_session_account_is_course_instructor(course.course_instance_id)
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
    p_capability_id uuid, p_course_instance_id text, p_roster_id text
)
RETURNS TABLE(roster_id text, state text)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_audit, ple_data, ple_private AS $$
DECLARE course text; student text; canonical_resource_path text;
BEGIN
    IF p_capability_id IS NULL
       OR p_course_instance_id IS NULL
       OR p_roster_id IS NULL OR p_roster_id !~ '^[A-Za-z0-9._-]+$'
       OR NOT ple_api.current_session_account_has_platform_administration() THEN
        RETURN;
    END IF;
    SELECT course_instance.course_instance_id INTO course
      FROM ple_data.course_instance AS course_instance
     WHERE course_instance.course_instance_id = p_course_instance_id;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    SELECT profile.student_account_id INTO student
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_instance_id = course
       AND profile.roster_id = p_roster_id;
    IF NOT FOUND THEN
        RETURN;
    END IF;
    canonical_resource_path := format(
        'course-instance/%s/roster/%s', p_course_instance_id, p_roster_id
    );
    PERFORM ple_api.record_support_repair_capability_use(
        p_capability_id, 'student', canonical_resource_path
    );
    IF NOT FOUND THEN
        RETURN;
    END IF;
    RETURN QUERY
    SELECT profile.roster_id,
           CASE
               WHEN EXISTS (
                   SELECT 1 FROM ple_data.course_membership AS membership
                    WHERE membership.course_instance_id = course
                      AND membership.account_id = student
                      AND membership.role = 'student'
                      AND ple_data.course_membership_is_active(membership.course_membership_id)
               ) THEN 'active_student'
               WHEN EXISTS (
                   SELECT 1 FROM ple_private.course_invitation AS invitation
                    WHERE invitation.course_instance_id = course
                      AND invitation.target_account_id = student
                      AND invitation.membership_role = 'student'
                      AND invitation.expires_at > pg_catalog.clock_timestamp()
                      AND NOT EXISTS (
                          SELECT 1 FROM ple_private.course_invitation_event AS event
                           WHERE event.course_invitation_id = invitation.course_invitation_id
                      )
               ) THEN 'invitation_pending'
               ELSE 'removed'
           END
      FROM ple_private.course_roster_profile AS profile
     WHERE profile.course_instance_id = course
       AND profile.student_account_id = student
       AND profile.roster_id = p_roster_id;
END
$$;

CREATE FUNCTION ple_api.list_live_student_course_landing()
RETURNS TABLE(course_instance_id text, course_short_name text, course_long_name text)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT course.course_instance_id, course.course_short_name, course.course_long_name
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
       AND ple_data.course_membership_is_active(membership.course_membership_id)
      JOIN ple_data.course_instance AS course ON course.course_instance_id = membership.course_instance_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_long_name, course.course_instance_id
$$;

CREATE FUNCTION ple_api.list_pending_student_course_invitations()
RETURNS TABLE(course_instance_id text, course_short_name text, course_long_name text,
              instructor_display_name text, term_starts_on date, term_ends_on date)
LANGUAGE sql VOLATILE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    -- ASVS 8.2.2/8.2.3/14.2.6: self-only invitations reveal the assigned
    -- Instructor's verified display name and Course term, never roster identities.
    SELECT course.course_instance_id, course.course_short_name, course.course_long_name,
           ple_private.verified_instructor_display_name(
               pending_invitation.inviting_instructor_account_id
           ),
           course.term_starts_on, course.term_ends_on
      FROM ple_private.account AS account
      JOIN LATERAL (
          SELECT event.state
            FROM ple_private.account_state_event AS event
           WHERE event.account_id = account.account_id
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS account_state ON account_state.state = 'active'
      JOIN LATERAL (
          SELECT DISTINCT ON (invitation.course_instance_id)
                 invitation.course_instance_id,
                 invitation.inviting_instructor_account_id
            FROM ple_private.course_invitation AS invitation
           WHERE invitation.target_account_id = account.account_id
             AND invitation.membership_role = 'student'
             AND invitation.expires_at > pg_catalog.clock_timestamp()
             AND NOT EXISTS (
                 SELECT 1 FROM ple_private.course_invitation_event AS event
                  WHERE event.course_invitation_id = invitation.course_invitation_id
             )
             AND NOT EXISTS (
                 SELECT 1 FROM ple_data.course_membership AS membership
                  WHERE membership.course_instance_id = invitation.course_instance_id
                    AND membership.account_id = account.account_id
                    AND membership.role = 'student'
                    AND ple_data.course_membership_is_active(membership.course_membership_id)
             )
           ORDER BY invitation.course_instance_id, invitation.issued_at DESC, invitation.course_invitation_id DESC
      ) AS pending_invitation ON true
      JOIN ple_data.course_instance AS course
        ON course.course_instance_id = pending_invitation.course_instance_id
     WHERE account.account_id = ple_api.current_session_account_id()
       AND account.product_role = 'student'
     ORDER BY course.course_long_name, course.course_instance_id
$$;

CREATE FUNCTION ple_api.export_pending_course_invitations(
    p_course_instance_id text
)
RETURNS TABLE(roster_email text, roster_id text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE v_course_id text;
BEGIN
    IF p_course_instance_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;
    SELECT course.course_instance_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;
    RETURN QUERY
    -- Deliberate disclosure: delivery email is read only for a still-pending
    -- invitation export, after exact Instructor Course authorization.  The
    -- Course roster itself stores no duplicate email and never projects one.
    SELECT ple_private.pending_course_invitation_delivery_email(
               profile.course_instance_id, profile.student_account_id
           ), profile.roster_id
      FROM ple_private.course_roster_profile AS profile
      JOIN LATERAL (
          SELECT invitation.course_invitation_id
            FROM ple_private.course_invitation AS invitation
           WHERE invitation.course_instance_id = profile.course_instance_id
             AND invitation.target_account_id = profile.student_account_id
             AND invitation.membership_role = 'student'
             AND invitation.expires_at > pg_catalog.clock_timestamp()
             AND NOT EXISTS (
                 SELECT 1 FROM ple_private.course_invitation_event AS event
                  WHERE event.course_invitation_id = invitation.course_invitation_id
             )
           ORDER BY invitation.issued_at DESC, invitation.course_invitation_id DESC
           LIMIT 1
      ) AS pending_invitation ON true
     WHERE profile.course_instance_id = v_course_id
       AND NOT EXISTS (
           SELECT 1 FROM ple_data.course_membership AS membership
            WHERE membership.course_instance_id = profile.course_instance_id
              AND membership.account_id = profile.student_account_id
              AND membership.role = 'student'
              AND ple_data.course_membership_is_active(membership.course_membership_id)
       )
     ORDER BY profile.roster_id;
END
$$;

CREATE FUNCTION ple_api.load_invitation_export_course(
    p_course_instance_id text
)
RETURNS TABLE(course_name text)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE v_course_id text;
BEGIN
    IF p_course_instance_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Invitation export arguments are invalid';
    END IF;
    SELECT course.course_instance_id INTO v_course_id
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = p_course_instance_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(v_course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Invitation export requires a current Instructor Course Membership';
    END IF;
    RETURN QUERY
    SELECT course.course_long_name
      FROM ple_data.course_instance AS course
     WHERE course.course_instance_id = v_course_id;
END
$$;

CREATE FUNCTION ple_api.import_course_roster(
    p_course_instance_id text, p_normalized text[], p_delivery text[], p_roster text[], p_names text[]
)
RETURNS TABLE(roster_id text, roster_name text, state text)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course text; actor text; student text; item integer; now_at timestamptz;
    retention_state text;
BEGIN
    IF p_course_instance_id IS NULL OR coalesce(cardinality(p_normalized), 0) NOT BETWEEN 1 AND 50
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_delivery)
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_roster)
       OR cardinality(p_normalized) IS DISTINCT FROM cardinality(p_names) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import arguments are invalid';
    END IF;
    SELECT course_instance_id INTO course FROM ple_data.course_instance WHERE course_instance_id = p_course_instance_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Course Roster Import requires a current Instructor Course Membership';
    END IF;
    actor := ple_api.current_session_account_id();
    -- ASVS 8.2.2/15.4.2/15.4.3: authorize before taking an untrusted target
    -- lock, then serialize Course-local creation with the Course-first purge.
    SELECT retention_lifecycle_state INTO retention_state
      FROM ple_data.course_instance WHERE course_instance_id = course FOR UPDATE;
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
            WHERE profile.course_instance_id = course AND profile.student_account_id = student
              AND profile.roster_id <> p_roster[item]) THEN
            RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Course Roster Import identity does not match';
        END IF;
        INSERT INTO ple_private.course_roster_profile
            (course_roster_profile_id, course_instance_id, student_account_id, roster_id, roster_name, created_at)
        VALUES (pg_catalog.gen_random_uuid(), course, student, p_roster[item], p_names[item], now_at)
        ON CONFLICT (course_instance_id, student_account_id) DO NOTHING;
        UPDATE ple_private.course_roster_profile AS profile SET roster_name = p_names[item]
         WHERE profile.course_instance_id = course AND profile.student_account_id = student
           AND profile.roster_name IS DISTINCT FROM p_names[item];
        SELECT profile.roster_id, profile.roster_name INTO roster_id, roster_name
          FROM ple_private.course_roster_profile AS profile
         WHERE profile.course_instance_id = course AND profile.student_account_id = student;
        IF EXISTS (
            SELECT 1 FROM ple_data.course_membership AS membership
             WHERE membership.course_instance_id = course AND membership.account_id = student
               AND membership.role = 'student' AND ple_data.course_membership_is_active(membership.course_membership_id)
        ) THEN
            state := 'active_student';
            RETURN NEXT; CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM ple_private.course_invitation AS invitation
             WHERE invitation.course_instance_id = course AND invitation.target_account_id = student
               AND invitation.membership_role = 'student'
               AND invitation.expires_at > pg_catalog.clock_timestamp()
               AND NOT EXISTS (
                   SELECT 1 FROM ple_private.course_invitation_event AS event
                    WHERE event.course_invitation_id = invitation.course_invitation_id
               )
        ) THEN
            INSERT INTO ple_private.course_invitation (
                course_invitation_id, course_instance_id, target_account_id, membership_role,
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
    p_student_record uuid, p_membership uuid, p_event uuid, p_course_instance_id text
)
RETURNS TABLE(active_student_membership boolean)
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course text; student text; invitation uuid; inviter text; existing_record uuid; now_at timestamptz;
    retention_state text;
BEGIN
    student := ple_api.current_session_account_id();
    SELECT course_instance_id INTO course FROM ple_data.course_instance WHERE course_instance_id = p_course_instance_id;
    IF NOT FOUND OR student IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    -- ASVS 8.2.2: establish target-bound authority before locking a Course
    -- supplied by the caller. Revalidate the owning path after the root lock.
    IF NOT EXISTS (
        SELECT 1 FROM ple_data.course_membership
         WHERE course_instance_id = course AND account_id = student AND role = 'student'
           AND ple_data.course_membership_is_active(course_membership_id)
    ) AND NOT EXISTS (
        SELECT 1 FROM ple_private.course_invitation
         WHERE course_instance_id = course AND target_account_id = student AND membership_role = 'student'
           AND expires_at > pg_catalog.clock_timestamp()
           AND NOT EXISTS (
               SELECT 1 FROM ple_private.course_invitation_event AS event
                WHERE event.course_invitation_id = course_invitation.course_invitation_id
           )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    -- ASVS 15.4.2/15.4.3: Course -> Invitation/Student children matches purge;
    -- require ordinary retention access before the active-membership fast path.
    SELECT retention_lifecycle_state INTO retention_state
      FROM ple_data.course_instance WHERE course_instance_id = course FOR UPDATE;
    IF NOT FOUND OR retention_state <> 'active' THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course Invitation is unavailable';
    END IF;
    IF EXISTS (
        SELECT 1 FROM ple_data.course_membership
         WHERE course_instance_id = course AND account_id = student AND role = 'student'
           AND ple_data.course_membership_is_active(course_membership_id)
    ) THEN
        RETURN QUERY SELECT true;
        RETURN;
    END IF;
    SELECT course_invitation_id, inviting_instructor_account_id INTO invitation, inviter
      FROM ple_private.course_invitation
     WHERE course_instance_id = course AND target_account_id = student AND membership_role = 'student'
       AND expires_at > pg_catalog.clock_timestamp()
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.course_invitation_id = course_invitation.course_invitation_id
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
    ON CONFLICT (course_instance_id, student_account_id) DO NOTHING;
    SELECT student_record_id INTO existing_record FROM ple_data.student_record
     WHERE course_instance_id = course AND student_account_id = student;
    INSERT INTO ple_data.course_membership
    VALUES (p_membership, course, student, 'student', existing_record, now_at);
    INSERT INTO ple_private.course_invitation_event
    VALUES (p_event, invitation, 'accepted', student, now_at, 'student accepted Course Invitation');
    PERFORM ple_audit.record_course_roster_event(course, student, student, 'invitation_claimed');
    RETURN QUERY SELECT true;
END
$$;

CREATE FUNCTION ple_api.revoke_course_roster_entry(
    p_event uuid, p_course_instance_id text, p_roster_id text
)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private, ple_audit AS $$
DECLARE course text; student text; membership uuid; invitation uuid; actor text; now_at timestamptz;
BEGIN
    SELECT course_instance_id INTO course FROM ple_data.course_instance WHERE course_instance_id = p_course_instance_id;
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    -- ASVS 15.4.2/15.4.3: serialize with claim and purge at the Course root
    -- before reading membership or pending-event state. An Invitation lock
    -- alone can wait after those reads and leave their eligibility stale.
    PERFORM 1 FROM ple_data.course_instance WHERE course_instance_id = course FOR UPDATE;
    -- ASVS 8.2.2: authorization may change while the Course lock waits.
    IF NOT FOUND OR NOT ple_api.current_session_account_is_course_instructor(course) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    SELECT student_account_id INTO student FROM ple_private.course_roster_profile
     WHERE course_instance_id = course AND roster_id = p_roster_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Course roster entry is unavailable';
    END IF;
    -- A transaction may have begun before the claim it waited behind; date
    -- the revocation after serialization, not before that membership began.
    actor := ple_api.current_session_account_id(); now_at := pg_catalog.clock_timestamp();
    SELECT course_membership_id INTO membership FROM ple_data.course_membership
     WHERE course_instance_id = course AND account_id = student AND role = 'student'
       AND ple_data.course_membership_is_active(course_membership_id) LIMIT 1;
    IF FOUND THEN
        INSERT INTO ple_data.course_membership_event
        VALUES (p_event, membership, 'ended', now_at, 'Instructor revoked Student course access');
        PERFORM ple_audit.record_course_roster_event(course, student, actor, 'student_access_revoked');
        RETURN;
    END IF;
    SELECT course_invitation_id INTO invitation FROM ple_private.course_invitation
     WHERE course_instance_id = course AND target_account_id = student AND membership_role = 'student'
       AND expires_at > now_at
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.course_invitation_event AS event
            WHERE event.course_invitation_id = course_invitation.course_invitation_id
       )
     ORDER BY issued_at DESC LIMIT 1 FOR UPDATE;
    IF FOUND THEN
        INSERT INTO ple_private.course_invitation_event
        VALUES (p_event, invitation, 'revoked', actor, now_at, 'Instructor revoked pending Course Invitation');
        PERFORM ple_audit.record_course_roster_event(course, student, actor, 'student_access_revoked');
    END IF;
END
$$;

