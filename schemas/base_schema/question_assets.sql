-- Exact Question Revision asset publication.  A publisher claims only an
-- already-bound job and can make one Pending-to-Ready transition.

SET LOCAL ROLE ple_data_owner;
CREATE TABLE ple_data.question_asset_delivery (
    delivery_id uuid PRIMARY KEY,
    object_id uuid NOT NULL,
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    asset_id uuid NOT NULL,
    FOREIGN KEY (delivery_id, object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision (question_id, revision_number)
);
CREATE CONSTRAINT TRIGGER question_asset_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.question_asset_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();
ALTER TABLE ple_data.question_asset_delivery ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.question_asset_delivery FORCE ROW LEVEL SECURITY;
REVOKE ALL ON ple_data.question_asset_delivery FROM PUBLIC;
CREATE POLICY question_asset_delivery_data_owner_access ON ple_data.question_asset_delivery
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.question_asset_publication (
    question_id text NOT NULL,
    revision_number integer NOT NULL CHECK (revision_number > 0),
    asset_id uuid NOT NULL,
    source_object_id uuid NOT NULL REFERENCES ple_private.object_record,
    source_object_checksum bytea NOT NULL CHECK (octet_length(source_object_checksum) = 32),
    public_object_id uuid NOT NULL UNIQUE,
    public_object_checksum bytea NOT NULL CHECK (octet_length(public_object_checksum) = 32),
    public_byte_length bigint NOT NULL CHECK (public_byte_length >= 0),
    verified_media_type text NOT NULL CHECK (verified_media_type IN ('image/png', 'image/jpeg', 'image/webp')),
    intrinsic_width integer NOT NULL CHECK (intrinsic_width > 0),
    intrinsic_height integer NOT NULL CHECK (intrinsic_height > 0),
    delivery_id uuid NOT NULL UNIQUE,
    -- The late integration layer binds this UUID to the sole
    -- publish_public_assets/public_asset_publication Job after jobs.sql.
    job_id uuid NOT NULL UNIQUE,
    publication_state text NOT NULL CHECK (publication_state IN ('pending', 'ready')),
    PRIMARY KEY (question_id, revision_number, asset_id),
    FOREIGN KEY (question_id, revision_number) REFERENCES ple_data.question_revision,
    FOREIGN KEY (delivery_id, public_object_id) REFERENCES ple_data.object_delivery (delivery_id, object_id)
);
CREATE FUNCTION ple_private.reject_question_asset_publication_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.question_id IS DISTINCT FROM OLD.question_id OR NEW.revision_number IS DISTINCT FROM OLD.revision_number
       OR NEW.asset_id IS DISTINCT FROM OLD.asset_id OR NEW.source_object_id IS DISTINCT FROM OLD.source_object_id
       OR NEW.source_object_checksum IS DISTINCT FROM OLD.source_object_checksum OR NEW.public_object_id IS DISTINCT FROM OLD.public_object_id
       OR NEW.public_object_checksum IS DISTINCT FROM OLD.public_object_checksum OR NEW.public_byte_length IS DISTINCT FROM OLD.public_byte_length
       OR NEW.verified_media_type IS DISTINCT FROM OLD.verified_media_type OR NEW.intrinsic_width IS DISTINCT FROM OLD.intrinsic_width
       OR NEW.intrinsic_height IS DISTINCT FROM OLD.intrinsic_height OR NEW.delivery_id IS DISTINCT FROM OLD.delivery_id
       OR NEW.job_id IS DISTINCT FROM OLD.job_id OR OLD.publication_state <> 'pending' OR NEW.publication_state <> 'ready'
       OR current_setting('ple.question_asset_publication_activation', true) IS DISTINCT FROM OLD.job_id::text THEN
        RAISE EXCEPTION USING ERRCODE = '23514', MESSAGE = 'Question Asset Publication is immutable apart from Pending to Ready';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER question_asset_publication_is_immutable
BEFORE UPDATE OR DELETE ON ple_private.question_asset_publication
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_question_asset_publication_change();
CREATE FUNCTION ple_private.validate_question_asset_publication()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE source_address jsonb := jsonb_build_object('kind','restrictedQuestionAsset',
    'questionRevision',jsonb_build_object('questionId',NEW.question_id,'revisionNumber',NEW.revision_number),
    'asset',NEW.asset_id,'object',NEW.source_object_id);
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record record WHERE record.object_id=NEW.source_object_id
       AND record.object_address=source_address AND record.object_storage_area='private-content'
       AND record.object_data_class='question-asset' AND record.sha256=NEW.source_object_checksum
       AND record.media_type=NEW.verified_media_type) THEN
        RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Question Asset Publication requires its exact trusted source Object Record';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER question_asset_publication_has_trusted_objects_and_job
BEFORE INSERT OR UPDATE ON ple_private.question_asset_publication
FOR EACH ROW EXECUTE FUNCTION ple_private.validate_question_asset_publication();
CREATE FUNCTION ple_private.require_complete_question_asset_publication_delivery()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, ple_data, ple_private AS $$
DECLARE target_delivery_id uuid := COALESCE(NEW.delivery_id, OLD.delivery_id); publication ple_private.question_asset_publication%ROWTYPE;
BEGIN
    SELECT * INTO publication FROM ple_private.question_asset_publication
     WHERE question_asset_publication.delivery_id = target_delivery_id;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_data.object_delivery delivery WHERE delivery.delivery_id=publication.delivery_id
          AND delivery.object_id=publication.public_object_id AND delivery.sha256=publication.public_object_checksum
          AND delivery.media_type=publication.verified_media_type AND delivery.byte_length=publication.public_byte_length
          AND delivery.delivery_state=CASE publication.publication_state WHEN 'pending' THEN 'pending' WHEN 'ready' THEN 'available' END)
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_asset_delivery asset_delivery WHERE asset_delivery.delivery_id=publication.delivery_id
          AND asset_delivery.object_id=publication.public_object_id AND asset_delivery.question_id=publication.question_id
          AND asset_delivery.revision_number=publication.revision_number AND asset_delivery.asset_id=publication.asset_id) THEN
        RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Question Asset Publication requires exact state-matched delivery';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER question_asset_publication_requires_complete_delivery
AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_asset_publication DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();
-- Object Delivery relations are owned by ple_data_owner.  The validation
-- routine remains private-owner code, while the table owner installs its
-- two cross-table deferred triggers.
GRANT EXECUTE ON FUNCTION ple_private.require_complete_question_asset_publication_delivery() TO ple_data_owner;
SET LOCAL ROLE ple_data_owner;
CREATE CONSTRAINT TRIGGER object_delivery_preserves_question_asset_publication
AFTER INSERT OR UPDATE OR DELETE ON ple_data.object_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();
CREATE CONSTRAINT TRIGGER question_asset_delivery_preserves_question_asset_publication
AFTER INSERT OR UPDATE OR DELETE ON ple_data.question_asset_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();
SET LOCAL ROLE ple_private_owner;
ALTER TABLE ple_private.question_asset_publication ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_asset_publication FORCE ROW LEVEL SECURITY;
REVOKE ALL ON ple_private.question_asset_publication FROM PUBLIC;
REVOKE ALL ON FUNCTION ple_private.reject_question_asset_publication_change(),
    ple_private.validate_question_asset_publication(), ple_private.require_complete_question_asset_publication_delivery() FROM PUBLIC;
CREATE POLICY question_asset_publication_private_owner_access ON ple_private.question_asset_publication
    FOR ALL TO ple_private_owner USING (true) WITH CHECK (true);
SET LOCAL ROLE ple_data_owner;
GRANT SELECT, INSERT, UPDATE ON ple_data.object_delivery, ple_data.question_asset_delivery TO ple_private_owner;
CREATE POLICY question_asset_publication_private_delivery_read ON ple_data.object_delivery
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_asset_publication_private_delivery_update ON ple_data.object_delivery
    FOR UPDATE TO ple_private_owner USING (true) WITH CHECK (true);
CREATE POLICY question_asset_publication_private_delivery_insert ON ple_data.object_delivery
    FOR INSERT TO ple_private_owner WITH CHECK (true);
CREATE POLICY question_asset_publication_private_asset_delivery_read ON ple_data.question_asset_delivery
    FOR SELECT TO ple_private_owner USING (true);
CREATE POLICY question_asset_publication_private_asset_delivery_insert ON ple_data.question_asset_delivery
    FOR INSERT TO ple_private_owner WITH CHECK (true);
SET LOCAL ROLE ple_private_owner;
CREATE FUNCTION ple_private.select_ready_question_asset_renditions(p_question_id text, p_revision_number integer)
RETURNS TABLE(asset_id uuid, question_asset_checksum text, rendition_checksum text, intrinsic_width integer, intrinsic_height integer)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT publication.asset_id, encode(publication.source_object_checksum,'hex'), encode(publication.public_object_checksum,'hex'), publication.intrinsic_width, publication.intrinsic_height
    FROM ple_private.question_asset_publication publication
    JOIN ple_data.object_delivery delivery ON delivery.delivery_id=publication.delivery_id AND delivery.object_id=publication.public_object_id
    JOIN ple_data.question_asset_delivery asset_delivery ON asset_delivery.delivery_id=publication.delivery_id AND asset_delivery.asset_id=publication.asset_id
    WHERE publication.question_id=p_question_id AND publication.revision_number=p_revision_number
      AND publication.publication_state='ready' AND delivery.delivery_state='available' ORDER BY publication.asset_id
$$;
REVOKE ALL ON FUNCTION ple_private.select_ready_question_asset_renditions(text,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_private.select_ready_question_asset_renditions(text,integer) TO ple_api_owner;
GRANT USAGE ON SCHEMA ple_private TO ple_api_owner, ple_public_asset_publisher;
SET LOCAL ROLE ple_api_owner;
CREATE FUNCTION ple_api.select_ready_question_asset_renditions(p_question_id text, p_revision_number integer)
RETURNS TABLE(asset_id uuid, question_asset_checksum text, rendition_checksum text, intrinsic_width integer, intrinsic_height integer)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.select_ready_question_asset_renditions(p_question_id, p_revision_number)
$$;
REVOKE ALL ON FUNCTION ple_api.select_ready_question_asset_renditions(text,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION ple_api.select_ready_question_asset_renditions(text,integer) TO ple_app;

RESET ROLE;
