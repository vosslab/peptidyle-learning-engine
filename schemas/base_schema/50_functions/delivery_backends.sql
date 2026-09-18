-- Functions, triggers, and views from delivery_backends.sql.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.enforce_imathas_question_backend_session_transition()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF ROW(NEW.imathas_question_backend_session_id, NEW.course_id, NEW.assessment_id, NEW.question_attempt_id, NEW.account_id, NEW.imathas_deployment_reference, NEW.imathas_item_reference, NEW.question_id, NEW.revision_number, NEW.source_object_id, NEW.source_object_checksum, NEW.imathas_profile, NEW.question_seed, NEW.imathas_launch_binding_checksum, NEW.imathas_response_sha256, NEW.imathas_question_backend_session_challenge, NEW.imathas_question_backend_session_authentication, NEW.issued_at, NEW.expires_at, NEW.imathas_question_backend_state_key_id, NEW.imathas_question_backend_state_nonce, NEW.imathas_question_backend_state_ciphertext)
       IS DISTINCT FROM ROW(OLD.imathas_question_backend_session_id, OLD.course_id, OLD.assessment_id, OLD.question_attempt_id, OLD.account_id, OLD.imathas_deployment_reference, OLD.imathas_item_reference, OLD.question_id, OLD.revision_number, OLD.source_object_id, OLD.source_object_checksum, OLD.imathas_profile, OLD.question_seed, OLD.imathas_launch_binding_checksum, OLD.imathas_response_sha256, OLD.imathas_question_backend_session_challenge, OLD.imathas_question_backend_session_authentication, OLD.issued_at, OLD.expires_at, OLD.imathas_question_backend_state_key_id, OLD.imathas_question_backend_state_nonce, OLD.imathas_question_backend_state_ciphertext) THEN
        RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'iMathAS Question Backend Session binding is immutable';
    END IF;
    IF OLD.revoked_at IS NOT NULL OR OLD.consumed_at IS NOT NULL THEN
        IF NEW IS DISTINCT FROM OLD THEN RAISE EXCEPTION USING ERRCODE = '55000', MESSAGE = 'iMathAS Question Backend Session is terminal'; END IF;
    ELSIF NEW.revoked_at IS NOT NULL AND (NEW.revoked_at < OLD.issued_at OR NEW.consumed_at IS NOT NULL) THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'iMathAS Session revocation is invalid';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER imathas_question_backend_session_transition_is_forward_only BEFORE UPDATE ON ple_private.imathas_question_backend_session FOR EACH ROW EXECUTE FUNCTION ple_private.enforce_imathas_question_backend_session_transition();

