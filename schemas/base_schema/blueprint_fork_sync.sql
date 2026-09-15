-- Private immutable baselines for deliberate Blueprint Course fork updates.
--
-- The lineage module owns the one exact source origin.  This module snapshots
--only the four selectable units at that origin: names, each whole Blueprint
--Assessment, and the ordered Assessment list.  There is deliberately no
--application-facing read path here.  A later typed Store and server operation
--must authorize any source inspection and derive `source_unavailable` without
--enumerating a now-Private source.

SET LOCAL ROLE ple_data_owner;

-- The private baseline owner may read only the immutable Blueprint facts needed
--by its SECURITY DEFINER snapshot triggers.  It is a NOLOGIN capability, and
--the application role receives no direct access to these source tables.
GRANT USAGE ON SCHEMA ple_private TO ple_data_owner;
GRANT REFERENCES (reference_number) ON ple_data.blueprint_course TO ple_private_owner;
GRANT REFERENCES (blueprint_course_reference_number, blueprint_revision_number)
    ON ple_data.blueprint_course_revision TO ple_private_owner;
GRANT REFERENCES (blueprint_course_reference_number)
    ON ple_data.blueprint_course_fork TO ple_private_owner;
GRANT SELECT ON ple_data.blueprint_course, ple_data.blueprint_course_revision,
    ple_data.blueprint_course_fork TO ple_private_owner;
CREATE POLICY blueprint_course_fork_sync_private_owner_read
    ON ple_data.blueprint_course FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY blueprint_revision_fork_sync_private_owner_read
    ON ple_data.blueprint_course_revision FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY blueprint_fork_sync_private_owner_read
    ON ple_data.blueprint_course_fork FOR SELECT TO ple_private_owner USING (true);

-- C412's source row is the single origin for every later unit baseline.  It
--must remain a fact even though later selected applications may append a
--baseline that points at a newer Revision of that same source lineage.
CREATE FUNCTION ple_data.reject_blueprint_course_fork_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_data AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint Course fork origin is immutable';
END
$$;
CREATE TRIGGER blueprint_course_fork_origin_is_immutable
BEFORE UPDATE OR DELETE ON ple_data.blueprint_course_fork
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_blueprint_course_fork_change();
REVOKE ALL ON FUNCTION ple_data.reject_blueprint_course_fork_change() FROM PUBLIC;

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.blueprint_fork_sync_baseline (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course_fork (blueprint_course_reference_number),
    baseline_sequence bigint NOT NULL CHECK (baseline_sequence > 0),
    unit_kind text NOT NULL CHECK (unit_kind IN (
        'short_name', 'long_name', 'assessment', 'ordered_assessment_list'
    )),
    blueprint_assessment_reference uuid,
    source_blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    source_blueprint_revision_number bigint NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    baseline_content jsonb NOT NULL,
    recorded_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_fork_sync_baseline_source_revision_fk FOREIGN KEY (
        source_blueprint_course_reference_number, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    ),
    CONSTRAINT blueprint_fork_sync_baseline_unit_shape CHECK (
        (unit_kind = 'assessment' AND blueprint_assessment_reference IS NOT NULL)
        OR (unit_kind <> 'assessment' AND blueprint_assessment_reference IS NULL)
    ),
    CONSTRAINT blueprint_fork_sync_baseline_identity UNIQUE NULLS NOT DISTINCT (
        blueprint_course_reference_number, unit_kind,
        blueprint_assessment_reference, baseline_sequence
    )
);

ALTER TABLE ple_private.blueprint_fork_sync_baseline ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.blueprint_fork_sync_baseline FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_private.blueprint_fork_sync_baseline FROM PUBLIC;
CREATE POLICY blueprint_fork_sync_baseline_private_owner_access
    ON ple_private.blueprint_fork_sync_baseline FOR ALL TO ple_private_owner
    USING (true) WITH CHECK (true);

-- A baseline is historical fact.  A later selected application appends the
--next sequence for precisely one unit; it never overwrites this source fact.
CREATE FUNCTION ple_private.reject_blueprint_fork_sync_baseline_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Blueprint fork sync baselines are immutable';
END
$$;

CREATE FUNCTION ple_private.validate_blueprint_fork_sync_baseline()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    v_origin ple_data.blueprint_course_fork%ROWTYPE;
    v_next_sequence bigint;
BEGIN
    -- Serialize one unit's append-only history.  The unique constraint remains
    --the final race backstop if a caller bypasses the expected transaction flow.
    PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(
        pg_catalog.format('ple:blueprint-fork-sync:%s:%s:%s',
            NEW.blueprint_course_reference_number, NEW.unit_kind,
            COALESCE(NEW.blueprint_assessment_reference::text, '')), 0));
    SELECT * INTO v_origin
      FROM ple_data.blueprint_course_fork AS origin
     WHERE origin.blueprint_course_reference_number = NEW.blueprint_course_reference_number;
    IF NOT FOUND
       OR NEW.source_blueprint_course_reference_number
          <> v_origin.source_blueprint_course_reference_number THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint fork sync baseline must use its immutable source lineage';
    END IF;
    SELECT COALESCE(MAX(existing.baseline_sequence), 0) + 1
      INTO v_next_sequence
      FROM ple_private.blueprint_fork_sync_baseline AS existing
     WHERE existing.blueprint_course_reference_number = NEW.blueprint_course_reference_number
       AND existing.unit_kind = NEW.unit_kind
       AND existing.blueprint_assessment_reference IS NOT DISTINCT FROM
           NEW.blueprint_assessment_reference;
    IF NEW.baseline_sequence <> v_next_sequence THEN
        RAISE EXCEPTION USING ERRCODE = '23514',
            MESSAGE = 'Blueprint fork sync baseline sequence must append exactly once';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER blueprint_fork_sync_baseline_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.blueprint_fork_sync_baseline
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_blueprint_fork_sync_baseline_change();
CREATE TRIGGER blueprint_fork_sync_baseline_is_valid
BEFORE INSERT ON ple_private.blueprint_fork_sync_baseline
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_blueprint_fork_sync_baseline();

-- C412 creates the child and records its immutable origin.  This trigger runs
--in that same transaction and takes its values from the exact pinned source
--Revision, so a later source save cannot alter a fork baseline.
CREATE FUNCTION ple_private.initialize_blueprint_fork_sync_baselines()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE
    v_source ple_data.blueprint_course%ROWTYPE;
    v_revision ple_data.blueprint_course_revision%ROWTYPE;
    v_now timestamp with time zone := pg_catalog.clock_timestamp();
BEGIN
    SELECT * INTO v_source FROM ple_data.blueprint_course AS source_course
     WHERE source_course.reference_number = NEW.source_blueprint_course_reference_number;
    SELECT * INTO v_revision FROM ple_data.blueprint_course_revision AS revision
     WHERE revision.blueprint_course_reference_number
               = NEW.source_blueprint_course_reference_number
       AND revision.blueprint_revision_number = NEW.source_blueprint_revision_number;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = '23503',
            MESSAGE = 'Blueprint fork sync baseline source Revision is unavailable';
    END IF;

    INSERT INTO ple_private.blueprint_fork_sync_baseline (
        blueprint_course_reference_number, baseline_sequence, unit_kind,
        blueprint_assessment_reference, source_blueprint_course_reference_number,
        source_blueprint_revision_number, baseline_content, recorded_at
    ) VALUES
        (NEW.blueprint_course_reference_number, 1, 'short_name', NULL,
         NEW.source_blueprint_course_reference_number, NEW.source_blueprint_revision_number,
         pg_catalog.to_jsonb(v_source.short_name), v_now),
        (NEW.blueprint_course_reference_number, 1, 'long_name', NULL,
         NEW.source_blueprint_course_reference_number, NEW.source_blueprint_revision_number,
         pg_catalog.to_jsonb(v_source.long_name), v_now),
        (NEW.blueprint_course_reference_number, 1, 'ordered_assessment_list', NULL,
         NEW.source_blueprint_course_reference_number, NEW.source_blueprint_revision_number,
         COALESCE((
             SELECT pg_catalog.jsonb_agg(
                 pg_catalog.jsonb_build_object(
                     'blueprint_module_reference', module_row.module
                         -> 'blueprint_module_reference',
                     'blueprint_assessment_references', COALESCE((
                         SELECT pg_catalog.jsonb_agg(
                             assessment_row.assessment
                                 -> 'blueprint_assessment_reference'
                             ORDER BY assessment_row.assessment_ordinality
                         )
                           FROM pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
                                WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality)
                     ), '[]'::jsonb)
                 ) ORDER BY module_row.module_ordinality
             )
               FROM pg_catalog.jsonb_array_elements(v_revision.content -> 'modules')
                    WITH ORDINALITY AS module_row(module, module_ordinality)
         ), '[]'::jsonb), v_now);

    INSERT INTO ple_private.blueprint_fork_sync_baseline (
        blueprint_course_reference_number, baseline_sequence, unit_kind,
        blueprint_assessment_reference, source_blueprint_course_reference_number,
        source_blueprint_revision_number, baseline_content, recorded_at
    )
    SELECT NEW.blueprint_course_reference_number, 1, 'assessment',
           (assessment_row.assessment ->> 'blueprint_assessment_reference')::uuid,
           NEW.source_blueprint_course_reference_number, NEW.source_blueprint_revision_number,
           assessment_row.assessment, v_now
      FROM pg_catalog.jsonb_array_elements(v_revision.content -> 'modules')
           WITH ORDINALITY AS module_row(module, module_ordinality)
      CROSS JOIN LATERAL pg_catalog.jsonb_array_elements(module_row.module -> 'assessments')
           WITH ORDINALITY AS assessment_row(assessment, assessment_ordinality);
    RETURN NEW;
END
$$;

GRANT EXECUTE ON FUNCTION ple_private.initialize_blueprint_fork_sync_baselines()
    TO ple_data_owner;
SET LOCAL ROLE ple_data_owner;
CREATE TRIGGER blueprint_course_fork_initializes_sync_baselines
AFTER INSERT ON ple_data.blueprint_course_fork
FOR EACH ROW EXECUTE FUNCTION ple_private.initialize_blueprint_fork_sync_baselines();

SET LOCAL ROLE ple_private_owner;
GRANT SELECT, INSERT ON ple_private.blueprint_fork_sync_baseline TO ple_api_owner;
CREATE POLICY blueprint_fork_sync_baseline_api_owner_read
    ON ple_private.blueprint_fork_sync_baseline FOR SELECT TO ple_api_owner USING (true);
CREATE POLICY blueprint_fork_sync_baseline_api_owner_append
    ON ple_private.blueprint_fork_sync_baseline FOR INSERT TO ple_api_owner
    WITH CHECK (true);
REVOKE ALL ON FUNCTION ple_private.reject_blueprint_fork_sync_baseline_change(),
    ple_private.validate_blueprint_fork_sync_baseline(),
    ple_private.initialize_blueprint_fork_sync_baselines() FROM PUBLIC;

COMMENT ON TABLE ple_private.blueprint_fork_sync_baseline IS
    'Private append-only per-selectable-unit source snapshots for one Blueprint Course fork.';
COMMENT ON COLUMN ple_private.blueprint_fork_sync_baseline.baseline_content IS
    'Exact base value only; comparison and any source visibility decision remain later authorized operations.';

RESET ROLE;
