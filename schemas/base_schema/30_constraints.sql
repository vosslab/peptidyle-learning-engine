-- Late and circular foreign keys.

SET LOCAL ROLE ple_audit_owner;

ALTER TABLE ple_audit.instructor_account_creation_event
    ADD CONSTRAINT instructor_account_creation_event_vetting_decision_id_fkey
    FOREIGN KEY (instructor_identity_vetting_decision_id)
    REFERENCES ple_audit.instructor_identity_vetting_decision (decision_id);

SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.draft_question_source_binding
    ADD CONSTRAINT draft_question_source_binding_object_record_exists
    FOREIGN KEY (source_object_record_id) REFERENCES ple_private.object_record(object_record_id);

ALTER TABLE ple_private.question_revision_source_binding
    ADD CONSTRAINT question_revision_source_binding_object_record_exists
    FOREIGN KEY (source_object_record_id) REFERENCES ple_private.object_record(object_record_id);

SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.blueprint_course
    ADD CONSTRAINT blueprint_course_current_revision_fk
    FOREIGN KEY (blueprint_course_id, current_blueprint_revision_number)
    REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_id, blueprint_revision_number
    ) DEFERRABLE INITIALLY DEFERRED;



-- A complete immutable Revision has exactly one immutable receipt event.  The
-- circular deferred foreign keys let one transaction insert the Revision,
-- children, and its event in that order without an RLS-sensitive trigger.
ALTER TABLE ple_data.blueprint_course_revision
    ADD CONSTRAINT blueprint_revision_requires_event_fk
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
    REFERENCES ple_data.blueprint_revision_event (
        blueprint_course_id, blueprint_revision_number
    ) DEFERRABLE INITIALLY DEFERRED;

-- Course banner metadata and the database half of its object-store saga.



-- A generic Course-owned delivery is still ordinary Course media.  The
-- Object Delivery module cannot name this foreign key because Course does not
-- exist until this point in the manifest.
ALTER TABLE ple_data.course_object_delivery
    ADD CONSTRAINT course_object_delivery_course_fkey
    FOREIGN KEY (course_instance_id) REFERENCES ple_data.course_instance(course_instance_id);

ALTER TABLE ple_data.course_instance
    ADD COLUMN current_course_banner_id uuid,
    ADD COLUMN course_banner_alternative_kind text,
    ADD COLUMN course_banner_alternative_text text,
    ADD CONSTRAINT course_instance_banner_alternative_is_complete CHECK (
        (current_course_banner_id IS NULL AND course_banner_alternative_kind IS NULL
            AND course_banner_alternative_text IS NULL)
        OR (current_course_banner_id IS NOT NULL AND course_banner_alternative_kind = 'decorative'
            AND course_banner_alternative_text IS NULL)
        OR (current_course_banner_id IS NOT NULL AND course_banner_alternative_kind = 'informative'
            AND char_length(btrim(course_banner_alternative_text)) BETWEEN 1 AND 160)
    );

ALTER TABLE ple_data.assessment_entry_pool
    ADD CONSTRAINT assessment_entry_owns_exact_question_pool_fork
    FOREIGN KEY (
        assessment_entry_id, assessment_id, question_pool_id
    ) REFERENCES ple_data.assessment_question_pool_fork (
        assessment_entry_id, assessment_id, question_pool_id
    ) DEFERRABLE INITIALLY DEFERRED;

SET LOCAL ROLE ple_private_owner;

-- Late operations for immutable Question Revision public assets.  This module
-- follows Jobs and retained presentation evidence; it owns the resulting
-- publication transition and opaque resolver rather than a corrective layer.
ALTER TABLE ple_private.question_asset_publication
    ADD CONSTRAINT question_asset_publication_job_fkey
    FOREIGN KEY (job_id) REFERENCES ple_private.job(job_id);

SET LOCAL ROLE ple_data_owner;

-- Cross-domain foreign keys belong here only where the two relations have
-- distinct domain owners and the dependency cannot be declared by either
-- module at its creation point.  This is part of the canonical base, not a
-- later corrective layer.

-- Course Core carries the current-banner reference while Course Media owns
-- the banner lineage.  Installing this relationship after both modules keeps
-- each family self-contained without a corrective migration.
ALTER TABLE ple_data.course_instance
    ADD CONSTRAINT course_instance_current_banner_fkey
        FOREIGN KEY (course_instance_id, current_course_banner_id)
        REFERENCES ple_data.course_banner(course_instance_id, course_banner_id);

SET LOCAL ROLE ple_private_owner;





-- Object Records carries the generic check anchor.  Course Media owns the
-- banner-storage subject that supplies its alternate anchor, so this FK is
-- installed only after both canonical modules exist.
ALTER TABLE ple_private.object_storage_check
    ADD CONSTRAINT object_storage_check_course_banner_subject_fkey
        FOREIGN KEY (course_banner_storage_subject_id)
        REFERENCES ple_private.course_banner_storage_subject(
            course_banner_storage_subject_id);

