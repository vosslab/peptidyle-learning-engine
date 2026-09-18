-- Privileges from authentication.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL PRIVILEGES ON TABLE ple_private.account_authentication_email,
    ple_private.authentication_rate_limit, ple_private.email_authentication_challenge,
    ple_private.passkey_ceremony, ple_private.passkey, ple_private.sysadmin_totp_credential,
    ple_private.sysadmin_totp_attestation, ple_private.sysadmin_totp_used_counter,
    ple_private.sysadmin_totp_verification_attempt,
    ple_private.authenticated_session FROM PUBLIC;

REVOKE ALL PRIVILEGES ON FUNCTION ple_private.enforce_account_authentication_email_role(),
    ple_private.reject_authenticated_session_identity_change(),
    ple_private.revoke_sessions_after_account_deactivation_or_closure(),
    ple_private.resolve_active_authenticated_session(bytea),
    ple_private.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_private.revoke_authenticated_session(bytea),
    ple_private.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_private.consume_passkey_authentication(uuid, bytea, bytea),
    ple_private.provision_sysadmin_totp_credential(uuid, text, bytea, bytea),
    ple_private.create_pending_sysadmin_totp_attestation(uuid, uuid, bytea, bigint),
    ple_private.load_pending_sysadmin_totp_attestation(uuid, bytea),
    ple_private.reserve_sysadmin_totp_verification_attempt(uuid, bytea),
    ple_private.consume_sysadmin_totp_attestation_into_session(uuid, bytea, bigint, uuid, bytea, bigint)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.resolve_active_authenticated_session(bytea),
    ple_private.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_private.revoke_authenticated_session(bytea),
    ple_private.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_private.consume_passkey_authentication(uuid, bytea, bytea),
    ple_private.provision_sysadmin_totp_credential(uuid, text, bytea, bytea),
    ple_private.create_pending_sysadmin_totp_attestation(uuid, uuid, bytea, bigint),
    ple_private.load_pending_sysadmin_totp_attestation(uuid, bytea),
    ple_private.reserve_sysadmin_totp_verification_attempt(uuid, bytea),
    ple_private.consume_sysadmin_totp_attestation_into_session(uuid, bytea, bigint, uuid, bytea, bigint)
    TO ple_api_owner;

SET LOCAL ROLE ple_api_owner;

REVOKE ALL PRIVILEGES ON FUNCTION ple_api.resolve_and_install_session(bytea),
    ple_api.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_api.revoke_authenticated_session(bytea),
    ple_api.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_api.consume_passkey_authentication(uuid, bytea, bytea),
    ple_api.provision_sysadmin_totp_credential(uuid, text, bytea, bytea),
    ple_api.create_pending_sysadmin_totp_attestation(uuid, uuid, bytea, bigint),
    ple_api.load_pending_sysadmin_totp_attestation(uuid, bytea),
    ple_api.reserve_sysadmin_totp_verification_attempt(uuid, bytea),
    ple_api.consume_sysadmin_totp_attestation_into_session(uuid, bytea, bigint, uuid, bytea, bigint)
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_api.resolve_and_install_session(bytea),
    ple_api.create_authenticated_session(uuid, uuid, bytea, bigint),
    ple_api.revoke_authenticated_session(bytea),
    ple_api.consume_email_authentication_challenge(uuid, bytea, bytea),
    ple_api.consume_passkey_authentication(uuid, bytea, bytea),
    ple_api.provision_sysadmin_totp_credential(uuid, text, bytea, bytea),
    ple_api.create_pending_sysadmin_totp_attestation(uuid, uuid, bytea, bigint),
    ple_api.load_pending_sysadmin_totp_attestation(uuid, bytea),
    ple_api.reserve_sysadmin_totp_verification_attempt(uuid, bytea),
    ple_api.consume_sysadmin_totp_attestation_into_session(uuid, bytea, bigint, uuid, bytea, bigint)
    TO ple_auth;

