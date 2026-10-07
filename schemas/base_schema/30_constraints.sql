-- Late and circular foreign keys.

SET LOCAL ROLE ple_data_owner;

-- A Published Question records one exact parent Revision on the ordinary
-- lineage. The deferred FK permits atomic Question + Revision 1 creation.
ALTER TABLE ple_data.published_question
    ADD CONSTRAINT published_question_parent_revision_fk
    FOREIGN KEY (parent_published_question_id, parent_revision_number)
    REFERENCES ple_data.question_revision (published_question_id, revision_number)
    DEFERRABLE INITIALLY DEFERRED;

SET LOCAL ROLE ple_private_owner;

-- Account-owned Instructor appearance refers to the shared data-owned Theme
-- vocabulary. The late constraint follows its cross-schema REFERENCES grant.
ALTER TABLE ple_private.instructor_personal_theme
    ADD CONSTRAINT instructor_personal_theme_theme_fkey
    FOREIGN KEY (theme_id) REFERENCES ple_data.theme (theme_id);

ALTER TABLE ple_private.draft_question
    ADD CONSTRAINT draft_question_parent_revision_fk
    FOREIGN KEY (parent_published_question_id, parent_revision_number)
    REFERENCES ple_data.question_revision (published_question_id, revision_number);

ALTER TABLE ple_private.draft_question_metadata
    ADD CONSTRAINT draft_question_metadata_subject_discipline_fk
    FOREIGN KEY (content_subject_id, content_discipline_id)
    REFERENCES ple_data.content_subject_discipline(content_subject_id, content_discipline_id),
    ADD CONSTRAINT draft_question_metadata_subject_topic_fk
    FOREIGN KEY (content_subject_id, content_topic_id)
    REFERENCES ple_data.content_topic(content_subject_id, content_topic_id),
    ADD CONSTRAINT draft_question_metadata_topic_subtopic_fk
    FOREIGN KEY (content_topic_id, content_subtopic_id)
    REFERENCES ple_data.content_subtopic(content_topic_id, content_subtopic_id);

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

-- A fork records its exact parent Revision directly on the ordinary child
-- lineage. The deferred FK permits atomic Blueprint + Revision 1 creation.
ALTER TABLE ple_data.blueprint_course
    ADD CONSTRAINT blueprint_course_parent_revision_fk
    FOREIGN KEY (parent_blueprint_course_id, parent_blueprint_revision_number)
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

SET LOCAL ROLE ple_private_owner;

-- Late operations for immutable Question Revision public images.  This module
-- follows Jobs and retained presentation evidence; it owns the resulting
-- publication transition and opaque resolver rather than a corrective layer.
ALTER TABLE ple_private.question_image_publication
    ADD CONSTRAINT question_image_publication_job_fkey
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
