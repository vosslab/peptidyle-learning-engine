-- WP-PROF-B2: immutable curriculum-adoption evidence and broker boundary.
--
-- The current import pointer is intentionally the only repairable B2 state.
-- Receipt, evidence, whole-course topology, and Alpha fork lineage retain the
-- immutable authority needed to repair it without recreating teaching state.

BEGIN;

CREATE TABLE public.course_schedule_revision (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    revision bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, course_id),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course (tenant_id, course_id) ON DELETE CASCADE,
    CHECK (revision > 0)
);

INSERT INTO public.course_schedule_revision (tenant_id, course_id)
SELECT course.tenant_id, course.course_id FROM public.course AS course;

ALTER TABLE public.alpha_course
    ADD CONSTRAINT alpha_course_creator_identity_key
    UNIQUE (creator_tenant_id, alpha_course_id);

CREATE TABLE public.curriculum_adoption_receipt (
    tenant_id uuid NOT NULL,
    idempotency_key text NOT NULL,
    operation text NOT NULL,
    actor_user_id uuid NOT NULL,
    request_sha256 bytea NOT NULL,
    occurred_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    destination_course_id uuid,
    destination_assignment_id uuid,
    destination_alpha_course_id uuid,
    source_course_id uuid,
    source_alpha_course_id uuid,
    outcome_import_revision bigint,
    target_term_json jsonb,
    PRIMARY KEY (tenant_id, idempotency_key),
    UNIQUE (tenant_id, idempotency_key, operation, destination_course_id),
    UNIQUE (
        tenant_id, idempotency_key, operation,
        destination_course_id, destination_assignment_id
    ),
    UNIQUE (tenant_id, idempotency_key, destination_course_id),
    UNIQUE (tenant_id, idempotency_key, destination_alpha_course_id),
    FOREIGN KEY (tenant_id, destination_course_id)
        REFERENCES public.course (tenant_id, course_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, destination_course_id, destination_assignment_id)
        REFERENCES public.assignment (tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, destination_alpha_course_id)
        REFERENCES public.alpha_course (creator_tenant_id, alpha_course_id) ON DELETE RESTRICT,
    CHECK (idempotency_key ~ '^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$'),
    CHECK (operation IN (
        'forkAlpha', 'blueprintInstantiation', 'alphaInstantiation',
        'courseRollover', 'courseTermShift', 'assignmentFastForward',
        'sourceDerivedAssignment'
    )),
    CHECK (octet_length(request_sha256) = 32),
    CHECK (outcome_import_revision IS NULL OR outcome_import_revision > 0),
    CHECK (target_term_json IS NULL OR jsonb_typeof(target_term_json) = 'object'),
    CHECK (
        (operation = 'forkAlpha'
         AND destination_alpha_course_id IS NOT NULL AND source_alpha_course_id IS NOT NULL
         AND destination_course_id IS NULL AND destination_assignment_id IS NULL
         AND source_course_id IS NULL AND outcome_import_revision IS NULL
         AND target_term_json IS NULL)
        OR (operation = 'blueprintInstantiation'
            AND destination_course_id IS NOT NULL AND destination_assignment_id IS NOT NULL
            AND destination_alpha_course_id IS NULL AND source_course_id IS NULL
            AND source_alpha_course_id IS NULL AND outcome_import_revision IS NULL
            AND target_term_json IS NOT NULL)
        OR (operation = 'alphaInstantiation'
            AND destination_course_id IS NOT NULL AND destination_assignment_id IS NULL
            AND destination_alpha_course_id IS NULL AND source_course_id IS NULL
            AND source_alpha_course_id IS NOT NULL AND outcome_import_revision IS NULL
            AND target_term_json IS NOT NULL)
        OR (operation = 'courseRollover'
            AND destination_course_id IS NOT NULL AND destination_assignment_id IS NULL
            AND destination_alpha_course_id IS NULL AND source_course_id IS NOT NULL
            AND source_alpha_course_id IS NULL AND outcome_import_revision IS NULL
            AND target_term_json IS NOT NULL)
        OR (operation = 'courseTermShift'
            AND destination_course_id IS NOT NULL AND destination_assignment_id IS NULL
            AND destination_alpha_course_id IS NULL AND source_course_id IS NULL
            AND source_alpha_course_id IS NULL AND outcome_import_revision IS NULL
            AND target_term_json IS NOT NULL)
        OR (operation = 'assignmentFastForward'
            AND destination_course_id IS NOT NULL AND destination_assignment_id IS NOT NULL
            AND destination_alpha_course_id IS NULL AND source_course_id IS NULL
            AND source_alpha_course_id IS NULL AND outcome_import_revision IS NOT NULL
            AND target_term_json IS NULL)
        OR (operation = 'sourceDerivedAssignment'
            AND destination_course_id IS NOT NULL AND destination_assignment_id IS NOT NULL
            AND destination_alpha_course_id IS NULL AND source_course_id IS NULL
            AND source_alpha_course_id IS NULL AND outcome_import_revision IS NULL
            AND target_term_json IS NULL)
    )
);

-- A receipt can own one assignment directly or an ordered set through a
-- whole-course adoption.  This relation is the canonical exact destination
-- set for both shapes (ASVS 2.2.3).
CREATE TABLE public.curriculum_adoption_receipt_assignment (
    tenant_id uuid NOT NULL,
    receipt_key text NOT NULL,
    operation text NOT NULL,
    course_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    single_destination_assignment_id uuid,
    PRIMARY KEY (tenant_id, receipt_key, course_id, assignment_id),
    FOREIGN KEY (tenant_id, receipt_key, operation, course_id)
        REFERENCES public.curriculum_adoption_receipt
        (tenant_id, idempotency_key, operation, destination_course_id) ON DELETE CASCADE,
    FOREIGN KEY (
        tenant_id, receipt_key, operation, course_id, single_destination_assignment_id
    ) REFERENCES public.curriculum_adoption_receipt (
        tenant_id, idempotency_key, operation,
        destination_course_id, destination_assignment_id
    ) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment (tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CHECK (
        (operation IN (
            'blueprintInstantiation', 'assignmentFastForward', 'sourceDerivedAssignment'
         ) AND single_destination_assignment_id IS NOT NULL
           AND single_destination_assignment_id = assignment_id)
        OR (operation IN ('alphaInstantiation', 'courseRollover')
            AND single_destination_assignment_id IS NULL)
    )
);

CREATE TABLE public.curriculum_assignment_adoption_evidence (
    tenant_id uuid NOT NULL,
    receipt_key text NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    import_revision bigint NOT NULL,
    -- This closed JSON DTO lets the relational broker reconstruct the
    -- normalized semantic input.  It is operational input, not the immutable
    -- semantic evidence used to establish what the adoption meant.
    semantic_payload jsonb NOT NULL,
    -- The qmodel-owned envelope is the durable semantic evidence: the exact
    -- canonical encoding version, complete domain-separated bytes, and their
    -- SHA-256 digest.  The broker binds all three atomically with provenance.
    semantic_canonical_version smallint NOT NULL,
    semantic_canonical_bytes bytea NOT NULL,
    semantic_sha256 bytea NOT NULL,
    source_kind text NOT NULL,
    source_blueprint_reference integer,
    source_blueprint_revision bigint,
    source_alpha_course_id uuid,
    source_alpha_revision bigint,
    source_module_position integer,
    source_definition_position integer,
    source_course_id uuid,
    source_course_schedule_revision bigint,
    source_assignment_id uuid,
    source_assignment_revision bigint,
    PRIMARY KEY (tenant_id, receipt_key, assignment_id),
    UNIQUE (tenant_id, assignment_id, import_revision),
    UNIQUE (tenant_id, receipt_key, course_id, assignment_id),
    UNIQUE (tenant_id, receipt_key, course_id, assignment_id, source_kind),
    FOREIGN KEY (tenant_id, receipt_key, course_id, assignment_id)
        REFERENCES public.curriculum_adoption_receipt_assignment
        (tenant_id, receipt_key, course_id, assignment_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment (tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CHECK (import_revision > 0),
    CHECK (octet_length(semantic_payload::text) BETWEEN 2 AND 524288),
    CHECK (octet_length(semantic_sha256) = 32),
    CHECK (jsonb_typeof(semantic_payload) = 'object'),
    CHECK (semantic_canonical_version BETWEEN 1 AND 255),
    CHECK (octet_length(semantic_canonical_bytes) BETWEEN 1 AND 524288),
    CHECK (semantic_sha256 = digest(semantic_canonical_bytes, 'sha256')),
    CHECK (source_kind IN ('blueprint', 'alpha', 'rollover')),
    CHECK (
        (source_kind = 'blueprint'
         AND source_blueprint_reference IS NOT NULL AND source_blueprint_reference > 0
         AND source_blueprint_revision IS NOT NULL AND source_blueprint_revision > 0
         AND source_alpha_course_id IS NULL AND source_alpha_revision IS NULL
         AND source_module_position IS NULL AND source_definition_position IS NULL
         AND source_course_id IS NULL AND source_course_schedule_revision IS NULL
         AND source_assignment_id IS NULL AND source_assignment_revision IS NULL)
        OR (source_kind = 'alpha'
            AND source_alpha_course_id IS NOT NULL
            AND source_alpha_revision IS NOT NULL AND source_alpha_revision > 0
            AND source_module_position IS NOT NULL AND source_module_position >= 0
            AND source_definition_position IS NOT NULL AND source_definition_position >= 0
            AND source_blueprint_reference IS NULL AND source_blueprint_revision IS NULL
            AND source_course_id IS NULL AND source_course_schedule_revision IS NULL
            AND source_assignment_id IS NULL AND source_assignment_revision IS NULL)
        OR (source_kind = 'rollover'
            AND source_course_id IS NOT NULL
            AND source_course_schedule_revision IS NOT NULL
            AND source_course_schedule_revision > 0
            AND source_assignment_id IS NOT NULL
            AND source_assignment_revision IS NOT NULL AND source_assignment_revision > 0
            AND source_blueprint_reference IS NULL AND source_blueprint_revision IS NULL
            AND source_alpha_course_id IS NULL AND source_alpha_revision IS NULL
            AND source_module_position IS NULL AND source_definition_position IS NULL)
    )
);

CREATE INDEX curriculum_assignment_evidence_latest_idx
    ON public.curriculum_assignment_adoption_evidence
    (tenant_id, assignment_id, import_revision DESC);

CREATE TABLE public.curriculum_assignment_import_current (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    receipt_key text NOT NULL,
    PRIMARY KEY (tenant_id, assignment_id),
    FOREIGN KEY (tenant_id, receipt_key, assignment_id)
        REFERENCES public.curriculum_assignment_adoption_evidence
        (tenant_id, receipt_key, assignment_id) ON DELETE CASCADE
);

CREATE TABLE public.curriculum_whole_course_adoption (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    receipt_key text NOT NULL,
    receipt_operation text NOT NULL,
    origin_kind text NOT NULL,
    -- This closed JSON DTO supports broker reconstruction.  The following
    -- qmodel envelope, rather than this DTO, is the immutable semantic proof.
    semantic_payload jsonb NOT NULL,
    semantic_canonical_version smallint NOT NULL,
    semantic_canonical_bytes bytea NOT NULL,
    semantic_sha256 bytea NOT NULL,
    source_alpha_course_id uuid,
    source_alpha_revision bigint,
    source_course_id uuid,
    source_schedule_revision bigint,
    PRIMARY KEY (tenant_id, course_id),
    UNIQUE (tenant_id, course_id, receipt_key),
    UNIQUE (tenant_id, course_id, receipt_key, origin_kind),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course (tenant_id, course_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, receipt_key, receipt_operation, course_id)
        REFERENCES public.curriculum_adoption_receipt
        (tenant_id, idempotency_key, operation, destination_course_id) ON DELETE CASCADE,
    CHECK (origin_kind IN ('alpha', 'rollover')),
    CHECK (octet_length(semantic_payload::text) BETWEEN 2 AND 524288),
    CHECK (octet_length(semantic_sha256) = 32),
    CHECK (jsonb_typeof(semantic_payload) = 'object'),
    CHECK (semantic_canonical_version BETWEEN 1 AND 255),
    CHECK (octet_length(semantic_canonical_bytes) BETWEEN 1 AND 524288),
    CHECK (semantic_sha256 = digest(semantic_canonical_bytes, 'sha256')),
    CHECK (
        (origin_kind = 'alpha' AND receipt_operation = 'alphaInstantiation'
         AND source_alpha_course_id IS NOT NULL
         AND source_alpha_revision IS NOT NULL AND source_alpha_revision > 0
         AND source_course_id IS NULL AND source_schedule_revision IS NULL)
        OR (origin_kind = 'rollover' AND receipt_operation = 'courseRollover'
            AND source_course_id IS NOT NULL
            AND source_schedule_revision IS NOT NULL AND source_schedule_revision > 0
            AND source_alpha_course_id IS NULL AND source_alpha_revision IS NULL)
    )
);

CREATE TABLE public.curriculum_whole_course_module (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    module_position integer NOT NULL,
    label text NOT NULL,
    PRIMARY KEY (tenant_id, course_id, module_position),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.curriculum_whole_course_adoption (tenant_id, course_id) ON DELETE CASCADE,
    CHECK (module_position >= 0),
    CHECK (char_length(label) BETWEEN 1 AND 200 AND label = btrim(label))
);

CREATE TABLE public.curriculum_whole_course_assignment (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    receipt_key text NOT NULL,
    module_position integer NOT NULL,
    assignment_position integer NOT NULL,
    destination_assignment_id uuid NOT NULL,
    source_kind text NOT NULL,
    source_assignment_id uuid,
    source_assignment_revision bigint,
    PRIMARY KEY (tenant_id, course_id, module_position, assignment_position),
    UNIQUE (tenant_id, course_id, destination_assignment_id),
    FOREIGN KEY (tenant_id, course_id, module_position)
        REFERENCES public.curriculum_whole_course_module (tenant_id, course_id, module_position)
        ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, receipt_key)
        REFERENCES public.curriculum_whole_course_adoption (tenant_id, course_id, receipt_key)
        ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, receipt_key, source_kind)
        REFERENCES public.curriculum_whole_course_adoption
        (tenant_id, course_id, receipt_key, origin_kind) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, destination_assignment_id)
        REFERENCES public.assignment (tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    FOREIGN KEY (
        tenant_id, receipt_key, course_id, destination_assignment_id, source_kind
    )
        REFERENCES public.curriculum_assignment_adoption_evidence
        (tenant_id, receipt_key, course_id, assignment_id, source_kind) ON DELETE CASCADE,
    CHECK (module_position >= 0 AND assignment_position >= 0),
    CHECK (
        (source_kind = 'alpha'
         AND source_assignment_id IS NULL AND source_assignment_revision IS NULL)
        OR (source_kind = 'rollover'
            AND source_assignment_id IS NOT NULL
            AND source_assignment_revision IS NOT NULL AND source_assignment_revision > 0)
    )
);

CREATE TABLE public.curriculum_alpha_fork_lineage (
    tenant_id uuid NOT NULL,
    alpha_course_id uuid NOT NULL,
    receipt_key text NOT NULL,
    source_alpha_course_id uuid NOT NULL,
    source_alpha_revision bigint NOT NULL,
    -- The normalized DTO retains reconstruction input; the qmodel envelope
    -- persists the exact immutable meaning of the fork lineage.
    semantic_payload jsonb NOT NULL,
    semantic_canonical_version smallint NOT NULL,
    semantic_canonical_bytes bytea NOT NULL,
    semantic_sha256 bytea NOT NULL,
    PRIMARY KEY (tenant_id, alpha_course_id),
    FOREIGN KEY (tenant_id, alpha_course_id)
        REFERENCES public.alpha_course (creator_tenant_id, alpha_course_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, receipt_key, alpha_course_id)
        REFERENCES public.curriculum_adoption_receipt
        (tenant_id, idempotency_key, destination_alpha_course_id) ON DELETE CASCADE,
    CHECK (source_alpha_revision > 0),
    CHECK (octet_length(semantic_payload::text) BETWEEN 2 AND 524288),
    CHECK (jsonb_typeof(semantic_payload) = 'object'),
    CHECK (semantic_canonical_version BETWEEN 1 AND 255),
    CHECK (octet_length(semantic_canonical_bytes) BETWEEN 1 AND 524288),
    CHECK (octet_length(semantic_sha256) = 32),
    CHECK (semantic_sha256 = digest(semantic_canonical_bytes, 'sha256'))
);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ple_curriculum_adoption_broker') THEN
        CREATE ROLE ple_curriculum_adoption_broker NOLOGIN NOSUPERUSER NOCREATEDB
            NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles
         WHERE rolname = 'ple_curriculum_schedule_revision_broker'
    ) THEN
        CREATE ROLE ple_curriculum_schedule_revision_broker NOLOGIN NOSUPERUSER NOCREATEDB
            NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
    END IF;
END $$;
ALTER ROLE ple_curriculum_adoption_broker NOLOGIN NOSUPERUSER NOCREATEDB
    NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
ALTER ROLE ple_curriculum_schedule_revision_broker NOLOGIN NOSUPERUSER NOCREATEDB
    NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;

DO $$
DECLARE relation_name text;
BEGIN
    FOREACH relation_name IN ARRAY ARRAY[
        'course_schedule_revision', 'curriculum_adoption_receipt',
        'curriculum_adoption_receipt_assignment',
        'curriculum_assignment_adoption_evidence', 'curriculum_assignment_import_current',
        'curriculum_whole_course_adoption', 'curriculum_whole_course_module',
        'curriculum_whole_course_assignment', 'curriculum_alpha_fork_lineage'
    ] LOOP
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', relation_name);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', relation_name);
    END LOOP;
END $$;

CREATE POLICY curriculum_adoption_schedule_select ON public.course_schedule_revision
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_schedule_insert ON public.course_schedule_revision
    FOR INSERT TO ple_curriculum_adoption_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_schedule_update ON public.course_schedule_revision
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
-- The dedicated NOLOGIN helper owner can write only this derived relation.
-- Its helper validates the B2 caller and exact tenant/course pair before the
-- forced-RLS write (ASVS 8.4.1, 15.4.2).
CREATE POLICY curriculum_adoption_schedule_owner_select
    ON public.course_schedule_revision
    FOR SELECT TO ple_curriculum_schedule_revision_broker
    USING (true);
CREATE POLICY curriculum_adoption_schedule_owner_insert
    ON public.course_schedule_revision
    FOR INSERT TO ple_curriculum_schedule_revision_broker
    WITH CHECK (true);
CREATE POLICY curriculum_adoption_schedule_owner_update
    ON public.course_schedule_revision
    FOR UPDATE TO ple_curriculum_schedule_revision_broker
    USING (true) WITH CHECK (true);

CREATE POLICY curriculum_adoption_receipt_select ON public.curriculum_adoption_receipt
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_receipt_insert ON public.curriculum_adoption_receipt
    FOR INSERT TO ple_curriculum_adoption_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_receipt_retention_delete
    ON public.curriculum_adoption_receipt
    FOR DELETE TO ple_curriculum_adoption_broker
    USING (
        tenant_id = public.ple_current_tenant()
        AND current_setting('ple.curriculum_adoption_maintenance', true) = 'retention'
        AND destination_course_id::text =
            current_setting('ple.curriculum_adoption_course_id', true)
    );

DO $$
DECLARE relation_name text;
BEGIN
    FOREACH relation_name IN ARRAY ARRAY[
        'curriculum_adoption_receipt_assignment',
        'curriculum_assignment_adoption_evidence',
        'curriculum_whole_course_adoption', 'curriculum_whole_course_module',
        'curriculum_whole_course_assignment', 'curriculum_alpha_fork_lineage'
    ] LOOP
        EXECUTE format(
            'CREATE POLICY curriculum_adoption_select_%I ON public.%I FOR SELECT TO ple_curriculum_adoption_broker USING (tenant_id = public.ple_current_tenant())',
            relation_name, relation_name
        );
        EXECUTE format(
            'CREATE POLICY curriculum_adoption_insert_%I ON public.%I FOR INSERT TO ple_curriculum_adoption_broker WITH CHECK (tenant_id = public.ple_current_tenant())',
            relation_name, relation_name
        );
    END LOOP;
END $$;

CREATE POLICY curriculum_adoption_current_select
    ON public.curriculum_assignment_import_current
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_current_insert
    ON public.curriculum_assignment_import_current
    FOR INSERT TO ple_curriculum_adoption_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_current_update
    ON public.curriculum_assignment_import_current
    FOR UPDATE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_current_delete
    ON public.curriculum_assignment_import_current
    FOR DELETE TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());

-- Capability migration 2026081839 replaces this body with a validated,
-- DELETE-only retention path.  Until then every evidence mutation refuses.
CREATE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    RAISE EXCEPTION 'curriculum adoption immutable evidence is retained'
        USING ERRCODE = 'PBI01';
END $$;
REVOKE ALL ON FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1()
    FROM PUBLIC;

CREATE TRIGGER curriculum_adoption_receipt_immutable
BEFORE UPDATE OR DELETE ON public.curriculum_adoption_receipt
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1();
CREATE TRIGGER curriculum_adoption_receipt_assignment_immutable
BEFORE UPDATE OR DELETE ON public.curriculum_adoption_receipt_assignment
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1();
CREATE TRIGGER curriculum_adoption_evidence_immutable
BEFORE UPDATE OR DELETE ON public.curriculum_assignment_adoption_evidence
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1();
CREATE TRIGGER curriculum_whole_course_adoption_immutable
BEFORE UPDATE OR DELETE ON public.curriculum_whole_course_adoption
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1();
CREATE TRIGGER curriculum_whole_course_module_immutable
BEFORE UPDATE OR DELETE ON public.curriculum_whole_course_module
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1();
CREATE TRIGGER curriculum_whole_course_assignment_immutable
BEFORE UPDATE OR DELETE ON public.curriculum_whole_course_assignment
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1();
CREATE TRIGGER curriculum_alpha_fork_lineage_immutable
BEFORE UPDATE OR DELETE ON public.curriculum_alpha_fork_lineage
FOR EACH ROW EXECUTE FUNCTION public.ple_curriculum_adoption_immutable_refusal_v1();

-- This is the only write primitive owned by the schedule-revision broker.  The
-- exact pair remains FK-bound to course, while p_create selects one upsert or
-- one update per parent mutation (ASVS 8.4.1, 15.4.2).
CREATE FUNCTION public.ple_advance_course_schedule_revision_v1(
    p_tenant uuid, p_course uuid, p_create boolean, p_caller name
) RETURNS void LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF p_tenant IS NULL OR p_course IS NULL OR p_create IS NULL
       OR p_caller IS DISTINCT FROM 'ple_curriculum_adoption_broker'::name
    THEN
        RAISE EXCEPTION 'course schedule revision authority is unavailable'
            USING ERRCODE = '42501';
    END IF;

    IF p_create THEN
        INSERT INTO public.course_schedule_revision (tenant_id, course_id)
        VALUES (p_tenant, p_course)
        ON CONFLICT (tenant_id, course_id) DO UPDATE
            SET revision = public.course_schedule_revision.revision + 1,
                updated_at = transaction_timestamp();
    ELSE
        UPDATE public.course_schedule_revision
           SET revision = revision + 1, updated_at = transaction_timestamp()
         WHERE tenant_id = p_tenant AND course_id = p_course;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'course schedule revision is unavailable'
                USING ERRCODE = '23503';
        END IF;
    END IF;
END $$;
ALTER FUNCTION public.ple_advance_course_schedule_revision_v1(
    uuid, uuid, boolean, name
) OWNER TO ple_curriculum_schedule_revision_broker;
REVOKE ALL ON FUNCTION public.ple_advance_course_schedule_revision_v1(
    uuid, uuid, boolean, name
) FROM PUBLIC;

CREATE FUNCTION public.ple_bump_course_term_schedule_revision_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM public.ple_advance_course_schedule_revision_v1(
            NEW.tenant_id, NEW.course_id, true, current_user::name
        );
    ELSIF ROW(NEW.term_start_date, NEW.term_end_date, NEW.time_zone)
          IS DISTINCT FROM ROW(OLD.term_start_date, OLD.term_end_date, OLD.time_zone) THEN
        PERFORM public.ple_advance_course_schedule_revision_v1(
            NEW.tenant_id, NEW.course_id, false, current_user::name
        );
    END IF;
    RETURN NEW;
END $$;
ALTER FUNCTION public.ple_bump_course_term_schedule_revision_v1()
    OWNER TO ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_bump_course_term_schedule_revision_v1()
    FROM PUBLIC, ple_curriculum_adoption_broker;

CREATE FUNCTION public.ple_bump_assignment_schedule_revision_v1()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF TG_OP = 'INSERT'
       OR ROW(NEW.available_at, NEW.due_at, NEW.closes_at)
          IS DISTINCT FROM ROW(OLD.available_at, OLD.due_at, OLD.closes_at)
    THEN
        PERFORM public.ple_advance_course_schedule_revision_v1(
            NEW.tenant_id, NEW.course_id, false, current_user::name
        );
    END IF;
    RETURN NEW;
END $$;
ALTER FUNCTION public.ple_bump_assignment_schedule_revision_v1()
    OWNER TO ple_curriculum_adoption_broker;
REVOKE ALL ON FUNCTION public.ple_bump_assignment_schedule_revision_v1()
    FROM PUBLIC, ple_curriculum_adoption_broker;

CREATE TRIGGER curriculum_course_schedule_revision
AFTER INSERT OR UPDATE OF term_start_date, term_end_date, time_zone ON public.course
FOR EACH ROW EXECUTE FUNCTION public.ple_bump_course_term_schedule_revision_v1();
CREATE TRIGGER curriculum_assignment_schedule_revision
AFTER INSERT OR UPDATE OF available_at, due_at, closes_at
ON public.assignment_effective_policy_base
FOR EACH ROW EXECUTE FUNCTION public.ple_bump_assignment_schedule_revision_v1();

COMMIT;
