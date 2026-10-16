-- Functions, triggers, and views from question_library_operations.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.question_library_entries(
    p_published_question_id text DEFAULT NULL, p_revision_number integer DEFAULT NULL,
    p_require_available boolean DEFAULT true
) RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[], author_account_ids text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    viewer_may_edit_metadata boolean,
    question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    source_object_record_id uuid, source_object_checksum text, source_media_type text,
    webwork_pg_path text, language text, citation_text text,
    parent_published_question_id text, parent_revision_number integer
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    RETURN QUERY
    SELECT revision.published_question_id::text, revision.revision_number, revision.backend::text, binding.question_format::text, metadata.question_type::text,
           floor(extract(epoch FROM revision.published_at) * 1000)::bigint,
           metadata.question_title, metadata.question_description,
           ARRAY(SELECT authorship.author_display_name
               FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.published_question_id = revision.published_question_id
                AND authorship.revision_number = revision.revision_number
              ORDER BY authorship.author_position)::text[],
           ARRAY(SELECT CASE
                       WHEN account.user_role = 'instructor'
                        AND state.state = 'active'
                        AND ple_private.instructor_display_name(authorship.author_account_id::text)
                            IS NOT NULL
                         THEN authorship.author_account_id::text
                       ELSE NULL
                     END
               FROM ple_data.question_revision_authorship AS authorship
               LEFT JOIN ple_private.account AS account
                 ON account.account_id = authorship.author_account_id
               LEFT JOIN LATERAL (
                   SELECT event.state
                     FROM ple_private.account_state_event AS event
                    WHERE event.account_id = authorship.author_account_id
                    ORDER BY event.occurred_at DESC, event.event_id DESC
                    LIMIT 1
               ) AS state ON true
              WHERE authorship.published_question_id = revision.published_question_id
                AND authorship.revision_number = revision.revision_number
              ORDER BY authorship.author_position)::text[],
           EXISTS (SELECT 1 FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.published_question_id = revision.published_question_id
                AND authorship.revision_number = revision.revision_number
                AND authorship.author_account_id = ple_api.current_session_account_id()),
           (ple_api.current_session_account_is_sysadmin()
             OR (ple_api.current_session_account_is_instructor()
                 AND EXISTS (SELECT 1 FROM ple_data.question_current_owner AS owner
                              WHERE owner.published_question_id = revision.published_question_id
                                AND owner.owner_account_id = ple_api.current_session_account_id()))),
           (lineage.availability = 'available'
            AND (ple_api.current_session_account_is_sysadmin()
                 OR (ple_api.current_session_account_is_instructor()
                     AND EXISTS (SELECT 1 FROM ple_data.question_current_owner AS owner
                                  WHERE owner.published_question_id = revision.published_question_id
                                    AND owner.owner_account_id = ple_api.current_session_account_id())))),
           license.spdx_expression::text, lineage.availability::text, lineage.availability_edit_number,
           metadata.metadata_edit_number, metadata.tags, metadata.content_discipline_id, metadata.content_subject_id, metadata.content_topic_id, metadata.content_subtopic_id,
           metadata.bloom_cognitive_process::text, metadata.bloom_knowledge_dimension::text,
           binding.source_object_record_id, binding.source_object_checksum, record.media_type,
           binding.webwork_pg_path, metadata.language, citation.citation_text,
           lineage.parent_published_question_id::text, lineage.parent_revision_number
      FROM ple_data.question_revision AS revision
      JOIN ple_data.published_question AS lineage ON lineage.published_question_id = revision.published_question_id
      JOIN ple_data.question_revision_metadata AS metadata
        ON metadata.published_question_id = revision.published_question_id
       AND metadata.revision_number = revision.revision_number
      JOIN ple_data.question_revision_license AS license
        ON license.published_question_id = revision.published_question_id AND license.revision_number = revision.revision_number
      LEFT JOIN ple_data.question_revision_citation AS citation
        ON citation.published_question_id = revision.published_question_id
       AND citation.revision_number = revision.revision_number
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.published_question_id = revision.published_question_id AND binding.revision_number = revision.revision_number
      JOIN ple_private.object_record AS record ON record.object_record_id = binding.source_object_record_id
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
       revision.backend, metadata.question_type, revision.published_at, metadata.question_title,
       metadata.question_description, metadata.language, lineage.availability,
       lineage.availability_edit_number, lineage.parent_published_question_id,
       lineage.parent_revision_number
  FROM ple_data.published_question AS lineage
  INNER JOIN LATERAL (
      SELECT accepted.revision_number FROM ple_data.question_revision_acceptance AS accepted
       WHERE accepted.published_question_id = lineage.published_question_id
       ORDER BY accepted.revision_number DESC LIMIT 1
  ) AS latest ON true
  INNER JOIN ple_data.question_revision AS revision
    ON revision.published_question_id = lineage.published_question_id AND revision.revision_number = latest.revision_number
  INNER JOIN ple_data.question_revision_metadata AS metadata
    ON metadata.published_question_id = revision.published_question_id
   AND metadata.revision_number = revision.revision_number;

CREATE FUNCTION ple_api.list_question_library_entries()
RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[], author_account_ids text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    viewer_may_edit_metadata boolean,
    question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    source_object_record_id uuid, source_object_checksum text, source_media_type text,
    webwork_pg_path text, language text, citation_text text,
    parent_published_question_id text, parent_revision_number integer
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(NULL, NULL, true)
$$;

-- Explicit stable-ID reads retain the ordinary current Question view after Archive.
CREATE FUNCTION ple_api.load_question_library_lineage(p_published_question_id text)
RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[], author_account_ids text[],
    authored_by_current_account boolean, viewer_may_archive boolean, viewer_may_edit_metadata boolean,
    question_license text, availability text, availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    source_object_record_id uuid, source_object_checksum text, source_media_type text,
    webwork_pg_path text, language text, citation_text text,
    parent_published_question_id text, parent_revision_number integer
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(p_published_question_id, NULL, false)
$$;

SET LOCAL ROLE ple_private_owner;


-- One authorized relation supplies both result kinds, their facets, and the global page.
CREATE FUNCTION ple_private.search_question_library_entries(
    p_exact_question_id text, p_text_terms jsonb, p_author_names text[], p_backends text[],
    p_tags text[], p_subjects text[], p_topics text[], p_discipline_ids uuid[], p_subject_ids uuid[],
    p_topic_ids uuid[], p_subtopic_ids uuid[], p_cross_discipline boolean, p_bloom_cognitive_processes text[],
    p_bloom_knowledge_dimensions text[], p_question_types text[], p_question_licenses text[],
    p_authored_by_current_account boolean,
    p_result_kind text, p_questions text, p_owner_account_id text,
    p_sort text, p_after_title text, p_after_published_at_millis bigint, p_after_public_id text,
    p_limit integer, p_scope text DEFAULT 'public', p_after_starred_at_micros bigint DEFAULT NULL
) RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[], author_account_ids text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    viewer_may_edit_metadata boolean, question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    source_object_record_id uuid, source_object_checksum text, source_media_type text, webwork_pg_path text,
    subject_name text, topic_name text, discipline_name text, discipline_is_retired boolean, subtopic_name text,
    row_kind text, public_id text, owner_account_id text, in_pool boolean,
    question_pool_edit_number bigint, question_pool_metadata_edit_number bigint, question_pool_member_count bigint,
    facets jsonb, language text, citation_text text,
    parent_published_question_id text, parent_revision_number integer,
    starred_at_micros bigint
) LANGUAGE plpgsql SECURITY DEFINER
-- Interactive metadata search spends more on JIT compilation than on execution.
-- Live Demo EXPLAIN measured 1465 ms compiling versus 22 ms without JIT.
-- Keep this choice local to this function and its nested reads.
SET jit = off
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    IF p_limit NOT BETWEEN 1 AND 251
       OR p_sort NOT IN ('title_ascending', 'title_descending', 'kind_ascending', 'kind_descending',
                         'discipline_ascending', 'discipline_descending',
                         'question_type_ascending', 'question_type_descending', 'published_newest',
                         'starred_date_ascending', 'starred_date_descending')
       OR p_scope NOT IN ('public', 'current_starred')
       OR (p_scope = 'public' AND p_sort IN ('starred_date_ascending', 'starred_date_descending'))
       OR (p_scope = 'current_starred' AND (p_result_kind <> 'questions' OR NOT ple_api.current_session_account_is_instructor()))
       OR p_result_kind NOT IN ('both', 'questions', 'pools')
       OR p_questions NOT IN ('no_pool', 'all')
       OR jsonb_typeof(COALESCE(p_text_terms, '[]'::jsonb)) <> 'array'
       OR (cardinality(p_subject_ids) > 0 AND cardinality(p_discipline_ids) = 0)
       OR (cardinality(p_topic_ids) > 0 AND cardinality(p_subject_ids) = 0)
       OR (cardinality(p_subtopic_ids) > 0 AND cardinality(p_topic_ids) = 0)
       OR (p_cross_discipline AND (cardinality(p_discipline_ids) = 0 OR cardinality(p_subject_ids) = 0)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Library discovery request is invalid';
    END IF;
    RETURN QUERY
    WITH eligible AS MATERIALIZED (
        SELECT * FROM ple_private.matching_question_library_entries(
            p_exact_question_id, p_text_terms, p_author_names, p_backends, p_tags, p_subjects,
            p_topics, p_discipline_ids, p_subject_ids, p_topic_ids, p_subtopic_ids,
            p_cross_discipline, p_bloom_cognitive_processes, p_bloom_knowledge_dimensions,
            p_question_types, p_question_licenses, p_authored_by_current_account,
            p_owner_account_id, NULL, p_scope)
    ), filtered AS MATERIALIZED (
        SELECT * FROM eligible entry
         WHERE (p_result_kind = 'both' OR p_result_kind = CASE entry.row_kind WHEN 'question' THEN 'questions' ELSE 'pools' END)
           AND (entry.row_kind = 'pool' OR p_questions = 'all' OR NOT entry.in_pool)
    ), author_values AS (
        SELECT entry.public_id,
               lower(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               min(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))) AS display_value
          FROM filtered AS entry
          CROSS JOIN LATERAL unnest(entry.author_names) AS author(name)
         WHERE btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g')) <> ''
         GROUP BY entry.public_id,
                  lower(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g')))
    ), author_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM author_values
         GROUP BY normalized_value
    ), tag_values AS (
        SELECT entry.public_id,
               lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               min(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))) AS display_value
          FROM filtered AS entry
          CROSS JOIN LATERAL unnest(entry.tags) AS tag(value)
         WHERE btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g')) <> ''
         GROUP BY entry.public_id,
                  lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g')))
    ), tag_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM tag_values
         GROUP BY normalized_value
    ), subject_values AS (
        SELECT entry.public_id,
               lower(btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g')) AS display_value
          FROM filtered AS entry
         WHERE btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g')) <> ''
    ), subject_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM subject_values
         GROUP BY normalized_value
    ), topic_values AS (
        SELECT entry.public_id,
               lower(btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g'))) AS normalized_value,
               btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g')) AS display_value
          FROM filtered AS entry
         WHERE btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g')) <> ''
    ), topic_counts AS (
        SELECT normalized_value, min(display_value) AS display_value, count(*)::bigint AS facet_count
          FROM topic_values
         GROUP BY normalized_value
    ), keyed AS MATERIALIZED (
        SELECT entry.*,
               CASE
                   WHEN p_sort IN ('title_ascending', 'title_descending') THEN entry.question_title
                   WHEN p_sort IN ('kind_ascending', 'kind_descending') THEN
                       CASE entry.row_kind WHEN 'question' THEN 'Published Question' ELSE 'Question Pool' END
                   WHEN p_sort IN ('discipline_ascending', 'discipline_descending') THEN entry.discipline_name
                   WHEN p_sort IN ('question_type_ascending', 'question_type_descending') THEN
                       CASE entry.question_type
                           WHEN 'multipleChoice' THEN 'Multiple choice'
                           WHEN 'multipleAnswer' THEN 'Multiple answer'
                           WHEN 'fillInBlank' THEN 'Fill in the blank'
                           WHEN 'multipleFillInBlank' THEN 'Multiple fill in the blank'
                           WHEN 'numeric' THEN 'Numeric'
                           WHEN 'matching' THEN 'Matching'
                           WHEN 'ordering' THEN 'Ordering'
                           WHEN 'hotspot' THEN 'Hotspot'
                       END
               END AS sort_key
          FROM filtered entry
    ), page AS MATERIALIZED (
        SELECT * FROM keyed entry
         WHERE p_after_public_id IS NULL
            OR (p_sort IN ('title_ascending', 'kind_ascending', 'discipline_ascending', 'question_type_ascending')
                AND (entry.sort_key COLLATE "C" > p_after_title COLLATE "C"
                     OR (entry.sort_key COLLATE "C" = p_after_title COLLATE "C" AND entry.public_id > p_after_public_id)))
            OR (p_sort IN ('title_descending', 'kind_descending', 'discipline_descending', 'question_type_descending')
                AND (entry.sort_key COLLATE "C" < p_after_title COLLATE "C"
                     OR (entry.sort_key COLLATE "C" = p_after_title COLLATE "C" AND entry.public_id > p_after_public_id)))
            OR (p_sort = 'published_newest' AND (entry.published_at_millis < p_after_published_at_millis
                OR (entry.published_at_millis = p_after_published_at_millis AND entry.public_id > p_after_public_id)))
            OR (p_sort = 'starred_date_ascending' AND (entry.starred_at_micros > p_after_starred_at_micros
                OR (entry.starred_at_micros = p_after_starred_at_micros AND entry.public_id > p_after_public_id)))
            OR (p_sort = 'starred_date_descending' AND (entry.starred_at_micros < p_after_starred_at_micros
                OR (entry.starred_at_micros = p_after_starred_at_micros AND entry.public_id > p_after_public_id)))
         ORDER BY CASE WHEN p_sort IN ('title_ascending', 'kind_ascending', 'discipline_ascending', 'question_type_ascending')
                       THEN entry.sort_key END COLLATE "C" ASC NULLS LAST,
                  CASE WHEN p_sort IN ('title_descending', 'kind_descending', 'discipline_descending', 'question_type_descending')
                       THEN entry.sort_key END COLLATE "C" DESC NULLS LAST,
                  CASE WHEN p_sort = 'published_newest' THEN entry.published_at_millis END DESC NULLS LAST,
                  CASE WHEN p_sort = 'starred_date_ascending' THEN entry.starred_at_micros END ASC NULLS LAST,
                  CASE WHEN p_sort = 'starred_date_descending' THEN entry.starred_at_micros END DESC NULLS LAST,
                  entry.public_id ASC
         LIMIT p_limit
    ), aggregates AS (
        SELECT jsonb_build_object(
            'categories', jsonb_build_object(
                'questionsInNoPool', (SELECT count(*) FROM eligible AS entry WHERE entry.row_kind = 'question' AND NOT entry.in_pool),
                'questionsInPool', (SELECT count(*) FROM eligible AS entry WHERE entry.row_kind = 'question' AND entry.in_pool),
                'pools', (SELECT count(*) FROM eligible AS entry WHERE entry.row_kind = 'pool')
            ),
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
    SELECT page.published_question_id, page.revision_number, page.backend, page.question_format, page.question_type, page.published_at_millis,
           page.question_title, page.question_description, page.author_names, page.author_account_ids,
           page.authored_by_current_account, page.viewer_may_archive, page.viewer_may_edit_metadata,
           page.question_license, page.availability,
           page.availability_edit_number, page.metadata_edit_number, page.tags,
           page.content_discipline_id, page.content_subject_id, page.content_topic_id, page.content_subtopic_id,
           page.bloom_cognitive_process, page.bloom_knowledge_dimension,
           page.source_object_record_id, page.source_object_checksum, page.source_media_type, page.webwork_pg_path,
           page.subject_name, page.topic_name, page.discipline_name, page.discipline_is_retired, page.subtopic_name,
           page.row_kind, page.public_id, page.owner_account_id, page.in_pool,
           page.question_pool_edit_number, page.question_pool_metadata_edit_number, page.question_pool_member_count,
           aggregates.facets, page.language, page.citation_text,
           page.parent_published_question_id, page.parent_revision_number, page.starred_at_micros
      FROM aggregates LEFT JOIN page ON true
     ORDER BY CASE WHEN p_sort IN ('title_ascending', 'kind_ascending', 'discipline_ascending', 'question_type_ascending')
                   THEN page.sort_key END COLLATE "C" ASC NULLS LAST,
              CASE WHEN p_sort IN ('title_descending', 'kind_descending', 'discipline_descending', 'question_type_descending')
                   THEN page.sort_key END COLLATE "C" DESC NULLS LAST,
              CASE WHEN p_sort = 'published_newest' THEN page.published_at_millis END DESC NULLS LAST,
              CASE WHEN p_sort = 'starred_date_ascending' THEN page.starred_at_micros END ASC NULLS LAST,
              CASE WHEN p_sort = 'starred_date_descending' THEN page.starred_at_micros END DESC NULLS LAST,
              page.public_id ASC NULLS LAST;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.search_question_library_entries(
    p_exact_question_id text, p_text_terms jsonb, p_author_names text[], p_backends text[],
    p_tags text[], p_subjects text[], p_topics text[], p_discipline_ids uuid[], p_subject_ids uuid[],
    p_topic_ids uuid[], p_subtopic_ids uuid[], p_cross_discipline boolean, p_bloom_cognitive_processes text[],
    p_bloom_knowledge_dimensions text[], p_question_types text[], p_question_licenses text[],
    p_authored_by_current_account boolean,
    p_result_kind text, p_questions text, p_owner_account_id text,
    p_sort text, p_after_title text, p_after_published_at_millis bigint, p_after_public_id text,
    p_limit integer, p_scope text DEFAULT 'public', p_after_starred_at_micros bigint DEFAULT NULL
) RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[], author_account_ids text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    viewer_may_edit_metadata boolean, question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    source_object_record_id uuid, source_object_checksum text, source_media_type text, webwork_pg_path text,
    subject_name text, topic_name text, discipline_name text, discipline_is_retired boolean, subtopic_name text,
    row_kind text, public_id text, owner_account_id text, in_pool boolean,
    question_pool_edit_number bigint, question_pool_metadata_edit_number bigint, question_pool_member_count bigint,
    facets jsonb, language text, citation_text text,
    parent_published_question_id text, parent_revision_number integer,
    starred_at_micros bigint
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.search_question_library_entries(
        p_exact_question_id, p_text_terms, p_author_names, p_backends, p_tags, p_subjects, p_topics,
        p_discipline_ids, p_subject_ids, p_topic_ids, p_subtopic_ids, p_cross_discipline,
        p_bloom_cognitive_processes, p_bloom_knowledge_dimensions, p_question_types, p_question_licenses,
        p_authored_by_current_account, p_result_kind, p_questions,
        p_owner_account_id, p_sort, p_after_title, p_after_published_at_millis,
        p_after_public_id, p_limit, p_scope, p_after_starred_at_micros)
$$;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.load_library_object_kind(p_public_id text)
RETURNS TABLE (row_kind text) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Library requires active Instructor or Sysadmin';
    END IF;
    RETURN QUERY SELECT 'question'::text
      FROM ple_private.question_library_entries(p_public_id, NULL, false) question
      JOIN ple_data.question_current_owner owner USING (published_question_id)
     LIMIT 1;
    IF FOUND THEN RETURN; END IF;
    RETURN QUERY SELECT 'pool'::text FROM ple_data.question_pool pool
     WHERE pool.question_pool_id = p_public_id
       AND EXISTS (SELECT 1 FROM ple_data.question_pool_member member
                    WHERE member.question_pool_id = pool.question_pool_id)
     LIMIT 1;
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_library_object_kind(p_public_id text)
RETURNS TABLE (row_kind text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_library_object_kind(p_public_id)
$$;

CREATE FUNCTION ple_api.load_question_library_revision(p_published_question_id text, p_revision_number integer)
RETURNS TABLE (
    published_question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[], author_account_ids text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    viewer_may_edit_metadata boolean,
    question_license text, availability text,
    availability_edit_number bigint, metadata_edit_number bigint, tags text[],
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text,
    source_object_record_id uuid, source_object_checksum text, source_media_type text,
    webwork_pg_path text, language text, citation_text text,
    parent_published_question_id text, parent_revision_number integer,
    revision_general_feedback text, revision_hint text, revision_worked_solution text
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT entry.*, revision.general_feedback, revision.hint, revision.worked_solution
      FROM ple_private.question_library_entries(p_published_question_id, p_revision_number, false) AS entry
      JOIN ple_data.question_revision AS revision
        ON revision.published_question_id = entry.published_question_id
       AND revision.revision_number = entry.revision_number
$$;

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.load_current_published_question_shared_metadata(
    p_question_ids text[]
) RETURNS TABLE (
    published_question_id text, metadata_edit_number bigint, question_title text, question_description text,
    question_type text,
    tags text[], content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    returned_count integer;
BEGIN
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL
       OR NOT (ple_api.current_session_account_is_instructor()
               OR ple_api.current_session_account_is_sysadmin())
       OR (ple_api.current_session_account_is_instructor()
           AND ple_private.instructor_display_name(actor_id) IS NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Current Published Question metadata requires an active Library reader';
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
           metadata.question_title, metadata.question_description, metadata.question_type::text, metadata.tags,
           metadata.content_discipline_id, metadata.content_subject_id,
           metadata.content_topic_id, metadata.content_subtopic_id,
           metadata.bloom_cognitive_process::text, metadata.bloom_knowledge_dimension::text
      FROM ple_data.question_revision_metadata AS metadata
      JOIN LATERAL (
          SELECT accepted.revision_number
            FROM ple_data.question_revision_acceptance AS accepted
           WHERE accepted.published_question_id = metadata.published_question_id
           ORDER BY accepted.revision_number DESC
           LIMIT 1
      ) AS latest ON true
     WHERE metadata.published_question_id = ANY(p_question_ids)
       AND EXISTS (
           SELECT 1 FROM ple_data.published_question AS lineage
            WHERE lineage.published_question_id = metadata.published_question_id
              AND lineage.availability = 'available'
       )
       AND metadata.revision_number = latest.revision_number
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
    published_question_id text, metadata_edit_number bigint, question_title text, question_description text,
    question_type text,
    tags text[], content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT *
      FROM ple_private.load_current_published_question_shared_metadata(p_question_ids)
$$;

-- ASVS 8.2.2: the data owner reads all current fixed Assessment pins for
-- installation totals. The API owner's Assessment RLS sees only its Courses,
-- so it cannot calculate these totals itself. No Course or Student rows enter
-- this identity-free projection.
SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.question_use_global_counts(
    p_published_question_id text, p_revision_number integer
) RETURNS TABLE (course_count bigint, assessment_count bigint)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
    SELECT count(DISTINCT assessment.course_instance_id),
           count(DISTINCT assessment.assessment_id)
      FROM ple_data.assessment AS assessment
      JOIN ple_data.assessment_entry AS entry
        ON entry.assessment_id = assessment.assessment_id
       AND entry.availability = 'available'
      JOIN ple_data.assessment_entry_question AS fixed
        ON fixed.assessment_entry_id = entry.assessment_entry_id
     WHERE fixed.published_question_id = p_published_question_id
       AND fixed.question_revision_number = p_revision_number
$$;

SET LOCAL ROLE ple_api_owner;

-- ASVS 8.2.1, 8.2.2, 8.3.1: the exact publication is selected by the
-- authenticated Library route. Existing Assessment RLS limits named Courses
-- to current Instructors, including co-Instructors. Counts describe current
-- fixed Assessment entries; Pool membership is a separate reference.
CREATE FUNCTION ple_api.read_question_use_details(
    p_published_question_id text, p_revision_number integer
) RETURNS TABLE (
    global_course_count bigint,
    global_assessment_count bigint,
    own_course_count bigint,
    own_assessment_count bigint,
    own_course_instance_id text,
    own_course_title text,
    own_course_assessment_count bigint
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    IF p_published_question_id IS NULL OR p_revision_number IS NULL
       OR p_revision_number <= 0 THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Question use selection is invalid';
    END IF;

    RETURN QUERY
    WITH own_assessments AS (
        SELECT DISTINCT assessment.course_instance_id,
                        assessment.assessment_id
          FROM ple_data.assessment AS assessment
          JOIN ple_data.assessment_entry AS entry
            ON entry.assessment_id = assessment.assessment_id
           AND entry.availability = 'available'
          JOIN ple_data.assessment_entry_question AS fixed
            ON fixed.assessment_entry_id = entry.assessment_entry_id
         WHERE fixed.published_question_id = p_published_question_id
           AND fixed.question_revision_number = p_revision_number
           AND ple_api.current_session_account_is_course_instructor(
                   assessment.course_instance_id)
    ), own_courses AS (
        SELECT course.course_instance_id::text AS course_instance_id,
               course.course_long_name AS title,
               count(*)::bigint AS assessment_count
          FROM own_assessments AS matching
          JOIN ple_data.course_instance AS course
            ON course.course_instance_id = matching.course_instance_id
         GROUP BY course.course_instance_id, course.course_long_name
    ), own_totals AS (
        SELECT count(*)::bigint AS course_count,
               COALESCE(sum(assessment_count), 0)::bigint AS assessment_count
          FROM own_courses
    )
    SELECT global_counts.course_count,
           global_counts.assessment_count,
           own_totals.course_count,
           own_totals.assessment_count,
           own_courses.course_instance_id,
           own_courses.title,
           own_courses.assessment_count
      FROM ple_data.question_use_global_counts(
               p_published_question_id, p_revision_number) AS global_counts
      CROSS JOIN own_totals
      LEFT JOIN own_courses ON true
     ORDER BY own_courses.title, own_courses.course_instance_id;
END
$$;
