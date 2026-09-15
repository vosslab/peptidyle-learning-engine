-- Assessment-owned Question Pool fork commands and Instructor read projection.

SET LOCAL ROLE ple_data_owner;

-- The only import path for an Assessment-owned Pool creates the Entry, fresh
-- child Pool lineage/revision 1, and ownership association together.  It
-- accepts no raw member pins: membership is copied only from the resolved
-- immutable reusable source Revision.
CREATE FUNCTION ple_data.import_assessment_question_pool_fork(
    p_assessment_id uuid,
    p_assessment_entry_id uuid,
    p_expected_assessment_edit_number bigint,
    p_fork_question_pool_id uuid,
    p_fork_public_question_pool_id text,
    p_source_question_pool_id uuid,
    p_source_question_pool_revision_number bigint,
    p_authored_position integer,
    p_selection_count integer,
    p_points_per_item numeric,
    p_selected_question_order text,
    p_scoring_rule text
) RETURNS TABLE (
    assessment_entry_id uuid,
    question_pool_id uuid,
    question_pool_revision_number bigint,
    assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE forked record;
BEGIN
    IF p_assessment_id IS NULL OR p_assessment_entry_id IS NULL
       OR p_expected_assessment_edit_number IS NULL
       OR p_expected_assessment_edit_number <= 0
       OR p_expected_assessment_edit_number >= 9223372036854775807
       OR p_authored_position < 0 OR p_selection_count <= 0 OR p_points_per_item < 0
       OR p_selected_question_order NOT IN ('question_pool_order', 'random_order')
       OR p_scoring_rule NOT IN ('normal', 'full_credit', 'extra_credit', 'excluded') THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment Question Pool import is invalid';
    END IF;
    SELECT * INTO assessment_row FROM ple_data.assessment
     WHERE assessment_id = p_assessment_id FOR UPDATE;
    IF NOT FOUND OR assessment_row.assessment_status <> 'unreleased'
       OR NOT ple_api.current_session_account_is_course_instructor(assessment_row.course_id) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment Question Pool import is unavailable';
    END IF;
    IF assessment_row.assessment_edit_number <> p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Question Pool import is stale';
    END IF;
    SELECT * INTO forked FROM ple_data.fork_question_pool_revision(
        p_fork_question_pool_id, p_fork_public_question_pool_id,
        p_source_question_pool_id, p_source_question_pool_revision_number
    );
    IF p_selection_count > (
        SELECT pool_revision.member_count FROM ple_data.question_pool_revision AS pool_revision
         WHERE pool_revision.question_pool_id = forked.question_pool_id
           AND pool_revision.revision_number = 1
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Assessment Question Pool selection exceeds fork member count';
    END IF;
    INSERT INTO ple_data.assessment_entry(
        assessment_entry_id, assessment_id, authored_position, entry_kind, availability, scoring_rule,
        question_pool_id, question_pool_revision_number, selection_count, points_per_item,
        selected_question_order
    ) VALUES (
        p_assessment_entry_id, p_assessment_id, p_authored_position, 'question_pool', 'available', p_scoring_rule,
        forked.question_pool_id, 1, p_selection_count, p_points_per_item, p_selected_question_order
    );
    INSERT INTO ple_data.assessment_question_pool_fork(
        assessment_entry_id, assessment_id, question_pool_id, origin_question_pool_revision_number
    ) VALUES (p_assessment_entry_id, p_assessment_id, forked.question_pool_id, 1);
    -- Import changes current Assessment content.  It is not a Pool lineage
    -- Revision, but it must invalidate any stale full-Assessment save.
    UPDATE ple_data.assessment AS updated
       SET assessment_edit_number = updated.assessment_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE updated.assessment_id = p_assessment_id
     RETURNING updated.assessment_edit_number INTO assessment_edit_number;
    assessment_entry_id := p_assessment_entry_id;
    question_pool_id := forked.question_pool_id;
    question_pool_revision_number := 1;
    RETURN NEXT;
END
$$;

-- Only an Assessment-owned child Pool may receive a new immutable Revision.
-- The same qualified Assessment edit that changes the Entry advances the Pool
-- Revision, so a caller cannot append a root reusable Pool or a foreign fork.
CREATE FUNCTION ple_data.append_assessment_question_pool_fork_revision(
    p_assessment_id uuid,
    p_assessment_entry_id uuid,
    p_expected_assessment_edit_number bigint,
    p_expected_question_pool_metadata_etag uuid,
    p_member_question_ids text[],
    p_member_revision_numbers integer[],
    p_interchangeability_attested boolean
) RETURNS TABLE (
    assessment_entry_id uuid,
    question_pool_id uuid,
    question_pool_revision_number bigint,
    question_pool_metadata_etag uuid,
    assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assessment_row ple_data.assessment%ROWTYPE;
DECLARE entry_row ple_data.assessment_entry%ROWTYPE;
DECLARE append_result record;
BEGIN
    IF p_assessment_id IS NULL OR p_assessment_entry_id IS NULL
       OR p_expected_assessment_edit_number IS NULL OR p_expected_assessment_edit_number <= 0
       OR p_expected_question_pool_metadata_etag IS NULL
       OR p_member_question_ids IS NULL OR p_member_revision_numbers IS NULL
       OR cardinality(p_member_question_ids) IS NULL
       OR cardinality(p_member_question_ids) = 0
       OR cardinality(p_member_question_ids) > 1024
       OR cardinality(p_member_question_ids) <> cardinality(p_member_revision_numbers)
       OR p_interchangeability_attested IS DISTINCT FROM true THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Question Pool Revision append is invalid';
    END IF;
    SELECT assessment.* INTO assessment_row
      FROM ple_data.assessment AS assessment
     WHERE assessment.assessment_id = p_assessment_id
       AND ple_api.current_session_account_is_course_instructor(assessment.course_id)
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Question Pool Revision append is unavailable';
    END IF;
    IF assessment_row.assessment_edit_number IS DISTINCT FROM p_expected_assessment_edit_number THEN
        RAISE EXCEPTION USING ERRCODE = '40001', MESSAGE = 'Assessment Edit Number is stale';
    END IF;
    SELECT entry.* INTO entry_row
      FROM ple_data.assessment_entry AS entry
      JOIN ple_data.assessment_question_pool_fork AS owned
        ON owned.assessment_entry_id = entry.assessment_entry_id
       AND owned.assessment_id = entry.assessment_id
       AND owned.question_pool_id = entry.question_pool_id
      JOIN ple_data.question_pool AS pool
        ON pool.question_pool_id = entry.question_pool_id
       AND pool.source_question_pool_id IS NOT NULL
       AND pool.source_question_pool_revision_number IS NOT NULL
     WHERE entry.assessment_entry_id = p_assessment_entry_id
       AND entry.assessment_id = p_assessment_id
       AND entry.entry_kind = 'question_pool'
     FOR UPDATE OF entry;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Assessment Question Pool Revision append is unavailable';
    END IF;
    SELECT * INTO append_result FROM ple_data.append_question_pool_revision(
        entry_row.question_pool_id, p_expected_question_pool_metadata_etag,
        p_member_question_ids, p_member_revision_numbers, p_interchangeability_attested
    );
    IF entry_row.selection_count > (
        SELECT pool_revision.member_count FROM ple_data.question_pool_revision AS pool_revision
         WHERE pool_revision.question_pool_id = entry_row.question_pool_id
           AND pool_revision.revision_number = append_result.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Assessment Question Pool selection exceeds new fork member count';
    END IF;
    UPDATE ple_data.assessment_entry AS entry
       SET question_pool_revision_number = append_result.revision_number
     WHERE entry.assessment_entry_id = entry_row.assessment_entry_id
       AND entry.assessment_id = p_assessment_id;
    UPDATE ple_data.assessment AS assessment
       SET assessment_edit_number = assessment.assessment_edit_number + 1,
           updated_at = pg_catalog.clock_timestamp()
     WHERE assessment.assessment_id = p_assessment_id
     RETURNING assessment.assessment_edit_number INTO assessment_edit_number;
    IF assessment_row.assessment_status = 'released' THEN
        PERFORM ple_data.validate_assessment_release(p_assessment_id);
    END IF;
    assessment_entry_id := entry_row.assessment_entry_id;
    question_pool_id := entry_row.question_pool_id;
    question_pool_revision_number := append_result.revision_number;
    question_pool_metadata_etag := append_result.metadata_etag;
    RETURN NEXT;
END
$$;

REVOKE ALL ON FUNCTION
    ple_data.import_assessment_question_pool_fork(uuid, uuid, bigint, uuid, text, uuid, bigint, integer, integer, numeric, text, text),
    ple_data.append_assessment_question_pool_fork_revision(uuid, uuid, bigint, uuid, text[], integer[], boolean)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION
    ple_data.import_assessment_question_pool_fork(uuid, uuid, bigint, uuid, text, uuid, bigint, integer, integer, numeric, text, text),
    ple_data.append_assessment_question_pool_fork_revision(uuid, uuid, bigint, uuid, text[], integer[], boolean)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.import_assessment_question_pool_fork(
    uuid, uuid, bigint, uuid, text, uuid, bigint, integer, integer, numeric, text, text
) RETURNS TABLE (
    assessment_entry_id uuid,
    question_pool_id uuid,
    question_pool_revision_number bigint,
    assessment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.import_assessment_question_pool_fork(
        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12
    )
$$;
REVOKE ALL ON FUNCTION ple_api.import_assessment_question_pool_fork(uuid, uuid, bigint, uuid, text, uuid, bigint, integer, integer, numeric, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.import_assessment_question_pool_fork(uuid, uuid, bigint, uuid, text, uuid, bigint, integer, integer, numeric, text, text) TO ple_app;

-- Public-route wrapper: the application resolves an authorized Course and
-- Assessment by their opaque references inside this definer boundary.  It
-- never receives or accepts an internal Assessment UUID from the browser.
CREATE FUNCTION ple_api.import_assessment_question_pool_fork_for_reference(
    p_course_public_reference text,
    p_assessment_public_reference text,
    p_assessment_entry_id uuid,
    p_expected_assessment_edit_number bigint,
    p_fork_question_pool_id uuid,
    p_fork_public_question_pool_id text,
    p_source_question_pool_id uuid,
    p_source_question_pool_revision_number bigint,
    p_authored_position integer,
    p_selection_count integer,
    p_points_per_item numeric,
    p_selected_question_order text,
    p_scoring_rule text
) RETURNS TABLE (
    assessment_entry_id uuid,
    question_pool_id uuid,
    question_pool_revision_number bigint,
    assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assessment_id_value uuid;
BEGIN
    SELECT assessment.assessment_id INTO assessment_id_value
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id
     WHERE course.public_reference = p_course_public_reference
       AND assessment.public_reference = p_assessment_public_reference
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment Question Pool import is unavailable';
    END IF;
    RETURN QUERY SELECT * FROM ple_data.import_assessment_question_pool_fork(
        assessment_id_value, p_assessment_entry_id, p_expected_assessment_edit_number,
        p_fork_question_pool_id,
        p_fork_public_question_pool_id, p_source_question_pool_id,
        p_source_question_pool_revision_number, p_authored_position,
        p_selection_count, p_points_per_item, p_selected_question_order, p_scoring_rule
    );
END
$$;
REVOKE ALL ON FUNCTION ple_api.import_assessment_question_pool_fork_for_reference(text, text, uuid, bigint, uuid, text, uuid, bigint, integer, integer, numeric, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.import_assessment_question_pool_fork_for_reference(text, text, uuid, bigint, uuid, text, uuid, bigint, integer, integer, numeric, text, text) TO ple_app;

-- ASVS V1.2/V2.2/V8.3: this typed read derives the exact Pool Revision from
-- the Course-owned Assessment Entry under the installed Instructor session.
-- No browser-selected Pool identity or Revision crosses this boundary.
CREATE FUNCTION ple_api.read_assessment_question_pool_fork(
    p_course_reference text,
    p_assessment_reference text,
    p_assessment_entry_id uuid
) RETURNS TABLE (
    assessment_entry_id uuid,
    public_question_pool_id text,
    revision_number bigint,
    pool_metadata_etag uuid,
    selection_count integer,
    member_position integer,
    question_id text,
    question_revision_number integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT entry.assessment_entry_id,
           ple_data.canonical_public_crockford_display(pool.public_question_pool_id),
           entry.question_pool_revision_number,
           pool.metadata_etag,
           entry.selection_count,
           member.member_position,
           ple_data.canonical_public_crockford_display(member.question_id),
           member.question_revision_number
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id
      JOIN ple_data.assessment_entry AS entry ON entry.assessment_id = assessment.assessment_id
      JOIN ple_data.assessment_question_pool_fork AS owned
        ON owned.assessment_entry_id = entry.assessment_entry_id
       AND owned.assessment_id = assessment.assessment_id
       AND owned.question_pool_id = entry.question_pool_id
      JOIN ple_data.question_pool AS pool ON pool.question_pool_id = entry.question_pool_id
      JOIN ple_data.question_pool_revision_member AS member
        ON member.question_pool_id = entry.question_pool_id
       AND member.revision_number = entry.question_pool_revision_number
     WHERE course.public_reference = p_course_reference
       AND assessment.public_reference = p_assessment_reference
       AND entry.assessment_entry_id = p_assessment_entry_id
       AND entry.entry_kind = 'question_pool'
       AND ple_api.current_session_account_is_course_instructor(course.course_id)
     ORDER BY member.member_position
$$;
REVOKE ALL ON FUNCTION ple_api.read_assessment_question_pool_fork(text, text, uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.read_assessment_question_pool_fork(text, text, uuid) TO ple_app;

CREATE FUNCTION ple_api.append_assessment_question_pool_fork_revision(
    uuid, uuid, bigint, uuid, text[], integer[], boolean
) RETURNS TABLE (
    assessment_entry_id uuid, question_pool_id uuid, question_pool_revision_number bigint,
    question_pool_metadata_etag uuid, assessment_edit_number bigint
)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
    SELECT * FROM ple_data.append_assessment_question_pool_fork_revision(
        $1, $2, $3, $4, $5, $6, $7
    )
$$;
REVOKE ALL ON FUNCTION ple_api.append_assessment_question_pool_fork_revision(uuid, uuid, bigint, uuid, text[], integer[], boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.append_assessment_question_pool_fork_revision(uuid, uuid, bigint, uuid, text[], integer[], boolean) TO ple_app;

CREATE FUNCTION ple_api.append_assessment_question_pool_fork_revision_for_reference(
    p_course_public_reference text,
    p_assessment_public_reference text,
    p_assessment_entry_id uuid,
    p_expected_assessment_edit_number bigint,
    p_expected_question_pool_metadata_etag uuid,
    p_member_question_ids text[],
    p_member_revision_numbers integer[],
    p_interchangeability_attested boolean
) RETURNS TABLE (
    assessment_entry_id uuid, question_pool_id uuid, question_pool_revision_number bigint,
    question_pool_metadata_etag uuid, assessment_edit_number bigint
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
DECLARE assessment_id_value uuid;
BEGIN
    SELECT assessment.assessment_id INTO assessment_id_value
      FROM ple_data.course_instance AS course
      JOIN ple_data.assessment AS assessment ON assessment.course_id = course.course_id
     WHERE course.public_reference = p_course_public_reference
       AND assessment.public_reference = p_assessment_public_reference
       AND ple_api.current_session_account_is_course_instructor(course.course_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Assessment Question Pool Revision append is unavailable';
    END IF;
    RETURN QUERY SELECT * FROM ple_data.append_assessment_question_pool_fork_revision(
        assessment_id_value, p_assessment_entry_id, p_expected_assessment_edit_number,
        p_expected_question_pool_metadata_etag, p_member_question_ids,
        p_member_revision_numbers, p_interchangeability_attested
    );
END
$$;
REVOKE ALL ON FUNCTION ple_api.append_assessment_question_pool_fork_revision_for_reference(text, text, uuid, bigint, uuid, text[], integer[], boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.append_assessment_question_pool_fork_revision_for_reference(text, text, uuid, bigint, uuid, text[], integer[], boolean) TO ple_app;

RESET ROLE;
