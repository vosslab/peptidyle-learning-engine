-- A client timeout after dispatching an effectful provider POST cannot prove
-- that the upstream operation stopped. Fence the whole attempt permanently so
-- launch replacement, retry, and a second replica cannot duplicate it.
ALTER TABLE public.question_attempt
    ADD COLUMN external_tool_indeterminate_at timestamp with time zone,
    ADD COLUMN external_tool_indeterminate_token_sha256 bytea,
    ADD CONSTRAINT question_attempt_external_tool_indeterminate_shape_check CHECK (
        (external_tool_indeterminate_at IS NULL)
        = (external_tool_indeterminate_token_sha256 IS NULL)
        AND (external_tool_indeterminate_token_sha256 IS NULL
             OR octet_length(external_tool_indeterminate_token_sha256) = 32)
    );

CREATE INDEX question_attempt_external_tool_indeterminate_idx
    ON public.question_attempt (tenant_id, attempt_id)
    WHERE external_tool_indeterminate_at IS NOT NULL;
