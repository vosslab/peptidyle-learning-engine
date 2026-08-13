-- Durable, short-lived ownership for a single provider activity call.
--
-- The opaque launch cookie remains the browser proof. A replica first claims
-- a random activity capability whose digest alone is persisted, performs the
-- remote call without a database transaction, then releases that exact lease.
-- Submission/revocation check the expiry so an in-flight call cannot be
-- consumed concurrently; abandoned work recovers automatically after expiry.

ALTER TABLE public.external_tool_launch_session
    ADD COLUMN activity_lease_token_sha256 bytea,
    ADD COLUMN activity_lease_expires_at timestamp with time zone;

ALTER TABLE public.external_tool_launch_session
    ADD CONSTRAINT external_tool_launch_session_activity_lease_shape_check
    CHECK (
        (activity_lease_token_sha256 IS NULL) = (activity_lease_expires_at IS NULL)
        AND (activity_lease_token_sha256 IS NULL OR octet_length(activity_lease_token_sha256) = 32)
    );

CREATE INDEX external_tool_launch_session_activity_lease_idx
    ON public.external_tool_launch_session (activity_lease_expires_at)
    WHERE activity_lease_expires_at IS NOT NULL;
