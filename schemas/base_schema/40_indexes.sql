-- Explicit indexes. Constraint indexes are created with tables.

SET LOCAL ROLE ple_private_owner;

CREATE INDEX email_authentication_challenge_active_token_idx
ON ple_private.email_authentication_challenge (token_hash, expires_at) WHERE consumed_at IS NULL;

CREATE INDEX passkey_active_account_idx ON ple_private.passkey (account_id, created_at)
WHERE revoked_at IS NULL;

CREATE INDEX sysadmin_totp_attestation_active_idx
ON ple_private.sysadmin_totp_attestation (sysadmin_totp_attestation_id, expires_at) WHERE consumed_at IS NULL;

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
    ON ple_data.published_question(published_question_id) WHERE availability = 'available';

CREATE INDEX published_question_metadata_search_idx ON ple_data.published_question_metadata
    USING gin (to_tsvector('simple', question_title || ' ' || question_description));

CREATE UNIQUE INDEX question_ownership_event_initial_once
    ON ple_data.question_ownership_event(published_question_id) WHERE event_kind = 'initial';

CREATE INDEX question_star_instructor_collection_idx
    ON ple_data.question_star(instructor_account_id, starred_at DESC, published_question_id);

CREATE INDEX question_watch_instructor_collection_idx
    ON ple_data.question_watch(instructor_account_id, watched_at DESC, published_question_id);

CREATE INDEX library_improvement_thread_object_idx
    ON ple_data.library_improvement_thread(object_kind, public_object_id, created_at, library_improvement_thread_id);

CREATE INDEX library_improvement_post_thread_idx
    ON ple_data.library_improvement_post(library_improvement_thread_id, created_at, post_id);

CREATE INDEX library_impact_notice_object_idx
    ON ple_data.library_impact_notice(object_kind, public_object_id, created_at DESC, impact_notice_id);

CREATE INDEX library_watch_event_pending_idx
    ON ple_data.library_watch_event(occurred_at, event_id) WHERE processed_at IS NULL;

SET LOCAL ROLE ple_data_owner;

-- Inbox reads filter by recipient and join to event; the composite PK leads
-- with event_id, so this referencing-side index serves the Watch feed.
CREATE INDEX library_watch_event_recipient_account_idx
    ON ple_data.library_watch_event_recipient(recipient_account_id, library_watch_event_id);

SET LOCAL ROLE ple_private_owner;

CREATE UNIQUE INDEX object_storage_check_delivery_once
    ON ple_private.object_storage_check (object_delivery_id) WHERE object_delivery_id IS NOT NULL;

CREATE UNIQUE INDEX object_storage_check_banner_subject_once
    ON ple_private.object_storage_check (course_banner_storage_subject_id)
    WHERE course_banner_storage_subject_id IS NOT NULL;

SET LOCAL ROLE ple_data_owner;

CREATE INDEX blueprint_course_available_owner_idx
    ON ple_data.blueprint_course (availability, owner_account_id, blueprint_course_id);

CREATE INDEX blueprint_revision_question_pin_question_idx
    ON ple_data.blueprint_revision_question_pin (published_question_id, question_revision_number);

CREATE INDEX blueprint_course_fork_source_idx ON ple_data.blueprint_course_fork (
    source_blueprint_course_id, source_blueprint_revision_number
);

CREATE INDEX blueprint_course_star_instructor_collection_idx
    ON ple_data.blueprint_course_star (
        instructor_account_id, starred_at DESC, blueprint_course_id
    );

CREATE INDEX blueprint_course_watch_instructor_collection_idx
    ON ple_data.blueprint_course_watch (
        instructor_account_id, watched_at DESC, blueprint_course_id
    );

SET LOCAL ROLE ple_private_owner;

CREATE INDEX blueprint_course_watch_notification_recipient_idx
    ON ple_private.blueprint_course_watch_notification (
        recipient_account_id, occurred_at DESC, notification_id DESC
    );

SET LOCAL ROLE ple_data_owner;

CREATE INDEX course_membership_account_course_idx ON ple_data.course_membership (account_id, course_instance_id);

CREATE INDEX student_record_account_course_idx ON ple_data.student_record (student_account_id, course_instance_id);

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
    ON ple_data.blueprint_course_instance_source (source_course_instance_id);

SET LOCAL ROLE ple_private_owner;

CREATE INDEX assessment_attempt_student_assessment_lookup_idx
    ON ple_private.assessment_attempt(student_record_id, assessment_id, assessment_attempt_number DESC);

-- Unrelease deletes Student Work by Assessment ID. The primary key leads with
-- course_instance_id, so this referencing-side index matches that purge and
-- covers the (assessment_id, course_instance_id) foreign key.
CREATE INDEX assessment_attempt_assessment_id_idx
    ON ple_private.assessment_attempt(assessment_id, course_instance_id);

CREATE INDEX assessment_attempt_expiry_sweep_idx
    ON ple_private.assessment_attempt(expires_at, assessment_attempt_id)
    WHERE expires_at IS NOT NULL;

CREATE INDEX job_ready_claim_idx ON ple_private.job(available_at, job_id)
    WHERE state = 'ready';

CREATE INDEX job_expired_lease_idx ON ple_private.job(lease_expires_at, job_id)
    WHERE state = 'leased';

CREATE INDEX course_retention_notification_claim_idx
    ON ple_private.course_retention_notification (due_at, notification_id)
    WHERE provider_accepted_at IS NULL;

-- Authentication growth sweep (WP-3.7): 7-day grace after expiry, revoke, or
-- consume. Partial indexes match those predicates.
CREATE INDEX authenticated_session_expired_sweep_idx
    ON ple_private.authenticated_session (expires_at)
    WHERE revoked_at IS NULL;

CREATE INDEX authenticated_session_revoked_sweep_idx
    ON ple_private.authenticated_session (revoked_at)
    WHERE revoked_at IS NOT NULL;

CREATE INDEX email_authentication_challenge_expired_sweep_idx
    ON ple_private.email_authentication_challenge (expires_at)
    WHERE consumed_at IS NULL;

CREATE INDEX email_authentication_challenge_consumed_sweep_idx
    ON ple_private.email_authentication_challenge (consumed_at)
    WHERE consumed_at IS NOT NULL;

CREATE INDEX passkey_ceremony_expired_sweep_idx
    ON ple_private.passkey_ceremony (expires_at)
    WHERE consumed_at IS NULL;

CREATE INDEX passkey_ceremony_consumed_sweep_idx
    ON ple_private.passkey_ceremony (consumed_at)
    WHERE consumed_at IS NOT NULL;

CREATE INDEX authentication_rate_limit_sweep_idx
    ON ple_private.authentication_rate_limit (window_started_at);


-- Referencing-side indexes for every foreign key not already the
-- leading columns of a PRIMARY KEY, UNIQUE constraint, or earlier
-- explicit index. DATABASE_STYLE.md checklist 14: child rows of
-- purged, unreleased, or closed parents, and parent-to-child
-- lookups, need these covering indexes. No FK is left unindexed.

SET LOCAL ROLE ple_audit_owner;

CREATE INDEX assessment_unrelease_event_actor_account_id_fk_idx
    ON ple_audit.assessment_unrelease_event (actor_account_id);

CREATE INDEX assessment_unrelease_event_assessment_id_fk_idx
    ON ple_audit.assessment_unrelease_event (assessment_id);

CREATE INDEX course_instance_creation_event_blueprint_course_f6406743_fk_idx
    ON ple_audit.course_instance_creation_event (blueprint_course_id, blueprint_revision_number);

CREATE INDEX course_instance_creation_event_assigned_instruc_af26ef10_fk_idx
    ON ple_audit.course_instance_creation_event (assigned_instructor_account_id);

CREATE INDEX course_instance_creation_event_created_by_account_id_fk_idx
    ON ple_audit.course_instance_creation_event (created_by_account_id);

CREATE INDEX course_roster_event_acting_account_id_fk_idx
    ON ple_audit.course_roster_event (acting_account_id);

CREATE INDEX course_roster_event_course_instance_id_fk_idx
    ON ple_audit.course_roster_event (course_instance_id);

CREATE INDEX course_roster_event_student_account_id_fk_idx
    ON ple_audit.course_roster_event (student_account_id);

CREATE INDEX forced_question_correction_assessment_attempt_t_e6cf004d_fk_idx
    ON ple_audit.forced_question_correction_assessment_attempt_target (course_instance_id, assessment_attempt_id);

CREATE INDEX forced_question_correction_assessment_attempt_t_056ba53c_fk_idx
    ON ple_audit.forced_question_correction_assessment_attempt_target (forced_question_correction_id);

CREATE INDEX forced_question_correction_issued_question_targ_42536c1f_fk_idx
    ON ple_audit.forced_question_correction_issued_question_target (course_instance_id, issued_question_id);

CREATE INDEX forced_question_correction_issued_question_targ_bfe39ee6_fk_idx
    ON ple_audit.forced_question_correction_issued_question_target (forced_question_correction_id);

CREATE INDEX instructor_account_creation_event_created_by_sy_4138130d_fk_idx
    ON ple_audit.instructor_account_creation_event (created_by_sysadmin_account_id, created_by_sysadmin_product_role);

CREATE INDEX instructor_account_creation_event_created_instr_838b23f7_fk_idx
    ON ple_audit.instructor_account_creation_event (created_instructor_account_id, created_instructor_product_role);

CREATE INDEX instructor_account_creation_event_instructor_id_2eefdbbf_fk_idx
    ON ple_audit.instructor_account_creation_event (instructor_identity_vetting_decision_id);

CREATE INDEX instructor_identity_vetting_decision_completed__5e383e9f_fk_idx
    ON ple_audit.instructor_identity_vetting_decision (completed_by_sysadmin_account_id, completed_by_sysadmin_product_role);

CREATE INDEX object_cleanup_receipt_object_cleanup_manifest__f9f85e91_fk_idx
    ON ple_audit.object_cleanup_receipt (object_cleanup_manifest_id, disposition);

CREATE INDEX support_repair_capability_event_issuer_account_id_fk_idx
    ON ple_audit.support_repair_capability_event (issuer_account_id);

CREATE INDEX support_repair_capability_event_support_repair__975ebcdb_fk_idx
    ON ple_audit.support_repair_capability_event (support_repair_capability_id);

CREATE INDEX support_repair_capability_event_sysadmin_account_id_fk_idx
    ON ple_audit.support_repair_capability_event (sysadmin_account_id);

SET LOCAL ROLE ple_data_owner;

CREATE INDEX assessment_source_blueprint_course_id_a95c83d2_fk_idx
    ON ple_data.assessment (source_blueprint_course_id, source_blueprint_revision_number, source_blueprint_assessment_id);

CREATE INDEX assessment_assessment_policy_snapshot_id_fk_idx
    ON ple_data.assessment (assessment_policy_snapshot_id);

CREATE INDEX assessment_course_instance_id_fk_idx
    ON ple_data.assessment (course_instance_id);

CREATE INDEX assessment_entry_pool_question_pool_id_fk_idx
    ON ple_data.assessment_entry_pool (question_pool_id);

CREATE INDEX assessment_entry_question_published_question_id_570ef8b8_fk_idx
    ON ple_data.assessment_entry_question (published_question_id, question_revision_number);

CREATE INDEX blueprint_change_proposal_source_blueprint_cour_4dd90b27_fk_idx
    ON ple_data.blueprint_change_proposal (source_blueprint_course_id, source_blueprint_edit_number);

CREATE INDEX blueprint_change_proposal_source_blueprint_cour_0eaaf160_fk_idx
    ON ple_data.blueprint_change_proposal (source_blueprint_course_id, source_revision_number);

CREATE INDEX blueprint_change_proposal_target_blueprint_cour_8b0346f6_fk_idx
    ON ple_data.blueprint_change_proposal (target_blueprint_course_id, target_blueprint_edit_number);

CREATE INDEX blueprint_change_proposal_target_blueprint_cour_ac785fb9_fk_idx
    ON ple_data.blueprint_change_proposal (target_blueprint_course_id, target_revision_number);

CREATE INDEX blueprint_change_proposal_proposer_account_id_fk_idx
    ON ple_data.blueprint_change_proposal (proposer_account_id);

CREATE INDEX blueprint_change_proposal_acceptance_target_blu_103f00b2_fk_idx
    ON ple_data.blueprint_change_proposal_acceptance (target_blueprint_course_id, resulting_blueprint_edit_number);

CREATE INDEX blueprint_change_proposal_acceptance_target_blu_32957868_fk_idx
    ON ple_data.blueprint_change_proposal_acceptance (target_blueprint_course_id, resulting_revision_number);

CREATE INDEX blueprint_change_proposal_acceptance_actor_account_id_fk_idx
    ON ple_data.blueprint_change_proposal_acceptance (actor_account_id);

CREATE INDEX blueprint_course_blueprint_course_id_a1565d12_fk_idx
    ON ple_data.blueprint_course (blueprint_course_id, current_blueprint_revision_number);

CREATE INDEX blueprint_course_content_subject_id_a63d26bc_fk_idx
    ON ple_data.blueprint_course (content_subject_id, content_discipline_id);

CREATE INDEX blueprint_course_content_subject_id_content_topic_id_fk_idx
    ON ple_data.blueprint_course (content_subject_id, content_topic_id);

CREATE INDEX blueprint_course_content_topic_id_content_subtopic_id_fk_idx
    ON ple_data.blueprint_course (content_topic_id, content_subtopic_id);

CREATE INDEX blueprint_course_content_discipline_id_fk_idx
    ON ple_data.blueprint_course (content_discipline_id);

CREATE INDEX blueprint_course_owner_account_id_fk_idx
    ON ple_data.blueprint_course (owner_account_id);

CREATE INDEX blueprint_course_create_receipt_blueprint_cours_a188b3c4_fk_idx
    ON ple_data.blueprint_course_create_receipt (blueprint_course_id, blueprint_revision_number);

CREATE INDEX blueprint_course_fork_receipt_source_blueprint__840527c5_fk_idx
    ON ple_data.blueprint_course_fork_receipt (source_blueprint_course_id, source_blueprint_revision_number);

CREATE INDEX blueprint_course_fork_receipt_blueprint_course_id_fk_idx
    ON ple_data.blueprint_course_fork_receipt (blueprint_course_id);

CREATE INDEX blueprint_course_save_receipt_actor_account_id_fk_idx
    ON ple_data.blueprint_course_save_receipt (actor_account_id);

CREATE INDEX blueprint_metadata_event_actor_account_id_fk_idx
    ON ple_data.blueprint_metadata_event (actor_account_id);

CREATE INDEX blueprint_revision_event_actor_account_id_fk_idx
    ON ple_data.blueprint_revision_event (actor_account_id);

CREATE INDEX content_subject_discipline_content_discipline_id_fk_idx
    ON ple_data.content_subject_discipline (content_discipline_id);

CREATE INDEX course_banner_source_object_record_id_fk_idx
    ON ple_data.course_banner (source_object_record_id);

CREATE INDEX course_banner_delivery_course_instance_id_364cac3c_fk_idx
    ON ple_data.course_banner_delivery (course_instance_id, course_banner_id, rendition_kind, object_record_id);

CREATE INDEX course_banner_delivery_object_delivery_id_216c0c32_fk_idx
    ON ple_data.course_banner_delivery (object_delivery_id, object_record_id);

CREATE INDEX course_banner_rendition_object_record_id_fk_idx
    ON ple_data.course_banner_rendition (object_record_id);

CREATE INDEX course_instance_blueprint_course_id_4a126ed2_fk_idx
    ON ple_data.course_instance (blueprint_course_id, blueprint_revision_number);

CREATE INDEX course_instance_content_subject_id_content_discipline_id_fk_idx
    ON ple_data.course_instance (content_subject_id, content_discipline_id);

CREATE INDEX course_instance_content_subject_id_content_topic_id_fk_idx
    ON ple_data.course_instance (content_subject_id, content_topic_id);

CREATE INDEX course_instance_content_topic_id_content_subtopic_id_fk_idx
    ON ple_data.course_instance (content_topic_id, content_subtopic_id);

CREATE INDEX course_instance_course_instance_id_ce017de1_fk_idx
    ON ple_data.course_instance (course_instance_id, current_course_banner_id);

CREATE INDEX course_instance_content_discipline_id_fk_idx
    ON ple_data.course_instance (content_discipline_id);

CREATE INDEX course_instance_course_theme_id_fk_idx
    ON ple_data.course_instance (course_theme_id);

CREATE INDEX course_membership_account_id_role_fk_idx
    ON ple_data.course_membership (account_id, role);

CREATE INDEX course_membership_course_instance_id_fk_idx
    ON ple_data.course_membership (course_instance_id);

CREATE INDEX course_membership_student_record_id_fk_idx
    ON ple_data.course_membership (student_record_id);

CREATE INDEX course_object_delivery_object_delivery_id_e88c74aa_fk_idx
    ON ple_data.course_object_delivery (object_delivery_id, object_record_id);

CREATE INDEX course_object_delivery_course_instance_id_fk_idx
    ON ple_data.course_object_delivery (course_instance_id);

CREATE INDEX course_origin_blueprint_course_id_95fa803f_fk_idx
    ON ple_data.course_origin (blueprint_course_id, blueprint_revision_number);

CREATE INDEX course_origin_source_course_instance_id_fk_idx
    ON ple_data.course_origin (source_course_instance_id);

CREATE INDEX forced_question_correction_approved_by_account__198a0268_fk_idx
    ON ple_data.forced_question_correction (approved_by_account_id, approver_role);

CREATE INDEX forced_question_correction_replacement_question_67ad7428_fk_idx
    ON ple_data.forced_question_correction (replacement_question_id, replacement_revision_number);

CREATE INDEX library_impact_notice_cancelled_by_account_id_fk_idx
    ON ple_data.library_impact_notice (cancelled_by_account_id);

CREATE INDEX library_impact_notice_created_by_account_id_fk_idx
    ON ple_data.library_impact_notice (created_by_account_id);

CREATE INDEX library_improvement_post_author_account_id_fk_idx
    ON ple_data.library_improvement_post (author_account_id);

CREATE INDEX library_improvement_thread_created_by_account_id_fk_idx
    ON ple_data.library_improvement_thread (created_by_account_id);

CREATE INDEX library_improvement_thread_resolved_by_account_id_fk_idx
    ON ple_data.library_improvement_thread (resolved_by_account_id);

CREATE INDEX profile_image_delivery_object_record_id_fk_idx
    ON ple_data.profile_image_delivery (object_record_id);

CREATE INDEX published_question_metadata_content_subject_id_2904f118_fk_idx
    ON ple_data.published_question_metadata (content_subject_id, content_discipline_id);

CREATE INDEX published_question_metadata_content_subject_id_eed7807b_fk_idx
    ON ple_data.published_question_metadata (content_subject_id, content_topic_id);

CREATE INDEX published_question_metadata_content_topic_id_4033d80b_fk_idx
    ON ple_data.published_question_metadata (content_topic_id, content_subtopic_id);

CREATE INDEX question_image_delivery_object_delivery_id_72e8beb7_fk_idx
    ON ple_data.question_image_delivery (object_delivery_id, object_record_id);

CREATE INDEX question_image_delivery_published_question_id_db298a25_fk_idx
    ON ple_data.question_image_delivery (published_question_id, revision_number);

CREATE INDEX question_availability_event_actor_account_id_fk_idx
    ON ple_data.question_availability_event (actor_account_id);

CREATE INDEX question_change_event_recorded_by_account_id_fk_idx
    ON ple_data.question_change_event (recorded_by_account_id);

CREATE INDEX question_fork_source_source_question_id_e7bd1bc8_fk_idx
    ON ple_data.question_fork_source (source_question_id, source_revision_number);

CREATE INDEX question_ownership_event_owner_account_id_fk_idx
    ON ple_data.question_ownership_event (owner_account_id);

CREATE INDEX question_ownership_event_recorded_by_account_id_fk_idx
    ON ple_data.question_ownership_event (recorded_by_account_id);

CREATE INDEX question_pool_content_subject_id_content_discipline_id_fk_idx
    ON ple_data.question_pool (content_subject_id, content_discipline_id);

CREATE INDEX question_pool_content_subject_id_content_topic_id_fk_idx
    ON ple_data.question_pool (content_subject_id, content_topic_id);

CREATE INDEX question_pool_content_topic_id_content_subtopic_id_fk_idx
    ON ple_data.question_pool (content_topic_id, content_subtopic_id);

CREATE INDEX question_pool_interchangeability_attested_by_account_id_fk_idx
    ON ple_data.question_pool (interchangeability_attested_by_account_id);

CREATE INDEX question_pool_source_question_pool_id_fk_idx
    ON ple_data.question_pool (source_question_pool_id);

CREATE INDEX question_pool_member_published_question_id_c92544b6_fk_idx
    ON ple_data.question_pool_member (published_question_id, question_revision_number);

CREATE INDEX question_pool_member_statistics_published_question_id_fk_idx
    ON ple_data.question_pool_member_statistics (published_question_id);

CREATE INDEX question_pool_star_instructor_account_id_fk_idx
    ON ple_data.question_pool_star (instructor_account_id);

CREATE INDEX question_pool_watch_instructor_account_id_fk_idx
    ON ple_data.question_pool_watch (instructor_account_id);

CREATE INDEX question_publication_event_actor_account_id_fk_idx
    ON ple_data.question_publication_event (actor_account_id);

CREATE INDEX question_revision_acceptance_published_question_8abd36b6_fk_idx
    ON ple_data.question_revision_acceptance (published_question_id, parent_revision_number);

CREATE INDEX question_revision_acceptance_accepted_by_account_id_fk_idx
    ON ple_data.question_revision_acceptance (accepted_by_account_id);

CREATE INDEX question_revision_acceptance_editor_account_id_fk_idx
    ON ple_data.question_revision_acceptance (editor_account_id);

CREATE INDEX question_revision_authorship_author_account_id_fk_idx
    ON ple_data.question_revision_authorship (author_account_id);

SET LOCAL ROLE ple_private_owner;

CREATE INDEX account_avatar_profile_image_id_10940e09_fk_idx
    ON ple_private.account_avatar (profile_image_id, profile_image_delivery_id);

CREATE INDEX account_avatar_provided_avatar_id_fk_idx
    ON ple_private.account_avatar (provided_avatar_id);

CREATE INDEX account_state_event_account_id_fk_idx
    ON ple_private.account_state_event (account_id);

CREATE INDEX assessment_attempt_assessment_attempt_limit_acc_f173cbca_fk_idx
    ON ple_private.assessment_attempt (assessment_attempt_limit_accommodation_id, student_record_id, assessment_id, course_instance_id);

CREATE INDEX assessment_attempt_schedule_accommodation_id_7a50f886_fk_idx
    ON ple_private.assessment_attempt (schedule_accommodation_id, student_record_id, assessment_id, course_instance_id);

CREATE INDEX assessment_attempt_time_limit_accommodation_id_45e3ebf6_fk_idx
    ON ple_private.assessment_attempt (time_limit_accommodation_id, student_record_id, assessment_id, course_instance_id);

CREATE INDEX assessment_attempt_student_record_id_course_instance_id_fk_idx
    ON ple_private.assessment_attempt (student_record_id, course_instance_id);

CREATE INDEX assessment_attempt_assessment_policy_snapshot_id_fk_idx
    ON ple_private.assessment_attempt (assessment_policy_snapshot_id);

CREATE INDEX assessment_attempt_saved_response_course_instan_71d206a1_fk_idx
    ON ple_private.assessment_attempt_saved_response (course_instance_id, assessment_submission_id);

CREATE INDEX assessment_entry_snapshot_published_question_id_22f37768_fk_idx
    ON ple_private.assessment_entry_snapshot (published_question_id, question_revision_number);

CREATE INDEX assessment_entry_snapshot_question_pool_id_fk_idx
    ON ple_private.assessment_entry_snapshot (question_pool_id);

CREATE INDEX assessment_submission_authorized_by_account_id_fk_idx
    ON ple_private.assessment_submission (authorized_by_account_id);

CREATE INDEX assessment_template_assessment_policy_snapshot_id_fk_idx
    ON ple_private.assessment_template (assessment_policy_snapshot_id);

CREATE INDEX authenticated_session_account_id_product_role_fk_idx
    ON ple_private.authenticated_session (account_id, product_role);

CREATE INDEX authoring_workspace_owner_account_id_fk_idx
    ON ple_private.authoring_workspace (owner_account_id);

CREATE INDEX authoring_workspace_collaborator_event_actor_account_id_fk_idx
    ON ple_private.authoring_workspace_collaborator_event (actor_account_id);

CREATE INDEX authoring_workspace_collaborator_event_collabor_8f92f6cb_fk_idx
    ON ple_private.authoring_workspace_collaborator_event (collaborator_account_id);

CREATE INDEX blueprint_course_watch_notification_blueprint_course_id_fk_idx
    ON ple_private.blueprint_course_watch_notification (blueprint_course_id);

CREATE INDEX course_banner_storage_subject_course_instance_i_c9e745db_fk_idx
    ON ple_private.course_banner_storage_subject (course_instance_id, course_banner_id);

CREATE INDEX course_banner_storage_subject_course_banner_upload_id_fk_idx
    ON ple_private.course_banner_storage_subject (course_banner_upload_id);

CREATE INDEX course_banner_upload_account_id_fk_idx
    ON ple_private.course_banner_upload (account_id);

CREATE INDEX course_banner_upload_course_instance_id_fk_idx
    ON ple_private.course_banner_upload (course_instance_id);

CREATE INDEX course_banner_work_course_banner_storage_subjec_1ea914b2_fk_idx
    ON ple_private.course_banner_work (course_banner_storage_subject_id, object_record_id);

CREATE INDEX course_banner_work_object_delivery_id_object_record_id_fk_idx
    ON ple_private.course_banner_work (object_delivery_id, object_record_id);

CREATE INDEX course_banner_work_course_instance_id_fk_idx
    ON ple_private.course_banner_work (course_instance_id);

CREATE INDEX course_invitation_inviting_instructor_account_i_759dd1e2_fk_idx
    ON ple_private.course_invitation (inviting_instructor_account_id, inviting_instructor_role);

CREATE INDEX course_invitation_target_account_id_membership_role_fk_idx
    ON ple_private.course_invitation (target_account_id, membership_role);

CREATE INDEX course_invitation_course_instance_id_fk_idx
    ON ple_private.course_invitation (course_instance_id);

CREATE INDEX course_invitation_event_performed_by_account_id_fk_idx
    ON ple_private.course_invitation_event (performed_by_account_id);

CREATE INDEX course_retention_notification_recipient_account_5bb3f876_fk_idx
    ON ple_private.course_retention_notification (recipient_account_id, recipient_product_role);

CREATE INDEX course_roster_profile_student_account_id_fk_idx
    ON ple_private.course_roster_profile (student_account_id);

CREATE INDEX draft_question_authoring_workspace_id_fk_idx
    ON ple_private.draft_question (authoring_workspace_id);

CREATE INDEX draft_question_image_draft_question_id_b5a44014_fk_idx
    ON ple_private.draft_question_image (draft_question_id, authoring_workspace_id);

CREATE INDEX draft_question_fork_source_source_question_id_86c817bf_fk_idx
    ON ple_private.draft_question_fork_source (source_question_id, source_revision_number);

CREATE INDEX draft_question_source_binding_source_object_record_id_fk_idx
    ON ple_private.draft_question_source_binding (source_object_record_id);

CREATE INDEX email_authentication_challenge_target_account_id_fk_idx
    ON ple_private.email_authentication_challenge (target_account_id);

CREATE INDEX issued_question_course_instance_id_31d5d1c2_fk_idx
    ON ple_private.issued_question (course_instance_id, question_pool_selection_id, question_pool_member_position, published_question_id, revision_number);

CREATE INDEX issued_question_course_instance_id_b08795ae_fk_idx
    ON ple_private.issued_question (course_instance_id, question_pool_selection_id, assessment_attempt_id, assessment_entry_id);

CREATE INDEX issued_question_published_question_id_revision_number_fk_idx
    ON ple_private.issued_question (published_question_id, revision_number);

CREATE INDEX issued_question_assessment_entry_snapshot_id_fk_idx
    ON ple_private.issued_question (assessment_entry_snapshot_id);

CREATE INDEX job_published_question_id_revision_number_fk_idx
    ON ple_private.job (published_question_id, revision_number);

CREATE INDEX object_cleanup_manifest_object_storage_check_id_fk_idx
    ON ple_private.object_cleanup_manifest (object_storage_check_id);

CREATE INDEX passkey_ceremony_target_account_id_fk_idx
    ON ple_private.passkey_ceremony (target_account_id);

CREATE INDEX profile_image_work_object_delivery_id_object_record_id_fk_idx
    ON ple_private.profile_image_work (object_delivery_id, object_record_id);

CREATE INDEX profile_image_work_account_id_fk_idx
    ON ple_private.profile_image_work (account_id);

CREATE INDEX profile_image_work_object_record_id_fk_idx
    ON ple_private.profile_image_work (object_record_id);

CREATE INDEX question_image_publication_object_delivery_id_e5836f6f_fk_idx
    ON ple_private.question_image_publication (object_delivery_id, public_object_id);

CREATE INDEX question_image_publication_source_object_record_id_fk_idx
    ON ple_private.question_image_publication (source_object_record_id);

CREATE INDEX question_attempt_delivery_toolchain_id_fk_idx
    ON ple_private.question_attempt (delivery_toolchain_id);

CREATE INDEX question_attempt_source_object_record_id_fk_idx
    ON ple_private.question_attempt (source_object_record_id);

CREATE INDEX question_folder_owner_account_id_fk_idx
    ON ple_private.question_folder (owner_account_id);

CREATE INDEX question_folder_entry_published_question_id_fk_idx
    ON ple_private.question_folder_entry (published_question_id);

CREATE INDEX question_pool_selected_item_published_question__5b4ced3b_fk_idx
    ON ple_private.question_pool_selected_item (published_question_id, revision_number);

CREATE INDEX question_pool_selection_question_pool_id_fk_idx
    ON ple_private.question_pool_selection (question_pool_id);

CREATE INDEX question_revision_source_binding_source_object_record_id_fk_idx
    ON ple_private.question_revision_source_binding (source_object_record_id);

CREATE INDEX question_statistics_observation_receipt_course__d3f8d499_fk_idx
    ON ple_private.question_statistics_observation_receipt (course_instance_id, assessment_submission_id);

CREATE INDEX question_statistics_observation_receipt_publish_de307fa4_fk_idx
    ON ple_private.question_statistics_observation_receipt (published_question_id, revision_number);

CREATE INDEX saved_question_search_owner_account_id_fk_idx
    ON ple_private.saved_question_search (owner_account_id);

CREATE INDEX student_assessment_accommodation_assessment_id_d947306c_fk_idx
    ON ple_private.student_assessment_accommodation (assessment_id, course_instance_id);

CREATE INDEX student_assessment_accommodation_student_record_5951ea54_fk_idx
    ON ple_private.student_assessment_accommodation (student_record_id, course_instance_id);

CREATE INDEX support_repair_capability_sysadmin_account_id_49d4b3ce_fk_idx
    ON ple_private.support_repair_capability (sysadmin_account_id, sysadmin_role);

CREATE INDEX support_repair_capability_issuer_account_id_fk_idx
    ON ple_private.support_repair_capability (issuer_account_id);

CREATE INDEX sysadmin_totp_attestation_account_id_fk_idx
    ON ple_private.sysadmin_totp_attestation (account_id);

CREATE INDEX sysadmin_totp_verification_attempt_account_id_fk_idx
    ON ple_private.sysadmin_totp_verification_attempt (account_id);
