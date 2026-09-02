-- Question Summary derives one Latest Question Revision from immutable
-- acceptance evidence. Availability remains a separate relationship.

SET LOCAL ROLE ple_api_owner;

-- ASVS 2.2.1 and 8.3.1: an absent or malformed session setting is an
-- unauthenticated context, never a database-cast failure or inferred identity.
CREATE OR REPLACE FUNCTION ple_api.current_session_account_id()
RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    WITH configured AS (
        SELECT pg_catalog.current_setting('ple.session_account_id', true) AS raw_account_id
    )
    SELECT account.account_id
      FROM configured
      JOIN ple_private.account AS account
        ON account.account_id = CASE
            WHEN configured.raw_account_id ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
                THEN configured.raw_account_id::uuid
        END
$$;

-- ASVS 8.1.1, 8.2.1, and 8.3.1: the answer-free Question Library projection
-- authorizes the current authenticated Instructor at the trusted database
-- boundary; a browser cannot select a Product Role or a revision number.
CREATE FUNCTION ple_api.current_session_account_is_instructor()
RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private
AS $$
    SELECT EXISTS (
        SELECT 1
          FROM ple_private.account AS account
         WHERE account.account_id = ple_api.current_session_account_id()
           AND account.role = 'instructor'
    )
$$;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.current_session_account_is_instructor() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.current_session_account_is_instructor() TO ple_app;

RESET ROLE;

SET LOCAL ROLE ple_data_owner;

-- The API-owned view is the sole answer-free reader of these durable facts.
-- ASVS 8.2.2: no `ple_app` table grant exposes the underlying records.
GRANT SELECT ON TABLE ple_data.question_revision_acceptance TO ple_api_owner;
CREATE POLICY published_question_summary_api_owner_read
    ON ple_data.published_question FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_revision_summary_api_owner_read
    ON ple_data.question_revision FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY question_revision_acceptance_summary_api_owner_read
    ON ple_data.question_revision_acceptance FOR SELECT TO ple_api_owner USING (true);

RESET ROLE;

SET LOCAL ROLE ple_api_owner;

ALTER VIEW ple_api.published_question_summary
    RENAME COLUMN revision_number TO latest_question_revision_number;

CREATE OR REPLACE VIEW ple_api.published_question_summary
WITH (security_barrier = true) AS
SELECT questions.question_id,
       latest_acceptance.revision_number AS latest_question_revision_number,
       versions.backend,
       versions.published_at,
       versions.question_description,
       versions.public_metadata
  FROM ple_data.published_question AS questions
  JOIN LATERAL (
      SELECT acceptance.revision_number
        FROM ple_data.question_revision_acceptance AS acceptance
       WHERE acceptance.question_id = questions.question_id
       ORDER BY acceptance.revision_number DESC
       LIMIT 1
  ) AS latest_acceptance ON true
  JOIN ple_data.question_revision AS versions
    ON versions.question_id = questions.question_id
   AND versions.revision_number = latest_acceptance.revision_number
 WHERE ple_api.current_session_account_is_instructor();

REVOKE ALL PRIVILEGES ON TABLE ple_api.published_question_summary FROM PUBLIC;
GRANT SELECT ON TABLE ple_api.published_question_summary TO ple_app;

COMMENT ON VIEW ple_api.published_question_summary IS
    'Answer-free Instructor Question Summary projection. Latest Question Revision is the greatest accepted Question Revision Number in one Question lineage; Question Revision Availability remains independent.';

RESET ROLE;
