-- Explicit indexes. Constraint indexes are created with tables.

SET LOCAL ROLE ple_private_owner;

CREATE INDEX email_authentication_challenge_active_token_idx
ON ple_private.email_authentication_challenge (token_hash, expires_at) WHERE consumed_at IS NULL;

CREATE INDEX passkey_active_account_idx ON ple_private.passkey (account_id, created_at)
WHERE revoked_at IS NULL;

CREATE INDEX sysadmin_totp_attestation_active_idx
ON ple_private.sysadmin_totp_attestation (attestation_id, expires_at) WHERE consumed_at IS NULL;

CREATE INDEX authenticated_session_active_account_idx ON ple_private.authenticated_session (account_id, expires_at)
WHERE revoked_at IS NULL;

SET LOCAL ROLE ple_data_owner;



-- A retired name stays reserved. Renaming never changes the stable UUID and
-- restore cannot silently revive a case-only duplicate.
CREATE UNIQUE INDEX content_discipline_global_name_unique
    ON ple_data.content_discipline (lower(name));



-- One global Subject identity: preserve display case, reject case-only duplicates.
CREATE UNIQUE INDEX content_subject_global_name_unique
    ON ple_data.content_subject (lower(name));

CREATE INDEX published_question_available_discovery_idx
    ON ple_data.published_question(question_id) WHERE availability = 'available';

CREATE INDEX published_question_metadata_search_idx ON ple_data.published_question_metadata
    USING gin (to_tsvector('simple', question_title || ' ' || question_description));

CREATE UNIQUE INDEX question_ownership_event_initial_once
    ON ple_data.question_ownership_event(question_id) WHERE event_kind = 'initial';

CREATE INDEX question_star_instructor_collection_idx
    ON ple_data.question_star(instructor_account_id, starred_at DESC, question_id);

CREATE INDEX question_watch_instructor_collection_idx
    ON ple_data.question_watch(instructor_account_id, watched_at DESC, question_id);

CREATE INDEX library_improvement_thread_object_idx
    ON ple_data.library_improvement_thread(object_kind, public_object_id, created_at, thread_id);

CREATE INDEX library_improvement_post_thread_idx
    ON ple_data.library_improvement_post(thread_id, created_at, post_id);

CREATE INDEX library_impact_notice_object_idx
    ON ple_data.library_impact_notice(object_kind, public_object_id, created_at DESC, impact_notice_id);

CREATE INDEX library_watch_event_pending_idx
    ON ple_data.library_watch_event(occurred_at, event_id) WHERE processed_at IS NULL;

SET LOCAL ROLE ple_private_owner;

CREATE INDEX library_watch_notification_recipient_idx
    ON ple_private.library_watch_notification(
        recipient_account_id, occurred_at DESC, notification_id DESC
    );

CREATE UNIQUE INDEX object_storage_check_delivery_once
    ON ple_private.object_storage_check (delivery_id) WHERE delivery_id IS NOT NULL;

CREATE UNIQUE INDEX object_storage_check_banner_subject_once
    ON ple_private.object_storage_check (course_banner_storage_subject_id)
    WHERE course_banner_storage_subject_id IS NOT NULL;

SET LOCAL ROLE ple_data_owner;

CREATE INDEX blueprint_course_available_owner_idx
    ON ple_data.blueprint_course (availability, owner_account_id, reference_number);

CREATE INDEX blueprint_revision_question_pin_question_idx
    ON ple_data.blueprint_revision_question_pin (question_id, question_revision_number);

CREATE INDEX blueprint_course_fork_source_idx ON ple_data.blueprint_course_fork (
    source_blueprint_course_reference_number, source_blueprint_revision_number
);

CREATE INDEX blueprint_course_star_instructor_collection_idx
    ON ple_data.blueprint_course_star (
        instructor_account_id, starred_at DESC, blueprint_course_reference_number
    );

CREATE INDEX blueprint_course_watch_instructor_collection_idx
    ON ple_data.blueprint_course_watch (
        instructor_account_id, watched_at DESC, blueprint_course_reference_number
    );

SET LOCAL ROLE ple_private_owner;

CREATE INDEX blueprint_course_watch_notification_recipient_idx
    ON ple_private.blueprint_course_watch_notification (
        recipient_account_id, occurred_at DESC, notification_id DESC
    );

SET LOCAL ROLE ple_data_owner;

CREATE INDEX course_membership_event_current_lookup_idx
    ON ple_data.course_membership_event (membership_id, occurred_at DESC, course_membership_event_id DESC);

CREATE INDEX course_membership_account_course_idx ON ple_data.course_membership (account_id, course_id);

CREATE INDEX student_record_account_course_idx ON ple_data.student_record (student_account_id, course_id);

CREATE INDEX assessment_course_due_idx ON ple_data.assessment(course_id, due_at, reference_number)
    WHERE assessment_status IN ('unreleased', 'released');

CREATE INDEX assessment_due_soon_idx ON ple_data.assessment(due_at, course_id, reference_number)
    WHERE assessment_status IN ('unreleased', 'released') AND due_at IS NOT NULL;

CREATE INDEX assessment_entry_current_idx ON ple_data.assessment_entry(assessment_id, authored_position)
    WHERE availability = 'available';

SET LOCAL ROLE ple_private_owner;



-- The owner predicate and stable list ordering are the complete collection
-- workload; this index avoids scanning other Instructors' private Templates.
CREATE INDEX assessment_template_owner_list_idx
    ON ple_private.assessment_template (
        owner_account_id, template_name, assessment_template_id
    );

SET LOCAL ROLE ple_data_owner;

CREATE INDEX blueprint_course_instance_source_course_idx
    ON ple_data.blueprint_course_instance_source (source_course_id);

SET LOCAL ROLE ple_private_owner;

CREATE INDEX assessment_attempt_student_assessment_lookup_idx
    ON ple_private.assessment_attempt(student_record_id, assessment_id, assessment_attempt_number DESC);

CREATE INDEX assessment_attempt_expiry_sweep_idx
    ON ple_private.assessment_attempt(expires_at, assessment_attempt_id)
    WHERE expires_at IS NOT NULL;

CREATE INDEX imathas_question_backend_session_active_lookup_idx ON ple_private.imathas_question_backend_session(imathas_question_backend_session_id, account_id, expires_at) WHERE revoked_at IS NULL AND consumed_at IS NULL;

CREATE INDEX job_ready_claim_idx ON ple_private.job(available_at, job_id)
    WHERE state = 'ready';

CREATE INDEX job_expired_lease_idx ON ple_private.job(lease_expires_at, job_id)
    WHERE state = 'leased';

CREATE INDEX course_retention_notification_claim_idx
    ON ple_private.course_retention_notification (due_at, notification_id)
    WHERE provider_accepted_at IS NULL;

