BEGIN;
SET LOCAL ROLE ple_private_owner;
SELECT 'UVSV0001' || ple_private.crockford_checksum_character('UVSV0001') AS reader_id \gset
SELECT 'BPVS00000' || ple_private.crockford_checksum_character('BPVS00000') AS blueprint_id \gset
INSERT INTO ple_private.account (account_id, user_role, created_at)
VALUES (:'reader_id', 'instructor', pg_catalog.transaction_timestamp());

SET LOCAL ROLE ple_api_owner;
INSERT INTO ple_data.blueprint_course (
    blueprint_course_id, owner_account_id, short_name, long_name,
    content_discipline_id, tags, availability, blueprint_edit_number, created_at
) VALUES (
    :'blueprint_id', :'instructor_id', 'VIS', 'Visible Blueprint',
    '73000000-0000-0000-0000-00000000cc01', ARRAY[]::text[], 'public', 1,
    pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.blueprint_course_revision (
    blueprint_course_id, blueprint_revision_number, content, content_checksum, saved_at
) VALUES (
    :'blueprint_id', 1, '{}'::jsonb, decode(repeat('0', 64), 'hex'),
    pg_catalog.transaction_timestamp()
);
INSERT INTO ple_data.blueprint_revision_event (
    blueprint_course_id, blueprint_revision_number, actor_account_id, request_checksum, occurred_at
) VALUES (
    :'blueprint_id', 1, :'instructor_id', decode(repeat('11', 32), 'hex'),
    pg_catalog.transaction_timestamp()
);

SET LOCAL ROLE ple_app;
SELECT set_config('ple.session_account_id', :'reader_id', true);
SELECT set_config('ple.visibility_blueprint_id', :'blueprint_id', true);
SELECT set_config('ple.visibility_owner_id', :'instructor_id', true);
DO $$
DECLARE
    loaded record;
    denial_count integer := 0;
    blueprint_id text := current_setting('ple.visibility_blueprint_id');
BEGIN
    SELECT * INTO loaded FROM ple_api.load_blueprint_course(blueprint_id);
    IF NOT FOUND OR loaded.is_owner OR loaded.availability <> 'public'
       OR loaded.long_name <> 'Visible Blueprint' OR loaded.blueprint_edit_number <> 1 THEN
        RAISE EXCEPTION 'visible Blueprint was not readable by the non-owner';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM ple_api.load_blueprint_revision(blueprint_id, 1)) THEN
        RAISE EXCEPTION 'visible Blueprint Revision was not readable by the non-owner';
    END IF;

    BEGIN
        PERFORM * FROM ple_api.rename_blueprint_course(blueprint_id, 1, 'HIDDEN', 'Hidden Blueprint');
        RAISE EXCEPTION 'non-owner rename was accepted';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Blueprint Course is not available' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    BEGIN
        PERFORM * FROM ple_api.update_blueprint_classification(
            blueprint_id, 1, '73000000-0000-0000-0000-00000000cc01', NULL, NULL, NULL, ARRAY[]::text[]
        );
        RAISE EXCEPTION 'non-owner classification change was accepted';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Blueprint Course is not available' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    BEGIN
        PERFORM * FROM ple_api.set_blueprint_availability(
            blueprint_id, 1, 'archived', 'Visible Blueprint'
        );
        RAISE EXCEPTION 'non-owner archive was accepted';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Blueprint Course is not available' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    BEGIN
        PERFORM * FROM ple_api.save_blueprint_course(
            blueprint_id, 1, decode(repeat('ab', 32), 'hex'), '{}'::jsonb,
            decode(repeat('0', 64), 'hex'), '[]'::jsonb
        );
        RAISE EXCEPTION 'non-owner Save was accepted';
    EXCEPTION WHEN insufficient_privilege THEN
        IF SQLERRM <> 'Blueprint Course is not available' THEN RAISE; END IF;
        denial_count := denial_count + 1;
    END;

    PERFORM set_config('ple.session_account_id', current_setting('ple.visibility_owner_id'), true);
    SELECT * INTO loaded FROM ple_api.rename_blueprint_course(blueprint_id, 1, 'VIS', 'Visible Blueprint');
    IF loaded.short_name <> 'VIS' OR loaded.long_name <> 'Visible Blueprint'
       OR loaded.availability <> 'public' OR loaded.blueprint_edit_number <> 1 THEN
        RAISE EXCEPTION 'owner no-op rename changed the visible Blueprint';
    END IF;
    IF denial_count <> 4 THEN
        RAISE EXCEPTION 'visibility denial count %', denial_count;
    END IF;
    RAISE NOTICE 'visibility_does_not_grant_editing_authority';
END $$;
COMMIT;
