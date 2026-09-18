-- Functions referenced from CREATE TABLE CHECK constraints. They must exist
-- before 20_tables/ because PostgreSQL resolves CHECK expressions at table
-- creation.

SET LOCAL ROLE ple_private_owner;

CREATE FUNCTION ple_private.crockford_checksum_character(p_checksum_input text)
RETURNS text LANGUAGE sql IMMUTABLE STRICT
SET search_path = pg_catalog
AS $$
    SELECT substr(
        '0123456789ABCDEFGHJKMNPQRSTVWXYZ',
        (get_byte(sha256(convert_to(p_checksum_input, 'UTF8')), 0) >> 3) + 1,
        1
    )
$$;

CREATE FUNCTION ple_private.is_canonical_prefixed_public_id(
    p_public_id text, p_prefix text
) RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog, ple_private
AS $$
    SELECT p_public_id IS NOT NULL
       AND p_prefix IN ('BP', 'CI', 'A', 'U')
       AND p_public_id ~ (
           '^' || p_prefix || '[0-9ABCDEFGHJKMNPQRSTVWXYZ]{8}$'
       )
       AND right(p_public_id, 1) = ple_private.crockford_checksum_character(
           left(p_public_id, char_length(p_public_id) - 1)
       )
$$;

CREATE FUNCTION ple_private.account_time_zone_is_exact_iana(p_time_zone text)
RETURNS boolean LANGUAGE sql STABLE
SET search_path = pg_catalog
AS $$
    SELECT p_time_zone IS NOT NULL
       AND p_time_zone = btrim(p_time_zone)
       AND char_length(p_time_zone) BETWEEN 1 AND 100
       AND EXISTS (
           SELECT 1 FROM pg_catalog.pg_timezone_names AS zone
            WHERE zone.name = p_time_zone
       )
$$;

CREATE FUNCTION ple_private.assessment_template_name_is_valid(p_name text)
RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog, ple_private AS $$
    SELECT p_name IS NOT NULL
       AND char_length(p_name) <= 200
       AND p_name = btrim(
           p_name,
           U&'\0009\000A\000B\000C\000D\0020\0085\00A0\1680\2000\2001\2002\2003'
               || U&'\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000'
       )
       AND char_length(p_name) > 0
$$;

CREATE FUNCTION ple_private.question_source_binding_fields_are_valid(
    p_backend ple_data.question_backend,
    p_question_format ple_data.question_format,
    p_webwork_pg_path text,
    p_imathas_deployment_reference text, p_imathas_item_reference text,
    p_imathas_profile text, p_requires_profile boolean
) RETURNS boolean LANGUAGE sql IMMUTABLE SET search_path = pg_catalog AS $$
    SELECT COALESCE(
        (p_backend = 'ple' AND p_question_format = 'pleQuestionJson'
            AND p_webwork_pg_path IS NULL AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL AND p_imathas_profile IS NULL)
        OR (p_backend = 'webwork' AND p_question_format IN ('webworkPg', 'webworkPgml')
            AND p_webwork_pg_path IS NOT NULL AND p_imathas_deployment_reference IS NULL
            AND p_imathas_item_reference IS NULL AND p_imathas_profile IS NULL)
        OR (p_backend = 'imathas' AND p_question_format = 'imathas'
            AND p_webwork_pg_path IS NULL AND p_imathas_deployment_reference IS NOT NULL
            AND p_imathas_item_reference IS NOT NULL
            AND (p_imathas_profile IS NOT NULL) = p_requires_profile), false)
$$;

SET LOCAL ROLE ple_data_owner;

CREATE FUNCTION ple_data.course_classification_tags_are_valid(p_tags text[])
RETURNS boolean LANGUAGE sql IMMUTABLE SET search_path = pg_catalog AS $$
    SELECT p_tags IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM unnest(p_tags) AS tag(value)
            WHERE value IS NULL OR value <> btrim(value)
               OR char_length(value) NOT BETWEEN 1 AND 120 OR value ~ '[[:cntrl:]]')
       AND cardinality(p_tags) = cardinality(
           ARRAY(SELECT DISTINCT value FROM unnest(p_tags) AS tag(value)));
$$;

CREATE FUNCTION ple_data.question_metadata_tags_are_valid(p_tags text[])
RETURNS boolean LANGUAGE sql IMMUTABLE
SET search_path = pg_catalog AS $$
    SELECT p_tags IS NOT NULL
       AND NOT EXISTS (
           SELECT 1 FROM unnest(p_tags) AS tag(value)
            WHERE value IS NULL
               OR value <> btrim(value)
               OR char_length(value) NOT BETWEEN 1 AND 120
               OR value ~ '[[:cntrl:]]'
       )
       AND cardinality(p_tags) = cardinality(
           ARRAY(SELECT DISTINCT value FROM unnest(p_tags) AS tag(value)));
$$;

GRANT EXECUTE ON FUNCTION
    ple_data.course_classification_tags_are_valid(text[]),
    ple_data.question_metadata_tags_are_valid(text[])
    TO ple_private_owner;

SET LOCAL ROLE ple_private_owner;

GRANT EXECUTE ON FUNCTION
    ple_private.crockford_checksum_character(text),
    ple_private.is_canonical_prefixed_public_id(text, text),
    ple_private.account_time_zone_is_exact_iana(text),
    ple_private.assessment_template_name_is_valid(text),
    ple_private.question_source_binding_fields_are_valid(
        ple_data.question_backend, ple_data.question_format,
        text, text, text, text, boolean
    )
    TO ple_data_owner;
