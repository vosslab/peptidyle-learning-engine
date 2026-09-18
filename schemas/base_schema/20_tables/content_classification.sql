-- content_classification tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

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

CREATE TABLE ple_data.content_subject (
    subject_uuid uuid PRIMARY KEY,
    name text NOT NULL CHECK (
        char_length(name) BETWEEN 1 AND 120
        AND name !~ '^[[:space:]]|[[:space:]]$'
        AND name !~ '[[:cntrl:]]'
    )
);



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

COMMENT ON TABLE ple_data.content_discipline IS 'role: vocabulary, deleted by none; vocabulary rows persist. HUMAN_GUIDANCE.md Content classification.';

COMMENT ON TABLE ple_data.content_subject IS 'role: vocabulary, deleted by none; vocabulary rows persist. HUMAN_GUIDANCE.md Content classification.';

COMMENT ON TABLE ple_data.content_subject_discipline IS 'role: vocabulary, deleted by none; vocabulary rows persist. HUMAN_GUIDANCE.md Content classification.';

COMMENT ON TABLE ple_data.content_topic IS 'role: vocabulary, deleted by none; vocabulary rows persist. HUMAN_GUIDANCE.md Content classification.';

COMMENT ON TABLE ple_data.content_subtopic IS 'role: vocabulary, deleted by none; vocabulary rows persist. HUMAN_GUIDANCE.md Content classification.';

