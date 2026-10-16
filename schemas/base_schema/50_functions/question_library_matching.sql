-- One Instructor-authorized matching relation owns every Library discovery predicate.
SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.matching_question_library_entries(
    p_exact_question_id text, p_text_terms jsonb, p_author_names text[], p_backends text[],
    p_tags text[], p_subjects text[], p_topics text[], p_discipline_ids uuid[], p_subject_ids uuid[],
    p_topic_ids uuid[], p_subtopic_ids uuid[], p_cross_discipline boolean, p_bloom_cognitive_processes text[],
    p_bloom_knowledge_dimensions text[], p_question_types text[], p_question_licenses text[],
    p_authored_by_current_account boolean, p_owner_account_id text, p_exclude_field text,
    p_scope text DEFAULT 'public'
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
    language text, citation_text text,
    parent_published_question_id text, parent_revision_number integer,
    starred_at_micros bigint
) LANGUAGE plpgsql SECURITY DEFINER
SET jit = off
SET search_path = pg_catalog, ple_api, ple_private AS $$
BEGIN
    IF NOT (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_has_platform_administration()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor or Sysadmin Account';
    END IF;
    IF p_scope NOT IN ('public', 'current_starred')
       OR (p_scope = 'current_starred' AND NOT ple_api.current_session_account_is_instructor()) THEN
        RAISE EXCEPTION USING ERRCODE = '42501', MESSAGE = 'Question Library scope is unavailable';
    END IF;
    IF p_exclude_field IS NOT NULL AND p_exclude_field NOT IN (
        'kind', 'authorNames', 'backends', 'tags', 'nameSubjects', 'nameTopics',
        'disciplineUuids', 'subjectUuids', 'topicUuids', 'subtopicUuids',
        'bloomCognitiveProcesses', 'bloomKnowledgeDimensions', 'questionTypes',
        'capabilities', 'questionLicenses'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Question Library facet field is invalid';
    END IF;
    RETURN QUERY
    WITH question_rows AS MATERIALIZED (
        SELECT q.*, s.name AS subject_name, t.name AS topic_name, d.name AS discipline_name,
               d.is_retired AS discipline_is_retired, st.name AS subtopic_name,
               owner.owner_account_id::text AS owner_account_id,
               EXISTS (SELECT 1 FROM ple_data.question_pool_member member
                        WHERE member.published_question_id = q.published_question_id) AS in_pool,
               'question'::text AS row_kind, q.published_question_id AS public_id,
               NULL::bigint AS question_pool_edit_number,
               NULL::bigint AS question_pool_metadata_edit_number,
               NULL::bigint AS question_pool_member_count,
               floor(extract(epoch FROM star.starred_at) * 1000000)::bigint AS starred_at_micros
          FROM ple_private.question_library_entries(NULL, NULL, p_scope = 'public') q
          LEFT JOIN ple_data.question_star star
            ON star.published_question_id = q.published_question_id
           AND star.instructor_account_id = ple_api.current_session_account_id()
          JOIN ple_data.question_current_owner owner
            ON owner.published_question_id = q.published_question_id
          JOIN LATERAL ple_private.list_content_disciplines_including_retired() d ON d.content_discipline_id = q.content_discipline_id
          JOIN LATERAL ple_private.list_content_subjects(q.content_discipline_id) s ON s.content_subject_id = q.content_subject_id
          LEFT JOIN LATERAL ple_private.list_content_topics(q.content_subject_id) t ON t.content_topic_id = q.content_topic_id
          LEFT JOIN LATERAL ple_private.list_content_subtopics(q.content_topic_id) st ON st.content_subtopic_id = q.content_subtopic_id
         WHERE p_scope = 'public' OR star.published_question_id IS NOT NULL
    ), pool_rows AS MATERIALIZED (
        SELECT NULL::text AS published_question_id, NULL::integer AS revision_number,
               pool.backend::text AS backend, NULL::text AS question_format, pool.question_type::text AS question_type,
               floor(extract(epoch FROM pool.created_at) * 1000)::bigint AS published_at_millis,
               pool.title AS question_title, pool.description AS question_description,
               ARRAY[]::text[] AS author_names, ARRAY[]::text[] AS author_account_ids,
               false AS authored_by_current_account, false AS viewer_may_archive,
               false AS viewer_may_edit_metadata,
               pool.license::text AS question_license, NULL::text AS availability,
               NULL::bigint AS availability_edit_number, pool.question_pool_metadata_edit_number AS metadata_edit_number,
               pool.tags, pool.content_discipline_id, pool.content_subject_id, pool.content_topic_id, pool.content_subtopic_id,
               pool.bloom_cognitive_process::text AS bloom_cognitive_process,
               pool.bloom_knowledge_dimension::text AS bloom_knowledge_dimension,
               NULL::uuid AS source_object_record_id, NULL::text AS source_object_checksum,
               NULL::text AS source_media_type, NULL::text AS webwork_pg_path,
               NULL::text AS language, NULL::text AS citation_text,
               NULL::text AS parent_published_question_id, NULL::integer AS parent_revision_number,
               subject.name AS subject_name, topic.name AS topic_name, discipline.name AS discipline_name,
               discipline.is_retired AS discipline_is_retired, subtopic.name AS subtopic_name,
               pool.owner_account_id::text AS owner_account_id, false AS in_pool,
               'pool'::text AS row_kind, pool.question_pool_id::text AS public_id,
               pool.question_pool_edit_number, pool.question_pool_metadata_edit_number,
               members.member_count AS question_pool_member_count,
               NULL::bigint AS starred_at_micros
          FROM ple_data.question_pool pool
          JOIN LATERAL (SELECT count(*)::bigint AS member_count FROM ple_data.question_pool_member member
                         WHERE member.question_pool_id = pool.question_pool_id) members ON members.member_count > 0
          JOIN LATERAL ple_private.list_content_disciplines_including_retired() discipline ON discipline.content_discipline_id = pool.content_discipline_id
          JOIN LATERAL ple_private.list_content_subjects(pool.content_discipline_id) subject ON subject.content_subject_id = pool.content_subject_id
          LEFT JOIN LATERAL ple_private.list_content_topics(pool.content_subject_id) topic ON topic.content_topic_id = pool.content_topic_id
          LEFT JOIN LATERAL ple_private.list_content_subtopics(pool.content_topic_id) subtopic ON subtopic.content_subtopic_id = pool.content_subtopic_id
         WHERE p_scope = 'public'
    ), entries AS MATERIALIZED (
        SELECT * FROM question_rows UNION ALL SELECT * FROM pool_rows
    ), eligible AS MATERIALIZED (
        SELECT entry.* FROM entries entry
         WHERE (p_exact_question_id IS NULL OR entry.public_id = p_exact_question_id)
           AND (p_exclude_field = 'backends' OR p_backends IS NULL OR entry.backend = ANY(p_backends))
           AND (p_owner_account_id IS NULL OR entry.owner_account_id = p_owner_account_id)
           AND (p_exclude_field = 'tags' OR cardinality(p_tags) = 0 OR EXISTS (SELECT 1 FROM unnest(entry.tags) tag(value)
                WHERE lower(btrim(regexp_replace(tag.value, '[[:space:]]+', ' ', 'g'))) = ANY(p_tags)))
           AND (p_exclude_field = 'nameSubjects' OR cardinality(p_subjects) = 0 OR lower(btrim(regexp_replace(entry.subject_name, '[[:space:]]+', ' ', 'g'))) = ANY(p_subjects))
           AND (p_exclude_field = 'nameTopics' OR cardinality(p_topics) = 0 OR lower(btrim(regexp_replace(COALESCE(entry.topic_name, ''), '[[:space:]]+', ' ', 'g'))) = ANY(p_topics))
           AND (p_exclude_field = 'disciplineUuids' OR p_cross_discipline OR cardinality(p_discipline_ids) = 0 OR entry.content_discipline_id = ANY(p_discipline_ids))
           AND (p_exclude_field = 'subjectUuids' OR cardinality(p_subject_ids) = 0 OR entry.content_subject_id = ANY(p_subject_ids))
           AND (p_exclude_field = 'topicUuids' OR cardinality(p_topic_ids) = 0 OR entry.content_topic_id = ANY(p_topic_ids))
           AND (p_exclude_field = 'subtopicUuids' OR cardinality(p_subtopic_ids) = 0 OR entry.content_subtopic_id = ANY(p_subtopic_ids))
           AND (p_exclude_field = 'bloomCognitiveProcesses' OR cardinality(p_bloom_cognitive_processes) = 0 OR entry.bloom_cognitive_process = ANY(p_bloom_cognitive_processes))
           AND (p_exclude_field = 'bloomKnowledgeDimensions' OR cardinality(p_bloom_knowledge_dimensions) = 0 OR entry.bloom_knowledge_dimension = ANY(p_bloom_knowledge_dimensions))
           AND (p_exclude_field = 'questionTypes' OR cardinality(p_question_types) = 0 OR entry.question_type = ANY(p_question_types))
           AND (p_exclude_field = 'questionLicenses' OR cardinality(p_question_licenses) = 0 OR entry.question_license = ANY(p_question_licenses))
           AND (entry.row_kind = 'question' OR ((p_exclude_field = 'authorNames' OR cardinality(p_author_names) = 0)
                AND NOT p_authored_by_current_account
                AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements(COALESCE(p_text_terms, '[]'::jsonb)) term(value)
                                WHERE term.value ->> 'field' = 'author'
                                  AND COALESCE((term.value ->> 'excluded')::boolean, false) = false)))
           AND (p_exclude_field = 'authorNames' OR cardinality(p_author_names) = 0 OR EXISTS (SELECT 1 FROM unnest(entry.author_names) author(name)
                WHERE lower(btrim(regexp_replace(author.name, '[[:space:]]+', ' ', 'g'))) = ANY(p_author_names)))
           AND (NOT p_authored_by_current_account OR entry.authored_by_current_account)
           AND NOT EXISTS (
                SELECT 1 FROM jsonb_array_elements(COALESCE(p_text_terms, '[]'::jsonb)) term(value)
                 WHERE COALESCE(term.value ->> 'value', '') = '' OR (
                    COALESCE((term.value ->> 'excluded')::boolean, false) = CASE term.value ->> 'field'
                    WHEN 'discipline' THEN strpos(lower(entry.discipline_name), lower(term.value ->> 'value')) > 0
                    WHEN 'subtopic' THEN strpos(lower(COALESCE(entry.subtopic_name, '')), lower(term.value ->> 'value')) > 0
                    WHEN 'subject' THEN strpos(lower(entry.subject_name), lower(term.value ->> 'value')) > 0
                    WHEN 'topic' THEN strpos(lower(COALESCE(entry.topic_name, '')), lower(term.value ->> 'value')) > 0
                    WHEN 'tags' THEN EXISTS (SELECT 1 FROM unnest(entry.tags) tag(value) WHERE strpos(lower(tag.value), lower(term.value ->> 'value')) > 0)
                    WHEN 'question_type' THEN strpos(lower(CASE entry.question_type
                        WHEN 'multipleChoice' THEN 'multiple choice' WHEN 'multipleAnswer' THEN 'multiple answer'
                        WHEN 'fillInBlank' THEN 'fill in the blank' WHEN 'multipleFillInBlank' THEN 'multiple fill in the blank'
                        ELSE entry.question_type END), lower(term.value ->> 'value')) > 0
                    WHEN 'author' THEN EXISTS (SELECT 1 FROM unnest(entry.author_names) author(name) WHERE strpos(lower(author.name), lower(term.value ->> 'value')) > 0)
                    WHEN 'any' THEN strpos(lower(entry.question_title), lower(term.value ->> 'value')) > 0
                        OR strpos(lower(entry.question_description), lower(term.value ->> 'value')) > 0
                        OR strpos(lower(entry.subject_name), lower(term.value ->> 'value')) > 0
                        OR strpos(lower(COALESCE(entry.topic_name, '')), lower(term.value ->> 'value')) > 0
                        OR EXISTS (SELECT 1 FROM unnest(entry.tags) tag(value) WHERE strpos(lower(tag.value), lower(term.value ->> 'value')) > 0)
                        OR (entry.row_kind = 'question' AND EXISTS (SELECT 1 FROM unnest(entry.author_names) author(name) WHERE strpos(lower(author.name), lower(term.value ->> 'value')) > 0))
                        OR strpos(lower(CASE entry.question_type
                            WHEN 'multipleChoice' THEN 'multiple choice' WHEN 'multipleAnswer' THEN 'multiple answer'
                            WHEN 'fillInBlank' THEN 'fill in the blank' WHEN 'multipleFillInBlank' THEN 'multiple fill in the blank'
                            ELSE entry.question_type END), lower(term.value ->> 'value')) > 0
                    ELSE false END)
           )
    )
    SELECT entry.published_question_id, entry.revision_number, entry.backend, entry.question_format, entry.question_type, entry.published_at_millis, entry.question_title, entry.question_description, entry.author_names, entry.author_account_ids, entry.authored_by_current_account, entry.viewer_may_archive, entry.viewer_may_edit_metadata, entry.question_license, entry.availability, entry.availability_edit_number, entry.metadata_edit_number, entry.tags, entry.content_discipline_id, entry.content_subject_id, entry.content_topic_id, entry.content_subtopic_id, entry.bloom_cognitive_process, entry.bloom_knowledge_dimension, entry.source_object_record_id, entry.source_object_checksum, entry.source_media_type, entry.webwork_pg_path, entry.subject_name, entry.topic_name, entry.discipline_name, entry.discipline_is_retired, entry.subtopic_name, entry.row_kind, entry.public_id, entry.owner_account_id, entry.in_pool, entry.question_pool_edit_number, entry.question_pool_metadata_edit_number, entry.question_pool_member_count, entry.language, entry.citation_text, entry.parent_published_question_id, entry.parent_revision_number, entry.starred_at_micros FROM eligible entry;
END
$$;
