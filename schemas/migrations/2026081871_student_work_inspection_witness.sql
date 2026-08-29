-- WP-INST-G2 / G2-W3B: closed immutable evidence readable only by the broker.

BEGIN;

-- The view binds every private response to its exact accepted submission,
-- receipt, issued presentation, route composite, current policy receipt, and
-- retention state.  It deliberately exposes no feedback prose or solution.
CREATE VIEW public.ple_student_work_inspection_witness_v1
    WITH (security_invoker = true)
AS
SELECT
    attempt.tenant_id,
    course.course_id,
    course.public_id AS course_reference,
    member.course_membership_id,
    member.public_id AS membership_reference,
    assignment.assignment_id,
    assignment.public_id AS assignment_reference,
    run.run_id,
    run.public_id AS run_reference,
    attempt.attempt_id,
    attempt.assignment_position,
    attempt.submitted_at,
    assignment.scoring_generation,
    assignment.scoring_status,
    assignment.score_disclosure,
    assignment.per_item_correctness_disclosure,
    policy.resolved_due_at,
    policy.resolved_closes_at,
    response.response_canonical_json,
    response.response_sha256,
    receipt.canonical_json_version,
    receipt.receipt_attempt_canonical_json,
    receipt.receipt_attempt_payload,
    receipt.receipt_attempt_payload_sha256,
    receipt.presentation_canonical_json,
    receipt.presentation_payload,
    receipt.presentation_payload_sha256,
    receipt.presentation_required,
    attempt.presentation_digest AS issued_presentation_digest,
    attempt.presentation_capability,
    COALESCE(retention.lifecycle, 'active') AS retention_lifecycle
FROM public.question_attempt AS attempt
JOIN public.assignment_run AS run
  ON (run.tenant_id, run.run_id) = (attempt.tenant_id, attempt.run_id)
JOIN public.enrollment AS enrollment
  ON (enrollment.tenant_id, enrollment.enrollment_id) = (run.tenant_id, run.enrollment_id)
JOIN public.course_member AS member
  ON member.tenant_id = enrollment.tenant_id
 AND member.course_id = attempt.course_id
 AND member.user_id = enrollment.user_id
 AND member.role = 'student' AND member.status = 'active'
JOIN public.assignment AS assignment
  ON (assignment.tenant_id, assignment.assignment_id) = (enrollment.tenant_id, enrollment.assignment_id)
JOIN public.course AS course
  ON (course.tenant_id, course.course_id) = (assignment.tenant_id, assignment.course_id)
JOIN public.submission_idempotency AS accepted
  ON accepted.tenant_id = attempt.tenant_id AND accepted.attempt_id = attempt.attempt_id
 AND accepted.request_contract_version = 2
JOIN public.submission AS submission
  ON submission.tenant_id = accepted.tenant_id
 AND submission.course_id = assignment.course_id
 AND submission.attempt_id = accepted.attempt_id
 AND submission.submission_id = accepted.submission_id
 AND submission.occurred_at = accepted.submission_occurred_at
 AND submission.idempotency_key = accepted.idempotency_key
JOIN public.accepted_submission_private_response AS response
  ON response.tenant_id = accepted.tenant_id AND response.course_id = assignment.course_id
 AND response.attempt_id = accepted.attempt_id
 AND response.submission_id = accepted.submission_id
 AND response.submission_occurred_at = accepted.submission_occurred_at
JOIN public.submission_receipt_snapshot AS receipt
  ON receipt.tenant_id = attempt.tenant_id AND receipt.attempt_id = attempt.attempt_id
JOIN public.attempt_effective_policy_current AS current_policy
  ON current_policy.tenant_id = attempt.tenant_id AND current_policy.attempt_id = attempt.attempt_id
 AND current_policy.course_id = assignment.course_id AND current_policy.assignment_id = assignment.assignment_id
JOIN public.attempt_effective_policy_receipt AS policy
  ON policy.tenant_id = current_policy.tenant_id
 AND policy.attempt_id = current_policy.attempt_id
 AND policy.receipt_generation = current_policy.receipt_generation
 AND policy.course_id = assignment.course_id AND policy.assignment_id = assignment.assignment_id
 AND policy.sealed_at IS NOT NULL
LEFT JOIN public.course_retention AS retention
  ON (retention.tenant_id, retention.course_id) = (course.tenant_id, course.course_id)
WHERE attempt.course_id = assignment.course_id
  AND run.completed_at IS NOT NULL
  AND attempt.attempt_status = 'submitted'
  AND attempt.submitted_at IS NOT NULL
  AND accepted.course_id = assignment.course_id
  AND accepted.accepted_actor_id IS NOT NULL
  AND accepted.request_sha256 = response.response_sha256
  AND response.response_sha256 = encode(pg_catalog.sha256(
        convert_to(response.response_canonical_json, 'UTF8')), 'hex')
  AND submission.payload_sha256 = encode(pg_catalog.sha256(convert_to(
        '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8')), 'hex')
  AND submission.payload = '{"kind":"acceptedPrivateResponseV1"}'::jsonb
  AND accepted.payload_sha256 = encode(pg_catalog.sha256(convert_to(
        '{"kind":"acceptedPrivateResponseV1"}'::jsonb::text, 'UTF8')), 'hex')
  AND accepted.payload = '{"kind":"acceptedPrivateResponseV1"}'::jsonb
  AND receipt.canonical_json_version = 1
  AND receipt.receipt_attempt_payload_sha256 = encode(pg_catalog.sha256(
        convert_to(receipt.receipt_attempt_canonical_json, 'UTF8')), 'hex')
  AND receipt.receipt_attempt_canonical_json::jsonb = receipt.receipt_attempt_payload
  AND (receipt.presentation_required = false OR (
        receipt.presentation_canonical_json IS NOT NULL
    AND receipt.presentation_payload IS NOT NULL
    AND receipt.presentation_payload_sha256 = encode(pg_catalog.sha256(
        convert_to(receipt.presentation_canonical_json, 'UTF8')), 'hex')
    AND receipt.presentation_canonical_json::jsonb = receipt.presentation_payload
  ))
  AND (receipt.presentation_required = true OR (
        receipt.presentation_canonical_json IS NULL
    AND receipt.presentation_payload IS NULL
    AND receipt.presentation_payload_sha256 IS NULL
  ));

ALTER VIEW public.ple_student_work_inspection_witness_v1
    OWNER TO ple_student_work_inspection_broker;
REVOKE ALL ON public.ple_student_work_inspection_witness_v1 FROM PUBLIC, ple_app;

-- The definer owner sees only the exact relations represented by the witness.
GRANT SELECT ON public.question_attempt, public.assignment_run, public.enrollment,
    public.course_member, public.assignment, public.course, public.submission_idempotency,
    public.submission, public.accepted_submission_private_response,
    public.submission_receipt_snapshot, public.attempt_effective_policy_current,
    public.attempt_effective_policy_receipt, public.course_retention
    TO ple_student_work_inspection_broker;
GRANT INSERT ON public.record_access_log, public.audit_event
    TO ple_student_work_inspection_broker;
GRANT EXECUTE ON FUNCTION public.ple_course_records_accessible(uuid, uuid),
    public.ple_course_roster_actor(character, uuid, boolean),
    public.ple_current_tenant()
    TO ple_student_work_inspection_broker;

-- The witness runs with a dedicated invoker role under FORCE RLS.  Each
-- source relation gets the same tenant fence; route and role checks stay in
-- the single broker function, so the broker never inherits application table
-- authority.  ASVS 8.3.1 and 8.3.3: authorization is resolved at the
-- server-owned operation boundary using the originating session.
CREATE POLICY student_work_inspection_broker_attempt
    ON public.question_attempt FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_run
    ON public.assignment_run FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_enrollment
    ON public.enrollment FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_member
    ON public.course_member FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_assignment
    ON public.assignment FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_course
    ON public.course FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_idempotency
    ON public.submission_idempotency FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_submission
    ON public.submission FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_private_response
    ON public.accepted_submission_private_response FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_receipt
    ON public.submission_receipt_snapshot FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_current_policy
    ON public.attempt_effective_policy_current FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_policy_receipt
    ON public.attempt_effective_policy_receipt FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_retention
    ON public.course_retention FOR SELECT TO ple_student_work_inspection_broker
    USING (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_record_access
    ON public.record_access_log FOR INSERT TO ple_student_work_inspection_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());
CREATE POLICY student_work_inspection_broker_audit
    ON public.audit_event FOR INSERT TO ple_student_work_inspection_broker
    WITH CHECK (tenant_id = public.ple_current_tenant());

DO $$
BEGIN
    IF NOT has_function_privilege('ple_student_work_inspection_broker',
        'public.ple_course_records_accessible(uuid,uuid)', 'EXECUTE')
       OR NOT has_function_privilege('ple_student_work_inspection_broker',
        'public.ple_course_roster_actor(character,uuid,boolean)', 'EXECUTE')
       OR NOT has_function_privilege('ple_student_work_inspection_broker',
        'public.ple_current_tenant()', 'EXECUTE') THEN
        RAISE EXCEPTION 'student-work inspection broker helper authority is incomplete';
    END IF;
END;
$$;

COMMIT;
