-- Functions, triggers, and views from question_library_operations.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.question_library_entries(
    p_published_question_id text DEFAULT NULL, p_revision_number integer DEFAULT NULL,
    p_require_available boolean DEFAULT true
) RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, used_in_current_account_courses boolean,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    bloom_classification_edit_number bigint,
    source_object_record_id uuid, source_object_checksum text, source_media_type text,
    webwork_pg_path text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    -- ASVS 8.2.2 and 8.3.1: derive Course use from the current session's
    -- exact Instructor memberships at this trusted database boundary. The
    -- stable Question lineage match deliberately ignores the pinned Revision.
    RETURN QUERY
    WITH authorized_course_question_ids(published_question_id) AS MATERIALIZED (
        SELECT question.published_question_id
          FROM ple_data.assessment_entry AS entry
          JOIN ple_data.assessment_entry_question AS question
            ON question.assessment_entry_id = entry.assessment_entry_id
          JOIN ple_data.assessment AS assessment
            ON assessment.assessment_id = entry.assessment_id
         WHERE entry.availability = 'available'
           AND entry.entry_kind = 'fixed_question'
           AND ple_api.current_session_account_is_course_instructor(assessment.course_instance_id)
        UNION
        SELECT member.published_question_id
          FROM ple_data.assessment_entry AS entry
          JOIN ple_data.assessment_entry_pool AS pool_entry
            ON pool_entry.assessment_entry_id = entry.assessment_entry_id
          JOIN ple_data.assessment AS assessment
            ON assessment.assessment_id = entry.assessment_id
          JOIN ple_data.question_pool_member AS member
            ON member.question_pool_id = pool_entry.question_pool_id
         WHERE entry.availability = 'available'
           AND entry.entry_kind = 'question_pool'
           AND ple_api.current_session_account_is_course_instructor(assessment.course_instance_id)
    )
    SELECT revision.published_question_id::text, revision.revision_number, revision.backend::text, binding.question_format::text, revision.question_type::text,
           floor(extract(epoch FROM revision.published_at) * 1000)::bigint,
           metadata.question_title, metadata.question_description,
           ARRAY(SELECT authorship.author_display_name
               FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.published_question_id = revision.published_question_id
                AND authorship.revision_number = revision.revision_number
              ORDER BY authorship.author_position)::text[],
           EXISTS (SELECT 1 FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.published_question_id = revision.published_question_id
                AND authorship.revision_number = revision.revision_number
                AND authorship.author_account_id = ple_api.current_session_account_id()),
           EXISTS (SELECT 1 FROM ple_data.question_current_owner AS owner
              WHERE owner.published_question_id = revision.published_question_id
                AND owner.owner_account_id = ple_api.current_session_account_id()),
           license.spdx_expression::text, lineage.availability::text, lineage.availability_edit_number,
           metadata.metadata_edit_number, metadata.tags, metadata.content_discipline_id, metadata.content_subject_id, metadata.content_topic_id, metadata.content_subtopic_id,
           authorized_course_question.published_question_id IS NOT NULL,
           bloom.cognitive_process::text, bloom.knowledge_dimension::text,
           bloom.classification_edit_number,
           binding.source_object_record_id, binding.source_object_checksum, record.media_type,
           binding.webwork_pg_path
      FROM ple_data.question_revision AS revision
      JOIN ple_data.published_question AS lineage ON lineage.published_question_id = revision.published_question_id
      LEFT JOIN ple_data.question_revision_bloom AS bloom
        ON bloom.published_question_id = revision.published_question_id
       AND bloom.revision_number = revision.revision_number
      JOIN ple_data.published_question_metadata AS metadata ON metadata.published_question_id = revision.published_question_id
      JOIN ple_data.question_revision_license AS license
        ON license.published_question_id = revision.published_question_id AND license.revision_number = revision.revision_number
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.published_question_id = revision.published_question_id AND binding.revision_number = revision.revision_number
      JOIN ple_private.object_record AS record ON record.object_record_id = binding.source_object_record_id
      LEFT JOIN authorized_course_question_ids AS authorized_course_question
        ON authorized_course_question.published_question_id = revision.published_question_id
     WHERE (p_published_question_id IS NULL OR revision.published_question_id = p_published_question_id)
       AND (p_revision_number IS NULL OR revision.revision_number = p_revision_number)
       AND (p_revision_number IS NOT NULL OR revision.revision_number = (
           SELECT max(accepted.revision_number)
             FROM ple_data.question_revision_acceptance AS accepted
            WHERE accepted.published_question_id = revision.published_question_id
       ))
       AND (NOT p_require_available OR lineage.availability = 'available')
     ORDER BY metadata.question_title, revision.published_question_id, revision.revision_number;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE VIEW ple_api.published_question_summary
WITH (security_barrier = true, security_invoker = false) AS
SELECT lineage.published_question_id, revision.revision_number AS latest_question_revision_number,
       revision.backend, revision.question_type, revision.published_at, metadata.question_title,
       metadata.question_description, metadata.language, lineage.availability,
       lineage.availability_edit_number
  FROM ple_data.published_question AS lineage
  INNER JOIN ple_data.published_question_metadata AS metadata ON metadata.published_question_id = lineage.published_question_id
  INNER JOIN LATERAL (
      SELECT accepted.revision_number FROM ple_data.question_revision_acceptance AS accepted
       WHERE accepted.published_question_id = lineage.published_question_id
       ORDER BY accepted.revision_number DESC LIMIT 1
  ) AS latest ON true
  INNER JOIN ple_data.question_revision AS revision
    ON revision.published_question_id = lineage.published_question_id AND revision.revision_number = latest.revision_number;

CREATE FUNCTION ple_api.list_question_library_entries()
RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, used_in_current_account_courses boolean,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    bloom_classification_edit_number bigint,
    source_object_record_id uuid, source_object_checksum text, source_media_type text,
    webwork_pg_path text
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(NULL, NULL, true)
$$;

CREATE FUNCTION ple_api.load_question_library_revision(p_published_question_id text, p_revision_number integer)
RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, used_in_current_account_courses boolean,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    bloom_classification_edit_number bigint,
    source_object_record_id uuid, source_object_checksum text, source_media_type text,
    webwork_pg_path text
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(p_published_question_id, p_revision_number, false)
$$;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.load_current_published_question_shared_metadata(
    p_question_ids text[]
) RETURNS TABLE (
    published_question_id text, metadata_edit_number bigint, tags text[], content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    returned_count integer;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL
       OR NOT ple_api.current_session_account_is_instructor()
       OR ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Current Published Question metadata requires a vetted Instructor';
    END IF;
    IF p_question_ids IS NULL
       OR cardinality(p_question_ids) NOT BETWEEN 1 AND 1000
       OR EXISTS (
           SELECT 1 FROM unnest(p_question_ids) AS requested(published_question_id)
            WHERE requested.published_question_id IS NULL
               OR requested.published_question_id !~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
               OR substr(requested.published_question_id, 6, 1) <> ple_private.crockford_checksum_character(
                   substr(requested.published_question_id, 1, 4) || substr(requested.published_question_id, 7, 3)
               )
       )
       OR cardinality(p_question_ids) <> (
           SELECT count(DISTINCT requested.published_question_id)
             FROM unnest(p_question_ids) AS requested(published_question_id)
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Current Published Question metadata is unavailable';
    END IF;

    -- RETURN QUERY is buffered until this function completes. This one
    -- ordered data read therefore has one snapshot, and the later denial
    -- cannot expose rows from an incomplete selection.
    RETURN QUERY
    SELECT metadata.published_question_id::text, metadata.metadata_edit_number,
           metadata.tags, metadata.content_discipline_id, metadata.content_subject_id, metadata.content_topic_id, metadata.content_subtopic_id
      FROM ple_data.published_question_metadata AS metadata
      JOIN ple_data.published_question AS lineage
        ON lineage.published_question_id = metadata.published_question_id
       AND lineage.availability = 'available'
     WHERE metadata.published_question_id = ANY(p_question_ids)
       AND EXISTS (
           SELECT 1
             FROM ple_data.question_revision_acceptance AS acceptance
            WHERE acceptance.published_question_id = lineage.published_question_id
       )
     ORDER BY metadata.published_question_id;
    GET DIAGNOSTICS returned_count = ROW_COUNT;
    IF returned_count <> cardinality(p_question_ids) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Current Published Question metadata is unavailable';
    END IF;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_current_published_question_shared_metadata(p_question_ids text[])
RETURNS TABLE (
    published_question_id text, metadata_edit_number bigint, tags text[], content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT *
      FROM ple_private.load_current_published_question_shared_metadata(p_question_ids)
$$;
