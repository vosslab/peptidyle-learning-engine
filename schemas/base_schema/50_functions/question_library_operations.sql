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

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.search_question_library_entries(
    p_exact_question_id text,
    p_text_terms jsonb,
    p_author_names text[],
    p_backends text[],
    p_tags text[],
    p_subjects text[],
    p_topics text[],
    p_discipline_id uuid,
    p_subject_id uuid,
    p_topic_id uuid,
    p_subtopic_id uuid,
    p_cross_discipline boolean,
    p_bloom_cognitive_process text,
    p_bloom_knowledge_dimension text,
    p_question_types text[],
    p_question_licenses text[],
    p_used_in_current_account_courses boolean,
    p_authored_by_current_account boolean,
    p_sort text,
    p_after_title text,
    p_after_published_at_millis bigint,
    p_after_question_id text,
    p_limit integer
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
    webwork_pg_path text,
    subject_name text, topic_name text, discipline_name text, discipline_is_retired boolean, subtopic_name text,
    facets jsonb
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF p_limit NOT BETWEEN 1 AND 251
       OR p_sort NOT IN ('title_ascending', 'published_newest')
       OR jsonb_typeof(COALESCE(p_text_terms, '[]'::jsonb)) <> 'array' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Library discovery request is invalid';
    END IF;
    RETURN QUERY
    WITH entries AS MATERIALIZED (
        SELECT q.*, s.name AS subject_name, t.name AS topic_name,
               d.name AS discipline_name, d.is_retired AS discipline_is_retired,
               st.name AS subtopic_name
          FROM ple_private.question_library_entries(NULL, NULL, true) q
          JOIN LATERAL ple_private.list_content_disciplines_including_retired() d
            ON d.content_discipline_id = q.content_discipline_id
          LEFT JOIN LATERAL ple_private.list_content_subtopics(q.content_topic_id) st
            ON st.content_subtopic_id = q.content_subtopic_id
          JOIN LATERAL ple_private.list_content_subjects(q.content_discipline_id) s
            ON s.content_subject_id = q.content_subject_id
          LEFT JOIN LATERAL ple_private.list_content_topics(q.content_subject_id) t
            ON t.content_topic_id = q.content_topic_id
    ), filtered AS MATERIALIZED (
        SELECT entry.*
          FROM entries entry
         WHERE (p_exact_question_id IS NULL OR entry.published_question_id = p_exact_question_id)
           AND (p_backends IS NULL OR entry.backend = ANY(p_backends))
           AND (cardinality(p_author_names) = 0 OR EXISTS (
                SELECT 1 FROM unnest(entry.author_names) author(name)
                 WHERE lower(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))) = ANY(p_author_names)
           ))
           AND (cardinality(p_tags) = 0 OR EXISTS (
                SELECT 1 FROM unnest(entry.tags) tag(value)
                 WHERE lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))) = ANY(p_tags)
           ))
           AND (cardinality(p_subjects) = 0 OR lower(btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g'))) = ANY(p_subjects))
           AND (cardinality(p_topics) = 0 OR lower(btrim(regexp_replace(COALESCE(entry.topic_name, ''), '[[:space:]]+', ' ', 'g'))) = ANY(p_topics))
           AND (p_cross_discipline OR p_discipline_id IS NULL OR entry.content_discipline_id = p_discipline_id)
           AND (p_subject_id IS NULL OR entry.content_subject_id = p_subject_id)
           AND (p_topic_id IS NULL OR entry.content_topic_id = p_topic_id)
           AND (p_subtopic_id IS NULL OR entry.content_subtopic_id = p_subtopic_id)
           AND (p_bloom_cognitive_process IS NULL OR entry.bloom_cognitive_process = p_bloom_cognitive_process)
           AND (p_bloom_knowledge_dimension IS NULL OR entry.bloom_knowledge_dimension = p_bloom_knowledge_dimension)
           AND (cardinality(p_question_types) = 0 OR entry.question_type = ANY(p_question_types))
           AND (cardinality(p_question_licenses) = 0 OR entry.question_license = ANY(p_question_licenses))
           AND (NOT p_used_in_current_account_courses OR entry.used_in_current_account_courses)
           AND (NOT p_authored_by_current_account OR entry.authored_by_current_account)
           AND NOT EXISTS (
                SELECT 1
                  FROM jsonb_array_elements(COALESCE(p_text_terms, '[]'::jsonb)) term(value)
                 WHERE COALESCE(term.value ->> 'value', '') = ''
                    OR (COALESCE(term.value ->> 'value', '') <> ''
                        AND COALESCE((term.value ->> 'excluded')::boolean, false) = CASE term.value ->> 'field'
                    WHEN 'discipline' THEN strpos(lower(entry.discipline_name), lower(COALESCE(term.value ->> 'value', ''))) > 0
                    WHEN 'subtopic' THEN strpos(lower(COALESCE(entry.subtopic_name, '')), lower(COALESCE(term.value ->> 'value', ''))) > 0
                    WHEN 'subject' THEN strpos(lower(entry.subject_name), lower(COALESCE(term.value ->> 'value', ''))) > 0
                    WHEN 'topic' THEN strpos(lower(COALESCE(entry.topic_name, '')), lower(COALESCE(term.value ->> 'value', ''))) > 0
                    WHEN 'tags' THEN EXISTS (SELECT 1 FROM unnest(entry.tags) tag(value) WHERE strpos(lower(tag.value), lower(COALESCE(term.value ->> 'value', ''))) > 0)
                    WHEN 'question_type' THEN strpos(lower(CASE entry.question_type
                        WHEN 'multipleChoice' THEN 'multiple choice'
                        WHEN 'multipleAnswer' THEN 'multiple answer'
                        WHEN 'fillInBlank' THEN 'fill in the blank'
                        WHEN 'multipleFillInBlank' THEN 'multiple fill in the blank'
                        ELSE entry.question_type END), lower(COALESCE(term.value ->> 'value', ''))) > 0
                    WHEN 'author' THEN EXISTS (SELECT 1 FROM unnest(entry.author_names) author(name) WHERE strpos(lower(author.name), lower(COALESCE(term.value ->> 'value', ''))) > 0)
                    WHEN 'any' THEN strpos(lower(entry.question_title), lower(COALESCE(term.value ->> 'value', ''))) > 0
                        OR strpos(lower(entry.question_description), lower(COALESCE(term.value ->> 'value', ''))) > 0
                        OR strpos(lower(entry.subject_name), lower(COALESCE(term.value ->> 'value', ''))) > 0
                        OR strpos(lower(COALESCE(entry.topic_name, '')), lower(COALESCE(term.value ->> 'value', ''))) > 0
                        OR EXISTS (SELECT 1 FROM unnest(entry.tags) tag(value) WHERE strpos(lower(tag.value), lower(COALESCE(term.value ->> 'value', ''))) > 0)
                        OR EXISTS (SELECT 1 FROM unnest(entry.author_names) author(name) WHERE strpos(lower(author.name), lower(COALESCE(term.value ->> 'value', ''))) > 0)
                        OR strpos(lower(CASE entry.question_type
                            WHEN 'multipleChoice' THEN 'multiple choice'
                            WHEN 'multipleAnswer' THEN 'multiple answer'
                            WHEN 'fillInBlank' THEN 'fill in the blank'
                            WHEN 'multipleFillInBlank' THEN 'multiple fill in the blank'
                            ELSE entry.question_type END), lower(COALESCE(term.value ->> 'value', ''))) > 0
                    ELSE false
                 END)
           )
    ), author_values AS (
        SELECT entry.published_question_id,
               lower(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               min(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))) AS display_value
          FROM filtered AS entry
          CROSS JOIN LATERAL unnest(entry.author_names) AS author(name)
         WHERE btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g')) <> ''
         GROUP BY entry.published_question_id,
                  lower(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g')))
    ), author_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM author_values
         GROUP BY normalized_value
    ), tag_values AS (
        SELECT entry.published_question_id,
               lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               min(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))) AS display_value
          FROM filtered AS entry
          CROSS JOIN LATERAL unnest(entry.tags) AS tag(value)
         WHERE btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g')) <> ''
         GROUP BY entry.published_question_id,
                  lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g')))
    ), tag_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM tag_values
         GROUP BY normalized_value
    ), subject_values AS (
        SELECT entry.published_question_id,
               lower(btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g')) AS display_value
          FROM filtered AS entry
         WHERE btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g')) <> ''
    ), subject_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM subject_values
         GROUP BY normalized_value
    ), topic_values AS (
        SELECT entry.published_question_id,
               lower(btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g')) AS display_value
          FROM filtered AS entry
         WHERE btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g')) <> ''
    ), topic_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM topic_values
         GROUP BY normalized_value
    ), page AS (
        SELECT result.* FROM filtered AS result
     WHERE (p_after_question_id IS NULL)
        OR (p_sort = 'title_ascending' AND (
              (result.question_title COLLATE "C", result.published_question_id) >
              (p_after_title COLLATE "C", p_after_question_id)
           ))
        OR (p_sort = 'published_newest' AND (
              result.published_at_millis < p_after_published_at_millis
              OR (result.published_at_millis = p_after_published_at_millis
                  AND result.published_question_id > p_after_question_id)
           ))
     ORDER BY
        CASE WHEN p_sort = 'title_ascending' THEN result.question_title END COLLATE "C" ASC,
        CASE WHEN p_sort = 'published_newest' THEN result.published_at_millis END DESC,
        result.published_question_id ASC
     LIMIT p_limit
    ), aggregates AS (
        SELECT jsonb_build_object(
            'authorNames', COALESCE((
                SELECT jsonb_agg(jsonb_build_object('authorName', display_value, 'count', facet_count)
                                      ORDER BY facet_count DESC, display_value)
                  FROM (SELECT display_value, facet_count FROM author_counts
                         ORDER BY facet_count DESC, display_value LIMIT 64) AS bounded
            ), '[]'::jsonb),
            'authorNamesTruncated', (SELECT count(*) > 64 FROM author_counts),
            'backends', COALESCE((
                SELECT jsonb_agg(jsonb_build_object(
                           'backend', backend_counts.backend, 'count', backend_counts.facet_count
                       ) ORDER BY CASE backend_counts.backend
                           WHEN 'ple' THEN 1 WHEN 'webwork' THEN 2 WHEN 'imathas' THEN 3 END)
                  FROM (SELECT entry.backend, count(*)::bigint AS facet_count
                          FROM filtered AS entry GROUP BY entry.backend) AS backend_counts
            ), '[]'::jsonb),
            'tags', COALESCE((
                SELECT jsonb_agg(jsonb_build_object('tag', display_value, 'count', facet_count)
                                      ORDER BY facet_count DESC, display_value)
                  FROM (SELECT display_value, facet_count FROM tag_counts
                         ORDER BY facet_count DESC, display_value LIMIT 64) AS bounded
            ), '[]'::jsonb),
            'tagsTruncated', (SELECT count(*) > 64 FROM tag_counts),
            'subjects', COALESCE((
                SELECT jsonb_agg(jsonb_build_object('subject', display_value, 'count', facet_count)
                                      ORDER BY facet_count DESC, display_value)
                  FROM (SELECT display_value, facet_count FROM subject_counts
                         ORDER BY facet_count DESC, display_value LIMIT 64) AS bounded
            ), '[]'::jsonb),
            'subjectsTruncated', (SELECT count(*) > 64 FROM subject_counts),
            'topics', COALESCE((
                SELECT jsonb_agg(jsonb_build_object('topic', display_value, 'count', facet_count)
                                      ORDER BY facet_count DESC, display_value)
                  FROM (SELECT display_value, facet_count FROM topic_counts
                         ORDER BY facet_count DESC, display_value LIMIT 64) AS bounded
            ), '[]'::jsonb),
            'topicsTruncated', (SELECT count(*) > 64 FROM topic_counts),
            'questionTypes', COALESCE((
                SELECT jsonb_agg(jsonb_build_object(
                           'questionType', question_type_counts.question_type,
                           'count', question_type_counts.facet_count
                       ) ORDER BY CASE question_type_counts.question_type
                           WHEN 'multipleChoice' THEN 1 WHEN 'multipleAnswer' THEN 2
                           WHEN 'fillInBlank' THEN 3 WHEN 'multipleFillInBlank' THEN 4
                           WHEN 'numeric' THEN 5 WHEN 'matching' THEN 6
                           WHEN 'ordering' THEN 7 WHEN 'hotspot' THEN 8 END)
                  FROM (SELECT entry.question_type, count(*)::bigint AS facet_count
                          FROM filtered AS entry GROUP BY entry.question_type) AS question_type_counts
            ), '[]'::jsonb),
            'questionLicenses', COALESCE((
                SELECT jsonb_agg(jsonb_build_object(
                           'questionLicense', license_counts.question_license,
                           'count', license_counts.facet_count
                       ) ORDER BY CASE license_counts.question_license
                           WHEN 'CC0-1.0' THEN 1 WHEN 'CC-BY-4.0' THEN 2
                           WHEN 'CC-BY-SA-4.0' THEN 3 END)
                  FROM (SELECT entry.question_license, count(*)::bigint AS facet_count
                          FROM filtered AS entry GROUP BY entry.question_license) AS license_counts
            ), '[]'::jsonb),
            'usedInMyCourses', jsonb_build_object(
                'used', (SELECT count(*) FILTER (WHERE entry.used_in_current_account_courses)::bigint
                           FROM filtered AS entry)
            ),
            'bloomCognitiveProcesses', jsonb_build_array(
                jsonb_build_object('cognitiveProcess', 'Remember', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_cognitive_process = 'Remember')::bigint FROM filtered AS entry)),
                jsonb_build_object('cognitiveProcess', 'Understand', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_cognitive_process = 'Understand')::bigint FROM filtered AS entry)),
                jsonb_build_object('cognitiveProcess', 'Apply', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_cognitive_process = 'Apply')::bigint FROM filtered AS entry)),
                jsonb_build_object('cognitiveProcess', 'Analyze', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_cognitive_process = 'Analyze')::bigint FROM filtered AS entry)),
                jsonb_build_object('cognitiveProcess', 'Evaluate', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_cognitive_process = 'Evaluate')::bigint FROM filtered AS entry)),
                jsonb_build_object('cognitiveProcess', 'Create', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_cognitive_process = 'Create')::bigint FROM filtered AS entry))
            ),
            'bloomKnowledgeDimensions', jsonb_build_array(
                jsonb_build_object('knowledgeDimension', 'Factual Knowledge', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_knowledge_dimension = 'Factual Knowledge')::bigint FROM filtered AS entry)),
                jsonb_build_object('knowledgeDimension', 'Conceptual Knowledge', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_knowledge_dimension = 'Conceptual Knowledge')::bigint FROM filtered AS entry)),
                jsonb_build_object('knowledgeDimension', 'Procedural Knowledge', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_knowledge_dimension = 'Procedural Knowledge')::bigint FROM filtered AS entry)),
                jsonb_build_object('knowledgeDimension', 'Metacognitive Knowledge', 'count', (SELECT count(*) FILTER (WHERE entry.bloom_knowledge_dimension = 'Metacognitive Knowledge')::bigint FROM filtered AS entry))
            )
        ) AS facets
    )
    SELECT page.*, aggregates.facets
      FROM aggregates
      LEFT JOIN page ON true
     ORDER BY
        CASE WHEN p_sort = 'title_ascending' THEN page.question_title END COLLATE "C" ASC NULLS LAST,
        CASE WHEN p_sort = 'published_newest' THEN page.published_at_millis END DESC NULLS LAST,
        page.published_question_id ASC NULLS LAST;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.search_question_library_entries(
    p_exact_question_id text, p_text_terms jsonb, p_author_names text[], p_backends text[],
    p_tags text[], p_subjects text[], p_topics text[], p_discipline_id uuid, p_subject_id uuid,
    p_topic_id uuid, p_subtopic_id uuid, p_cross_discipline boolean, p_bloom_cognitive_process text,
    p_bloom_knowledge_dimension text, p_question_types text[], p_question_licenses text[],
    p_used_in_current_account_courses boolean, p_authored_by_current_account boolean, p_sort text,
    p_after_title text, p_after_published_at_millis bigint, p_after_question_id text, p_limit integer
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
    webwork_pg_path text,
    subject_name text, topic_name text, discipline_name text, discipline_is_retired boolean, subtopic_name text,
    facets jsonb
)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.search_question_library_entries(
        p_exact_question_id, p_text_terms, p_author_names, p_backends, p_tags, p_subjects, p_topics,
        p_discipline_id, p_subject_id, p_topic_id, p_subtopic_id, p_cross_discipline,
        p_bloom_cognitive_process, p_bloom_knowledge_dimension, p_question_types, p_question_licenses,
        p_used_in_current_account_courses, p_authored_by_current_account, p_sort, p_after_title,
        p_after_published_at_millis, p_after_question_id, p_limit)
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
