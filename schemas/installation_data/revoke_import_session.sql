-- ASVS 7.4.1: revoke only the exact temporary import session supplied through private stdin.
SET LOCAL ROLE ple_private_owner;
UPDATE ple_private.authenticated_session
   SET revoked_at = coalesce(revoked_at, clock_timestamp())
 WHERE session_id = :'import_session_id'::uuid
   AND token_hash = decode(:'import_session_token_hash', 'hex');
RESET ROLE;
