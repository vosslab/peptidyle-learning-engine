-- A staged provider verdict remains committed only by the replica lease that
-- obtained and verified it.  Keep a digest, never the expired lease secret.
ALTER TABLE external_tool_exchange
    ADD COLUMN verification_token_sha256 bytea;

ALTER TABLE external_tool_exchange
    ADD CONSTRAINT external_tool_exchange_verified_token_check CHECK (
        (state = 'verified_pending') = (verification_token_sha256 IS NOT NULL)
    );
