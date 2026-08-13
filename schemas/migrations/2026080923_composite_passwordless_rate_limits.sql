-- Independent budgets prevent one attacker-controlled identity from becoming
-- the authority that locks out a mailbox or account.
ALTER TABLE public.authentication_rate_limit
    DROP CONSTRAINT authentication_rate_limit_scope_check,
    ADD CONSTRAINT authentication_rate_limit_scope_check CHECK (
        limit_scope IN ('email', 'network', 'principal', 'service')
    );

-- This pre-production system has no users. Remove the ten-minute development
-- challenges rather than retaining a legacy completion path that cannot prove
-- which email budget it may release.
DELETE FROM public.email_authentication_challenge;

-- New challenges always carry this opaque server HMAC.
ALTER TABLE public.email_authentication_challenge
    ADD COLUMN rate_limit_key_hash bytea NOT NULL,
    ADD CONSTRAINT email_authentication_challenge_rate_limit_key_check CHECK (
        octet_length(rate_limit_key_hash) = 32
    );
