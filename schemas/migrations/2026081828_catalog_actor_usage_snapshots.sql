-- WP-PROF-D1: bounded actor-only usage snapshots for stable catalog cursors.
BEGIN;

CREATE TABLE public.catalog_usage_snapshot (
    snapshot_token character(64) PRIMARY KEY, tenant_id uuid NOT NULL,
    actor_user_id uuid NOT NULL, usage_digest bytea NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    expires_at timestamptz NOT NULL, row_count integer NOT NULL,
    CHECK (snapshot_token ~ '^[0-9a-f]{64}$' AND octet_length(usage_digest)=32),
    CHECK (row_count BETWEEN 0 AND 5000 AND expires_at > created_at)
);
CREATE TABLE public.catalog_usage_snapshot_row (
    snapshot_token character(64) NOT NULL REFERENCES public.catalog_usage_snapshot ON DELETE CASCADE,
    course_reference integer NOT NULL, course_title text NOT NULL,
    problem_id uuid NOT NULL, version_id uuid NOT NULL,
    assignment_count bigint NOT NULL, fixed_reference_count bigint NOT NULL,
    pool_candidate_count bigint NOT NULL,
    PRIMARY KEY(snapshot_token,course_reference,problem_id,version_id)
);
CREATE INDEX catalog_usage_snapshot_reuse_idx ON public.catalog_usage_snapshot
    (tenant_id,actor_user_id,usage_digest,expires_at DESC);
CREATE INDEX catalog_usage_snapshot_expiry_idx ON public.catalog_usage_snapshot(expires_at);
ALTER TABLE public.catalog_usage_snapshot ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.catalog_usage_snapshot FORCE ROW LEVEL SECURITY;
ALTER TABLE public.catalog_usage_snapshot_row ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.catalog_usage_snapshot_row FORCE ROW LEVEL SECURITY;
CREATE POLICY catalog_usage_snapshot_broker ON public.catalog_usage_snapshot
    TO ple_catalog_usage_broker USING(true) WITH CHECK(true);
CREATE POLICY catalog_usage_snapshot_row_broker ON public.catalog_usage_snapshot_row
    TO ple_catalog_usage_broker USING(true) WITH CHECK(true);
GRANT SELECT,INSERT,DELETE ON public.catalog_usage_snapshot TO ple_catalog_usage_broker;
GRANT SELECT,INSERT ON public.catalog_usage_snapshot_row TO ple_catalog_usage_broker;
GRANT EXECUTE ON FUNCTION public.ple_course_records_accessible(uuid,uuid)
    TO ple_catalog_usage_broker;

CREATE FUNCTION public.ple_begin_instructor_catalog_usage_snapshot(
    p_tenant uuid,p_session character(64),p_ttl_seconds integer,p_max_rows integer
) RETURNS TABLE(snapshot_token character(64),expires_at timestamptz,row_count integer)
LANGUAGE plpgsql SECURITY DEFINER SET search_path TO 'pg_catalog','public' AS $$
DECLARE v_actor uuid; v_rows jsonb; v_digest bytea; v_token character(64);
BEGIN
    IF p_tenant IS DISTINCT FROM public.ple_current_tenant()
       OR p_ttl_seconds NOT BETWEEN 30 AND 900 OR p_max_rows NOT BETWEEN 1 AND 5000 THEN
        RAISE EXCEPTION 'invalid usage snapshot request' USING ERRCODE='22023';
    END IF;
    SELECT subject.user_id INTO v_actor
      FROM public.ple_catalog_discovery_actor(p_session,p_tenant) subject;
    IF NOT FOUND THEN RAISE EXCEPTION 'usage snapshot actor is unauthorized' USING ERRCODE='42501'; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_tenant::text||v_actor::text,0));
    DELETE FROM public.catalog_usage_snapshot expired WHERE expired.snapshot_token IN (
        SELECT s.snapshot_token FROM public.catalog_usage_snapshot s
         WHERE s.expires_at<=statement_timestamp() ORDER BY s.expires_at LIMIT 32);
    WITH ref AS (
        SELECT i.tenant_id,i.assignment_id,i.problem_id,i.version_id,1 fixed,0 pool
          FROM public.assignment_item i WHERE i.delivery_state='active'
        UNION ALL SELECT c.tenant_id,c.assignment_id,c.problem_id,c.version_id,0,1
          FROM public.assignment_selection_candidate c WHERE c.delivery_state='active'
    ), rows AS (
        SELECT c.public_id course_reference,c.title course_title,r.problem_id,r.version_id,
               count(DISTINCT a.assignment_id)::bigint assignment_count,
               sum(r.fixed)::bigint fixed_reference_count,sum(r.pool)::bigint pool_candidate_count
          FROM public.course_member m JOIN public.course c USING(tenant_id,course_id)
          JOIN public.assignment a USING(tenant_id,course_id) JOIN ref r USING(tenant_id,assignment_id)
         WHERE m.tenant_id=p_tenant AND m.user_id=v_actor AND m.role='instructor' AND m.status='active'
           AND public.ple_course_records_accessible(c.tenant_id,c.course_id)
         GROUP BY c.public_id,c.title,r.problem_id,r.version_id ORDER BY c.public_id,r.problem_id,r.version_id
    ) SELECT coalesce(jsonb_agg(to_jsonb(rows) ORDER BY course_reference,problem_id,version_id),
        '[]'::jsonb) INTO v_rows FROM rows;
    IF jsonb_array_length(v_rows)>p_max_rows THEN RAISE EXCEPTION 'usage snapshot exceeds bound' USING ERRCODE='54000'; END IF;
    v_digest:=digest(convert_to(v_rows::text,'UTF8'),'sha256');
    SELECT s.snapshot_token INTO v_token FROM public.catalog_usage_snapshot s
     WHERE s.tenant_id=p_tenant AND s.actor_user_id=v_actor AND s.usage_digest=v_digest
       AND s.expires_at>statement_timestamp() ORDER BY s.expires_at DESC LIMIT 1;
    IF NOT FOUND THEN
        IF (SELECT count(*) FROM public.catalog_usage_snapshot s WHERE s.tenant_id=p_tenant
              AND s.actor_user_id=v_actor AND s.expires_at>statement_timestamp())>=8 THEN
            -- Fresh searches win: the displaced continuation receives the
            -- same recoverable invalid-snapshot outcome as expiry.
            DELETE FROM public.catalog_usage_snapshot evicted WHERE evicted.snapshot_token=(
                SELECT s.snapshot_token FROM public.catalog_usage_snapshot s
                 WHERE s.tenant_id=p_tenant AND s.actor_user_id=v_actor
                   AND s.expires_at>statement_timestamp()
                 ORDER BY s.created_at,s.snapshot_token LIMIT 1);
        END IF;
        v_token:=encode(gen_random_bytes(32),'hex');
        INSERT INTO public.catalog_usage_snapshot VALUES
            (v_token,p_tenant,v_actor,v_digest,transaction_timestamp(),
             transaction_timestamp()+make_interval(secs=>p_ttl_seconds),jsonb_array_length(v_rows));
        INSERT INTO public.catalog_usage_snapshot_row
        SELECT v_token,x.* FROM jsonb_to_recordset(v_rows) x(course_reference integer,
            course_title text,problem_id uuid,version_id uuid,assignment_count bigint,
            fixed_reference_count bigint,pool_candidate_count bigint);
    END IF;
    RETURN QUERY SELECT s.snapshot_token,s.expires_at,s.row_count
      FROM public.catalog_usage_snapshot s WHERE s.snapshot_token=v_token;
END $$;
ALTER FUNCTION public.ple_begin_instructor_catalog_usage_snapshot(uuid,character,integer,integer)
    OWNER TO ple_catalog_usage_broker;
REVOKE ALL ON FUNCTION public.ple_begin_instructor_catalog_usage_snapshot(uuid,character,integer,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_begin_instructor_catalog_usage_snapshot(uuid,character,integer,integer) TO ple_app;

CREATE FUNCTION public.ple_instructor_catalog_usage_snapshot_rows(
    p_tenant uuid,p_session character(64),p_snapshot character(64)
) RETURNS SETOF public.catalog_usage_snapshot_row LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog','public' AS $$
DECLARE v_actor uuid;
BEGIN
    SELECT actor.user_id INTO v_actor FROM public.catalog_usage_snapshot snapshot
     JOIN public.ple_catalog_discovery_actor(p_session,p_tenant) actor
        ON actor.user_id=snapshot.actor_user_id
     WHERE snapshot.snapshot_token=p_snapshot AND snapshot.tenant_id=p_tenant
       AND p_tenant=public.ple_current_tenant() AND snapshot.expires_at>statement_timestamp();
    IF NOT FOUND THEN
        RAISE EXCEPTION 'usage snapshot is invalid or expired' USING ERRCODE='22023';
    END IF;
    -- A nonempty snapshot promises only courses that the actor directly
    -- instructs.  Do not turn membership revocation into a misleading empty
    -- continuation; zero-row Sysadmin snapshots have no named-course edge and
    -- remain valid aggregate-only cursors.
    IF EXISTS (
        SELECT 1
          FROM public.catalog_usage_snapshot_row AS row
          LEFT JOIN public.course AS course
            ON course.tenant_id = p_tenant
           AND course.public_id = row.course_reference
          LEFT JOIN public.course_member AS membership
            ON membership.tenant_id = course.tenant_id
           AND membership.course_id = course.course_id
           AND membership.user_id = v_actor
           AND membership.role = 'instructor'
           AND membership.status = 'active'
         WHERE row.snapshot_token = p_snapshot
           AND membership.course_membership_id IS NULL
    ) THEN
        RAISE EXCEPTION 'usage snapshot is invalid or expired' USING ERRCODE='22023';
    END IF;
    RETURN QUERY SELECT row.* FROM public.catalog_usage_snapshot snapshot
      JOIN public.catalog_usage_snapshot_row row USING(snapshot_token)
      JOIN public.course course ON course.tenant_id=snapshot.tenant_id AND course.public_id=row.course_reference
      JOIN public.course_member member ON member.tenant_id=course.tenant_id
        AND member.course_id=course.course_id AND member.user_id=v_actor
        AND member.role='instructor' AND member.status='active'
     WHERE snapshot.snapshot_token=p_snapshot AND snapshot.tenant_id=p_tenant
       AND public.ple_course_records_accessible(course.tenant_id,course.course_id)
     ORDER BY row.course_reference,row.problem_id,row.version_id;
END
$$;
ALTER FUNCTION public.ple_instructor_catalog_usage_snapshot_rows(uuid,character,character)
    OWNER TO ple_catalog_usage_broker;
REVOKE ALL ON FUNCTION public.ple_instructor_catalog_usage_snapshot_rows(uuid,character,character) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.ple_instructor_catalog_usage_snapshot_rows(uuid,character,character) TO ple_app;

DO $$ BEGIN
    IF has_function_privilege('public',
        'public.ple_begin_instructor_catalog_usage_snapshot(uuid,character,integer,integer)'::regprocedure,
        'EXECUTE') OR has_function_privilege('public',
        'public.ple_instructor_catalog_usage_snapshot_rows(uuid,character,character)'::regprocedure,
        'EXECUTE') OR EXISTS (
            SELECT 1 FROM pg_catalog.pg_roles AS role_row
             WHERE role_row.rolname IN ('ple_statistics_broker','ple_catalog_usage_broker')
               AND (role_row.rolcanlogin OR role_row.rolsuper OR role_row.rolcreatedb
                    OR role_row.rolcreaterole OR role_row.rolinherit
                    OR role_row.rolreplication OR role_row.rolbypassrls)
       ) OR EXISTS (
            SELECT 1 FROM pg_catalog.pg_auth_members AS membership
             WHERE membership.member IN ('ple_statistics_broker'::regrole,
                                         'ple_catalog_usage_broker'::regrole)
                OR membership.roleid IN ('ple_statistics_broker'::regrole,
                                         'ple_catalog_usage_broker'::regrole)
       )
       OR NOT has_function_privilege('ple_app',
            'public.ple_begin_instructor_catalog_usage_snapshot(uuid,character,integer,integer)'::regprocedure,
            'EXECUTE') OR NOT has_function_privilege('ple_app',
            'public.ple_instructor_catalog_usage_snapshot_rows(uuid,character,character)'::regprocedure,
            'EXECUTE') OR EXISTS (
            SELECT 1 FROM unnest(ARRAY['catalog_usage_snapshot','catalog_usage_snapshot_row']) r(name)
            CROSS JOIN unnest(ARRAY['SELECT','INSERT','UPDATE','DELETE','TRUNCATE','REFERENCES','TRIGGER']) p(privilege)
             WHERE has_table_privilege('ple_app',format('public.%I',r.name),p.privilege)
       ) OR EXISTS (
            SELECT 1 FROM pg_attribute a JOIN pg_class c ON c.oid=a.attrelid
            JOIN pg_namespace n ON n.oid=c.relnamespace
             WHERE n.nspname='public' AND c.relname IN
                 ('catalog_usage_snapshot','catalog_usage_snapshot_row')
               AND a.attnum>0 AND NOT a.attisdropped
               AND has_column_privilege('ple_app',c.oid,a.attnum,'SELECT')
       ) OR EXISTS (
            SELECT 1 FROM pg_class sequence JOIN pg_depend dependency
              ON dependency.objid=sequence.oid AND dependency.deptype IN ('a','i')
            JOIN pg_class relation ON relation.oid=dependency.refobjid
             WHERE sequence.relkind='S' AND relation.relname IN
                 ('catalog_usage_snapshot','catalog_usage_snapshot_row')
               AND (has_sequence_privilege('ple_app',sequence.oid,'USAGE')
                    OR has_sequence_privilege('ple_app',sequence.oid,'SELECT')
                    OR has_sequence_privilege('ple_app',sequence.oid,'UPDATE'))
       ) OR EXISTS (
            WITH expected(role_name,relation_name,privilege_type) AS (
                VALUES
                    ('ple_statistics_broker','assignment','SELECT'),
                    ('ple_statistics_broker','assignment_run','SELECT'),
                    ('ple_statistics_broker','assignment_run_item','SELECT'),
                    ('ple_statistics_broker','catalog_discovery_course_fingerprint_receipt','INSERT'),
                    ('ple_statistics_broker','catalog_discovery_course_fingerprint_receipt','SELECT'),
                    ('ple_statistics_broker','catalog_discovery_evidence_revision','INSERT'),
                    ('ple_statistics_broker','catalog_discovery_evidence_revision','SELECT'),
                    ('ple_statistics_broker','catalog_discovery_learner_fingerprint_receipt','INSERT'),
                    ('ple_statistics_broker','catalog_discovery_learner_fingerprint_receipt','SELECT'),
                    ('ple_statistics_broker','catalog_search_document','SELECT'),
                    ('ple_statistics_broker','catalog_tenant_grant','SELECT'),
                    ('ple_statistics_broker','enrollment','SELECT'),
                    ('ple_statistics_broker','problem_version','SELECT'),
                    ('ple_statistics_broker','question_attempt','SELECT'),
                    ('ple_statistics_broker','question_statistics_aggregate','INSERT'),
                    ('ple_statistics_broker','question_statistics_aggregate','SELECT'),
                    ('ple_statistics_broker','question_statistics_aggregate','UPDATE'),
                    ('ple_statistics_broker','question_statistics_contribution_receipt','INSERT'),
                    ('ple_statistics_broker','question_statistics_contribution_receipt','SELECT'),
                    ('ple_statistics_broker','submission_evaluation','SELECT'),
                    ('ple_catalog_usage_broker','assignment','SELECT'),
                    ('ple_catalog_usage_broker','assignment_item','SELECT'),
                    ('ple_catalog_usage_broker','assignment_selection_candidate','SELECT'),
                    ('ple_catalog_usage_broker','catalog_search_document','SELECT'),
                    ('ple_catalog_usage_broker','catalog_tenant_grant','SELECT'),
                    ('ple_catalog_usage_broker','catalog_usage_snapshot','DELETE'),
                    ('ple_catalog_usage_broker','catalog_usage_snapshot','INSERT'),
                    ('ple_catalog_usage_broker','catalog_usage_snapshot','SELECT'),
                    ('ple_catalog_usage_broker','catalog_usage_snapshot_row','INSERT'),
                    ('ple_catalog_usage_broker','catalog_usage_snapshot_row','SELECT'),
                    ('ple_catalog_usage_broker','course','SELECT'),
                    ('ple_catalog_usage_broker','course_member','SELECT'),
                    ('ple_catalog_usage_broker','problem_version','SELECT')
            ), actual AS (
                SELECT role_row.rolname::text, relation_row.relname::text, privilege.privilege_type
                  FROM pg_catalog.pg_class AS relation_row
                  JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=relation_row.relnamespace
                  CROSS JOIN LATERAL aclexplode(
                      COALESCE(relation_row.relacl,acldefault('r',relation_row.relowner))
                  ) AS privilege
                  JOIN pg_catalog.pg_roles AS role_row ON role_row.oid=privilege.grantee
                 WHERE namespace.nspname='public' AND relation_row.relkind IN ('r','p')
                   AND role_row.rolname IN ('ple_statistics_broker','ple_catalog_usage_broker')
            )
            SELECT 1 FROM (
                (SELECT * FROM expected EXCEPT SELECT * FROM actual)
                UNION ALL
                (SELECT * FROM actual EXCEPT SELECT * FROM expected)
            ) AS privilege_drift
       ) OR EXISTS (
            WITH expected(role_name,relation_name,column_name,privilege_type) AS (
                VALUES
                    ('ple_statistics_broker','catalog_search_document','quality_signal','UPDATE'),
                    ('ple_statistics_broker','catalog_search_document','updated_at','UPDATE')
            ), actual AS (
                SELECT role_row.rolname::text, relation_row.relname::text,
                       attribute.attname::text, privilege.privilege_type
                  FROM pg_catalog.pg_attribute AS attribute
                  JOIN pg_catalog.pg_class AS relation_row ON relation_row.oid=attribute.attrelid
                  JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=relation_row.relnamespace
                  CROSS JOIN LATERAL aclexplode(attribute.attacl) AS privilege
                  JOIN pg_catalog.pg_roles AS role_row ON role_row.oid=privilege.grantee
                 WHERE namespace.nspname='public' AND relation_row.relkind IN ('r','p')
                   AND attribute.attnum>0 AND NOT attribute.attisdropped
                   AND role_row.rolname IN ('ple_statistics_broker','ple_catalog_usage_broker')
            )
            SELECT 1 FROM (
                (SELECT * FROM expected EXCEPT SELECT * FROM actual)
                UNION ALL
                (SELECT * FROM actual EXCEPT SELECT * FROM expected)
            ) AS column_privilege_drift
       ) OR EXISTS (
            WITH expected(role_name,sequence_name,privilege_type) AS (
                VALUES ('ple_statistics_broker','catalog_search_publication_sequence','USAGE')
            ), actual AS (
                SELECT role_row.rolname::text, sequence_row.relname::text, privilege.privilege_type
                  FROM pg_catalog.pg_class AS sequence_row
                  JOIN pg_catalog.pg_namespace AS namespace ON namespace.oid=sequence_row.relnamespace
                  CROSS JOIN LATERAL aclexplode(
                      COALESCE(sequence_row.relacl,acldefault('r',sequence_row.relowner))
                  ) AS privilege
                  JOIN pg_catalog.pg_roles AS role_row ON role_row.oid=privilege.grantee
                 WHERE namespace.nspname='public' AND sequence_row.relkind='S'
                   AND role_row.rolname IN ('ple_statistics_broker','ple_catalog_usage_broker')
            )
            SELECT 1 FROM (
                (SELECT * FROM expected EXCEPT SELECT * FROM actual)
                UNION ALL
                (SELECT * FROM actual EXCEPT SELECT * FROM expected)
            ) AS sequence_privilege_drift
       ) THEN
        RAISE EXCEPTION 'catalog usage snapshot capability matrix is unsafe';
    END IF;
END $$;

COMMIT;
