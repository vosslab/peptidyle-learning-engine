-- Functions, triggers, and views from question_assets.sql.

SET LOCAL ROLE ple_data_owner;

CREATE CONSTRAINT TRIGGER question_asset_delivery_preserves_available_owner
AFTER INSERT OR UPDATE OR DELETE ON ple_data.question_asset_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_data.require_exact_available_object_delivery_owner();

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.reject_question_asset_publication_change()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog, ple_private AS $$
BEGIN
    IF NEW.published_question_id IS DISTINCT FROM OLD.published_question_id OR NEW.revision_number IS DISTINCT FROM OLD.revision_number
       OR NEW.asset_id IS DISTINCT FROM OLD.asset_id OR NEW.source_object_record_id IS DISTINCT FROM OLD.source_object_record_id
       OR NEW.source_object_checksum IS DISTINCT FROM OLD.source_object_checksum OR NEW.public_object_id IS DISTINCT FROM OLD.public_object_id
       OR NEW.public_object_checksum IS DISTINCT FROM OLD.public_object_checksum OR NEW.public_byte_length IS DISTINCT FROM OLD.public_byte_length
       OR NEW.verified_media_type IS DISTINCT FROM OLD.verified_media_type OR NEW.intrinsic_width IS DISTINCT FROM OLD.intrinsic_width
       OR NEW.intrinsic_height IS DISTINCT FROM OLD.intrinsic_height OR NEW.object_delivery_id IS DISTINCT FROM OLD.object_delivery_id
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
    'questionRevisionTuple',jsonb_build_object('questionId',NEW.published_question_id,'revisionNumber',NEW.revision_number),
    'asset',NEW.asset_id,'object',NEW.source_object_record_id);
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ple_private.object_record record WHERE record.object_record_id=NEW.source_object_record_id
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
DECLARE target_delivery_id uuid := COALESCE(NEW.object_delivery_id, OLD.object_delivery_id); publication ple_private.question_asset_publication%ROWTYPE;
BEGIN
    SELECT * INTO publication FROM ple_private.question_asset_publication
     WHERE question_asset_publication.object_delivery_id = target_delivery_id;
    IF NOT FOUND THEN RETURN NULL; END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_data.object_delivery delivery WHERE delivery.object_delivery_id=publication.object_delivery_id
          AND delivery.object_record_id=publication.public_object_id AND delivery.sha256=publication.public_object_checksum
          AND delivery.media_type=publication.verified_media_type AND delivery.byte_length=publication.public_byte_length
          AND delivery.delivery_state=CASE publication.publication_state WHEN 'pending' THEN 'pending' WHEN 'ready' THEN 'available' END)
       OR NOT EXISTS (SELECT 1 FROM ple_data.question_asset_delivery asset_delivery WHERE asset_delivery.object_delivery_id=publication.object_delivery_id
          AND asset_delivery.object_record_id=publication.public_object_id AND asset_delivery.published_question_id=publication.published_question_id
          AND asset_delivery.revision_number=publication.revision_number AND asset_delivery.asset_id=publication.asset_id) THEN
        RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='Question Asset Publication requires exact state-matched delivery';
    END IF;
    RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER question_asset_publication_requires_complete_delivery
AFTER INSERT OR UPDATE OR DELETE ON ple_private.question_asset_publication DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();

SET LOCAL ROLE ple_data_owner;

CREATE CONSTRAINT TRIGGER object_delivery_preserves_question_asset_publication
AFTER INSERT OR UPDATE OR DELETE ON ple_data.object_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();

CREATE CONSTRAINT TRIGGER question_asset_delivery_preserves_question_asset_publication
AFTER INSERT OR UPDATE OR DELETE ON ple_data.question_asset_delivery DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION ple_private.require_complete_question_asset_publication_delivery();

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.select_ready_question_asset_renditions(p_published_question_id text, p_revision_number integer)
RETURNS TABLE(asset_id uuid, question_asset_checksum text, rendition_checksum text, intrinsic_width integer, intrinsic_height integer)
LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, ple_data, ple_private AS $$
    SELECT publication.asset_id, encode(publication.source_object_checksum,'hex'), encode(publication.public_object_checksum,'hex'), publication.intrinsic_width, publication.intrinsic_height
    FROM ple_private.question_asset_publication publication
    JOIN ple_data.object_delivery delivery ON delivery.object_delivery_id=publication.object_delivery_id AND delivery.object_record_id=publication.public_object_id
    JOIN ple_data.question_asset_delivery asset_delivery ON asset_delivery.object_delivery_id=publication.object_delivery_id AND asset_delivery.asset_id=publication.asset_id
    WHERE publication.published_question_id=p_published_question_id AND publication.revision_number=p_revision_number
      AND publication.publication_state='ready' AND delivery.delivery_state='available' ORDER BY publication.asset_id
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.select_ready_question_asset_renditions(p_published_question_id text, p_revision_number integer)
RETURNS TABLE(asset_id uuid, question_asset_checksum text, rendition_checksum text, intrinsic_width integer, intrinsic_height integer)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.select_ready_question_asset_renditions(p_published_question_id, p_revision_number)
$$;

SET LOCAL ROLE ple_private_owner;



-- Narrow historical source read used only while forking one exact native
-- HOTSPOT Revision. Availability is deliberately absent: the fork mutation
-- rechecks it after its actor/key receipt lookup.
CREATE FUNCTION ple_private.load_question_fork_asset(
    p_published_question_id text, p_revision_number integer
) RETURNS TABLE(
    asset_id uuid, object_record_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint,
    intrinsic_width integer, intrinsic_height integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_data, ple_private AS $$
    SELECT publication.asset_id, record.object_record_id, record.object_address,
           record.sha256, record.size_bytes, record.media_type,
           pg_catalog.round(extract(epoch FROM record.created_at) * 1000)::bigint,
           publication.intrinsic_width, publication.intrinsic_height
      FROM ple_private.question_asset_publication AS publication
      JOIN ple_data.question_revision AS revision
        ON revision.published_question_id = publication.published_question_id
       AND revision.revision_number = publication.revision_number
       AND revision.backend = 'ple' AND revision.question_type = 'hotspot'
      JOIN ple_private.object_record AS record
        ON record.object_record_id = publication.source_object_record_id
       AND record.sha256 = publication.source_object_checksum
       AND record.media_type = publication.verified_media_type::text
     WHERE ple_api.current_session_account_is_instructor()
       AND publication.published_question_id = p_published_question_id
       AND publication.revision_number = p_revision_number
       AND record.object_address = pg_catalog.jsonb_build_object(
           'kind', 'restrictedQuestionAsset',
           'questionRevisionTuple', pg_catalog.jsonb_build_object(
               'questionId', p_published_question_id, 'revisionNumber', p_revision_number),
           'asset', publication.asset_id, 'object', publication.source_object_record_id)
       AND record.object_storage_area = 'private-content'
       AND record.object_data_class = 'question-asset'
$$;

SET LOCAL ROLE ple_api_owner;

CREATE FUNCTION ple_api.load_question_fork_asset(
    p_published_question_id text, p_revision_number integer
) RETURNS TABLE(
    asset_id uuid, object_record_id uuid, object_address jsonb, sha256 bytea,
    size_bytes bigint, media_type text, created_at_millis bigint,
    intrinsic_width integer, intrinsic_height integer
) LANGUAGE sql STABLE SECURITY DEFINER
SET search_path = pg_catalog, ple_api, ple_private AS $$
    SELECT * FROM ple_private.load_question_fork_asset(p_published_question_id, p_revision_number)
$$;

