-- One global vocabulary for Courses and Library Objects; Assessments have no
-- independent classification.
-- Published Question references enforce this hierarchy independently of commands.
-- UUIDs are internal identities supplied by trusted writers, not public IDs.
-- ASVS 2.1.1/2.2.1/2.2.3: names have bounded canonical storage. Subjects are
-- shared across Disciplines; Topics and Subtopics have one real parent.
-- These are implementation bounds, not HG constants:
-- Discipline/Subject 120, Topic 240, Subtopic 480 characters.
-- CHECK rejects untrimmed storage; authenticated operations normalize boundary
-- input before checking these same bounds.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.content_discipline (
    discipline_uuid uuid PRIMARY KEY,
    name text NOT NULL CHECK (
        char_length(name) BETWEEN 1 AND 120
        AND name !~ '^[[:space:]]|[[:space:]]$'
        AND name !~ '[[:cntrl:]]'
    ),
    -- Retirement is reversible: existing rows keep their stable UUID and may
    -- continue to resolve it, while new classification choices use only
    -- active Disciplines.
    is_retired boolean NOT NULL DEFAULT false
);

-- A retired name stays reserved. Renaming never changes the stable UUID and
-- restore cannot silently revive a case-only duplicate.
CREATE UNIQUE INDEX content_discipline_global_name_unique
    ON ple_data.content_discipline (lower(name));

CREATE TABLE ple_data.content_subject (
    subject_uuid uuid PRIMARY KEY,
    name text NOT NULL CHECK (
        char_length(name) BETWEEN 1 AND 120
        AND name !~ '^[[:space:]]|[[:space:]]$'
        AND name !~ '[[:cntrl:]]'
    )
);

-- One global Subject identity: preserve display case, reject case-only duplicates.
CREATE UNIQUE INDEX content_subject_global_name_unique
    ON ple_data.content_subject (lower(name));

-- Authenticated Subject creation maintains an initial association atomically;
-- association addition and replacement serialize on the Subject row. This
-- relation enforces real, nonduplicate associations independently of commands.
CREATE TABLE ple_data.content_subject_discipline (
    subject_uuid uuid NOT NULL REFERENCES ple_data.content_subject(subject_uuid),
    discipline_uuid uuid NOT NULL REFERENCES ple_data.content_discipline(discipline_uuid),
    PRIMARY KEY (subject_uuid, discipline_uuid)
);

CREATE TABLE ple_data.content_topic (
    topic_uuid uuid PRIMARY KEY,
    UNIQUE (subject_uuid, topic_uuid),
    subject_uuid uuid NOT NULL
        REFERENCES ple_data.content_subject(subject_uuid),
    name text NOT NULL CHECK (
        char_length(name) BETWEEN 1 AND 240
        AND name !~ '^[[:space:]]|[[:space:]]$'
        AND name !~ '[[:cntrl:]]'
    )
);

CREATE TABLE ple_data.content_subtopic (
    subtopic_uuid uuid PRIMARY KEY,
    UNIQUE (topic_uuid, subtopic_uuid),
    topic_uuid uuid NOT NULL REFERENCES ple_data.content_topic(topic_uuid),
    name text NOT NULL CHECK (
        char_length(name) BETWEEN 1 AND 480
        AND name !~ '^[[:space:]]|[[:space:]]$'
        AND name !~ '[[:cntrl:]]'
    )
);

-- ASVS 8.2.1/8.2.2: no runtime/API direct reads or writes are exposed by
-- the foundation. SQL ownership is an installation capability, not Sysadmin
-- Product Role authorization. Authenticated operations grant the private command
-- owner explicit access separately; API/runtime roles retain no table grants.
ALTER TABLE ple_data.content_discipline ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_discipline FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_subject ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_subject FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_subject_discipline ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_subject_discipline FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_topic ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_topic FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_subtopic ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_data.content_subtopic FORCE ROW LEVEL SECURITY;

REVOKE ALL ON TABLE ple_data.content_discipline, ple_data.content_subject,
    ple_data.content_subject_discipline, ple_data.content_topic, ple_data.content_subtopic
    FROM PUBLIC, ple_app, ple_auth, ple_student, ple_api_owner;

CREATE POLICY content_discipline_data_owner_access ON ple_data.content_discipline
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY content_subject_data_owner_access ON ple_data.content_subject
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY content_subject_discipline_data_owner_access ON ple_data.content_subject_discipline
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY content_topic_data_owner_access ON ple_data.content_topic
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);
CREATE POLICY content_subtopic_data_owner_access ON ple_data.content_subtopic
    FOR ALL TO ple_data_owner USING (true) WITH CHECK (true);

RESET ROLE;
