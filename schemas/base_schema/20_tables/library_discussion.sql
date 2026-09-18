-- library_discussion tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.library_improvement_thread (
    thread_id uuid PRIMARY KEY,
    object_kind text NOT NULL CHECK (object_kind IN ('question', 'question_pool')),
    public_object_id text NOT NULL CHECK (
        public_object_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(public_object_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(public_object_id, 1, 4) || substr(public_object_id, 7, 3)
        )
    ),
    creation_revision_number bigint NOT NULL CHECK (creation_revision_number > 0),
    created_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    created_at timestamptz NOT NULL,
    state text NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'resolved')),
    resolved_by_account_id uuid REFERENCES ple_private.account(account_id),
    resolved_at timestamptz,
    CHECK ((state = 'open' AND resolved_by_account_id IS NULL AND resolved_at IS NULL)
        OR (state = 'resolved' AND resolved_by_account_id IS NOT NULL
            AND resolved_at IS NOT NULL AND resolved_at >= created_at))
);

CREATE TABLE ple_data.library_improvement_post (
    post_id uuid PRIMARY KEY,
    thread_id uuid NOT NULL REFERENCES ple_data.library_improvement_thread(thread_id),
    author_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    author_display_name text NOT NULL CHECK (
        author_display_name = btrim(author_display_name)
        AND char_length(author_display_name) BETWEEN 1 AND 200
        AND author_display_name !~ '[[:cntrl:]]'
    ),
    body text NOT NULL CHECK (
        body = btrim(body) AND char_length(body) BETWEEN 1 AND 4000 AND body !~ '[[:cntrl:]]'
    ),
    created_at timestamptz NOT NULL,
    updated_at timestamptz
        CHECK (updated_at IS NULL OR updated_at >= created_at)
);

CREATE TABLE ple_data.library_impact_notice (
    impact_notice_id uuid PRIMARY KEY,
    object_kind text NOT NULL CHECK (object_kind IN ('question', 'question_pool')),
    public_object_id text NOT NULL CHECK (
        public_object_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(public_object_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(public_object_id, 1, 4) || substr(public_object_id, 7, 3)
        )
    ),
    affected_revision_number bigint CHECK (affected_revision_number > 0),
    created_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    author_display_name text NOT NULL CHECK (
        author_display_name = btrim(author_display_name)
        AND char_length(author_display_name) BETWEEN 1 AND 200
        AND author_display_name !~ '[[:cntrl:]]'
    ),
    body text NOT NULL CHECK (
        body = btrim(body) AND char_length(body) BETWEEN 1 AND 4000 AND body !~ '[[:cntrl:]]'
    ),
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL CHECK (updated_at >= created_at),
    state text NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'cancelled')),
    cancelled_by_account_id uuid REFERENCES ple_private.account(account_id),
    cancelled_at timestamptz,
    CHECK ((state = 'active' AND cancelled_by_account_id IS NULL AND cancelled_at IS NULL)
        OR (state = 'cancelled' AND cancelled_by_account_id IS NOT NULL
            AND cancelled_at IS NOT NULL AND cancelled_at >= created_at
            AND updated_at = cancelled_at))
);

COMMENT ON TABLE ple_data.library_improvement_thread IS 'role: current state, Retained vetted-Instructor improvement thread targeting a stable Library Object lineage and its exact creation Revision.';

COMMENT ON TABLE ple_data.library_improvement_post IS 'role: event, Retained text-only vetted-Instructor thread post with a visible creation-time verified display name.';

COMMENT ON TABLE ple_data.library_impact_notice IS 'role: event, Question-owner- or Sysadmin-maintained retained impact notice; Pool administration is Sysadmin-only and cancelled notices remain historical.';

