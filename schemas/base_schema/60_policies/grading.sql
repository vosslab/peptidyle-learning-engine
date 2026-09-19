-- Row security policies from grading.sql.

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.automated_grading_receipt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_audit.automated_grading_receipt FORCE ROW LEVEL SECURITY;

CREATE POLICY automated_grading_receipt_audit_owner_access
    ON ple_audit.automated_grading_receipt
    FOR ALL TO ple_audit_owner USING (true) WITH CHECK (true);

CREATE POLICY automated_grading_receipt_api_owner_read
    ON ple_audit.automated_grading_receipt
    FOR SELECT TO ple_api_owner USING (true);

CREATE POLICY automated_grading_receipt_private_owner_insert
    ON ple_audit.automated_grading_receipt
    FOR INSERT TO ple_private_owner WITH CHECK (true);

CREATE POLICY automated_grading_receipt_private_owner_read
    ON ple_audit.automated_grading_receipt
    FOR SELECT TO ple_private_owner USING (true);

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.grading_result ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.grading_result FORCE ROW LEVEL SECURITY;

CREATE POLICY grading_result_private_owner_access
    ON ple_private.grading_result FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

