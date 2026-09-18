-- Privileges from public_references.sql.

SET LOCAL ROLE ple_private_owner;

REVOKE ALL ON TABLE ple_private.public_id_reservation FROM PUBLIC;

REVOKE ALL ON FUNCTION ple_private.crockford_reference_suffix(),
    ple_private.crockford_checksum_character(text),
    ple_private.is_canonical_prefixed_public_id(text, text),
    ple_private.is_canonical_question_family_id(text),
    ple_private.reject_public_id_reservation_change(),
    ple_private.reserve_public_id(text, text),
    ple_private.reserve_public_id_from_trigger(),
    ple_private.assign_human_reference()
    FROM PUBLIC;

GRANT EXECUTE ON FUNCTION ple_private.crockford_reference_suffix(),
    ple_private.crockford_checksum_character(text),
    ple_private.is_canonical_prefixed_public_id(text, text),
    ple_private.is_canonical_question_family_id(text),
    ple_private.reserve_public_id(text, text),
    ple_private.reserve_public_id_from_trigger(),
    ple_private.assign_human_reference()
    TO ple_data_owner, ple_api_owner;

