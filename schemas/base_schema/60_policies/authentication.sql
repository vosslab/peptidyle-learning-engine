-- Row security policies from authentication.sql.

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.account_authentication_email ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.account_authentication_email FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.authentication_rate_limit ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.authentication_rate_limit FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.email_authentication_challenge ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.email_authentication_challenge FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.passkey_ceremony ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.passkey_ceremony FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.passkey ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.passkey FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_credential ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_credential FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_attestation ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_attestation FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_used_counter ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_used_counter FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_verification_attempt ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.sysadmin_totp_verification_attempt FORCE ROW LEVEL SECURITY;

ALTER TABLE ple_private.authenticated_session ENABLE ROW LEVEL SECURITY;

ALTER TABLE ple_private.authenticated_session FORCE ROW LEVEL SECURITY;

CREATE POLICY account_authentication_email_private_owner_access ON ple_private.account_authentication_email
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY authentication_rate_limit_private_owner_access ON ple_private.authentication_rate_limit
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY email_authentication_challenge_private_owner_access ON ple_private.email_authentication_challenge
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY passkey_ceremony_private_owner_access ON ple_private.passkey_ceremony
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY passkey_private_owner_access ON ple_private.passkey
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY sysadmin_totp_credential_private_owner_access ON ple_private.sysadmin_totp_credential
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY sysadmin_totp_attestation_private_owner_access ON ple_private.sysadmin_totp_attestation
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY sysadmin_totp_used_counter_private_owner_access ON ple_private.sysadmin_totp_used_counter
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY sysadmin_totp_verification_attempt_private_owner_access ON ple_private.sysadmin_totp_verification_attempt
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

CREATE POLICY authenticated_session_private_owner_access ON ple_private.authenticated_session
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);

