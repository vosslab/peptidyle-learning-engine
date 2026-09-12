-- Immutable membership episodes bind current teaching-team authority and exact Student records.

SET LOCAL ROLE ple_data_owner;
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
CREATE INDEX course_membership_event_current_lookup_idx
    ON ple_data.course_membership_event (membership_id, occurred_at DESC, course_membership_event_id DESC);
CREATE INDEX course_membership_account_course_idx ON ple_data.course_membership (account_id, course_id);
CREATE INDEX student_record_account_course_idx ON ple_data.student_record (student_account_id, course_id);

CREATE FUNCTION ple_data.reject_course_membership_change() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Course Membership episodes are immutable'; END $$;
CREATE FUNCTION ple_data.reject_course_membership_event_change() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'Course Membership Events are immutable'; END $$;
CREATE TRIGGER course_membership_is_immutable BEFORE UPDATE OR DELETE ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_membership_change();
CREATE TRIGGER course_membership_event_is_immutable BEFORE UPDATE OR DELETE ON ple_data.course_membership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.reject_course_membership_event_change();

CREATE FUNCTION ple_data.record_course_membership_start() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
 INSERT INTO ple_data.course_membership_event(course_membership_event_id,membership_id,event_kind,occurred_at,reason)
 VALUES (NEW.membership_id,NEW.membership_id,'started',NEW.joined_at,'membership created'); RETURN NEW;
END $$;
CREATE TRIGGER course_membership_creation_records_started_event AFTER INSERT ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.record_course_membership_start();
CREATE FUNCTION ple_data.course_membership_is_active(p_membership_id uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path = pg_catalog, ple_data AS $$
 SELECT event_kind = 'started' FROM ple_data.course_membership_event
  WHERE membership_id = p_membership_id ORDER BY occurred_at DESC, course_membership_event_id DESC LIMIT 1
$$;
CREATE FUNCTION ple_data.assert_student_membership_record() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
 IF NEW.role = 'student' AND NOT EXISTS (SELECT 1 FROM ple_data.student_record r
  WHERE r.student_record_id=NEW.student_record_id AND r.course_id=NEW.course_id AND r.student_account_id=NEW.account_id) THEN
  RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Student Course Membership must bind its exact Student Record'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER course_membership_binds_exact_student_record BEFORE INSERT OR UPDATE
OF course_id, account_id, role, student_record_id ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.assert_student_membership_record();
CREATE FUNCTION ple_data.assert_course_membership_event_transition() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE m ple_data.course_membership%ROWTYPE; current_kind text;
BEGIN
 SELECT * INTO m FROM ple_data.course_membership WHERE membership_id=NEW.membership_id;
 IF NOT FOUND OR NEW.occurred_at < m.joined_at THEN RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Course Membership Event is outside its episode'; END IF;
 PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(pg_catalog.format('%s:%s:%s',m.course_id,m.account_id,m.role),0));
 SELECT event_kind INTO current_kind FROM ple_data.course_membership_event WHERE membership_id=m.membership_id ORDER BY occurred_at DESC, course_membership_event_id DESC LIMIT 1;
 IF (NEW.event_kind='started' AND current_kind IS NOT NULL) OR (NEW.event_kind='ended' AND current_kind <> 'started') THEN RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Course Membership Event transition is invalid'; END IF;
 IF NEW.event_kind='started' AND EXISTS (SELECT 1 FROM ple_data.course_membership other WHERE other.course_id=m.course_id AND other.account_id=m.account_id AND other.role=m.role AND other.membership_id<>m.membership_id AND ple_data.course_membership_is_active(other.membership_id)) THEN RAISE EXCEPTION USING ERRCODE='23505', MESSAGE='an Account can have only one active Course Membership for a Course and role'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER course_membership_event_has_valid_transition BEFORE INSERT ON ple_data.course_membership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.assert_course_membership_event_transition();
CREATE FUNCTION ple_data.assert_assigned_instructor_membership() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
DECLARE checked_course uuid; checked_membership uuid;
BEGIN
 IF TG_TABLE_NAME = 'course_instance' THEN
  IF TG_OP = 'DELETE' THEN checked_course := OLD.course_id; ELSE checked_course := NEW.course_id; END IF;
 ELSE
  IF TG_OP = 'DELETE' THEN checked_membership := OLD.membership_id; ELSE checked_membership := NEW.membership_id; END IF;
  SELECT course_id INTO checked_course FROM ple_data.course_membership WHERE membership_id = checked_membership;
 END IF;
 IF NOT EXISTS (SELECT 1 FROM ple_data.course_instance c JOIN ple_data.course_membership m ON m.course_id=c.course_id AND m.account_id=c.assigned_instructor_account_id AND m.role='instructor' AND ple_data.course_membership_is_active(m.membership_id) WHERE c.course_id=checked_course) THEN RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='a Course Instance requires one current assigned Instructor membership'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER course_instance_assigned_instructor_is_current AFTER INSERT OR UPDATE OF assigned_instructor_account_id ON ple_data.course_instance DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_data.assert_assigned_instructor_membership();
CREATE CONSTRAINT TRIGGER course_membership_preserves_assigned_instructor AFTER INSERT ON ple_data.course_membership_event DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_data.assert_assigned_instructor_membership();

ALTER TABLE ple_data.student_record ENABLE ROW LEVEL SECURITY; ALTER TABLE ple_data.student_record FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_membership ENABLE ROW LEVEL SECURITY; ALTER TABLE ple_data.course_membership FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.course_membership_event ENABLE ROW LEVEL SECURITY; ALTER TABLE ple_data.course_membership_event FORCE ROW LEVEL SECURITY;
REVOKE ALL ON TABLE ple_data.student_record, ple_data.course_membership, ple_data.course_membership_event FROM PUBLIC;
GRANT SELECT, INSERT ON ple_data.student_record, ple_data.course_membership, ple_data.course_membership_event TO ple_api_owner;
CREATE POLICY student_record_api_owner_access ON ple_data.student_record FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_membership_api_owner_access ON ple_data.course_membership FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
CREATE POLICY course_membership_event_api_owner_access ON ple_data.course_membership_event FOR ALL TO ple_api_owner USING (true) WITH CHECK (true);
-- The deferred invariant executes as ple_data_owner.  Forced RLS therefore
-- needs this narrow read capability for the three relations it verifies.
CREATE POLICY course_instance_data_owner_assigned_instructor_check ON ple_data.course_instance
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY course_membership_data_owner_assigned_instructor_check ON ple_data.course_membership
    FOR SELECT TO ple_data_owner USING (true);
CREATE POLICY course_membership_event_data_owner_assigned_instructor_check ON ple_data.course_membership_event
    FOR SELECT TO ple_data_owner USING (true);
REVOKE ALL ON FUNCTION ple_data.reject_course_membership_change(), ple_data.reject_course_membership_event_change(), ple_data.record_course_membership_start(), ple_data.assert_student_membership_record(), ple_data.assert_course_membership_event_transition(), ple_data.assert_assigned_instructor_membership() FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_data.course_membership_is_active(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_data.course_membership_is_active(uuid) TO ple_api_owner, ple_app, ple_private_owner, ple_data_owner;
RESET ROLE;
