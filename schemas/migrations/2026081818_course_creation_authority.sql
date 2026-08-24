BEGIN; CREATE EXTENSION IF NOT EXISTS pgcrypto;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_course_creation_broker') THEN CREATE ROLE ple_course_creation_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_base_course_installer') THEN CREATE ROLE ple_base_course_installer NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_base_course_install_broker') THEN CREATE ROLE ple_base_course_install_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_base_course_freshness_broker') THEN CREATE ROLE ple_base_course_freshness_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_base_course_completion_verification_broker') THEN BEGIN CREATE ROLE ple_base_course_completion_verification_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; END; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_course_roster_mutator_broker') THEN CREATE ROLE ple_course_roster_mutator_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF;
END $$;
DO $$ DECLARE role_name text; BEGIN FOREACH role_name IN ARRAY ARRAY['ple_course_creation_broker','ple_base_course_installer','ple_base_course_install_broker','ple_base_course_freshness_broker','ple_base_course_completion_verification_broker','ple_course_roster_mutator_broker'] LOOP IF EXISTS(SELECT 1 FROM pg_roles WHERE rolname=role_name AND (rolcanlogin OR rolsuper OR rolcreatedb OR rolcreaterole OR rolinherit OR rolreplication OR rolbypassrls)) THEN EXECUTE format('ALTER ROLE %I NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS',role_name); END IF; END LOOP; END $$;
REVOKE ALL ON SCHEMA public FROM ple_course_creation_broker,ple_base_course_installer,ple_base_course_install_broker,ple_base_course_freshness_broker,ple_base_course_completion_verification_broker,ple_course_roster_mutator_broker; GRANT USAGE ON SCHEMA public TO ple_course_creation_broker,ple_base_course_installer,ple_base_course_install_broker,ple_base_course_freshness_broker,ple_base_course_completion_verification_broker,ple_course_roster_mutator_broker;
REVOKE INSERT,UPDATE,DELETE ON public.course,public.course_member,public.course_roster_state,public.course_group_membership_policy,public.course_grade_scheme FROM ple_app; CREATE POLICY course_creation_broker_course_tenant ON public.course TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_member_tenant ON public.course_member TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_creation_broker_roster_tenant ON public.course_roster_state TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_group_tenant ON public.course_group_membership_policy TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_creation_broker_scheme_tenant ON public.course_grade_scheme TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_appearance_tenant ON public.course_appearance TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_creation_broker_session_tenant ON public.auth_session TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_profile_tenant ON public.course_roster_profile TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_creation_broker_identity_tenant ON public.tenant_learner_identity TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_domain_tenant ON public.course_allowed_email_domain TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_creation_broker_course_group_tenant ON public.course_group TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_group_member_tenant ON public.course_group_member TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_creation_broker_grade_category_tenant ON public.course_grade_category TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_category_assignment_tenant ON public.course_grade_category_assignment TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_letter_band_tenant ON public.course_grade_letter_band TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant());
CREATE POLICY course_creation_broker_assignment_tenant ON public.assignment TO ple_course_creation_broker USING(tenant_id=public.ple_current_tenant());
GRANT SELECT,INSERT,UPDATE(course_id) ON public.course TO ple_course_creation_broker;
GRANT SELECT,INSERT,UPDATE(course_membership_id) ON public.course_member TO ple_course_creation_broker;
GRANT SELECT,INSERT,UPDATE(course_id) ON public.course_roster_state TO ple_course_creation_broker;
GRANT SELECT,INSERT ON public.course_group_membership_policy,public.course_grade_scheme,public.course_appearance TO ple_course_creation_broker;
GRANT SELECT,UPDATE(session_hash) ON public.auth_session TO ple_course_creation_broker;
GRANT SELECT ON public.course_roster_profile,public.tenant_learner_identity,public.course_allowed_email_domain,public.course_group,public.course_group_member,public.course_grade_category,public.course_grade_category_assignment,public.course_grade_letter_band,public.assignment TO ple_course_creation_broker;
GRANT USAGE,SELECT ON SEQUENCE public.course_public_id_seq,public.course_member_public_id_seq TO ple_course_creation_broker;
-- ASVS 2.3.3, 2.3.4, 8.2.1-8.2.3: retain the accepted approval row lock without broad approval-table visibility.
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),public.ple_course_records_accessible(uuid,uuid),public.ple_lock_instructor_approval_eligibility(uuid) TO ple_course_creation_broker;
CREATE POLICY course_roster_mutator_course_tenant ON public.course TO ple_course_roster_mutator_broker USING(tenant_id=public.ple_current_tenant());
CREATE POLICY course_roster_mutator_member_tenant ON public.course_member TO ple_course_roster_mutator_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
CREATE POLICY course_roster_mutator_identity_tenant ON public.tenant_learner_identity TO ple_course_roster_mutator_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
CREATE POLICY course_roster_mutator_profile_tenant ON public.course_roster_profile TO ple_course_roster_mutator_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
CREATE POLICY course_roster_mutator_state_tenant ON public.course_roster_state TO ple_course_roster_mutator_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant());
GRANT SELECT,UPDATE(course_id) ON public.course TO ple_course_roster_mutator_broker;
GRANT SELECT,INSERT,UPDATE(course_membership_id,status,revoked_at) ON public.course_member TO ple_course_roster_mutator_broker;
GRANT SELECT,INSERT ON public.tenant_learner_identity,public.course_roster_profile TO ple_course_roster_mutator_broker;
GRANT SELECT,UPDATE(revision,updated_at) ON public.course_roster_state TO ple_course_roster_mutator_broker;
GRANT USAGE,SELECT ON SEQUENCE public.course_member_public_id_seq TO ple_course_roster_mutator_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),public.ple_course_records_accessible(uuid,uuid),public.ple_course_roster_support_actor(character,uuid,text) TO ple_course_roster_mutator_broker;
CREATE TABLE public.live_demo_install_recipe (
 singleton boolean NOT NULL DEFAULT true, installation_generation uuid NOT NULL, tenant_id uuid NOT NULL,
 baseline_version text NOT NULL, recipe jsonb NOT NULL, recipe_sha256 text NOT NULL,
 CONSTRAINT live_demo_install_recipe_pkey PRIMARY KEY(singleton,installation_generation),
 CONSTRAINT live_demo_install_recipe_singleton CHECK(singleton),
 CONSTRAINT live_demo_install_recipe_baseline CHECK(baseline_version='base-course-v1'),
 CONSTRAINT live_demo_install_recipe_sha256 CHECK(recipe_sha256 ~ '^[0-9a-f]{64}$'),
 CONSTRAINT live_demo_install_recipe_generation UNIQUE(installation_generation));
ALTER TABLE public.live_demo_install_recipe ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.live_demo_install_recipe FORCE ROW LEVEL SECURITY;
CREATE POLICY base_course_install_broker_recipe ON public.live_demo_install_recipe TO ple_base_course_install_broker USING(true) WITH CHECK(true);
CREATE POLICY base_course_install_broker_account ON public.ple_account TO ple_base_course_install_broker USING(true) WITH CHECK(true);
ALTER TABLE public.live_demo_install_state DROP CONSTRAINT live_demo_install_state_lifecycle_check,ADD COLUMN completion_receipt_sha256 text,ADD CONSTRAINT live_demo_install_state_completion_receipt_sha256_check CHECK(completion_receipt_sha256 IS NULL OR completion_receipt_sha256~'^[0-9a-f]{64}$'),ADD CONSTRAINT live_demo_install_state_lifecycle_check CHECK((state='installing' AND tenant_id IS NOT NULL AND storage_receipt_sha256 IS NULL AND completion_receipt_sha256 IS NULL AND completed_at IS NULL) OR (state='complete' AND tenant_id IS NOT NULL AND storage_receipt_sha256 IS NOT NULL AND completion_receipt_sha256 IS NOT NULL AND completed_at IS NOT NULL));
CREATE TABLE public.live_demo_install_completion_receipt(installation_generation uuid PRIMARY KEY,tenant_id uuid NOT NULL,schema_version integer NOT NULL,baseline_version text NOT NULL,recipe_sha256 text NOT NULL,canonical_receipt jsonb NOT NULL,receipt_sha256 text NOT NULL,completed_at timestamp with time zone NOT NULL DEFAULT transaction_timestamp(),CONSTRAINT live_demo_install_completion_receipt_recipe_fk FOREIGN KEY(installation_generation) REFERENCES public.live_demo_install_recipe(installation_generation),CONSTRAINT live_demo_install_completion_receipt_schema_check CHECK(schema_version=1),CONSTRAINT live_demo_install_completion_receipt_baseline_check CHECK(baseline_version='base-course-v1'),CONSTRAINT live_demo_install_completion_receipt_recipe_sha_check CHECK(recipe_sha256~'^[0-9a-f]{64}$'),CONSTRAINT live_demo_install_completion_receipt_sha_check CHECK(receipt_sha256~'^[0-9a-f]{64}$'),CONSTRAINT live_demo_install_completion_receipt_digest_check CHECK(receipt_sha256=encode(digest(convert_to(canonical_receipt::text,'UTF8'),'sha256'),'hex')));
ALTER TABLE public.live_demo_install_completion_receipt ENABLE ROW LEVEL SECURITY; ALTER TABLE public.live_demo_install_completion_receipt FORCE ROW LEVEL SECURITY;
CREATE POLICY base_course_install_broker_completion_receipt ON public.live_demo_install_completion_receipt FOR INSERT TO ple_base_course_install_broker WITH CHECK(true);
GRANT SELECT,INSERT,UPDATE ON public.live_demo_install_state,public.live_demo_install_recipe,public.ple_account,public.instructor_approval TO ple_base_course_install_broker;
GRANT INSERT ON public.live_demo_install_completion_receipt TO ple_base_course_install_broker;
GRANT USAGE,SELECT ON SEQUENCE public.ple_account_public_id_seq TO ple_base_course_install_broker;
GRANT EXECUTE ON FUNCTION public.ple_current_tenant() TO ple_base_course_install_broker;
-- ASVS 8.2.1-8.2.3, 15.4.2, 15.4.3: the sealed freshness broker has catalog-complete read/SHARE-lock authority; registration fails closed on drift.
DO $$
DECLARE
    relation record;
BEGIN
    FOR relation IN
        SELECT namespace.nspname, table_row.relname, table_row.relrowsecurity
          FROM pg_catalog.pg_class AS table_row
          JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace
         WHERE namespace.nspname='public'
           AND table_row.relkind IN ('r','p')
           AND table_row.relname<>'_sqlx_migrations'
         ORDER BY namespace.nspname,table_row.relname,table_row.oid
    LOOP
        EXECUTE format(
            'GRANT SELECT,MAINTAIN ON TABLE %I.%I TO ple_base_course_freshness_broker',
            relation.nspname,
            relation.relname
        );
        IF relation.relrowsecurity THEN
            EXECUTE format(
                'CREATE POLICY ple_base_course_freshness_select ON %I.%I FOR SELECT TO ple_base_course_freshness_broker USING (true)',
                relation.nspname,
                relation.relname
            );
        END IF;
    END LOOP;
END
$$;
-- ASVS 8.2.1-8.2.3, 13.2.2: final verification is a sealed SELECT-only capability over the exact versioned completion graph.
DO $$ DECLARE relation_name text; relation_row record; BEGIN FOREACH relation_name IN ARRAY ARRAY['ple_account','instructor_approval','tenant_learner_identity','course','course_roster_state','course_appearance','course_allowed_email_domain','course_group_membership_policy','course_grade_scheme','course_grade_category','course_grade_category_assignment','course_grade_letter_band','course_total_export_audit','course_member','course_roster_profile','course_group','course_group_member','problem','problem_version','problem_version_payload','catalog_tenant_grant','catalog_search_document','published_source_artifact','published_flat_import_origin','published_flat_import_choice_map','published_qti_grading','answer_key','workspace_draft','workspace_draft_access','workspace_flat_question_source','workspace_flat_question_grading','assignment','assignment_item','assignment_selection_group','assignment_selection_candidate','assignment_audience_group','assignment_effective_policy_base','assignment_group_schedule_offset','assignment_group_accommodation','assignment_individual_policy_exception','enrollment','enrollment_entitlement_basis_receipt','enrollment_applicable_policy_scope_receipt','student_assignment_summary','assignment_run','assignment_run_item','question_attempt','attempt_effective_policy_receipt','attempt_effective_policy_receipt_field_source','attempt_effective_policy_current','submission','submission_idempotency','submission_evaluation','attempt_feedback','attempt_score_current','submission_receipt_snapshot','submission_next_attempt','feedback_release','manual_grade_receipt','question_prefetch','question_statistics_contribution_receipt','question_statistics_aggregate'] LOOP SELECT c.relrowsecurity INTO relation_row FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relname=relation_name AND c.relkind IN('r','p'); IF NOT FOUND THEN RAISE EXCEPTION 'missing Base Course completion relation public.%',relation_name; END IF; EXECUTE format('GRANT SELECT ON TABLE public.%I TO ple_base_course_completion_verification_broker',relation_name); IF relation_row.relrowsecurity THEN EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY',relation_name); EXECUTE format('CREATE POLICY ple_base_course_completion_select ON public.%I FOR SELECT TO ple_base_course_completion_verification_broker USING(true)',relation_name); END IF; END LOOP; END $$;
CREATE FUNCTION public.ple_course_creation_deny_internal() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN RAISE EXCEPTION 'course creation is unavailable' USING ERRCODE='42501';
END $$;
CREATE FUNCTION public.ple_course_creation_validate_inputs(p_tenant uuid,p_course uuid,p_title text,p_start date,p_end date,p_zone text) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN
 IF p_tenant IS NULL OR p_course IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant() OR p_title IS NULL OR p_title<>btrim(p_title) OR char_length(p_title) NOT BETWEEN 1 AND 200 OR p_start IS NULL OR p_end IS NULL OR p_start>p_end OR p_zone IS NULL OR p_zone<>btrim(p_zone) OR char_length(p_zone) NOT BETWEEN 1 AND 255 OR p_zone~'[[:space:]]' THEN
PERFORM public.ple_course_creation_deny_internal();
END IF;
END $$;
CREATE FUNCTION public.ple_create_course_core_internal(p_tenant uuid,p_course uuid,p_title text,p_start date,p_end date,p_zone text,p_instructor uuid) RETURNS TABLE(course_id uuid,instructor_membership_id uuid) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE member uuid:=gen_random_uuid();
BEGIN
 PERFORM public.ple_course_creation_validate_inputs(p_tenant,p_course,p_title,p_start,p_end,p_zone);
IF p_instructor IS NULL THEN
PERFORM public.ple_course_creation_deny_internal();
END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text||':'||p_course::text,0));
PERFORM 1 FROM public.course c WHERE c.tenant_id=p_tenant AND c.course_id=p_course FOR UPDATE;
IF FOUND THEN
RAISE EXCEPTION 'course identity already exists' USING ERRCODE='23505';
END IF;
 INSERT INTO public.course(tenant_id,course_id,title,term_start_date,term_end_date,time_zone) VALUES(p_tenant,p_course,p_title,p_start,p_end,p_zone);
 INSERT INTO public.course_member(tenant_id,course_id,course_membership_id,user_id,role,student_id,status,joined_at) VALUES(p_tenant,p_course,member,p_instructor,'instructor',NULL,'active',transaction_timestamp());
 INSERT INTO public.course_roster_state(tenant_id,course_id) VALUES(p_tenant,p_course) ON CONFLICT ON CONSTRAINT course_roster_state_pkey DO NOTHING;
 SET CONSTRAINTS course_creates_group_membership_policies,course_creates_grade_scheme IMMEDIATE;
 IF NOT EXISTS(SELECT 1 FROM public.course_appearance a WHERE a.tenant_id=p_tenant AND a.course_id=p_course) OR (SELECT count(*) FROM public.course_group_membership_policy g WHERE g.tenant_id=p_tenant AND g.course_id=p_course)<>5 OR NOT EXISTS(SELECT 1 FROM public.course_grade_scheme s WHERE s.tenant_id=p_tenant AND s.course_id=p_course) THEN
RAISE EXCEPTION 'course bootstrap defaults are incomplete' USING ERRCODE='55000';
END IF;
-- ASVS 2.3.3: creation succeeds only after the strict revision-1 aggregate verifies in-transaction.
RETURN QUERY SELECT * FROM public.ple_verify_course_creation_aggregate_internal(p_tenant,p_course,p_title,p_start,p_end,p_zone,p_instructor);
END $$;
CREATE FUNCTION public.ple_verify_course_creation_aggregate_internal(p_tenant uuid,p_course uuid,p_title text,p_start date,p_end date,p_zone text,p_instructor uuid) RETURNS TABLE(course_id uuid,instructor_membership_id uuid) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM public.course co WHERE co.tenant_id=p_tenant AND co.course_id=p_course) THEN
RETURN;
END IF;
 SELECT cm.course_id,cm.course_membership_id INTO course_id,instructor_membership_id FROM public.course co JOIN public.course_member cm ON cm.tenant_id=co.tenant_id AND cm.course_id=co.course_id WHERE co.tenant_id=p_tenant AND co.course_id=p_course AND co.title=p_title AND co.term_start_date=p_start AND co.term_end_date=p_end AND co.time_zone=p_zone AND cm.user_id=p_instructor AND cm.role='instructor' AND cm.status='active';
 IF NOT FOUND
    OR (SELECT count(*) FROM public.course_member cm WHERE cm.tenant_id=p_tenant AND cm.course_id=p_course)<>1
    OR NOT EXISTS(SELECT 1 FROM public.course_roster_state rs WHERE rs.tenant_id=p_tenant AND rs.course_id=p_course AND rs.revision=1 AND rs.signup_posture='invitation_only')
    OR NOT EXISTS(SELECT 1 FROM public.course_appearance ca WHERE ca.tenant_id=p_tenant AND ca.course_id=p_course AND ca.theme_id='grass' AND ca.current_banner_delivery_id IS NULL AND ca.banner_alt_kind IS NULL AND ca.banner_alt_text IS NULL AND ca.revision=1)
    OR (SELECT count(*) FROM public.course_group_membership_policy gp WHERE gp.tenant_id=p_tenant AND gp.course_id=p_course AND (gp.purpose,gp.multiple_membership,gp.revision) IN (('section','warn',1),('lab','allow',1),('cohort','allow',1),('accommodation','allow',1),('work','allow',1)))<>5
    OR NOT EXISTS(SELECT 1 FROM public.course_grade_scheme gs WHERE gs.tenant_id=p_tenant AND gs.course_id=p_course AND gs.mode='total_points' AND gs.rounding='four_decimal_places_half_away_from_zero' AND gs.revision=1) THEN
RAISE EXCEPTION 'course creation aggregate conflicts' USING ERRCODE='55000';
END IF;
RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_create_course_as_instructor_v1(p_tenant uuid,p_course uuid,p_title text,p_start date,p_end date,p_zone text,p_actor uuid,p_session character(64)) RETURNS TABLE(course_id uuid,instructor_membership_id uuid) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE actor uuid;
roles jsonb;
BEGIN
 PERFORM public.ple_course_creation_validate_inputs(p_tenant,p_course,p_title,p_start,p_end,p_zone);
IF p_actor IS NULL OR p_session IS NULL THEN
PERFORM public.ple_course_creation_deny_internal();
END IF;
 SELECT s.user_id,s.roles INTO actor,roles FROM public.auth_session s WHERE s.session_hash=p_session AND s.tenant_id=p_tenant AND s.revoked_at IS NULL AND s.expires_at>transaction_timestamp() FOR UPDATE;
IF NOT FOUND OR actor IS DISTINCT FROM p_actor OR NOT roles@>'["instructor"]'::jsonb THEN
PERFORM public.ple_course_creation_deny_internal();
END IF;
 IF NOT public.ple_lock_instructor_approval_eligibility(p_actor) THEN
PERFORM public.ple_course_creation_deny_internal();
END IF;
RETURN QUERY SELECT * FROM public.ple_create_course_core_internal(p_tenant,p_course,p_title,p_start,p_end,p_zone,p_actor);
END $$;
CREATE FUNCTION public.ple_create_course_as_sysadmin_v1(p_tenant uuid,p_course uuid,p_title text,p_start date,p_end date,p_zone text,p_actor uuid,p_session character(64)) RETURNS TABLE(course_id uuid,instructor_membership_id uuid) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE actor uuid;
roles jsonb;
BEGIN
 PERFORM public.ple_course_creation_validate_inputs(p_tenant,p_course,p_title,p_start,p_end,p_zone);
IF p_actor IS NULL OR p_session IS NULL THEN
PERFORM public.ple_course_creation_deny_internal();
END IF;
 SELECT s.user_id,s.roles INTO actor,roles FROM public.auth_session s WHERE s.session_hash=p_session AND s.tenant_id=p_tenant AND s.revoked_at IS NULL AND s.expires_at>transaction_timestamp() FOR UPDATE;
IF NOT FOUND OR actor IS DISTINCT FROM p_actor OR NOT roles@>'["sysadmin"]'::jsonb THEN
PERFORM public.ple_course_creation_deny_internal();
END IF;
RETURN QUERY SELECT * FROM public.ple_create_course_core_internal(p_tenant,p_course,p_title,p_start,p_end,p_zone,p_actor);
END $$;
CREATE FUNCTION public.ple_upsert_course_student_as_instructor_v1(p_tenant uuid,p_actor uuid,p_course uuid,p_target_user uuid,p_candidate_student uuid,p_candidate_membership uuid,p_display_name text,p_email_normalized text,p_email_delivery text,p_roster_id text) RETURNS TABLE(tenant_id uuid,actor_id uuid,direct_instructor_membership_id uuid,course_id uuid,target_user_id uuid,student_id uuid,course_membership_id uuid,created boolean,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE v_course uuid;v_instructor uuid;v_instructor_count bigint;v_student uuid;v_membership uuid;v_role text;v_revision bigint;v_profile_count bigint;
BEGIN
 IF p_tenant IS NULL OR p_actor IS NULL OR p_course IS NULL OR p_target_user IS NULL OR p_candidate_student IS NULL OR p_candidate_membership IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant() OR p_display_name IS NULL OR p_display_name<>btrim(p_display_name) OR char_length(p_display_name) NOT BETWEEN 1 AND 200 OR ((p_email_normalized IS NULL OR p_email_delivery IS NULL OR p_roster_id IS NULL) AND (p_email_normalized IS NOT NULL OR p_email_delivery IS NOT NULL OR p_roster_id IS NOT NULL)) THEN RAISE EXCEPTION 'course roster mutation arguments are invalid' USING ERRCODE='22023'; END IF;
 IF p_email_normalized IS NOT NULL AND (p_email_normalized<>lower(p_email_normalized) OR p_email_normalized<>btrim(p_email_normalized) OR octet_length(p_email_normalized) NOT BETWEEN 3 AND 320 OR p_email_delivery<>btrim(p_email_delivery) OR octet_length(p_email_delivery) NOT BETWEEN 3 AND 320 OR p_roster_id!~'^[A-Za-z0-9._-]{1,64}$') THEN RAISE EXCEPTION 'course roster mutation arguments are invalid' USING ERRCODE='22023'; END IF;
 SELECT c.course_id INTO v_course FROM public.course c WHERE c.tenant_id=p_tenant AND c.course_id=p_course AND public.ple_course_records_accessible(c.tenant_id,c.course_id) FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'course roster mutation is unavailable' USING ERRCODE='42501'; END IF;
 SELECT rs.revision INTO v_revision FROM public.course_roster_state rs WHERE rs.tenant_id=p_tenant AND rs.course_id=p_course FOR UPDATE;
 IF NOT FOUND OR v_revision<1 THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF;
 PERFORM cm.course_membership_id FROM public.course_member cm WHERE cm.tenant_id=p_tenant AND cm.course_id=p_course AND cm.user_id=p_actor AND cm.role='instructor' AND cm.status='active' ORDER BY cm.course_membership_id FOR UPDATE;
 SELECT count(*),(array_agg(cm.course_membership_id ORDER BY cm.course_membership_id))[1] INTO v_instructor_count,v_instructor FROM public.course_member cm WHERE cm.tenant_id=p_tenant AND cm.course_id=p_course AND cm.user_id=p_actor AND cm.role='instructor' AND cm.status='active';
 IF v_instructor_count<>1 THEN RAISE EXCEPTION 'direct Instructor membership is required' USING ERRCODE='42501'; END IF;
 SELECT cm.course_membership_id,cm.role,cm.student_id INTO v_membership,v_role,v_student FROM public.course_member cm WHERE cm.tenant_id=p_tenant AND cm.course_id=p_course AND cm.user_id=p_target_user AND cm.status='active' FOR UPDATE;
 IF FOUND THEN
  IF v_role<>'student' OR v_student IS NULL THEN RAISE EXCEPTION 'active course membership conflicts' USING ERRCODE='55000'; END IF;
  SELECT count(*) INTO v_profile_count FROM public.course_roster_profile rp WHERE rp.tenant_id=p_tenant AND rp.course_id=p_course AND rp.course_membership_id=v_membership;
  IF v_profile_count<>1 OR NOT EXISTS(SELECT 1 FROM public.tenant_learner_identity li WHERE li.tenant_id=p_tenant AND li.user_id=p_target_user AND li.student_id=v_student) THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF;
  created:=false;
 ELSE
  INSERT INTO public.tenant_learner_identity(tenant_id,user_id,student_id) VALUES(p_tenant,p_target_user,p_candidate_student) ON CONFLICT ON CONSTRAINT tenant_learner_identity_pkey DO NOTHING;
  SELECT li.student_id INTO v_student FROM public.tenant_learner_identity li WHERE li.tenant_id=p_tenant AND li.user_id=p_target_user;
  IF v_student IS NULL THEN RAISE EXCEPTION 'learner identity is unavailable' USING ERRCODE='55000'; END IF;
  v_membership:=p_candidate_membership;
  INSERT INTO public.course_member(tenant_id,course_id,course_membership_id,user_id,role,student_id,roster_id,status,joined_at) VALUES(p_tenant,p_course,v_membership,p_target_user,'student',v_student,p_roster_id,'active',transaction_timestamp());
  INSERT INTO public.course_roster_profile(tenant_id,course_id,course_membership_id,display_name,roster_email_normalized,roster_email_delivery) VALUES(p_tenant,p_course,v_membership,p_display_name,p_email_normalized,p_email_delivery);
  UPDATE public.course_roster_state rs SET revision=rs.revision+1,updated_at=transaction_timestamp() WHERE rs.tenant_id=p_tenant AND rs.course_id=p_course RETURNING rs.revision INTO v_revision;
  created:=true;
 END IF;
 tenant_id:=p_tenant;actor_id:=p_actor;direct_instructor_membership_id:=v_instructor;course_id:=p_course;target_user_id:=p_target_user;student_id:=v_student;course_membership_id:=v_membership;roster_revision:=v_revision;RETURN NEXT;
END $$;
-- ASVS 2.2.1-2.2.3, 2.3.1-2.3.4, 8.2.1-8.2.3, 15.4.2-15.4.3: locked atomic roster revocation.
CREATE FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(p_tenant uuid,p_session character(64),p_course uuid,p_member uuid,p_expected_revision bigint) RETURNS TABLE(tenant_id uuid,actor_id uuid,course_id uuid,course_membership_id uuid,was_revoked boolean,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE v_course uuid;v_revision bigint;v_member uuid;v_role text;v_status text;v_actor uuid;
BEGIN
 IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL OR p_member IS NULL OR p_expected_revision IS NULL OR p_tenant='00000000-0000-0000-0000-000000000000'::uuid OR p_course='00000000-0000-0000-0000-000000000000'::uuid OR p_member='00000000-0000-0000-0000-000000000000'::uuid OR p_tenant IS DISTINCT FROM public.ple_current_tenant() OR p_expected_revision<1 THEN RAISE EXCEPTION 'course roster revocation arguments are invalid' USING ERRCODE='22023'; END IF;
 SELECT c.course_id INTO v_course FROM public.course c WHERE c.tenant_id=p_tenant AND c.course_id=p_course AND public.ple_course_records_accessible(c.tenant_id,c.course_id) FOR UPDATE;
 IF NOT FOUND THEN RAISE EXCEPTION 'course roster revocation is unavailable' USING ERRCODE='42501'; END IF;
 SELECT rs.revision INTO v_revision FROM public.course_roster_state rs WHERE rs.tenant_id=p_tenant AND rs.course_id=p_course FOR UPDATE;
 IF NOT FOUND OR v_revision<1 OR v_revision<>p_expected_revision THEN RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE='55000'; END IF;
 SELECT cm.course_membership_id,cm.role,cm.status INTO v_member,v_role,v_status FROM public.course_member cm WHERE cm.tenant_id=p_tenant AND cm.course_id=p_course AND cm.course_membership_id=p_member FOR UPDATE;
 IF NOT FOUND THEN RETURN; END IF;
 IF v_role<>'student' THEN RAISE EXCEPTION 'course roster membership is invalid' USING ERRCODE='55000'; END IF;
 v_actor:=public.ple_course_roster_support_actor(p_session,p_course,'revokeMember');
 IF v_actor IS NULL THEN RAISE EXCEPTION 'course roster revocation actor is unavailable' USING ERRCODE='42501'; END IF;
 tenant_id:=p_tenant;actor_id:=v_actor;course_id:=p_course;course_membership_id:=v_member;
 IF v_status='revoked' THEN was_revoked:=true;roster_revision:=v_revision;RETURN NEXT;RETURN; END IF;
 UPDATE public.course_member SET status='revoked',revoked_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=p_course AND course_membership_id=v_member AND role='student' AND status='active';
 IF NOT FOUND THEN RAISE EXCEPTION 'course roster membership is invalid' USING ERRCODE='55000'; END IF;
 UPDATE public.course_roster_state SET revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=p_course AND revision=v_revision RETURNING revision INTO roster_revision;
 IF NOT FOUND OR roster_revision<1 THEN RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE='55000'; END IF;
 was_revoked:=false;RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_require_base_course_install_lock_internal() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND pid=pg_backend_pid() AND granted AND classid=70463 AND objid=1818 AND objsubid=2) THEN
RAISE EXCEPTION 'Base Course installation lock is unavailable' USING ERRCODE='42501';
END IF;
END $$;
CREATE FUNCTION public.ple_base_course_install_validate_recipe_internal(p_recipe jsonb) RETURNS text LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE p jsonb;c jsonb;b jsonb;g jsonb;graph jsonb;e uuid;m uuid;j uuid;a uuid;o uuid;bi uuid;gi uuid;graph_ids uuid[];
BEGIN
 IF p_recipe IS NULL OR jsonb_typeof(p_recipe)<>'object' OR jsonb_typeof(p_recipe->'schemaVersion')<>'number' OR (SELECT array_agg(key ORDER BY key) FROM jsonb_object_keys(p_recipe) key) IS DISTINCT FROM ARRAY['courses','graph','participants','schemaVersion'] OR (p_recipe->>'schemaVersion')::numeric<>1 THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END IF;
 p:=p_recipe->'participants';c:=p_recipe->'courses';graph:=p_recipe->'graph';
IF jsonb_typeof(p)<>'object' OR jsonb_typeof(c)<>'object' OR jsonb_typeof(graph)<>'object' OR (SELECT array_agg(key ORDER BY key) FROM jsonb_object_keys(p) key) IS DISTINCT FROM ARRAY['avery','elena','jack','mary','morgan'] OR (SELECT array_agg(key ORDER BY key) FROM jsonb_object_keys(c) key) IS DISTINCT FROM ARRAY['baseCourse','geneticsPractice'] OR (SELECT array_agg(key ORDER BY key) FROM jsonb_object_keys(graph) key) IS DISTINCT FROM ARRAY['assignment','assignmentItem','jackAttempt','jackRun','maryAttempt','maryRun','problem','version','workspace'] OR EXISTS(SELECT 1 FROM jsonb_each(graph) field WHERE jsonb_typeof(field.value)<>'string') THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END IF;
IF jsonb_typeof(p->'elena')<>'string' OR jsonb_typeof(p->'mary')<>'string'
   OR jsonb_typeof(p->'jack')<>'string' OR jsonb_typeof(p->'avery')<>'string'
   OR jsonb_typeof(p->'morgan')<>'string' THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END IF;
BEGIN
e:=(p->>'elena')::uuid;m:=(p->>'mary')::uuid;j:=(p->>'jack')::uuid;a:=(p->>'avery')::uuid;o:=(p->>'morgan')::uuid;graph_ids:=ARRAY[(graph->>'workspace')::uuid,(graph->>'problem')::uuid,(graph->>'version')::uuid,(graph->>'assignment')::uuid,(graph->>'assignmentItem')::uuid,(graph->>'maryRun')::uuid,(graph->>'maryAttempt')::uuid,(graph->>'jackRun')::uuid,(graph->>'jackAttempt')::uuid];
EXCEPTION WHEN invalid_text_representation THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END;
IF cardinality(ARRAY(SELECT DISTINCT x FROM unnest(ARRAY[e,m,j,a,o]) x))<>5 OR cardinality(ARRAY(SELECT DISTINCT x FROM unnest(graph_ids) x))<>9 THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END IF;
 b:=c->'baseCourse';g:=c->'geneticsPractice';
IF jsonb_typeof(b)<>'object' OR jsonb_typeof(g)<>'object' OR (SELECT array_agg(key ORDER BY key) FROM jsonb_object_keys(b) key) IS DISTINCT FROM ARRAY['id','initialInstructor','termEnd','termStart','timeZone','title'] OR (SELECT array_agg(key ORDER BY key) FROM jsonb_object_keys(g) key) IS DISTINCT FROM ARRAY['id','initialInstructor','termEnd','termStart','timeZone','title'] THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END IF;
IF EXISTS (SELECT 1 FROM jsonb_each(b) AS field WHERE jsonb_typeof(field.value)<>'string')
   OR EXISTS (SELECT 1 FROM jsonb_each(g) AS field WHERE jsonb_typeof(field.value)<>'string') THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END IF;
BEGIN
bi:=(b->>'id')::uuid;gi:=(g->>'id')::uuid;
EXCEPTION WHEN invalid_text_representation THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END;
IF bi IS NULL OR gi IS NULL OR bi=gi OR bi=ANY(ARRAY[e,m,j,a,o]) OR gi=ANY(ARRAY[e,m,j,a,o]) OR b->>'title'<>'Biochemistry Base Course' OR g->>'title'<>'Genetics Practice Course' OR b->>'termStart'<>'2026-01-01' OR g->>'termStart'<>'2026-01-01' OR b->>'termEnd'<>'2099-12-31' OR g->>'termEnd'<>'2099-12-31' OR b->>'timeZone'<>'America/Chicago' OR g->>'timeZone'<>'America/Chicago' OR b->>'initialInstructor' IS DISTINCT FROM e::text OR g->>'initialInstructor' IS DISTINCT FROM o::text THEN
RAISE EXCEPTION 'Base Course recipe is invalid' USING ERRCODE='22023';
END IF;
 RETURN encode(digest(convert_to(p_recipe::text,'UTF8'),'sha256'),'hex');
END $$;
CREATE FUNCTION public.ple_base_course_install_acquire_lock_v1() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN PERFORM pg_advisory_lock(70463,1818);
END $$;
CREATE FUNCTION public.ple_base_course_install_read_v2() RETURNS TABLE(state text,tenant_id uuid,baseline_version text,installation_generation uuid,object_manifest jsonb,storage_receipt_sha256 text,completion_receipt_sha256 text,recipe_sha256 text) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN PERFORM public.ple_require_base_course_install_lock_internal();
RETURN QUERY SELECT i.state,i.tenant_id,i.baseline_version,i.installation_generation,i.object_manifest,i.storage_receipt_sha256,i.completion_receipt_sha256,r.recipe_sha256 FROM public.live_demo_install_state i LEFT JOIN public.live_demo_install_recipe r ON r.singleton AND r.installation_generation=i.installation_generation WHERE i.singleton;
END $$;
CREATE FUNCTION public.ple_require_fresh_base_course_install_internal() RETURNS TABLE(failure_kind text,relation_name text) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE
 relation_oids oid[];
 relation_oid oid;
 relation_schema name;
 relation_table name;
 namespace_rows bigint;
 unconsumed_namespace_rows bigint;
 relation_has_rows boolean;
BEGIN
 -- ASVS 1.2.4: dynamic identifiers come only from pg_catalog and are quoted by format(%I).
 LOCK TABLE ONLY public.question_id_namespace IN SHARE MODE;
 SELECT count(*),count(*) FILTER(WHERE singleton AND issued_count=0)
   INTO namespace_rows,unconsumed_namespace_rows
   FROM public.question_id_namespace;
 IF (namespace_rows,unconsumed_namespace_rows) IS DISTINCT FROM (1::bigint,1::bigint) THEN
failure_kind:='unconsumed_question_namespace';relation_name:=NULL;RETURN NEXT;RETURN;
END IF;
 SELECT array_agg(table_row.oid ORDER BY namespace.nspname,table_row.relname,table_row.oid) INTO relation_oids FROM pg_catalog.pg_class AS table_row JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace WHERE namespace.nspname='public' AND table_row.relkind IN ('r','p') AND table_row.relname NOT IN ('_sqlx_migrations','question_id_namespace');
 FOREACH relation_oid IN ARRAY COALESCE(relation_oids,ARRAY[]::oid[]) LOOP SELECT namespace.nspname,table_row.relname INTO STRICT relation_schema,relation_table FROM pg_catalog.pg_class AS table_row JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace WHERE table_row.oid=relation_oid; EXECUTE format('LOCK TABLE ONLY %I.%I IN SHARE MODE',relation_schema,relation_table); END LOOP;
 FOREACH relation_oid IN ARRAY COALESCE(relation_oids,ARRAY[]::oid[]) LOOP SELECT namespace.nspname,table_row.relname INTO STRICT relation_schema,relation_table FROM pg_catalog.pg_class AS table_row JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace WHERE table_row.oid=relation_oid;
EXECUTE format('SELECT EXISTS(SELECT 1 FROM ONLY %I.%I LIMIT 1)',relation_schema,relation_table) INTO relation_has_rows;
IF relation_has_rows THEN
failure_kind:='nonempty_application_relation';relation_name:=relation_schema::text||'.'||relation_table::text;RETURN NEXT;RETURN;
END IF;
END LOOP;
 failure_kind:=NULL;relation_name:=NULL;RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_base_course_install_prepare_v2(p_tenant uuid,p_baseline text,p_manifest jsonb,p_recipe jsonb) RETURNS TABLE(state text,installation_generation uuid,recipe_sha256 text,freshness_failure_kind text,freshness_relation_name text) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE d text;i public.live_demo_install_state%ROWTYPE;gen uuid;freshness record;
BEGIN
 PERFORM public.ple_require_base_course_install_lock_internal();
IF p_tenant IS NULL OR p_baseline<>'base-course-v1' OR p_manifest<>'[]'::jsonb THEN
RAISE EXCEPTION 'Base Course installation request is invalid' USING ERRCODE='22023';
END IF;
d:=public.ple_base_course_install_validate_recipe_internal(p_recipe);
SELECT * INTO i FROM public.live_demo_install_state WHERE singleton FOR UPDATE;
 IF FOUND THEN
IF i.tenant_id IS DISTINCT FROM p_tenant OR i.baseline_version<>p_baseline OR i.object_manifest<>p_manifest THEN
RAISE EXCEPTION 'Base Course installation conflicts with retained state' USING ERRCODE='23505';
END IF;
PERFORM 1 FROM public.live_demo_install_recipe r WHERE r.singleton AND r.installation_generation=i.installation_generation AND r.tenant_id=p_tenant AND r.baseline_version=p_baseline AND r.recipe=p_recipe AND r.recipe_sha256=d FOR UPDATE;
IF NOT FOUND THEN
RAISE EXCEPTION 'Base Course installation recipe conflicts' USING ERRCODE='23505';
END IF;
RETURN QUERY SELECT i.state,i.installation_generation,d,NULL::text,NULL::text;
RETURN;
END IF;
 SELECT * INTO STRICT freshness FROM public.ple_require_fresh_base_course_install_internal();
 IF freshness.failure_kind IS NOT NULL THEN
RETURN QUERY SELECT NULL::text,NULL::uuid,NULL::text,freshness.failure_kind,freshness.relation_name;
RETURN;
END IF;
 gen:=gen_random_uuid();
INSERT INTO public.live_demo_install_state(singleton,state,baseline_version,tenant_id,installation_generation,object_manifest) VALUES(true,'installing',p_baseline,p_tenant,gen,p_manifest);
INSERT INTO public.live_demo_install_recipe(singleton,installation_generation,tenant_id,baseline_version,recipe,recipe_sha256) VALUES(true,gen,p_tenant,p_baseline,p_recipe,d);
RETURN QUERY SELECT 'installing'::text,gen,d,NULL::text,NULL::text;
END $$;
CREATE FUNCTION public.ple_base_course_install_seed_accounts_v2(p_generation uuid) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE r jsonb;e uuid;m uuid;j uuid;a uuid;o uuid;
BEGIN
 PERFORM public.ple_require_base_course_install_lock_internal();
SELECT x.recipe INTO r FROM public.live_demo_install_state i JOIN public.live_demo_install_recipe x ON x.singleton AND x.installation_generation=i.installation_generation WHERE i.singleton AND i.state='installing' AND i.installation_generation=p_generation FOR UPDATE OF i,x;
IF NOT FOUND THEN
RAISE EXCEPTION 'Base Course installation is unavailable' USING ERRCODE='42501';
END IF;
 e:=(r#>>'{participants,elena}')::uuid;m:=(r#>>'{participants,mary}')::uuid;j:=(r#>>'{participants,jack}')::uuid;a:=(r#>>'{participants,avery}')::uuid;o:=(r#>>'{participants,morgan}')::uuid;
 INSERT INTO public.ple_account(user_id,normalized_email,delivery_email,display_name,platform_roles) VALUES(e,'elena.rivera@live-demo.ple.example','elena.rivera@live-demo.ple.example','Dr. Elena Rivera','[]'),(m,'mary.okafor@live-demo.ple.example','mary.okafor@live-demo.ple.example','Mary Okafor','[]'),(j,'jack.chen@live-demo.ple.example','jack.chen@live-demo.ple.example','Jack Chen','[]'),(a,'avery.singh@live-demo.ple.example','avery.singh@live-demo.ple.example','Avery Singh','[]'),(o,'morgan.reyes@live-demo.ple.example','morgan.reyes@live-demo.ple.example','Morgan Reyes','["sysadmin"]') ON CONFLICT(user_id) DO NOTHING;
 PERFORM 1 FROM public.ple_account q WHERE q.user_id=o AND q.normalized_email='morgan.reyes@live-demo.ple.example' AND q.delivery_email='morgan.reyes@live-demo.ple.example' AND q.display_name='Morgan Reyes' AND q.platform_roles='["sysadmin"]' FOR UPDATE;
IF NOT FOUND OR EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id IN(e,m,j,a) AND q.platform_roles<>'[]'::jsonb) THEN
RAISE EXCEPTION 'Base Course account recipe conflicts' USING ERRCODE='23505';
END IF;
IF NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=e AND q.normalized_email='elena.rivera@live-demo.ple.example' AND q.delivery_email='elena.rivera@live-demo.ple.example' AND q.display_name='Dr. Elena Rivera' AND q.platform_roles='[]'::jsonb)
   OR NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=m AND q.normalized_email='mary.okafor@live-demo.ple.example' AND q.delivery_email='mary.okafor@live-demo.ple.example' AND q.display_name='Mary Okafor' AND q.platform_roles='[]'::jsonb)
   OR NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=j AND q.normalized_email='jack.chen@live-demo.ple.example' AND q.delivery_email='jack.chen@live-demo.ple.example' AND q.display_name='Jack Chen' AND q.platform_roles='[]'::jsonb)
   OR NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=a AND q.normalized_email='avery.singh@live-demo.ple.example' AND q.delivery_email='avery.singh@live-demo.ple.example' AND q.display_name='Avery Singh' AND q.platform_roles='[]'::jsonb) THEN
RAISE EXCEPTION 'Base Course account recipe conflicts' USING ERRCODE='23505';
END IF;
 INSERT INTO public.instructor_approval(user_id,approved_by,approved_at,revoked_at,revision) VALUES(e,o,transaction_timestamp(),NULL,1) ON CONFLICT(user_id) DO NOTHING;
PERFORM 1 FROM public.instructor_approval q WHERE q.user_id=e AND q.approved_by=o AND q.revoked_at IS NULL FOR UPDATE;
IF NOT FOUND OR EXISTS(SELECT 1 FROM public.instructor_approval q WHERE q.user_id IN(m,j,a,o) AND q.revoked_at IS NULL) THEN
RAISE EXCEPTION 'Base Course approval recipe conflicts' USING ERRCODE='23505';
END IF;
END $$;
CREATE FUNCTION public.ple_verify_base_course_accounts_internal(p_recipe jsonb) RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE e uuid:=(p_recipe#>>'{participants,elena}')::uuid; m uuid:=(p_recipe#>>'{participants,mary}')::uuid; j uuid:=(p_recipe#>>'{participants,jack}')::uuid; a uuid:=(p_recipe#>>'{participants,avery}')::uuid; o uuid:=(p_recipe#>>'{participants,morgan}')::uuid;
BEGIN
 IF NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=e AND q.normalized_email='elena.rivera@live-demo.ple.example' AND q.delivery_email='elena.rivera@live-demo.ple.example' AND q.display_name='Dr. Elena Rivera' AND q.platform_roles='[]'::jsonb)
 OR NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=m AND q.normalized_email='mary.okafor@live-demo.ple.example' AND q.delivery_email='mary.okafor@live-demo.ple.example' AND q.display_name='Mary Okafor' AND q.platform_roles='[]'::jsonb)
 OR NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=j AND q.normalized_email='jack.chen@live-demo.ple.example' AND q.delivery_email='jack.chen@live-demo.ple.example' AND q.display_name='Jack Chen' AND q.platform_roles='[]'::jsonb)
 OR NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=a AND q.normalized_email='avery.singh@live-demo.ple.example' AND q.delivery_email='avery.singh@live-demo.ple.example' AND q.display_name='Avery Singh' AND q.platform_roles='[]'::jsonb)
 OR NOT EXISTS(SELECT 1 FROM public.ple_account q WHERE q.user_id=o AND q.normalized_email='morgan.reyes@live-demo.ple.example' AND q.delivery_email='morgan.reyes@live-demo.ple.example' AND q.display_name='Morgan Reyes' AND q.platform_roles='["sysadmin"]'::jsonb)
 OR NOT EXISTS(SELECT 1 FROM public.instructor_approval q WHERE q.user_id=e AND q.approved_by=o AND q.revoked_at IS NULL)
 OR EXISTS(SELECT 1 FROM public.instructor_approval q WHERE q.user_id IN(m,j,a,o) AND q.revoked_at IS NULL) THEN
RAISE EXCEPTION 'Base Course account recipe conflicts' USING ERRCODE='23505'; END IF;
END $$;
CREATE FUNCTION public.ple_verify_base_course_course_prefix_internal(p_recipe jsonb,p_slot text) RETURNS TABLE(prefix_state text,course_id uuid,instructor_membership_id uuid) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE t uuid:=public.ple_current_tenant();c jsonb;u uuid;cid uuid;aid uuid:=(p_recipe#>>'{graph,assignment}')::uuid;student_users uuid[];student_names text[];student_count integer;scheme_revision integer;instructor_id uuid;actual_members bigint;actual_profiles bigint;assignment_count bigint;
BEGIN
 IF p_slot NOT IN('base_course','genetics_practice') THEN RAISE EXCEPTION 'Base Course slot is invalid' USING ERRCODE='22023'; END IF;
 IF t IS NULL THEN RAISE EXCEPTION 'Base Course tenant is unavailable' USING ERRCODE='42501'; END IF;
 IF p_slot='base_course' THEN c:=p_recipe#>'{courses,baseCourse}';u:=(p_recipe#>>'{participants,elena}')::uuid;cid:=(c->>'id')::uuid;student_users:=ARRAY[(p_recipe#>>'{participants,mary}')::uuid,(p_recipe#>>'{participants,jack}')::uuid];student_names:=ARRAY['Mary Okafor','Jack Chen']; ELSE c:=p_recipe#>'{courses,geneticsPractice}';u:=(p_recipe#>>'{participants,morgan}')::uuid;cid:=(c->>'id')::uuid;student_users:=ARRAY[(p_recipe#>>'{participants,avery}')::uuid];student_names:=ARRAY['Avery Singh']; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(t::text||':'||cid::text,0)); PERFORM 1 FROM public.course co WHERE co.tenant_id=t AND co.course_id=cid FOR SHARE; IF NOT FOUND THEN prefix_state:='absent';course_id:=NULL;instructor_membership_id:=NULL;RETURN NEXT;RETURN; END IF; PERFORM 1 FROM public.course_roster_state rs WHERE rs.tenant_id=t AND rs.course_id=cid FOR SHARE; PERFORM 1 FROM public.course_member cm WHERE cm.tenant_id=t AND cm.course_id=cid FOR SHARE;
 SELECT cm.course_membership_id INTO instructor_id FROM public.course_member cm WHERE cm.tenant_id=t AND cm.course_id=cid AND cm.user_id=u AND cm.role='instructor' AND cm.student_id IS NULL AND cm.status='active' AND cm.roster_id IS NULL AND cm.revoked_at IS NULL;
 SELECT count(*) INTO actual_members FROM public.course_member cm WHERE cm.tenant_id=t AND cm.course_id=cid; SELECT count(*) INTO actual_profiles FROM public.course_roster_profile rp WHERE rp.tenant_id=t AND rp.course_id=cid;
 SELECT count(*) INTO student_count FROM unnest(student_users,student_names) expected(user_id,display_name) WHERE EXISTS(SELECT 1 FROM public.course_member cm JOIN public.tenant_learner_identity li ON li.tenant_id=cm.tenant_id AND li.user_id=cm.user_id AND li.student_id=cm.student_id JOIN public.course_roster_profile rp ON rp.tenant_id=cm.tenant_id AND rp.course_id=cm.course_id AND rp.course_membership_id=cm.course_membership_id WHERE cm.tenant_id=t AND cm.course_id=cid AND cm.user_id=expected.user_id AND cm.role='student' AND cm.student_id IS NOT NULL AND cm.status='active' AND cm.roster_id IS NULL AND cm.revoked_at IS NULL AND rp.display_name=expected.display_name AND rp.roster_email_normalized IS NULL AND rp.roster_email_delivery IS NULL);
 IF p_slot='base_course' AND student_count=1 AND NOT EXISTS(SELECT 1 FROM public.course_member cm WHERE cm.tenant_id=t AND cm.course_id=cid AND cm.user_id=student_users[1] AND cm.role='student' AND cm.status='active') THEN student_count:=-1; END IF;
 SELECT count(*) INTO assignment_count FROM public.assignment x WHERE x.tenant_id=t AND x.course_id=cid; IF p_slot='base_course' THEN IF assignment_count=0 THEN scheme_revision:=1; ELSIF assignment_count=1 AND EXISTS(SELECT 1 FROM public.assignment x WHERE x.tenant_id=t AND x.course_id=cid AND x.assignment_id=aid) THEN scheme_revision:=2; ELSE scheme_revision:=NULL; END IF; ELSE IF assignment_count=0 THEN scheme_revision:=1; ELSE scheme_revision:=NULL; END IF; END IF;
 IF instructor_id IS NULL OR scheme_revision IS NULL OR actual_members<>1+student_count OR actual_profiles<>student_count OR NOT EXISTS(SELECT 1 FROM public.course co WHERE co.tenant_id=t AND co.course_id=cid AND co.title=c->>'title' AND co.term_start_date=(c->>'termStart')::date AND co.term_end_date=(c->>'termEnd')::date AND co.time_zone=c->>'timeZone') OR NOT EXISTS(SELECT 1 FROM public.course_roster_state rs WHERE rs.tenant_id=t AND rs.course_id=cid AND rs.revision=1+student_count AND rs.signup_posture='invitation_only') OR NOT EXISTS(SELECT 1 FROM public.course_appearance ca WHERE ca.tenant_id=t AND ca.course_id=cid AND ca.theme_id='grass' AND ca.current_banner_delivery_id IS NULL AND ca.banner_alt_kind IS NULL AND ca.banner_alt_text IS NULL AND ca.revision=1) OR (SELECT count(*) FROM public.course_group_membership_policy gp WHERE gp.tenant_id=t AND gp.course_id=cid AND (gp.purpose,gp.multiple_membership,gp.revision) IN (('section','warn',1),('lab','allow',1),('cohort','allow',1),('accommodation','allow',1),('work','allow',1)))<>5 OR NOT EXISTS(SELECT 1 FROM public.course_grade_scheme gs WHERE gs.tenant_id=t AND gs.course_id=cid AND gs.mode='total_points' AND gs.rounding='four_decimal_places_half_away_from_zero' AND gs.revision=scheme_revision) OR EXISTS(SELECT 1 FROM public.course_allowed_email_domain d WHERE d.tenant_id=t AND d.course_id=cid) OR EXISTS(SELECT 1 FROM public.course_group cg WHERE cg.tenant_id=t AND cg.course_id=cid) OR EXISTS(SELECT 1 FROM public.course_group_member gm WHERE gm.tenant_id=t AND gm.course_id=cid) OR EXISTS(SELECT 1 FROM public.course_grade_category gc WHERE gc.tenant_id=t AND gc.course_id=cid) OR EXISTS(SELECT 1 FROM public.course_grade_category_assignment ga WHERE ga.tenant_id=t AND ga.course_id=cid) OR EXISTS(SELECT 1 FROM public.course_grade_letter_band gl WHERE gl.tenant_id=t AND gl.course_id=cid) THEN prefix_state:='conflict';course_id:=NULL;instructor_membership_id:=NULL;RETURN NEXT;RETURN; END IF;
 prefix_state:='exact_prefix';course_id:=cid;instructor_membership_id:=instructor_id;RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_base_course_install_seed_course_v2(p_generation uuid,p_slot text) RETURNS TABLE(seed_outcome text,course_id uuid,instructor_membership_id uuid,failure_kind text) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE r jsonb;t uuid;c jsonb;u uuid;cid uuid;prefix record;
BEGIN
 PERFORM public.ple_require_base_course_install_lock_internal();
IF p_slot NOT IN('base_course','genetics_practice') THEN
RAISE EXCEPTION 'Base Course slot is invalid' USING ERRCODE='22023';
END IF;
SELECT x.recipe,i.tenant_id INTO r,t FROM public.live_demo_install_state i JOIN public.live_demo_install_recipe x ON x.singleton AND x.installation_generation=i.installation_generation WHERE i.singleton AND i.state='installing' AND i.installation_generation=p_generation FOR UPDATE OF i,x;
IF NOT FOUND THEN
RAISE EXCEPTION 'Base Course installation is unavailable' USING ERRCODE='42501';
END IF;
 IF p_slot='base_course' THEN
c:=r#>'{courses,baseCourse}';u:=(r#>>'{participants,elena}')::uuid;
ELSE c:=r#>'{courses,geneticsPractice}';u:=(r#>>'{participants,morgan}')::uuid;
END IF;
cid:=(c->>'id')::uuid;
PERFORM public.ple_verify_base_course_accounts_internal(r);
 PERFORM set_config('ple.tenant_id',t::text,true); SELECT * INTO STRICT prefix FROM public.ple_verify_base_course_course_prefix_internal(r,p_slot);
 IF prefix.prefix_state='exact_prefix' THEN seed_outcome:='exact_prefix';course_id:=prefix.course_id;instructor_membership_id:=prefix.instructor_membership_id;failure_kind:=NULL;RETURN NEXT;RETURN; END IF;
 IF prefix.prefix_state='conflict' THEN seed_outcome:='refused';course_id:=NULL;instructor_membership_id:=NULL;failure_kind:='course_aggregate_conflict';RETURN NEXT;RETURN; END IF;
 SELECT created.course_id,created.instructor_membership_id INTO course_id,instructor_membership_id FROM public.ple_create_course_core_internal(t,cid,c->>'title',(c->>'termStart')::date,(c->>'termEnd')::date,c->>'timeZone',u) created; seed_outcome:='created';failure_kind:=NULL;RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_verify_base_course_completion_internal(p_tenant uuid,p_generation uuid,p_recipe_sha256 text,p_recipe jsonb) RETURNS TABLE(failure_kind text,canonical_receipt jsonb,receipt_sha256 text) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE g jsonb:=p_recipe->'graph';e uuid:=(p_recipe#>>'{participants,elena}')::uuid;m uuid:=(p_recipe#>>'{participants,mary}')::uuid;j uuid:=(p_recipe#>>'{participants,jack}')::uuid;a uuid:=(p_recipe#>>'{participants,avery}')::uuid;o uuid:=(p_recipe#>>'{participants,morgan}')::uuid;bc uuid:=(p_recipe#>>'{courses,baseCourse,id}')::uuid;pc uuid:=(p_recipe#>>'{courses,geneticsPractice,id}')::uuid;wid uuid:=(g->>'workspace')::uuid;pid uuid:=(g->>'problem')::uuid;vid uuid:=(g->>'version')::uuid;aid uuid:=(g->>'assignment')::uuid;item uuid:=(g->>'assignmentItem')::uuid;mr uuid:=(g->>'maryRun')::uuid;ma uuid:=(g->>'maryAttempt')::uuid;jr uuid:=(g->>'jackRun')::uuid;ja uuid:=(g->>'jackAttempt')::uuid;bi uuid;mm uuid;ms uuid;jm uuid;js uuid;pi uuid;am uuid;as_id uuid;me uuid;je uuid;sub uuid;qid text;content_hash text;payload_hash text;mrh text;jrh text;mah text;jah text;maph text;japh text;magh text;jagh text;subh text;idem_request text;idem_payload text;evalh text;feedbackh text;snap_runh text;snap_summaryh text;snap_presentationh text;mary_basis text;jack_basis text;mary_summary text;jack_summary text;receipt jsonb;
BEGIN
 SELECT course_membership_id INTO bi FROM public.course_member WHERE tenant_id=p_tenant AND course_id=bc AND user_id=e AND role='instructor' AND status='active'; SELECT course_membership_id,student_id INTO mm,ms FROM public.course_member WHERE tenant_id=p_tenant AND course_id=bc AND user_id=m AND role='student' AND status='active'; SELECT course_membership_id,student_id INTO jm,js FROM public.course_member WHERE tenant_id=p_tenant AND course_id=bc AND user_id=j AND role='student' AND status='active'; SELECT course_membership_id INTO pi FROM public.course_member WHERE tenant_id=p_tenant AND course_id=pc AND user_id=o AND role='instructor' AND status='active'; SELECT course_membership_id,student_id INTO am,as_id FROM public.course_member WHERE tenant_id=p_tenant AND course_id=pc AND user_id=a AND role='student' AND status='active';
 SELECT enrollment_id INTO me FROM public.enrollment WHERE tenant_id=p_tenant AND assignment_id=aid AND user_id=m; SELECT enrollment_id INTO je FROM public.enrollment WHERE tenant_id=p_tenant AND assignment_id=aid AND user_id=j; SELECT question_id INTO qid FROM public.problem WHERE problem_id=pid; SELECT pv.content_sha256,pvp.payload_sha256 INTO content_hash,payload_hash FROM public.problem_version pv JOIN public.problem_version_payload pvp USING(problem_id,version_id) WHERE pv.problem_id=pid AND pv.version_id=vid; SELECT payload_sha256 INTO mrh FROM public.assignment_run WHERE tenant_id=p_tenant AND run_id=mr; SELECT payload_sha256 INTO jrh FROM public.assignment_run WHERE tenant_id=p_tenant AND run_id=jr; SELECT payload_sha256,presentation_payload_sha256,grading_envelope_payload_sha256 INTO mah,maph,magh FROM public.question_attempt WHERE tenant_id=p_tenant AND attempt_id=ma; SELECT payload_sha256,presentation_payload_sha256,grading_envelope_payload_sha256 INTO jah,japh,jagh FROM public.question_attempt WHERE tenant_id=p_tenant AND attempt_id=ja; SELECT submission_id,payload_sha256 INTO sub,subh FROM public.submission WHERE tenant_id=p_tenant AND attempt_id=ma; SELECT request_sha256,payload_sha256 INTO idem_request,idem_payload FROM public.submission_idempotency WHERE tenant_id=p_tenant AND attempt_id=ma; SELECT payload_sha256 INTO evalh FROM public.submission_evaluation WHERE tenant_id=p_tenant AND attempt_id=ma; SELECT content_sha256 INTO feedbackh FROM public.attempt_feedback WHERE tenant_id=p_tenant AND attempt_id=ma; SELECT run_payload_sha256,summary_payload_sha256,presentation_payload_sha256 INTO snap_runh,snap_summaryh,snap_presentationh FROM public.submission_receipt_snapshot WHERE tenant_id=p_tenant AND attempt_id=ma;
 IF (SELECT count(*) FROM public.ple_account)<>5 OR NOT EXISTS(SELECT 1 FROM public.ple_account WHERE user_id=e AND normalized_email='elena.rivera@live-demo.ple.example' AND delivery_email='elena.rivera@live-demo.ple.example' AND display_name='Dr. Elena Rivera' AND platform_roles='[]') OR NOT EXISTS(SELECT 1 FROM public.ple_account WHERE user_id=m AND display_name='Mary Okafor' AND platform_roles='[]') OR NOT EXISTS(SELECT 1 FROM public.ple_account WHERE user_id=j AND display_name='Jack Chen' AND platform_roles='[]') OR NOT EXISTS(SELECT 1 FROM public.ple_account WHERE user_id=a AND display_name='Avery Singh' AND platform_roles='[]') OR NOT EXISTS(SELECT 1 FROM public.ple_account WHERE user_id=o AND display_name='Morgan Reyes' AND platform_roles='["sysadmin"]') OR (SELECT count(*) FROM public.instructor_approval)<>1 OR NOT EXISTS(SELECT 1 FROM public.instructor_approval WHERE user_id=e AND approved_by=o AND revoked_at IS NULL) OR (SELECT count(*) FROM public.tenant_learner_identity)<>3 THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 IF (SELECT count(*) FROM public.course)<>2 OR NOT EXISTS(SELECT 1 FROM public.course WHERE tenant_id=p_tenant AND course_id=bc AND title='Biochemistry Base Course' AND term_start_date='2026-01-01' AND term_end_date='2099-12-31' AND time_zone='America/Chicago') OR NOT EXISTS(SELECT 1 FROM public.course WHERE tenant_id=p_tenant AND course_id=pc AND title='Genetics Practice Course' AND term_start_date='2026-01-01' AND term_end_date='2099-12-31' AND time_zone='America/Chicago') OR (SELECT count(*) FROM public.course_member)<>5 OR bi IS NULL OR mm IS NULL OR jm IS NULL OR pi IS NULL OR am IS NULL OR EXISTS(SELECT 1 FROM public.course_member WHERE status<>'active' OR revoked_at IS NOT NULL OR roster_id IS NOT NULL) OR NOT EXISTS(SELECT 1 FROM public.tenant_learner_identity WHERE tenant_id=p_tenant AND user_id=m AND student_id=ms) OR NOT EXISTS(SELECT 1 FROM public.tenant_learner_identity WHERE tenant_id=p_tenant AND user_id=j AND student_id=js) OR NOT EXISTS(SELECT 1 FROM public.tenant_learner_identity WHERE tenant_id=p_tenant AND user_id=a AND student_id=as_id) OR (SELECT count(*) FROM public.course_roster_profile)<>3 OR NOT EXISTS(SELECT 1 FROM public.course_roster_profile WHERE tenant_id=p_tenant AND course_id=bc AND course_membership_id=mm AND display_name='Mary Okafor' AND roster_email_normalized IS NULL AND roster_email_delivery IS NULL) OR NOT EXISTS(SELECT 1 FROM public.course_roster_profile WHERE tenant_id=p_tenant AND course_id=bc AND course_membership_id=jm AND display_name='Jack Chen' AND roster_email_normalized IS NULL AND roster_email_delivery IS NULL) OR NOT EXISTS(SELECT 1 FROM public.course_roster_profile WHERE tenant_id=p_tenant AND course_id=pc AND course_membership_id=am AND display_name='Avery Singh' AND roster_email_normalized IS NULL AND roster_email_delivery IS NULL) THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 IF (SELECT count(*) FROM public.course_roster_state)<>2 OR NOT EXISTS(SELECT 1 FROM public.course_roster_state WHERE tenant_id=p_tenant AND course_id=bc AND revision=3 AND signup_posture='invitation_only') OR NOT EXISTS(SELECT 1 FROM public.course_roster_state WHERE tenant_id=p_tenant AND course_id=pc AND revision=2 AND signup_posture='invitation_only') OR (SELECT count(*) FROM public.course_appearance WHERE theme_id='grass' AND current_banner_delivery_id IS NULL AND banner_alt_kind IS NULL AND banner_alt_text IS NULL AND revision=1)<>2 OR (SELECT count(*) FROM public.course_group_membership_policy WHERE revision=1 AND (purpose,multiple_membership) IN(('section','warn'),('lab','allow'),('cohort','allow'),('accommodation','allow'),('work','allow')))<>10 OR (SELECT count(*) FROM public.course_grade_scheme)<>2 OR NOT EXISTS(SELECT 1 FROM public.course_grade_scheme WHERE tenant_id=p_tenant AND course_id=bc AND mode='total_points' AND rounding='four_decimal_places_half_away_from_zero' AND revision=2) OR NOT EXISTS(SELECT 1 FROM public.course_grade_scheme WHERE tenant_id=p_tenant AND course_id=pc AND mode='total_points' AND rounding='four_decimal_places_half_away_from_zero' AND revision=1) OR EXISTS(SELECT 1 FROM public.course_allowed_email_domain UNION ALL SELECT 1 FROM public.course_group UNION ALL SELECT 1 FROM public.course_group_member UNION ALL SELECT 1 FROM public.course_grade_category UNION ALL SELECT 1 FROM public.course_grade_category_assignment UNION ALL SELECT 1 FROM public.course_grade_letter_band UNION ALL SELECT 1 FROM public.course_total_export_audit) THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 IF (SELECT count(*) FROM public.problem)<>1 OR NOT EXISTS(SELECT 1 FROM public.problem WHERE problem_id=pid AND owner_tenant_id=p_tenant AND owner_user_id=e AND visibility='institution' AND license='ccBy' AND lifecycle='published') OR (SELECT count(*) FROM public.problem_version)<>1 OR NOT EXISTS(SELECT 1 FROM public.problem_version WHERE problem_id=pid AND version_id=vid AND workspace_id=wid AND model_schema_version=1 AND title='Peptide bond resonance and planarity' AND lifecycle='published' AND backend='native' AND publication_scope='institution' AND lifecycle_reason IS NULL AND author_ids=jsonb_build_array(e) AND public_byline=ARRAY['Dr. Elena Rivera'] AND derived_from_problem_id IS NULL AND derived_from_version_id IS NULL AND content_sha256~'^[0-9a-f]{64}$') OR (SELECT count(*) FROM public.problem_version_payload)<>1 OR content_hash IS DISTINCT FROM payload_hash OR qid IS NULL OR (SELECT count(*) FROM public.catalog_tenant_grant WHERE tenant_id=p_tenant AND problem_id=pid AND version_id=vid)<>1 OR (SELECT count(*) FROM public.catalog_search_document WHERE problem_id=pid AND version_id=vid AND question_id=qid AND title='Peptide bond resonance and planarity' AND backend='native' AND publication_scope='institution' AND lifecycle='published' AND byline_text='Dr. Elena Rivera' AND question_type='native' AND language='en-US' AND license='ccBy')<>1 OR EXISTS(SELECT 1 FROM public.published_source_artifact UNION ALL SELECT 1 FROM public.published_flat_import_origin UNION ALL SELECT 1 FROM public.published_flat_import_choice_map UNION ALL SELECT 1 FROM public.published_qti_grading UNION ALL SELECT 1 FROM public.answer_key UNION ALL SELECT 1 FROM public.workspace_draft WHERE tenant_id=p_tenant AND workspace_id=wid UNION ALL SELECT 1 FROM public.workspace_draft_access WHERE tenant_id=p_tenant AND workspace_id=wid UNION ALL SELECT 1 FROM public.workspace_flat_question_source WHERE tenant_id=p_tenant AND workspace_id=wid UNION ALL SELECT 1 FROM public.workspace_flat_question_grading WHERE tenant_id=p_tenant AND workspace_id=wid) THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 IF (SELECT count(*) FROM public.assignment)<>1 OR NOT EXISTS(SELECT 1 FROM public.assignment WHERE tenant_id=p_tenant AND assignment_id=aid AND course_id=bc AND title='Peptide Bonds: Structure and Resonance' AND instructions='Work through the peptide-bond geometry evidence before submitting.' AND lifecycle='published' AND audience_kind='course_wide' AND score_disclosure='after_submit' AND per_item_correctness_disclosure='after_submit' AND feedback_text_disclosure='after_submit' AND solution_disclosure='after_submit' AND class_statistics_disclosure='never' AND completion_policy='answer_all' AND attempt_selection_policy='highest' AND continued_practice_policy='unlimited' AND variation_policy='new_seeds') OR (SELECT count(*) FROM public.assignment_item)<>1 OR NOT EXISTS(SELECT 1 FROM public.assignment_item WHERE tenant_id=p_tenant AND assignment_id=aid AND assignment_item_id=item AND position=0 AND problem_id=pid AND version_id=vid AND points_possible=1 AND delivery_state='active' AND scoring_mode='normal') OR NOT EXISTS(SELECT 1 FROM public.assignment_effective_policy_base WHERE tenant_id=p_tenant AND assignment_id=aid AND course_id=bc AND available_at IS NULL AND due_at IS NULL AND closes_at IS NULL AND late_submission_policy='accept' AND deadline_behavior='auto_submit' AND time_limit_seconds IS NULL AND attempt_limit IS NULL) OR EXISTS(SELECT 1 FROM public.assignment_selection_group UNION ALL SELECT 1 FROM public.assignment_selection_candidate UNION ALL SELECT 1 FROM public.assignment_audience_group UNION ALL SELECT 1 FROM public.assignment_group_schedule_offset UNION ALL SELECT 1 FROM public.assignment_group_accommodation UNION ALL SELECT 1 FROM public.assignment_individual_policy_exception) THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 IF (SELECT count(*) FROM public.enrollment)<>2 OR me IS NULL OR je IS NULL OR NOT EXISTS(SELECT 1 FROM public.enrollment WHERE tenant_id=p_tenant AND enrollment_id=me AND assignment_id=aid AND user_id=m AND student_id=ms AND course_id=bc AND course_membership_id=mm AND materialization_purpose='instructor_issue' AND materialized_by_user_id=e AND evaluator_version=1 AND entitlement_receipts_sealed_at IS NOT NULL) OR NOT EXISTS(SELECT 1 FROM public.enrollment WHERE tenant_id=p_tenant AND enrollment_id=je AND assignment_id=aid AND user_id=j AND student_id=js AND course_id=bc AND course_membership_id=jm AND materialization_purpose='instructor_issue' AND materialized_by_user_id=e AND evaluator_version=1 AND entitlement_receipts_sealed_at IS NOT NULL) OR (SELECT count(*) FROM public.enrollment_entitlement_basis_receipt WHERE scope_kind='course_wide' AND course_id=bc AND course_group_id IS NULL AND course_group_purpose IS NULL)<>2 OR EXISTS(SELECT 1 FROM public.enrollment_applicable_policy_scope_receipt) OR NOT EXISTS(SELECT 1 FROM public.student_assignment_summary WHERE tenant_id=p_tenant AND enrollment_id=me AND current_score=1 AND best_score=1 AND latest_score=1 AND completed_run_count=1 AND total_question_attempts=1) OR NOT EXISTS(SELECT 1 FROM public.student_assignment_summary WHERE tenant_id=p_tenant AND enrollment_id=je AND current_score IS NULL AND best_score IS NULL AND latest_score IS NULL AND completed_run_count=0 AND total_question_attempts=0) THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 IF (SELECT count(*) FROM public.assignment_run)<>2
    OR NOT EXISTS(SELECT 1 FROM public.assignment_run WHERE tenant_id=p_tenant AND run_id=mr AND enrollment_id=me AND run_number=1 AND completed_at IS NOT NULL AND payload_sha256~'^[0-9a-f]{64}$' AND payload->>'mode'='assigned' AND payload->>'variation'='newSeeds' AND (payload->>'score')::numeric=1)
    OR NOT EXISTS(SELECT 1 FROM public.assignment_run WHERE tenant_id=p_tenant AND run_id=jr AND enrollment_id=je AND run_number=1 AND completed_at IS NULL AND payload_sha256~'^[0-9a-f]{64}$' AND payload->>'mode'='assigned' AND payload->>'variation'='newSeeds' AND payload->'score'='null')
    OR (SELECT count(*) FROM public.assignment_run_item WHERE assignment_item_id=item AND source_position=0 AND issued_position=0 AND problem_id=pid AND version_id=vid AND selection_group_id IS NULL AND selection_seed IS NULL AND delivery_status='issued')<>2
    OR (SELECT count(*) FROM public.question_attempt)<>2
    OR NOT EXISTS(SELECT 1 FROM public.question_attempt WHERE tenant_id=p_tenant AND attempt_id=ma AND run_id=mr AND problem_id=pid AND version_id=vid AND attempt_status='submitted' AND submitted_at IS NOT NULL AND assignment_position=0 AND course_id=bc AND external_tool_indeterminate_at IS NULL)
    OR NOT EXISTS(SELECT 1 FROM public.question_attempt WHERE tenant_id=p_tenant AND attempt_id=ja AND run_id=jr AND problem_id=pid AND version_id=vid AND attempt_status='in_progress' AND submitted_at IS NULL AND assignment_position=0 AND course_id=bc AND external_tool_indeterminate_at IS NULL)
    OR NOT public.ple_verify_native_private_execution_shape(p_tenant,ma)
    OR NOT public.ple_verify_native_private_execution_shape(p_tenant,ja)
 THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 IF (SELECT count(*) FROM public.attempt_effective_policy_receipt WHERE attempt_id IN(ma,ja) AND receipt_generation=1 AND assignment_id=aid AND course_id=bc AND resolved_available_at IS NULL AND resolved_due_at IS NULL AND resolved_closes_at IS NULL AND resolved_late_submission_policy='accept' AND resolved_deadline_behavior='auto_submit' AND resolved_time_limit_seconds IS NULL AND resolved_attempt_limit IS NULL AND effective_deadline IS NULL AND effective_grace_seconds=0 AND auto_submit_at IS NULL AND sealed_at IS NOT NULL)<>2 OR (SELECT count(*) FROM public.attempt_effective_policy_current WHERE attempt_id IN(ma,ja) AND assignment_id=aid AND course_id=bc AND receipt_generation=1)<>2 OR (SELECT count(*) FROM public.attempt_effective_policy_receipt_field_source WHERE attempt_id IN(ma,ja) AND receipt_generation=1 AND source_layer='base' AND source_order=0 AND source_id IS NULL)<>14 OR (SELECT count(*) FROM public.submission)<>1 OR sub IS NULL OR sub<>ma OR NOT EXISTS(SELECT 1 FROM public.submission WHERE tenant_id=p_tenant AND attempt_id=ma AND idempotency_key='installed-base-course-mary-answer' AND payload=jsonb_build_object('kind','multipleChoice','selected',jsonb_build_array('amide')) AND course_id=bc) OR (SELECT count(*) FROM public.submission_idempotency WHERE attempt_id=ma AND idempotency_key='installed-base-course-mary-answer' AND course_id=bc)<>1 OR (SELECT count(*) FROM public.submission_evaluation WHERE attempt_id=ma AND submission_id=sub AND credit_fraction=1 AND correct AND grading_status='graded' AND course_id=bc AND evaluation_revision=1)<>1 OR (SELECT count(*) FROM public.attempt_feedback WHERE attempt_id=ma AND course_id=bc AND content_sha256~'^[0-9a-f]{64}$')<>1 OR (SELECT count(*) FROM public.attempt_score_current WHERE attempt_id=ma AND assignment_id=aid AND assignment_item_id=item AND earned_points=1 AND possible_points=1 AND course_id=bc)<>1 OR (SELECT count(*) FROM public.submission_receipt_snapshot WHERE attempt_id=ma AND presentation_required AND run_payload_sha256=mrh AND presentation_payload_sha256=maph)<>1 OR (SELECT count(*) FROM public.question_statistics_contribution_receipt WHERE enrollment_id=me AND first_completed_run_id=mr AND attempt_id=ma AND problem_id=pid AND version_id=vid AND octet_length(observation_sha256)=32)<>1 OR NOT EXISTS(SELECT 1 FROM public.question_statistics_aggregate WHERE problem_id=pid AND version_id=vid AND cohort_size=1 AND score_sum=1 AND attempts_sum=1 AND duration_histogram_version=1) OR EXISTS(SELECT 1 FROM public.submission WHERE attempt_id=ja UNION ALL SELECT 1 FROM public.submission_idempotency WHERE attempt_id=ja UNION ALL SELECT 1 FROM public.submission_evaluation WHERE attempt_id=ja UNION ALL SELECT 1 FROM public.attempt_feedback WHERE attempt_id=ja UNION ALL SELECT 1 FROM public.attempt_score_current WHERE attempt_id=ja UNION ALL SELECT 1 FROM public.submission_receipt_snapshot WHERE attempt_id=ja UNION ALL SELECT 1 FROM public.submission_next_attempt UNION ALL SELECT 1 FROM public.feedback_release UNION ALL SELECT 1 FROM public.manual_grade_receipt UNION ALL SELECT 1 FROM public.question_prefetch) THEN failure_kind:='completion_aggregate_incomplete';RETURN NEXT;RETURN; END IF;
 SELECT encode(digest(convert_to(jsonb_build_object('scopeKind',scope_kind,'courseId',course_id,'groupId',course_group_id,'groupPurpose',course_group_purpose)::text,'UTF8'),'sha256'),'hex') INTO mary_basis FROM public.enrollment_entitlement_basis_receipt WHERE enrollment_id=me; SELECT encode(digest(convert_to(jsonb_build_object('scopeKind',scope_kind,'courseId',course_id,'groupId',course_group_id,'groupPurpose',course_group_purpose)::text,'UTF8'),'sha256'),'hex') INTO jack_basis FROM public.enrollment_entitlement_basis_receipt WHERE enrollment_id=je; SELECT encode(digest(convert_to(jsonb_build_object('currentScore',current_score,'bestScore',best_score,'latestScore',latest_score,'completedRuns',completed_run_count,'attempts',total_question_attempts)::text,'UTF8'),'sha256'),'hex') INTO mary_summary FROM public.student_assignment_summary WHERE enrollment_id=me; SELECT encode(digest(convert_to(jsonb_build_object('currentScore',current_score,'bestScore',best_score,'latestScore',latest_score,'completedRuns',completed_run_count,'attempts',total_question_attempts)::text,'UTF8'),'sha256'),'hex') INTO jack_summary FROM public.student_assignment_summary WHERE enrollment_id=je;
 receipt:=jsonb_build_object('schemaVersion',1,'baselineVersion','base-course-v1','installationGeneration',p_generation,'tenantId',p_tenant,'recipeSha256',p_recipe_sha256,'courseGraph',jsonb_build_object('baseCourseId',bc,'practiceCourseId',pc,'baseInstructorMembershipId',bi,'maryMembershipId',mm,'maryStudentId',ms,'jackMembershipId',jm,'jackStudentId',js,'practiceInstructorMembershipId',pi,'averyMembershipId',am,'averyStudentId',as_id,'baseRosterRevision',3,'practiceRosterRevision',2),'contentGraph',jsonb_build_object('questionId',substr(qid,1,3)||'-'||substr(qid,4),'problemId',pid,'versionId',vid,'assignmentId',aid,'assignmentItemId',item,'contentSha256',content_hash,'payloadSha256',payload_hash),'entitlementGraph',jsonb_build_object('maryEnrollmentId',me,'jackEnrollmentId',je,'maryBasisSha256',mary_basis,'jackBasisSha256',jack_basis,'applicableScopeSha256',encode(digest(convert_to('[]','UTF8'),'sha256'),'hex'),'marySummarySha256',mary_summary,'jackSummarySha256',jack_summary),'activityGraph',jsonb_build_object('maryRunId',mr,'maryAttemptId',ma,'marySubmissionId',sub,'jackRunId',jr,'jackAttemptId',ja,'maryRunSha256',mrh,'jackRunSha256',jrh,'maryAttemptSha256',mah,'jackAttemptSha256',jah,'maryPresentationSha256',maph,'jackPresentationSha256',japh,'maryGradingSha256',magh,'jackGradingSha256',jagh,'submissionSha256',subh,'idempotencyRequestSha256',idem_request,'idempotencyPayloadSha256',idem_payload,'evaluationSha256',evalh,'feedbackSha256',feedbackh,'snapshotRunSha256',snap_runh,'snapshotSummarySha256',snap_summaryh,'snapshotPresentationSha256',snap_presentationh)); failure_kind:=NULL;canonical_receipt:=receipt;receipt_sha256:=encode(digest(convert_to(receipt::text,'UTF8'),'sha256'),'hex');RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_base_course_install_complete_v2(p_tenant uuid,p_generation uuid,p_baseline text,p_manifest jsonb,p_receipt text) RETURNS TABLE(failure_kind text,canonical_receipt jsonb,canonical_receipt_text text,receipt_sha256 text) LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
DECLARE r jsonb;t uuid;recipe_hash text;verified record;
BEGIN PERFORM public.ple_require_base_course_install_lock_internal(); IF current_setting('transaction_isolation')<>'serializable' THEN RAISE EXCEPTION 'Base Course completion requires serializable isolation' USING ERRCODE='42501'; END IF; IF p_tenant IS NULL OR p_generation IS NULL OR p_baseline<>'base-course-v1' OR p_manifest<>'[]'::jsonb OR p_receipt !~ '^[0-9a-f]{64}$' THEN RAISE EXCEPTION 'Base Course completion request is invalid' USING ERRCODE='22023'; END IF;
 SELECT x.recipe,i.tenant_id,x.recipe_sha256 INTO r,t,recipe_hash FROM public.live_demo_install_state i JOIN public.live_demo_install_recipe x ON x.singleton AND x.installation_generation=i.installation_generation WHERE i.singleton AND i.state='installing' AND i.tenant_id=p_tenant AND i.installation_generation=p_generation AND i.baseline_version=p_baseline AND i.object_manifest=p_manifest FOR UPDATE OF i,x; IF NOT FOUND THEN RAISE EXCEPTION 'Base Course installation is unavailable' USING ERRCODE='42501'; END IF;
 PERFORM set_config('ple.tenant_id',t::text,true);
 SELECT * INTO STRICT verified FROM public.ple_verify_base_course_completion_internal(t,p_generation,recipe_hash,r); IF verified.failure_kind IS NOT NULL THEN failure_kind:=verified.failure_kind;canonical_receipt:=NULL;canonical_receipt_text:=NULL;receipt_sha256:=NULL;RETURN NEXT;RETURN; END IF;
 INSERT INTO public.live_demo_install_completion_receipt(installation_generation,tenant_id,schema_version,baseline_version,recipe_sha256,canonical_receipt,receipt_sha256) VALUES(p_generation,t,1,p_baseline,recipe_hash,verified.canonical_receipt,verified.receipt_sha256); UPDATE public.live_demo_install_state SET state='complete',storage_receipt_sha256=p_receipt,completion_receipt_sha256=verified.receipt_sha256,completed_at=transaction_timestamp() WHERE singleton AND state='installing' AND tenant_id=p_tenant AND installation_generation=p_generation AND baseline_version=p_baseline AND object_manifest=p_manifest; IF NOT FOUND THEN RAISE EXCEPTION 'Base Course installation is unavailable' USING ERRCODE='42501'; END IF; failure_kind:=NULL;canonical_receipt:=verified.canonical_receipt;canonical_receipt_text:=verified.canonical_receipt::text;receipt_sha256:=verified.receipt_sha256;RETURN NEXT;
END $$;
CREATE FUNCTION public.ple_base_course_install_release_lock_v1() RETURNS void LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$
BEGIN PERFORM public.ple_require_base_course_install_lock_internal();
IF NOT pg_advisory_unlock(70463,1818) THEN
RAISE EXCEPTION 'Base Course installation lock is unavailable' USING ERRCODE='42501';
END IF;
END $$;
ALTER FUNCTION public.ple_course_creation_deny_internal() OWNER TO ple_course_creation_broker; ALTER FUNCTION public.ple_course_creation_validate_inputs(uuid,uuid,text,date,date,text) OWNER TO ple_course_creation_broker; ALTER FUNCTION public.ple_create_course_core_internal(uuid,uuid,text,date,date,text,uuid) OWNER TO ple_course_creation_broker; ALTER FUNCTION public.ple_verify_course_creation_aggregate_internal(uuid,uuid,text,date,date,text,uuid) OWNER TO ple_course_creation_broker; ALTER FUNCTION public.ple_create_course_as_instructor_v1(uuid,uuid,text,date,date,text,uuid,character) OWNER TO ple_course_creation_broker; ALTER FUNCTION public.ple_create_course_as_sysadmin_v1(uuid,uuid,text,date,date,text,uuid,character) OWNER TO ple_course_creation_broker;
ALTER FUNCTION public.ple_upsert_course_student_as_instructor_v1(uuid,uuid,uuid,uuid,uuid,uuid,text,text,text,text) OWNER TO ple_course_roster_mutator_broker; ALTER FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(uuid,character,uuid,uuid,bigint) OWNER TO ple_course_roster_mutator_broker; CREATE FUNCTION public.ple_accept_co_instructor_invitation_v1(p_tenant uuid,p_session character(64),p_invitation uuid,p_expected_revision bigint) RETURNS TABLE(tenant_id uuid,actor_id uuid,course_id uuid,course_membership_id uuid,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE v_actor uuid;v_course uuid;v_target uuid;v_status text;v_revision bigint;v_invitation_revision bigint;v_membership uuid;v_role text; BEGIN IF p_tenant IS NULL OR p_session IS NULL OR p_invitation IS NULL OR p_expected_revision IS NULL OR p_tenant IS DISTINCT FROM public.ple_current_tenant() OR p_expected_revision<1 THEN RAISE EXCEPTION 'co-instructor acceptance arguments are invalid' USING ERRCODE='22023'; END IF; SELECT s.user_id INTO v_actor FROM public.auth_session s WHERE s.session_hash=p_session AND s.tenant_id=p_tenant AND s.revoked_at IS NULL AND s.expires_at>transaction_timestamp() FOR UPDATE; IF NOT FOUND THEN RETURN; END IF; SELECT c.course_id INTO v_course FROM public.course c JOIN public.course_instructor_invitation i ON i.tenant_id=c.tenant_id AND i.course_id=c.course_id WHERE c.tenant_id=p_tenant AND i.invitation_id=p_invitation FOR UPDATE OF c; IF NOT FOUND THEN RETURN; END IF; SELECT revision INTO v_revision FROM public.course_roster_state WHERE tenant_id=p_tenant AND course_id=v_course FOR UPDATE; IF NOT FOUND OR v_revision<1 THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF; SELECT target_user_id,status,revision,accepted_membership_id INTO v_target,v_status,v_invitation_revision,v_membership FROM public.course_instructor_invitation WHERE tenant_id=p_tenant AND invitation_id=p_invitation FOR UPDATE; IF v_target IS DISTINCT FROM v_actor THEN RETURN; END IF; IF v_status='accepted' THEN IF v_membership IS NULL THEN RAISE EXCEPTION 'co-instructor acceptance aggregate is invalid' USING ERRCODE='55000'; END IF; tenant_id:=p_tenant;actor_id:=v_actor;course_id:=v_course;course_membership_id:=v_membership;roster_revision:=v_revision;RETURN NEXT;RETURN; END IF; IF v_status<>'pending' OR v_invitation_revision<>p_expected_revision OR NOT EXISTS(SELECT 1 FROM public.course_instructor_invitation WHERE tenant_id=p_tenant AND invitation_id=p_invitation AND expires_at>transaction_timestamp()) OR NOT public.ple_lock_instructor_approval_eligibility(v_actor) THEN RAISE EXCEPTION 'co-instructor acceptance is unavailable' USING ERRCODE='55000'; END IF; SELECT course_membership_id,role INTO v_membership,v_role FROM public.course_member WHERE tenant_id=p_tenant AND course_id=v_course AND user_id=v_actor AND status='active' FOR UPDATE; IF FOUND AND v_role<>'instructor' THEN RAISE EXCEPTION 'co-instructor membership conflicts' USING ERRCODE='55000'; END IF; IF NOT FOUND THEN v_membership:=gen_random_uuid(); INSERT INTO public.course_member(tenant_id,course_id,course_membership_id,user_id,role,student_id,status,joined_at) VALUES(p_tenant,v_course,v_membership,v_actor,'instructor',NULL,'active',transaction_timestamp()); END IF; UPDATE public.course_instructor_invitation SET status='accepted',accepted_at=transaction_timestamp(),accepted_membership_id=v_membership,revision=revision+1 WHERE tenant_id=p_tenant AND invitation_id=p_invitation AND status='pending' AND revision=p_expected_revision; IF NOT FOUND THEN RAISE EXCEPTION 'co-instructor acceptance is unavailable' USING ERRCODE='55000'; END IF; UPDATE public.course_roster_state SET revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=v_course AND revision=v_revision RETURNING revision INTO roster_revision; IF NOT FOUND THEN RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE='55000'; END IF; tenant_id:=p_tenant;actor_id:=v_actor;course_id:=v_course;course_membership_id:=v_membership;RETURN NEXT; END $$; ALTER FUNCTION public.ple_accept_co_instructor_invitation_v1(uuid,character,uuid,bigint) OWNER TO ple_teaching_authority_broker; CREATE POLICY teaching_authority_accept_member_write ON public.course_member FOR INSERT TO ple_teaching_authority_broker WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY teaching_authority_accept_invitation_write ON public.course_instructor_invitation FOR UPDATE TO ple_teaching_authority_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY teaching_authority_accept_roster_write ON public.course_roster_state FOR UPDATE TO ple_teaching_authority_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); GRANT INSERT ON public.course_member TO ple_teaching_authority_broker; GRANT UPDATE(status,accepted_at,accepted_membership_id,revision) ON public.course_instructor_invitation TO ple_teaching_authority_broker; GRANT UPDATE(revision,updated_at) ON public.course_roster_state TO ple_teaching_authority_broker;
ALTER FUNCTION public.ple_require_base_course_install_lock_internal() OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_base_course_install_validate_recipe_internal(jsonb) OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_base_course_install_acquire_lock_v1() OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_base_course_install_read_v2() OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_require_fresh_base_course_install_internal() OWNER TO ple_base_course_freshness_broker;
ALTER FUNCTION public.ple_base_course_install_prepare_v2(uuid,text,jsonb,jsonb) OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_base_course_install_seed_accounts_v2(uuid) OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_verify_base_course_accounts_internal(jsonb) OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_verify_base_course_course_prefix_internal(jsonb,text) OWNER TO ple_course_creation_broker;
ALTER FUNCTION public.ple_base_course_install_seed_course_v2(uuid,text) OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb) OWNER TO ple_base_course_completion_verification_broker;
ALTER FUNCTION public.ple_base_course_install_complete_v2(uuid,uuid,text,jsonb,text) OWNER TO ple_base_course_install_broker;
ALTER FUNCTION public.ple_base_course_install_release_lock_v1() OWNER TO ple_base_course_install_broker;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM ple_base_course_installer;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_base_course_installer;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM ple_base_course_installer;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_base_course_freshness_broker;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM ple_base_course_freshness_broker;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM ple_base_course_completion_verification_broker;
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA public FROM ple_base_course_completion_verification_broker;
REVOKE ALL ON FUNCTION public.ple_upsert_course_student_as_instructor_v1(uuid,uuid,uuid,uuid,uuid,uuid,text,text,text,text) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(uuid,character,uuid,uuid,bigint) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_course_creation_deny_internal(),public.ple_course_creation_validate_inputs(uuid,uuid,text,date,date,text),public.ple_create_course_core_internal(uuid,uuid,text,date,date,text,uuid),public.ple_verify_course_creation_aggregate_internal(uuid,uuid,text,date,date,text,uuid),public.ple_create_course_as_instructor_v1(uuid,uuid,text,date,date,text,uuid,character),public.ple_create_course_as_sysadmin_v1(uuid,uuid,text,date,date,text,uuid,character),public.ple_require_base_course_install_lock_internal(),public.ple_base_course_install_validate_recipe_internal(jsonb),public.ple_verify_base_course_accounts_internal(jsonb),public.ple_verify_base_course_course_prefix_internal(jsonb,text),public.ple_require_fresh_base_course_install_internal(),public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.ple_base_course_install_acquire_lock_v1(),public.ple_base_course_install_read_v2(),public.ple_base_course_install_prepare_v2(uuid,text,jsonb,jsonb),public.ple_base_course_install_seed_accounts_v2(uuid),public.ple_base_course_install_seed_course_v2(uuid,text),public.ple_base_course_install_complete_v2(uuid,uuid,text,jsonb,text),public.ple_base_course_install_release_lock_v1() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_create_course_as_instructor_v1(uuid,uuid,text,date,date,text,uuid,character),public.ple_create_course_as_sysadmin_v1(uuid,uuid,text,date,date,text,uuid,character) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_upsert_course_student_as_instructor_v1(uuid,uuid,uuid,uuid,uuid,uuid,text,text,text,text) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_revoke_course_student_as_roster_actor_v1(uuid,character,uuid,uuid,bigint) TO ple_app; REVOKE ALL ON FUNCTION public.ple_accept_co_instructor_invitation_v1(uuid,character,uuid,bigint) FROM PUBLIC; GRANT EXECUTE ON FUNCTION public.ple_accept_co_instructor_invitation_v1(uuid,character,uuid,bigint) TO ple_app;
GRANT EXECUTE ON FUNCTION public.ple_create_course_core_internal(uuid,uuid,text,date,date,text,uuid),public.ple_verify_base_course_course_prefix_internal(jsonb,text),public.ple_course_creation_deny_internal(),public.ple_course_creation_validate_inputs(uuid,uuid,text,date,date,text) TO ple_base_course_install_broker;
GRANT EXECUTE ON FUNCTION public.ple_require_fresh_base_course_install_internal() TO ple_base_course_freshness_broker;
GRANT EXECUTE ON FUNCTION public.ple_require_fresh_base_course_install_internal() TO ple_base_course_install_broker;
GRANT EXECUTE ON FUNCTION public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb),public.ple_verify_native_private_execution_shape(uuid,uuid) TO ple_base_course_install_broker,ple_base_course_completion_verification_broker;
GRANT EXECUTE ON FUNCTION public.ple_base_course_install_acquire_lock_v1(),public.ple_base_course_install_read_v2(),public.ple_base_course_install_prepare_v2(uuid,text,jsonb,jsonb),public.ple_base_course_install_seed_accounts_v2(uuid),public.ple_base_course_install_seed_course_v2(uuid,text),public.ple_base_course_install_complete_v2(uuid,uuid,text,jsonb,text),public.ple_base_course_install_release_lock_v1() TO ple_base_course_installer;
DO $$
DECLARE roster_functions regprocedure[]:=ARRAY['public.ple_upsert_course_student_as_instructor_v1(uuid,uuid,uuid,uuid,uuid,uuid,text,text,text,text)'::regprocedure,'public.ple_revoke_course_student_as_roster_actor_v1(uuid,character,uuid,uuid,bigint)'::regprocedure];roster_support regprocedure:='public.ple_course_roster_support_actor(character,uuid,text)'::regprocedure;
BEGIN
 IF (SELECT count(*) FROM pg_proc p WHERE p.oid=ANY(roster_functions) AND p.proowner='ple_course_roster_mutator_broker'::regrole AND p.prosecdef AND p.provolatile='v' AND p.proconfig=ARRAY['search_path=pg_catalog, public, pg_temp'])<>2 THEN RAISE EXCEPTION 'course-roster mutator function catalog is unsafe'; END IF;
 IF EXISTS(WITH expected(procedure_oid,grantee_name) AS (VALUES (roster_functions[1],'ple_app'),(roster_functions[2],'ple_app'),(roster_support,'ple_app'),(roster_support,'ple_course_roster_mutator_broker')),actual AS (SELECT p.oid,r.rolname FROM pg_proc p CROSS JOIN LATERAL aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) x JOIN pg_roles r ON r.oid=x.grantee WHERE (p.oid=ANY(roster_functions) OR p.oid=roster_support) AND x.privilege_type='EXECUTE' AND x.grantee<>p.proowner) SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference) THEN RAISE EXCEPTION 'course-roster mutator execution matrix is unsafe'; END IF;
 IF EXISTS(SELECT 1 FROM pg_auth_members WHERE member='ple_course_roster_mutator_broker'::regrole OR roleid='ple_course_roster_mutator_broker'::regrole) THEN RAISE EXCEPTION 'course-roster mutator broker has a membership edge'; END IF;
 IF EXISTS(WITH expected(relation_name,privilege_type) AS (VALUES ('course','SELECT'),('course_member','SELECT'),('course_member','INSERT'),('tenant_learner_identity','SELECT'),('tenant_learner_identity','INSERT'),('course_roster_profile','SELECT'),('course_roster_profile','INSERT'),('course_roster_state','SELECT')),actual AS (SELECT c.relname,x.privilege_type FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace CROSS JOIN LATERAL aclexplode(COALESCE(c.relacl,acldefault('r',c.relowner))) x WHERE n.nspname='public' AND c.relkind IN ('r','p') AND x.grantee='ple_course_roster_mutator_broker'::regrole AND x.grantee<>c.relowner) SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference) THEN RAISE EXCEPTION 'course-roster mutator table privilege matrix is unsafe'; END IF;
 IF EXISTS(WITH expected(relation_name,column_name) AS (VALUES ('course','course_id'),('course_member','course_membership_id'),('course_member','status'),('course_member','revoked_at'),('course_roster_state','revision'),('course_roster_state','updated_at')),actual AS (SELECT c.relname,a.attname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_attribute a ON a.attrelid=c.oid CROSS JOIN LATERAL aclexplode(a.attacl) x WHERE n.nspname='public' AND a.attnum>0 AND NOT a.attisdropped AND x.grantee='ple_course_roster_mutator_broker'::regrole AND x.privilege_type='UPDATE') SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference) THEN RAISE EXCEPTION 'course-roster mutator column privilege matrix is unsafe'; END IF;
 IF EXISTS(WITH expected(policy_name,relation_name) AS (VALUES ('course_roster_mutator_course_tenant','course'),('course_roster_mutator_member_tenant','course_member'),('course_roster_mutator_identity_tenant','tenant_learner_identity'),('course_roster_mutator_profile_tenant','course_roster_profile'),('course_roster_mutator_state_tenant','course_roster_state')),actual AS (SELECT p.polname,c.relname FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid WHERE 'ple_course_roster_mutator_broker'::regrole::oid=ANY(p.polroles)) SELECT 1 FROM ((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL (SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference) THEN RAISE EXCEPTION 'course-roster mutator policy matrix is unsafe'; END IF;
 IF has_table_privilege('ple_course_roster_mutator_broker','public.audit_event','SELECT') OR has_table_privilege('ple_course_roster_mutator_broker','public.course_invitation','SELECT') OR has_table_privilege('ple_course_roster_mutator_broker','public.course_group_member','DELETE') OR has_table_privilege('ple_course_roster_mutator_broker','public.live_demo_install_state','SELECT') OR NOT has_sequence_privilege('ple_course_roster_mutator_broker','public.course_member_public_id_seq','USAGE') THEN RAISE EXCEPTION 'course-roster mutator authority ceiling is unsafe'; END IF;
END $$;
DO $$
DECLARE
    role_name text;
BEGIN
    FOREACH role_name IN ARRAY ARRAY[
        'ple_course_creation_broker',
        'ple_base_course_installer',
        'ple_base_course_install_broker',
        'ple_base_course_freshness_broker',
        'ple_base_course_completion_verification_broker',
        'ple_course_roster_mutator_broker'
    ] LOOP
        IF NOT EXISTS (
            SELECT 1 FROM pg_roles
             WHERE rolname=role_name AND NOT rolcanlogin AND NOT rolsuper
               AND NOT rolcreatedb AND NOT rolcreaterole AND NOT rolinherit
               AND NOT rolreplication AND NOT rolbypassrls
        ) THEN
            RAISE EXCEPTION 'unsafe course-creation capability role %', role_name;
        END IF;
    END LOOP;
    IF EXISTS (
        SELECT 1 FROM pg_auth_members
         WHERE member IN (
             'ple_course_creation_broker'::regrole,
             'ple_base_course_installer'::regrole,
             'ple_base_course_install_broker'::regrole,
             'ple_base_course_freshness_broker'::regrole,
             'ple_base_course_completion_verification_broker'::regrole,
             'ple_course_roster_mutator_broker'::regrole
         ) OR roleid IN ('ple_base_course_freshness_broker'::regrole,'ple_base_course_completion_verification_broker'::regrole,'ple_course_roster_mutator_broker'::regrole)
    ) THEN
        RAISE EXCEPTION 'course-creation capability role has a membership edge';
    END IF;
    IF EXISTS (
        WITH expected(signature, owner_name) AS (
            VALUES
                ('public.ple_course_creation_deny_internal()','ple_course_creation_broker'),('public.ple_course_creation_validate_inputs(uuid,uuid,text,date,date,text)','ple_course_creation_broker'),('public.ple_create_course_core_internal(uuid,uuid,text,date,date,text,uuid)','ple_course_creation_broker'),('public.ple_verify_course_creation_aggregate_internal(uuid,uuid,text,date,date,text,uuid)','ple_course_creation_broker'),
                ('public.ple_verify_base_course_course_prefix_internal(jsonb,text)','ple_course_creation_broker'),('public.ple_create_course_as_instructor_v1(uuid,uuid,text,date,date,text,uuid,character)','ple_course_creation_broker'),('public.ple_create_course_as_sysadmin_v1(uuid,uuid,text,date,date,text,uuid,character)','ple_course_creation_broker'),
                ('public.ple_require_base_course_install_lock_internal()','ple_base_course_install_broker'),('public.ple_base_course_install_validate_recipe_internal(jsonb)','ple_base_course_install_broker'),('public.ple_base_course_install_acquire_lock_v1()','ple_base_course_install_broker'),('public.ple_base_course_install_read_v2()','ple_base_course_install_broker'),('public.ple_require_fresh_base_course_install_internal()','ple_base_course_freshness_broker'),('public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb)','ple_base_course_completion_verification_broker'),
                ('public.ple_base_course_install_prepare_v2(uuid,text,jsonb,jsonb)','ple_base_course_install_broker'),('public.ple_base_course_install_seed_accounts_v2(uuid)','ple_base_course_install_broker'),('public.ple_verify_base_course_accounts_internal(jsonb)','ple_base_course_install_broker'),('public.ple_base_course_install_seed_course_v2(uuid,text)','ple_base_course_install_broker'),('public.ple_base_course_install_complete_v2(uuid,uuid,text,jsonb,text)','ple_base_course_install_broker'),('public.ple_base_course_install_release_lock_v1()','ple_base_course_install_broker')
        ), actual AS (
            SELECT p.oid AS procedure_oid, r.rolname AS owner_name,
                   p.prosecdef, p.proconfig
              FROM pg_proc AS p
              JOIN pg_namespace AS n ON n.oid=p.pronamespace
              JOIN pg_roles AS r ON r.oid=p.proowner
             WHERE n.nspname='public'
               AND p.proname IN (
                   'ple_course_creation_deny_internal',
                   'ple_course_creation_validate_inputs',
                   'ple_create_course_core_internal',
                   'ple_verify_course_creation_aggregate_internal',
                   'ple_verify_base_course_course_prefix_internal',
                   'ple_create_course_as_instructor_v1',
                   'ple_create_course_as_sysadmin_v1',
                   'ple_require_base_course_install_lock_internal',
                   'ple_base_course_install_validate_recipe_internal',
                   'ple_base_course_install_acquire_lock_v1',
                   'ple_base_course_install_read_v2',
                   'ple_require_fresh_base_course_install_internal',
                   'ple_verify_base_course_completion_internal',
                   'ple_base_course_install_prepare_v2',
                   'ple_base_course_install_seed_accounts_v2',
                   'ple_verify_base_course_accounts_internal',
                   'ple_base_course_install_seed_course_v2',
                   'ple_base_course_install_complete_v2',
                   'ple_base_course_install_release_lock_v1'
               )
        )
        SELECT 1
          FROM (
              (SELECT to_regprocedure(signature)::oid, owner_name FROM expected
               EXCEPT SELECT procedure_oid, owner_name FROM actual)
              UNION ALL
              (SELECT procedure_oid, owner_name FROM actual
               EXCEPT SELECT to_regprocedure(signature)::oid, owner_name FROM expected)
          ) AS function_set_difference
        UNION ALL
        SELECT 1 FROM expected
         WHERE to_regprocedure(signature) IS NULL
        UNION ALL
        SELECT 1 FROM actual
         WHERE NOT prosecdef
            OR proconfig IS DISTINCT FROM ARRAY['search_path=pg_catalog, public, pg_temp']
    ) THEN
        RAISE EXCEPTION 'course-creation definer function catalog is unsafe';
    END IF;
    IF EXISTS (
        WITH expected(signature, grantee_name) AS (
            VALUES
                ('public.ple_create_course_as_instructor_v1(uuid,uuid,text,date,date,text,uuid,character)','ple_app'),
                ('public.ple_create_course_as_sysadmin_v1(uuid,uuid,text,date,date,text,uuid,character)','ple_app'),
                ('public.ple_base_course_install_acquire_lock_v1()','ple_base_course_installer'),
                ('public.ple_base_course_install_read_v2()','ple_base_course_installer'),
                ('public.ple_base_course_install_prepare_v2(uuid,text,jsonb,jsonb)','ple_base_course_installer'),
                ('public.ple_base_course_install_seed_accounts_v2(uuid)','ple_base_course_installer'),
                ('public.ple_base_course_install_seed_course_v2(uuid,text)','ple_base_course_installer'),
                ('public.ple_base_course_install_complete_v2(uuid,uuid,text,jsonb,text)','ple_base_course_installer'),
                ('public.ple_base_course_install_release_lock_v1()','ple_base_course_installer'),
                ('public.ple_require_fresh_base_course_install_internal()','ple_base_course_install_broker'),
                ('public.ple_course_creation_deny_internal()','ple_base_course_install_broker'),
                ('public.ple_course_creation_validate_inputs(uuid,uuid,text,date,date,text)','ple_base_course_install_broker'),
                ('public.ple_create_course_core_internal(uuid,uuid,text,date,date,text,uuid)','ple_base_course_install_broker'),
                ('public.ple_verify_base_course_course_prefix_internal(jsonb,text)','ple_base_course_install_broker'),
                ('public.ple_verify_base_course_completion_internal(uuid,uuid,text,jsonb)','ple_base_course_install_broker'),
                ('public.ple_current_tenant()','ple_course_creation_broker'),
                ('public.ple_course_records_accessible(uuid,uuid)','ple_course_creation_broker'),
                ('public.ple_lock_instructor_approval_eligibility(uuid)','ple_app'),
                ('public.ple_lock_instructor_approval_eligibility(uuid)','ple_course_creation_broker'),
                ('public.ple_current_tenant()','ple_base_course_install_broker')
        ), actual AS (
            SELECT p.oid AS procedure_oid, grantee.rolname AS grantee_name,
                   privilege.is_grantable
              FROM pg_proc AS p
              JOIN pg_namespace AS n ON n.oid=p.pronamespace
              CROSS JOIN LATERAL aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) AS privilege
              JOIN pg_roles AS grantee ON grantee.oid=privilege.grantee
             WHERE n.nspname='public' AND privilege.privilege_type='EXECUTE'
               AND (
                   p.proname IN (
                       'ple_course_creation_deny_internal',
                       'ple_course_creation_validate_inputs',
                       'ple_create_course_core_internal',
                       'ple_verify_course_creation_aggregate_internal',
                       'ple_verify_base_course_course_prefix_internal',
                       'ple_create_course_as_instructor_v1',
                       'ple_create_course_as_sysadmin_v1',
                       'ple_require_base_course_install_lock_internal',
                       'ple_base_course_install_validate_recipe_internal',
                       'ple_base_course_install_acquire_lock_v1',
                       'ple_base_course_install_read_v2',
                       'ple_require_fresh_base_course_install_internal',
                       'ple_verify_base_course_completion_internal',
                       'ple_base_course_install_prepare_v2',
                       'ple_base_course_install_seed_accounts_v2',
                       'ple_verify_base_course_accounts_internal',
                       'ple_base_course_install_seed_course_v2',
                       'ple_base_course_install_complete_v2',
                       'ple_base_course_install_release_lock_v1'
                   )
                   OR (
                       p.oid=to_regprocedure('public.ple_current_tenant()')
                       AND grantee.rolname IN (
                           'ple_course_creation_broker',
                           'ple_base_course_install_broker'
                       )
                   )
                   OR (
                       p.oid=to_regprocedure('public.ple_course_records_accessible(uuid,uuid)')
                       AND grantee.rolname='ple_course_creation_broker'
                   )
                   OR (
                       p.oid=to_regprocedure('public.ple_lock_instructor_approval_eligibility(uuid)')
                       AND grantee.rolname IN ('ple_app','ple_course_creation_broker')
                   )
               )
               AND privilege.grantee<>p.proowner
        )
        SELECT 1
          FROM (
              (SELECT to_regprocedure(signature)::oid, grantee_name FROM expected
               EXCEPT SELECT procedure_oid, grantee_name FROM actual WHERE NOT is_grantable)
              UNION ALL
              (SELECT procedure_oid, grantee_name FROM actual
               EXCEPT SELECT to_regprocedure(signature)::oid, grantee_name FROM expected)
          ) AS execute_set_difference
        UNION ALL
        SELECT 1
          FROM pg_proc AS p
          JOIN pg_namespace AS n ON n.oid=p.pronamespace
          CROSS JOIN LATERAL aclexplode(COALESCE(p.proacl,acldefault('f',p.proowner))) AS privilege
         WHERE n.nspname='public' AND privilege.privilege_type='EXECUTE'
           AND privilege.grantee=0
           AND (
               p.proname IN (
                   'ple_course_creation_deny_internal',
                   'ple_course_creation_validate_inputs',
                   'ple_create_course_core_internal',
                   'ple_verify_course_creation_aggregate_internal',
                   'ple_verify_base_course_course_prefix_internal',
                   'ple_create_course_as_instructor_v1',
                   'ple_create_course_as_sysadmin_v1',
                   'ple_require_base_course_install_lock_internal',
                   'ple_base_course_install_validate_recipe_internal',
                   'ple_base_course_install_acquire_lock_v1',
                   'ple_base_course_install_read_v2',
                   'ple_require_fresh_base_course_install_internal',
                   'ple_verify_base_course_completion_internal',
                   'ple_base_course_install_prepare_v2',
                   'ple_base_course_install_seed_accounts_v2',
                   'ple_verify_base_course_accounts_internal',
                   'ple_base_course_install_seed_course_v2',
                   'ple_base_course_install_complete_v2',
                   'ple_base_course_install_release_lock_v1'
               )
               OR p.oid=to_regprocedure('public.ple_lock_instructor_approval_eligibility(uuid)')
           )
    ) THEN
        RAISE EXCEPTION 'course-creation function execution matrix is unsafe';
    END IF;
    IF EXISTS (
        WITH expected(relation_name, grantee_name, privilege_type) AS (
            VALUES
                ('public.course','ple_course_creation_broker','SELECT'),
                ('public.course','ple_course_creation_broker','INSERT'),
                ('public.course_member','ple_course_creation_broker','SELECT'),
                ('public.course_member','ple_course_creation_broker','INSERT'),
                ('public.course_roster_state','ple_course_creation_broker','SELECT'),
                ('public.course_roster_state','ple_course_creation_broker','INSERT'),
                ('public.course_group_membership_policy','ple_course_creation_broker','SELECT'),
                ('public.course_group_membership_policy','ple_course_creation_broker','INSERT'),
                ('public.course_grade_scheme','ple_course_creation_broker','SELECT'),
                ('public.course_grade_scheme','ple_course_creation_broker','INSERT'),
                ('public.course_appearance','ple_course_creation_broker','SELECT'),
                ('public.course_appearance','ple_course_creation_broker','INSERT'),
                ('public.auth_session','ple_course_creation_broker','SELECT'),
                ('public.course_roster_profile','ple_course_creation_broker','SELECT'),
                ('public.tenant_learner_identity','ple_course_creation_broker','SELECT'),
                ('public.assignment','ple_course_creation_broker','SELECT'),
                ('public.course_allowed_email_domain','ple_course_creation_broker','SELECT'),('public.course_group','ple_course_creation_broker','SELECT'),('public.course_group_member','ple_course_creation_broker','SELECT'),('public.course_grade_category','ple_course_creation_broker','SELECT'),('public.course_grade_category_assignment','ple_course_creation_broker','SELECT'),('public.course_grade_letter_band','ple_course_creation_broker','SELECT'),
                ('public.live_demo_install_state','ple_base_course_install_broker','SELECT'),
                ('public.live_demo_install_state','ple_base_course_install_broker','INSERT'),
                ('public.live_demo_install_state','ple_base_course_install_broker','UPDATE'),
                ('public.live_demo_install_recipe','ple_base_course_install_broker','SELECT'),
                ('public.live_demo_install_recipe','ple_base_course_install_broker','INSERT'),
                ('public.live_demo_install_recipe','ple_base_course_install_broker','UPDATE'),
                ('public.ple_account','ple_base_course_install_broker','SELECT'),
                ('public.ple_account','ple_base_course_install_broker','INSERT'),
                ('public.ple_account','ple_base_course_install_broker','UPDATE'),
                ('public.instructor_approval','ple_base_course_install_broker','SELECT'),
                ('public.instructor_approval','ple_base_course_install_broker','INSERT'),
                ('public.instructor_approval','ple_base_course_install_broker','UPDATE')
                ,('public.live_demo_install_completion_receipt','ple_base_course_install_broker','INSERT')
        ), actual AS (
            SELECT format('%I.%I',n.nspname,c.relname) AS relation_name,
                   grantee.rolname AS grantee_name, privilege.privilege_type
              FROM pg_class AS c
              JOIN pg_namespace AS n ON n.oid=c.relnamespace
              CROSS JOIN LATERAL aclexplode(COALESCE(c.relacl,acldefault('r',c.relowner))) AS privilege
              JOIN pg_roles AS grantee ON grantee.oid=privilege.grantee
             WHERE n.nspname='public' AND c.relkind IN ('r','p')
               AND grantee.rolname IN (
                   'ple_course_creation_broker',
                   'ple_base_course_installer',
                   'ple_base_course_install_broker'
               )
               AND privilege.grantee<>c.relowner
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS relation_acl_set_difference
        UNION ALL
        SELECT 1 FROM expected WHERE to_regclass(relation_name) IS NULL
    ) THEN
        RAISE EXCEPTION 'course-creation table privilege matrix is unsafe';
    END IF;
    IF EXISTS (
        WITH expected(relation_name, column_name, grantee_name) AS (
            VALUES
                ('public.course','course_id','ple_course_creation_broker'),
                ('public.course_member','course_membership_id','ple_course_creation_broker'),
                ('public.course_roster_state','course_id','ple_course_creation_broker'),
                ('public.auth_session','session_hash','ple_course_creation_broker')
        ), actual AS (
            SELECT format('%I.%I',n.nspname,c.relname) AS relation_name,
                   a.attname AS column_name, grantee.rolname AS grantee_name
              FROM pg_class AS c
              JOIN pg_namespace AS n ON n.oid=c.relnamespace
              JOIN pg_attribute AS a ON a.attrelid=c.oid
              CROSS JOIN LATERAL aclexplode(a.attacl) AS privilege
              JOIN pg_roles AS grantee ON grantee.oid=privilege.grantee
             WHERE n.nspname='public' AND c.relkind IN ('r','p')
               AND a.attnum>0 AND NOT a.attisdropped
               AND privilege.privilege_type='UPDATE'
               AND grantee.rolname IN (
                   'ple_course_creation_broker',
                   'ple_base_course_installer',
                   'ple_base_course_install_broker'
               )
               AND privilege.grantee<>c.relowner
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS column_update_set_difference
    ) THEN
        RAISE EXCEPTION 'course-creation column update matrix is unsafe';
    END IF;
    IF EXISTS (
        WITH expected(sequence_name, grantee_name, privilege_type) AS (
            VALUES
                ('public.course_public_id_seq','ple_course_creation_broker','SELECT'),
                ('public.course_public_id_seq','ple_course_creation_broker','USAGE'),
                ('public.course_member_public_id_seq','ple_course_creation_broker','SELECT'),
                ('public.course_member_public_id_seq','ple_course_creation_broker','USAGE'),
                ('public.ple_account_public_id_seq','ple_base_course_install_broker','SELECT'),
                ('public.ple_account_public_id_seq','ple_base_course_install_broker','USAGE')
        ), actual AS (
            SELECT format('%I.%I',n.nspname,c.relname) AS sequence_name,
                   grantee.rolname AS grantee_name, privilege.privilege_type
              FROM pg_class AS c
              JOIN pg_namespace AS n ON n.oid=c.relnamespace
              CROSS JOIN LATERAL aclexplode(COALESCE(c.relacl,acldefault('S',c.relowner))) AS privilege
              JOIN pg_roles AS grantee ON grantee.oid=privilege.grantee
             WHERE n.nspname='public' AND c.relkind='S'
               AND grantee.rolname IN (
                   'ple_course_creation_broker',
                   'ple_base_course_installer',
                   'ple_base_course_install_broker'
               )
               AND privilege.grantee<>c.relowner
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS sequence_acl_set_difference
        UNION ALL
        SELECT 1 FROM expected WHERE to_regclass(sequence_name) IS NULL
    ) THEN
        RAISE EXCEPTION 'course-creation sequence privilege matrix is unsafe';
    END IF;
    IF EXISTS (
        WITH expected(relation_name, privilege_type) AS (
            SELECT format('%I.%I',namespace.nspname,table_row.relname), privilege_type
              FROM pg_catalog.pg_class AS table_row
              JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace
              CROSS JOIN (VALUES ('SELECT'),('MAINTAIN')) AS privilege(privilege_type)
             WHERE namespace.nspname='public' AND table_row.relkind IN ('r','p')
               AND table_row.relname NOT IN ('_sqlx_migrations')
        ), actual AS (
            SELECT format('%I.%I',namespace.nspname,table_row.relname), privilege.privilege_type
              FROM pg_catalog.pg_class AS table_row
              JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace
              CROSS JOIN LATERAL aclexplode(COALESCE(table_row.relacl,acldefault('r',table_row.relowner))) AS privilege
             WHERE privilege.grantee='ple_base_course_freshness_broker'::regrole
               AND privilege.grantee<>table_row.relowner
        )
        SELECT 1 FROM (
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
            UNION ALL
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
        ) AS freshness_relation_acl_set_difference
        UNION ALL
        SELECT 1
          FROM pg_catalog.pg_class AS table_row
          JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace
          CROSS JOIN (VALUES ('INSERT'),('UPDATE'),('DELETE'),('TRUNCATE'),('REFERENCES'),('TRIGGER')) AS forbidden(privilege_type)
         WHERE namespace.nspname='public' AND table_row.relkind IN ('r','p')
           AND has_table_privilege('ple_base_course_freshness_broker',table_row.oid,forbidden.privilege_type)
        UNION ALL
        SELECT 1
          FROM pg_catalog.pg_attribute AS attribute
          CROSS JOIN LATERAL aclexplode(attribute.attacl) AS privilege
         WHERE privilege.grantee='ple_base_course_freshness_broker'::regrole
        UNION ALL
        SELECT 1
          FROM pg_catalog.pg_class AS sequence_row
          JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=sequence_row.relnamespace
         WHERE namespace.nspname='public' AND sequence_row.relkind='S'
           AND (has_sequence_privilege('ple_base_course_freshness_broker',sequence_row.oid,'USAGE')
                OR has_sequence_privilege('ple_base_course_freshness_broker',sequence_row.oid,'SELECT')
                OR has_sequence_privilege('ple_base_course_freshness_broker',sequence_row.oid,'UPDATE'))
    ) THEN
        RAISE EXCEPTION 'Base Course freshness relation privilege matrix is unsafe';
    END IF;
    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_proc AS procedure
         WHERE procedure.proowner='ple_base_course_freshness_broker'::regrole
           AND procedure.oid<>to_regprocedure('public.ple_require_fresh_base_course_install_internal()')
        UNION ALL
        SELECT 1 WHERE (
            SELECT count(*) FROM pg_catalog.pg_proc AS procedure
             WHERE procedure.proowner='ple_base_course_freshness_broker'::regrole
        )<>1
        UNION ALL
        SELECT 1 FROM (
            (SELECT privilege.grantee,privilege.privilege_type,privilege.is_grantable
               FROM pg_catalog.pg_proc AS procedure
               CROSS JOIN LATERAL aclexplode(COALESCE(procedure.proacl,acldefault('f',procedure.proowner))) AS privilege
              WHERE procedure.oid=to_regprocedure('public.ple_require_fresh_base_course_install_internal()')
                AND privilege.grantee<>procedure.proowner
             EXCEPT SELECT 'ple_base_course_install_broker'::regrole::oid,'EXECUTE'::text,false)
            UNION ALL
            (SELECT 'ple_base_course_install_broker'::regrole::oid,'EXECUTE'::text,false
             EXCEPT SELECT privilege.grantee,privilege.privilege_type,privilege.is_grantable
               FROM pg_catalog.pg_proc AS procedure
               CROSS JOIN LATERAL aclexplode(COALESCE(procedure.proacl,acldefault('f',procedure.proowner))) AS privilege
              WHERE procedure.oid=to_regprocedure('public.ple_require_fresh_base_course_install_internal()')
                AND privilege.grantee<>procedure.proowner)
        ) AS freshness_function_acl_set_difference
    ) THEN
        RAISE EXCEPTION 'Base Course freshness function authority is unsafe';
    END IF;
    IF EXISTS (
        WITH expected(relation_name,policy_name,command,permissive,using_expression,check_expression) AS (
            SELECT format('%I.%I',namespace.nspname,table_row.relname),
                   'ple_base_course_freshness_select'::name,'r'::"char",true,'true'::text,NULL::text
              FROM pg_catalog.pg_class AS table_row
              JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace
             WHERE namespace.nspname='public' AND table_row.relkind IN ('r','p')
               AND table_row.relrowsecurity AND table_row.relname NOT IN ('_sqlx_migrations')
        ), actual AS (
            SELECT format('%I.%I',namespace.nspname,table_row.relname),policy.polname,
                   policy.polcmd,policy.polpermissive,
                   pg_get_expr(policy.polqual,policy.polrelid),
                   pg_get_expr(policy.polwithcheck,policy.polrelid)
              FROM pg_catalog.pg_policy AS policy
              JOIN pg_catalog.pg_class AS table_row ON table_row.oid=policy.polrelid
              JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=table_row.relnamespace
             WHERE 'ple_base_course_freshness_broker'::regrole::oid=ANY(policy.polroles)
        )
        SELECT 1 FROM (
            (SELECT * FROM expected EXCEPT SELECT * FROM actual)
            UNION ALL
            (SELECT * FROM actual EXCEPT SELECT * FROM expected)
        ) AS freshness_policy_set_difference
    ) THEN
        RAISE EXCEPTION 'Base Course freshness RLS policy matrix is unsafe';
    END IF;
    IF EXISTS (
        WITH expected(policy_name, relation_name, role_name, command, permissive, using_expression, check_expression) AS (
            VALUES
                ('course_creation_broker_course_tenant','public.course','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())','(tenant_id = ple_current_tenant())'),
                ('course_creation_broker_member_tenant','public.course_member','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())','(tenant_id = ple_current_tenant())'),
                ('course_creation_broker_roster_tenant','public.course_roster_state','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())','(tenant_id = ple_current_tenant())'),
                ('course_creation_broker_group_tenant','public.course_group_membership_policy','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())','(tenant_id = ple_current_tenant())'),
                ('course_creation_broker_scheme_tenant','public.course_grade_scheme','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())','(tenant_id = ple_current_tenant())'),
                ('course_creation_broker_appearance_tenant','public.course_appearance','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),
                ('course_creation_broker_session_tenant','public.auth_session','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),
                ('course_creation_broker_profile_tenant','public.course_roster_profile','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),
                ('course_creation_broker_identity_tenant','public.tenant_learner_identity','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),
                ('course_creation_broker_assignment_tenant','public.assignment','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),
                ('course_creation_broker_domain_tenant','public.course_allowed_email_domain','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),('course_creation_broker_course_group_tenant','public.course_group','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),('course_creation_broker_group_member_tenant','public.course_group_member','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),('course_creation_broker_grade_category_tenant','public.course_grade_category','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),('course_creation_broker_category_assignment_tenant','public.course_grade_category_assignment','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),('course_creation_broker_letter_band_tenant','public.course_grade_letter_band','ple_course_creation_broker','*',true,'(tenant_id = ple_current_tenant())',NULL),
                ('base_course_install_broker_recipe','public.live_demo_install_recipe','ple_base_course_install_broker','*',true,'true','true'),
                ('base_course_install_broker_account','public.ple_account','ple_base_course_install_broker','*',true,'true','true'),
                ('base_course_install_broker_completion_receipt','public.live_demo_install_completion_receipt','ple_base_course_install_broker','a',true,NULL,'true')
        ), actual AS (
            SELECT p.polname AS policy_name, format('%I.%I',n.nspname,c.relname) AS relation_name,
                   role_row.rolname AS role_name, p.polcmd::text AS command,
                   p.polpermissive AS permissive, pg_get_expr(p.polqual,p.polrelid) AS using_expression,
                   pg_get_expr(p.polwithcheck,p.polrelid) AS check_expression
              FROM pg_policy AS p
              JOIN pg_class AS c ON c.oid=p.polrelid
              JOIN pg_namespace AS n ON n.oid=c.relnamespace
              CROSS JOIN LATERAL unnest(p.polroles) AS policy_role(role_oid)
              JOIN pg_roles AS role_row ON role_row.oid=policy_role.role_oid
             WHERE n.nspname='public'
               AND p.polname IN (
                   'course_creation_broker_course_tenant',
                   'course_creation_broker_member_tenant',
                   'course_creation_broker_roster_tenant',
                   'course_creation_broker_group_tenant',
                   'course_creation_broker_scheme_tenant',
                   'course_creation_broker_appearance_tenant',
                   'course_creation_broker_session_tenant',
                   'course_creation_broker_profile_tenant',
                   'course_creation_broker_identity_tenant',
                   'course_creation_broker_assignment_tenant',
                   'course_creation_broker_domain_tenant','course_creation_broker_course_group_tenant','course_creation_broker_group_member_tenant','course_creation_broker_grade_category_tenant','course_creation_broker_category_assignment_tenant','course_creation_broker_letter_band_tenant',
                   'base_course_install_broker_recipe',
                   'base_course_install_broker_account',
                   'base_course_install_broker_completion_receipt'
               )
        )
        SELECT 1
          FROM (
              (SELECT * FROM expected EXCEPT SELECT * FROM actual)
              UNION ALL
              (SELECT * FROM actual EXCEPT SELECT * FROM expected)
          ) AS policy_set_difference
        UNION ALL
        SELECT 1
          FROM pg_class AS c
          JOIN pg_namespace AS n ON n.oid=c.relnamespace
         WHERE n.nspname='public' AND c.relname='live_demo_install_recipe'
           AND (NOT c.relrowsecurity OR NOT c.relforcerowsecurity)
        UNION ALL
        SELECT 1
         WHERE to_regclass('public.live_demo_install_recipe') IS NULL OR to_regclass('public.live_demo_install_completion_receipt') IS NULL
    ) THEN
        RAISE EXCEPTION 'course-creation RLS policy catalog is unsafe';
    END IF;
END
$$;
DO $$
DECLARE completion_relations text[]:=ARRAY['ple_account','instructor_approval','tenant_learner_identity','course','course_roster_state','course_appearance','course_allowed_email_domain','course_group_membership_policy','course_grade_scheme','course_grade_category','course_grade_category_assignment','course_grade_letter_band','course_total_export_audit','course_member','course_roster_profile','course_group','course_group_member','problem','problem_version','problem_version_payload','catalog_tenant_grant','catalog_search_document','published_source_artifact','published_flat_import_origin','published_flat_import_choice_map','published_qti_grading','answer_key','workspace_draft','workspace_draft_access','workspace_flat_question_source','workspace_flat_question_grading','assignment','assignment_item','assignment_selection_group','assignment_selection_candidate','assignment_audience_group','assignment_effective_policy_base','assignment_group_schedule_offset','assignment_group_accommodation','assignment_individual_policy_exception','enrollment','enrollment_entitlement_basis_receipt','enrollment_applicable_policy_scope_receipt','student_assignment_summary','assignment_run','assignment_run_item','question_attempt','attempt_effective_policy_receipt','attempt_effective_policy_receipt_field_source','attempt_effective_policy_current','submission','submission_idempotency','submission_evaluation','attempt_feedback','attempt_score_current','submission_receipt_snapshot','submission_next_attempt','feedback_release','manual_grade_receipt','question_prefetch','question_statistics_contribution_receipt','question_statistics_aggregate']; BEGIN IF EXISTS(WITH expected AS(SELECT relation_name,'SELECT'::text privilege_type FROM unnest(completion_relations) relation_name),actual AS(SELECT c.relname relation_name,x.privilege_type FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace CROSS JOIN LATERAL aclexplode(COALESCE(c.relacl,acldefault('r',c.relowner))) x WHERE n.nspname='public' AND c.relkind IN('r','p') AND x.grantee='ple_base_course_completion_verification_broker'::regrole AND x.grantee<>c.relowner) SELECT 1 FROM((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL(SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference) OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace CROSS JOIN LATERAL unnest(ARRAY['INSERT','UPDATE','DELETE','TRUNCATE','REFERENCES','TRIGGER']) privilege WHERE n.nspname='public' AND c.relkind IN('r','p') AND has_table_privilege('ple_base_course_completion_verification_broker',c.oid,privilege)) OR EXISTS(SELECT 1 FROM pg_attribute a CROSS JOIN LATERAL aclexplode(a.attacl) x WHERE x.grantee='ple_base_course_completion_verification_broker'::regrole) OR EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind='S' AND(has_sequence_privilege('ple_base_course_completion_verification_broker',c.oid,'USAGE') OR has_sequence_privilege('ple_base_course_completion_verification_broker',c.oid,'SELECT') OR has_sequence_privilege('ple_base_course_completion_verification_broker',c.oid,'UPDATE'))) THEN RAISE EXCEPTION 'Base Course completion verifier privilege matrix is unsafe'; END IF; IF EXISTS(WITH expected AS(SELECT c.relname relation_name FROM unnest(completion_relations) r(relation_name) JOIN pg_class c ON c.relname=r.relation_name JOIN pg_namespace n ON n.oid=c.relnamespace AND n.nspname='public' WHERE c.relkind IN('r','p') AND c.relrowsecurity),actual AS(SELECT c.relname relation_name FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid WHERE 'ple_base_course_completion_verification_broker'::regrole::oid=ANY(p.polroles) AND p.polname='ple_base_course_completion_select' AND p.polcmd='r' AND p.polpermissive AND pg_get_expr(p.polqual,p.polrelid)='true' AND p.polwithcheck IS NULL) SELECT 1 FROM((SELECT * FROM expected EXCEPT SELECT * FROM actual) UNION ALL(SELECT * FROM actual EXCEPT SELECT * FROM expected)) difference) OR EXISTS(SELECT 1 FROM unnest(completion_relations) r(relation_name) JOIN pg_class c ON c.relname=r.relation_name JOIN pg_namespace n ON n.oid=c.relnamespace AND n.nspname='public' WHERE c.relkind IN('r','p') AND c.relrowsecurity AND NOT c.relforcerowsecurity) THEN RAISE EXCEPTION 'Base Course completion verifier RLS policy matrix is unsafe'; END IF; END $$;
REVOKE INSERT,UPDATE,DELETE ON public.course_invitation,public.course_invitation_delivery,public.course_member,public.tenant_learner_identity,public.course_roster_profile,public.course_roster_state FROM ple_app;
DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_course_invitation_broker') THEN CREATE ROLE ple_course_invitation_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF; END $$;
REVOKE ALL ON SCHEMA public FROM ple_course_invitation_broker; GRANT USAGE ON SCHEMA public TO ple_course_invitation_broker; CREATE POLICY course_invitation_broker_invitation ON public.course_invitation FOR ALL TO ple_course_invitation_broker USING(true) WITH CHECK(true); CREATE POLICY course_invitation_broker_delivery ON public.course_invitation_delivery FOR ALL TO ple_course_invitation_broker USING(true) WITH CHECK(true); CREATE POLICY course_invitation_broker_course ON public.course FOR SELECT TO ple_course_invitation_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_invitation_broker_state ON public.course_roster_state FOR ALL TO ple_course_invitation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_invitation_broker_member ON public.course_member FOR ALL TO ple_course_invitation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_invitation_broker_identity ON public.tenant_learner_identity FOR ALL TO ple_course_invitation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_invitation_broker_profile ON public.course_roster_profile FOR ALL TO ple_course_invitation_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_invitation_broker_domain ON public.course_allowed_email_domain FOR SELECT TO ple_course_invitation_broker USING(tenant_id=public.ple_current_tenant()); GRANT SELECT,INSERT,UPDATE(status,claimed_user_id,claimed_at) ON public.course_invitation TO ple_course_invitation_broker; GRANT SELECT,INSERT,UPDATE(state,updated_at) ON public.course_invitation_delivery TO ple_course_invitation_broker; GRANT SELECT ON public.course,public.course_allowed_email_domain TO ple_course_invitation_broker; GRANT SELECT,INSERT,UPDATE(revision,updated_at) ON public.course_roster_state TO ple_course_invitation_broker; GRANT SELECT,INSERT ON public.course_member,public.tenant_learner_identity,public.course_roster_profile TO ple_course_invitation_broker; GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),public.ple_course_roster_support_actor(character,uuid,text) TO ple_course_invitation_broker;
CREATE FUNCTION public.ple_create_course_invitation_v1(p_tenant uuid,p_session character(64),p_course uuid,p_invitation uuid,p_token bytea,p_normalized text,p_delivery text,p_roster text,p_key text,p_lifetime bigint) RETURNS TABLE(invitation_id uuid,normalized_email text,delivery_email text,roster_id text,invited_by uuid,status text,claimed_user_id uuid,created_at_millis bigint,expires_at_millis bigint,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE a uuid;r bigint;i record; BEGIN IF p_tenant IS NULL OR p_course IS NULL OR p_invitation IS NULL OR p_token IS NULL OR octet_length(p_token)<>32 OR p_normalized IS NULL OR p_delivery IS NULL OR p_roster IS NULL OR p_key IS NULL OR p_lifetime<1 OR p_lifetime>2592000 OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN RAISE EXCEPTION 'invitation arguments are invalid' USING ERRCODE='22023'; END IF; PERFORM 1 FROM public.course WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE; IF NOT FOUND THEN RETURN; END IF; INSERT INTO public.course_roster_state(tenant_id,course_id) VALUES(p_tenant,p_course) ON CONFLICT ON CONSTRAINT course_roster_state_pkey DO NOTHING; SELECT revision INTO r FROM public.course_roster_state WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE; IF NOT FOUND THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF; a:=public.ple_course_roster_support_actor(p_session,p_course,'createInvitation'); IF a IS NULL THEN RETURN; END IF; IF EXISTS(SELECT 1 FROM public.course_allowed_email_domain d WHERE d.tenant_id=p_tenant AND d.course_id=p_course) AND NOT EXISTS(SELECT 1 FROM public.course_allowed_email_domain d WHERE d.tenant_id=p_tenant AND d.course_id=p_course AND (split_part(p_normalized,'@',2)=d.normalized_domain OR(d.include_subdomains AND split_part(p_normalized,'@',2) LIKE '%.'||d.normalized_domain))) THEN RAISE EXCEPTION 'invitation email domain is not permitted' USING ERRCODE='22023'; END IF; UPDATE public.course_invitation AS invitation_row SET status='expired' WHERE invitation_row.tenant_id=p_tenant AND invitation_row.course_id=p_course AND invitation_row.idempotency_key=p_key AND invitation_row.status='pending' AND invitation_row.expires_at<=transaction_timestamp(); SELECT * INTO i FROM public.course_invitation WHERE tenant_id=p_tenant AND course_id=p_course AND idempotency_key=p_key FOR UPDATE; IF FOUND THEN IF i.token_hash IS DISTINCT FROM p_token OR i.normalized_email IS DISTINCT FROM p_normalized OR i.delivery_email IS DISTINCT FROM p_delivery OR i.roster_id IS DISTINCT FROM p_roster OR i.status<>'pending' THEN RAISE EXCEPTION 'invitation idempotency conflicts' USING ERRCODE='55000'; END IF; invitation_id:=i.invitation_id;normalized_email:=i.normalized_email;delivery_email:=i.delivery_email;roster_id:=i.roster_id;invited_by:=i.invited_by;status:=i.status;claimed_user_id:=i.claimed_user_id;created_at_millis:=floor(extract(epoch FROM i.created_at)*1000);expires_at_millis:=floor(extract(epoch FROM i.expires_at)*1000);roster_revision:=r;RETURN NEXT;RETURN; END IF; INSERT INTO public.course_invitation(tenant_id,course_id,invitation_id,token_hash,normalized_email,delivery_email,roster_id,invited_by,idempotency_key,expires_at) VALUES(p_tenant,p_course,p_invitation,p_token,p_normalized,p_delivery,p_roster,a,p_key,transaction_timestamp()+p_lifetime*interval '1 second') RETURNING * INTO i; INSERT INTO public.course_invitation_delivery(tenant_id,course_id,invitation_id,delivery_id) VALUES(p_tenant,p_course,p_invitation,gen_random_uuid()); UPDATE public.course_roster_state SET revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=p_course AND revision=r RETURNING revision INTO roster_revision; IF NOT FOUND THEN RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE='55000'; END IF; invitation_id:=i.invitation_id;normalized_email:=i.normalized_email;delivery_email:=i.delivery_email;roster_id:=i.roster_id;invited_by:=i.invited_by;status:=i.status;claimed_user_id:=i.claimed_user_id;created_at_millis:=floor(extract(epoch FROM i.created_at)*1000);expires_at_millis:=floor(extract(epoch FROM i.expires_at)*1000);RETURN NEXT; END $$;
CREATE FUNCTION public.ple_claim_course_invitation_v1(p_token bytea,p_user uuid,p_normalized text,p_delivery text,p_display text) RETURNS TABLE(tenant_id uuid,course_id uuid,invitation_id uuid,claimed_user_id uuid,student_id uuid,record_id uuid,user_id uuid,member_role text,status text,roster_id text,created_at_millis bigint,revoked_at_millis bigint,display_name text,normalized_email text,delivery_email text,invitation_status text,invitation_claimed_user_id uuid,replayed boolean,delivery_state text,delivery_outcome_code text,delivery_terminal_at_millis bigint,delivery_accepted_at_millis bigint,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE i record;r bigint;s uuid;membership uuid;m record;d record; BEGIN IF p_token IS NULL OR octet_length(p_token)<>32 OR p_user IS NULL OR p_normalized IS NULL OR p_delivery IS NULL OR p_display IS NULL OR p_display<>btrim(p_display) OR char_length(p_display) NOT BETWEEN 1 AND 200 THEN RAISE EXCEPTION 'invitation claim arguments are invalid' USING ERRCODE='22023'; END IF; SELECT * INTO i FROM public.course_invitation ci WHERE ci.token_hash=p_token FOR UPDATE; IF NOT FOUND THEN RETURN; END IF; PERFORM set_config('ple.tenant_id',i.tenant_id::text,true); PERFORM 1 FROM public.course c WHERE c.tenant_id=i.tenant_id AND c.course_id=i.course_id FOR UPDATE; IF NOT FOUND THEN RAISE EXCEPTION 'course invitation aggregate is invalid' USING ERRCODE='55000'; END IF; SELECT rs.revision INTO r FROM public.course_roster_state rs WHERE rs.tenant_id=i.tenant_id AND rs.course_id=i.course_id FOR UPDATE; IF NOT FOUND OR r<1 THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF; IF i.normalized_email IS DISTINCT FROM p_normalized OR i.delivery_email IS DISTINCT FROM p_delivery THEN RAISE EXCEPTION 'invitation email conflicts' USING ERRCODE='42501'; END IF; IF i.status='claimed' AND i.claimed_user_id IS DISTINCT FROM p_user THEN RAISE EXCEPTION 'invitation claimant conflicts' USING ERRCODE='23505'; END IF; IF i.status='claimed' THEN replayed:=true; ELSIF i.status='pending' AND i.expires_at>transaction_timestamp() THEN replayed:=false; INSERT INTO public.tenant_learner_identity AS li(tenant_id,user_id,student_id) VALUES(i.tenant_id,p_user,gen_random_uuid()) ON CONFLICT ON CONSTRAINT tenant_learner_identity_pkey DO NOTHING RETURNING li.student_id INTO s; IF s IS NULL THEN SELECT li.student_id INTO s FROM public.tenant_learner_identity li WHERE li.tenant_id=i.tenant_id AND li.user_id=p_user; END IF; IF s IS NULL THEN RAISE EXCEPTION 'learner identity is unavailable' USING ERRCODE='55000'; END IF; SELECT cm.course_membership_id,cm.role,cm.student_id INTO m FROM public.course_member cm WHERE cm.tenant_id=i.tenant_id AND cm.course_id=i.course_id AND cm.user_id=p_user AND cm.status='active'; IF FOUND THEN IF m.role<>'student' OR m.student_id IS DISTINCT FROM s OR NOT EXISTS(SELECT 1 FROM public.course_roster_profile p WHERE p.tenant_id=i.tenant_id AND p.course_id=i.course_id AND p.course_membership_id=m.course_membership_id AND p.display_name=p_display AND p.roster_email_normalized=p_normalized AND p.roster_email_delivery=p_delivery) THEN RAISE EXCEPTION 'course member conflicts' USING ERRCODE='23505'; END IF; ELSE membership:=gen_random_uuid(); INSERT INTO public.course_member(tenant_id,course_id,course_membership_id,user_id,role,student_id,roster_id,status,joined_at) VALUES(i.tenant_id,i.course_id,membership,p_user,'student',s,i.roster_id,'active',transaction_timestamp()); INSERT INTO public.course_roster_profile(tenant_id,course_id,course_membership_id,display_name,roster_email_normalized,roster_email_delivery) VALUES(i.tenant_id,i.course_id,membership,p_display,p_normalized,p_delivery); END IF; UPDATE public.course_invitation ci SET status='claimed',claimed_user_id=p_user,claimed_at=transaction_timestamp() WHERE ci.tenant_id=i.tenant_id AND ci.course_id=i.course_id AND ci.invitation_id=i.invitation_id AND ci.status='pending'; IF NOT FOUND THEN RAISE EXCEPTION 'invitation claim transition conflicts' USING ERRCODE='55000'; END IF; UPDATE public.course_roster_state rs SET revision=rs.revision+1,updated_at=transaction_timestamp() WHERE rs.tenant_id=i.tenant_id AND rs.course_id=i.course_id AND rs.revision=r RETURNING rs.revision INTO r; IF NOT FOUND OR r<2 THEN RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE='55000'; END IF; ELSE UPDATE public.course_invitation ci SET status='expired' WHERE ci.tenant_id=i.tenant_id AND ci.course_id=i.course_id AND ci.invitation_id=i.invitation_id AND ci.status='pending'; RETURN; END IF; SELECT cm.student_id,cm.course_membership_id AS record_id,cm.user_id,cm.role AS member_role,cm.status,cm.roster_id,floor(extract(epoch FROM cm.joined_at)*1000)::bigint AS created_at_millis,floor(extract(epoch FROM cm.revoked_at)*1000)::bigint AS revoked_at_millis,p.display_name,p.roster_email_normalized AS normalized_email,p.roster_email_delivery AS delivery_email INTO m FROM public.course_member cm JOIN public.course_roster_profile p ON(p.tenant_id,p.course_id,p.course_membership_id)=(cm.tenant_id,cm.course_id,cm.course_membership_id) WHERE cm.tenant_id=i.tenant_id AND cm.course_id=i.course_id AND cm.user_id=p_user AND cm.role='student' AND cm.status='active'; IF NOT FOUND OR m.student_id IS NULL OR m.record_id IS NULL OR m.display_name IS DISTINCT FROM p_display OR m.normalized_email IS DISTINCT FROM p_normalized OR m.delivery_email IS DISTINCT FROM p_delivery OR m.roster_id IS DISTINCT FROM i.roster_id THEN RAISE EXCEPTION 'claimed invitation aggregate is invalid' USING ERRCODE='55000'; END IF; SELECT cd.state,cd.outcome_code,cd.terminal_at,cd.accepted_at INTO d FROM public.course_invitation_delivery cd WHERE cd.tenant_id=i.tenant_id AND cd.course_id=i.course_id AND cd.invitation_id=i.invitation_id FOR UPDATE; IF NOT FOUND OR NOT((d.state='cancelled' AND d.outcome_code='cancelled' AND d.terminal_at IS NOT NULL AND d.accepted_at IS NULL) OR(d.state='accepted_by_provider' AND d.outcome_code='accepted' AND d.terminal_at IS NOT NULL AND d.accepted_at IS NOT NULL) OR(d.state='permanent_failed' AND d.outcome_code='permanent_failure' AND d.terminal_at IS NOT NULL AND d.accepted_at IS NULL) OR(d.state='ambiguous' AND d.outcome_code='ambiguous_transport' AND d.terminal_at IS NOT NULL AND d.accepted_at IS NULL)) THEN RAISE EXCEPTION 'invitation delivery closure is invalid' USING ERRCODE='55000'; END IF; tenant_id:=i.tenant_id;course_id:=i.course_id;invitation_id:=i.invitation_id;claimed_user_id:=p_user;student_id:=m.student_id;record_id:=m.record_id;user_id:=m.user_id;member_role:=m.member_role;status:=m.status;roster_id:=m.roster_id;created_at_millis:=m.created_at_millis;revoked_at_millis:=m.revoked_at_millis;display_name:=m.display_name;normalized_email:=m.normalized_email;delivery_email:=m.delivery_email;invitation_status:='claimed';invitation_claimed_user_id:=p_user;delivery_state:=d.state;delivery_outcome_code:=d.outcome_code;delivery_terminal_at_millis:=CASE WHEN d.terminal_at IS NULL THEN NULL ELSE floor(extract(epoch FROM d.terminal_at)*1000)::bigint END;delivery_accepted_at_millis:=CASE WHEN d.accepted_at IS NULL THEN NULL ELSE floor(extract(epoch FROM d.accepted_at)*1000)::bigint END;roster_revision:=r;RETURN NEXT; END $$;
CREATE FUNCTION public.ple_revoke_course_invitation_v1(p_tenant uuid,p_session character(64),p_course uuid,p_invitation uuid,p_expected bigint) RETURNS TABLE(tenant_id uuid,actor_id uuid,course_id uuid,invitation_id uuid,was_revoked boolean,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE a uuid;r bigint;i record; BEGIN IF p_tenant IS NULL OR p_course IS NULL OR p_invitation IS NULL OR p_expected<1 OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN RAISE EXCEPTION 'invitation revocation arguments are invalid' USING ERRCODE='22023'; END IF; PERFORM 1 FROM public.course WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE; IF NOT FOUND THEN RETURN; END IF; SELECT revision INTO r FROM public.course_roster_state WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE; IF NOT FOUND OR r<>p_expected THEN RAISE EXCEPTION 'course roster revision conflicts' USING ERRCODE='23505'; END IF; a:=public.ple_course_roster_support_actor(p_session,p_course,'revokeInvitation'); IF a IS NULL THEN RETURN; END IF; SELECT * INTO i FROM public.course_invitation WHERE tenant_id=p_tenant AND course_id=p_course AND invitation_id=p_invitation FOR UPDATE; IF NOT FOUND THEN RETURN; END IF; IF i.status='pending' AND i.expires_at<=transaction_timestamp() THEN UPDATE public.course_invitation SET status='expired' WHERE tenant_id=p_tenant AND course_id=p_course AND invitation_id=p_invitation; i.status:='expired'; END IF; IF i.status='revoked' THEN tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;invitation_id:=p_invitation;was_revoked:=true;roster_revision:=r;RETURN NEXT;RETURN; END IF; IF i.status<>'pending' THEN RAISE EXCEPTION 'invitation terminal conflict' USING ERRCODE='23505'; END IF; UPDATE public.course_invitation SET status='revoked' WHERE tenant_id=p_tenant AND course_id=p_course AND invitation_id=p_invitation AND status='pending'; UPDATE public.course_roster_state SET revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=p_course AND revision=r RETURNING revision INTO roster_revision; tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;invitation_id:=p_invitation;was_revoked:=false;RETURN NEXT; END $$;
ALTER FUNCTION public.ple_create_course_invitation_v1(uuid,character,uuid,uuid,bytea,text,text,text,text,bigint) OWNER TO ple_course_invitation_broker; ALTER FUNCTION public.ple_claim_course_invitation_v1(bytea,uuid,text,text,text) OWNER TO ple_course_invitation_broker; ALTER FUNCTION public.ple_revoke_course_invitation_v1(uuid,character,uuid,uuid,bigint) OWNER TO ple_course_invitation_broker; REVOKE ALL ON FUNCTION public.ple_create_course_invitation_v1(uuid,character,uuid,uuid,bytea,text,text,text,text,bigint),public.ple_claim_course_invitation_v1(bytea,uuid,text,text,text),public.ple_revoke_course_invitation_v1(uuid,character,uuid,uuid,bigint) FROM PUBLIC; GRANT EXECUTE ON FUNCTION public.ple_create_course_invitation_v1(uuid,character,uuid,uuid,bytea,text,text,text,text,bigint),public.ple_claim_course_invitation_v1(bytea,uuid,text,text,text),public.ple_revoke_course_invitation_v1(uuid,character,uuid,uuid,bigint) TO ple_app;
DO $$ BEGIN IF EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_course_invitation_broker' AND(rolcanlogin OR rolinherit OR rolbypassrls OR rolsuper)) OR has_table_privilege('ple_app','public.course_invitation','INSERT,UPDATE,DELETE') OR has_table_privilege('ple_app','public.course_invitation_delivery','INSERT,UPDATE,DELETE') OR has_function_privilege('public','public.ple_claim_course_invitation_v1(bytea,uuid,text,text,text)'::regprocedure,'EXECUTE') OR NOT has_function_privilege('ple_app','public.ple_claim_course_invitation_v1(bytea,uuid,text,text,text)'::regprocedure,'EXECUTE') THEN RAISE EXCEPTION 'course invitation authority catalog is unsafe'; END IF; END $$;
GRANT UPDATE(course_id) ON public.course TO ple_course_invitation_broker;
/* roster policy and import aggregate capabilities */ DO $$ BEGIN  IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_course_roster_policy_broker') THEN CREATE ROLE ple_course_roster_policy_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF;  IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='ple_course_roster_import_broker') THEN CREATE ROLE ple_course_roster_import_broker NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS; END IF; END $$; REVOKE ALL ON SCHEMA public FROM ple_course_roster_policy_broker,ple_course_roster_import_broker; GRANT USAGE ON SCHEMA public TO ple_course_roster_policy_broker,ple_course_roster_import_broker; CREATE POLICY course_roster_policy_broker_course ON public.course FOR SELECT TO ple_course_roster_policy_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_policy_broker_state ON public.course_roster_state FOR ALL TO ple_course_roster_policy_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_policy_broker_domain ON public.course_allowed_email_domain FOR ALL TO ple_course_roster_policy_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); GRANT SELECT ON public.course TO ple_course_roster_policy_broker; GRANT SELECT,UPDATE(revision,updated_at,signup_posture) ON public.course_roster_state TO ple_course_roster_policy_broker; GRANT SELECT,INSERT,DELETE ON public.course_allowed_email_domain TO ple_course_roster_policy_broker; GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),public.ple_course_roster_support_actor(character,uuid,text) TO ple_course_roster_policy_broker; CREATE FUNCTION public.ple_replace_course_enrollment_policy_v1(p_tenant uuid,p_session character(64),p_course uuid,p_expected bigint,p_posture text,p_domains jsonb) RETURNS TABLE(tenant_id uuid,actor_id uuid,course_id uuid,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE a uuid;r bigint;v_count integer;v_domain record; BEGIN  IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL OR p_expected<1 OR p_posture NOT IN('invitation_only','permitted_domains') OR jsonb_typeof(p_domains)<>'array' OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN RAISE EXCEPTION 'enrollment policy arguments are invalid' USING ERRCODE='22023'; END IF;  SELECT count(*) INTO v_count FROM jsonb_to_recordset(p_domains) AS x(domain text,include_subdomains boolean);  IF v_count>32 OR (p_posture='permitted_domains' AND v_count=0) OR EXISTS(SELECT 1 FROM jsonb_to_recordset(p_domains) AS x(domain text,include_subdomains boolean) WHERE domain IS NULL OR domain<>lower(btrim(domain)) OR domain='' OR char_length(domain)>253 OR domain !~ '^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+$' OR include_subdomains IS NULL) OR (SELECT count(DISTINCT domain) FROM jsonb_to_recordset(p_domains) AS x(domain text,include_subdomains boolean))<>v_count THEN RAISE EXCEPTION 'enrollment policy shape is invalid' USING ERRCODE='22023'; END IF;  PERFORM 1 FROM public.course WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE; IF NOT FOUND THEN RETURN; END IF;  SELECT revision INTO r FROM public.course_roster_state WHERE tenant_id=p_tenant AND course_id=p_course FOR UPDATE; IF NOT FOUND THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF;  a:=public.ple_course_roster_support_actor(p_session,p_course,'replaceEnrollmentPolicy'); IF a IS NULL THEN RETURN; END IF;  IF r<>p_expected THEN RAISE EXCEPTION 'course roster revision conflicts' USING ERRCODE='23505'; END IF;  IF (SELECT signup_posture FROM public.course_roster_state WHERE tenant_id=p_tenant AND course_id=p_course) = p_posture AND COALESCE((SELECT jsonb_agg(jsonb_build_object('domain',d.normalized_domain,'include_subdomains',d.include_subdomains) ORDER BY d.normalized_domain) FROM public.course_allowed_email_domain d WHERE d.tenant_id=p_tenant AND d.course_id=p_course),'[]'::jsonb)=p_domains THEN tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;roster_revision:=r;RETURN NEXT;RETURN; END IF;  DELETE FROM public.course_allowed_email_domain WHERE tenant_id=p_tenant AND course_id=p_course;  INSERT INTO public.course_allowed_email_domain(tenant_id,course_id,normalized_domain,include_subdomains) SELECT p_tenant,p_course,x.domain,x.include_subdomains FROM jsonb_to_recordset(p_domains) AS x(domain text,include_subdomains boolean);  UPDATE public.course_roster_state SET signup_posture=p_posture,revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=p_course AND revision=r RETURNING revision INTO roster_revision; IF NOT FOUND THEN RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE='55000'; END IF;  tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;RETURN NEXT; END $$; ALTER FUNCTION public.ple_replace_course_enrollment_policy_v1(uuid,character,uuid,bigint,text,jsonb) OWNER TO ple_course_roster_policy_broker; REVOKE ALL ON FUNCTION public.ple_replace_course_enrollment_policy_v1(uuid,character,uuid,bigint,text,jsonb) FROM PUBLIC; GRANT EXECUTE ON FUNCTION public.ple_replace_course_enrollment_policy_v1(uuid,character,uuid,bigint,text,jsonb) TO ple_app; CREATE POLICY course_roster_import_broker_course ON public.course FOR SELECT TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_course_lock ON public.course FOR UPDATE TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_state ON public.course_roster_state FOR ALL TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_import ON public.course_roster_import FOR ALL TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_row ON public.course_roster_import_row FOR ALL TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_member ON public.course_member FOR SELECT TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_profile ON public.course_roster_profile FOR SELECT TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_invitation ON public.course_invitation FOR ALL TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()) WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_delivery ON public.course_invitation_delivery FOR INSERT TO ple_course_roster_import_broker WITH CHECK(tenant_id=public.ple_current_tenant()); CREATE POLICY course_roster_import_broker_domain ON public.course_allowed_email_domain FOR SELECT TO ple_course_roster_import_broker USING(tenant_id=public.ple_current_tenant()); GRANT SELECT,UPDATE(course_id) ON public.course TO ple_course_roster_import_broker; GRANT SELECT ON public.course_member,public.course_roster_profile,public.course_allowed_email_domain TO ple_course_roster_import_broker; GRANT SELECT,UPDATE(revision,updated_at) ON public.course_roster_state TO ple_course_roster_import_broker; GRANT SELECT,INSERT,UPDATE(status,revision,commit_idempotency_key,committed_roster_revision,committed_at),DELETE ON public.course_roster_import TO ple_course_roster_import_broker; GRANT SELECT,INSERT ON public.course_roster_import_row TO ple_course_roster_import_broker; GRANT SELECT,INSERT ON public.course_invitation,public.course_invitation_delivery TO ple_course_roster_import_broker; GRANT EXECUTE ON FUNCTION public.ple_current_tenant(),public.ple_course_roster_support_actor(character,uuid,text) TO ple_course_roster_import_broker; CREATE FUNCTION public.ple_stage_course_roster_import_v1(p_tenant uuid,p_session character(64),p_course uuid,p_import uuid,p_expected bigint,p_digest bytea,p_key text,p_lifetime bigint,p_rows jsonb) RETURNS TABLE(tenant_id uuid,actor_id uuid,course_id uuid,roster_import_id uuid,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE a uuid;r bigint;i record;row_count integer; BEGIN  IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL OR p_import IS NULL OR p_expected<1 OR p_digest IS NULL OR octet_length(p_digest)<>32 OR p_key IS NULL OR p_lifetime<1 OR p_lifetime>86400 OR jsonb_typeof(p_rows)<>'array' OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN RAISE EXCEPTION 'roster import stage arguments are invalid' USING ERRCODE='22023'; END IF;  SELECT count(*) INTO row_count FROM jsonb_to_recordset(p_rows) AS x(row_number integer,normalized_email text,delivery_email text,roster_id text); IF row_count<1 OR row_count>500 OR EXISTS(SELECT 1 FROM jsonb_to_recordset(p_rows) AS x(row_number integer,normalized_email text,delivery_email text,roster_id text) WHERE row_number<2 OR ((normalized_email IS NULL)<>(delivery_email IS NULL)) OR ((normalized_email IS NULL)<>(roster_id IS NULL))) OR (SELECT count(DISTINCT row_number) FROM jsonb_to_recordset(p_rows) AS x(row_number integer,normalized_email text,delivery_email text,roster_id text))<>row_count THEN RAISE EXCEPTION 'roster import rows are invalid' USING ERRCODE='22023'; END IF;  PERFORM 1 FROM public.course AS course_row WHERE course_row.tenant_id=p_tenant AND course_row.course_id=p_course FOR UPDATE; IF NOT FOUND THEN RETURN; END IF; SELECT roster_state.revision INTO r FROM public.course_roster_state AS roster_state WHERE roster_state.tenant_id=p_tenant AND roster_state.course_id=p_course FOR UPDATE; IF NOT FOUND THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF; a:=public.ple_course_roster_support_actor(p_session,p_course,'stageImport'); IF a IS NULL THEN RETURN; END IF; IF r<>p_expected THEN RAISE EXCEPTION 'course roster revision conflicts' USING ERRCODE='23505'; END IF;  DELETE FROM public.course_roster_import AS roster_import_row WHERE roster_import_row.tenant_id=p_tenant AND roster_import_row.course_id=p_course AND roster_import_row.status='preview' AND roster_import_row.expires_at<=transaction_timestamp();  SELECT * INTO i FROM public.course_roster_import AS roster_import_row WHERE roster_import_row.tenant_id=p_tenant AND roster_import_row.course_id=p_course AND roster_import_row.stage_idempotency_key=p_key FOR UPDATE; IF FOUND THEN IF i.normalized_digest IS DISTINCT FROM p_digest OR i.roster_revision<>p_expected THEN RAISE EXCEPTION 'roster import idempotency conflicts' USING ERRCODE='23505'; END IF; tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;roster_import_id:=i.roster_import_id;roster_revision:=r;RETURN NEXT;RETURN; END IF;  INSERT INTO public.course_roster_import(tenant_id,course_id,roster_import_id,normalized_digest,stage_idempotency_key,roster_revision,created_by,expires_at) VALUES(p_tenant,p_course,p_import,p_digest,p_key,p_expected,a,transaction_timestamp()+p_lifetime*interval '1 second');  INSERT INTO public.course_roster_import_row(tenant_id,course_id,roster_import_id,row_number,normalized_email,delivery_email,roster_id,row_status)  SELECT p_tenant,p_course,p_import,x.row_number,x.normalized_email,x.delivery_email,x.roster_id,CASE WHEN x.normalized_email IS NULL THEN 'invalid' WHEN count(*) OVER(PARTITION BY x.normalized_email)>1 OR count(*) OVER(PARTITION BY x.roster_id)>1 THEN 'duplicate' WHEN EXISTS(SELECT 1 FROM public.course_allowed_email_domain d WHERE d.tenant_id=p_tenant AND d.course_id=p_course) AND NOT EXISTS(SELECT 1 FROM public.course_allowed_email_domain d WHERE d.tenant_id=p_tenant AND d.course_id=p_course AND (split_part(x.normalized_email,'@',2)=d.normalized_domain OR(d.include_subdomains AND split_part(x.normalized_email,'@',2) LIKE '%.'||d.normalized_domain))) THEN 'invalid' WHEN EXISTS(SELECT 1 FROM public.course_member m JOIN public.course_roster_profile rp ON(rp.tenant_id,rp.course_id,rp.course_membership_id)=(m.tenant_id,m.course_id,m.course_membership_id) WHERE m.tenant_id=p_tenant AND m.course_id=p_course AND m.role='student' AND m.status='active' AND (rp.roster_email_normalized=x.normalized_email OR m.roster_id=x.roster_id) AND NOT(rp.roster_email_normalized=x.normalized_email AND m.roster_id=x.roster_id)) THEN 'invalid' WHEN EXISTS(SELECT 1 FROM public.course_member m JOIN public.course_roster_profile rp ON(rp.tenant_id,rp.course_id,rp.course_membership_id)=(m.tenant_id,m.course_id,m.course_membership_id) WHERE m.tenant_id=p_tenant AND m.course_id=p_course AND m.role='student' AND m.status='active' AND rp.roster_email_normalized=x.normalized_email AND m.roster_id=x.roster_id) THEN 'already_member' WHEN EXISTS(SELECT 1 FROM public.course_invitation ci WHERE ci.tenant_id=p_tenant AND ci.course_id=p_course AND ci.status='pending' AND ci.expires_at>transaction_timestamp() AND (ci.normalized_email=x.normalized_email OR ci.roster_id=x.roster_id) AND NOT(ci.normalized_email=x.normalized_email AND ci.roster_id=x.roster_id)) THEN 'invalid' WHEN EXISTS(SELECT 1 FROM public.course_invitation ci WHERE ci.tenant_id=p_tenant AND ci.course_id=p_course AND ci.status='pending' AND ci.expires_at>transaction_timestamp() AND ci.normalized_email=x.normalized_email AND ci.roster_id=x.roster_id) THEN 'already_pending' ELSE 'ready_to_invite' END FROM jsonb_to_recordset(p_rows) AS x(row_number integer,normalized_email text,delivery_email text,roster_id text);  tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;roster_import_id:=p_import;roster_revision:=r;RETURN NEXT; END $$; CREATE FUNCTION public.ple_commit_course_roster_import_v1(p_tenant uuid,p_session character(64),p_course uuid,p_import uuid,p_expected bigint,p_key text,p_bindings jsonb) RETURNS TABLE(tenant_id uuid,actor_id uuid,course_id uuid,roster_import_id uuid,import_revision bigint,roster_revision bigint) LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path TO 'pg_catalog','public',pg_temp AS $$ DECLARE a uuid;r bigint;i record;binding_count integer;ready_count integer; BEGIN  IF p_tenant IS NULL OR p_session IS NULL OR p_course IS NULL OR p_import IS NULL OR p_expected<1 OR p_key IS NULL OR jsonb_typeof(p_bindings)<>'array' OR p_tenant IS DISTINCT FROM public.ple_current_tenant() THEN RAISE EXCEPTION 'roster import commit arguments are invalid' USING ERRCODE='22023'; END IF;  PERFORM 1 FROM public.course AS course_row WHERE course_row.tenant_id=p_tenant AND course_row.course_id=p_course FOR UPDATE; IF NOT FOUND THEN RETURN; END IF; SELECT roster_state.revision INTO r FROM public.course_roster_state AS roster_state WHERE roster_state.tenant_id=p_tenant AND roster_state.course_id=p_course FOR UPDATE; IF NOT FOUND THEN RAISE EXCEPTION 'course roster aggregate is invalid' USING ERRCODE='55000'; END IF; a:=public.ple_course_roster_support_actor(p_session,p_course,'commitImport'); IF a IS NULL THEN RETURN; END IF; SELECT * INTO i FROM public.course_roster_import AS roster_import_row WHERE roster_import_row.tenant_id=p_tenant AND roster_import_row.course_id=p_course AND roster_import_row.roster_import_id=p_import FOR UPDATE; IF NOT FOUND THEN RETURN; END IF;  IF i.status='committed' THEN IF i.commit_idempotency_key IS DISTINCT FROM p_key THEN RAISE EXCEPTION 'roster import commit idempotency conflicts' USING ERRCODE='23505'; END IF; tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;roster_import_id:=p_import;import_revision:=i.revision;roster_revision:=i.committed_roster_revision;RETURN NEXT;RETURN; END IF;  IF i.status<>'preview' OR i.expires_at<=transaction_timestamp() OR i.revision<>p_expected OR i.roster_revision<>r THEN RAISE EXCEPTION 'roster import commit conflicts' USING ERRCODE='23505'; END IF;  SELECT count(*) INTO ready_count FROM public.course_roster_import_row WHERE tenant_id=p_tenant AND course_id=p_course AND roster_import_id=p_import AND row_status='ready_to_invite'; SELECT count(*) INTO binding_count FROM jsonb_to_recordset(p_bindings) AS b(row_number integer,token_hex text,idempotency_key text,lifetime bigint); IF binding_count<>ready_count OR binding_count<>(SELECT count(DISTINCT row_number) FROM jsonb_to_recordset(p_bindings) AS b(row_number integer,token_hex text,idempotency_key text,lifetime bigint)) OR EXISTS(SELECT 1 FROM jsonb_to_recordset(p_bindings) AS b(row_number integer,token_hex text,idempotency_key text,lifetime bigint) WHERE row_number IS NULL OR token_hex !~ '^[0-9a-f]{64}$' OR idempotency_key IS NULL OR lifetime<1 OR lifetime>2592000) OR EXISTS((SELECT row_number FROM public.course_roster_import_row WHERE tenant_id=p_tenant AND course_id=p_course AND roster_import_id=p_import AND row_status='ready_to_invite') EXCEPT (SELECT row_number FROM jsonb_to_recordset(p_bindings) AS b(row_number integer,token_hex text,idempotency_key text,lifetime bigint))) THEN RAISE EXCEPTION 'roster import invitation bindings are invalid' USING ERRCODE='22023'; END IF;  INSERT INTO public.course_invitation(tenant_id,course_id,invitation_id,token_hash,normalized_email,delivery_email,roster_id,invited_by,idempotency_key,expires_at,roster_import_id,roster_import_row_number) SELECT p_tenant,p_course,gen_random_uuid(),decode(b.token_hex,'hex'),rr.normalized_email,rr.delivery_email,rr.roster_id,a,b.idempotency_key,transaction_timestamp()+b.lifetime*interval '1 second',p_import,rr.row_number FROM public.course_roster_import_row rr JOIN jsonb_to_recordset(p_bindings) AS b(row_number integer,token_hex text,idempotency_key text,lifetime bigint) USING(row_number) WHERE rr.tenant_id=p_tenant AND rr.course_id=p_course AND rr.roster_import_id=p_import AND rr.row_status='ready_to_invite';  INSERT INTO public.course_invitation_delivery(tenant_id,course_id,invitation_id,delivery_id) SELECT p_tenant,p_course,invitation_id,gen_random_uuid() FROM public.course_invitation WHERE tenant_id=p_tenant AND course_id=p_course AND roster_import_id=p_import;  UPDATE public.course_roster_state SET revision=revision+1,updated_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=p_course AND revision=r RETURNING revision INTO roster_revision; IF NOT FOUND THEN RAISE EXCEPTION 'course roster revision is unavailable' USING ERRCODE='55000'; END IF;  UPDATE public.course_roster_import SET status='committed',revision=revision+1,commit_idempotency_key=p_key,committed_roster_revision=roster_revision,committed_at=transaction_timestamp() WHERE tenant_id=p_tenant AND course_id=p_course AND roster_import_id=p_import AND status='preview' RETURNING revision INTO import_revision; IF NOT FOUND THEN RAISE EXCEPTION 'roster import transition is unavailable' USING ERRCODE='55000'; END IF;  tenant_id:=p_tenant;actor_id:=a;course_id:=p_course;roster_import_id:=p_import;RETURN NEXT; END $$; ALTER FUNCTION public.ple_stage_course_roster_import_v1(uuid,character,uuid,uuid,bigint,bytea,text,bigint,jsonb) OWNER TO ple_course_roster_import_broker; ALTER FUNCTION public.ple_commit_course_roster_import_v1(uuid,character,uuid,uuid,bigint,text,jsonb) OWNER TO ple_course_roster_import_broker; REVOKE ALL ON FUNCTION public.ple_stage_course_roster_import_v1(uuid,character,uuid,uuid,bigint,bytea,text,bigint,jsonb),public.ple_commit_course_roster_import_v1(uuid,character,uuid,uuid,bigint,text,jsonb) FROM PUBLIC; GRANT EXECUTE ON FUNCTION public.ple_stage_course_roster_import_v1(uuid,character,uuid,uuid,bigint,bytea,text,bigint,jsonb),public.ple_commit_course_roster_import_v1(uuid,character,uuid,uuid,bigint,text,jsonb) TO ple_app; REVOKE INSERT,UPDATE,DELETE ON public.course_allowed_email_domain,public.course_roster_import,public.course_roster_import_row FROM ple_app; DO $$ BEGIN  IF EXISTS(SELECT 1 FROM pg_roles WHERE rolname IN('ple_course_roster_policy_broker','ple_course_roster_import_broker') AND(rolcanlogin OR rolinherit OR rolbypassrls OR rolsuper)) OR has_table_privilege('ple_app','public.course_allowed_email_domain','INSERT,UPDATE,DELETE') OR has_table_privilege('ple_app','public.course_roster_import','INSERT,UPDATE,DELETE') OR has_table_privilege('ple_app','public.course_roster_import_row','INSERT,UPDATE,DELETE') OR has_function_privilege('public','public.ple_replace_course_enrollment_policy_v1(uuid,character,uuid,bigint,text,jsonb)'::regprocedure,'EXECUTE') OR has_function_privilege('public','public.ple_stage_course_roster_import_v1(uuid,character,uuid,uuid,bigint,bytea,text,bigint,jsonb)'::regprocedure,'EXECUTE') OR has_function_privilege('public','public.ple_commit_course_roster_import_v1(uuid,character,uuid,uuid,bigint,text,jsonb)'::regprocedure,'EXECUTE') THEN RAISE EXCEPTION 'roster policy/import authority catalog is unsafe'; END IF; END $$; COMMIT;
