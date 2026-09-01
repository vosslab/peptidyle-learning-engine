-- SD1 immutable Question Authorship, Question Owner, Question License, and
-- Question Citation relations. These facts are distinct from Question Source,
-- revision publication, and ordinary Question metadata.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.question_revision_acceptance (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    parent_revision_number integer,
    editor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    accepted_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    accepted_at timestamp with time zone NOT NULL,
    reason_for_edit text NOT NULL CHECK (char_length(btrim(reason_for_edit)) BETWEEN 1 AND 2000),
    PRIMARY KEY (question_id, revision_number),
    CONSTRAINT question_revision_acceptance_revision_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_revision_acceptance_parent_matches FOREIGN KEY (question_id, parent_revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_revision_acceptance_parent_precedes_revision CHECK (
        (revision_number = 1 AND parent_revision_number IS NULL)
        OR (revision_number > 1 AND parent_revision_number BETWEEN 1 AND revision_number - 1)
    )
);

CREATE TABLE ple_data.question_revision_authorship (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    author_position integer NOT NULL CHECK (author_position > 0),
    author_display_name text NOT NULL CHECK (char_length(btrim(author_display_name)) BETWEEN 1 AND 120),
    author_account_id uuid REFERENCES ple_private.account (account_id),
    PRIMARY KEY (question_id, revision_number, author_position),
    CONSTRAINT question_revision_authorship_revision_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_revision_authorship_display_name_is_unique
        UNIQUE (question_id, revision_number, author_display_name)
);

CREATE TABLE ple_data.question_revision_license (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    spdx_expression text NOT NULL CHECK (spdx_expression IN (
        'CC0-1.0', 'CC-BY-4.0', 'CC-BY-SA-4.0'
    )),
    PRIMARY KEY (question_id, revision_number),
    CONSTRAINT question_revision_license_revision_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);

CREATE TABLE ple_data.question_revision_citation (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    citation_url text,
    citation_text text,
    PRIMARY KEY (question_id, revision_number),
    CONSTRAINT question_revision_citation_revision_matches FOREIGN KEY (question_id, revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number),
    CONSTRAINT question_revision_citation_has_text_or_url CHECK (
        NULLIF(btrim(citation_url), '') IS NOT NULL
        OR NULLIF(btrim(citation_text), '') IS NOT NULL
    ),
    CONSTRAINT question_revision_citation_url_is_bounded CHECK (
        citation_url IS NULL OR char_length(btrim(citation_url)) <= 2048
    ),
    CONSTRAINT question_revision_citation_text_is_bounded CHECK (
        citation_text IS NULL OR char_length(btrim(citation_text)) <= 4000
    )
);

CREATE TABLE ple_data.question_ownership_event (
    question_ownership_event_id uuid PRIMARY KEY,
    question_id text NOT NULL REFERENCES ple_data.published_question (question_id),
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('initial', 'transferred')),
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT question_ownership_event_initial_is_unique UNIQUE NULLS NOT DISTINCT (question_id, event_kind)
);

CREATE FUNCTION ple_data.reject_question_credit_and_stewardship_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question credit and stewardship records are immutable';
END
$$;

CREATE FUNCTION ple_data.validate_question_ownership_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(NEW.question_id, 0)
    );

    IF NEW.event_kind = 'initial' THEN
        IF EXISTS (
            SELECT 1 FROM ple_data.question_ownership_event AS earlier
            WHERE earlier.question_id = NEW.question_id
        ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'a Question Owner initial event must be the first ownership event';
        END IF;
    ELSIF NOT EXISTS (
        SELECT 1 FROM ple_data.question_ownership_event AS earlier
        WHERE earlier.question_id = NEW.question_id
          AND earlier.occurred_at <= NEW.occurred_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Question Owner transfer requires an earlier ownership event';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM ple_private.account AS account
        WHERE account.account_id = NEW.owner_account_id
          AND account.role = 'instructor'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Question Owner must be an Active Instructor Account';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_revision_acceptance()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    revision_published_at timestamp with time zone;
BEGIN
    SELECT revision.published_at
      INTO revision_published_at
      FROM ple_data.question_revision AS revision
     WHERE revision.question_id = NEW.question_id
       AND revision.revision_number = NEW.revision_number;

    IF revision_published_at IS DISTINCT FROM NEW.accepted_at THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Accepted At must match its accepted Question Revision';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM ple_private.account AS account
        WHERE account.account_id = NEW.editor_account_id AND account.role = 'instructor'
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_private.account AS account
        WHERE account.account_id = NEW.accepted_by_account_id AND account.role = 'instructor'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Editor and Question Revision Accepted By must be Instructor Accounts';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_data.validate_question_publication_credit()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM ple_data.question_revision_authorship AS authorship
        WHERE authorship.question_id = NEW.question_id
          AND authorship.revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires Question Authorship';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM ple_data.question_revision_acceptance AS acceptance
        WHERE acceptance.question_id = NEW.question_id
          AND acceptance.revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires Question Revision acceptance';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM ple_data.question_revision_license AS license
        WHERE license.question_id = NEW.question_id
          AND license.revision_number = NEW.revision_number
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires a compatible Question License';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM ple_data.question_current_owner AS owner
        WHERE owner.question_id = NEW.question_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Publication requires one current Question Owner';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_revision_authorship_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_authorship
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_credit_and_stewardship_change();
CREATE TRIGGER question_revision_acceptance_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_acceptance
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_credit_and_stewardship_change();
CREATE TRIGGER question_revision_acceptance_is_valid
BEFORE INSERT ON ple_data.question_revision_acceptance
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_revision_acceptance();
CREATE TRIGGER question_revision_license_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_license
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_credit_and_stewardship_change();
CREATE TRIGGER question_revision_citation_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_revision_citation
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_credit_and_stewardship_change();
CREATE TRIGGER question_ownership_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_ownership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_credit_and_stewardship_change();
CREATE TRIGGER question_ownership_event_has_valid_transition
BEFORE INSERT ON ple_data.question_ownership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_ownership_event();
CREATE CONSTRAINT TRIGGER question_publication_event_has_required_credit
AFTER INSERT ON ple_data.question_publication_event
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_publication_credit();

CREATE VIEW ple_data.question_current_owner
WITH (security_barrier = true, security_invoker = true) AS
SELECT DISTINCT ON (event.question_id)
    event.question_id,
    event.owner_account_id,
    event.question_ownership_event_id,
    event.occurred_at
FROM ple_data.question_ownership_event AS event
ORDER BY event.question_id, event.occurred_at DESC, event.question_ownership_event_id DESC;

ALTER TABLE ple_data.question_revision_authorship ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_authorship FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_acceptance ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_acceptance FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_license ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_license FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_citation ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_revision_citation FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_ownership_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_ownership_event FORCE ROW LEVEL SECURITY;

REVOKE ALL PRIVILEGES ON TABLE ple_data.question_revision_acceptance,
    ple_data.question_revision_authorship,
    ple_data.question_revision_license, ple_data.question_revision_citation,
    ple_data.question_ownership_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON TABLE ple_data.question_current_owner FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_question_credit_and_stewardship_change(),
    ple_data.validate_question_ownership_event(), ple_data.validate_question_revision_acceptance(),
    ple_data.validate_question_publication_credit() FROM PUBLIC;

COMMENT ON TABLE ple_data.question_revision_authorship IS
    'Ordered immutable Question Authorship snapshot for one Question Revision; display names support external authors and optional Account references support contributor history.';
COMMENT ON TABLE ple_data.question_revision_acceptance IS
    'Immutable Question Revision acceptance with its exact parent Question Revision, Editor Account, accepting Instructor Account, accepted time, and Reason for Edit.';
COMMENT ON TABLE ple_data.question_revision_license IS
    'Required immutable Question License snapshot for one Question Revision; only adaptation-permitting versioned Creative Commons expressions are publishable.';
COMMENT ON TABLE ple_data.question_revision_citation IS
    'Optional immutable Question Citation for one Question Revision; citation supplements rather than replaces Authorship or License.';
COMMENT ON TABLE ple_data.question_ownership_event IS
    'Immutable initial or transferred Question Owner stewardship evidence for one Published Question lineage.';
COMMENT ON VIEW ple_data.question_current_owner IS
    'The current Question Owner derived from the latest immutable Question Ownership Event and evaluated with the authorized caller RLS context.';

RESET ROLE;
