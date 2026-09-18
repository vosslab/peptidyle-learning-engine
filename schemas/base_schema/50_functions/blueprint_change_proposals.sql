-- Functions, triggers, and views from blueprint_change_proposals.sql.

SET LOCAL ROLE ple_data_owner;



-- Installed before the shared Revision guards; keep Proposal evidence append-only
-- even for a privileged maintenance role accidentally attempting an edit.
CREATE FUNCTION ple_data.reject_blueprint_proposal_evidence_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION 'Blueprint Proposal evidence is immutable' USING ERRCODE = '55000';
END
$$;

CREATE TRIGGER blueprint_change_proposal_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_change_proposal
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_proposal_evidence_change();

CREATE TRIGGER blueprint_change_proposal_acceptance_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_change_proposal_acceptance
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_proposal_evidence_change();

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.create_blueprint_change_proposal(
    p_source_reference text, p_source_revision bigint, p_source_blueprint_edit_number bigint,
    p_target_reference text, p_target_revision bigint, p_target_blueprint_edit_number bigint
) RETURNS uuid LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_actor text := ple_api.current_session_account_id();
    v_source ple_data.blueprint_course%ROWTYPE;
    v_target ple_data.blueprint_course%ROWTYPE;
    v_locked_reference text;
    v_proposal_id uuid;
BEGIN
    -- ASVS 8.2.1/2, 8.3.1/2: authorization is enforced at the SQL boundary.
    IF NOT ple_api.current_session_account_is_instructor()
       OR v_actor IS NULL OR p_source_reference IS NULL OR p_target_reference IS NULL
       OR NOT ple_private.is_canonical_prefixed_public_id(p_source_reference, 'BP')
       OR NOT ple_private.is_canonical_prefixed_public_id(p_target_reference, 'BP')
       OR p_source_reference = p_target_reference THEN
        RAISE EXCEPTION 'Blueprint Proposal is unavailable' USING ERRCODE = '42501';
    END IF;
    -- Same stable lock order as Blueprint fork operations; SHARE prevents changes
    -- to visibility and the target comparison head during creation.
    FOR v_locked_reference IN
        SELECT course.blueprint_course_id FROM ple_data.blueprint_course AS course
         WHERE course.blueprint_course_id IN (p_source_reference, p_target_reference)
         ORDER BY course.blueprint_course_id
    LOOP
        PERFORM 1 FROM ple_data.blueprint_course AS course
         WHERE course.blueprint_course_id = v_locked_reference FOR SHARE;
    END LOOP;
    SELECT course.* INTO v_source FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_source_reference;
    SELECT course.* INTO v_target FROM ple_data.blueprint_course AS course
     WHERE course.blueprint_course_id = p_target_reference;
    IF v_source.blueprint_course_id IS NULL OR v_target.blueprint_course_id IS NULL
       OR NOT (v_source.availability IN ('public', 'archived')
               OR v_source.owner_account_id = v_actor)
       OR NOT (v_target.availability IN ('public', 'archived')
               OR v_target.owner_account_id = v_actor)
       OR v_target.owner_account_id = v_actor THEN
        RAISE EXCEPTION 'Blueprint Proposal is unavailable' USING ERRCODE = '42501';
    END IF;
    -- ASVS 2.3.3: the receiving target basis must still be the reviewed current
    -- head. Historical source content/metadata are permitted exact evidence.
    IF p_target_revision IS DISTINCT FROM v_target.current_blueprint_revision_number
       OR p_target_blueprint_edit_number IS DISTINCT FROM v_target.blueprint_edit_number THEN
        RAISE EXCEPTION 'Blueprint Proposal comparison is stale' USING ERRCODE = '40001';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_data.blueprint_course_revision AS revision
         WHERE revision.blueprint_course_id = v_source.blueprint_course_id
           AND revision.blueprint_revision_number = p_source_revision)
       OR NOT EXISTS (SELECT 1 FROM ple_data.blueprint_metadata_event AS event
         WHERE event.blueprint_course_id = v_source.blueprint_course_id
           AND event.blueprint_edit_number = p_source_blueprint_edit_number) THEN
        RAISE EXCEPTION 'Blueprint Proposal basis is unavailable' USING ERRCODE = '42501';
    END IF;
    v_proposal_id := gen_random_uuid();
    INSERT INTO ple_data.blueprint_change_proposal VALUES (
        v_proposal_id, v_actor, v_source.blueprint_course_id, p_source_revision,
        p_source_blueprint_edit_number, v_target.blueprint_course_id, p_target_revision,
        p_target_blueprint_edit_number, clock_timestamp()
    );
    RETURN v_proposal_id;
END
$$;

CREATE FUNCTION ple_api.read_blueprint_change_proposal(p_proposal_id uuid)
RETURNS TABLE (
    blueprint_change_proposal_id uuid, proposer_account_id text, created_at_ms bigint,
    target_is_stale boolean, source_position integer, public_reference text,
    revision_number bigint, blueprint_edit_number bigint, content jsonb, content_checksum bytea,
    short_name text, long_name text, content_discipline_id uuid, content_subject_id uuid,
    content_topic_id uuid, content_subtopic_id uuid, tags text[], can_accept boolean
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    -- ASVS 8.2.2, 8.3.2: submission intentionally shares the exact proposed
    -- evidence with the receiving owner, including a Private source. This grants
    -- no general source or history reads. Proposer reads retain ordinary source
    -- visibility, and both participants retain ordinary target visibility.
    SELECT proposal.blueprint_change_proposal_id, proposal.proposer_account_id,
           (extract(epoch FROM proposal.created_at) * 1000)::bigint,
           target.current_blueprint_revision_number <> proposal.target_revision_number
               OR target.blueprint_edit_number <> proposal.target_blueprint_edit_number,
           basis.position, course.blueprint_course_id, basis.revision_number,
           basis.blueprint_edit_number, revision.content, revision.content_checksum,
           event.short_name, event.long_name, event.content_discipline_id, event.content_subject_id,
           event.content_topic_id, event.content_subtopic_id, event.tags,
           target.owner_account_id = ple_api.current_session_account_id()
               AND target.availability <> 'archived'
               AND target.current_blueprint_revision_number = proposal.target_revision_number
               AND target.blueprint_edit_number = proposal.target_blueprint_edit_number
               AND NOT EXISTS (SELECT 1 FROM ple_data.blueprint_change_proposal_acceptance AS accepted
                    WHERE accepted.blueprint_change_proposal_id = proposal.blueprint_change_proposal_id)
      FROM ple_data.blueprint_change_proposal AS proposal
      JOIN ple_data.blueprint_course AS source
        ON source.blueprint_course_id = proposal.source_blueprint_course_id
      JOIN ple_data.blueprint_course AS target
        ON target.blueprint_course_id = proposal.target_blueprint_course_id
      CROSS JOIN LATERAL (VALUES
          (0, proposal.source_blueprint_course_id, proposal.source_revision_number,
              proposal.source_blueprint_edit_number),
          (1, proposal.target_blueprint_course_id, proposal.target_revision_number,
              proposal.target_blueprint_edit_number)
      ) AS basis(position, blueprint_course_id, revision_number, blueprint_edit_number)
      JOIN ple_data.blueprint_course AS course
        ON course.blueprint_course_id = basis.blueprint_course_id
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_id = basis.blueprint_course_id
       AND revision.blueprint_revision_number = basis.revision_number
      JOIN ple_data.blueprint_metadata_event AS event
        ON event.blueprint_course_id = basis.blueprint_course_id
       AND event.blueprint_edit_number = basis.blueprint_edit_number
     WHERE proposal.blueprint_change_proposal_id = p_proposal_id
       AND ple_api.current_session_account_is_instructor()
       AND (proposal.proposer_account_id = ple_api.current_session_account_id()
            OR target.owner_account_id = ple_api.current_session_account_id())
       AND (source.availability IN ('public', 'archived')
            OR source.owner_account_id = ple_api.current_session_account_id()
            OR target.owner_account_id = ple_api.current_session_account_id())
       AND (target.availability IN ('public', 'archived')
            OR target.owner_account_id = ple_api.current_session_account_id())
     ORDER BY basis.position;
$$;

CREATE FUNCTION ple_api.list_blueprint_change_proposals(
    p_target_reference text, p_mine boolean, p_after_created_at text,
    p_after_proposal_id uuid, p_limit integer
) RETURNS TABLE (
    blueprint_change_proposal_id uuid, created_at_ms bigint, created_at_key text,
    source_public_reference text, source_revision_number bigint, source_blueprint_edit_number bigint,
    source_short_name text, source_long_name text,
    target_public_reference text, target_revision_number bigint, target_blueprint_edit_number bigint,
    target_short_name text, target_long_name text, target_is_stale boolean,
    accepted_at_ms bigint, accepted_target_revision_number bigint,
    accepted_target_blueprint_edit_number bigint
) LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_actor text := ple_api.current_session_account_id();
    v_after timestamp with time zone;
BEGIN
    -- ASVS 2.2.1/2/3: exactly one scope, bounded lookahead, and paired precise keys.
    IF p_mine IS NULL OR p_limit IS NULL OR p_limit NOT BETWEEN 1 AND 101
       OR (p_mine AND p_target_reference IS NOT NULL)
       OR (NOT p_mine AND (p_target_reference IS NULL
           OR NOT ple_private.is_canonical_prefixed_public_id(p_target_reference, 'BP')))
       OR (p_after_created_at IS NULL) <> (p_after_proposal_id IS NULL) THEN
        RAISE EXCEPTION 'Blueprint Proposal list request is invalid' USING ERRCODE = '22023';
    END IF;
    IF p_after_created_at IS NOT NULL THEN
        IF p_after_created_at !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{6}Z$' THEN
            RAISE EXCEPTION 'Blueprint Proposal cursor is invalid' USING ERRCODE = '22023';
        END IF;
        BEGIN
            v_after := p_after_created_at::timestamp with time zone;
        EXCEPTION WHEN invalid_datetime_format OR datetime_field_overflow THEN
            RAISE EXCEPTION 'Blueprint Proposal cursor is invalid' USING ERRCODE = '22023';
        END;
        IF to_char(v_after AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') <> p_after_created_at THEN
            RAISE EXCEPTION 'Blueprint Proposal cursor is invalid' USING ERRCODE = '22023';
        END IF;
    END IF;
    -- ASVS 8.2.1/2, 8.3.1/2: a Public target grants no ambient Proposal discovery.
    IF v_actor IS NULL OR NOT ple_api.current_session_account_is_instructor() THEN
        RAISE EXCEPTION 'Blueprint Proposal list is unavailable' USING ERRCODE = '42501';
    END IF;
    IF NOT p_mine AND NOT EXISTS (
        SELECT 1 FROM ple_data.blueprint_course AS target
         WHERE target.blueprint_course_id = p_target_reference
           AND (target.availability IN ('public', 'archived') OR target.owner_account_id = v_actor)
           AND (target.owner_account_id = v_actor OR EXISTS (
               SELECT 1 FROM ple_data.blueprint_change_proposal AS proposal
                JOIN ple_data.blueprint_course AS source
                  ON source.blueprint_course_id = proposal.source_blueprint_course_id
                WHERE proposal.target_blueprint_course_id = target.blueprint_course_id
                  AND proposal.proposer_account_id = v_actor
                  AND (source.availability IN ('public', 'archived') OR source.owner_account_id = v_actor)
           ))
    ) THEN
        RAISE EXCEPTION 'Blueprint Proposal list is unavailable' USING ERRCODE = '42501';
    END IF;
    -- ASVS 1.2.4: static parameterized SQL; recheck the exact-read predicates
    -- for each returned record. Only pinned metadata supplies basis names.
    RETURN QUERY
    SELECT proposal.blueprint_change_proposal_id, (extract(epoch FROM proposal.created_at) * 1000)::bigint,
           to_char(proposal.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"'),
           source.blueprint_course_id, proposal.source_revision_number, proposal.source_blueprint_edit_number,
           source_event.short_name, source_event.long_name,
           target.blueprint_course_id, proposal.target_revision_number, proposal.target_blueprint_edit_number,
           target_event.short_name, target_event.long_name,
           target.current_blueprint_revision_number <> proposal.target_revision_number
               OR target.blueprint_edit_number <> proposal.target_blueprint_edit_number,
           (extract(epoch FROM accepted.accepted_at) * 1000)::bigint,
           accepted.resulting_revision_number, accepted.resulting_blueprint_edit_number
      FROM ple_data.blueprint_change_proposal AS proposal
      JOIN ple_data.blueprint_course AS source
        ON source.blueprint_course_id = proposal.source_blueprint_course_id
      JOIN ple_data.blueprint_course AS target
        ON target.blueprint_course_id = proposal.target_blueprint_course_id
      JOIN ple_data.blueprint_metadata_event AS source_event
        ON source_event.blueprint_course_id = proposal.source_blueprint_course_id
       AND source_event.blueprint_edit_number = proposal.source_blueprint_edit_number
      JOIN ple_data.blueprint_metadata_event AS target_event
        ON target_event.blueprint_course_id = proposal.target_blueprint_course_id
       AND target_event.blueprint_edit_number = proposal.target_blueprint_edit_number
      LEFT JOIN ple_data.blueprint_change_proposal_acceptance AS accepted
        ON accepted.blueprint_change_proposal_id = proposal.blueprint_change_proposal_id
       AND accepted.target_blueprint_course_id = proposal.target_blueprint_course_id
     WHERE (p_mine AND proposal.proposer_account_id = v_actor
            OR NOT p_mine AND target.blueprint_course_id = p_target_reference)
       AND (proposal.proposer_account_id = v_actor OR target.owner_account_id = v_actor)
       AND (source.availability IN ('public', 'archived')
            OR source.owner_account_id = v_actor OR target.owner_account_id = v_actor)
       AND (target.availability IN ('public', 'archived') OR target.owner_account_id = v_actor)
       AND (v_after IS NULL OR (proposal.created_at, proposal.blueprint_change_proposal_id) < (v_after, p_after_proposal_id))
     ORDER BY proposal.created_at DESC, proposal.blueprint_change_proposal_id DESC
     LIMIT p_limit;
END
$$;



-- ASVS 8.2.2/8.3.1, 2.3.3: authorize and retain the target owner lock before
-- trusted Store selection/materialization. No current source-head dependency.
CREATE FUNCTION ple_api.lock_blueprint_change_proposal_acceptance(
    p_proposal_id uuid, p_target_reference text, p_expected_revision bigint,
    p_expected_blueprint_edit_number bigint
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_proposal ple_data.blueprint_change_proposal%ROWTYPE;
    v_target ple_data.blueprint_course%ROWTYPE;
BEGIN
    SELECT * INTO v_proposal FROM ple_data.blueprint_change_proposal
     WHERE blueprint_change_proposal_id = p_proposal_id;
    SELECT * INTO v_target FROM ple_data.blueprint_course
     WHERE blueprint_course_id = v_proposal.target_blueprint_course_id
       AND blueprint_course_id = p_target_reference
       AND owner_account_id = ple_api.current_session_account_id()
       AND ple_api.current_session_account_is_instructor() FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Blueprint Proposal acceptance is unavailable' USING ERRCODE = '42501';
    END IF;
    IF v_target.availability = 'archived' THEN
        RAISE EXCEPTION 'Archived Blueprint Course is read-only' USING ERRCODE = '55000';
    END IF;
    IF EXISTS (SELECT 1 FROM ple_data.blueprint_change_proposal_acceptance
        WHERE blueprint_change_proposal_id = p_proposal_id) THEN
        RAISE EXCEPTION 'Blueprint Proposal is already accepted' USING ERRCODE = '55000';
    END IF;
    IF p_expected_revision IS DISTINCT FROM v_target.current_blueprint_revision_number
       OR p_expected_blueprint_edit_number IS DISTINCT FROM v_target.blueprint_edit_number
       OR p_expected_revision IS DISTINCT FROM v_proposal.target_revision_number
       OR p_expected_blueprint_edit_number IS DISTINCT FROM v_proposal.target_blueprint_edit_number THEN
        RAISE EXCEPTION 'Blueprint Proposal comparison is stale' USING ERRCODE = '40001';
    END IF;
END
$$;

CREATE FUNCTION ple_api.finalize_blueprint_change_proposal_acceptance(
    p_proposal_id uuid, p_target_reference text, p_expected_revision bigint,
    p_expected_blueprint_edit_number bigint, p_decision jsonb, p_request_checksum bytea,
    p_content jsonb, p_content_checksum bytea
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
DECLARE
    v_proposal ple_data.blueprint_change_proposal%ROWTYPE;
    v_target ple_data.blueprint_course%ROWTYPE;
    v_source ple_data.blueprint_metadata_event%ROWTYPE;
    v_prior ple_data.blueprint_course_revision%ROWTYPE;
    v_result_revision bigint;
    v_etag uuid;
    v_now timestamp with time zone := clock_timestamp();
    v_short_name text;
    v_long_name text;
    v_source_classification boolean;
    v_metadata_changed boolean;
BEGIN
    PERFORM ple_api.lock_blueprint_change_proposal_acceptance(
        p_proposal_id, p_target_reference, p_expected_revision, p_expected_blueprint_edit_number);
    IF p_decision IS NULL OR jsonb_typeof(p_decision) <> 'object'
       OR coalesce(p_decision #>> '{decision,kind}', '') NOT IN ('entire', 'selected')
       OR p_request_checksum IS NULL OR octet_length(p_request_checksum) <> 32
       OR p_content IS NULL OR p_content_checksum IS NULL
       OR octet_length(p_content_checksum) <> 32 THEN
        RAISE EXCEPTION 'Blueprint Proposal decision is invalid' USING ERRCODE = '22023';
    END IF;
    SELECT * INTO STRICT v_proposal FROM ple_data.blueprint_change_proposal
     WHERE blueprint_change_proposal_id = p_proposal_id;
    SELECT * INTO STRICT v_target FROM ple_data.blueprint_course
     WHERE blueprint_course_id = v_proposal.target_blueprint_course_id;
    SELECT * INTO STRICT v_source FROM ple_data.blueprint_metadata_event
     WHERE blueprint_course_id = v_proposal.source_blueprint_course_id
       AND blueprint_edit_number = v_proposal.source_blueprint_edit_number;
    SELECT * INTO STRICT v_prior FROM ple_data.blueprint_course_revision
     WHERE blueprint_course_id = v_proposal.target_blueprint_course_id
       AND blueprint_revision_number = p_expected_revision;
    v_short_name := CASE WHEN p_decision #>> '{decision,kind}' = 'entire'
        OR p_decision #> '{decision,source_short_name}' = 'true'::jsonb
        THEN v_source.short_name ELSE v_target.short_name END;
    v_long_name := CASE WHEN p_decision #>> '{decision,kind}' = 'entire'
        OR p_decision #> '{decision,source_long_name}' = 'true'::jsonb
        THEN v_source.long_name ELSE v_target.long_name END;
    v_source_classification := p_decision #>> '{decision,kind}' = 'entire'
        OR coalesce(p_decision #> '{decision,source_classification}' = 'true'::jsonb, false);
    v_metadata_changed := v_short_name <> v_target.short_name
        OR v_long_name <> v_target.long_name
        OR (v_source_classification AND ROW(v_source.content_discipline_id, v_source.content_subject_id,
            v_source.content_topic_id, v_source.content_subtopic_id, v_source.tags) IS DISTINCT FROM
            ROW(v_target.content_discipline_id, v_target.content_subject_id, v_target.content_topic_id,
                v_target.content_subtopic_id, v_target.tags));
    -- Metadata-only is an actual accepted change, never a normal no-op Save receipt.
    IF p_content = v_prior.content AND p_content_checksum = v_prior.content_checksum THEN
        IF NOT v_metadata_changed THEN
            RAISE EXCEPTION 'Blueprint Proposal decision changes no canonical content'
                USING ERRCODE = '22023';
        END IF;
        v_result_revision := p_expected_revision + 1;
        INSERT INTO ple_data.blueprint_course_revision VALUES (
            v_target.blueprint_course_id, v_result_revision,
            v_prior.content, v_prior.content_checksum, v_now);
        INSERT INTO ple_data.blueprint_revision_question_pin
        SELECT v_target.blueprint_course_id, v_result_revision, content_path,
               published_question_id, question_revision_number FROM ple_data.blueprint_revision_question_pin
         WHERE blueprint_course_id = v_target.blueprint_course_id
           AND blueprint_revision_number = p_expected_revision;
        INSERT INTO ple_data.blueprint_revision_module
        SELECT v_target.blueprint_course_id, v_result_revision, blueprint_module_reference,
               module_position FROM ple_data.blueprint_revision_module
         WHERE blueprint_course_id = v_target.blueprint_course_id
           AND blueprint_revision_number = p_expected_revision;
        INSERT INTO ple_data.blueprint_revision_assessment
        SELECT v_target.blueprint_course_id, v_result_revision, blueprint_module_reference,
               blueprint_assessment_reference, assessment_position
          FROM ple_data.blueprint_revision_assessment
         WHERE blueprint_course_id = v_target.blueprint_course_id
           AND blueprint_revision_number = p_expected_revision;
        INSERT INTO ple_data.blueprint_revision_event (
            blueprint_course_id, blueprint_revision_number,
            actor_account_id, request_checksum, occurred_at
        ) VALUES (
            v_target.blueprint_course_id, v_result_revision,
            ple_api.current_session_account_id(), p_request_checksum, v_now);
        UPDATE ple_data.blueprint_course SET current_blueprint_revision_number = v_result_revision
         WHERE blueprint_course_id = v_target.blueprint_course_id;
    ELSE
        -- Blueprint-only five-argument Save is private to the API owner after
        -- adoption installs. Never call the ordinary auto-daughter overload.
        SELECT resulting_blueprint_revision_number INTO STRICT v_result_revision
          FROM ple_api.save_blueprint_course(p_target_reference, p_expected_revision,
              p_request_checksum, p_content, p_content_checksum);
    END IF;
    IF v_result_revision <> p_expected_revision + 1 THEN
        RAISE EXCEPTION 'Blueprint Proposal acceptance needs one successor' USING ERRCODE = '22023';
    END IF;
    SELECT blueprint_edit_number INTO STRICT v_etag FROM ple_api.rename_blueprint_course(
        p_target_reference, p_expected_blueprint_edit_number, v_short_name, v_long_name);
    IF v_source_classification THEN
        SELECT blueprint_edit_number INTO STRICT v_etag FROM ple_api.update_blueprint_classification(
            p_target_reference, v_etag, v_source.content_discipline_id, v_source.content_subject_id,
            v_source.content_topic_id, v_source.content_subtopic_id, v_source.tags);
    END IF;
    INSERT INTO ple_data.blueprint_change_proposal_acceptance VALUES (
        p_proposal_id, ple_api.current_session_account_id(), v_now, p_decision,
        v_target.blueprint_course_id, v_result_revision, v_etag);
END
$$;

CREATE FUNCTION ple_api.read_accepted_blueprint_change_proposal(p_proposal_id uuid)
RETURNS TABLE (
    blueprint_change_proposal_id uuid, actor_account_id text, accepted_at_ms bigint, decision jsonb,
    public_reference text, revision_number bigint, blueprint_edit_number bigint,
    content jsonb, content_checksum bytea, short_name text, long_name text,
    content_discipline_id uuid, content_subject_id uuid, content_topic_id uuid, content_subtopic_id uuid, tags text[]
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT acceptance.blueprint_change_proposal_id, acceptance.actor_account_id,
           (extract(epoch FROM acceptance.accepted_at) * 1000)::bigint, acceptance.decision,
           target.blueprint_course_id, acceptance.resulting_revision_number,
           acceptance.resulting_blueprint_edit_number, revision.content, revision.content_checksum,
           event.short_name, event.long_name, event.content_discipline_id, event.content_subject_id,
           event.content_topic_id, event.content_subtopic_id, event.tags
      FROM ple_data.blueprint_change_proposal_acceptance AS acceptance
      JOIN ple_data.blueprint_course AS target
        ON target.blueprint_course_id = acceptance.target_blueprint_course_id
      JOIN ple_data.blueprint_course_revision AS revision
        ON revision.blueprint_course_id = acceptance.target_blueprint_course_id
       AND revision.blueprint_revision_number = acceptance.resulting_revision_number
      JOIN ple_data.blueprint_metadata_event AS event
        ON event.blueprint_course_id = acceptance.target_blueprint_course_id
       AND event.blueprint_edit_number = acceptance.resulting_blueprint_edit_number
     WHERE acceptance.blueprint_change_proposal_id = p_proposal_id
       AND EXISTS (SELECT 1 FROM ple_api.read_blueprint_change_proposal(p_proposal_id));
$$;

