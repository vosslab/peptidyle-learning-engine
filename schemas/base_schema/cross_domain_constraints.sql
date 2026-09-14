-- Cross-domain foreign keys belong here only where the two relations have
-- distinct domain owners and the dependency cannot be declared by either
-- module at its creation point.  This is part of the canonical base, not a
-- later corrective layer.

-- Course Core carries the current-banner reference while Course Media owns
-- the banner lineage.  Installing this relationship after both modules keeps
-- each family self-contained without a corrective migration.
SET LOCAL ROLE ple_data_owner;

ALTER TABLE ple_data.course_instance
    ADD CONSTRAINT course_instance_current_banner_fkey
        FOREIGN KEY (course_id, current_course_banner_id)
        REFERENCES ple_data.course_banner(course_id, course_banner_id);

RESET ROLE;

-- Object Records carries the generic check anchor.  Course Media owns the
-- banner-storage subject that supplies its alternate anchor, so this FK is
-- installed only after both canonical modules exist.
SET LOCAL ROLE ple_private_owner;

ALTER TABLE ple_private.object_storage_check
    ADD CONSTRAINT object_storage_check_course_banner_subject_fkey
        FOREIGN KEY (course_banner_storage_subject_id)
        REFERENCES ple_private.course_banner_storage_subject(
            course_banner_storage_subject_id);

RESET ROLE;
