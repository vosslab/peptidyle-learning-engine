-- Draft ownership and private authoring reads, with their API entry points.

SET LOCAL ROLE ple_private_owner;

-- Drafts belong to the Account that owns their Authoring Workspace.  Workspace
-- collaborators may read and edit a Draft through the separate access
-- predicate, but never acquire this ownership fact.  C351 consumes this
-- narrow predicate for its destructive operation.
CREATE FUNCTION ple_private.current_session_account_owns_draft_question(
    p_draft_question_uuid uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT p_draft_question_uuid IS NOT NULL
       AND (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
       AND EXISTS (
           SELECT 1
             FROM ple_private.draft_question AS question
             JOIN ple_private.authoring_workspace AS workspace
               ON workspace.authoring_workspace_id = question.authoring_workspace_id
            WHERE question.draft_question_id = p_draft_question_uuid
              AND workspace.owner_account_id = ple_api.current_session_account_id()
              AND workspace.revoked_at IS NULL
       )
$$;

CREATE FUNCTION ple_private.list_authoring_drafts()
RETURNS TABLE (
    draft_question_id uuid, draft_question_edit_number bigint,
    question_title text, question_description text, tags text[], question_license text,
    citation_text text, language text,
    parent_published_question_id text, parent_revision_number integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.draft_question_id, question.draft_question_edit_number,
           metadata.question_title, metadata.question_description, metadata.tags,
           metadata.question_license::text, metadata.citation_text,
           metadata.language, question.parent_published_question_id::text,
           question.parent_revision_number
      FROM ple_private.draft_question AS question
      JOIN ple_private.draft_question_metadata AS metadata
        ON metadata.draft_question_id = question.draft_question_id
     WHERE (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
       AND ple_api.current_session_account_can_access_authoring_workspace(question.authoring_workspace_id)
     ORDER BY question.updated_at DESC, question.draft_question_id DESC
$$;

CREATE FUNCTION ple_private.load_authoring_draft(p_draft_question_uuid uuid)
RETURNS TABLE (
    draft_question_id uuid, authoring_workspace_id uuid,
    draft_question_edit_number bigint, question_title text, question_description text,
    tags text[], question_license text, citation_text text,
    general_feedback text, hint text, worked_solution text, language text,
    author_names text[],
    question_backend text, question_format text, webwork_pg_path text,
    question_type text, object_record_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint,
    parent_published_question_id text, parent_revision_number integer,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT question.draft_question_id, question.authoring_workspace_id,
           question.draft_question_edit_number, metadata.question_title,
           metadata.question_description, metadata.tags, metadata.question_license::text,
           metadata.citation_text,
           metadata.general_feedback, metadata.hint,
           metadata.worked_solution, metadata.language,
           ARRAY(
               SELECT authorship.author_display_name
                 FROM ple_private.draft_question_authorship AS authorship
                WHERE authorship.draft_question_id = question.draft_question_id
                ORDER BY authorship.author_position
           ),
           binding.backend::text,
           binding.question_format::text, binding.webwork_pg_path,
           binding.question_type, record.object_record_id,
           record.object_address, record.sha256, record.size_bytes, record.media_type,
           pg_catalog.round(extract(epoch FROM record.created_at) * 1000)::bigint,
           question.parent_published_question_id::text, question.parent_revision_number,
           metadata.content_discipline_id, metadata.content_subject_id,
           metadata.content_topic_id, metadata.content_subtopic_id,
           metadata.bloom_cognitive_process::text, metadata.bloom_knowledge_dimension::text
      FROM ple_private.draft_question AS question
      JOIN ple_private.draft_question_metadata AS metadata
        ON metadata.draft_question_id = question.draft_question_id
      JOIN ple_private.draft_question_source_binding AS binding
        ON binding.draft_question_id = question.draft_question_id
      JOIN ple_private.object_record AS record
        ON record.object_record_id = binding.source_object_record_id
       AND binding.source_object_checksum = pg_catalog.encode(record.sha256, 'hex')
     WHERE p_draft_question_uuid IS NOT NULL
       AND (ple_api.current_session_account_is_instructor()
            OR ple_api.current_session_account_is_sysadmin())
       AND ple_api.current_session_account_can_access_authoring_workspace(question.authoring_workspace_id)
       AND question.draft_question_id = p_draft_question_uuid
       AND record.object_storage_area = 'private-content'
       AND record.object_data_class = 'authoring-content'
       AND record.object_address = pg_catalog.jsonb_build_object(
           'kind', 'workspaceQuestionSource', 'workspaceId', question.authoring_workspace_id,
           'objectId', record.object_record_id)
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.current_session_account_owns_draft_question(
    p_draft_question_uuid uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.current_session_account_owns_draft_question(p_draft_question_uuid)
$$;

CREATE FUNCTION ple_api.list_authoring_drafts()
RETURNS TABLE (
    draft_question_id uuid, draft_question_edit_number bigint,
    question_title text, question_description text, tags text[], question_license text,
    citation_text text, language text,
    parent_published_question_id text, parent_revision_number integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.list_authoring_drafts()
$$;

CREATE FUNCTION ple_api.load_authoring_draft(p_draft_question_uuid uuid)
RETURNS TABLE (
    draft_question_id uuid, authoring_workspace_id uuid,
    draft_question_edit_number bigint, question_title text, question_description text,
    tags text[], question_license text, citation_text text,
    general_feedback text, hint text, worked_solution text, language text,
    author_names text[],
    question_backend text, question_format text, webwork_pg_path text,
    question_type text, object_record_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint,
    parent_published_question_id text, parent_revision_number integer,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid,
    bloom_cognitive_process text, bloom_knowledge_dimension text
) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_authoring_draft(p_draft_question_uuid)
$$;
