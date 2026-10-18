-- Bounded choices from the same authorized matching relation as result search.
SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.question_library_facet_choices(
    p_exact_question_id text, p_text_terms jsonb, p_author_names text[], p_backends text[],
    p_tags text[], p_subjects text[], p_topics text[], p_discipline_ids uuid[], p_subject_ids uuid[],
    p_topic_ids uuid[], p_subtopic_ids uuid[], p_cross_discipline boolean, p_bloom_cognitive_processes text[],
    p_bloom_knowledge_dimensions text[], p_question_types text[], p_question_licenses text[],
    p_authored_by_current_account boolean, p_owner_account_id text,
    p_result_kind text, p_questions text, p_field text, p_choice_search text,
    p_after_key text, p_limit integer, p_selected_keys text[], p_scope text,
    p_excluded_question_ids text[]
) RETURNS TABLE (key text, label text, choice_count bigint, is_selected boolean)
LANGUAGE plpgsql SECURITY DEFINER SET jit = off
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    -- ASVS 2.2.1, 2.2.2: enforce the closed field set at the trusted SQL boundary.
    IF p_field NOT IN (
        'kind', 'authorNames', 'backends', 'tags', 'nameSubjects', 'nameTopics',
        'disciplineUuids', 'subjectUuids', 'topicUuids', 'subtopicUuids',
        'bloomCognitiveProcesses', 'bloomKnowledgeDimensions', 'questionTypes',
        'capabilities', 'questionLicenses'
    ) OR p_result_kind NOT IN ('both', 'questions', 'pools')
      OR p_questions NOT IN ('all', 'no_pool')
      OR p_scope NOT IN ('public', 'current_starred')
      OR (p_scope = 'current_starred' AND (p_result_kind <> 'questions' OR NOT ple_api.current_session_account_is_instructor()))
      OR p_limit NOT BETWEEN 1 AND 101
      OR char_length(p_choice_search) > 256
      OR cardinality(p_selected_keys) > 64
      OR (cardinality(p_subject_ids) > 0 AND cardinality(p_discipline_ids) = 0)
      OR (cardinality(p_topic_ids) > 0 AND cardinality(p_subject_ids) = 0)
      OR (cardinality(p_subtopic_ids) > 0 AND cardinality(p_topic_ids) = 0)
      OR (p_cross_discipline AND (cardinality(p_discipline_ids) = 0 OR cardinality(p_subject_ids) = 0)) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Library facet request is invalid';
    END IF;
    RETURN QUERY
    WITH matching AS MATERIALIZED (
        SELECT * FROM ple_private.matching_question_library_entries(
            p_exact_question_id, p_text_terms, p_author_names, p_backends, p_tags, p_subjects,
            p_topics, p_discipline_ids, p_subject_ids, p_topic_ids, p_subtopic_ids,
            p_cross_discipline, p_bloom_cognitive_processes, p_bloom_knowledge_dimensions,
            p_question_types, p_question_licenses, p_authored_by_current_account,
            p_owner_account_id, CASE WHEN p_field = 'backends' THEN NULL ELSE p_field END,
            p_scope)
    ), scoped AS MATERIALIZED (
        SELECT * FROM matching entry
         WHERE (p_field = 'kind' OR p_result_kind = 'both' OR p_result_kind = CASE entry.row_kind WHEN 'question' THEN 'questions' ELSE 'pools' END)
           AND (entry.row_kind = 'pool' OR p_questions = 'all' OR NOT entry.in_pool)
           AND (entry.row_kind <> 'question' OR NOT (entry.published_question_id = ANY(COALESCE(p_excluded_question_ids, ARRAY[]::text[]))))
    ), raw_values AS (
        SELECT entry.public_id, value.key, value.label
          FROM scoped entry
          CROSS JOIN LATERAL (
              SELECT CASE p_field
                  WHEN 'kind' THEN CASE entry.row_kind WHEN 'question' THEN 'questions' ELSE 'pools' END
                  WHEN 'backends' THEN entry.backend
                  WHEN 'capabilities' THEN entry.backend
                  WHEN 'nameSubjects' THEN lower(btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g')))
                  WHEN 'nameTopics' THEN lower(btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g')))
                  WHEN 'disciplineUuids' THEN entry.content_discipline_id::text
                  WHEN 'subjectUuids' THEN entry.content_subject_id::text
                  WHEN 'topicUuids' THEN entry.content_topic_id::text
                  WHEN 'subtopicUuids' THEN entry.content_subtopic_id::text
                  WHEN 'bloomCognitiveProcesses' THEN entry.bloom_cognitive_process
                  WHEN 'bloomKnowledgeDimensions' THEN entry.bloom_knowledge_dimension
                  WHEN 'questionTypes' THEN entry.question_type
                  WHEN 'questionLicenses' THEN entry.question_license
                  END AS key,
                  CASE p_field
                  WHEN 'kind' THEN CASE entry.row_kind WHEN 'question' THEN 'Published Questions' ELSE 'Question Pools' END
                  WHEN 'nameSubjects' THEN btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g'))
                  WHEN 'nameTopics' THEN btrim(regexp_replace(entry.topic_name, '[[:space:]]+', ' ', 'g'))
                  WHEN 'disciplineUuids' THEN entry.discipline_name
                  WHEN 'subjectUuids' THEN entry.subject_name
                  WHEN 'topicUuids' THEN entry.topic_name
                  WHEN 'subtopicUuids' THEN entry.subtopic_name
                  WHEN 'capabilities' THEN entry.backend
                  WHEN 'backends' THEN entry.backend
                  WHEN 'bloomCognitiveProcesses' THEN entry.bloom_cognitive_process
                  WHEN 'bloomKnowledgeDimensions' THEN entry.bloom_knowledge_dimension
                  WHEN 'questionTypes' THEN entry.question_type
                  WHEN 'questionLicenses' THEN entry.question_license
                  END AS label
              UNION ALL
              SELECT lower(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))),
                     btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))
                FROM unnest(entry.author_names) author(name) WHERE p_field = 'authorNames'
              UNION ALL
              SELECT lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))),
                     btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))
                FROM unnest(entry.tags) tag(value) WHERE p_field = 'tags'
          ) value
         WHERE value.key IS NOT NULL AND value.key <> ''
    ), per_object AS (
        SELECT public_id, raw_values.key, min(raw_values.label) AS label
          FROM raw_values GROUP BY public_id, raw_values.key
    ), grouped AS MATERIALIZED (
        SELECT per_object.key, min(per_object.label) AS label, count(*)::bigint AS choice_count
          FROM per_object GROUP BY per_object.key
    ), choice_page AS (
        SELECT grouped.key, grouped.label, grouped.choice_count
          FROM grouped
         WHERE (p_after_key IS NULL OR grouped.key COLLATE "C" > p_after_key COLLATE "C")
           AND (p_choice_search IS NULL OR strpos(lower(grouped.label), lower(p_choice_search)) > 0)
         ORDER BY grouped.key COLLATE "C" LIMIT p_limit
    ), selected_keys AS (
        SELECT DISTINCT selected.key FROM unnest(p_selected_keys) selected(key)
    ), selected_labels AS (
        SELECT selected.key, COALESCE(grouped.label,
            CASE p_field
                WHEN 'kind' THEN CASE selected.key WHEN 'questions' THEN 'Published Questions' WHEN 'pools' THEN 'Question Pools' END
                WHEN 'disciplineUuids' THEN (SELECT vocabulary.name FROM ple_private.list_content_disciplines_including_retired() vocabulary WHERE vocabulary.content_discipline_id::text = selected.key)
                WHEN 'subjectUuids' THEN (SELECT vocabulary.name FROM unnest(p_discipline_ids) parent(id) CROSS JOIN LATERAL ple_private.list_content_subjects(parent.id) vocabulary WHERE vocabulary.content_subject_id::text = selected.key LIMIT 1)
                WHEN 'topicUuids' THEN (SELECT vocabulary.name FROM unnest(p_subject_ids) parent(id) CROSS JOIN LATERAL ple_private.list_content_topics(parent.id) vocabulary WHERE vocabulary.content_topic_id::text = selected.key LIMIT 1)
                WHEN 'subtopicUuids' THEN (SELECT vocabulary.name FROM unnest(p_topic_ids) parent(id) CROSS JOIN LATERAL ple_private.list_content_subtopics(parent.id) vocabulary WHERE vocabulary.content_subtopic_id::text = selected.key LIMIT 1)
                ELSE selected.key
            END) AS label, COALESCE(grouped.choice_count, 0)::bigint AS choice_count
          FROM selected_keys selected LEFT JOIN grouped ON grouped.key = selected.key
    ), all_choices AS (
        SELECT page.key, page.label, page.choice_count, false AS is_selected FROM choice_page page
        UNION ALL
        SELECT selected.key, selected.label, selected.choice_count, true AS is_selected FROM selected_labels selected
    )
    SELECT choices.key, choices.label, choices.choice_count, choices.is_selected
      FROM all_choices choices
     ORDER BY choices.is_selected, choices.key COLLATE "C";
END
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.question_library_facet_choices(
    p_exact_question_id text, p_text_terms jsonb, p_author_names text[], p_backends text[],
    p_tags text[], p_subjects text[], p_topics text[], p_discipline_ids uuid[], p_subject_ids uuid[],
    p_topic_ids uuid[], p_subtopic_ids uuid[], p_cross_discipline boolean, p_bloom_cognitive_processes text[],
    p_bloom_knowledge_dimensions text[], p_question_types text[], p_question_licenses text[],
    p_authored_by_current_account boolean, p_owner_account_id text,
    p_result_kind text, p_questions text, p_field text, p_choice_search text,
    p_after_key text, p_limit integer, p_selected_keys text[], p_scope text,
    p_excluded_question_ids text[]
) RETURNS TABLE (key text, label text, choice_count bigint, is_selected boolean)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_facet_choices(
        p_exact_question_id, p_text_terms, p_author_names, p_backends, p_tags, p_subjects,
        p_topics, p_discipline_ids, p_subject_ids, p_topic_ids, p_subtopic_ids,
        p_cross_discipline, p_bloom_cognitive_processes, p_bloom_knowledge_dimensions,
        p_question_types, p_question_licenses, p_authored_by_current_account,
        p_owner_account_id, p_result_kind, p_questions, p_field, p_choice_search,
        p_after_key, p_limit, p_selected_keys, p_scope, p_excluded_question_ids)
$$;
