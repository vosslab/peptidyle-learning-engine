-- Authored shared vocabulary for the bundled Pilot and Genetics Questions.
-- This is installation data, not a publisher fallback or Course inheritance.
-- Each pair below explicitly accepts that global Subject in that Discipline,
-- including an existing Subject previously associated with another Discipline.
-- No Topics, Subtopics, fictional Accounts, or login capabilities are needed.

-- ASVS 8.2.1: use the existing trusted installation owner; runtime roles
-- remain unable to write vocabulary tables directly.
SET LOCAL ROLE ple_data_owner;

-- Discipline names are not globally unique in storage. Serialize this fixed
-- installation declaration against other writers and reject ambiguous names
-- rather than selecting an arbitrary existing Discipline.
-- ASVS 2.3.3/2.3.4: the coordinator runs this complete fixture atomically.
LOCK TABLE ple_data.content_discipline IN SHARE ROW EXCLUSIVE MODE;

DO $$
DECLARE
    declared record;
    selected_discipline uuid;
    selected_subject uuid;
    discipline_matches bigint;
BEGIN
    -- ASVS 1.2.4/2.2.3: fixed authored values, no dynamic SQL or derived
    -- Course classification. This list is the installation's explicit approval.
    FOR declared IN
        SELECT * FROM (VALUES
            ('Biology', 'Genetics'),
            ('Biology', 'Biochemistry'),
            ('Biology', 'Molecular Biology'),
            ('Biology', 'DNA Profiling'),
            ('Biology', 'Inheritance Genetics'),
            ('Mathematics', 'Biostatistics')
        ) AS vocabulary(discipline_name, subject_name)
    LOOP
        SELECT count(*), min(item.content_discipline_id::text)::uuid
          INTO discipline_matches, selected_discipline
          FROM ple_data.content_discipline AS item
         WHERE lower(item.name) = lower(declared.discipline_name);
        IF discipline_matches > 1 THEN
            RAISE EXCEPTION USING ERRCODE = '22023',
                MESSAGE = 'Bundled content Discipline name is ambiguous';
        END IF;
        IF discipline_matches = 0 THEN
            selected_discipline := pg_catalog.gen_random_uuid();
            INSERT INTO ple_data.content_discipline(content_discipline_id, name)
            VALUES (selected_discipline, declared.discipline_name);
        END IF;

        -- Preserve global identities, display names, and all unrelated
        -- associations; only the explicitly declared association is added.
        INSERT INTO ple_data.content_subject(content_subject_id, name)
        VALUES (pg_catalog.gen_random_uuid(), declared.subject_name)
        ON CONFLICT (lower(name)) DO NOTHING;
        SELECT item.content_subject_id INTO STRICT selected_subject
          FROM ple_data.content_subject AS item
         WHERE lower(item.name) = lower(declared.subject_name);
        INSERT INTO ple_data.content_subject_discipline(content_subject_id, content_discipline_id)
        VALUES (selected_subject, selected_discipline)
        ON CONFLICT DO NOTHING;
    END LOOP;
END
$$;

RESET ROLE;
