-- Functions, triggers, and views from content_classification_operations.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.require_content_classification_actor(p_sysadmin_only boolean)
RETURNS text LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE actor_id text;
BEGIN
    IF p_sysadmin_only OR ple_api.current_session_account_has_platform_administration() THEN
        RETURN ple_private.require_current_sysadmin_account();
    END IF;
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL OR NOT ple_api.current_session_account_is_instructor()
       OR ple_private.verified_instructor_display_name(actor_id) IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Content classification requires a vetted active Instructor or active Sysadmin';
    END IF;
    RETURN actor_id;
END
$$;



-- ASVS 8.1.1/8.2.1/8.3.1: global vocabulary reads require an installed active
-- Instructor or Sysadmin session, not an attributed, vetted identity. They expose
-- no Course membership, Student work, or other FERPA data. Mutations retain the
-- vetted-identity guard above; installation publishers receive no special bypass.
CREATE FUNCTION ple_private.require_content_classification_reader()
RETURNS text LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
DECLARE actor_id text;
BEGIN
    IF ple_api.current_session_account_has_platform_administration() THEN
        RETURN ple_private.require_current_sysadmin_account();
    END IF;
    actor_id := ple_api.current_session_account_id();
    IF actor_id IS NULL OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION USING ERRCODE = '42501',
            MESSAGE = 'Content classification reading requires an active Instructor or active Sysadmin';
    END IF;
    RETURN actor_id;
END
$$;



-- ASVS 2.2.1/2.2.2: strip boundary whitespace before length/control validation.
-- Names remain plain display text; later renderers must encode for their context.
CREATE FUNCTION ple_private.normalize_content_classification_name(p_name text, p_limit integer)
RETURNS text LANGUAGE plpgsql IMMUTABLE
SET search_path = pg_catalog AS $$
DECLARE normalized_name text;
BEGIN
    normalized_name := regexp_replace(p_name, '^[[:space:]]+|[[:space:]]+$', '', 'g');
    IF normalized_name IS NULL OR char_length(normalized_name) NOT BETWEEN 1 AND p_limit
       OR normalized_name ~ '[[:cntrl:]]' THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content classification name is invalid';
    END IF;
    RETURN normalized_name;
END
$$;



-- ASVS 1.2.4: closed typed parameters and static SQL; no dynamic identifiers.
CREATE FUNCTION ple_private.create_content_discipline(p_name text)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE new_uuid uuid; normalized_name text;
BEGIN
    PERFORM ple_private.require_content_classification_actor(true);
    normalized_name := ple_private.normalize_content_classification_name(p_name, 120);
    new_uuid := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.content_discipline (content_discipline_id, name)
    VALUES (new_uuid, normalized_name);
    RETURN new_uuid;
END
$$;



-- This is deliberately a state change rather than deletion: content and
-- historical records retain their stable vocabulary UUIDs.
CREATE FUNCTION ple_private.rename_content_discipline(p_discipline_uuid uuid, p_name text)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE normalized_name text;
BEGIN
    PERFORM ple_private.require_content_classification_actor(true);
    normalized_name := ple_private.normalize_content_classification_name(p_name, 120);
    UPDATE ple_data.content_discipline SET name = normalized_name
     WHERE content_discipline_id = p_discipline_uuid;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content Discipline is invalid';
    END IF;
END
$$;

CREATE FUNCTION ple_private.retire_content_discipline(p_discipline_uuid uuid)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_actor(true);
    UPDATE ple_data.content_discipline SET is_retired = true
     WHERE content_discipline_id = p_discipline_uuid;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content Discipline is invalid';
    END IF;
END
$$;

CREATE FUNCTION ple_private.restore_content_discipline(p_discipline_uuid uuid)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_actor(true);
    UPDATE ple_data.content_discipline SET is_retired = false
     WHERE content_discipline_id = p_discipline_uuid;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content Discipline is invalid';
    END IF;
END
$$;



-- Reusable guard for every classified-object creation or replacement command.
-- It deliberately does not inspect existing references, so retirement neither
-- breaks history nor invalidates an unchanged classification. FOR SHARE holds
-- the active row through the caller's transaction, so a concurrent retirement
-- cannot commit between this check and a new/changed binding.
CREATE FUNCTION ple_private.require_active_content_discipline(p_discipline_uuid uuid)
RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    IF p_discipline_uuid IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content Discipline is unavailable';
    END IF;
    PERFORM 1 FROM ple_data.content_discipline AS item
     WHERE item.content_discipline_id = p_discipline_uuid
       AND NOT item.is_retired
     FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content Discipline is unavailable';
    END IF;
END
$$;



-- ASVS 1.2.4: closed typed parameters and static SQL; no dynamic identifiers.
CREATE FUNCTION ple_private.create_content_subject(p_name text, p_discipline_uuid uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    new_uuid uuid;
    existing_uuid uuid;
    normalized_name text;
BEGIN
    PERFORM ple_private.require_content_classification_actor(false);
    normalized_name := ple_private.normalize_content_classification_name(p_name, 120);
    PERFORM ple_private.require_active_content_discipline(p_discipline_uuid);
    -- An existing global name is the offer. Association stays a separate acceptance.
    SELECT item.content_subject_id INTO existing_uuid
      FROM ple_data.content_subject AS item
     WHERE lower(item.name) = lower(normalized_name)
     FOR UPDATE;
    IF FOUND THEN
        RETURN existing_uuid;
    END IF;
    new_uuid := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.content_subject (content_subject_id, name)
    VALUES (new_uuid, normalized_name);
    INSERT INTO ple_data.content_subject_discipline (content_subject_id, content_discipline_id)
    VALUES (new_uuid, p_discipline_uuid);
    RETURN new_uuid;
EXCEPTION WHEN unique_violation THEN
    SELECT item.content_subject_id INTO existing_uuid
      FROM ple_data.content_subject AS item
     WHERE lower(item.name) = lower(normalized_name);
    IF existing_uuid IS NULL THEN
        RAISE;
    END IF;
    RETURN existing_uuid;
END
$$;



-- ASVS 1.2.4: closed typed parameters and static SQL; no dynamic identifiers.
CREATE FUNCTION ple_private.create_content_topic(p_name text, p_subject_uuid uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE new_uuid uuid; normalized_name text;
BEGIN
    PERFORM ple_private.require_content_classification_actor(false);
    normalized_name := ple_private.normalize_content_classification_name(p_name, 240);
    IF p_subject_uuid IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_data.content_subject WHERE content_subject_id = p_subject_uuid
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content classification parent is invalid';
    END IF;
    new_uuid := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.content_topic (content_topic_id, content_subject_id, name)
    VALUES (new_uuid, p_subject_uuid, normalized_name);
    RETURN new_uuid;
END
$$;



-- ASVS 1.2.4: closed typed parameters and static SQL; no dynamic identifiers.
CREATE FUNCTION ple_private.create_content_subtopic(p_name text, p_topic_uuid uuid)
RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE new_uuid uuid; normalized_name text;
BEGIN
    PERFORM ple_private.require_content_classification_actor(false);
    normalized_name := ple_private.normalize_content_classification_name(p_name, 480);
    IF p_topic_uuid IS NULL OR NOT EXISTS (
        SELECT 1 FROM ple_data.content_topic WHERE content_topic_id = p_topic_uuid
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Content classification parent is invalid';
    END IF;
    new_uuid := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.content_subtopic (content_subtopic_id, content_topic_id, name)
    VALUES (new_uuid, p_topic_uuid, normalized_name);
    RETURN new_uuid;
END
$$;



-- Explicit acceptance of an existing global Subject in another Discipline.
-- ASVS 8.2.1/2.3.3: normal Instructor addition is not administrative replacement;
-- the composite key makes replay idempotent without receipts or new state.
CREATE FUNCTION ple_private.add_content_subject_discipline(
    p_subject_uuid uuid, p_discipline_uuid uuid
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_actor(false);
    PERFORM 1 FROM ple_data.content_subject WHERE content_subject_id = p_subject_uuid FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Subject Discipline selection is invalid';
    END IF;
    PERFORM ple_private.require_active_content_discipline(p_discipline_uuid);
    INSERT INTO ple_data.content_subject_discipline(content_subject_id, content_discipline_id)
    VALUES (p_subject_uuid, p_discipline_uuid) ON CONFLICT DO NOTHING;
END
$$;




-- ASVS 2.3.3/2.3.4: a nonempty replacement is validated before DELETE and
-- serializes on the one Subject row until transaction end.
CREATE FUNCTION ple_private.replace_content_subject_disciplines(
    p_subject_uuid uuid, p_discipline_uuids uuid[]
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE new_discipline_uuid uuid;
BEGIN
    PERFORM ple_private.require_content_classification_actor(true);
    IF p_discipline_uuids IS NULL OR cardinality(p_discipline_uuids) < 1
       OR array_ndims(p_discipline_uuids) <> 1
       OR EXISTS (SELECT 1 FROM unnest(p_discipline_uuids) AS selected(id) WHERE id IS NULL)
       OR cardinality(p_discipline_uuids) <> (
           SELECT count(DISTINCT id) FROM unnest(p_discipline_uuids) AS selected(id)
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Subject Discipline selection is invalid';
    END IF;
    PERFORM 1 FROM ple_data.content_subject WHERE content_subject_id = p_subject_uuid FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023', MESSAGE = 'Subject is invalid';
    END IF;
    -- Retained associations may name retired Disciplines. Every newly added
    -- UUID instead takes the shared active-row lock, so retirement cannot
    -- commit between validation and the replacement write below.
    FOR new_discipline_uuid IN
        SELECT selected.id FROM unnest(p_discipline_uuids) AS selected(id)
         WHERE NOT EXISTS (
             SELECT 1 FROM ple_data.content_subject_discipline AS existing
              WHERE existing.content_subject_id = p_subject_uuid
                AND existing.content_discipline_id = selected.id
         )
         ORDER BY selected.id
    LOOP
        PERFORM ple_private.require_active_content_discipline(new_discipline_uuid);
    END LOOP;
    -- Retain unchanged associations: their identity may be referenced by
    -- Published Questions. Immediate foreign keys refuse referenced removals
    -- and roll back the entire replacement, including concurrent writers.
    DELETE FROM ple_data.content_subject_discipline
     WHERE content_subject_id = p_subject_uuid
       AND NOT (content_discipline_id = ANY(p_discipline_uuids));
    INSERT INTO ple_data.content_subject_discipline (content_subject_id, content_discipline_id)
    SELECT p_subject_uuid, id FROM unnest(p_discipline_uuids) AS selected(id) ORDER BY id
    ON CONFLICT DO NOTHING;
END
$$;

CREATE FUNCTION ple_private.list_content_disciplines()
RETURNS TABLE (content_discipline_id uuid, name text) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_reader();
    RETURN QUERY SELECT item.content_discipline_id, item.name FROM ple_data.content_discipline AS item
    WHERE NOT item.is_retired
    ORDER BY lower(item.name), item.name, item.content_discipline_id;
END
$$;



-- Discovery and exact-reference surfaces retain retired values with their
-- current display name and explicit status. Authoring selection uses the
-- active-only projection above.
CREATE FUNCTION ple_private.list_content_disciplines_including_retired()
RETURNS TABLE (content_discipline_id uuid, name text, is_retired boolean)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_reader();
    RETURN QUERY SELECT item.content_discipline_id, item.name, item.is_retired
    FROM ple_data.content_discipline AS item
    ORDER BY item.is_retired, lower(item.name), item.name, item.content_discipline_id;
END
$$;

CREATE FUNCTION ple_private.get_content_discipline(p_discipline_uuid uuid)
RETURNS TABLE (content_discipline_id uuid, name text, is_retired boolean)
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_reader();
    RETURN QUERY SELECT item.content_discipline_id, item.name, item.is_retired
    FROM ple_data.content_discipline AS item
    WHERE item.content_discipline_id = p_discipline_uuid;
END
$$;

CREATE FUNCTION ple_private.list_content_subjects(p_discipline_uuid uuid)
RETURNS TABLE (content_subject_id uuid, name text) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_reader();
    RETURN QUERY SELECT item.content_subject_id, item.name FROM ple_data.content_subject AS item
    JOIN ple_data.content_subject_discipline AS association ON association.content_subject_id = item.content_subject_id
    WHERE association.content_discipline_id = p_discipline_uuid
    ORDER BY lower(item.name), item.name, item.content_subject_id;
END
$$;

CREATE FUNCTION ple_private.list_content_topics(p_subject_uuid uuid)
RETURNS TABLE (content_topic_id uuid, name text) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_reader();
    RETURN QUERY SELECT item.content_topic_id, item.name FROM ple_data.content_topic AS item
    WHERE item.content_subject_id = p_subject_uuid
    ORDER BY lower(item.name), item.name, item.content_topic_id;
END
$$;

CREATE FUNCTION ple_private.list_content_subtopics(p_topic_uuid uuid)
RETURNS TABLE (content_subtopic_id uuid, name text) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_content_classification_reader();
    RETURN QUERY SELECT item.content_subtopic_id, item.name FROM ple_data.content_subtopic AS item
    WHERE item.content_topic_id = p_topic_uuid
    ORDER BY lower(item.name), item.name, item.content_subtopic_id;
END
$$;

CREATE FUNCTION ple_private.find_content_subject(p_name text)
RETURNS TABLE (content_subject_id uuid, name text) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE normalized_name text;
BEGIN
    PERFORM ple_private.require_content_classification_reader();
    normalized_name := ple_private.normalize_content_classification_name(p_name, 120);
    RETURN QUERY SELECT item.content_subject_id, item.name
    FROM ple_data.content_subject AS item WHERE lower(item.name) = lower(normalized_name)
    ORDER BY item.content_subject_id;
END
$$;

-- ASVS 1.2.4/8.2.1/8.3.1: a request stores a bounded name. It does not create a Discipline.
CREATE FUNCTION ple_private.request_content_discipline(p_name text)
RETURNS TABLE (
    content_discipline_request_id uuid,
    requested_name text,
    requested_by_account_id ple_data.account_id
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    actor_id text;
    normalized_name text;
    new_uuid uuid;
BEGIN
    actor_id := ple_private.require_content_classification_actor(false);
    normalized_name := ple_private.normalize_content_classification_name(p_name, 120);
    RETURN QUERY
    SELECT item.content_discipline_request_id, item.requested_name, item.requested_by_account_id
      FROM ple_data.content_discipline_request AS item
     WHERE item.requested_by_account_id = actor_id
       AND lower(item.requested_name) = lower(normalized_name)
       AND item.resolved_at IS NULL;
    IF FOUND THEN
        RETURN;
    END IF;
    new_uuid := pg_catalog.gen_random_uuid();
    INSERT INTO ple_data.content_discipline_request (
        content_discipline_request_id, requested_name, requested_by_account_id
    ) VALUES (new_uuid, normalized_name, actor_id);
    RETURN QUERY
    SELECT item.content_discipline_request_id, item.requested_name, item.requested_by_account_id
      FROM ple_data.content_discipline_request AS item
     WHERE item.content_discipline_request_id = new_uuid;
EXCEPTION WHEN unique_violation THEN
    RETURN QUERY
    SELECT item.content_discipline_request_id, item.requested_name, item.requested_by_account_id
      FROM ple_data.content_discipline_request AS item
     WHERE item.requested_by_account_id = actor_id
       AND lower(item.requested_name) = lower(normalized_name)
       AND item.resolved_at IS NULL;
    IF NOT FOUND THEN
        RAISE;
    END IF;
END
$$;



-- ASVS 8.2.2/14.2.6: a Sysadmin sees the requester Account ID and the requested name only.
CREATE FUNCTION ple_private.list_open_content_discipline_requests()
RETURNS TABLE (
    content_discipline_request_id uuid,
    requested_name text,
    requested_by_account_id ple_data.account_id
) LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    RETURN QUERY
    SELECT item.content_discipline_request_id, item.requested_name, item.requested_by_account_id
      FROM ple_data.content_discipline_request AS item
     WHERE item.resolved_at IS NULL
     ORDER BY item.requested_at, item.content_discipline_request_id;
END
$$;



-- ASVS 8.3.1/16.5.1: only a Sysadmin resolves a request, and a missing request stays generic.
CREATE FUNCTION ple_private.resolve_content_discipline_request(p_request_uuid uuid)
RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    UPDATE ple_data.content_discipline_request AS item
       SET resolved_at = pg_catalog.transaction_timestamp()
     WHERE item.content_discipline_request_id = p_request_uuid
       AND item.resolved_at IS NULL;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Content Discipline request is unavailable';
    END IF;
END
$$;

-- ASVS 2.3.3: fulfillment creates the requested Discipline and resolves its request atomically.
CREATE FUNCTION ple_private.fulfill_content_discipline_request(p_request_uuid uuid)
RETURNS uuid LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE requested_name text; discipline_uuid uuid;
BEGIN
    PERFORM ple_private.require_current_sysadmin_account();
    SELECT item.requested_name INTO requested_name
      FROM ple_data.content_discipline_request AS item
     WHERE item.content_discipline_request_id = p_request_uuid
       AND item.resolved_at IS NULL
     FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Content Discipline request is unavailable';
    END IF;
    discipline_uuid := ple_private.create_content_discipline(requested_name);
    PERFORM ple_private.resolve_content_discipline_request(p_request_uuid);
    RETURN discipline_uuid;
END
$$;



SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.find_content_subject(p_name text)
RETURNS TABLE (content_subject_id uuid, name text) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.find_content_subject(p_name)
$$;

CREATE FUNCTION ple_api.add_content_subject_discipline(
    p_subject_uuid uuid, p_discipline_uuid uuid
) RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.add_content_subject_discipline(p_subject_uuid, p_discipline_uuid)
$$;

CREATE FUNCTION ple_api.create_content_discipline(p_name text)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT ple_private.create_content_discipline(p_name) $$;

CREATE FUNCTION ple_api.rename_content_discipline(p_discipline_uuid uuid, p_name text)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.rename_content_discipline(p_discipline_uuid, p_name)
$$;

CREATE FUNCTION ple_api.retire_content_discipline(p_discipline_uuid uuid)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.retire_content_discipline(p_discipline_uuid)
$$;

CREATE FUNCTION ple_api.restore_content_discipline(p_discipline_uuid uuid)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.restore_content_discipline(p_discipline_uuid)
$$;

CREATE FUNCTION ple_api.require_active_content_discipline(p_discipline_uuid uuid)
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.require_active_content_discipline(p_discipline_uuid)
$$;

CREATE FUNCTION ple_api.create_content_subject(p_name text, p_discipline_uuid uuid)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT ple_private.create_content_subject(p_name, p_discipline_uuid) $$;

CREATE FUNCTION ple_api.create_content_topic(p_name text, p_subject_uuid uuid)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT ple_private.create_content_topic(p_name, p_subject_uuid) $$;

CREATE FUNCTION ple_api.create_content_subtopic(p_name text, p_topic_uuid uuid)
RETURNS uuid LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT ple_private.create_content_subtopic(p_name, p_topic_uuid) $$;

CREATE FUNCTION ple_api.replace_content_subject_disciplines(p_subject_uuid uuid, p_discipline_uuids uuid[])
RETURNS void LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT ple_private.replace_content_subject_disciplines(p_subject_uuid, p_discipline_uuids) $$;

CREATE FUNCTION ple_api.list_content_disciplines()
RETURNS TABLE (content_discipline_id uuid, name text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT * FROM ple_private.list_content_disciplines() $$;

CREATE FUNCTION ple_api.list_content_disciplines_including_retired()
RETURNS TABLE (content_discipline_id uuid, name text, is_retired boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.list_content_disciplines_including_retired()
$$;

CREATE FUNCTION ple_api.get_content_discipline(p_discipline_uuid uuid)
RETURNS TABLE (content_discipline_id uuid, name text, is_retired boolean)
LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.get_content_discipline(p_discipline_uuid)
$$;

CREATE FUNCTION ple_api.list_content_subjects(p_discipline_uuid uuid)
RETURNS TABLE (content_subject_id uuid, name text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT * FROM ple_private.list_content_subjects(p_discipline_uuid) $$;

CREATE FUNCTION ple_api.list_content_topics(p_subject_uuid uuid)
RETURNS TABLE (content_topic_id uuid, name text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT * FROM ple_private.list_content_topics(p_subject_uuid) $$;

CREATE FUNCTION ple_api.list_content_subtopics(p_topic_uuid uuid)
RETURNS TABLE (content_subtopic_id uuid, name text) LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$ SELECT * FROM ple_private.list_content_subtopics(p_topic_uuid) $$;

CREATE FUNCTION ple_api.request_content_discipline(p_name text)
RETURNS TABLE (
    content_discipline_request_id uuid,
    requested_name text,
    requested_by_account_id ple_data.account_id
) LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.request_content_discipline(p_name)
$$;

CREATE FUNCTION ple_api.list_open_content_discipline_requests()
RETURNS TABLE (
    content_discipline_request_id uuid,
    requested_name text,
    requested_by_account_id ple_data.account_id
) LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.list_open_content_discipline_requests()
$$;

CREATE FUNCTION ple_api.resolve_content_discipline_request(p_request_uuid uuid)
RETURNS void LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.resolve_content_discipline_request(p_request_uuid)
$$;

CREATE FUNCTION ple_api.fulfill_content_discipline_request(p_request_uuid uuid)
RETURNS uuid LANGUAGE sql VOLATILE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT ple_private.fulfill_content_discipline_request(p_request_uuid)
$$;
