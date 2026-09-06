-- Instructor-authorized Question Library server-read boundary.
--
-- The function returns immutable source-binding evidence only to the API
-- process. The Server Route resolves that evidence server-side and never
-- serializes an Object Record, Object Address, checksum, or source bytes.

SET LOCAL ROLE ple_data_owner;

GRANT SELECT ON TABLE
    ple_data.question_revision_authorship,
    ple_data.question_revision_license,
    ple_data.question_revision_availability_event
TO ple_api_owner;

CREATE POLICY question_revision_authorship_api_owner_read
    ON ple_data.question_revision_authorship FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_revision_license_api_owner_read
    ON ple_data.question_revision_license FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_revision_availability_event_api_owner_read
    ON ple_data.question_revision_availability_event FOR SELECT TO ple_api_owner USING (true);

RESET ROLE;

SET LOCAL ROLE ple_private_owner;

GRANT SELECT ON TABLE ple_private.object_record TO ple_api_owner;
-- The Server Route needs the exact content type of the already-bound private
-- Question Source.  This does not permit browsing arbitrary private Objects:
-- source bindings remain the only join path and the policy excludes student,
-- workspace, temporary, and public records.
CREATE POLICY object_record_api_owner_question_source_read
    ON ple_private.object_record FOR SELECT TO ple_api_owner
    USING (
        object_storage_area = 'private-content'
        AND object_data_class = 'question-source'
    );

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.list_question_library_entries()
RETURNS TABLE (
    question_id text,
    revision_number integer,
    backend text,
    published_at_millis bigint,
    question_title text,
    question_description text,
    author_names text[],
    authored_by_current_account boolean,
    question_license text,
    availability text,
    availability_reason text,
    source_object_id uuid,
    source_object_checksum text,
    source_media_type text
)
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private
AS $$
BEGIN
    IF NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Question Library requires an active Instructor Account';
    END IF;

    RETURN QUERY
    SELECT summary.question_id,
           summary.latest_question_revision_number,
           summary.backend,
           floor(extract(epoch FROM summary.published_at) * 1000)::bigint,
           summary.question_title,
           summary.question_description,
           ARRAY(
               SELECT authorship.author_display_name
                 FROM ple_data.question_revision_authorship AS authorship
                WHERE authorship.question_id = summary.question_id
                  AND authorship.revision_number = summary.latest_question_revision_number
                ORDER BY authorship.author_position
           ),
           EXISTS (
               SELECT 1
                 FROM ple_data.question_revision_authorship AS authorship
                WHERE authorship.question_id = summary.question_id
                  AND authorship.revision_number = summary.latest_question_revision_number
                  AND authorship.author_account_id = ple_api.current_session_account_id()
           ),
           license.spdx_expression,
           availability.availability,
           availability.reason,
           binding.source_object_id,
           binding.source_object_checksum,
           records.media_type
      FROM ple_api.published_question_summary AS summary
      JOIN ple_data.question_revision_license AS license
        ON license.question_id = summary.question_id
       AND license.revision_number = summary.latest_question_revision_number
      JOIN LATERAL (
          SELECT event.availability, event.reason
            FROM ple_data.question_revision_availability_event AS event
           WHERE event.question_id = summary.question_id
             AND event.revision_number = summary.latest_question_revision_number
           ORDER BY event.occurred_at DESC, event.event_id DESC
           LIMIT 1
      ) AS availability ON true
      JOIN ple_private.question_revision_source_binding AS binding
        ON binding.question_id = summary.question_id
       AND binding.revision_number = summary.latest_question_revision_number
      JOIN ple_private.object_record AS records
        ON records.object_id = binding.source_object_id
     ORDER BY summary.question_title, summary.question_id;
END
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.list_question_library_entries() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.list_question_library_entries() TO ple_app;

COMMENT ON FUNCTION ple_api.list_question_library_entries() IS
    'Returns active-Instructor Published Question metadata and server-only immutable source bindings for Question Library read assembly.';

RESET ROLE;
