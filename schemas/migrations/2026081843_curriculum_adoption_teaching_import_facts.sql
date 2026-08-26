-- WP-PROF-B2: private teaching, lifecycle, and import relational facts.
--
-- This follows 1841's canonical ordinary-course topology and 1842's closed
-- source authorization.  It deliberately returns stored facts only: Rust is
-- the sole owner of relative schedule/DST normalization and semantic evidence.

BEGIN;

-- ASVS 8.2.1, 8.2.2, 8.4.1: forced-RLS broker reads only tenant facts needed
-- by private compiler functions.  No application role receives these grants.
CREATE POLICY curriculum_adoption_item_fact_read ON public.assignment_item
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_selection_fact_read ON public.assignment_selection_group
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_candidate_fact_read ON public.assignment_selection_candidate
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_base_policy_fact_read ON public.assignment_effective_policy_base
    FOR SELECT TO ple_curriculum_adoption_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY curriculum_adoption_alpha_fact_read ON public.alpha_course
    FOR SELECT TO ple_curriculum_adoption_broker USING (true);
GRANT SELECT ON public.assignment_item, public.assignment_selection_group,
    public.assignment_selection_candidate, public.assignment_effective_policy_base,
    public.alpha_course TO ple_curriculum_adoption_broker;

-- ASVS 1.5.2: serde expects number arrays, never PostgreSQL bytea hex text.
CREATE FUNCTION public.ple_cac_byte_array_v1(p_value bytea)
RETURNS jsonb LANGUAGE sql IMMUTABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
    SELECT CASE WHEN octet_length(p_value) = 0 THEN '[]'::jsonb ELSE (
        SELECT jsonb_agg(get_byte(p_value, position) ORDER BY position)
          FROM generate_series(0, octet_length(p_value) - 1) AS position
    ) END
$$;

CREATE FUNCTION public.ple_cac_term_v1(p_tenant uuid, p_course uuid)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_term jsonb;
BEGIN
    SELECT jsonb_build_object('startDate', course.term_start_date::text,
        'endDate', course.term_end_date::text, 'timeZone', course.time_zone)
      INTO v_term FROM public.course AS course
     WHERE course.tenant_id = p_tenant AND course.course_id = p_course;
    IF v_term IS NULL THEN
        RAISE EXCEPTION 'curriculum adoption course is unavailable' USING ERRCODE = '42501';
    END IF;
    RETURN v_term;
END $$;

-- Lock order is source aggregate (1842), destination course/member/schedule,
-- assignment/topology, then receipt (1844).  This helper owns the middle step.
CREATE FUNCTION public.ple_cac_lock_course_snapshot_v1(
    p_tenant uuid, p_actor uuid, p_reference jsonb
) RETURNS uuid LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_course uuid;
BEGIN
    v_course := public.ple_curriculum_adoption_lock_course_v1(p_tenant, p_actor, p_reference);
    PERFORM 1 FROM public.course_schedule_revision AS schedule
     WHERE schedule.tenant_id = p_tenant AND schedule.course_id = v_course FOR KEY SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'curriculum adoption course schedule is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    PERFORM 1 FROM public.assignment AS assignment
     WHERE assignment.tenant_id = p_tenant AND assignment.course_id = v_course
     ORDER BY assignment.public_id FOR KEY SHARE;
    RETURN v_course;
END $$;

CREATE FUNCTION public.ple_cac_witness_v1(p_tenant uuid, p_course uuid)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_witness jsonb;
BEGIN
    SELECT jsonb_build_object('course', 'C-' || course.public_id::text,
        'scheduleRevision', schedule.revision::text, 'assignmentRevisions', coalesce((
            SELECT jsonb_agg(jsonb_build_object('assignment', 'A-' || assignment.public_id::text,
                'revision', assignment.revision::text) ORDER BY assignment.public_id)
              FROM public.assignment AS assignment
             WHERE assignment.tenant_id = p_tenant AND assignment.course_id = course.course_id
        ), '[]'::jsonb)) INTO v_witness
      FROM public.course AS course JOIN public.course_schedule_revision AS schedule
        ON schedule.tenant_id = course.tenant_id AND schedule.course_id = course.course_id
     WHERE course.tenant_id = p_tenant AND course.course_id = p_course;
    IF v_witness IS NULL OR jsonb_array_length(v_witness->'assignmentRevisions') > 1024 THEN
        RAISE EXCEPTION 'curriculum adoption course witness is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    RETURN v_witness;
END $$;

CREATE FUNCTION public.ple_cac_require_witness_v1(
    p_tenant uuid, p_course uuid, p_witness jsonb
) RETURNS void LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
BEGIN
    IF p_witness IS DISTINCT FROM public.ple_cac_witness_v1(p_tenant, p_course) THEN
        RAISE EXCEPTION 'curriculum adoption preview witness is stale' USING ERRCODE = 'PBC01';
    END IF;
END $$;

-- Raw stored teaching state.  ASVS 1.5.2/2.2.1: no currentSemantic aliases
-- exist; Rust validates this closed DTO against the destination term.
CREATE FUNCTION public.ple_cac_teaching_assignment_v1(p_tenant uuid, p_assignment uuid)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_value jsonb;
BEGIN
    SELECT jsonb_build_object('title', assignment.title, 'instructions', assignment.instructions,
        'entries', coalesce((
            SELECT jsonb_agg(entry.value ORDER BY entry.position) FROM (
                SELECT item.position, jsonb_build_object('kind', 'fixed',
                    'reference', jsonb_build_object('problem', item.problem_id, 'version', item.version_id),
                    'pointsPossible', item.points_possible::text,
                    'scoringMode', CASE item.scoring_mode
                        WHEN 'normal' THEN 'normal'
                        WHEN 'full_credit' THEN 'fullCredit'
                        WHEN 'extra_credit' THEN 'extraCredit'
                        WHEN 'excluded' THEN 'excluded'
                    END) AS value
                  FROM public.assignment_item AS item
                 WHERE item.tenant_id = p_tenant AND item.assignment_id = assignment.assignment_id
                UNION ALL
                SELECT pool.position, jsonb_build_object('kind', 'pool', 'candidates', coalesce((
                    SELECT jsonb_agg(jsonb_build_object('problem', candidate.problem_id,
                        'version', candidate.version_id) ORDER BY candidate.position)
                      FROM public.assignment_selection_candidate AS candidate
                     WHERE candidate.tenant_id = p_tenant AND candidate.assignment_id = pool.assignment_id
                       AND candidate.selection_group_id = pool.selection_group_id
                ), '[]'::jsonb), 'drawCount', pool.draw_count,
                    'pointsPerItem', pool.points_per_item::text,
                    'ordering', CASE pool.ordering_policy WHEN 'candidate_order' THEN 'candidateOrder'
                        ELSE 'randomized' END, 'algorithm', 'v1')
                  FROM public.assignment_selection_group AS pool
                 WHERE pool.tenant_id = p_tenant AND pool.assignment_id = assignment.assignment_id
            ) AS entry
        ), '[]'::jsonb), 'defaults', jsonb_build_object(
            'timeLimitSeconds', policy.time_limit_seconds, 'attemptLimit', policy.attempt_limit,
            'lateSubmission', CASE policy.late_submission_policy WHEN 'mark_late' THEN 'markLate'
                ELSE policy.late_submission_policy END, 'deadlineBehavior', 'autoSubmit',
            'runPolicies', jsonb_build_object('completion', CASE assignment.completion_policy
                    WHEN 'answer_all' THEN jsonb_build_object('kind', 'answerAll')
                    WHEN 'all_correct' THEN jsonb_build_object('kind', 'allCorrect')
                    ELSE jsonb_build_object('kind', 'scoreAtLeast', 'fraction', assignment.completion_threshold) END,
                'grade', CASE assignment.attempt_selection_policy WHEN 'instructor_selected'
                    THEN 'instructorSelected' ELSE assignment.attempt_selection_policy END,
                'continuedPractice', CASE assignment.continued_practice_policy WHEN 'capped'
                    THEN jsonb_build_object('kind', 'capped', 'maxAdditionalRuns', assignment.practice_max_additional_runs)
                    ELSE jsonb_build_object('kind', assignment.continued_practice_policy) END,
                'variation', CASE assignment.variation_policy WHEN 'new_seeds' THEN 'newSeeds'
                    WHEN 'selected_problem_variants' THEN 'selectedProblemVariants' ELSE 'fullRegeneration' END),
            -- ASVS 1.5.2, 2.2.1: translate database scalars to the closed
            -- camelCase Rust wire vocabulary at this broker boundary.
            'learnerDisclosure', jsonb_build_object(
                'score', CASE assignment.score_disclosure WHEN 'during_attempt' THEN 'duringAttempt'
                    WHEN 'after_submit' THEN 'afterSubmit' WHEN 'after_due' THEN 'afterDue'
                    WHEN 'after_close' THEN 'afterClose' ELSE assignment.score_disclosure END,
                'perItemCorrectness', CASE assignment.per_item_correctness_disclosure
                    WHEN 'during_attempt' THEN 'duringAttempt' WHEN 'after_submit' THEN 'afterSubmit'
                    WHEN 'after_due' THEN 'afterDue' WHEN 'after_close' THEN 'afterClose'
                    ELSE assignment.per_item_correctness_disclosure END,
                'feedbackText', CASE assignment.feedback_text_disclosure
                    WHEN 'during_attempt' THEN 'duringAttempt' WHEN 'after_submit' THEN 'afterSubmit'
                    WHEN 'after_due' THEN 'afterDue' WHEN 'after_close' THEN 'afterClose'
                    ELSE assignment.feedback_text_disclosure END,
                'solution', CASE assignment.solution_disclosure WHEN 'during_attempt' THEN 'duringAttempt'
                    WHEN 'after_submit' THEN 'afterSubmit' WHEN 'after_due' THEN 'afterDue'
                    WHEN 'after_close' THEN 'afterClose' ELSE assignment.solution_disclosure END,
                'classStatistics', CASE assignment.class_statistics_disclosure
                    WHEN 'during_attempt' THEN 'duringAttempt' WHEN 'after_submit' THEN 'afterSubmit'
                    WHEN 'after_due' THEN 'afterDue' WHEN 'after_close' THEN 'afterClose'
                    ELSE assignment.class_statistics_disclosure END)),
        'basePolicy', jsonb_build_object('availableAt', CASE WHEN policy.available_at IS NULL THEN NULL
                ELSE floor(extract(epoch FROM policy.available_at) * 1000)::bigint END,
            'dueAt', CASE WHEN policy.due_at IS NULL THEN NULL ELSE floor(extract(epoch FROM policy.due_at) * 1000)::bigint END,
            'closesAt', CASE WHEN policy.closes_at IS NULL THEN NULL ELSE floor(extract(epoch FROM policy.closes_at) * 1000)::bigint END,
            'timeLimitSeconds', policy.time_limit_seconds, 'attemptLimit', policy.attempt_limit,
            'lateSubmission', CASE policy.late_submission_policy WHEN 'mark_late' THEN 'markLate'
                ELSE policy.late_submission_policy END, 'deadlineBehavior', 'autoSubmit')) INTO v_value
      FROM public.assignment AS assignment JOIN public.assignment_effective_policy_base AS policy
        ON (policy.tenant_id, policy.course_id, policy.assignment_id) =
           (assignment.tenant_id, assignment.course_id, assignment.assignment_id)
     WHERE assignment.tenant_id = p_tenant AND assignment.assignment_id = p_assignment;
    IF v_value IS NULL OR jsonb_array_length(v_value->'entries') > 1024 THEN
        RAISE EXCEPTION 'curriculum adoption teaching assignment is unavailable' USING ERRCODE = 'PBI01';
    END IF;
    RETURN v_value;
END $$;

CREATE FUNCTION public.ple_cac_teaching_pin_availability_v1(
    p_tenant uuid, p_course uuid, p_replacements jsonb
) RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_pin jsonb; v_choices jsonb;
BEGIN
    WITH pins AS (
        SELECT module.position AS module_index, positioned.position AS assignment_index,
               item.position AS entry_index, NULL::integer AS candidate_index, item.problem_id, item.version_id
          FROM public.teaching_course_module AS module
          JOIN public.teaching_course_assignment_position AS positioned
            ON (positioned.tenant_id, positioned.course_id, positioned.course_module_id) =
               (module.tenant_id, module.course_id, module.course_module_id)
          JOIN public.assignment_item AS item
            ON item.tenant_id = positioned.tenant_id AND item.assignment_id = positioned.assignment_id
         WHERE module.tenant_id = p_tenant AND module.course_id = p_course
        UNION ALL
        SELECT module.position, positioned.position, pool.position, candidate.position,
               candidate.problem_id, candidate.version_id
          FROM public.teaching_course_module AS module
          JOIN public.teaching_course_assignment_position AS positioned
            ON (positioned.tenant_id, positioned.course_id, positioned.course_module_id) =
               (module.tenant_id, module.course_id, module.course_module_id)
          JOIN public.assignment_selection_group AS pool
            ON pool.tenant_id = positioned.tenant_id AND pool.assignment_id = positioned.assignment_id
          JOIN public.assignment_selection_candidate AS candidate
            ON (candidate.tenant_id, candidate.assignment_id, candidate.selection_group_id) =
               (pool.tenant_id, pool.assignment_id, pool.selection_group_id)
         WHERE module.tenant_id = p_tenant AND module.course_id = p_course
    ) SELECT jsonb_build_object('position', jsonb_build_object('moduleIndex', module_index,
        'assignmentIndex', assignment_index, 'entryIndex', entry_index,
        'candidateIndex', to_jsonb(candidate_index)), 'reference',
        jsonb_build_object('problem', pins.problem_id, 'version', pins.version_id)) INTO v_pin
      FROM pins LEFT JOIN public.catalog_search_document AS document
        ON (document.problem_id, document.version_id) = (pins.problem_id, pins.version_id)
     WHERE document.problem_id IS NULL AND NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements(coalesce(p_replacements, '[]'::jsonb)) AS replacement(value)
         WHERE replacement.value->'position' = jsonb_build_object('moduleIndex', module_index,
            'assignmentIndex', assignment_index, 'entryIndex', entry_index,
            'candidateIndex', to_jsonb(candidate_index)))
     ORDER BY module_index, assignment_index, entry_index, candidate_index NULLS FIRST LIMIT 1;
    IF v_pin IS NULL THEN RETURN jsonb_build_object('kind', 'available'); END IF;
    SELECT coalesce(jsonb_agg(substr(question_id::text, 1, 3) || '-' || substr(question_id::text, 4)
        ORDER BY question_id), '[]'::jsonb) INTO v_choices
      FROM (SELECT question_id FROM public.catalog_search_document ORDER BY question_id LIMIT 32) AS choices;
    IF jsonb_array_length(v_choices) = 0 THEN
        RAISE EXCEPTION 'curriculum adoption replacement choices are unavailable' USING ERRCODE = 'PBC01';
    END IF;
    RETURN jsonb_build_object('kind', 'unavailable', 'pin', v_pin, 'candidates', v_choices);
END $$;

CREATE FUNCTION public.ple_cac_lifecycle_facts_v1(
    p_tenant uuid, p_course uuid, p_target_term jsonb, p_title jsonb,
    p_replacements jsonb, p_term_shift boolean
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_modules jsonb; v_sources jsonb; v_eligibility jsonb;
BEGIN
    PERFORM 1 FROM public.teaching_course_module AS module
     WHERE module.tenant_id = p_tenant AND module.course_id = p_course ORDER BY module.position FOR KEY SHARE;
    PERFORM 1 FROM public.teaching_course_assignment_position AS positioned
     WHERE positioned.tenant_id = p_tenant AND positioned.course_id = p_course
     ORDER BY positioned.course_module_id, positioned.position FOR KEY SHARE;
    SELECT coalesce(jsonb_agg(jsonb_build_object('label', module.title, 'assignments', coalesce((
        SELECT jsonb_agg(public.ple_cac_teaching_assignment_v1(p_tenant, positioned.assignment_id)
            ORDER BY positioned.position) FROM public.teaching_course_assignment_position AS positioned
         WHERE positioned.tenant_id = p_tenant AND positioned.course_id = p_course
           AND positioned.course_module_id = module.course_module_id), '[]'::jsonb))
        ORDER BY module.position), '[]'::jsonb) INTO v_modules
      FROM public.teaching_course_module AS module
     WHERE module.tenant_id = p_tenant AND module.course_id = p_course;
    SELECT coalesce(jsonb_agg(jsonb_build_object('modulePosition', module.position,
        'assignmentPosition', positioned.position, 'sourceAssignmentId', assignment.assignment_id,
        'sourceAssignmentRevision', assignment.revision) ORDER BY module.position, positioned.position),
        '[]'::jsonb) INTO v_sources
      FROM public.teaching_course_module AS module JOIN public.teaching_course_assignment_position AS positioned
        ON (positioned.tenant_id, positioned.course_id, positioned.course_module_id) =
           (module.tenant_id, module.course_id, module.course_module_id)
      JOIN public.assignment AS assignment
        ON assignment.tenant_id = positioned.tenant_id AND assignment.assignment_id = positioned.assignment_id
     WHERE module.tenant_id = p_tenant AND module.course_id = p_course;
    IF jsonb_array_length(v_modules) > 1024 OR jsonb_array_length(v_sources) > 1024 THEN
        RAISE EXCEPTION 'curriculum adoption course topology exceeds its bound' USING ERRCODE = 'PBI01';
    END IF;
    IF NOT p_term_shift OR public.ple_curriculum_adoption_course_has_issued_work_v1(p_tenant, p_course) THEN
        v_eligibility := jsonb_build_object('kind', 'issuedWork');
    ELSE
        SELECT jsonb_build_object('kind', 'eligible', 'orderedAssignments', coalesce(jsonb_agg(
            jsonb_build_object('modulePosition', module.position, 'assignmentPosition', positioned.position,
                'assignment', 'A-' || assignment.public_id::text, 'expectedRevision', assignment.revision::text)
            ORDER BY module.position, positioned.position), '[]'::jsonb)) INTO v_eligibility
          FROM public.teaching_course_module AS module JOIN public.teaching_course_assignment_position AS positioned
            ON (positioned.tenant_id, positioned.course_id, positioned.course_module_id) =
               (module.tenant_id, module.course_id, module.course_module_id)
          JOIN public.assignment AS assignment ON assignment.tenant_id = positioned.tenant_id
             AND assignment.assignment_id = positioned.assignment_id
         WHERE module.tenant_id = p_tenant AND module.course_id = p_course;
    END IF;
    RETURN jsonb_build_object('sourceTitle', (SELECT title FROM public.course
            WHERE tenant_id = p_tenant AND course_id = p_course),
        'sourceTerm', public.ple_cac_term_v1(p_tenant, p_course), 'modules', v_modules,
        'resolvedReplacements', public.ple_cac_resolved_replacements_v1(coalesce(p_replacements, '[]'::jsonb)),
        'targetTerm', p_target_term, 'witness', public.ple_cac_witness_v1(p_tenant, p_course),
        'orderedRolloverSources', v_sources, 'termShiftEligibility', v_eligibility,
        'resultingTitle', p_title, 'requestedReplacements', coalesce(p_replacements, '[]'::jsonb),
        'pinAvailability', public.ple_cac_teaching_pin_availability_v1(p_tenant, p_course, p_replacements));
END $$;

-- Immutable import evidence is rendered only as the closed Rust DTO fields.
CREATE FUNCTION public.ple_cac_evidence_envelope_v1(p_evidence public.curriculum_assignment_adoption_evidence)
RETURNS jsonb LANGUAGE sql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
    SELECT jsonb_build_object('canonicalVersion', p_evidence.semantic_canonical_version,
        'canonicalBytes', public.ple_cac_byte_array_v1(p_evidence.semantic_canonical_bytes),
        'digest', public.ple_cac_byte_array_v1(p_evidence.semantic_sha256))
$$;

CREATE FUNCTION public.ple_cac_alpha_source_v1(p_alpha uuid, p_revision bigint)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_source jsonb;
BEGIN
    SELECT jsonb_build_object('reference', 'AC-' || alpha.alpha_course_reference::text,
        'revision', p_revision::text) INTO v_source
      FROM public.alpha_course AS alpha WHERE alpha.alpha_course_id = p_alpha;
    IF v_source IS NULL THEN RAISE EXCEPTION 'curriculum adoption source is unavailable' USING ERRCODE = 'PBI01'; END IF;
    RETURN v_source;
END $$;

CREATE FUNCTION public.ple_cac_import_source_v1(p_evidence public.curriculum_assignment_adoption_evidence)
RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_assignment text;
BEGIN
    IF p_evidence.source_kind = 'blueprint' THEN
        RETURN jsonb_build_object('kind', 'reusable', 'definition', jsonb_build_object('kind', 'blueprint',
            'reference', 'BP-' || p_evidence.source_blueprint_reference::text,
            'revision', p_evidence.source_blueprint_revision::text));
    ELSIF p_evidence.source_kind = 'alpha' THEN
        RETURN jsonb_build_object('kind', 'reusable', 'definition', jsonb_build_object('kind', 'alpha',
            'reference', (public.ple_cac_alpha_source_v1(p_evidence.source_alpha_course_id,
                p_evidence.source_alpha_revision))->'reference', 'revision', p_evidence.source_alpha_revision::text,
            'moduleIndex', p_evidence.source_module_position, 'assignmentIndex', p_evidence.source_definition_position));
    END IF;
    SELECT 'A-' || assignment.public_id::text INTO v_assignment FROM public.assignment AS assignment
     WHERE assignment.tenant_id = p_evidence.tenant_id AND assignment.assignment_id = p_evidence.source_assignment_id;
    IF v_assignment IS NULL THEN RAISE EXCEPTION 'curriculum adoption rollover source is unavailable' USING ERRCODE = 'PBI01'; END IF;
    RETURN jsonb_build_object('kind', 'rollover', 'source', jsonb_build_object('assignment',
        jsonb_build_object('assignment', v_assignment, 'revision', p_evidence.source_assignment_revision::text)));
END $$;

CREATE FUNCTION public.ple_cac_import_destination_v1(
    p_tenant uuid, p_course uuid, p_assignment uuid
) RETURNS jsonb LANGUAGE plpgsql STABLE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_evidence public.curriculum_assignment_adoption_evidence; v_value jsonb;
BEGIN
    SELECT evidence.* INTO v_evidence FROM public.curriculum_assignment_import_current AS current_row
      JOIN public.curriculum_assignment_adoption_evidence AS evidence
        ON (evidence.tenant_id, evidence.receipt_key, evidence.assignment_id) =
           (current_row.tenant_id, current_row.receipt_key, current_row.assignment_id)
     WHERE current_row.tenant_id = p_tenant AND current_row.assignment_id = p_assignment
     FOR KEY SHARE OF current_row, evidence;
    IF NOT FOUND THEN RAISE EXCEPTION 'curriculum adoption import is unavailable' USING ERRCODE = 'PBC01'; END IF;
    SELECT jsonb_build_object('witness', public.ple_cac_witness_v1(p_tenant, p_course),
        'targetTerm', public.ple_cac_term_v1(p_tenant, p_course),
        'assignment', 'A-' || assignment.public_id::text, 'assignmentRevision', assignment.revision::text,
        'importRevision', v_evidence.import_revision::text,
        'importedSource', CASE WHEN v_evidence.source_kind = 'rollover' THEN NULL ELSE
            (public.ple_cac_import_source_v1(v_evidence))->'definition' END,
        'baselineSemantic', v_evidence.semantic_payload,
        'baselineEvidence', public.ple_cac_evidence_envelope_v1(v_evidence),
        'currentTeaching', public.ple_cac_teaching_assignment_v1(p_tenant, assignment.assignment_id),
        'issuedWork', EXISTS (SELECT 1 FROM public.assignment_run AS run_row JOIN public.enrollment AS enrollment
            ON enrollment.tenant_id = run_row.tenant_id AND enrollment.enrollment_id = run_row.enrollment_id
           WHERE enrollment.tenant_id = p_tenant AND enrollment.assignment_id = assignment.assignment_id)) INTO v_value
      FROM public.assignment AS assignment
     WHERE assignment.tenant_id = p_tenant AND assignment.course_id = p_course
       AND assignment.assignment_id = p_assignment;
    IF v_value IS NULL THEN RAISE EXCEPTION 'curriculum adoption assignment is unavailable' USING ERRCODE = '42501'; END IF;
    RETURN v_value;
END $$;

CREATE FUNCTION public.ple_cac_import_inspection_v1(p_tenant uuid, p_course uuid)
RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public' AS $$
DECLARE v_rows jsonb; v_origin jsonb; v_adoption public.curriculum_whole_course_adoption;
DECLARE v_source_course text; v_source_assignments jsonb; v_has_adoption boolean := false;
BEGIN
    SELECT adoption.* INTO v_adoption FROM public.curriculum_whole_course_adoption AS adoption
     WHERE adoption.tenant_id = p_tenant AND adoption.course_id = p_course FOR KEY SHARE;
    v_has_adoption := FOUND;
    -- A whole-course receipt is immutable provenance for every original
    -- destination assignment. Missing or detached current pointers are
    -- corruption, not an empty ordinary import inspection (ASVS 2.3.1).
    -- Later assignment imports are valid additions and remain inspectable.
    IF v_has_adoption AND EXISTS (
            SELECT 1 FROM public.curriculum_whole_course_assignment AS whole
            LEFT JOIN public.curriculum_assignment_import_current AS current_row
              ON (current_row.tenant_id, current_row.assignment_id) =
                 (whole.tenant_id, whole.destination_assignment_id)
            LEFT JOIN public.curriculum_assignment_adoption_evidence AS evidence
              ON (evidence.tenant_id, evidence.receipt_key, evidence.assignment_id) =
                 (current_row.tenant_id, current_row.receipt_key, current_row.assignment_id)
             WHERE whole.tenant_id = p_tenant AND whole.course_id = p_course
               AND (current_row.assignment_id IS NULL OR evidence.assignment_id IS NULL)
        ) THEN
        RAISE EXCEPTION 'curriculum adoption whole-course current import is incomplete'
            USING ERRCODE = 'PBI01';
    END IF;
    PERFORM 1 FROM public.curriculum_assignment_import_current AS current_row
      JOIN public.curriculum_assignment_adoption_evidence AS evidence
        ON (evidence.tenant_id, evidence.receipt_key, evidence.assignment_id) =
           (current_row.tenant_id, current_row.receipt_key, current_row.assignment_id)
      JOIN public.assignment AS assignment ON assignment.tenant_id = current_row.tenant_id
         AND assignment.assignment_id = current_row.assignment_id
     WHERE current_row.tenant_id = p_tenant AND assignment.course_id = p_course
     ORDER BY assignment.public_id FOR KEY SHARE OF current_row, evidence, assignment;
    SELECT coalesce(jsonb_agg(jsonb_build_object('assignment', 'A-' || assignment.public_id::text,
        'source', public.ple_cac_import_source_v1(evidence), 'revision', evidence.import_revision::text,
        'baselineSemantic', evidence.semantic_payload,
        'baselineEvidence', public.ple_cac_evidence_envelope_v1(evidence),
        'currentTeaching', public.ple_cac_teaching_assignment_v1(p_tenant, assignment.assignment_id))
        ORDER BY assignment.public_id), '[]'::jsonb) INTO v_rows
      FROM public.curriculum_assignment_import_current AS current_row
      JOIN public.curriculum_assignment_adoption_evidence AS evidence
        ON (evidence.tenant_id, evidence.receipt_key, evidence.assignment_id) =
           (current_row.tenant_id, current_row.receipt_key, current_row.assignment_id)
      JOIN public.assignment AS assignment ON assignment.tenant_id = current_row.tenant_id
         AND assignment.assignment_id = current_row.assignment_id
     WHERE current_row.tenant_id = p_tenant AND assignment.course_id = p_course;
    IF jsonb_array_length(v_rows) = 0 THEN
        IF v_has_adoption THEN
            RAISE EXCEPTION 'curriculum adoption whole-course current import is incomplete'
                USING ERRCODE = 'PBI01';
        END IF;
        RETURN NULL;
    END IF;
    IF jsonb_array_length(v_rows) > 1024 THEN RAISE EXCEPTION 'curriculum adoption imports exceed bound' USING ERRCODE = 'PBI01'; END IF;
    IF NOT v_has_adoption THEN v_origin := jsonb_build_object('kind', 'ordinary');
    ELSIF v_adoption.origin_kind = 'alpha' THEN
        v_origin := jsonb_build_object('kind', 'alpha', 'source',
            public.ple_cac_alpha_source_v1(v_adoption.source_alpha_course_id, v_adoption.source_alpha_revision));
    ELSE
        SELECT 'C-' || course.public_id::text INTO v_source_course FROM public.course AS course
         WHERE course.tenant_id = p_tenant AND course.course_id = v_adoption.source_course_id;
        SELECT coalesce(jsonb_agg(jsonb_build_object('assignment', 'A-' || assignment.public_id::text,
            'revision', whole.source_assignment_revision::text) ORDER BY assignment.public_id), '[]'::jsonb)
          INTO v_source_assignments FROM public.curriculum_whole_course_assignment AS whole
          JOIN public.assignment AS assignment ON assignment.tenant_id = whole.tenant_id
             AND assignment.assignment_id = whole.source_assignment_id
         WHERE whole.tenant_id = p_tenant AND whole.course_id = p_course;
        IF v_source_course IS NULL OR jsonb_array_length(v_source_assignments) = 0 THEN
            RAISE EXCEPTION 'curriculum adoption rollover origin is unavailable' USING ERRCODE = 'PBI01';
        END IF;
        v_origin := jsonb_build_object('kind', 'rollover', 'source', jsonb_build_object('sourceSchedule',
            jsonb_build_object('course', v_source_course, 'scheduleRevision', v_adoption.source_schedule_revision::text,
                'assignmentRevisions', v_source_assignments)));
    END IF;
    RETURN jsonb_build_object('kind', 'inspection', 'course', jsonb_build_object(
        'witness', public.ple_cac_witness_v1(p_tenant, p_course), 'origin', v_origin,
        'term', public.ple_cac_term_v1(p_tenant, p_course), 'assignments', v_rows));
END $$;

CREATE FUNCTION public.ple_cac_reconciliation_facts_v1(
    p_tenant uuid, p_actor uuid, p_request jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_receipt public.curriculum_adoption_receipt; v_course uuid; v_source_course uuid;
DECLARE v_rows jsonb; v_destinations jsonb;
BEGIN
    SELECT receipt.* INTO v_receipt FROM public.curriculum_adoption_receipt AS receipt
     WHERE receipt.tenant_id = p_tenant AND receipt.idempotency_key = p_request#>>'{receipt,idempotencyKey}'
     FOR KEY SHARE;
    IF NOT FOUND OR v_receipt.destination_course_id IS NULL THEN
        RAISE EXCEPTION 'curriculum adoption receipt is unavailable' USING ERRCODE = '42501';
    END IF;
    -- Current Instructor authority, not the original receipt actor, authorizes
    -- repair.  Rollover keeps its source-course authority check from Memory.
    IF v_receipt.operation = 'courseRollover' THEN
        SELECT course.course_id INTO v_source_course FROM public.course AS course
         WHERE course.tenant_id = p_tenant AND course.course_id = v_receipt.source_course_id;
        IF v_source_course IS NULL THEN
            RAISE EXCEPTION 'curriculum adoption source course is unavailable' USING ERRCODE = '42501';
        END IF;
        PERFORM public.ple_cac_lock_course_snapshot_v1(p_tenant, p_actor,
            to_jsonb('C-' || (SELECT public_id::text FROM public.course
                WHERE tenant_id = p_tenant AND course_id = v_source_course)));
    END IF;
    SELECT course.course_id INTO v_course FROM public.course AS course
     WHERE course.tenant_id = p_tenant AND course.course_id = v_receipt.destination_course_id;
    IF v_course IS NULL THEN
        RAISE EXCEPTION 'curriculum adoption destination course is unavailable' USING ERRCODE = '42501';
    END IF;
    PERFORM public.ple_cac_lock_course_snapshot_v1(p_tenant, p_actor,
        to_jsonb('C-' || (SELECT public_id::text FROM public.course
            WHERE tenant_id = p_tenant AND course_id = v_course)));
    PERFORM 1 FROM public.curriculum_adoption_receipt_assignment AS destination
      JOIN public.assignment AS assignment ON assignment.tenant_id = destination.tenant_id
         AND assignment.assignment_id = destination.assignment_id
     WHERE destination.tenant_id = p_tenant AND destination.receipt_key = v_receipt.idempotency_key
     ORDER BY assignment.public_id FOR KEY SHARE OF destination, assignment;
    PERFORM 1 FROM public.curriculum_assignment_adoption_evidence AS evidence
      JOIN public.curriculum_adoption_receipt_assignment AS destination
        ON (destination.tenant_id, destination.receipt_key, destination.assignment_id) =
           (evidence.tenant_id, evidence.receipt_key, evidence.assignment_id)
     WHERE destination.tenant_id = p_tenant AND destination.receipt_key = v_receipt.idempotency_key
     ORDER BY evidence.assignment_id, evidence.import_revision FOR KEY SHARE OF evidence, destination;
    PERFORM 1 FROM public.curriculum_assignment_import_current AS current_row
      JOIN public.curriculum_adoption_receipt_assignment AS destination
        ON destination.tenant_id = current_row.tenant_id
       AND destination.assignment_id = current_row.assignment_id
     WHERE destination.tenant_id = p_tenant AND destination.receipt_key = v_receipt.idempotency_key
     ORDER BY current_row.assignment_id FOR KEY SHARE OF current_row, destination;
    SELECT coalesce(jsonb_agg('A-' || assignment.public_id::text ORDER BY assignment.public_id), '[]'::jsonb)
      INTO v_destinations FROM public.curriculum_adoption_receipt_assignment AS destination
      JOIN public.assignment AS assignment ON assignment.tenant_id = destination.tenant_id
         AND assignment.assignment_id = destination.assignment_id
     WHERE destination.tenant_id = p_tenant AND destination.receipt_key = v_receipt.idempotency_key;
    SELECT coalesce(jsonb_agg(jsonb_build_object('assignment', 'A-' || assignment.public_id::text,
        'expectedRevision', assignment.revision::text, 'currentPointer', CASE WHEN current_row.assignment_id IS NULL
            THEN NULL ELSE jsonb_build_object('receipt', jsonb_build_object('idempotencyKey', current_row.receipt_key),
                'revision', current_evidence.import_revision::text) END, 'immutableEvidence', coalesce((
            SELECT jsonb_agg(jsonb_build_object('receipt', jsonb_build_object('idempotencyKey', evidence.receipt_key),
                'revision', evidence.import_revision::text, 'baselineSemantic', evidence.semantic_payload,
                'baselineEvidence', public.ple_cac_evidence_envelope_v1(evidence)) ORDER BY evidence.import_revision)
              FROM public.curriculum_assignment_adoption_evidence AS evidence
             WHERE evidence.tenant_id = p_tenant AND evidence.assignment_id = assignment.assignment_id), '[]'::jsonb))
        ORDER BY assignment.public_id), '[]'::jsonb) INTO v_rows
      FROM public.curriculum_adoption_receipt_assignment AS destination
      JOIN public.assignment AS assignment ON assignment.tenant_id = destination.tenant_id
         AND assignment.assignment_id = destination.assignment_id
      LEFT JOIN public.curriculum_assignment_import_current AS current_row
        ON current_row.tenant_id = assignment.tenant_id AND current_row.assignment_id = assignment.assignment_id
      LEFT JOIN public.curriculum_assignment_adoption_evidence AS current_evidence
        ON (current_evidence.tenant_id, current_evidence.receipt_key, current_evidence.assignment_id) =
           (current_row.tenant_id, current_row.receipt_key, current_row.assignment_id)
     WHERE destination.tenant_id = p_tenant AND destination.receipt_key = v_receipt.idempotency_key;
    IF jsonb_array_length(v_rows) = 0 OR jsonb_array_length(v_rows) > 1024 THEN
        RAISE EXCEPTION 'curriculum adoption receipt has no bounded import destination' USING ERRCODE = 'PBI01';
    END IF;
    RETURN jsonb_build_object('kind', 'reconciliation', 'receipt', jsonb_build_object('receipt',
        jsonb_build_object('idempotencyKey', v_receipt.idempotency_key), 'destinationAssignments', v_destinations),
        'assignments', v_rows);
END $$;

-- Private compiler: 1847 will wrap its facts in SnapshotFactsV1, mint prepare
-- records, and handle replay.  This function never creates a receipt or digest.
CREATE FUNCTION public.ple_compile_curriculum_adoption_facts_v1(
    p_tenant uuid, p_session character(64), p_actor uuid, p_kind text, p_request jsonb
) RETURNS jsonb LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path TO 'pg_catalog', 'public', pg_temp AS $$
DECLARE v_course uuid; v_assignment uuid; v_source jsonb; v_witness jsonb; v_destination jsonb; v_pin jsonb;
BEGIN
    PERFORM public.ple_cac_validate_request_v1(p_kind, p_request);
    IF p_actor IS NULL THEN RAISE EXCEPTION 'curriculum adoption actor is unavailable' USING ERRCODE = '42501'; END IF;
    IF p_kind IN ('previewForkAlpha', 'applyForkAlpha') THEN
        RETURN jsonb_build_object('kind', 'forkAlpha', 'source', public.ple_cac_source_facts_v1(
            p_tenant, p_session, p_request->'source', p_request->'replacements', NULL, false));
    ELSIF p_kind IN ('previewBlueprintInstantiation', 'applyBlueprintInstantiation') THEN
        v_source := public.ple_cac_source_facts_v1(p_tenant, p_session, p_request->'source', p_request->'replacements', p_request->'targetTerm', false);
        v_course := public.ple_cac_lock_course_snapshot_v1(p_tenant, p_actor, p_request->'course');
        IF p_kind LIKE 'apply%' THEN PERFORM public.ple_cac_require_witness_v1(p_tenant, v_course, p_request->'previewWitness'); END IF;
        RETURN jsonb_build_object('kind', 'blueprintInstantiation', 'source', v_source,
            'destination', jsonb_build_object('witness', public.ple_cac_witness_v1(p_tenant, v_course)));
    ELSIF p_kind IN ('previewAlphaInstantiation', 'applyAlphaInstantiation') THEN
        RETURN jsonb_build_object('kind', 'alphaInstantiation', 'source', public.ple_cac_source_facts_v1(
            p_tenant, p_session, p_request->'source', p_request->'replacements', p_request->'targetTerm', false));
    ELSIF p_kind IN ('previewCourseRollover', 'applyCourseRollover', 'previewCourseTermShift', 'applyCourseTermShift') THEN
        v_witness := coalesce(p_request->'witness', p_request->'previewWitness');
        v_course := public.ple_cac_lock_course_snapshot_v1(p_tenant, p_actor, v_witness->'course');
        PERFORM public.ple_cac_require_witness_v1(p_tenant, v_course, v_witness);
        IF p_kind LIKE '%Rollover' THEN RETURN jsonb_build_object('kind', 'courseRollover', 'source',
            public.ple_cac_lifecycle_facts_v1(p_tenant, v_course, p_request->'targetTerm', p_request->'title', p_request->'replacements', false)); END IF;
        RETURN jsonb_build_object('kind', 'courseTermShift', 'course',
            public.ple_cac_lifecycle_facts_v1(p_tenant, v_course, p_request->'targetTerm', NULL, '[]'::jsonb, true));
    ELSIF p_kind IN ('previewSourceDerivedAssignment', 'createSourceDerivedAssignment') THEN
        v_source := public.ple_cac_source_facts_v1(p_tenant, p_session, p_request->'source', p_request->'replacements', NULL, true);
        v_course := public.ple_cac_lock_course_snapshot_v1(p_tenant, p_actor, p_request->'course');
        IF p_kind = 'createSourceDerivedAssignment' THEN PERFORM public.ple_cac_require_witness_v1(p_tenant, v_course, p_request->'previewWitness'); END IF;
        RETURN jsonb_build_object('kind', 'sourceDerivedAssignment', 'source', jsonb_set(v_source, '{targetTerm}', public.ple_cac_term_v1(p_tenant, v_course)),
            'destination', jsonb_build_object('witness', public.ple_cac_witness_v1(p_tenant, v_course)));
    ELSIF p_kind IN ('previewAssignmentFastForward', 'applyAssignmentFastForward') THEN
        v_source := public.ple_cac_source_facts_v1(p_tenant, p_session, p_request->'source', '[]'::jsonb, NULL, true);
        v_course := public.ple_cac_lock_course_snapshot_v1(p_tenant, p_actor, p_request->'course');
        v_assignment := public.ple_curriculum_adoption_lock_assignment_v1(p_tenant, v_course, p_request#>'{assignment,assignment}');
        IF p_kind = 'applyAssignmentFastForward' THEN PERFORM public.ple_cac_require_witness_v1(p_tenant, v_course, p_request->'previewWitness'); END IF;
        v_destination := public.ple_cac_import_destination_v1(p_tenant, v_course, v_assignment);
        IF p_request#>>'{assignment,revision}' <> v_destination->>'assignmentRevision'
           OR p_request->>'importRevision' <> v_destination->>'importRevision' THEN
            RAISE EXCEPTION 'curriculum adoption import witness is stale' USING ERRCODE = 'PBC01';
        END IF;
        v_pin := v_source->'pinAvailability';
        RETURN jsonb_build_object('kind', 'assignmentFastForward', 'import', jsonb_build_object('kind', 'fastForward',
            'destination', v_destination, 'source', jsonb_build_object('requestedSource', v_source#>'{requestedSource,source}',
                'currentSource', v_source#>'{currentSource,source}', 'rawSemantic', v_source->'rawSemantic',
                'resolvedReplacements', v_source->'resolvedReplacements', 'unavailablePin', CASE WHEN v_pin->>'kind' = 'unavailable' THEN v_pin->'pin' ELSE NULL END,
                'replacementChoices', CASE WHEN v_pin->>'kind' = 'unavailable' THEN v_pin->'candidates' ELSE NULL END)));
    ELSIF p_kind = 'inspectImports' THEN
        v_course := public.ple_cac_lock_course_snapshot_v1(p_tenant, p_actor, p_request);
        RETURN public.ple_cac_import_inspection_v1(p_tenant, v_course);
    ELSIF p_kind = 'reconcile' THEN
        RETURN jsonb_build_object('kind', 'reconcile', 'reconciliation',
            public.ple_cac_reconciliation_facts_v1(p_tenant, p_actor, p_request));
    END IF;
    RAISE EXCEPTION 'curriculum adoption bridge operation is invalid' USING ERRCODE = '22023';
END $$;

ALTER FUNCTION public.ple_cac_byte_array_v1(bytea) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_term_v1(uuid, uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_lock_course_snapshot_v1(uuid, uuid, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_witness_v1(uuid, uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_require_witness_v1(uuid, uuid, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_teaching_assignment_v1(uuid, uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_teaching_pin_availability_v1(uuid, uuid, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_lifecycle_facts_v1(uuid, uuid, jsonb, jsonb, jsonb, boolean) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_evidence_envelope_v1(public.curriculum_assignment_adoption_evidence) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_alpha_source_v1(uuid, bigint) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_import_source_v1(public.curriculum_assignment_adoption_evidence) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_import_destination_v1(uuid, uuid, uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_import_inspection_v1(uuid, uuid) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_cac_reconciliation_facts_v1(uuid, uuid, jsonb) OWNER TO ple_curriculum_adoption_broker;
ALTER FUNCTION public.ple_compile_curriculum_adoption_facts_v1(uuid, character, uuid, text, jsonb) OWNER TO ple_curriculum_adoption_broker;

REVOKE ALL ON FUNCTION public.ple_cac_byte_array_v1(bytea), public.ple_cac_term_v1(uuid, uuid),
    public.ple_cac_lock_course_snapshot_v1(uuid, uuid, jsonb), public.ple_cac_witness_v1(uuid, uuid),
    public.ple_cac_require_witness_v1(uuid, uuid, jsonb), public.ple_cac_teaching_assignment_v1(uuid, uuid),
    public.ple_cac_teaching_pin_availability_v1(uuid, uuid, jsonb),
    public.ple_cac_lifecycle_facts_v1(uuid, uuid, jsonb, jsonb, jsonb, boolean),
    public.ple_cac_evidence_envelope_v1(public.curriculum_assignment_adoption_evidence),
    public.ple_cac_alpha_source_v1(uuid, bigint),
    public.ple_cac_import_source_v1(public.curriculum_assignment_adoption_evidence),
    public.ple_cac_import_destination_v1(uuid, uuid, uuid), public.ple_cac_import_inspection_v1(uuid, uuid),
    public.ple_cac_reconciliation_facts_v1(uuid, uuid, jsonb),
    public.ple_compile_curriculum_adoption_facts_v1(uuid, character, uuid, text, jsonb)
    FROM PUBLIC, ple_app, ple_curriculum_adoption_broker;

-- The adoption broker composes this private fact family behind the public
-- bridge.  Its complete execute capability keeps every internal edge explicit
-- while application roles retain only the public facade.
GRANT EXECUTE ON FUNCTION public.ple_cac_byte_array_v1(bytea), public.ple_cac_term_v1(uuid, uuid),
    public.ple_cac_lock_course_snapshot_v1(uuid, uuid, jsonb), public.ple_cac_witness_v1(uuid, uuid),
    public.ple_cac_require_witness_v1(uuid, uuid, jsonb), public.ple_cac_teaching_assignment_v1(uuid, uuid),
    public.ple_cac_teaching_pin_availability_v1(uuid, uuid, jsonb),
    public.ple_cac_lifecycle_facts_v1(uuid, uuid, jsonb, jsonb, jsonb, boolean),
    public.ple_cac_evidence_envelope_v1(public.curriculum_assignment_adoption_evidence),
    public.ple_cac_alpha_source_v1(uuid, bigint),
    public.ple_cac_import_source_v1(public.curriculum_assignment_adoption_evidence),
    public.ple_cac_import_destination_v1(uuid, uuid, uuid), public.ple_cac_import_inspection_v1(uuid, uuid),
    public.ple_cac_reconciliation_facts_v1(uuid, uuid, jsonb),
    public.ple_compile_curriculum_adoption_facts_v1(uuid, character, uuid, text, jsonb)
    TO ple_curriculum_adoption_broker;

DO $$
DECLARE v_function regprocedure; v_role text;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'public.ple_cac_byte_array_v1(bytea)'::regprocedure,
        'public.ple_cac_term_v1(uuid,uuid)'::regprocedure,
        'public.ple_cac_lock_course_snapshot_v1(uuid,uuid,jsonb)'::regprocedure,
        'public.ple_cac_witness_v1(uuid,uuid)'::regprocedure,
        'public.ple_cac_require_witness_v1(uuid,uuid,jsonb)'::regprocedure,
        'public.ple_cac_teaching_assignment_v1(uuid,uuid)'::regprocedure,
        'public.ple_cac_teaching_pin_availability_v1(uuid,uuid,jsonb)'::regprocedure,
        'public.ple_cac_lifecycle_facts_v1(uuid,uuid,jsonb,jsonb,jsonb,boolean)'::regprocedure,
        'public.ple_cac_evidence_envelope_v1(curriculum_assignment_adoption_evidence)'::regprocedure,
        'public.ple_cac_alpha_source_v1(uuid,bigint)'::regprocedure,
        'public.ple_cac_import_source_v1(curriculum_assignment_adoption_evidence)'::regprocedure,
        'public.ple_cac_import_destination_v1(uuid,uuid,uuid)'::regprocedure,
        'public.ple_cac_import_inspection_v1(uuid,uuid)'::regprocedure,
        'public.ple_cac_reconciliation_facts_v1(uuid,uuid,jsonb)'::regprocedure,
        'public.ple_compile_curriculum_adoption_facts_v1(uuid,character,uuid,text,jsonb)'::regprocedure
    ] LOOP
        IF (SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid = v_function) <> 'ple_curriculum_adoption_broker' THEN
            RAISE EXCEPTION 'curriculum adoption teaching fact ownership is unsafe';
        END IF;
        IF NOT has_function_privilege('ple_curriculum_adoption_broker', v_function, 'EXECUTE') THEN
            RAISE EXCEPTION 'curriculum adoption teaching fact capability is incomplete';
        END IF;
        FOREACH v_role IN ARRAY ARRAY['public', 'ple_app', 'ple_auth', 'ple_student', 'ple_grader', 'ple_grading_reader'] LOOP
            IF has_function_privilege(v_role, v_function, 'EXECUTE') THEN
                RAISE EXCEPTION 'curriculum adoption teaching facts leaked to %', v_role;
            END IF;
        END LOOP;
    END LOOP;
    IF has_table_privilege('ple_curriculum_adoption_broker', 'public.assignment_item', 'INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_curriculum_adoption_broker', 'public.assignment_effective_policy_base', 'INSERT,UPDATE,DELETE')
       OR has_table_privilege('ple_curriculum_adoption_broker', 'public.alpha_course', 'INSERT,UPDATE,DELETE') THEN
        RAISE EXCEPTION 'curriculum adoption fact reader can mutate source state';
    END IF;
END $$;

COMMIT;
