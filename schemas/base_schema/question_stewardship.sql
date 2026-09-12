-- Immutable revision credit and lineage stewardship.

SET LOCAL ROLE ple_private_owner;
GRANT SELECT ON ple_private.account, ple_private.account_state_event TO ple_data_owner;
CREATE POLICY account_question_stewardship_data_read ON ple_private.account
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY account_state_question_stewardship_data_read ON ple_private.account_state_event
    FOR SELECT TO ple_data_owner USING (true);
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;

CREATE TABLE ple_data.question_revision_acceptance (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    parent_revision_number integer,
    editor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    accepted_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    accepted_at timestamptz NOT NULL,
    reason_for_edit text NOT NULL CHECK (
        reason_for_edit = btrim(reason_for_edit)
        AND char_length(reason_for_edit) BETWEEN 1 AND 2000
        AND reason_for_edit !~ '[[:cntrl:]]'
    ),
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    FOREIGN KEY (question_id, parent_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK ((revision_number = 1 AND parent_revision_number IS NULL)
        OR (revision_number > 1 AND parent_revision_number BETWEEN 1 AND revision_number - 1))
);

CREATE TABLE ple_data.question_revision_authorship (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    author_position integer NOT NULL CHECK (author_position BETWEEN 1 AND 16),
    author_display_name text NOT NULL CHECK (
        author_display_name = btrim(author_display_name)
        AND char_length(author_display_name) BETWEEN 1 AND 120
        AND author_display_name !~ '[[:cntrl:]]'
    ),
    author_account_id uuid REFERENCES ple_private.account(account_id),
    PRIMARY KEY (question_id, revision_number, author_position),
    UNIQUE (question_id, revision_number, author_display_name),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_data.question_revision_license (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    spdx_expression text NOT NULL CHECK (spdx_expression IN (
        'CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0'
    )),
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number)
);

CREATE TABLE ple_data.question_revision_citation (
    question_id text NOT NULL,
    revision_number integer NOT NULL,
    citation_url text,
    citation_text text,
    PRIMARY KEY (question_id, revision_number),
    FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (NULLIF(btrim(citation_url), '') IS NOT NULL
        OR NULLIF(btrim(citation_text), '') IS NOT NULL),
    CHECK (citation_url IS NULL OR char_length(btrim(citation_url)) <= 2048),
    CHECK (citation_text IS NULL OR char_length(btrim(citation_text)) <= 4000)
);

CREATE TABLE ple_data.question_ownership_event (
    question_ownership_event_id uuid PRIMARY KEY,
    question_id text NOT NULL REFERENCES ple_data.published_question(question_id),
    owner_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('initial', 'transferred')),
    occurred_at timestamptz NOT NULL
);
CREATE UNIQUE INDEX question_ownership_event_initial_once
    ON ple_data.question_ownership_event(question_id) WHERE event_kind = 'initial';

CREATE TABLE ple_data.question_fork_source (
    forked_question_id text PRIMARY KEY REFERENCES ple_data.published_question(question_id),
    source_question_id text NOT NULL,
    source_revision_number integer NOT NULL CHECK (source_revision_number > 0),
    recorded_at timestamptz NOT NULL,
    FOREIGN KEY (source_question_id, source_revision_number)
        REFERENCES ple_data.question_revision(question_id, revision_number),
    CHECK (forked_question_id <> source_question_id)
);

CREATE FUNCTION ple_data.reject_question_stewardship_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question stewardship evidence is immutable';
END
$$;

CREATE FUNCTION ple_data.validate_question_revision_acceptance()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    published_at timestamptz;
BEGIN
    SELECT revision.published_at INTO published_at
      FROM ple_data.question_revision AS revision
     WHERE revision.question_id = NEW.question_id
       AND revision.revision_number = NEW.revision_number;
    IF published_at IS DISTINCT FROM NEW.accepted_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision acceptance time must match published time';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (SELECT state FROM ple_private.account_state_event
            WHERE account_id = account.account_id ORDER BY occurred_at DESC, event_id DESC LIMIT 1
        ) AS state_event ON state_event.state = 'active'
        WHERE account.account_id = NEW.editor_account_id AND account.product_role = 'instructor')
       OR NOT EXISTS (SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (SELECT state FROM ple_private.account_state_event
            WHERE account_id = account.account_id ORDER BY occurred_at DESC, event_id DESC LIMIT 1
        ) AS state_event ON state_event.state = 'active'
        WHERE account.account_id = NEW.accepted_by_account_id AND account.product_role = 'instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision editor and accepter must be Active Instructor Accounts';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_ownership_event()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    prior ple_data.question_ownership_event%ROWTYPE;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(NEW.question_id, 0));
    SELECT * INTO prior FROM ple_data.question_ownership_event
     WHERE question_id = NEW.question_id
     ORDER BY occurred_at DESC, question_ownership_event_id DESC LIMIT 1;
    IF NEW.event_kind = 'initial' AND FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Owner already exists';
    END IF;
    IF NEW.event_kind = 'transferred' AND (
        NOT FOUND OR NEW.recorded_by_account_id <> prior.owner_account_id
        OR NEW.occurred_at <= prior.occurred_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question ownership transfer requires the current owner and later time';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_private.account AS account
        JOIN LATERAL (SELECT state FROM ple_private.account_state_event
            WHERE account_id = account.account_id ORDER BY occurred_at DESC, event_id DESC LIMIT 1
        ) AS state_event ON state_event.state = 'active'
        WHERE account.account_id = NEW.owner_account_id AND account.product_role = 'instructor') THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Owner must be an Active Instructor Account';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_publication()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
DECLARE
    author_count integer;
    final_position integer;
BEGIN
    SELECT count(*), max(author_position) INTO author_count, final_position
      FROM ple_data.question_revision_authorship
     WHERE question_id = NEW.question_id AND revision_number = NEW.revision_number;
    IF author_count = 0 OR author_count <> final_position
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision_acceptance
           WHERE question_id = NEW.question_id AND revision_number = NEW.revision_number)
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_revision_license
           WHERE question_id = NEW.question_id AND revision_number = NEW.revision_number)
       OR NOT ple_private.question_revision_has_source_binding(
           NEW.question_id, NEW.revision_number)
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_current_owner
           WHERE question_id = NEW.question_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question publication requires acceptance, contiguous authorship, license, exact source, and owner';
    END IF;
    RETURN NEW;
END
$$;

CREATE VIEW ple_data.question_current_owner
WITH (security_barrier = true, security_invoker = true) AS
SELECT DISTINCT ON (question_id) question_id, owner_account_id,
    question_ownership_event_id, occurred_at
FROM ple_data.question_ownership_event
ORDER BY question_id, occurred_at DESC, question_ownership_event_id DESC;

CREATE TRIGGER question_revision_acceptance_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_acceptance
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_revision_authorship_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_authorship
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_revision_license_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_license
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_revision_citation_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_citation
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_ownership_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_ownership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_fork_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_stewardship_change();
CREATE TRIGGER question_revision_acceptance_is_valid
BEFORE INSERT ON ple_data.question_revision_acceptance
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_revision_acceptance();
CREATE TRIGGER question_ownership_event_is_valid
BEFORE INSERT ON ple_data.question_ownership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_ownership_event();
CREATE CONSTRAINT TRIGGER question_publication_is_complete
AFTER INSERT ON ple_data.question_publication_event
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
EXECUTE FUNCTION ple_data.validate_question_publication();

ALTER TABLE ple_data.question_revision_acceptance ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_acceptance FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_authorship ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_authorship FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_license ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_license FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_citation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_citation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_ownership_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_ownership_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_fork_source ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_fork_source FORCE ROW LEVEL SECURITY;

CREATE POLICY question_stewardship_data_owner_access ON ple_data.question_revision_acceptance
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_authorship_data_owner_access ON ple_data.question_revision_authorship
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_license_data_owner_access ON ple_data.question_revision_license
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_citation_data_owner_access ON ple_data.question_revision_citation
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_ownership_data_owner_access ON ple_data.question_ownership_event
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_fork_source_data_owner_access ON ple_data.question_fork_source
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY question_revision_acceptance_private_publication_insert
    ON ple_data.question_revision_acceptance FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_authorship_private_publication_insert
    ON ple_data.question_revision_authorship FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_license_private_publication_insert
    ON ple_data.question_revision_license FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_ownership_event_private_publication_insert
    ON ple_data.question_ownership_event FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_fork_source_private_publication_insert
    ON ple_data.question_fork_source FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_revision_acceptance_private_publication_read
    ON ple_data.question_revision_acceptance FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_revision_authorship_private_publication_read
    ON ple_data.question_revision_authorship FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_revision_license_private_publication_read
    ON ple_data.question_revision_license FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_ownership_event_private_publication_read
    ON ple_data.question_ownership_event FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_revision_acceptance_api_summary_read
    ON ple_data.question_revision_acceptance FOR SELECT TO ple_api_owner USING (true);
REVOKE ALL ON ALL TABLES IN SCHEMA ple_data FROM PUBLIC;
GRANT INSERT, SELECT ON ple_data.question_revision_acceptance,
    ple_data.question_revision_authorship, ple_data.question_revision_license,
    ple_data.question_ownership_event, ple_data.question_fork_source TO ple_private_owner;
GRANT SELECT ON ple_data.question_current_owner TO ple_private_owner;
GRANT SELECT ON ple_data.question_revision_acceptance TO ple_api_owner;
REVOKE ALL ON FUNCTION ple_data.reject_question_stewardship_change(),
    ple_data.validate_question_revision_acceptance(),
    ple_data.validate_question_ownership_event(), ple_data.validate_question_publication() FROM PUBLIC;
RESET ROLE;
