-- course_membership tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Immutable membership episodes bind current teaching-team authority and exact Student records.
CREATE TABLE ple_data.student_record (
    student_record_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    UNIQUE (course_id, student_account_id), UNIQUE (student_record_id, course_id)
);

CREATE TABLE ple_data.course_membership (
    membership_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    role text NOT NULL CHECK (role IN ('student', 'instructor')),
    student_record_id uuid REFERENCES ple_data.student_record (student_record_id),
    joined_at timestamp with time zone NOT NULL,
    FOREIGN KEY (account_id, role) REFERENCES ple_private.account (account_id, product_role),
    CHECK ((role = 'student' AND student_record_id IS NOT NULL)
        OR (role = 'instructor' AND student_record_id IS NULL))
);

CREATE TABLE ple_data.course_membership_event (
    course_membership_event_id uuid PRIMARY KEY,
    membership_id uuid NOT NULL REFERENCES ple_data.course_membership (membership_id),
    event_kind text NOT NULL CHECK (event_kind IN ('started', 'ended')),
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000),
    UNIQUE (membership_id, occurred_at, course_membership_event_id)
);

SET LOCAL ROLE ple_private_owner;

-- Invitation and roster evidence.  Email/persona resolution is owned by accounts.sql.
CREATE TABLE ple_private.course_invitation (
    invitation_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    target_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    membership_role text NOT NULL CHECK (membership_role IN ('student','instructor')),
    inviting_instructor_account_id uuid NOT NULL,
    inviting_instructor_role text NOT NULL DEFAULT 'instructor'
        CHECK (inviting_instructor_role = 'instructor'),
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    FOREIGN KEY (target_account_id,membership_role)
        REFERENCES ple_private.account(account_id,product_role),
    FOREIGN KEY (inviting_instructor_account_id,inviting_instructor_role)
        REFERENCES ple_private.account(account_id,product_role)
);

CREATE TABLE ple_private.course_invitation_event (
    course_invitation_event_id uuid PRIMARY KEY,
    invitation_id uuid NOT NULL UNIQUE REFERENCES ple_private.course_invitation (invitation_id),
    event_kind text NOT NULL CHECK (event_kind IN ('accepted','declined','revoked')),
    performed_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000)
);

CREATE TABLE ple_private.course_roster_profile (
    course_roster_profile_id uuid PRIMARY KEY,
    course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    roster_id text NOT NULL CHECK (char_length(roster_id) BETWEEN 1 AND 64 AND roster_id ~ '^[A-Za-z0-9._-]+$'),
    roster_name text NOT NULL CHECK (char_length(roster_name) BETWEEN 1 AND 200
        AND roster_name = btrim(roster_name,
            U&'\0009\000A\000B\000C\000D\0020\0085\00A0\1680\2000\2001\2002\2003\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000')
        AND roster_name !~ U&'[\0001-\001F\007F-\009F]'),
    created_at timestamp with time zone NOT NULL,
    UNIQUE(course_id,student_account_id), UNIQUE(course_id,roster_id)
);

SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_roster_event (
 course_roster_event_id uuid PRIMARY KEY, course_id uuid NOT NULL REFERENCES ple_data.course_instance(course_id),
 student_account_id uuid NOT NULL REFERENCES ple_private.account(account_id), acting_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
 event_kind text NOT NULL CHECK(event_kind IN ('invitation_created','invitation_claimed','student_access_revoked')), occurred_at timestamp with time zone NOT NULL);

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

