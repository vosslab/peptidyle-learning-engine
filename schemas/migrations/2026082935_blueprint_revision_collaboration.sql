-- SD1 exact Blueprint Course draft collaboration and publication evidence.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.blueprint_course_revision
    ADD CONSTRAINT blueprint_course_revision_identity_is_unique
    UNIQUE (blueprint_revision_id, blueprint_id);

CREATE TABLE ple_data.blueprint_publication_event (
    blueprint_publication_event_id uuid PRIMARY KEY,
    blueprint_revision_id uuid NOT NULL,
    blueprint_id uuid NOT NULL,
    published_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_publication_event_revision_is_unique
        UNIQUE (blueprint_revision_id),
    CONSTRAINT blueprint_publication_event_revision_matches_blueprint
        FOREIGN KEY (blueprint_revision_id, blueprint_id)
        REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id, blueprint_id)
);

CREATE TABLE ple_data.blueprint_collaborator_event (
    blueprint_collaborator_event_id uuid PRIMARY KEY,
    blueprint_revision_id uuid NOT NULL,
    blueprint_id uuid NOT NULL,
    collaborator_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('granted', 'ended')),
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_collaborator_event_kind_is_unique
        UNIQUE (blueprint_revision_id, collaborator_account_id, event_kind),
    CONSTRAINT blueprint_collaborator_event_revision_matches_blueprint
        FOREIGN KEY (blueprint_revision_id, blueprint_id)
        REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id, blueprint_id),
    CONSTRAINT blueprint_collaborator_event_grant_has_distinct_accounts
        CHECK (event_kind <> 'granted' OR collaborator_account_id <> recorded_by_account_id)
);

CREATE TABLE ple_data.blueprint_revision_availability_event (
    blueprint_revision_availability_event_id uuid PRIMARY KEY,
    blueprint_revision_id uuid NOT NULL,
    blueprint_id uuid NOT NULL,
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    event_kind text NOT NULL CHECK (event_kind IN ('available', 'archived')),
    occurred_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_revision_availability_event_kind_is_unique
        UNIQUE (blueprint_revision_id, event_kind),
    CONSTRAINT blueprint_revision_availability_event_revision_matches_blueprint
        FOREIGN KEY (blueprint_revision_id, blueprint_id)
        REFERENCES ple_data.blueprint_course_revision (blueprint_revision_id, blueprint_id)
);

CREATE FUNCTION ple_data.validate_blueprint_publication_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_data.blueprint_course AS blueprint
          JOIN ple_private.account AS account
            ON account.account_id = NEW.published_by_account_id
           AND account.role = 'instructor'
         WHERE blueprint.blueprint_id = NEW.blueprint_id
           AND blueprint.owner_account_id = NEW.published_by_account_id
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'only the Approved Blueprint Course Owner may publish an exact Blueprint Revision';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER blueprint_publication_event_has_valid_authority
BEFORE INSERT ON ple_data.blueprint_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_blueprint_publication_event();

CREATE FUNCTION ple_data.reject_blueprint_publication_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Publication Events are immutable';
END
$$;
CREATE TRIGGER blueprint_publication_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_publication_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_publication_event_change();

CREATE FUNCTION ple_data.validate_blueprint_revision_availability_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE
    course_owner_account_id uuid;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(NEW.blueprint_revision_id::text, 0)
    );
    SELECT blueprint.owner_account_id
      INTO course_owner_account_id
      FROM ple_data.blueprint_course AS blueprint
     WHERE blueprint.blueprint_id = NEW.blueprint_id;
    IF NEW.recorded_by_account_id <> course_owner_account_id
       OR NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_publication_event AS publication
            WHERE publication.blueprint_revision_id = NEW.blueprint_revision_id
       )
       OR (NEW.event_kind = 'archived' AND NOT EXISTS (
           SELECT 1 FROM ple_data.blueprint_revision_availability_event AS available_event
            WHERE available_event.blueprint_revision_id = NEW.blueprint_revision_id
              AND available_event.event_kind = 'available'
              AND available_event.occurred_at <= NEW.occurred_at
       )) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'only the Blueprint Course Owner may record ordered availability for a published Blueprint Revision';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER blueprint_revision_availability_event_has_valid_transition
BEFORE INSERT ON ple_data.blueprint_revision_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_blueprint_revision_availability_event();
CREATE FUNCTION ple_data.reject_blueprint_revision_availability_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Revision Availability Events are immutable';
END
$$;
CREATE TRIGGER blueprint_revision_availability_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_revision_availability_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_revision_availability_event_change();

CREATE FUNCTION ple_data.validate_blueprint_collaborator_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    course_owner_account_id uuid;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NEW.blueprint_revision_id::text || ':' || NEW.collaborator_account_id::text,
            0
        )
    );
    SELECT blueprint.owner_account_id
      INTO course_owner_account_id
      FROM ple_data.blueprint_course AS blueprint
     WHERE blueprint.blueprint_id = NEW.blueprint_id;

    IF course_owner_account_id IS NULL
       OR EXISTS (
           SELECT 1
             FROM ple_data.blueprint_publication_event AS publication
            WHERE publication.blueprint_revision_id = NEW.blueprint_revision_id
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint Collaborator Events apply only to an exact Draft Blueprint Revision';
    END IF;

    IF NEW.event_kind = 'granted' THEN
        IF NEW.collaborator_account_id = course_owner_account_id
           OR NEW.recorded_by_account_id <> course_owner_account_id
           OR NOT EXISTS (
               SELECT 1
                 FROM ple_private.account AS account
                WHERE account.account_id = NEW.collaborator_account_id
                  AND account.role = 'instructor'
           ) THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'only the Blueprint Course Owner may grant a Draft Blueprint Revision contribution relationship to an Instructor Account';
        END IF;
    ELSIF NOT EXISTS (
        SELECT 1
          FROM ple_data.blueprint_collaborator_event AS grant_event
         WHERE grant_event.blueprint_revision_id = NEW.blueprint_revision_id
           AND grant_event.collaborator_account_id = NEW.collaborator_account_id
           AND grant_event.event_kind = 'granted'
           AND grant_event.occurred_at <= NEW.occurred_at
    ) OR NEW.recorded_by_account_id NOT IN (course_owner_account_id, NEW.collaborator_account_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Blueprint Collaborator relationship can end only after its grant and by its owner or collaborator';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER blueprint_collaborator_event_has_valid_transition
BEFORE INSERT ON ple_data.blueprint_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_blueprint_collaborator_event();

CREATE FUNCTION ple_data.reject_blueprint_collaborator_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Collaborator Events are immutable';
END
$$;
CREATE TRIGGER blueprint_collaborator_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_collaborator_event_change();

ALTER TABLE ple_data.blueprint_publication_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_publication_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_collaborator_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_collaborator_event FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_availability_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.blueprint_revision_availability_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.blueprint_publication_event,
    ple_data.blueprint_collaborator_event,
    ple_data.blueprint_revision_availability_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_blueprint_publication_event_change(),
    ple_data.validate_blueprint_publication_event(),
    ple_data.validate_blueprint_collaborator_event(),
    ple_data.reject_blueprint_collaborator_event_change(),
    ple_data.validate_blueprint_revision_availability_event(),
    ple_data.reject_blueprint_revision_availability_event_change() FROM PUBLIC;
COMMENT ON TABLE ple_data.blueprint_publication_event IS
    'Immutable evidence that one exact Blueprint Revision left private draft collaboration and entered reusable publication.';
COMMENT ON TABLE ple_data.blueprint_collaborator_event IS
    'Immutable grant or end evidence for one Instructor Account contribution relationship to one exact Draft Blueprint Revision.';
COMMENT ON TABLE ple_data.blueprint_revision_availability_event IS
    'Immutable Available or Archived selection state evidence for one exact published Blueprint Revision.';

RESET ROLE;
