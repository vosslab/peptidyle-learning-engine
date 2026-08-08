-- MOD-API-COURSE: tenant courses, course-local membership, and scoped assignments.

CREATE TABLE course (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    title text NOT NULL
        CHECK (
            char_length(title) BETWEEN 1 AND 200
            AND title = btrim(title)
        ),
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (tenant_id, course_id)
);

CREATE TABLE course_member (
    tenant_id uuid NOT NULL,
    course_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role text NOT NULL CHECK (role IN ('student', 'instructor')),
    PRIMARY KEY (tenant_id, course_id, user_id),
    FOREIGN KEY (tenant_id, course_id)
        REFERENCES course(tenant_id, course_id) ON DELETE CASCADE
);

CREATE INDEX course_member_user_courses_idx
    ON course_member (tenant_id, user_id, course_id);

-- Upgrade databases can contain pre-course assignment payloads. Do not invent
-- ownership or titles for those rows. These NOT VALID checks constrain every
-- new write while leaving legacy rows visible for an explicit owner-led data
-- migration. Fresh databases satisfy the constraints immediately.
ALTER TABLE assignment
    ADD COLUMN course_id uuid,
    ADD COLUMN title text,
    ADD CONSTRAINT assignment_course_required_check
        CHECK (course_id IS NOT NULL) NOT VALID,
    ADD CONSTRAINT assignment_title_required_check
        CHECK (
            title IS NOT NULL
            AND char_length(title) BETWEEN 1 AND 200
            AND title = btrim(title)
        ) NOT VALID,
    ADD CONSTRAINT assignment_course_fk
        FOREIGN KEY (tenant_id, course_id)
        REFERENCES course(tenant_id, course_id)
        NOT VALID;

CREATE INDEX assignment_course_page_idx
    ON assignment (tenant_id, course_id, assignment_id);

ALTER TABLE course ENABLE ROW LEVEL SECURITY;
ALTER TABLE course FORCE ROW LEVEL SECURITY;
CREATE POLICY course_tenant ON course
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

ALTER TABLE course_member ENABLE ROW LEVEL SECURITY;
ALTER TABLE course_member FORCE ROW LEVEL SECURITY;
CREATE POLICY course_member_tenant ON course_member
    USING (tenant_id = ple_current_tenant())
    WITH CHECK (tenant_id = ple_current_tenant());

GRANT SELECT, INSERT, UPDATE, DELETE ON course, course_member TO ple_app;
GRANT SELECT ON course, course_member TO ple_student;
