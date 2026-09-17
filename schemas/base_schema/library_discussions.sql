-- Retained text-only stewardship discussions for stable Question Library lineages.
--
-- A thread records the exact immutable Revision current when it was created,
-- while its target remains the stable Question or Pool lineage.  The later
-- operation module is the only browser-facing mutation boundary.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

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
CREATE INDEX library_improvement_thread_object_idx
    ON ple_data.library_improvement_thread(object_kind, public_object_id, created_at, thread_id);

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
CREATE INDEX library_improvement_post_thread_idx
    ON ple_data.library_improvement_post(thread_id, created_at, post_id);

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
CREATE INDEX library_impact_notice_object_idx
    ON ple_data.library_impact_notice(object_kind, public_object_id, created_at DESC, impact_notice_id);

ALTER TABLE ple_data.library_improvement_thread ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.library_improvement_thread FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.library_improvement_post ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.library_improvement_post FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.library_impact_notice ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.library_impact_notice FORCE ROW LEVEL SECURITY;

CREATE POLICY library_improvement_thread_data_owner_access
    ON ple_data.library_improvement_thread FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);
CREATE POLICY library_improvement_post_data_owner_access
    ON ple_data.library_improvement_post FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);
CREATE POLICY library_impact_notice_data_owner_access
    ON ple_data.library_impact_notice FOR ALL TO ple_data_owner
    USING (true) WITH CHECK (true);

REVOKE ALL ON TABLE ple_data.library_improvement_thread,
    ple_data.library_improvement_post, ple_data.library_impact_notice FROM PUBLIC;

COMMENT ON TABLE ple_data.library_improvement_thread IS
    'Retained vetted-Instructor improvement thread targeting a stable Library Object lineage and its exact creation Revision.';
COMMENT ON TABLE ple_data.library_improvement_post IS
    'Retained text-only vetted-Instructor thread post with a visible creation-time verified display name.';
COMMENT ON TABLE ple_data.library_impact_notice IS
    'Question-owner- or Sysadmin-maintained retained impact notice; Pool administration is Sysadmin-only and cancelled notices remain historical.';

RESET ROLE;
