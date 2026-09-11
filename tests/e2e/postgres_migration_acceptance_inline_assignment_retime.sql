-- M11 released Assignment title and due-date retime acceptance oracle.
--
-- This extends the M4 released Assignment fixture through the restricted
-- application procedure and verifies native PLE delivery capture, resume,
-- and pinning across the inline retime.
\set ON_ERROR_STOP on
\set VERBOSITY verbose

DO $$
BEGIN
    IF to_regprocedure(
        'ple_api.save_live_demo_assignment_inline(bigint,bigint,bigint,text,bigint)'
    ) IS NULL
       OR NOT has_function_privilege(
           'ple_app',
           'ple_api.save_live_demo_assignment_inline(bigint,bigint,bigint,text,bigint)',
           'EXECUTE'
       )
       OR has_function_privilege(
           'public',
           'ple_api.save_live_demo_assignment_inline(bigint,bigint,bigint,text,bigint)',
           'EXECUTE'
       ) THEN
        RAISE EXCEPTION 'M11 inline Assignment save procedure authority is not exact';
    END IF;
END
$$;

SELECT course.reference_number AS m11_course_reference,
       assignment.reference_number AS m11_assignment_reference,
       assignment.assignment_edit_number AS m11_original_edit_number,
       (SELECT count(*)
          FROM ple_data.assignment_revision AS revision
         WHERE revision.assignment_id = assignment.assignment_id) AS m11_original_revision_count
  FROM ple_data.assignment AS assignment
  JOIN ple_data.course_instance AS course
    ON course.course_id = assignment.course_id
 WHERE assignment.assignment_status = 'released'
   AND EXISTS (
       SELECT 1
         FROM ple_data.course_membership AS membership
        WHERE membership.course_id = course.course_id
          AND membership.role = 'instructor'
          AND ple_data.course_membership_is_active(membership.membership_id)
   )
 ORDER BY assignment.reference_number
 LIMIT 1
\gset

-- A released row changes only current title/due/Edit Number and produces no
-- release revision.  NULL is deliberately a real current no-deadline value.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT assignment_reference_number AS m11_returned_reference,
       assignment_title AS m11_returned_title,
       due_at_millis AS m11_returned_due_at_millis,
       due_at_millis IS NULL AS m11_returned_due_at_millis_is_null,
       assignment_status AS m11_returned_status,
       assignment_edit_number AS m11_returned_edit_number
  FROM ple_api.save_live_demo_assignment_inline(
      :'m11_course_reference'::bigint,
      :'m11_assignment_reference'::bigint,
      :'m11_original_edit_number'::bigint,
      'M11 released no-deadline title',
      NULL
  )
\gset
COMMIT;

SELECT :'m11_returned_reference'::bigint = :'m11_assignment_reference'::bigint
   AND :'m11_returned_title' = 'M11 released no-deadline title'
   AND :'m11_returned_due_at_millis_is_null'::boolean
   AND :'m11_returned_status' = 'released'
   AND :'m11_returned_edit_number'::bigint = :'m11_original_edit_number'::bigint + 1
       AS m11_released_inline_save_returns_exact_current_projection
\gset
\if :m11_released_inline_save_returns_exact_current_projection
\else
\echo m11_released_inline_save_returns_exact_current_projection
SELECT 1 / 0;
\endif

SELECT assignment.assignment_edit_number AS m11_changed_edit_number,
       assignment.updated_at AS m11_changed_updated_at
  FROM ple_data.assignment AS assignment
 WHERE assignment.reference_number = :'m11_assignment_reference'::bigint
\gset

SELECT (
    SELECT count(*) FROM ple_data.assignment_revision AS revision
     JOIN ple_data.assignment AS assignment ON assignment.assignment_id = revision.assignment_id
    WHERE assignment.reference_number = :'m11_assignment_reference'::bigint
) = :'m11_original_revision_count'::bigint
AND EXISTS (
    SELECT 1 FROM ple_data.assignment AS assignment
     WHERE assignment.reference_number = :'m11_assignment_reference'::bigint
       AND assignment.assignment_title = 'M11 released no-deadline title'
       AND assignment.due_at IS NULL
       AND assignment.assignment_edit_number = :'m11_original_edit_number'::bigint + 1
) AS m11_inline_save_did_not_create_a_revision
\gset
\if :m11_inline_save_did_not_create_a_revision
\else
\echo m11_inline_save_did_not_create_a_revision
SELECT 1 / 0;
\endif

-- Same valid payload at the current Edit Number is a true no-op, including
-- the modification timestamp and Edit Number.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT assignment_edit_number AS m11_noop_edit_number
  FROM ple_api.save_live_demo_assignment_inline(
      :'m11_course_reference'::bigint,
      :'m11_assignment_reference'::bigint,
      :'m11_changed_edit_number'::bigint,
      'M11 released no-deadline title',
      NULL
  )
\gset
COMMIT;

SELECT :'m11_noop_edit_number'::bigint = :'m11_changed_edit_number'::bigint
AND EXISTS (
    SELECT 1 FROM ple_data.assignment AS assignment
     WHERE assignment.reference_number = :'m11_assignment_reference'::bigint
       AND assignment.updated_at = :'m11_changed_updated_at'::timestamptz
) AS m11_inline_save_same_payload_is_noop
\gset
\if :m11_inline_save_same_payload_is_noop
\else
\echo m11_inline_save_same_payload_is_noop
SELECT 1 / 0;
\endif

SELECT set_config('ple_e2e.m11_course_reference', :'m11_course_reference', false);
SELECT set_config('ple_e2e.m11_assignment_reference', :'m11_assignment_reference', false);
SELECT set_config('ple_e2e.m11_original_edit_number', :'m11_original_edit_number', false);
SELECT set_config('ple_e2e.m11_changed_edit_number', :'m11_changed_edit_number', false);

-- The Course list is the read-side receipt consumed by the inline editor.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM ple_api.list_course_assignments(
              current_setting('ple_e2e.m11_course_reference')::bigint
          ) AS item
         WHERE item.assignment_reference_number =
                   current_setting('ple_e2e.m11_assignment_reference')::bigint
           AND item.assignment_title = 'M11 released no-deadline title'
           AND item.due_at_millis IS NULL
           AND item.assignment_status = 'released'
           AND item.assignment_edit_number =
                   current_setting('ple_e2e.m11_changed_edit_number')::bigint
    ) OR EXISTS (
        SELECT 1
          FROM ple_api.list_course_assignments(
              current_setting('ple_e2e.m11_course_reference')::bigint
          ) AS item,
               LATERAL jsonb_object_keys(to_jsonb(item)) AS key(name)
         WHERE key.name NOT IN (
             'assignment_reference_number', 'assignment_title', 'due_at_millis',
             'assignment_status', 'assignment_edit_number'
         )
    ) THEN
        RAISE EXCEPTION 'M11 Course Assignment list projection is not exact';
    END IF;
END
$$;
COMMIT;

-- Stale writes, a Student session, and every non-editable status refuse
-- through the application boundary without persisting a mutation.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_live_demo_assignment_inline(
        current_setting('ple_e2e.m11_course_reference')::bigint,
        current_setting('ple_e2e.m11_assignment_reference')::bigint,
        NULL, 'NULL expected Edit Number must not persist', NULL
    );
    RAISE EXCEPTION 'NULL expected Edit Number M11 inline save unexpectedly succeeded';
EXCEPTION WHEN invalid_parameter_value THEN NULL;
END
$$;
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_live_demo_assignment_inline(
        current_setting('ple_e2e.m11_course_reference')::bigint,
        current_setting('ple_e2e.m11_assignment_reference')::bigint,
        current_setting('ple_e2e.m11_original_edit_number')::bigint,
        'stale write must not persist', NULL
    );
    RAISE EXCEPTION 'stale M11 inline Assignment save unexpectedly succeeded';
EXCEPTION WHEN serialization_failure THEN NULL;
END
$$;
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('ab', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_live_demo_assignment_inline(
        current_setting('ple_e2e.m11_course_reference')::bigint,
        current_setting('ple_e2e.m11_assignment_reference')::bigint,
        current_setting('ple_e2e.m11_changed_edit_number')::bigint,
        'Student write must not persist', NULL
    );
    RAISE EXCEPTION 'non-Instructor M11 inline Assignment save unexpectedly succeeded';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
COMMIT;

BEGIN;
SET LOCAL ROLE ple_data_owner;
UPDATE ple_data.assignment
   SET assignment_status = 'closed'
 WHERE reference_number = :'m11_assignment_reference'::bigint;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_live_demo_assignment_inline(
        current_setting('ple_e2e.m11_course_reference')::bigint,
        current_setting('ple_e2e.m11_assignment_reference')::bigint,
        current_setting('ple_e2e.m11_changed_edit_number')::bigint,
        'closed write must not persist', NULL
    );
    RAISE EXCEPTION 'closed M11 inline Assignment save unexpectedly succeeded';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
ROLLBACK;

BEGIN;
SET LOCAL ROLE ple_data_owner;
UPDATE ple_data.assignment
   SET assignment_status = 'archived'
 WHERE reference_number = :'m11_assignment_reference'::bigint;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    PERFORM 1 FROM ple_api.save_live_demo_assignment_inline(
        current_setting('ple_e2e.m11_course_reference')::bigint,
        current_setting('ple_e2e.m11_assignment_reference')::bigint,
        current_setting('ple_e2e.m11_changed_edit_number')::bigint,
        'archived write must not persist', NULL
    );
    RAISE EXCEPTION 'archived M11 inline Assignment save unexpectedly succeeded';
EXCEPTION WHEN insufficient_privilege THEN NULL;
END
$$;
ROLLBACK;

SELECT EXISTS (
    SELECT 1 FROM ple_data.assignment AS assignment
     WHERE assignment.reference_number = :'m11_assignment_reference'::bigint
       AND assignment.assignment_status = 'released'
       AND assignment.assignment_title = 'M11 released no-deadline title'
       AND assignment.due_at IS NULL
       AND assignment.assignment_edit_number = :'m11_changed_edit_number'::bigint
       AND assignment.updated_at = :'m11_changed_updated_at'::timestamptz
) AS m11_refused_saves_leave_current_assignment_unchanged
\gset
\if :m11_refused_saves_leave_current_assignment_unchanged
\else
\echo m11_refused_saves_leave_current_assignment_unchanged
SELECT 1 / 0;
\endif

-- M5's timed Assignment already owns one exact released Entry, Question, and
-- source binding.  Its immutable Revision due instant is deliberately in the
-- past and rejects late work.  Reuse it with fresh owned Students so a current
-- inline retime can distinguish a captured NULL delivery deadline from a
-- legacy Attempt fallback to that Revision deadline.
SELECT course.reference_number AS m11_delivery_course_reference,
       assignment.reference_number AS m11_delivery_assignment_reference,
       assignment.assignment_edit_number AS m11_delivery_original_edit_number
  FROM ple_data.assignment AS assignment
  JOIN ple_data.assignment_revision AS revision
    ON revision.assignment_revision_id = assignment.released_assignment_revision_id
  JOIN ple_data.course_instance AS course ON course.course_id = assignment.course_id
 WHERE assignment.assignment_status = 'released'
   AND revision.assignment_title = 'M4 due-boundary fixture'
   AND revision.due_at IS NOT NULL
   AND revision.due_at < clock_timestamp()
   AND revision.late_work_rule = 'reject'
 ORDER BY assignment.created_at DESC
 LIMIT 1
\gset

SELECT gen_random_uuid() AS m11_preedit_student_record_id \gset
SELECT gen_random_uuid() AS m11_preedit_membership_id \gset
SELECT gen_random_uuid() AS m11_preedit_invitation_event_id \gset
SELECT gen_random_uuid() AS m11_postedit_student_record_id \gset
SELECT gen_random_uuid() AS m11_postedit_membership_id \gset
SELECT gen_random_uuid() AS m11_postedit_invitation_event_id \gset

-- Roster import and Student invitation claim are the production path that
-- creates Student Records and active Student Course Memberships under forced
-- RLS. Keep the delivery sessions as bounded test credentials after those
-- accounts have been resolved by their unique roster emails.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT * FROM ple_api.import_live_demo_course_roster(
    :'m11_delivery_course_reference'::bigint,
    ARRAY[
        'm11-preedit-student@example.test',
        'm11-postedit-student@example.test'
    ],
    ARRAY[
        'm11-preedit-student@example.test',
        'm11-postedit-student@example.test'
    ],
    ARRAY['M11PREEDIT', 'M11POSTEDIT']
);
COMMIT;

SET ROLE ple_private_owner;
SELECT email.account_id AS m11_preedit_account_id
  FROM ple_private.account_authentication_email AS email
 WHERE email.normalized_email = 'm11-preedit-student@example.test'
\gset
SELECT email.account_id AS m11_postedit_account_id
  FROM ple_private.account_authentication_email AS email
 WHERE email.normalized_email = 'm11-postedit-student@example.test'
\gset
INSERT INTO ple_private.authenticated_session (
    session_id, account_id, product_role, token_hash, created_at, expires_at, revoked_at
) VALUES
    (gen_random_uuid(), :'m11_preedit_account_id'::uuid, 'student',
     decode(repeat('d1', 32), 'hex'), clock_timestamp(),
     clock_timestamp() + interval '1 hour', NULL),
    (gen_random_uuid(), :'m11_postedit_account_id'::uuid, 'student',
     decode(repeat('d2', 32), 'hex'), clock_timestamp(),
     clock_timestamp() + interval '1 hour', NULL);
RESET ROLE;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT active_student_membership AS m11_preedit_membership_is_active
  FROM ple_api.claim_live_demo_course_invitation(
      :'m11_preedit_student_record_id'::uuid,
      :'m11_preedit_membership_id'::uuid,
      :'m11_preedit_invitation_event_id'::uuid,
      :'m11_delivery_course_reference'::bigint
  )
\gset
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d2', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT active_student_membership AS m11_postedit_membership_is_active
  FROM ple_api.claim_live_demo_course_invitation(
      :'m11_postedit_student_record_id'::uuid,
      :'m11_postedit_membership_id'::uuid,
      :'m11_postedit_invitation_event_id'::uuid,
      :'m11_delivery_course_reference'::bigint
  )
\gset
COMMIT;

SELECT :'m11_preedit_membership_is_active'::boolean
   AND :'m11_postedit_membership_is_active'::boolean
   AND EXISTS (
       SELECT 1
         FROM ple_data.course_membership AS membership
        WHERE membership.membership_id = :'m11_preedit_membership_id'::uuid
          AND membership.student_record_id = :'m11_preedit_student_record_id'::uuid
          AND ple_data.course_membership_is_active(membership.membership_id)
   )
   AND EXISTS (
       SELECT 1
         FROM ple_data.course_membership AS membership
        WHERE membership.membership_id = :'m11_postedit_membership_id'::uuid
          AND membership.student_record_id = :'m11_postedit_student_record_id'::uuid
          AND ple_data.course_membership_is_active(membership.membership_id)
   ) AS m11_delivery_students_are_active_owned_members
\gset
\if :m11_delivery_students_are_active_owned_members
\else
\echo m11_delivery_students_are_active_owned_members
SELECT 1 / 0;
\endif

-- Establish a future current deadline through the restricted Instructor save,
-- then start the first Attempt.  The next inline save is the retime whose
-- changed facts this existing Attempt must not observe.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT assignment_edit_number AS m11_delivery_preedit_edit_number
  FROM ple_api.save_live_demo_assignment_inline(
      :'m11_delivery_course_reference'::bigint,
      :'m11_delivery_assignment_reference'::bigint,
      :'m11_delivery_original_edit_number'::bigint,
      'M11 delivery title before retime', 4102444800000
  )
\gset
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT COALESCE(jsonb_agg(jsonb_build_object(
           'issued_question_id', gen_random_uuid(),
           'assignment_entry_id', prepared.assignment_entry_id,
           'issued_position', prepared.issued_position,
           'question_id', prepared.question_id,
           'revision_number', prepared.revision_number,
           'question_seed', prepared.issued_position + 101,
           'parameter_hash', repeat('a', 64),
           'reproduction_details', '{}'::jsonb,
           'presentation_nonce', repeat('1', 32),
           'presentation_checksum', repeat('b', 64)
       ) ORDER BY prepared.issued_position), '[]'::jsonb) AS m11_preedit_presentations,
       count(*) > 0 AND bool_and(NOT prepared.resumed) AS m11_preedit_is_new
  FROM ple_api.prepare_live_demo_native_ple_issuance(
      :'m11_delivery_course_reference'::bigint,
      :'m11_delivery_assignment_reference'::bigint
  ) AS prepared
\gset
\if :m11_preedit_is_new
\else
\echo m11_preedit_is_new
SELECT 1 / 0;
\endif
SELECT min(started.attempt_number) AS m11_preedit_attempt_number,
       bool_and(NOT started.resumed) AS m11_preedit_started_new,
       min(started.assignment_title) AS m11_preedit_started_title
  FROM ple_api.start_live_demo_native_ple_assignment(
      :'m11_delivery_course_reference'::bigint,
      :'m11_delivery_assignment_reference'::bigint,
      :'m11_preedit_presentations'::jsonb
  ) AS started
\gset
COMMIT;

SELECT attempt.reference_number AS m11_preedit_attempt_reference
  FROM ple_private.assignment_attempt AS attempt
 WHERE attempt.student_record_id = :'m11_preedit_student_record_id'::uuid
   AND attempt.assignment_id = (
       SELECT assignment.assignment_id FROM ple_data.assignment AS assignment
        WHERE assignment.reference_number = :'m11_delivery_assignment_reference'::bigint
   )
   AND attempt.completed_at IS NULL
\gset

SELECT :'m11_preedit_started_new'::boolean
AND :'m11_preedit_started_title' = 'M11 delivery title before retime'
AND EXISTS (
    SELECT 1 FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.reference_number = :'m11_preedit_attempt_reference'::bigint
       AND attempt.delivery_assignment_title = 'M11 delivery title before retime'
       AND attempt.delivery_due_at = '2100-01-01 00:00:00+00'::timestamptz
) AS m11_preedit_native_start_captures_current_delivery_facts
\gset
\if :m11_preedit_native_start_captures_current_delivery_facts
\else
\echo m11_preedit_native_start_captures_current_delivery_facts
SELECT 1 / 0;
\endif

-- Retiming the released Assignment to no deadline must leave the first
-- Attempt's title and future due instant intact.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT assignment_edit_number AS m11_delivery_null_edit_number
  FROM ple_api.save_live_demo_assignment_inline(
      :'m11_delivery_course_reference'::bigint,
      :'m11_delivery_assignment_reference'::bigint,
      :'m11_delivery_preedit_edit_number'::bigint,
      'M11 delivery title after retime', NULL
  )
\gset
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT set_config('ple_e2e.m11_preedit_attempt_reference',
                  :'m11_preedit_attempt_reference', false);
SELECT set_config('ple_e2e.m11_delivery_course_reference',
                  :'m11_delivery_course_reference', false);
SELECT set_config('ple_e2e.m11_delivery_assignment_reference',
                  :'m11_delivery_assignment_reference', false);
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_api.read_student_assignment_attempt_context(
            current_setting('ple_e2e.m11_preedit_attempt_reference')::bigint
        ) AS context
         WHERE context.assignment_title = 'M11 delivery title before retime'
    ) OR NOT EXISTS (
        SELECT 1 FROM ple_api.live_demo_assignment_access(
            current_setting('ple_e2e.m11_delivery_course_reference')::bigint,
            current_setting('ple_e2e.m11_delivery_assignment_reference')::bigint
        ) AS access
         WHERE access.start_decision = 'may_start'
    ) THEN
        RAISE EXCEPTION 'pre-retime Attempt did not retain its delivery context';
    END IF;
END
$$;
SELECT min(started.assignment_title) AS m11_preedit_resumed_title,
       bool_and(started.resumed) AS m11_preedit_resumed
  FROM ple_api.start_live_demo_native_ple_assignment(
      current_setting('ple_e2e.m11_delivery_course_reference')::bigint,
      current_setting('ple_e2e.m11_delivery_assignment_reference')::bigint,
      '[]'::jsonb
  ) AS started
\gset
SELECT :'m11_preedit_resumed'::boolean
AND :'m11_preedit_resumed_title' = 'M11 delivery title before retime'
       AS m11_preedit_native_resume_retains_delivery_title
\gset
\if :m11_preedit_native_resume_retains_delivery_title
\else
\echo m11_preedit_native_resume_retains_delivery_title
SELECT 1 / 0;
\endif
SELECT 1 FROM ple_api.save_student_assignment_attempt_response(
    :'m11_preedit_attempt_reference'::bigint, 1, '{}'::jsonb
);
COMMIT;

-- The second Student has no active Attempt after the no-deadline retime.  It
-- must capture NULL as a real delivery fact even though its released Revision
-- still has the M5 past due instant.  A second retime to that past instant
-- then proves native access, resume, and response save never fall back.
BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d2', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT COALESCE(jsonb_agg(jsonb_build_object(
           'issued_question_id', gen_random_uuid(),
           'assignment_entry_id', prepared.assignment_entry_id,
           'issued_position', prepared.issued_position,
           'question_id', prepared.question_id,
           'revision_number', prepared.revision_number,
           'question_seed', prepared.issued_position + 201,
           'parameter_hash', repeat('c', 64),
           'reproduction_details', '{}'::jsonb,
           'presentation_nonce', repeat('2', 32),
           'presentation_checksum', repeat('d', 64)
       ) ORDER BY prepared.issued_position), '[]'::jsonb) AS m11_postedit_presentations,
       count(*) > 0 AND bool_and(NOT prepared.resumed) AS m11_postedit_is_new
  FROM ple_api.prepare_live_demo_native_ple_issuance(
      :'m11_delivery_course_reference'::bigint,
      :'m11_delivery_assignment_reference'::bigint
  ) AS prepared
\gset
\if :m11_postedit_is_new
\else
\echo m11_postedit_is_new
SELECT 1 / 0;
\endif
SELECT min(started.assignment_title) AS m11_postedit_started_title,
       bool_and(NOT started.resumed) AS m11_postedit_started_new
  FROM ple_api.start_live_demo_native_ple_assignment(
      :'m11_delivery_course_reference'::bigint,
      :'m11_delivery_assignment_reference'::bigint,
      :'m11_postedit_presentations'::jsonb
  ) AS started
\gset
COMMIT;

SELECT attempt.reference_number AS m11_postedit_attempt_reference
  FROM ple_private.assignment_attempt AS attempt
 WHERE attempt.student_record_id = :'m11_postedit_student_record_id'::uuid
   AND attempt.assignment_id = (
       SELECT assignment.assignment_id FROM ple_data.assignment AS assignment
        WHERE assignment.reference_number = :'m11_delivery_assignment_reference'::bigint
   )
   AND attempt.completed_at IS NULL
\gset

SELECT :'m11_postedit_started_new'::boolean
AND :'m11_postedit_started_title' = 'M11 delivery title after retime'
AND EXISTS (
    SELECT 1 FROM ple_private.assignment_attempt AS attempt
     WHERE attempt.reference_number = :'m11_postedit_attempt_reference'::bigint
       AND attempt.delivery_assignment_title = 'M11 delivery title after retime'
       AND attempt.delivery_due_at IS NULL
) AS m11_no_deadline_attempt_uses_title_discriminator
\gset
\if :m11_no_deadline_attempt_uses_title_discriminator
\else
\echo m11_no_deadline_attempt_uses_title_discriminator
SELECT 1 / 0;
\endif

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('f1', 32), 'hex'));
SET LOCAL ROLE ple_app;
SELECT assignment_edit_number AS m11_delivery_past_edit_number
  FROM ple_api.save_live_demo_assignment_inline(
      :'m11_delivery_course_reference'::bigint,
      :'m11_delivery_assignment_reference'::bigint,
      :'m11_delivery_null_edit_number'::bigint,
      'M11 delivery title after second retime', 0
  )
\gset
COMMIT;

BEGIN;
SET LOCAL ROLE ple_auth;
SELECT ple_api.resolve_and_install_session(decode(repeat('d2', 32), 'hex'));
SET LOCAL ROLE ple_app;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM ple_api.live_demo_assignment_access(
            current_setting('ple_e2e.m11_delivery_course_reference')::bigint,
            current_setting('ple_e2e.m11_delivery_assignment_reference')::bigint
        ) AS access
         WHERE access.start_decision = 'may_start'
    ) THEN
        RAISE EXCEPTION 'captured no-deadline Attempt fell back to a past Revision due instant';
    END IF;
END
$$;
SELECT min(started.assignment_title) AS m11_postedit_resumed_title,
       bool_and(started.resumed) AS m11_postedit_resumed
  FROM ple_api.start_live_demo_native_ple_assignment(
      current_setting('ple_e2e.m11_delivery_course_reference')::bigint,
      current_setting('ple_e2e.m11_delivery_assignment_reference')::bigint,
      '[]'::jsonb
  ) AS started
\gset
SELECT :'m11_postedit_resumed'::boolean
AND :'m11_postedit_resumed_title' = 'M11 delivery title after retime'
       AS m11_postedit_native_resume_retains_no_deadline_delivery_title
\gset
\if :m11_postedit_native_resume_retains_no_deadline_delivery_title
\else
\echo m11_postedit_native_resume_retains_no_deadline_delivery_title
SELECT 1 / 0;
\endif
SELECT 1 FROM ple_api.save_student_assignment_attempt_response(
    :'m11_postedit_attempt_reference'::bigint, 1, '{}'::jsonb
);
COMMIT;
