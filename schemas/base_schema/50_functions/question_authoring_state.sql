-- Functions, triggers, and views from question_authoring_state.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.reject_immutable_question_source_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Question Revision source and Question Fork Source are immutable';
END
$$;



-- Fork provenance cannot be edited or directly discarded.  It is nevertheless
-- private working state: an authorized deletion of the parent Draft must
-- remove it with that Draft.  The per-transaction token is installed only by
-- the narrow active-Instructor deletion procedure, so a direct parent/child delete still
-- fails closed and rolls back as one transaction.
CREATE FUNCTION ple_private.reject_draft_question_fork_source_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF TG_OP = 'DELETE'
       AND pg_catalog.current_setting('ple.authorized_draft_delete_uuid', true)
            IS NOT DISTINCT FROM OLD.draft_question_id::text
       AND NOT EXISTS (
           SELECT 1 FROM ple_private.draft_question AS question
            WHERE question.draft_question_id = OLD.draft_question_id
       ) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Draft Question Fork Source is immutable outside authorized Draft deletion';
END
$$;

CREATE FUNCTION ple_private.validate_authoring_workspace_collaborator_event()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE
    workspace_owner uuid;
BEGIN
    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            NEW.authoring_workspace_id::text || ':' || NEW.collaborator_account_id::text, 0));
    SELECT owner_account_id INTO workspace_owner
      FROM ple_private.authoring_workspace
     WHERE authoring_workspace_id = NEW.authoring_workspace_id AND revoked_at IS NULL;
    IF workspace_owner IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Workspace Collaborator Events require an active Authoring Workspace';
    END IF;
    IF NEW.event_kind = 'started' THEN
        IF NEW.actor_account_id <> workspace_owner
           OR NEW.collaborator_account_id = workspace_owner
           OR NOT EXISTS (SELECT 1 FROM ple_private.account
               WHERE account_id = NEW.collaborator_account_id AND product_role = 'instructor') THEN
            RAISE EXCEPTION USING ERRCODE = '23514',
                MESSAGE = 'only the Authoring Workspace Owner starts an Instructor collaborator relationship';
        END IF;
    ELSIF NOT EXISTS (SELECT 1 FROM ple_private.authoring_workspace_collaborator_event AS started
        WHERE started.authoring_workspace_id = NEW.authoring_workspace_id
          AND started.collaborator_account_id = NEW.collaborator_account_id
          AND started.event_kind = 'started' AND started.occurred_at <= NEW.occurred_at)
       OR NEW.actor_account_id NOT IN (workspace_owner, NEW.collaborator_account_id) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'a Workspace Collaborator relationship ends after its start by its owner or collaborator';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.reject_authoring_workspace_collaborator_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Workspace Collaborator Events are immutable';
END
$$;

CREATE FUNCTION ple_private.reject_committed_workspace_import_item_result_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM ple_private.workspace_import
        WHERE authoring_workspace_id = OLD.authoring_workspace_id AND import_id = OLD.import_id AND state = 'committed') THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'Committed Workspace Import Item Result is immutable';
    END IF;
    RETURN COALESCE(NEW, OLD);
END
$$;

CREATE FUNCTION ple_private.validate_draft_question_source_binding_object_record()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE expected_address jsonb; workspace uuid;
BEGIN
    SELECT authoring_workspace_id INTO workspace FROM ple_private.draft_question
     WHERE draft_question_id = NEW.draft_question_id;
    expected_address := jsonb_build_object('kind', 'workspaceQuestionSource',
        'workspaceId', workspace, 'objectId', NEW.source_object_record_id);
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record AS record
        WHERE record.object_record_id = NEW.source_object_record_id
          AND record.object_storage_area = 'private-content'
          AND record.object_data_class = 'authoring-content'
          AND encode(record.sha256, 'hex') = NEW.source_object_checksum
          AND record.object_address = expected_address) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Draft Question Source Binding requires its exact private Object Address';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_question_revision_source_binding_object_record()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
DECLARE expected_address jsonb;
BEGIN
    expected_address := jsonb_build_object('kind', 'questionSource',
        'questionRevisionTuple', jsonb_build_object('questionId', NEW.published_question_id,
            'revisionNumber', NEW.revision_number), 'objectId', NEW.source_object_record_id);
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record AS record
        WHERE record.object_record_id = NEW.source_object_record_id
          AND record.object_storage_area = 'private-content'
          AND record.object_data_class = 'question-source'
          AND encode(record.sha256, 'hex') = NEW.source_object_checksum
          AND record.object_address = expected_address) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Source Binding requires its exact private Object Address';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION ple_private.validate_question_revision_source_binding()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data, ple_private AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_data.question_revision
        WHERE published_question_id = NEW.published_question_id AND revision_number = NEW.revision_number
          AND backend = NEW.backend) THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Question Revision Source Binding backend must match its Question Revision';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER question_revision_source_binding_is_valid
BEFORE INSERT OR UPDATE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_source_binding();

CREATE TRIGGER authoring_workspace_collaborator_event_has_valid_transition
BEFORE INSERT ON ple_private.authoring_workspace_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_authoring_workspace_collaborator_event();

CREATE TRIGGER authoring_workspace_collaborator_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.authoring_workspace_collaborator_event
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_authoring_workspace_collaborator_event_change();

CREATE TRIGGER draft_question_source_binding_object_record_matches_owner
BEFORE INSERT OR UPDATE ON ple_private.draft_question_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_draft_question_source_binding_object_record();

CREATE TRIGGER question_revision_source_binding_object_record_matches_owner
BEFORE INSERT ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_revision_source_binding_object_record();

CREATE TRIGGER question_revision_source_binding_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_revision_source_binding
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_immutable_question_source_change();

CREATE TRIGGER draft_question_fork_source_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.draft_question_fork_source
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_draft_question_fork_source_change();

CREATE TRIGGER workspace_import_item_result_is_immutable_after_commit
BEFORE UPDATE OR DELETE ON ple_private.workspace_import_item_result
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_committed_workspace_import_item_result_change();



-- These narrowly scoped predicates keep API authorization and the deferred
-- publication-completeness check table-free.  Their SECURITY DEFINER owners
-- are the only roles that can inspect the underlying private rows.
-- ASVS 2.2.1-2.2.3 and 2.3.1-2.3.4: authorization is derived from the
-- current session and immutable source binding, never caller-supplied facts.
CREATE FUNCTION ple_private.current_session_is_authoring_workspace_owner(
    p_authoring_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT p_authoring_workspace_id IS NOT NULL AND EXISTS (
        SELECT 1
          FROM ple_private.authoring_workspace AS workspace
         WHERE workspace.authoring_workspace_id = p_authoring_workspace_id
           AND workspace.owner_account_id = ple_api.current_session_account_id()
           AND workspace.revoked_at IS NULL
    )
$$;

CREATE FUNCTION ple_private.current_session_can_access_authoring_workspace(
    p_authoring_workspace_id uuid
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT p_authoring_workspace_id IS NOT NULL AND EXISTS (
        SELECT 1
          FROM ple_private.authoring_workspace AS workspace
         WHERE workspace.authoring_workspace_id = p_authoring_workspace_id
           AND workspace.revoked_at IS NULL
           AND (
               workspace.owner_account_id = ple_api.current_session_account_id()
               OR EXISTS (
                   SELECT 1
                     FROM ple_private.authoring_workspace_collaborator_event AS collaborator
                    WHERE collaborator.authoring_workspace_id = workspace.authoring_workspace_id
                      AND collaborator.collaborator_account_id = ple_api.current_session_account_id()
                      AND collaborator.event_kind = 'started'
                      AND NOT EXISTS (
                          SELECT 1
                            FROM ple_private.authoring_workspace_collaborator_event AS ended
                           WHERE ended.authoring_workspace_id = collaborator.authoring_workspace_id
                             AND ended.collaborator_account_id = collaborator.collaborator_account_id
                             AND ended.event_kind = 'ended'
                      )
               )
           )
    )
$$;

CREATE FUNCTION ple_private.question_revision_has_source_binding(
    p_published_question_id text, p_revision_number integer
) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_private AS $$
    SELECT p_published_question_id IS NOT NULL AND p_revision_number IS NOT NULL AND EXISTS (
        SELECT 1 FROM ple_private.question_revision_source_binding AS binding
         WHERE binding.published_question_id = p_published_question_id
           AND binding.revision_number = p_revision_number
    )
$$;

