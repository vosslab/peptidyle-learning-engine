-- Forward migration: revisioned course appearance and protected banner lifecycle.

ALTER TABLE public.asset_delivery
    DROP CONSTRAINT asset_delivery_check;

ALTER TABLE public.asset_delivery
    DROP CONSTRAINT asset_delivery_course_shape;

ALTER TABLE public.asset_delivery
    DROP CONSTRAINT asset_delivery_delivery_kind_check;

ALTER TABLE public.asset_delivery
    ADD CONSTRAINT asset_delivery_check CHECK (
        (delivery_kind = 'catalog'
            AND tenant_id IS NULL AND course_id IS NULL
            AND problem_id IS NOT NULL AND version_id IS NOT NULL AND asset_id IS NOT NULL
            AND delivery_id = asset_id)
        OR
        (delivery_kind = 'student_record'
            AND tenant_id IS NOT NULL AND course_id IS NOT NULL
            AND problem_id IS NULL AND version_id IS NULL AND asset_id IS NULL
            AND delivery_id = object_id)
        OR
        (delivery_kind = 'course_banner'
            AND tenant_id IS NOT NULL AND course_id IS NOT NULL
            AND problem_id IS NULL AND version_id IS NULL AND asset_id IS NULL)
    );

ALTER TABLE public.asset_delivery
    ADD CONSTRAINT asset_delivery_course_shape CHECK (
        (delivery_kind = 'catalog' AND course_id IS NULL)
        OR (delivery_kind IN ('student_record', 'course_banner') AND course_id IS NOT NULL)
    );

ALTER TABLE public.asset_delivery
    ADD CONSTRAINT asset_delivery_delivery_kind_check CHECK (
        delivery_kind IN ('catalog', 'student_record', 'course_banner')
    );

ALTER TABLE public.record_access_log
    DROP CONSTRAINT record_access_log_delivery_scope_check;

ALTER TABLE public.record_access_log
    DROP CONSTRAINT record_access_log_delivery_scope_shape_check;

ALTER TABLE public.record_access_log
    ADD CONSTRAINT record_access_log_delivery_scope_check CHECK (
        delivery_scope IN ('catalog', 'student_record', 'course_banner')
    );

ALTER TABLE public.record_access_log
    ADD CONSTRAINT record_access_log_delivery_scope_shape_check CHECK (
        (delivery_scope = 'catalog' AND delivery_id IS NOT NULL AND course_id IS NULL)
        OR
        (delivery_scope IN ('student_record', 'course_banner')
            AND delivery_id IS NOT NULL AND course_id IS NOT NULL)
    );

CREATE TABLE public.course_appearance (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    theme_id text DEFAULT 'grass' NOT NULL,
    current_banner_delivery_id uuid,
    banner_alt_kind text,
    banner_alt_text text,
    revision bigint DEFAULT 1 NOT NULL,
    updated_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT course_appearance_theme_check CHECK (
        theme_id IN (
            'tundra', 'forest', 'desert', 'grass', 'arctic', 'ocean', 'tropical',
            'coral-reef', 'swamp', 'underground', 'salt-marsh', 'wetland',
            'sea-floor', 'magma', 'beach'
        )
    ),
    CONSTRAINT course_appearance_revision_check CHECK (revision > 0),
    CONSTRAINT course_appearance_banner_shape_check CHECK (
        (current_banner_delivery_id IS NULL
            AND banner_alt_kind IS NULL AND banner_alt_text IS NULL)
        OR
        (current_banner_delivery_id IS NOT NULL
            AND banner_alt_kind = 'decorative' AND banner_alt_text IS NULL)
        OR
        (current_banner_delivery_id IS NOT NULL
            AND banner_alt_kind = 'informative'
            AND char_length(banner_alt_text) BETWEEN 1 AND 160
            AND banner_alt_text <> '' AND banner_alt_text !~ '^\s*$')
    )
);

ALTER TABLE ONLY public.course_appearance FORCE ROW LEVEL SECURITY;

CREATE TABLE public.course_banner_candidate (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    candidate_id uuid NOT NULL,
    created_by uuid NOT NULL,
    candidate_object_id uuid NOT NULL,
    normalized_sha256 character(64) NOT NULL,
    size_bytes bigint NOT NULL,
    width integer NOT NULL,
    height integer NOT NULL,
    future_banner_id uuid NOT NULL,
    future_object_id uuid NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    promoted_payload jsonb,
    promoted_payload_sha256 character(64),
    consumed boolean DEFAULT false NOT NULL,
    candidate_deleted boolean DEFAULT false NOT NULL,
    cleanup_claim_id uuid,
    cleanup_claim_expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT transaction_timestamp() NOT NULL,
    CONSTRAINT course_banner_candidate_normalized_check CHECK (
        width = 1200 AND height = 328 AND size_bytes > 0
            AND normalized_sha256 ~ '^[0-9a-f]{64}$'
    ),
    CONSTRAINT course_banner_candidate_expiry_check CHECK (expires_at > created_at),
    CONSTRAINT course_banner_candidate_promotion_check CHECK (
        (promoted_payload IS NULL) = (promoted_payload_sha256 IS NULL)
    ),
    CONSTRAINT course_banner_candidate_cleanup_check CHECK (
        (cleanup_claim_id IS NULL) = (cleanup_claim_expires_at IS NULL)
    )
);

ALTER TABLE ONLY public.course_banner_candidate FORCE ROW LEVEL SECURITY;

ALTER TABLE ONLY public.course_appearance
    ADD CONSTRAINT course_appearance_pkey PRIMARY KEY (tenant_id, course_id);

ALTER TABLE ONLY public.course_banner_candidate
    ADD CONSTRAINT course_banner_candidate_pkey PRIMARY KEY (tenant_id, course_id, candidate_id);

ALTER TABLE ONLY public.course_banner_candidate
    ADD CONSTRAINT course_banner_candidate_future_banner_key UNIQUE (future_banner_id);

ALTER TABLE ONLY public.course_banner_candidate
    ADD CONSTRAINT course_banner_candidate_future_object_key UNIQUE (future_object_id);

ALTER TABLE ONLY public.course_banner_candidate
    ADD CONSTRAINT course_banner_candidate_object_key UNIQUE (candidate_object_id);

CREATE INDEX course_banner_candidate_cleanup_idx
    ON public.course_banner_candidate (tenant_id, expires_at, candidate_id)
    WHERE expires_at IS NOT NULL;

ALTER TABLE ONLY public.course_appearance
    ADD CONSTRAINT course_appearance_course_fk
    FOREIGN KEY (tenant_id, course_id)
    REFERENCES public.course(tenant_id, course_id) ON DELETE CASCADE;

ALTER TABLE ONLY public.course_appearance
    ADD CONSTRAINT course_appearance_banner_delivery_fk
    FOREIGN KEY (current_banner_delivery_id)
    REFERENCES public.asset_delivery(delivery_id);

ALTER TABLE ONLY public.course_banner_candidate
    ADD CONSTRAINT course_banner_candidate_course_fk
    FOREIGN KEY (tenant_id, course_id)
    REFERENCES public.course(tenant_id, course_id) ON DELETE CASCADE;

CREATE FUNCTION public.ple_create_course_appearance() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    INSERT INTO public.course_appearance (tenant_id, course_id)
    VALUES (NEW.tenant_id, NEW.course_id)
    ON CONFLICT (tenant_id, course_id) DO NOTHING;
    RETURN NEW;
END $$;

CREATE FUNCTION public.ple_course_appearance_actor(
    p_session character,
    p_course uuid,
    p_manager_only boolean DEFAULT false
) RETURNS uuid
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
DECLARE
    actor uuid;
    roles jsonb;
    member_role text;
BEGIN
    PERFORM set_config('ple.session_hash', p_session, true);
    SELECT user_id, auth_session.roles INTO actor, roles
      FROM public.auth_session
     WHERE session_hash = p_session
       AND tenant_id = public.ple_current_tenant()
       AND revoked_at IS NULL
       AND expires_at > transaction_timestamp();
    IF actor IS NULL OR NOT EXISTS (
        SELECT 1 FROM public.course
         WHERE tenant_id = public.ple_current_tenant() AND course_id = p_course
    ) THEN
        RETURN NULL;
    END IF;
    IF roles @> '["administrator"]'::jsonb THEN
        RETURN actor;
    END IF;
    SELECT role INTO member_role
      FROM public.course_member
     WHERE tenant_id = public.ple_current_tenant()
       AND course_id = p_course AND user_id = actor;
    IF member_role = 'instructor' THEN
        RETURN actor;
    END IF;
    IF NOT p_manager_only
       AND member_role = 'student'
       AND public.ple_course_records_accessible(public.ple_current_tenant(), p_course) THEN
        RETURN actor;
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION public.ple_course_appearance_authorize(
    p_session character,
    p_course uuid,
    p_manager_only boolean DEFAULT false
) RETURNS boolean
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
    SELECT public.ple_course_appearance_actor(p_session, p_course, p_manager_only) IS NOT NULL
$$;

CREATE FUNCTION public.ple_validate_course_appearance_banner() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public'
    AS $$
BEGIN
    IF NEW.current_banner_delivery_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
          FROM public.asset_delivery
         WHERE delivery_id = NEW.current_banner_delivery_id
           AND delivery_kind = 'course_banner'
           AND tenant_id = NEW.tenant_id
           AND course_id = NEW.course_id
    ) THEN
        RAISE EXCEPTION 'current course banner delivery does not match its course'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER course_create_appearance
    AFTER INSERT ON public.course
    FOR EACH ROW EXECUTE FUNCTION public.ple_create_course_appearance();

CREATE TRIGGER course_appearance_validate_banner
    BEFORE INSERT OR UPDATE OF tenant_id, course_id, current_banner_delivery_id
    ON public.course_appearance
    FOR EACH ROW EXECUTE FUNCTION public.ple_validate_course_appearance_banner();

INSERT INTO public.course_appearance (tenant_id, course_id)
SELECT tenant_id, course_id FROM public.course
ON CONFLICT (tenant_id, course_id) DO NOTHING;

ALTER TABLE public.course_appearance ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_appearance_tenant ON public.course_appearance TO ple_app
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

ALTER TABLE public.course_banner_candidate ENABLE ROW LEVEL SECURITY;

CREATE POLICY course_banner_candidate_tenant ON public.course_banner_candidate TO ple_app
    USING (tenant_id = public.ple_current_tenant())
    WITH CHECK (tenant_id = public.ple_current_tenant());

CREATE POLICY asset_delivery_course_banner_select ON public.asset_delivery
    FOR SELECT TO ple_app
    USING (delivery_kind = 'course_banner' AND tenant_id = public.ple_current_tenant());

CREATE POLICY asset_delivery_course_banner_insert ON public.asset_delivery
    FOR INSERT TO ple_app
    WITH CHECK (delivery_kind = 'course_banner' AND tenant_id = public.ple_current_tenant());

CREATE POLICY asset_delivery_course_banner_delete ON public.asset_delivery
    FOR DELETE TO ple_app
    USING (delivery_kind = 'course_banner' AND tenant_id = public.ple_current_tenant());

GRANT SELECT, INSERT, UPDATE ON TABLE public.course_appearance TO ple_app;
GRANT SELECT, INSERT, DELETE, UPDATE ON TABLE public.course_banner_candidate TO ple_app;

REVOKE ALL ON FUNCTION public.ple_create_course_appearance() FROM PUBLIC;

REVOKE ALL ON FUNCTION public.ple_course_appearance_actor(character, uuid, boolean)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_course_appearance_actor(character, uuid, boolean)
    TO ple_app;

REVOKE ALL ON FUNCTION public.ple_course_appearance_authorize(character, uuid, boolean)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_course_appearance_authorize(character, uuid, boolean)
    TO ple_app;

REVOKE ALL ON FUNCTION public.ple_validate_course_appearance_banner() FROM PUBLIC;
