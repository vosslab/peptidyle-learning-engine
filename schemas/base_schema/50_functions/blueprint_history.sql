-- Functions, triggers, and views from blueprint_history.sql.

SET LOCAL ROLE ple_api_owner;

-- Bounded reads of existing immutable Revision and metadata event facts.
CREATE FUNCTION ple_api.list_blueprint_history(
    p_reference text, p_kind text, p_after text, p_page_size integer
)
RETURNS TABLE (
    continuation_key text, revision_number bigint, recorded_at_ms bigint,
    short_name text, long_name text, availability text,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, tags text[]
)
LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
DECLARE
    v_reference text;
BEGIN
    -- ASVS 2.2.1/2: SQL callers share the trusted boundary's row/key limits.
    IF p_kind NOT IN ('revisions', 'metadata') OR p_kind IS NULL
       OR p_page_size IS NULL OR p_page_size NOT BETWEEN 1 AND 100 THEN
        RAISE EXCEPTION 'invalid Blueprint history page' USING ERRCODE = '22023';
    END IF;
    -- ASVS 8.2.1/2/3, 8.3.1/2: current ordinary visibility governs all history.
    SELECT course.blueprint_course_id INTO v_reference
      FROM ple_data.blueprint_course AS course
     WHERE ple_private.is_canonical_prefixed_public_id(p_reference, 'BP')
       AND course.blueprint_course_id = p_reference
       AND ple_api.current_session_account_is_instructor()
       AND (course.availability IN ('public', 'archived')
            OR course.owner_account_id = ple_api.current_session_account_id());
    IF v_reference IS NULL THEN
        RAISE EXCEPTION 'Blueprint not found' USING ERRCODE = '42501';
    END IF;
    IF p_kind = 'revisions' THEN
        IF p_after IS NOT NULL THEN
            IF p_after !~ '^[1-9][0-9]{0,18}$' THEN
                RAISE EXCEPTION 'invalid Blueprint history page' USING ERRCODE = '22023';
            END IF;
            IF p_after::numeric > 9223372036854775807 THEN
                RAISE EXCEPTION 'invalid Blueprint history page' USING ERRCODE = '22023';
            END IF;
        END IF;
        RETURN QUERY
        SELECT revision.blueprint_revision_number::text, revision.blueprint_revision_number,
               (extract(epoch FROM revision.saved_at) * 1000)::bigint,
               NULL::text, NULL::text, NULL::text,
               NULL::uuid, NULL::uuid, NULL::uuid, NULL::uuid, NULL::text[]
          FROM ple_data.blueprint_course_revision AS revision
         WHERE revision.blueprint_course_id = v_reference
           AND (p_after IS NULL OR revision.blueprint_revision_number < p_after::bigint)
         ORDER BY revision.blueprint_revision_number DESC LIMIT p_page_size + 1;
    ELSE
        IF p_after IS NOT NULL AND p_after !~
           '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' THEN
            RAISE EXCEPTION 'invalid Blueprint history page' USING ERRCODE = '22023';
        END IF;
        -- Exact recorded metadata states, not invented historical name diffs.
        RETURN QUERY
        SELECT event.blueprint_edit_number::text, NULL::bigint,
               (extract(epoch FROM event.occurred_at) * 1000)::bigint,
               event.short_name, event.long_name, event.availability,
               event.content_discipline_id, event.content_subject_id, event.content_topic_id,
               event.content_subtopic_id, event.tags
          FROM ple_data.blueprint_metadata_event AS event
         WHERE event.blueprint_course_id = v_reference
           AND (p_after IS NULL OR event.blueprint_metadata_event_id < (
               SELECT prior.blueprint_metadata_event_id
                 FROM ple_data.blueprint_metadata_event AS prior
                WHERE prior.blueprint_course_id = v_reference
                  AND prior.blueprint_edit_number = p_after::bigint))
         ORDER BY event.blueprint_metadata_event_id DESC LIMIT p_page_size + 1;
    END IF;
END
$$;

