-- course_membership tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

-- Immutable membership episodes bind current teaching-team authority and exact Student records.
CREATE TABLE ple_data.student_record (
    student_record_id uuid PRIMARY KEY,
    course_instance_id uuid NOT NULL REFERENCES ple_data.course_instance (course_instance_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    created_at timestamp with time zone NOT NULL,
    UNIQUE (course_instance_id, student_account_id),
    UNIQUE (student_record_id, course_instance_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



CREATE TABLE ple_data.course_membership (
    course_membership_id uuid PRIMARY KEY,
    course_instance_id uuid NOT NULL REFERENCES ple_data.course_instance (course_instance_id),
    account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    role ple_data.product_role NOT NULL,
    student_record_id uuid REFERENCES ple_data.student_record (student_record_id),
    joined_at timestamp with time zone NOT NULL,
    FOREIGN KEY (account_id, role) REFERENCES ple_private.account (account_id, product_role),
    CHECK ((role = 'student' AND student_record_id IS NOT NULL)
        OR (role = 'instructor' AND student_record_id IS NULL)),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);



CREATE TABLE ple_data.course_membership_event (
    course_membership_event_id uuid PRIMARY KEY,
    course_membership_id uuid NOT NULL REFERENCES ple_data.course_membership (course_membership_id),
    event_kind ple_data.membership_event_kind NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000),
    UNIQUE (course_membership_id, occurred_at, course_membership_event_id)
);


SET LOCAL ROLE ple_private_owner;

-- Invitation and roster evidence.  Email/persona resolution is owned by accounts.sql.
CREATE TABLE ple_private.course_invitation (
    course_invitation_id uuid PRIMARY KEY,
    course_instance_id uuid NOT NULL REFERENCES ple_data.course_instance (course_instance_id),
    target_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    membership_role ple_data.product_role NOT NULL,
    inviting_instructor_account_id uuid NOT NULL,
    inviting_instructor_role ple_data.product_role NOT NULL DEFAULT 'instructor',
    issued_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL CHECK (expires_at > issued_at),
    FOREIGN KEY (target_account_id,membership_role)
        REFERENCES ple_private.account(account_id,product_role),
    FOREIGN KEY (inviting_instructor_account_id,inviting_instructor_role)
        REFERENCES ple_private.account(account_id,product_role),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);



CREATE TABLE ple_private.course_invitation_event (
    course_invitation_event_id uuid PRIMARY KEY,
    course_invitation_id uuid NOT NULL UNIQUE REFERENCES ple_private.course_invitation (course_invitation_id),
    event_kind ple_data.invitation_response NOT NULL,
    performed_by_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    occurred_at timestamp with time zone NOT NULL,
    reason text NOT NULL CHECK (char_length(btrim(reason)) BETWEEN 1 AND 1000)
);


CREATE TABLE ple_private.course_roster_profile (
    course_roster_profile_id uuid PRIMARY KEY,
    course_instance_id uuid NOT NULL REFERENCES ple_data.course_instance (course_instance_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    roster_id text NOT NULL CHECK (char_length(roster_id) BETWEEN 1 AND 64 AND roster_id ~ '^[A-Za-z0-9._-]+$'),
    roster_name text NOT NULL CHECK (char_length(roster_name) BETWEEN 1 AND 200
        AND roster_name = btrim(roster_name,
            U&'\0009\000A\000B\000C\000D\0020\0085\00A0\1680\2000\2001\2002\2003\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000')
        AND roster_name !~ U&'[\0001-\001F\007F-\009F]'),
    created_at timestamp with time zone NOT NULL,
    UNIQUE(course_instance_id,student_account_id),
    UNIQUE(course_instance_id,roster_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);



SET LOCAL ROLE ple_audit_owner;

CREATE TABLE ple_audit.course_roster_event (
    course_roster_event_id uuid PRIMARY KEY,
    course_instance_id uuid NOT NULL REFERENCES ple_data.course_instance(course_instance_id),
    student_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    acting_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    event_kind ple_data.roster_event_kind NOT NULL,
    occurred_at timestamp with time zone NOT NULL
);



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';




SET LOCAL ROLE ple_audit_owner;



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';





SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';



SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.student_record IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_data.course_membership_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_private_owner;

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.course_invitation IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_invitation_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

COMMENT ON TABLE ple_private.course_roster_profile IS 'role: current state, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

SET LOCAL ROLE ple_audit_owner;

SET LOCAL ROLE ple_audit_owner;
COMMENT ON TABLE ple_audit.course_roster_event IS 'role: event, deleted by FERPA purge of the Course Instance. HUMAN_GUIDANCE.md Course membership.';

