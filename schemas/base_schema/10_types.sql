-- Enums and domains. One definition per vocabulary.

SET LOCAL ROLE ple_data_owner;

CREATE TYPE ple_data.bloom_cognitive_process AS ENUM (
    'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create'
);
CREATE TYPE ple_data.bloom_knowledge_dimension AS ENUM (
    'Factual Knowledge', 'Conceptual Knowledge', 'Procedural Knowledge',
    'Metacognitive Knowledge'
);
CREATE TYPE ple_data.product_role AS ENUM (
    'student', 'instructor', 'sysadmin'
);
CREATE TYPE ple_data.assessment_type AS ENUM (
    'regular_assignment', 'practice_question_assignment', 'quiz',
    'exam', 'bonus_assignment'
);
CREATE TYPE ple_data.late_work_rule AS ENUM (
    'accept', 'mark_late', 'reject'
);
CREATE TYPE ple_data.question_variation_rule AS ENUM (
    'reuse_variation', 'new_variation'
);
CREATE TYPE ple_data.question_order_rule AS ENUM (
    'authored_order', 'shuffled'
);
CREATE TYPE ple_data.selected_question_order AS ENUM (
    'question_pool_order', 'random_order'
);
CREATE TYPE ple_data.feedback_release AS ENUM (
    'during_attempt', 'after_submit', 'after_due', 'after_close', 'never'
);
CREATE TYPE ple_data.scoring_rule AS ENUM (
    'normal', 'full_credit', 'extra_credit', 'excluded'
);
CREATE TYPE ple_data.entry_kind AS ENUM (
    'fixed_question', 'question_pool'
);
CREATE TYPE ple_data.entry_availability AS ENUM (
    'available', 'retired'
);
CREATE TYPE ple_data.question_backend AS ENUM (
    'ple', 'webwork'
);
CREATE TYPE ple_data.question_format AS ENUM (
    'pleQuestionJson', 'webworkPg', 'webworkPgml', 'qti'
);
CREATE TYPE ple_data.question_type AS ENUM (
    'multipleChoice', 'multipleAnswer', 'fillInBlank', 'multipleFillInBlank',
    'numeric', 'matching', 'ordering', 'hotspot'
);
CREATE TYPE ple_data.library_object_kind AS ENUM (
    'question', 'question_pool'
);
CREATE TYPE ple_data.library_watch_event_kind AS ENUM (
    'revision', 'members_changed', 'fork', 'improvement_thread', 'impact_notice'
);
CREATE TYPE ple_data.media_type AS ENUM (
    'image/png', 'image/jpeg', 'image/webp'
);
CREATE TYPE ple_data.object_storage_area AS ENUM (
    'private-content', 'public-assets', 'student-records', 'temp-processing'
);
CREATE TYPE ple_data.object_data_class AS ENUM (
    'authoring-content', 'course-appearance', 'profile-image', 'question-asset',
    'question-render', 'question-source', 'student-record', 'temporary-processing'
);
CREATE TYPE ple_data.lease_state AS ENUM (
    'pending', 'completed', 'repair-required', 'finalized'
);
CREATE TYPE ple_data.account_state AS ENUM (
    'active', 'deactivated', 'closed'
);
CREATE TYPE ple_data.assessment_origin_kind AS ENUM (
    'direct', 'adopted'
);
CREATE TYPE ple_data.assessment_status AS ENUM (
    'unreleased', 'released'
);
CREATE TYPE ple_data.course_source_kind AS ENUM (
    'empty', 'adopted'
);
CREATE TYPE ple_data.course_lifecycle_state AS ENUM (
    'active', 'inactive'
);
CREATE TYPE ple_data.retention_lifecycle_state AS ENUM (
    'active', 'archived', 'deleted'
);
CREATE TYPE ple_data.blueprint_availability AS ENUM (
    'private', 'public', 'archived'
);
CREATE TYPE ple_data.question_availability AS ENUM (
    'available', 'archived'
);
CREATE TYPE ple_data.membership_role AS ENUM (
    'student', 'instructor'
);
CREATE TYPE ple_data.membership_event_kind AS ENUM (
    'started', 'ended'
);
CREATE TYPE ple_data.public_id_object_kind AS ENUM (
    'account', 'assessment', 'blueprint_course', 'course_instance',
    'published_question', 'question_pool'
);
CREATE TYPE ple_data.license_spdx AS ENUM (
    'CC-BY-4.0', 'CC-BY-SA-4.0', 'CC0-1.0'
);
CREATE TYPE ple_data.correction_reason AS ENUM (
    'critical_correctness_flaw', 'security_flaw'
);
CREATE TYPE ple_data.thread_state AS ENUM (
    'open', 'resolved'
);
CREATE TYPE ple_data.notice_state AS ENUM (
    'active', 'cancelled'
);
CREATE TYPE ple_data.delivery_state AS ENUM (
    'available', 'pending', 'retired'
);
CREATE TYPE ple_data.cleanup_disposition AS ENUM (
    'already_absent', 'deleted', 'retained'
);
CREATE TYPE ple_data.invitation_response AS ENUM (
    'accepted', 'declined', 'revoked'
);
CREATE TYPE ple_data.roster_event_kind AS ENUM (
    'invitation_created', 'invitation_claimed', 'student_access_revoked'
);
CREATE TYPE ple_data.ownership_event_kind AS ENUM (
    'initial', 'transferred'
);
CREATE TYPE ple_data.import_format AS ENUM (
    'pleQuestionJson', 'webworkPg', 'webworkPgml', 'qti'
);
CREATE TYPE ple_data.import_state AS ENUM (
    'staged', 'committed'
);
CREATE TYPE ple_data.import_item_result AS ENUM (
    'accepted', 'rejected'
);
CREATE TYPE ple_data.avatar_kind AS ENUM (
    'provided', 'profile-image'
);
CREATE TYPE ple_data.object_work_operation AS ENUM (
    'put', 'delete'
);
CREATE TYPE ple_data.banner_work_operation AS ENUM (
    'put-source', 'put-rendition', 'put-upload',
    'delete-source', 'delete-rendition', 'delete-upload'
);
CREATE TYPE ple_data.banner_subject_kind AS ENUM (
    'source', 'upload'
);
CREATE TYPE ple_data.alternative_kind AS ENUM (
    'decorative', 'informative'
);
CREATE TYPE ple_data.publication_state AS ENUM (
    'pending', 'ready'
);
CREATE TYPE ple_data.job_state AS ENUM (
    'ready', 'leased', 'completed'
);
CREATE TYPE ple_data.issued_capability AS ENUM (
    'question_presentation', 'ple_question_json_presentation',
    'webwork_presentation', 'not_applicable'
);
CREATE TYPE ple_data.retention_action_kind AS ENUM (
    'warn_inactive', 'notify_archive'
);
CREATE TYPE ple_data.retention_failure_kind AS ENUM (
    'not_configured', 'provider_rejected', 'provider_transient'
);
CREATE TYPE ple_data.repair_result AS ENUM (
    'issued', 'revoked', 'used'
);
CREATE TYPE ple_data.watch_notification_event_kind AS ENUM (
    'revision', 'published', 'archived', 'restored'
);
CREATE TYPE ple_data.auth_rate_scope AS ENUM (
    'email', 'network', 'principal', 'service'
);
CREATE TYPE ple_data.email_challenge_purpose AS ENUM (
    'sign_in', 'change_email'
);
CREATE TYPE ple_data.passkey_ceremony_kind AS ENUM (
    'authentication', 'registration'
);

GRANT USAGE ON TYPE
    ple_data.bloom_cognitive_process,
    ple_data.bloom_knowledge_dimension,
    ple_data.product_role,
    ple_data.assessment_type,
    ple_data.late_work_rule,
    ple_data.question_variation_rule,
    ple_data.question_order_rule,
    ple_data.selected_question_order,
    ple_data.feedback_release,
    ple_data.scoring_rule,
    ple_data.entry_kind,
    ple_data.entry_availability,
    ple_data.question_backend,
    ple_data.question_format,
    ple_data.question_type,
    ple_data.library_object_kind,
    ple_data.library_watch_event_kind,
    ple_data.media_type,
    ple_data.object_storage_area,
    ple_data.object_data_class,
    ple_data.lease_state,
    ple_data.account_state,
    ple_data.assessment_origin_kind,
    ple_data.assessment_status,
    ple_data.course_source_kind,
    ple_data.course_lifecycle_state,
    ple_data.retention_lifecycle_state,
    ple_data.blueprint_availability,
    ple_data.question_availability,
    ple_data.membership_role,
    ple_data.membership_event_kind,
    ple_data.public_id_object_kind,
    ple_data.license_spdx,
    ple_data.correction_reason,
    ple_data.thread_state,
    ple_data.notice_state,
    ple_data.delivery_state,
    ple_data.cleanup_disposition,
    ple_data.invitation_response,
    ple_data.roster_event_kind,
    ple_data.ownership_event_kind,
    ple_data.import_format,
    ple_data.import_state,
    ple_data.import_item_result,
    ple_data.avatar_kind,
    ple_data.object_work_operation,
    ple_data.banner_work_operation,
    ple_data.banner_subject_kind,
    ple_data.alternative_kind,
    ple_data.publication_state,
    ple_data.job_state,
    ple_data.issued_capability,
    ple_data.retention_action_kind,
    ple_data.retention_failure_kind,
    ple_data.repair_result,
    ple_data.watch_notification_event_kind,
    ple_data.auth_rate_scope,
    ple_data.email_challenge_purpose,
    ple_data.passkey_ceremony_kind
    TO ple_private_owner, ple_audit_owner, ple_api_owner;

SET LOCAL ROLE ple_private_owner;

CREATE TYPE ple_private.bloom_preparation_target_kind AS ENUM (
    'question_revision', 'question_pool'
);

GRANT USAGE ON TYPE ple_private.bloom_preparation_target_kind
    TO ple_data_owner, ple_audit_owner, ple_api_owner;

-- Public-ID checksum and domains. One CHECK per identity shape, reused by
-- every column that stores that identity (DATABASE_STYLE.md Types).

CREATE FUNCTION ple_private.crockford_checksum_character(p_checksum_input text)
RETURNS text LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog
AS $$
    SELECT substr(
        '0123456789ABCDEFGHJKMNPQRSTVWXYZ',
        (get_byte(sha256(convert_to(p_checksum_input, 'UTF8')), 0) >> 3) + 1,
        1
    )
$$;

CREATE FUNCTION ple_private.is_canonical_prefixed_public_id(
    p_public_id text, p_prefix text
) RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog, ple_private
AS $$
    SELECT p_public_id IS NOT NULL
       AND p_prefix IN ('BP', 'CI', 'A', 'U')
       AND p_public_id ~ (
           '^' || p_prefix || '[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}$'
       )
       AND right(p_public_id, 1) = ple_private.crockford_checksum_character(
           left(p_public_id, char_length(p_public_id) - 1)
       )
$$;

CREATE FUNCTION ple_private.is_canonical_question_family_id(p_public_id text)
RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog, ple_private
AS $$
    SELECT p_public_id IS NOT NULL
       AND p_public_id ~ '^[0-9A-HJKMNP-TV-Z]{4}-[0-9A-HJKMNP-TV-Z]{4}$'
       AND substr(p_public_id, 6, 1) = ple_private.crockford_checksum_character(
           substr(p_public_id, 1, 4) || substr(p_public_id, 7, 3)
       )
$$;

GRANT EXECUTE ON FUNCTION
    ple_private.crockford_checksum_character(text),
    ple_private.is_canonical_prefixed_public_id(text, text),
    ple_private.is_canonical_question_family_id(text)
    TO PUBLIC;

SET LOCAL ROLE ple_data_owner;

CREATE DOMAIN ple_data.account_id AS text
    CHECK (VALUE IS NULL OR ple_private.is_canonical_prefixed_public_id(VALUE, 'U'));
CREATE DOMAIN ple_data.course_instance_id AS text
    CHECK (VALUE IS NULL OR ple_private.is_canonical_prefixed_public_id(VALUE, 'CI'));
CREATE DOMAIN ple_data.blueprint_course_id AS text
    CHECK (VALUE IS NULL OR ple_private.is_canonical_prefixed_public_id(VALUE, 'BP'));
CREATE DOMAIN ple_data.assessment_id AS text
    CHECK (VALUE IS NULL OR ple_private.is_canonical_prefixed_public_id(VALUE, 'A'));
CREATE DOMAIN ple_data.question_family_id AS text
    CHECK (VALUE IS NULL OR ple_private.is_canonical_question_family_id(VALUE));
CREATE DOMAIN ple_data.sha256_digest AS bytea
    CHECK (VALUE IS NULL OR octet_length(VALUE) = 32);

GRANT USAGE ON TYPE
    ple_data.account_id,
    ple_data.course_instance_id,
    ple_data.blueprint_course_id,
    ple_data.assessment_id,
    ple_data.question_family_id,
    ple_data.sha256_digest
    TO ple_private_owner, ple_audit_owner, ple_api_owner;
