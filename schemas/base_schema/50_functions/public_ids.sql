-- Functions, triggers, and views from public_ids.sql.

SET LOCAL ROLE ple_private_owner;

-- Canonical browser-visible public identifiers. Each stored value is the ID;
-- no display reconstruction, compact form, or normalization is permitted.



-- Seven independent five-bit draws give the random Crockford Base32 suffix
-- without exposing creation order. UUID byte 6 has RFC 4122 version bits, so
-- it is deliberately skipped; these seven source bytes each provide all five
-- selected random bits.
CREATE FUNCTION ple_private.crockford_id_suffix()
RETURNS text LANGUAGE sql VOLATILE
SET search_path = pg_catalog
AS $$
    WITH bytes AS (
        SELECT decode(replace(gen_random_uuid()::text, '-', ''), 'hex') AS value
    )
    SELECT string_agg(
        substr('0123456789ABCDEFGHJKMNPQRSTVWXYZ', (get_byte(value, byte_position) & 31) + 1, 1),
        '' ORDER BY position
    )
      FROM bytes,
           unnest(ARRAY[0, 1, 2, 3, 4, 5, 7]) WITH ORDINALITY
               AS source(byte_position, position)
$$;



-- ASVS 2.1.1, 2.2.1, 2.2.2: enforce the documented canonical public-ID
-- checksum at the trusted database boundary. Callers supply the exact
-- canonical Crockford characters before Z; separators and Z are excluded by
-- the caller, never normalized from a stored identifier. The public,
-- unsalted SHA-256 derivation permits independent checksum verification.
CREATE FUNCTION ple_private.reject_public_id_reservation_change()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = '55000',
        MESSAGE = 'Public ID reservations are permanent';
END
$$;

CREATE TRIGGER public_id_reservation_is_permanent
BEFORE UPDATE OR DELETE ON ple_private.public_id_reservation
FOR EACH ROW EXECUTE FUNCTION ple_private.reject_public_id_reservation_change();



-- The registry primary key is the transactional collision boundary. A failed
-- enclosing object insert rolls its reservation back with that transaction;
-- a committed reservation remains as the no-reuse tombstone.
CREATE FUNCTION ple_private.reserve_public_id(
    p_canonical_public_id text, p_object_kind text
) RETURNS void LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, ple_private
AS $$
BEGIN
    IF p_canonical_public_id IS NULL
       OR p_object_kind NOT IN (
           'account', 'assessment', 'blueprint_course', 'course_instance',
           'published_question', 'question_pool'
       ) THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Public ID reservation is invalid';
    END IF;

    INSERT INTO ple_private.public_id_reservation(canonical_public_id, object_kind)
    VALUES (
        p_canonical_public_id,
        p_object_kind::ple_data.public_id_object_kind
    );
EXCEPTION WHEN unique_violation THEN
    -- QP001 is the shared, retryable collision signal for the common
    -- Published Question/Question Pool namespace. The existing Question
    -- publication and Pool creation adapters both recognize it without
    -- inspecting a registry implementation constraint name.
    RAISE EXCEPTION USING ERRCODE = 'QP001',
        MESSAGE = 'Public ID has already been issued';
END
$$;



-- A supplied Question-family ID is registered by the owning table's insert
-- trigger.  Typed server code still performs syntax/checksum validation; this
-- is only the cross-type, lifetime uniqueness boundary.
CREATE FUNCTION ple_private.reserve_public_id_from_trigger()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
DECLARE canonical_public_id text;
BEGIN
    IF TG_NARGS <> 2 OR TG_ARGV[0] NOT IN ('published_question', 'question_pool')
       OR TG_ARGV[1] NOT IN ('published_question_id', 'question_pool_id') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Public ID reservation trigger is invalid';
    END IF;
    canonical_public_id := pg_catalog.to_jsonb(NEW) ->> TG_ARGV[1];
    PERFORM ple_private.reserve_public_id(canonical_public_id, TG_ARGV[0]);
    RETURN NEW;
END
$$;



-- Mint placeholders pass the domain CHECK, then this trigger replaces them.
-- A caller-supplied canonical ID that is not a mint placeholder is reserved
-- and kept so installation and Live Demo can store the same public ID the
-- application already holds.
CREATE FUNCTION ple_private.assign_public_id()
RETURNS trigger LANGUAGE plpgsql
SET search_path = pg_catalog, ple_private
AS $$
DECLARE
    candidate text;
    object_kind text;
    supplied text;
    mint_placeholder text;
BEGIN
    IF TG_NARGS <> 1 OR TG_ARGV[0] NOT IN ('BP', 'CI', 'A', 'U') THEN
        RAISE EXCEPTION USING ERRCODE = '22023',
            MESSAGE = 'Public-ID minting requires a supported ID prefix';
    END IF;
    object_kind := CASE TG_ARGV[0]
        WHEN 'BP' THEN 'blueprint_course'
        WHEN 'CI' THEN 'course_instance'
        WHEN 'A' THEN 'assessment'
        WHEN 'U' THEN 'account'
    END;
    supplied := CASE TG_ARGV[0]
        WHEN 'U' THEN NEW.account_id
        WHEN 'CI' THEN NEW.course_instance_id
        WHEN 'BP' THEN NEW.blueprint_course_id
        WHEN 'A' THEN NEW.assessment_id
    END;
    mint_placeholder := CASE TG_ARGV[0]
        WHEN 'U' THEN 'U00000009'
        WHEN 'CI' THEN 'CI0000000Y'
        WHEN 'A' THEN 'A0000000A'
        WHEN 'BP' THEN 'BP0000000C'
    END;
    IF supplied IS NOT NULL
       AND supplied IS DISTINCT FROM mint_placeholder
       AND ple_private.is_canonical_prefixed_public_id(supplied, TG_ARGV[0]) THEN
        PERFORM ple_private.reserve_public_id(supplied, object_kind);
        RETURN NEW;
    END IF;
    LOOP
        candidate := TG_ARGV[0] || ple_private.crockford_id_suffix();
        candidate := candidate || ple_private.crockford_checksum_character(candidate);
        BEGIN
            PERFORM ple_private.reserve_public_id(candidate, object_kind);
            EXIT;
        EXCEPTION WHEN SQLSTATE 'QP001' THEN
            -- A concurrent or historic reservation used this random value.
            -- Draw again; the registry remains the final global boundary.
        END;
    END LOOP;
    IF TG_ARGV[0] = 'U' THEN
        NEW.account_id := candidate;
    ELSIF TG_ARGV[0] = 'CI' THEN
        NEW.course_instance_id := candidate;
    ELSIF TG_ARGV[0] = 'BP' THEN
        NEW.blueprint_course_id := candidate;
    ELSIF TG_ARGV[0] = 'A' THEN
        NEW.assessment_id := candidate;
    END IF;
    RETURN NEW;
END
$$;

