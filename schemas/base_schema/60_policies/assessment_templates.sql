-- Row security policies from assessment_templates.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.assessment_template ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.assessment_template FORCE ROW LEVEL SECURITY;

CREATE POLICY assessment_template_private_owner_access
    ON ple_private.assessment_template FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);

CREATE POLICY assessment_template_api_owner_access
    ON ple_private.assessment_template FOR ALL TO ple_api_owner
    USING (
        owner_account_id = ple_api.current_session_account_id()
        AND ple_api.current_session_account_is_instructor()
    )
    WITH CHECK (
        owner_account_id = ple_api.current_session_account_id()
        AND ple_api.current_session_account_is_instructor()
    );

