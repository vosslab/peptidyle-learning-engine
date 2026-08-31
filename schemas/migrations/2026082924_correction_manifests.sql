-- SD1 immutable forced-question corrections and recalculation evidence.

SET LOCAL ROLE ple_private_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES ON TABLE ple_private.account TO ple_data_owner;
RESET ROLE;

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.forced_question_correction (
    correction_id uuid PRIMARY KEY,
    flawed_question_id text NOT NULL,
    flawed_version_number integer NOT NULL CHECK (flawed_version_number > 0),
    replacement_question_id text NOT NULL,
    replacement_version_number integer NOT NULL CHECK (replacement_version_number > 0),
    approved_by_account_id uuid NOT NULL,
    approver_role text NOT NULL DEFAULT 'sysadmin' CHECK (approver_role = 'sysadmin'),
    approved_at timestamp with time zone NOT NULL,
    generation integer NOT NULL CHECK (generation > 0),
    reason text NOT NULL CHECK (reason IN ('security_flaw', 'critical_correctness_flaw')),
    remediation jsonb NOT NULL CHECK (jsonb_typeof(remediation) = 'object'),
    CONSTRAINT forced_question_correction_versions_differ CHECK (
        (flawed_question_id, flawed_version_number) <> (replacement_question_id, replacement_version_number)
    ),
    CONSTRAINT forced_question_correction_approver_role_matches FOREIGN KEY (approved_by_account_id, approver_role)
        REFERENCES ple_private.account (account_id, role),
    CONSTRAINT forced_question_correction_flawed_generation_is_unique UNIQUE (flawed_question_id, flawed_version_number, generation),
    CONSTRAINT forced_question_correction_flawed_version_matches FOREIGN KEY (flawed_question_id, flawed_version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number),
    CONSTRAINT forced_question_correction_replacement_version_matches FOREIGN KEY (replacement_question_id, replacement_version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);
CREATE FUNCTION ple_data.reject_forced_question_correction_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a forced question correction is immutable';
END
$$;
CREATE TRIGGER forced_question_correction_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.forced_question_correction
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_forced_question_correction_change();
CREATE TABLE ple_data.question_change_event (
    question_change_event_id uuid PRIMARY KEY,
    proposal_id uuid,
    proposal_revision_id uuid,
    forced_question_correction_id uuid REFERENCES ple_data.forced_question_correction (correction_id),
    event_kind text NOT NULL CHECK (event_kind IN ('opened', 'merged', 'closed', 'forced_correction')),
    recorded_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    merged_question_id text,
    merged_version_number integer CHECK (merged_version_number > 0),
    evidence jsonb NOT NULL CHECK (jsonb_typeof(evidence) = 'object'),
    CONSTRAINT question_change_event_proposal_revision_matches FOREIGN KEY (proposal_id, proposal_revision_id)
        REFERENCES ple_data.question_change_proposal_revision (proposal_id, proposal_revision_id),
    CONSTRAINT question_change_event_uses_one_authority_path CHECK (
        (event_kind IN ('opened', 'merged', 'closed')
            AND proposal_id IS NOT NULL
            AND proposal_revision_id IS NOT NULL
            AND forced_question_correction_id IS NULL)
        OR (event_kind = 'forced_correction'
            AND proposal_id IS NULL
            AND proposal_revision_id IS NULL
            AND forced_question_correction_id IS NOT NULL)
    ),
    CONSTRAINT question_change_event_merge_result_matches_kind CHECK (
        (event_kind = 'merged'
            AND merged_question_id IS NOT NULL
            AND merged_version_number IS NOT NULL)
        OR (event_kind <> 'merged'
            AND merged_question_id IS NULL
            AND merged_version_number IS NULL)
    ),
    CONSTRAINT question_change_event_merged_version_matches FOREIGN KEY (merged_question_id, merged_version_number)
        REFERENCES ple_data.published_question_version (question_id, version_number)
);
CREATE UNIQUE INDEX question_change_proposal_has_one_open_event
    ON ple_data.question_change_event (proposal_id) WHERE event_kind = 'opened';
CREATE UNIQUE INDEX question_change_proposal_has_one_terminal_event
    ON ple_data.question_change_event (proposal_id) WHERE event_kind IN ('merged', 'closed');
CREATE UNIQUE INDEX forced_question_correction_has_one_change_event
    ON ple_data.question_change_event (forced_question_correction_id) WHERE event_kind = 'forced_correction';
CREATE FUNCTION ple_data.validate_question_change_event()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE
    base_question_id text;
    base_version_number integer;
BEGIN
    IF NEW.proposal_id IS NULL THEN
        RETURN NEW;
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(NEW.proposal_id::text, 0));
    SELECT revision.base_question_id, revision.base_version_number
    INTO base_question_id, base_version_number
    FROM ple_data.question_change_proposal_revision AS revision
    WHERE revision.proposal_id = NEW.proposal_id
      AND revision.proposal_revision_id = NEW.proposal_revision_id;
    IF NEW.event_kind IN ('merged', 'closed') AND NOT EXISTS (
        SELECT 1 FROM ple_data.question_change_event AS opened
        WHERE opened.proposal_id = NEW.proposal_id AND opened.event_kind = 'opened'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a Question Change Proposal must be opened before it is merged or closed';
    END IF;
    IF NEW.event_kind = 'merged' AND (
        NEW.merged_question_id <> base_question_id
        OR NEW.merged_version_number <= base_version_number
        OR EXISTS (
            SELECT 1 FROM ple_data.published_question_version AS later_version
            WHERE later_version.question_id = base_question_id
              AND later_version.version_number > base_version_number
              AND later_version.version_number <> NEW.merged_version_number
        )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'a merged Question Change Proposal Revision requires its current exact base and a later same-lineage Question Version';
    END IF;
    RETURN NEW;
END
$$;
CREATE FUNCTION ple_data.reject_question_change_event_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Change Events are immutable'; END
$$;
CREATE TRIGGER question_change_event_has_valid_transition
BEFORE INSERT ON ple_data.question_change_event
FOR EACH ROW EXECUTE FUNCTION ple_data.validate_question_change_event();
CREATE TRIGGER question_change_event_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.question_change_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_question_change_event_change();
ALTER TABLE ple_data.forced_question_correction ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.forced_question_correction FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_change_event FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_data.forced_question_correction, ple_data.question_change_event FROM PUBLIC;
REVOKE ALL PRIVILEGES ON FUNCTION ple_data.reject_forced_question_correction_change(), ple_data.validate_question_change_event(), ple_data.reject_question_change_event_change() FROM PUBLIC;
GRANT USAGE ON SCHEMA ple_data TO ple_audit_owner;
GRANT REFERENCES ON TABLE ple_data.course_instance, ple_data.forced_question_correction TO ple_audit_owner;
RESET ROLE;

SET LOCAL ROLE ple_audit_owner;
CREATE TABLE ple_audit.correction_recalculation_evidence (
    evidence_id uuid PRIMARY KEY,
    correction_id uuid NOT NULL REFERENCES ple_data.forced_question_correction (correction_id),
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    generation integer NOT NULL CHECK (generation > 0),
    recorded_at timestamp with time zone NOT NULL,
    outcome jsonb NOT NULL CHECK (jsonb_typeof(outcome) = 'object'),
    digest bytea NOT NULL CHECK (pg_catalog.octet_length(digest) = 32),
    CONSTRAINT correction_recalculation_evidence_is_unique UNIQUE (correction_id, course_id, generation)
);
ALTER TABLE ple_audit.correction_recalculation_evidence ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_audit.correction_recalculation_evidence FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_audit.correction_recalculation_evidence FROM PUBLIC;
RESET ROLE;
