-- WP-PROF-S5: normalized course membership, assignment entitlement, and
-- materialized educational receipts.  This is a pre-production direct cutover:
-- there is one current membership authority and no legacy compatibility surface.
BEGIN;
-- `course_member` remains the sole physical membership authority; the roster
-- relation is renamed to a subordinate profile of that membership episode.
ALTER TABLE public.course_roster_member RENAME TO course_roster_profile;
DROP TRIGGER course_member_retention_fence ON public.course_member;
ALTER TABLE public.course_roster_profile
    RENAME COLUMN course_member_id TO course_membership_id;
ALTER TABLE public.course_member
    RENAME CONSTRAINT course_member_pkey TO course_membership_pkey;
ALTER TABLE public.course_member
    RENAME CONSTRAINT course_member_role_check TO course_membership_role_check;
ALTER TABLE public.course_member
    RENAME CONSTRAINT course_member_tenant_id_course_id_fkey TO course_membership_course_fkey;
ALTER TABLE public.course_group_member
    DROP CONSTRAINT course_group_member_course_member_fkey;
ALTER TABLE public.course_member
    DROP CONSTRAINT course_membership_pkey,
    ADD COLUMN course_membership_id uuid NOT NULL DEFAULT gen_random_uuid(),
    ADD COLUMN student_id uuid,
    ADD COLUMN status text NOT NULL DEFAULT 'active',
    ADD COLUMN roster_id text,
    ADD COLUMN joined_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    ADD COLUMN revoked_at timestamp with time zone,
    ADD CONSTRAINT course_membership_pkey PRIMARY KEY (tenant_id, course_id, course_membership_id),
    ADD CONSTRAINT course_membership_status_check CHECK (status IN ('active', 'revoked')),
    ADD CONSTRAINT course_membership_roster_id_check CHECK (
        roster_id IS NULL OR roster_id ~ '^[A-Za-z0-9._-]{1,64}$'
    ),
    ADD CONSTRAINT course_membership_revocation_check CHECK (
        (status = 'active' AND revoked_at IS NULL)
        OR (status = 'revoked' AND revoked_at IS NOT NULL AND revoked_at >= joined_at)
    );
UPDATE public.course_member membership
   SET student_id = identity.student_id
  FROM public.tenant_learner_identity identity
 WHERE membership.tenant_id = identity.tenant_id
   AND membership.user_id = identity.user_id
   AND membership.role = 'student';
ALTER TABLE public.course_member
    ALTER COLUMN course_membership_id DROP DEFAULT,
    ALTER COLUMN status DROP DEFAULT,
    ALTER COLUMN joined_at DROP DEFAULT,
    ADD CONSTRAINT course_membership_learner_fkey
        FOREIGN KEY (tenant_id, user_id, student_id)
        REFERENCES public.tenant_learner_identity(tenant_id, user_id, student_id),
    ADD CONSTRAINT course_membership_student_shape_check CHECK (
        (role = 'student' AND student_id IS NOT NULL)
        OR (role = 'instructor' AND student_id IS NULL)
    );
CREATE UNIQUE INDEX course_membership_one_active_user_key
    ON public.course_member (tenant_id, course_id, user_id)
    WHERE status = 'active';
CREATE UNIQUE INDEX course_membership_one_active_roster_id_key
    ON public.course_member (tenant_id, course_id, roster_id)
    WHERE status = 'active' AND roster_id IS NOT NULL;
-- A profile cannot create, revoke, or re-role a membership.  Its identity and
-- enrollment fields live exclusively on course_member.
UPDATE public.course_roster_profile profile
   SET course_membership_id = membership.course_membership_id
  FROM public.course_member membership
 WHERE membership.tenant_id = profile.tenant_id
   AND membership.course_id = profile.course_id
   AND membership.user_id = profile.user_id;
UPDATE public.course_member membership
   SET roster_id = profile.roster_id
  FROM public.course_roster_profile profile
 WHERE membership.tenant_id = profile.tenant_id
   AND membership.course_id = profile.course_id
   AND membership.course_membership_id = profile.course_membership_id;
ALTER TABLE public.course_roster_profile
    DROP CONSTRAINT course_roster_member_pkey,
    DROP CONSTRAINT course_roster_member_email_check,
    DROP CONSTRAINT course_roster_member_display_name_check,
    DROP CONSTRAINT course_roster_member_status_check,
    DROP CONSTRAINT course_roster_member_revocation_check,
    DROP CONSTRAINT course_roster_member_tenant_id_user_id_student_id_fkey,
    DROP CONSTRAINT course_roster_member_tenant_id_course_id_fkey,
    DROP CONSTRAINT course_roster_member_tenant_id_course_id_user_id_key,
    DROP COLUMN user_id,
    DROP COLUMN student_id,
    DROP COLUMN status,
    DROP COLUMN joined_at,
    DROP COLUMN revoked_at,
    DROP COLUMN roster_id,
    ADD CONSTRAINT course_roster_profile_pkey
        PRIMARY KEY (tenant_id, course_id, course_membership_id),
    ADD CONSTRAINT course_roster_profile_membership_fkey
        FOREIGN KEY (tenant_id, course_id, course_membership_id)
        REFERENCES public.course_member(tenant_id, course_id, course_membership_id)
        ON DELETE CASCADE,
    ADD CONSTRAINT course_roster_profile_email_check CHECK (
        (roster_email_normalized IS NULL AND roster_email_delivery IS NULL)
        OR (
            octet_length(roster_email_normalized) BETWEEN 3 AND 320
            AND roster_email_normalized = lower(roster_email_normalized)
            AND roster_email_normalized = btrim(roster_email_normalized)
            AND octet_length(roster_email_delivery) BETWEEN 3 AND 320
            AND roster_email_delivery = btrim(roster_email_delivery)
        )
    ),
    ADD CONSTRAINT course_roster_profile_display_name_check CHECK (
        char_length(display_name) BETWEEN 1 AND 200 AND display_name = btrim(display_name)
    );
ALTER TABLE public.course_group_member
    ADD COLUMN course_membership_id uuid;
UPDATE public.course_group_member grouped
   SET course_membership_id = membership.course_membership_id
  FROM public.course_member membership
 WHERE membership.tenant_id = grouped.tenant_id
   AND membership.course_id = grouped.course_id
   AND membership.user_id = grouped.user_id;
ALTER TABLE public.course_group_member
    DROP CONSTRAINT course_group_member_pkey,
    DROP COLUMN user_id,
    ALTER COLUMN course_membership_id SET NOT NULL,
    ADD CONSTRAINT course_group_member_pkey
        PRIMARY KEY (tenant_id, course_group_id, course_membership_id),
    ADD CONSTRAINT course_group_member_membership_fkey
        FOREIGN KEY (tenant_id, course_id, course_membership_id)
        REFERENCES public.course_member(tenant_id, course_id, course_membership_id)
        ON DELETE CASCADE;
CREATE INDEX course_group_member_membership_idx
    ON public.course_group_member (tenant_id, course_id, course_membership_id, course_group_id);
CREATE FUNCTION public.ple_validate_current_course_group_membership() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.course_member
         WHERE tenant_id = NEW.tenant_id
           AND course_id = NEW.course_id
           AND course_membership_id = NEW.course_membership_id
           AND role = 'student'
           AND status = 'active'
    ) THEN
        RAISE EXCEPTION 'course group memberships require an active student membership'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER course_group_member_current_membership_check
    BEFORE INSERT OR UPDATE OF course_membership_id ON public.course_group_member
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_current_course_group_membership();
ALTER TABLE public.course_group
    ADD COLUMN purpose text NOT NULL DEFAULT 'accommodation',
    ADD CONSTRAINT course_group_purpose_check CHECK (
        purpose IN ('section', 'lab', 'cohort', 'accommodation', 'work')
    );
ALTER TABLE public.course_group ALTER COLUMN purpose DROP DEFAULT;
ALTER TABLE public.assignment
    ADD COLUMN audience_kind text NOT NULL DEFAULT 'course_wide',
    ADD CONSTRAINT assignment_audience_kind_check
        CHECK (audience_kind IN ('course_wide', 'any_of_groups'));
ALTER TABLE public.assignment ALTER COLUMN audience_kind DROP DEFAULT;
CREATE TABLE public.assignment_audience_group (
    tenant_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_group_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, assignment_id, course_group_id),
    FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id)
        ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, course_group_id)
        REFERENCES public.course_group(tenant_id, course_id, course_group_id)
        ON DELETE RESTRICT
);
ALTER TABLE public.assignment_audience_group ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.assignment_audience_group FORCE ROW LEVEL SECURITY;
CREATE POLICY assignment_audience_group_tenant ON public.assignment_audience_group
    TO ple_app USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
GRANT SELECT, INSERT, UPDATE, DELETE ON public.assignment_audience_group TO ple_app;
GRANT SELECT, DELETE ON public.assignment_audience_group TO ple_retention_broker;
CREATE FUNCTION public.ple_validate_assignment_audience() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_assignment uuid := COALESCE(NEW.assignment_id, OLD.assignment_id);
    current_tenant uuid := COALESCE(NEW.tenant_id, OLD.tenant_id);
    kind text;
BEGIN
    SELECT audience_kind INTO kind FROM public.assignment
     WHERE tenant_id = current_tenant AND assignment_id = current_assignment;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    IF kind = 'course_wide' AND EXISTS (
        SELECT 1 FROM public.assignment_audience_group
         WHERE tenant_id = current_tenant AND assignment_id = current_assignment
    ) THEN
        RAISE EXCEPTION 'course-wide assignment cannot name audience groups' USING ERRCODE = '23514';
    END IF;
    IF kind = 'any_of_groups' AND NOT EXISTS (
        SELECT 1 FROM public.assignment_audience_group
         WHERE tenant_id = current_tenant AND assignment_id = current_assignment
    ) THEN
        RAISE EXCEPTION 'group assignment audience must be nonempty' USING ERRCODE = '23514';
    END IF;
    IF EXISTS (
        SELECT 1 FROM public.assignment_audience_group audience
        JOIN public.course_group groups
          ON groups.tenant_id = audience.tenant_id
         AND groups.course_id = audience.course_id
         AND groups.course_group_id = audience.course_group_id
        WHERE audience.tenant_id = current_tenant
          AND audience.assignment_id = current_assignment
          AND groups.purpose NOT IN ('section', 'lab', 'cohort')
    ) THEN
        RAISE EXCEPTION 'only section, lab, and cohort groups can define an assignment audience'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER assignment_audience_assignment_check
    AFTER INSERT OR UPDATE OF audience_kind ON public.assignment
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION public.ple_validate_assignment_audience();
CREATE CONSTRAINT TRIGGER assignment_audience_group_check
    AFTER INSERT OR UPDATE OR DELETE ON public.assignment_audience_group
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION public.ple_validate_assignment_audience();
-- A course-group row has no assignment_id.  Keep its capability validation
-- typed to the group relation instead of sending it through the assignment
-- trigger's row contract.
CREATE FUNCTION public.ple_validate_course_group_audience_purpose() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.purpose NOT IN ('section', 'lab', 'cohort') AND EXISTS (
        SELECT 1 FROM public.assignment_audience_group audience
         WHERE audience.tenant_id = NEW.tenant_id
           AND audience.course_id = NEW.course_id
           AND audience.course_group_id = NEW.course_group_id
    ) THEN
        RAISE EXCEPTION 'only section, lab, and cohort groups can define an assignment audience'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER course_group_audience_purpose_check
    AFTER UPDATE OF purpose ON public.course_group
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION public.ple_validate_course_group_audience_purpose();
-- Individual timing exceptions are authored policy for a stable learner
-- identity. They must not require or create an entitlement receipt, because a
-- modifier never becomes the G2 entitlement gate.
ALTER TABLE public.assignment_policy_exception
    DROP CONSTRAINT assignment_policy_exception_student_fkey,
    ADD CONSTRAINT assignment_policy_exception_student_identity_fkey
        FOREIGN KEY (tenant_id, student_id)
        REFERENCES public.tenant_learner_identity(tenant_id, student_id)
        ON DELETE CASCADE;
-- Enrollment is durable evidence, not current authority.  Its small mutable
-- state is relational; grant provenance and the evaluated scope are immutable
-- normalized receipt rows rather than a duplicate JSON blob and checksum.
-- This pre-production cutover intentionally refuses invented provenance.  A
-- database containing the retired eager enrollment cross-product must be
-- recreated from the migration epoch before this schema can be applied.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM public.enrollment) THEN
        RAISE EXCEPTION
            'WP-PROF-S5 requires a clean database: wipe and recreate before applying 2026081803; legacy enrollment rows have no entitlement provenance'
            USING ERRCODE = '55000';
    END IF;
END
$$;
ALTER TABLE public.enrollment
    ADD COLUMN course_id uuid,
    ADD COLUMN course_membership_id uuid,
    ADD COLUMN materialized_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    ADD COLUMN materialization_purpose text NOT NULL DEFAULT 'instructor_issue',
    ADD COLUMN materialized_by_user_id uuid,
    ADD COLUMN materialization_rule text,
    ADD COLUMN evaluator_version integer NOT NULL DEFAULT 1,
    ADD COLUMN entitlement_receipts_sealed_at timestamp with time zone,
    ADD COLUMN first_completed_at timestamp with time zone,
    ADD COLUMN current_grade_run_id uuid,
    ADD COLUMN best_grade_run_id uuid,
    DROP CONSTRAINT enrollment_user_required_check;
ALTER TABLE public.enrollment
    ALTER COLUMN course_id SET NOT NULL,
    ALTER COLUMN course_membership_id SET NOT NULL,
    ALTER COLUMN materialized_at DROP DEFAULT,
    ALTER COLUMN materialization_purpose DROP DEFAULT,
    ALTER COLUMN evaluator_version DROP DEFAULT,
    DROP COLUMN payload,
    DROP COLUMN payload_sha256,
    ADD CONSTRAINT enrollment_materialization_purpose_check CHECK (
        materialization_purpose IN ('start_run', 'grade_bearing_action', 'instructor_issue')
    ),
    ADD CONSTRAINT enrollment_materialization_authority_check CHECK (
        (materialized_by_user_id IS NOT NULL AND materialization_rule IS NULL)
        OR (materialized_by_user_id IS NULL
            AND materialization_purpose = 'grade_bearing_action'
            AND materialization_rule IN ('imported_grade', 'automated_grader'))
    ),
    ADD CONSTRAINT enrollment_evaluator_version_check CHECK (evaluator_version > 0),
    ADD CONSTRAINT enrollment_assignment_course_fkey
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id),
    ADD CONSTRAINT enrollment_membership_fkey
        FOREIGN KEY (tenant_id, course_id, course_membership_id)
        REFERENCES public.course_member(tenant_id, course_id, course_membership_id),
    -- UserId provenance is shared by account and development-session identity
    -- providers, so it must not be coupled to the optional ple_account row.
    ADD CONSTRAINT enrollment_student_identity_fkey
        FOREIGN KEY (tenant_id, user_id, student_id)
        REFERENCES public.tenant_learner_identity(tenant_id, user_id, student_id),
    ADD CONSTRAINT enrollment_current_grade_run_fkey
        FOREIGN KEY (tenant_id, current_grade_run_id)
        REFERENCES public.assignment_run(tenant_id, run_id),
    ADD CONSTRAINT enrollment_best_grade_run_fkey
        FOREIGN KEY (tenant_id, best_grade_run_id)
        REFERENCES public.assignment_run(tenant_id, run_id);
CREATE FUNCTION public.ple_validate_enrollment_grade_runs() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.current_grade_run_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM public.assignment_run
         WHERE tenant_id = NEW.tenant_id
           AND run_id = NEW.current_grade_run_id
           AND enrollment_id = NEW.enrollment_id
    ) THEN
        RAISE EXCEPTION 'current grade run belongs to another enrollment' USING ERRCODE = '23514';
    END IF;
    IF NEW.best_grade_run_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM public.assignment_run
         WHERE tenant_id = NEW.tenant_id
           AND run_id = NEW.best_grade_run_id
           AND enrollment_id = NEW.enrollment_id
    ) THEN
        RAISE EXCEPTION 'best grade run belongs to another enrollment' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER enrollment_grade_run_owner
    BEFORE INSERT OR UPDATE OF current_grade_run_id, best_grade_run_id ON public.enrollment
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_enrollment_grade_runs();
CREATE FUNCTION public.ple_guard_enrollment_materialization_immutability() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF (NEW.tenant_id, NEW.enrollment_id, NEW.assignment_id, NEW.course_id,
        NEW.course_membership_id, NEW.user_id, NEW.student_id, NEW.materialized_at,
        NEW.materialization_purpose, NEW.materialized_by_user_id, NEW.materialization_rule,
        NEW.evaluator_version)
       IS DISTINCT FROM
       (OLD.tenant_id, OLD.enrollment_id, OLD.assignment_id, OLD.course_id,
        OLD.course_membership_id, OLD.user_id, OLD.student_id, OLD.materialized_at,
        OLD.materialization_purpose, OLD.materialized_by_user_id, OLD.materialization_rule,
        OLD.evaluator_version)
    THEN
        RAISE EXCEPTION 'enrollment materialization evidence is immutable' USING ERRCODE = '55000';
    END IF;
    IF NEW.entitlement_receipts_sealed_at IS DISTINCT FROM OLD.entitlement_receipts_sealed_at
       AND NOT (OLD.entitlement_receipts_sealed_at IS NULL
                AND NEW.entitlement_receipts_sealed_at IS NOT NULL)
    THEN
        RAISE EXCEPTION 'enrollment entitlement receipt seal is immutable'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER enrollment_materialization_immutable
    BEFORE UPDATE ON public.enrollment
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_enrollment_materialization_immutability();
CREATE TABLE public.enrollment_entitlement_basis_receipt (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    scope_receipt_id uuid NOT NULL DEFAULT gen_random_uuid(),
    scope_kind text NOT NULL,
    course_id uuid NOT NULL,
    course_group_id uuid,
    course_group_purpose text,
    PRIMARY KEY (tenant_id, enrollment_id, scope_receipt_id),
    FOREIGN KEY (tenant_id, enrollment_id)
        REFERENCES public.enrollment(tenant_id, enrollment_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, course_group_id)
        REFERENCES public.course_group(tenant_id, course_id, course_group_id)
        ON DELETE RESTRICT,
    CONSTRAINT enrollment_entitlement_basis_receipt_one_per_enrollment
        UNIQUE (tenant_id, enrollment_id),
    CONSTRAINT enrollment_entitlement_basis_receipt_shape_check CHECK (
        (scope_kind = 'course_wide' AND course_group_id IS NULL AND course_group_purpose IS NULL)
        OR (scope_kind = 'group_audience'
            AND course_group_id IS NOT NULL
            AND course_group_purpose IN ('section', 'lab', 'cohort'))
    )
);
ALTER TABLE public.enrollment_entitlement_basis_receipt
    ALTER COLUMN scope_receipt_id DROP DEFAULT;
ALTER TABLE public.enrollment_entitlement_basis_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.enrollment_entitlement_basis_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY enrollment_entitlement_basis_receipt_tenant
    ON public.enrollment_entitlement_basis_receipt TO ple_app
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY enrollment_entitlement_basis_receipt_retention
    ON public.enrollment_entitlement_basis_receipt TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT, INSERT ON public.enrollment_entitlement_basis_receipt TO ple_app;
GRANT SELECT, DELETE ON public.enrollment_entitlement_basis_receipt TO ple_retention_broker;
CREATE TABLE public.enrollment_applicable_policy_scope_receipt (
    tenant_id uuid NOT NULL,
    enrollment_id uuid NOT NULL,
    course_id uuid NOT NULL,
    course_group_id uuid NOT NULL,
    course_group_purpose text NOT NULL,
    PRIMARY KEY (tenant_id, enrollment_id, course_group_id),
    FOREIGN KEY (tenant_id, enrollment_id)
        REFERENCES public.enrollment(tenant_id, enrollment_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, course_id, course_group_id)
        REFERENCES public.course_group(tenant_id, course_id, course_group_id)
        ON DELETE RESTRICT,
    CONSTRAINT enrollment_applicable_policy_scope_purpose_check CHECK (
        course_group_purpose IN ('section', 'lab', 'cohort', 'accommodation')
    )
);
ALTER TABLE public.enrollment_applicable_policy_scope_receipt ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.enrollment_applicable_policy_scope_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY enrollment_applicable_policy_scope_receipt_tenant
    ON public.enrollment_applicable_policy_scope_receipt TO ple_app
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY enrollment_applicable_policy_scope_receipt_retention
    ON public.enrollment_applicable_policy_scope_receipt TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
GRANT SELECT, INSERT ON public.enrollment_applicable_policy_scope_receipt TO ple_app;
GRANT SELECT, DELETE ON public.enrollment_applicable_policy_scope_receipt TO ple_retention_broker;
CREATE FUNCTION public.ple_guard_entitlement_receipt_immutability() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_user = 'ple_retention_broker' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'entitlement receipt is immutable' USING ERRCODE = '55000';
END
$$;
CREATE TRIGGER enrollment_entitlement_basis_receipt_immutable
    BEFORE UPDATE OR DELETE ON public.enrollment_entitlement_basis_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_entitlement_receipt_immutability();
CREATE TRIGGER enrollment_applicable_policy_scope_receipt_immutable
    BEFORE UPDATE OR DELETE ON public.enrollment_applicable_policy_scope_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_entitlement_receipt_immutability();
CREATE FUNCTION public.ple_validate_entitlement_receipt_scope() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    enrollment_course uuid;
    enrollment_membership uuid;
    receipts_sealed_at timestamp with time zone;
    stored_purpose text;
BEGIN
    SELECT course_id, course_membership_id, entitlement_receipts_sealed_at
      INTO enrollment_course, enrollment_membership, receipts_sealed_at
      FROM public.enrollment
     WHERE tenant_id = NEW.tenant_id AND enrollment_id = NEW.enrollment_id;
    IF NOT FOUND OR NEW.course_id IS DISTINCT FROM enrollment_course THEN
        RAISE EXCEPTION 'entitlement receipt crosses an enrollment course boundary'
            USING ERRCODE = '23503';
    END IF;
    IF receipts_sealed_at IS NOT NULL THEN
        RAISE EXCEPTION 'enrollment entitlement receipt set is sealed'
            USING ERRCODE = '55000';
    END IF;
    IF TG_TABLE_NAME = 'enrollment_entitlement_basis_receipt' THEN
        IF NEW.scope_kind = 'course_wide' THEN
            RETURN NEW;
        END IF;
    END IF;
    SELECT purpose INTO stored_purpose
      FROM public.course_group
     WHERE tenant_id = NEW.tenant_id
       AND course_id = NEW.course_id
       AND course_group_id = NEW.course_group_id;
    IF stored_purpose IS DISTINCT FROM NEW.course_group_purpose THEN
        RAISE EXCEPTION 'entitlement receipt group purpose does not match the materialized scope'
            USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM public.course_group_member
         WHERE tenant_id = NEW.tenant_id
           AND course_id = NEW.course_id
           AND course_group_id = NEW.course_group_id
           AND course_membership_id = enrollment_membership
    ) THEN
        RAISE EXCEPTION 'entitlement receipt group does not contain the enrolled learner'
            USING ERRCODE = '23503';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER enrollment_entitlement_basis_receipt_scope_match
    BEFORE INSERT ON public.enrollment_entitlement_basis_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_entitlement_receipt_scope();
CREATE TRIGGER enrollment_applicable_policy_scope_receipt_scope_match
    BEFORE INSERT ON public.enrollment_applicable_policy_scope_receipt
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_entitlement_receipt_scope();
-- Every enrollment has exactly one immutable grant basis.  The many-row
-- policy-scope receipt is deliberately separate because it describes all
-- evaluator-approved scope, including Accommodation, not just the grant.
CREATE FUNCTION public.ple_validate_enrollment_entitlement_basis() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    current_enrollment uuid := COALESCE(NEW.enrollment_id, OLD.enrollment_id);
    current_tenant uuid := COALESCE(NEW.tenant_id, OLD.tenant_id);
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM public.enrollment_entitlement_basis_receipt
         WHERE tenant_id = current_tenant AND enrollment_id = current_enrollment
    ) THEN
        RAISE EXCEPTION 'enrollment requires exactly one entitlement basis receipt'
            USING ERRCODE = '23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM public.enrollment
         WHERE tenant_id = current_tenant
           AND enrollment_id = current_enrollment
           AND entitlement_receipts_sealed_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'enrollment entitlement receipt set must be sealed'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END
$$;
CREATE CONSTRAINT TRIGGER enrollment_entitlement_basis_required
    AFTER INSERT ON public.enrollment
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION public.ple_validate_enrollment_entitlement_basis();
-- The Store persists an evaluator-owned decision inside the same ordinary
-- tenant-RLS transaction that locked its membership and audience facts. No
-- SECURITY DEFINER materializer or independently callable grant path exists.
REVOKE UPDATE, DELETE ON public.enrollment FROM ple_app;
GRANT SELECT, INSERT ON public.enrollment TO ple_app;
GRANT UPDATE (first_completed_at, current_grade_run_id, best_grade_run_id)
    ON public.enrollment TO ple_app;
GRANT UPDATE (entitlement_receipts_sealed_at) ON public.enrollment TO ple_app;
REVOKE UPDATE, DELETE ON public.enrollment_entitlement_basis_receipt FROM ple_app;
REVOKE UPDATE, DELETE ON public.enrollment_applicable_policy_scope_receipt FROM ple_app;
-- The compact mutable scoring projection is typed relational state. Historical
-- runs remain the source evidence; neither the projection nor its worker
-- staging copy owns an opaque JSON representation.
ALTER TABLE public.student_assignment_summary
    DROP COLUMN payload,
    DROP COLUMN payload_sha256,
    ADD COLUMN current_score double precision,
    ADD COLUMN best_score double precision,
    ADD COLUMN latest_score double precision,
    ADD COLUMN completed_run_count bigint NOT NULL DEFAULT 0,
    ADD COLUMN total_question_attempts bigint NOT NULL DEFAULT 0,
    ADD COLUMN last_activity_at timestamp with time zone,
    ADD CONSTRAINT student_assignment_summary_value_check CHECK (
        (current_score IS NULL OR current_score BETWEEN 0 AND 1)
        AND (best_score IS NULL OR best_score BETWEEN 0 AND 1)
        AND (latest_score IS NULL OR latest_score BETWEEN 0 AND 1)
        AND completed_run_count >= 0 AND total_question_attempts >= 0
    );
ALTER TABLE public.student_assignment_summary
    ALTER COLUMN completed_run_count DROP DEFAULT,
    ALTER COLUMN total_question_attempts DROP DEFAULT;
ALTER TABLE public.assignment_summary_staging
    DROP CONSTRAINT assignment_summary_staging_payload_check,
    DROP COLUMN summary_payload,
    DROP COLUMN summary_payload_sha256,
    DROP COLUMN enrollment_payload,
    DROP COLUMN enrollment_payload_sha256,
    ADD COLUMN current_score double precision,
    ADD COLUMN best_score double precision,
    ADD COLUMN latest_score double precision,
    ADD COLUMN completed_run_count bigint NOT NULL DEFAULT 0,
    ADD COLUMN total_question_attempts bigint NOT NULL DEFAULT 0,
    ADD COLUMN last_activity_at timestamp with time zone,
    ADD COLUMN first_completed_at timestamp with time zone,
    ADD COLUMN current_grade_run_id uuid,
    ADD COLUMN best_grade_run_id uuid,
    ADD CONSTRAINT assignment_summary_staging_value_check CHECK (
        (current_score IS NULL OR current_score BETWEEN 0 AND 1)
        AND (best_score IS NULL OR best_score BETWEEN 0 AND 1)
        AND (latest_score IS NULL OR latest_score BETWEEN 0 AND 1)
        AND completed_run_count >= 0 AND total_question_attempts >= 0
    );
ALTER TABLE public.assignment_summary_staging
    ALTER COLUMN completed_run_count DROP DEFAULT,
    ALTER COLUMN total_question_attempts DROP DEFAULT;
CREATE OR REPLACE FUNCTION public.ple_retention_authorize(
    p_session character,
    p_course uuid DEFAULT NULL::uuid,
    p_admin_only boolean DEFAULT false
) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE actor uuid; roles jsonb;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles FROM public.auth_session
     WHERE session_hash = p_session AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL THEN RETURN false; END IF;
    IF p_course IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM public.course
         WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
    ) THEN RETURN false; END IF;
    IF roles @> '["sysadmin"]'::jsonb THEN RETURN true; END IF;
    IF p_admin_only OR p_course IS NULL THEN RETURN false; END IF;
    RETURN EXISTS (
        SELECT 1 FROM public.course_member
         WHERE tenant_id = public.ple_current_tenant()
           AND course_id = p_course AND user_id = actor
           AND role = 'instructor' AND status = 'active'
    );
END
$$;
CREATE OR REPLACE FUNCTION public.ple_course_appearance_actor(
    p_session character,
    p_course uuid,
    p_manager_only boolean DEFAULT false
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE actor uuid; member_role text;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id INTO actor FROM public.auth_session
     WHERE session_hash = p_session AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL OR NOT EXISTS (
        SELECT 1 FROM public.course
         WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
    ) THEN RETURN NULL; END IF;
    SELECT role INTO member_role FROM public.course_member
     WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
       AND user_id = actor AND status = 'active';
    IF member_role = 'instructor' THEN RETURN actor; END IF;
    IF NOT p_manager_only AND member_role = 'student'
       AND public.ple_course_records_accessible(public.ple_current_tenant(), p_course)
    THEN RETURN actor; END IF;
    RETURN NULL;
END
$$;
CREATE OR REPLACE FUNCTION public.ple_course_roster_actor(
    p_session character,
    p_course uuid,
    p_manager_only boolean DEFAULT true
) RETURNS uuid
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT public.ple_course_appearance_actor(p_session, p_course, p_manager_only)
$$;
CREATE OR REPLACE FUNCTION public.ple_course_roster_support_precheck(
    p_session character, p_course uuid
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE actor uuid; roles jsonb; member_role text;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles FROM public.auth_session
     WHERE session_hash = p_session AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL
       OR NOT public.ple_course_records_accessible(public.ple_current_tenant(), p_course)
    THEN RETURN NULL; END IF;
    SELECT role INTO member_role FROM public.course_member
     WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
       AND user_id = actor AND status = 'active';
    IF member_role = 'instructor' OR roles @> '["sysadmin"]'::jsonb THEN RETURN actor; END IF;
    RETURN NULL;
END
$$;
CREATE OR REPLACE FUNCTION public.ple_course_roster_support_actor(
    p_session character, p_course uuid, p_action text
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE actor uuid; roles jsonb; member_role text; audit_payload jsonb;
BEGIN
    IF p_action NOT IN ('listRoster','createInvitation','replaceEnrollmentPolicy',
                        'revokeMember','revokeInvitation','stageImport','commitImport')
    THEN RETURN NULL; END IF;
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles FROM public.auth_session
     WHERE session_hash = p_session AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL AND expires_at > transaction_timestamp();
    IF actor IS NULL
       OR NOT public.ple_course_records_accessible(public.ple_current_tenant(), p_course)
    THEN RETURN NULL; END IF;
    SELECT role INTO member_role FROM public.course_member
     WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
       AND user_id = actor AND status = 'active' FOR KEY SHARE;
    IF member_role = 'instructor' THEN RETURN actor; END IF;
    IF NOT roles @> '["sysadmin"]'::jsonb THEN RETURN NULL; END IF;
    audit_payload := jsonb_build_object('supportAction', p_action);
    INSERT INTO public.audit_event
        (tenant_id, audit_event_id, occurred_at, actor_id, course_id, action,
         target_kind, target_id, payload, payload_sha256)
    VALUES
        (public.ple_current_tenant(), gen_random_uuid(), transaction_timestamp(), actor,
         p_course, 'sysadmin.rosterSupport', 'courseRoster', p_course, audit_payload,
         encode(pg_catalog.sha256(convert_to(audit_payload::text, 'UTF8')), 'hex'));
    RETURN actor;
END
$$;
CREATE OR REPLACE FUNCTION public.ple_account_course_context_page(
    p_user uuid, p_after_tenant uuid, p_after_course uuid, p_limit integer
) RETURNS TABLE (tenant_id uuid, course_id uuid, title text, role text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT membership.tenant_id, membership.course_id, course.title, membership.role
      FROM public.course_member membership
      JOIN public.course course
        ON course.tenant_id = membership.tenant_id AND course.course_id = membership.course_id
     WHERE membership.user_id = p_user AND membership.status = 'active'
       AND (membership.role <> 'student'
            OR public.ple_course_records_accessible(membership.tenant_id, membership.course_id))
       AND (p_after_tenant IS NULL
            OR (membership.tenant_id, membership.course_id) > (p_after_tenant, p_after_course))
     ORDER BY membership.tenant_id, membership.course_id
     LIMIT least(greatest(p_limit, 1), 101)
$$;
CREATE OR REPLACE FUNCTION public.ple_account_course_context(
    p_user uuid, p_course uuid
) RETURNS TABLE (tenant_id uuid, course_id uuid, title text, role text)
    LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT membership.tenant_id, membership.course_id, course.title, membership.role
      FROM public.course_member membership
      JOIN public.course course
        ON course.tenant_id = membership.tenant_id AND course.course_id = membership.course_id
     WHERE membership.user_id = p_user AND membership.course_id = p_course
       AND membership.status = 'active'
       AND (membership.role <> 'student'
            OR public.ple_course_records_accessible(membership.tenant_id, membership.course_id))
     ORDER BY membership.tenant_id LIMIT 2
$$;
CREATE FUNCTION public.ple_fence_course_membership_write() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.role = 'student' OR (TG_OP = 'UPDATE' AND OLD.role = 'student') THEN
        IF NOT public.ple_lock_course_write(NEW.tenant_id, NEW.course_id, false) THEN
            RAISE EXCEPTION 'course membership is unavailable' USING ERRCODE = '23503';
        END IF;
    END IF;
    RETURN NEW;
END
$$;
ALTER FUNCTION public.ple_fence_course_membership_write() OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_fence_course_membership_write() FROM PUBLIC;
CREATE TRIGGER course_membership_retention_fence
    BEFORE INSERT OR UPDATE ON public.course_member
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_membership_write();
CREATE FUNCTION public.ple_guard_course_membership_episode() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF (NEW.tenant_id, NEW.course_id, NEW.course_membership_id, NEW.user_id,
        NEW.student_id, NEW.role, NEW.joined_at)
       IS DISTINCT FROM
       (OLD.tenant_id, OLD.course_id, OLD.course_membership_id, OLD.user_id,
        OLD.student_id, OLD.role, OLD.joined_at)
    THEN
        RAISE EXCEPTION 'course membership episode identity is immutable'
            USING ERRCODE = '55000';
    END IF;
    IF NEW.status = OLD.status THEN
        IF OLD.status <> 'active' OR NEW.revoked_at IS DISTINCT FROM OLD.revoked_at THEN
            RAISE EXCEPTION 'revoked course membership episodes are immutable'
                USING ERRCODE = '55000';
        END IF;
        RETURN NEW;
    END IF;
    IF OLD.status <> 'active' OR NEW.status <> 'revoked'
       OR NEW.revoked_at IS NULL OR NEW.revoked_at < OLD.joined_at THEN
        RAISE EXCEPTION 'course membership status may only transition active to revoked'
            USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER course_membership_episode_immutable
    BEFORE UPDATE ON public.course_member
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_course_membership_episode();
CREATE FUNCTION public.ple_remove_revoked_course_group_memberships() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF OLD.status = 'active' AND NEW.status = 'revoked' THEN
        DELETE FROM public.course_group_member
         WHERE tenant_id = NEW.tenant_id
           AND course_id = NEW.course_id
           AND course_membership_id = NEW.course_membership_id;
    END IF;
    RETURN NEW;
END
$$;
ALTER FUNCTION public.ple_remove_revoked_course_group_memberships()
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_remove_revoked_course_group_memberships() FROM PUBLIC;
CREATE TRIGGER course_membership_revocation_removes_current_group_memberships
    AFTER UPDATE OF status ON public.course_member
    FOR EACH ROW EXECUTE FUNCTION public.ple_remove_revoked_course_group_memberships();
GRANT SELECT ON public.course_member TO ple_roster_support_broker;
GRANT UPDATE (tenant_id) ON public.course_member TO ple_roster_support_broker;
REVOKE DELETE ON public.course_member FROM ple_app;
REVOKE UPDATE ON public.course_member FROM ple_app;
GRANT UPDATE (status, revoked_at, roster_id) ON public.course_member TO ple_app;
CREATE FUNCTION public.ple_guard_course_membership_delete() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF current_user = 'ple_retention_broker' THEN RETURN OLD; END IF;
    RAISE EXCEPTION 'course membership deletion is retention-owned' USING ERRCODE = '55000';
END
$$;
CREATE TRIGGER course_membership_delete_retention_only
    BEFORE DELETE ON public.course_member
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_course_membership_delete();
CREATE OR REPLACE FUNCTION public.ple_commit_delete_retention_work(
    p_tenant uuid, p_job uuid, p_token uuid, p_course uuid, p_stage text, p_generation bigint
) RETURNS boolean
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE committed boolean;
BEGIN
    committed := public.ple_commit_delete_retention_work_before_passwordless_identity(
        p_tenant, p_job, p_token, p_course, p_stage, p_generation
    );
    IF NOT committed THEN RETURN false; END IF;
    PERFORM set_config('ple.tenant_id', p_tenant::text, true);
    DELETE FROM public.course_grade_export_audit
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_roster_import
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_invitation
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.tenant_learner_identity learner
     WHERE learner.tenant_id = p_tenant
       AND NOT EXISTS (
           SELECT 1 FROM public.course_member membership
            WHERE membership.tenant_id = learner.tenant_id
              AND membership.user_id = learner.user_id
              AND membership.role = 'student'
       )
       AND NOT EXISTS (
           SELECT 1 FROM public.enrollment enrollment
            WHERE enrollment.tenant_id = learner.tenant_id
              AND enrollment.user_id = learner.user_id
       );
    RETURN NOT EXISTS (
        SELECT 1 FROM public.course_invitation
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL
        SELECT 1 FROM public.course_roster_profile
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL
        SELECT 1 FROM public.course_grade_export_audit
         WHERE tenant_id = p_tenant AND course_id = p_course
    );
END
$$;
ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    FROM PUBLIC;
REVOKE DELETE ON public.enrollment FROM ple_app;
CREATE FUNCTION public.ple_guard_enrollment_delete() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF current_user = 'ple_retention_broker' THEN RETURN OLD; END IF;
    RAISE EXCEPTION 'enrollment deletion is retention-owned' USING ERRCODE = '55000';
END
$$;
CREATE TRIGGER enrollment_delete_retention_only
    BEFORE DELETE ON public.enrollment
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_enrollment_delete();
COMMIT;
