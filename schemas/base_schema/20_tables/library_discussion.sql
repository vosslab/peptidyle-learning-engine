-- library_impact_notice tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.library_impact_notice (
    impact_notice_id uuid PRIMARY KEY,
    object_kind ple_data.library_object_kind NOT NULL,
    public_object_id text NOT NULL CHECK (
        public_object_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
        AND substr(public_object_id, 6, 1) = ple_private.crockford_checksum_character(
            substr(public_object_id, 1, 4) || substr(public_object_id, 7, 3)
        )
    ),
    affected_revision_number integer CHECK (affected_revision_number > 0),
    created_by_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
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
    state ple_data.notice_state NOT NULL DEFAULT 'active',
    cancelled_by_account_id ple_data.account_id REFERENCES ple_private.account(account_id),
    cancelled_at timestamptz,
    CHECK ((state = 'active' AND cancelled_by_account_id IS NULL AND cancelled_at IS NULL)
        OR (state = 'cancelled' AND cancelled_by_account_id IS NOT NULL
            AND cancelled_at IS NOT NULL AND cancelled_at >= created_at
            AND updated_at = cancelled_at))
);

COMMENT ON TABLE ple_data.library_impact_notice IS 'role: event, Question-owner- or Sysadmin-maintained retained impact notice; Pool notice authorship remains an unresolved product seam and cancelled notices remain historical.';
COMMENT ON COLUMN ple_data.library_impact_notice.affected_revision_number IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.library_impact_notice.cancelled_by_account_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.library_impact_notice.cancelled_at IS 'NULL means this optional fact is absent.';
