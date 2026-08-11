-- WP-I2 local-development roster source. This capability never models a
-- canonical account, an invitation, or an email address.

ALTER TABLE public.course_roster_member
    DROP CONSTRAINT course_roster_member_source_check,
    ADD CONSTRAINT course_roster_member_source_check
        CHECK (source IN ('invitation', 'local_development', 'legacy'));

ALTER TABLE public.course_roster_member
    DROP CONSTRAINT course_roster_member_managed_fields_check,
    ADD CONSTRAINT course_roster_member_managed_fields_check CHECK (
        source = 'legacy'
        OR (source = 'invitation'
            AND roster_email_normalized IS NOT NULL
            AND roster_email_delivery IS NOT NULL
            AND roster_id IS NOT NULL)
        OR (source = 'local_development'
            AND roster_email_normalized IS NULL
            AND roster_email_delivery IS NULL
            AND roster_id IS NULL)
    );
