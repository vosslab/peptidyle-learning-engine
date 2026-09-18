-- Functions, triggers, and views from course_membership.sql.

SET LOCAL ROLE ple_data_owner;

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
 INSERT INTO ple_data.course_membership_event(course_membership_event_id,course_membership_id,event_kind,occurred_at,reason)
 VALUES (NEW.course_membership_id,NEW.course_membership_id,'started',NEW.joined_at,'membership created'); RETURN NEW;
END $$;

CREATE TRIGGER course_membership_creation_records_started_event AFTER INSERT ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.record_course_membership_start();

CREATE FUNCTION ple_data.course_membership_is_active(p_membership_id uuid) RETURNS boolean
LANGUAGE sql STABLE SET search_path = pg_catalog, ple_data AS $$
 SELECT event_kind = 'started' FROM ple_data.course_membership_event
  WHERE course_membership_id = p_membership_id ORDER BY occurred_at DESC, course_membership_event_id DESC LIMIT 1
$$;

CREATE FUNCTION ple_data.assert_student_membership_record() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
BEGIN
 IF NEW.role = 'student' AND NOT EXISTS (SELECT 1 FROM ple_data.student_record r
  WHERE r.student_record_id=NEW.student_record_id AND r.course_instance_id=NEW.course_instance_id AND r.student_account_id=NEW.account_id) THEN
  RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Student Course Membership must bind its exact Student Record'; END IF;
 RETURN NEW;
END $$;

CREATE TRIGGER course_membership_binds_exact_student_record BEFORE INSERT OR UPDATE
OF course_instance_id, account_id, role, student_record_id ON ple_data.course_membership
FOR EACH ROW EXECUTE FUNCTION ple_data.assert_student_membership_record();

CREATE FUNCTION ple_data.assert_course_membership_event_transition() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, ple_data AS $$
DECLARE m ple_data.course_membership%ROWTYPE; current_kind text;
BEGIN
 SELECT * INTO m FROM ple_data.course_membership WHERE course_membership_id=NEW.course_membership_id;
 IF NOT FOUND OR NEW.occurred_at < m.joined_at THEN RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Course Membership Event is outside its episode'; END IF;
 PERFORM pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(pg_catalog.format('%s:%s:%s',m.course_instance_id,m.account_id,m.role),0));
 SELECT event_kind INTO current_kind FROM ple_data.course_membership_event WHERE course_membership_id=m.course_membership_id ORDER BY occurred_at DESC, course_membership_event_id DESC LIMIT 1;
 IF (NEW.event_kind='started' AND current_kind IS NOT NULL) OR (NEW.event_kind='ended' AND current_kind <> 'started') THEN RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Course Membership Event transition is invalid'; END IF;
 IF NEW.event_kind='started' AND EXISTS (SELECT 1 FROM ple_data.course_membership other WHERE other.course_instance_id=m.course_instance_id AND other.account_id=m.account_id AND other.role=m.role AND other.course_membership_id<>m.course_membership_id AND ple_data.course_membership_is_active(other.course_membership_id)) THEN RAISE EXCEPTION USING ERRCODE='23505', MESSAGE='an Account can have only one active Course Membership for a Course and role'; END IF;
 RETURN NEW;
END $$;

CREATE TRIGGER course_membership_event_has_valid_transition BEFORE INSERT ON ple_data.course_membership_event
FOR EACH ROW EXECUTE FUNCTION ple_data.assert_course_membership_event_transition();

CREATE FUNCTION ple_data.assert_assigned_instructor_membership() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_data AS $$
DECLARE checked_course text; checked_membership uuid;
BEGIN
 IF TG_TABLE_NAME = 'course_instance' THEN
  IF TG_OP = 'DELETE' THEN checked_course := OLD.course_instance_id; ELSE checked_course := NEW.course_instance_id; END IF;
 ELSE
  IF TG_OP = 'DELETE' THEN checked_membership := OLD.course_membership_id; ELSE checked_membership := NEW.course_membership_id; END IF;
  SELECT course_instance_id INTO checked_course FROM ple_data.course_membership WHERE course_membership_id = checked_membership;
 END IF;
 IF NOT EXISTS (
    SELECT 1
      FROM ple_data.course_membership AS membership
     WHERE membership.course_instance_id = checked_course
       AND membership.role = 'instructor'
       AND ple_data.course_membership_is_active(membership.course_membership_id)
 ) THEN
    RAISE EXCEPTION USING ERRCODE='23514',
        MESSAGE='a Course Instance requires one current Instructor membership';
 END IF;
 RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER course_membership_preserves_instructor AFTER INSERT ON ple_data.course_membership_event DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION ple_data.assert_assigned_instructor_membership();

