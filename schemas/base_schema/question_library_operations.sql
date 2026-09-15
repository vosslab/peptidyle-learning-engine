SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.question_library_entries(
    p_question_id text DEFAULT NULL, p_revision_number integer DEFAULT NULL,
    p_require_available boolean DEFAULT true
) RETURNS TABLE (
    question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    question_license text, availability text,
    availability_edit_number bigint, source_object_id uuid, source_object_checksum text,
    source_media_type text
) LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor Account';
    END IF;
    RETURN QUERY
    SELECT revision.question_id, revision.revision_number, revision.backend, binding.question_format, revision.question_type,
           floor(extract(epoch FROM revision.published_at) * 1000)::bigint,
           metadata.question_title, metadata.question_description,
           ARRAY(SELECT authorship.author_display_name
               FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.question_id = revision.question_id
                AND authorship.revision_number = revision.revision_number
              ORDER BY authorship.author_position),
           EXISTS (SELECT 1 FROM ple_data.question_revision_authorship AS authorship
              WHERE authorship.question_id = revision.question_id
                AND authorship.revision_number = revision.revision_number
                AND authorship.author_account_id = ple_api.current_session_account_id()),
           EXISTS (SELECT 1 FROM ple_data.question_current_owner AS owner
              WHERE owner.question_id = revision.question_id
                AND owner.owner_account_id = ple_api.current_session_account_id()),
           license.spdx_expression, lineage.availability, lineage.availability_edit_number,
           binding.source_object_id, binding.source_object_checksum, record.media_type
      FROM ple_data.question_revision AS revision
      JOIN ple_data.published_question AS lineage ON lineage.question_id = revision.question_id
      JOIN ple_data.published_question_metadata AS metadata ON metadata.question_id = revision.question_id
      JOIN ple_data.question_revision_license AS license
        ON license.question_id = revision.question_id AND license.revision_number = revision.revision_number
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = revision.question_id AND binding.revision_number = revision.revision_number
      JOIN ple_private.object_record AS record ON record.object_id = binding.source_object_id
     WHERE (p_question_id IS NULL OR revision.question_id = p_question_id)
       AND (p_revision_number IS NULL OR revision.revision_number = p_revision_number)
       AND (p_revision_number IS NOT NULL OR revision.revision_number = (
           SELECT max(accepted.revision_number)
             FROM ple_data.question_revision_acceptance AS accepted
            WHERE accepted.question_id = revision.question_id
       ))
       AND (NOT p_require_available OR lineage.availability = 'available')
     ORDER BY metadata.question_title, revision.question_id, revision.revision_number;
END
$$;
REVOKE ALL ON FUNCTION ple_private.question_library_entries(text, integer, boolean) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.question_library_entries(text, integer, boolean) TO ple_api_owner;
RESET ROLE;

SET LOCAL ROLE ple_api_owner;
CREATE VIEW ple_api.published_question_summary
WITH (security_barrier = true, security_invoker = false) AS
SELECT lineage.question_id, revision.revision_number AS latest_question_revision_number,
       revision.backend, revision.question_type, revision.published_at, metadata.question_title,
       metadata.question_description, metadata.language, lineage.availability,
       lineage.availability_edit_number
  FROM ple_data.published_question AS lineage
  JOIN ple_data.published_question_metadata AS metadata ON metadata.question_id = lineage.question_id
  JOIN LATERAL (
      SELECT accepted.revision_number FROM ple_data.question_revision_acceptance AS accepted
       WHERE accepted.question_id = lineage.question_id
       ORDER BY accepted.revision_number DESC LIMIT 1
  ) AS latest ON true
  JOIN ple_data.question_revision AS revision
    ON revision.question_id = lineage.question_id AND revision.revision_number = latest.revision_number;
CREATE FUNCTION ple_api.list_question_library_entries()
RETURNS TABLE (
    question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    question_license text, availability text,
    availability_edit_number bigint, source_object_id uuid, source_object_checksum text,
    source_media_type text
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(NULL, NULL, true)
$$;
CREATE FUNCTION ple_api.load_question_library_revision(p_question_id text, p_revision_number integer)
RETURNS TABLE (
    question_id text, revision_number integer, backend text, question_format text, question_type text, published_at_millis bigint,
    question_title text, question_description text, author_names text[],
    authored_by_current_account boolean, viewer_may_archive boolean,
    question_license text, availability text,
    availability_edit_number bigint, source_object_id uuid, source_object_checksum text,
    source_media_type text
) LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.question_library_entries(p_question_id, p_revision_number, false)
$$;
REVOKE ALL ON TABLE ple_api.published_question_summary FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_revision(text, integer) FROM PUBLIC;
GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;
GRANT EXECUTE ON FUNCTION ple_api.list_question_library_entries(),
    ple_api.load_question_library_revision(text, integer) TO ple_app;
RESET ROLE;
