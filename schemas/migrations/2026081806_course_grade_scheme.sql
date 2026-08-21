-- WP-PROF-S6: course-owned total-points and weighted-category grade schemes.
BEGIN;

CREATE TABLE public.course_grade_scheme (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    mode text NOT NULL,
    rounding text NOT NULL,
    revision bigint NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT course_grade_scheme_pkey PRIMARY KEY (tenant_id, course_id),
    CONSTRAINT course_grade_scheme_course_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id),
    CONSTRAINT course_grade_scheme_mode_check
        CHECK (mode IN ('total_points', 'weighted_categories')),
    CONSTRAINT course_grade_scheme_rounding_check
        CHECK (rounding = 'four_decimal_places_half_away_from_zero'),
    CONSTRAINT course_grade_scheme_revision_check CHECK (revision > 0)
);

CREATE TABLE public.course_grade_category (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    category_id uuid NOT NULL,
    position integer NOT NULL,
    title text NOT NULL,
    weight_basis_points integer NOT NULL,
    drop_lowest integer NOT NULL,
    CONSTRAINT course_grade_category_pkey PRIMARY KEY (tenant_id, category_id),
    CONSTRAINT course_grade_category_course_scheme_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course_grade_scheme(tenant_id, course_id),
    CONSTRAINT course_grade_category_course_identity_key UNIQUE (tenant_id, course_id, category_id),
    CONSTRAINT course_grade_category_position_key UNIQUE (tenant_id, course_id, position),
    CONSTRAINT course_grade_category_title_check
        CHECK (char_length(title) BETWEEN 1 AND 200 AND title = btrim(title)),
    CONSTRAINT course_grade_category_position_check CHECK (position >= 0),
    CONSTRAINT course_grade_category_weight_check
        CHECK (weight_basis_points BETWEEN 1 AND 10000),
    CONSTRAINT course_grade_category_drop_lowest_check CHECK (drop_lowest >= 0)
);

CREATE TABLE public.course_grade_category_assignment (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    category_id uuid NOT NULL,
    assignment_id uuid NOT NULL,
    position integer NOT NULL,
    CONSTRAINT course_grade_category_assignment_pkey
        PRIMARY KEY (tenant_id, category_id, assignment_id),
    CONSTRAINT course_grade_category_assignment_category_fk
        FOREIGN KEY (tenant_id, course_id, category_id)
        REFERENCES public.course_grade_category(tenant_id, course_id, category_id),
    CONSTRAINT course_grade_category_assignment_assignment_fk
        FOREIGN KEY (tenant_id, course_id, assignment_id)
        REFERENCES public.assignment(tenant_id, course_id, assignment_id) ON DELETE CASCADE,
    CONSTRAINT course_grade_category_assignment_membership_key
        UNIQUE (tenant_id, assignment_id),
    CONSTRAINT course_grade_category_assignment_position_key
        UNIQUE (tenant_id, category_id, position),
    CONSTRAINT course_grade_category_assignment_position_check CHECK (position >= 0)
);

CREATE TABLE public.course_grade_letter_band (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    letter_band_id uuid NOT NULL,
    label text NOT NULL,
    minimum_basis_points integer NOT NULL,
    CONSTRAINT course_grade_letter_band_pkey PRIMARY KEY (tenant_id, letter_band_id),
    CONSTRAINT course_grade_letter_band_course_scheme_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course_grade_scheme(tenant_id, course_id),
    CONSTRAINT course_grade_letter_band_label_key UNIQUE (tenant_id, course_id, label),
    CONSTRAINT course_grade_letter_band_minimum_key
        UNIQUE (tenant_id, course_id, minimum_basis_points),
    CONSTRAINT course_grade_letter_band_label_check
        CHECK (char_length(label) BETWEEN 1 AND 32 AND label = btrim(label)),
    CONSTRAINT course_grade_letter_band_minimum_check
        CHECK (minimum_basis_points BETWEEN 0 AND 10000)
);

CREATE TABLE public.course_total_export_audit (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    export_id uuid NOT NULL,
    requested_by uuid NOT NULL,
    row_count integer NOT NULL,
    scheme_revision bigint NOT NULL,
    mode text NOT NULL,
    rounding text NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),
    CONSTRAINT course_total_export_audit_pkey PRIMARY KEY (tenant_id, export_id),
    CONSTRAINT course_total_export_audit_course_fk FOREIGN KEY (tenant_id, course_id)
        REFERENCES public.course(tenant_id, course_id),
    CONSTRAINT course_total_export_audit_row_count_check CHECK (row_count BETWEEN 0 AND 500),
    CONSTRAINT course_total_export_audit_scheme_revision_check CHECK (scheme_revision > 0),
    CONSTRAINT course_total_export_audit_mode_check
        CHECK (mode IN ('total_points', 'weighted_categories')),
    CONSTRAINT course_total_export_audit_rounding_check
        CHECK (rounding = 'four_decimal_places_half_away_from_zero')
);

CREATE INDEX course_total_export_audit_course_created_idx
    ON public.course_total_export_audit (tenant_id, course_id, created_at, export_id);

-- The S3 compact score projection remains the source for course aggregation,
-- but its accepted 0..1 check cannot represent the already-supported
-- extra-credit and penalty range. Preserve the existing nonnegative counters
-- exactly while widening only the three selected-score columns.
ALTER TABLE public.student_assignment_summary
    DROP CONSTRAINT student_assignment_summary_value_check,
    ADD CONSTRAINT student_assignment_summary_value_check CHECK (
        (current_score IS NULL OR current_score BETWEEN -1000 AND 1000)
        AND (best_score IS NULL OR best_score BETWEEN -1000 AND 1000)
        AND (latest_score IS NULL OR latest_score BETWEEN -1000 AND 1000)
        AND completed_run_count >= 0 AND total_question_attempts >= 0
    );
ALTER TABLE public.assignment_summary_staging
    DROP CONSTRAINT assignment_summary_staging_value_check,
    ADD CONSTRAINT assignment_summary_staging_value_check CHECK (
        (current_score IS NULL OR current_score BETWEEN -1000 AND 1000)
        AND (best_score IS NULL OR best_score BETWEEN -1000 AND 1000)
        AND (latest_score IS NULL OR latest_score BETWEEN -1000 AND 1000)
        AND completed_run_count >= 0 AND total_question_attempts >= 0
    );

INSERT INTO public.course_grade_scheme (tenant_id, course_id, mode, rounding, revision)
SELECT tenant_id, course_id, 'total_points', 'four_decimal_places_half_away_from_zero', 1
FROM public.course;

CREATE FUNCTION public.ple_create_course_grade_scheme() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    INSERT INTO public.course_grade_scheme
        (tenant_id, course_id, mode, rounding, revision)
    VALUES
        (NEW.tenant_id, NEW.course_id, 'total_points',
         'four_decimal_places_half_away_from_zero', 1);
    RETURN NEW;
END
$$;

CREATE CONSTRAINT TRIGGER course_creates_grade_scheme
    AFTER INSERT ON public.course
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION public.ple_create_course_grade_scheme();

ALTER TABLE public.course_grade_scheme ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_scheme FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_category ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_category FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_category_assignment ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_category_assignment FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_letter_band ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_grade_letter_band FORCE ROW LEVEL SECURITY;
ALTER TABLE public.course_total_export_audit ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.course_total_export_audit FORCE ROW LEVEL SECURITY;

CREATE POLICY course_grade_scheme_app ON public.course_grade_scheme TO ple_app
    USING (tenant_id = public.ple_current_tenant()
           AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant()
                AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY course_grade_category_app ON public.course_grade_category TO ple_app
    USING (tenant_id = public.ple_current_tenant()
           AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant()
                AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY course_grade_category_assignment_app
    ON public.course_grade_category_assignment TO ple_app
    USING (tenant_id = public.ple_current_tenant()
           AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant()
                AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY course_grade_letter_band_app ON public.course_grade_letter_band TO ple_app
    USING (tenant_id = public.ple_current_tenant()
           AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant()
                AND public.ple_course_records_accessible(tenant_id, course_id));
CREATE POLICY course_total_export_audit_app ON public.course_total_export_audit TO ple_app
    USING (tenant_id = public.ple_current_tenant()
           AND public.ple_course_records_accessible(tenant_id, course_id))
    WITH CHECK (tenant_id = public.ple_current_tenant()
                AND public.ple_course_records_accessible(tenant_id, course_id));

CREATE POLICY course_grade_scheme_retention ON public.course_grade_scheme
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_category_retention ON public.course_grade_category
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_category_assignment_retention
    ON public.course_grade_category_assignment TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_grade_letter_band_retention ON public.course_grade_letter_band
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());
CREATE POLICY course_total_export_audit_retention ON public.course_total_export_audit
    TO ple_retention_broker USING (tenant_id = public.ple_current_tenant());

-- The accepted S3 deletion function already removes this S3/S4 learner-owned
-- exception relation, but its original migration omitted the broker policy
-- and grant. The S6 retention oracle exercises that established deletion path
-- before removing the new course-grade relations.
CREATE POLICY assignment_individual_policy_exception_retention
    ON public.assignment_individual_policy_exception TO ple_retention_broker
    USING (tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT, UPDATE ON public.course_grade_scheme TO ple_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.course_grade_category,
    public.course_grade_category_assignment, public.course_grade_letter_band TO ple_app;
GRANT SELECT, INSERT ON public.course_total_export_audit TO ple_app;
GRANT SELECT, DELETE ON public.course_grade_scheme, public.course_grade_category,
    public.course_grade_category_assignment, public.course_grade_letter_band,
    public.course_total_export_audit TO ple_retention_broker;
GRANT SELECT, DELETE ON public.assignment_individual_policy_exception
    TO ple_retention_broker;

-- The deferred course trigger owns the only insert and must also support
-- privileged course-bootstrap paths that intentionally have no tenant session.
-- The primary key, foreign key, and absence of application DELETE authority
-- make a later application insert impossible; fence the mutable path.
CREATE TRIGGER course_grade_scheme_retention_fence
    BEFORE UPDATE ON public.course_grade_scheme
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();
CREATE TRIGGER course_grade_category_retention_fence
    BEFORE INSERT OR UPDATE ON public.course_grade_category
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();
CREATE TRIGGER course_grade_category_assignment_retention_fence
    BEFORE INSERT OR UPDATE ON public.course_grade_category_assignment
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();
CREATE TRIGGER course_grade_letter_band_retention_fence
    BEFORE INSERT OR UPDATE ON public.course_grade_letter_band
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();
CREATE TRIGGER course_total_export_audit_retention_fence
    BEFORE INSERT ON public.course_total_export_audit
    FOR EACH ROW EXECUTE FUNCTION public.ple_fence_course_roster_write();

-- Categories are a normalized representation of the weighted mode only. The
-- roster-write fence fires first (trigger names sort before `zz_`), which also
-- serializes a mode switch against category creation for one course.
CREATE FUNCTION public.ple_guard_course_grade_category_mode() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE scheme_mode text;
BEGIN
    SELECT mode INTO scheme_mode
      FROM public.course_grade_scheme
     WHERE tenant_id = NEW.tenant_id AND course_id = NEW.course_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'course grade scheme is unavailable' USING ERRCODE = '23503';
    END IF;
    IF scheme_mode <> 'weighted_categories' THEN
        RAISE EXCEPTION 'grade categories require weighted_categories mode'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
ALTER FUNCTION public.ple_guard_course_grade_category_mode() OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_guard_course_grade_category_mode() FROM PUBLIC;

CREATE FUNCTION public.ple_guard_course_grade_scheme_category_mode() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.mode = 'total_points'
       AND EXISTS (
           SELECT 1 FROM public.course_grade_category
            WHERE tenant_id = NEW.tenant_id AND course_id = NEW.course_id
       )
    THEN
        RAISE EXCEPTION 'delete grade categories before selecting total_points mode'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END
$$;
ALTER FUNCTION public.ple_guard_course_grade_scheme_category_mode()
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_guard_course_grade_scheme_category_mode() FROM PUBLIC;

CREATE TRIGGER zz_course_grade_category_mode_guard
    BEFORE INSERT OR UPDATE ON public.course_grade_category
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_course_grade_category_mode();
CREATE TRIGGER zz_course_grade_scheme_category_mode_guard
    BEFORE UPDATE OF mode ON public.course_grade_scheme
    FOR EACH ROW EXECUTE FUNCTION public.ple_guard_course_grade_scheme_category_mode();

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
    DELETE FROM public.course_total_export_audit
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_category_assignment
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_letter_band
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_category
     WHERE tenant_id = p_tenant AND course_id = p_course;
    DELETE FROM public.course_grade_scheme
     WHERE tenant_id = p_tenant AND course_id = p_course;
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
              AND membership.user_id = learner.user_id AND membership.role = 'student'
       )
       AND NOT EXISTS (
           SELECT 1 FROM public.enrollment enrollment
            WHERE enrollment.tenant_id = learner.tenant_id AND enrollment.user_id = learner.user_id
       );
    RETURN NOT EXISTS (
        SELECT 1 FROM public.course_invitation
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_roster_profile
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_export_audit
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_total_export_audit
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_category_assignment
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_letter_band
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_category
         WHERE tenant_id = p_tenant AND course_id = p_course
        UNION ALL SELECT 1 FROM public.course_grade_scheme
         WHERE tenant_id = p_tenant AND course_id = p_course
    );
END
$$;
ALTER FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    OWNER TO ple_retention_broker;
REVOKE ALL ON FUNCTION public.ple_commit_delete_retention_work(uuid, uuid, uuid, uuid, text, bigint)
    FROM PUBLIC;

COMMIT;
