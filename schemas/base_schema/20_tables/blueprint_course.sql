-- blueprint_course tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.blueprint_course (
    blueprint_id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE NOT NULL,
    public_reference text NOT NULL UNIQUE CHECK (
        ple_private.is_canonical_prefixed_public_id(public_reference, 'BP')
    ),
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL CHECK (char_length(btrim(short_name)) BETWEEN 1 AND 500),
    long_name text NOT NULL CHECK (char_length(btrim(long_name)) BETWEEN 1 AND 500),
    discipline_uuid uuid NOT NULL REFERENCES ple_data.content_discipline(discipline_uuid),
    subject_uuid uuid,
    topic_uuid uuid,
    subtopic_uuid uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    CHECK (topic_uuid IS NULL OR subject_uuid IS NOT NULL),
    CHECK (subtopic_uuid IS NULL OR topic_uuid IS NOT NULL),
    FOREIGN KEY (subject_uuid, discipline_uuid)
        REFERENCES ple_data.content_subject_discipline(subject_uuid, discipline_uuid),
    FOREIGN KEY (subject_uuid, topic_uuid)
        REFERENCES ple_data.content_topic(subject_uuid, topic_uuid),
    FOREIGN KEY (topic_uuid, subtopic_uuid)
        REFERENCES ple_data.content_subtopic(topic_uuid, subtopic_uuid),
    -- A new reusable Blueprint is owner-only until its owner explicitly makes
    -- it Public. Public is the sole state eligible for discovery/adoption;
    -- Archived remains readable by exact historical Revision reference only.
    availability text NOT NULL DEFAULT 'private'
        CHECK (availability IN ('private', 'public', 'archived')),
    promoted boolean NOT NULL DEFAULT false,
    metadata_etag uuid NOT NULL,
    current_blueprint_revision_number bigint NOT NULL DEFAULT 1
        CHECK (current_blueprint_revision_number > 0),
    created_at timestamp with time zone NOT NULL,
    CONSTRAINT blueprint_course_reference_is_bounded CHECK (
        reference_number BETWEEN 1 AND 2147483647
    )
);

CREATE TABLE ple_data.blueprint_course_revision (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    blueprint_revision_number bigint NOT NULL CHECK (blueprint_revision_number > 0),
    content jsonb NOT NULL CHECK (jsonb_typeof(content) = 'object'),
    content_checksum bytea NOT NULL CHECK (octet_length(content_checksum) = 32),
    saved_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE TABLE ple_data.blueprint_revision_question_pin (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    content_path text NOT NULL CHECK (char_length(content_path) BETWEEN 1 AND 500),
    question_id text NOT NULL,
    question_revision_number bigint NOT NULL CHECK (question_revision_number > 0),
    PRIMARY KEY (
        blueprint_course_reference_number, blueprint_revision_number, content_path
    ),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number),
    FOREIGN KEY (question_id, question_revision_number)
        REFERENCES ple_data.question_revision (question_id, revision_number)
);



-- Module and Assessment References are durable identities across Revisions.
-- Their positions and Module membership belong to each immutable Revision.
CREATE TABLE ple_data.blueprint_revision_module (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    module_position integer NOT NULL CHECK (module_position BETWEEN 1 AND 1024),
    PRIMARY KEY (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference
    ),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number),
    UNIQUE (blueprint_course_reference_number, blueprint_revision_number, module_position)
);

CREATE TABLE ple_data.blueprint_revision_assessment (
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    blueprint_assessment_reference uuid NOT NULL,
    assessment_position integer NOT NULL CHECK (assessment_position BETWEEN 1 AND 1024),
    PRIMARY KEY (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_assessment_reference
    ),
    FOREIGN KEY (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference
    ) REFERENCES ple_data.blueprint_revision_module (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference
    ),
    UNIQUE (
        blueprint_course_reference_number, blueprint_revision_number,
        blueprint_module_reference, assessment_position
    )
);

CREATE TABLE ple_data.blueprint_revision_event (
    blueprint_revision_event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (blueprint_course_reference_number, blueprint_revision_number),
    UNIQUE (blueprint_course_reference_number, actor_account_id, request_checksum),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE TABLE ple_data.blueprint_metadata_event (
    blueprint_metadata_event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL,
    long_name text NOT NULL,
    -- Immutable exact snapshots, not joins to today's vocabulary associations.
    discipline_uuid uuid NOT NULL,
    subject_uuid uuid,
    topic_uuid uuid,
    subtopic_uuid uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    availability text NOT NULL CHECK (availability IN ('private', 'public', 'archived')),
    metadata_etag uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (blueprint_course_reference_number, metadata_etag)
);

CREATE TABLE ple_data.blueprint_course_create_receipt (
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_reference_number bigint NOT NULL,
    blueprint_revision_number bigint NOT NULL,
    metadata_etag uuid NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum),
    FOREIGN KEY (blueprint_course_reference_number, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number)
);

CREATE TABLE ple_data.blueprint_course_save_receipt (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    resulting_blueprint_revision_number bigint NOT NULL CHECK (
        resulting_blueprint_revision_number > 0
    ),
    changed boolean NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, actor_account_id, request_checksum)
);

CREATE TABLE ple_data.blueprint_course_fork (
    blueprint_course_reference_number bigint PRIMARY KEY
        REFERENCES ple_data.blueprint_course (reference_number),
    source_blueprint_course_reference_number bigint NOT NULL,
    source_blueprint_revision_number bigint NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    forked_at timestamp with time zone NOT NULL,
    CHECK (blueprint_course_reference_number <> source_blueprint_course_reference_number),
    FOREIGN KEY (
        source_blueprint_course_reference_number, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    )
);



-- An idempotent fork request is distinct from ordinary Blueprint creation:
-- the receipt preserves the source fact and prevents a retry from creating a
-- second child lineage.
CREATE TABLE ple_data.blueprint_course_fork_receipt (
    actor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    source_blueprint_course_reference_number bigint NOT NULL,
    source_blueprint_revision_number bigint NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    metadata_etag uuid NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum),
    FOREIGN KEY (
        source_blueprint_course_reference_number, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_reference_number, blueprint_revision_number
    )
);

COMMENT ON TABLE ple_data.blueprint_course IS 'role: current state, Stable reusable Blueprint Course lineage with names, availability, metadata ETag, and current Revision.';

COMMENT ON TABLE ple_data.blueprint_course_revision IS 'role: revision, Immutable complete Blueprint Revision; exact references remain valid after later Saves or archive.';

COMMENT ON TABLE ple_data.blueprint_revision_assessment IS 'role: revision, Durable Blueprint Assessment identity and Revision membership retained for provenance and comparison.';

COMMENT ON TABLE ple_data.blueprint_course_fork IS 'role: event, Exact immutable source Blueprint Revision for one independent child Blueprint lineage.';

-- Immutable source-copy Proposal evidence; canonical JSON is derived from exact
-- Revision and metadata-event pins by the existing trusted domain exporter.
CREATE TABLE ple_data.blueprint_change_proposal (
    proposal_id uuid PRIMARY KEY,
    proposer_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    source_reference_number bigint NOT NULL,
    source_revision_number bigint NOT NULL,
    source_metadata_etag uuid NOT NULL,
    target_reference_number bigint NOT NULL,
    target_revision_number bigint NOT NULL,
    target_metadata_etag uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    CHECK (source_reference_number <> target_reference_number),
    FOREIGN KEY (source_reference_number, source_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number),
    FOREIGN KEY (target_reference_number, target_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number),
    FOREIGN KEY (source_reference_number, source_metadata_etag)
        REFERENCES ple_data.blueprint_metadata_event
            (blueprint_course_reference_number, metadata_etag),
    FOREIGN KEY (target_reference_number, target_metadata_etag)
        REFERENCES ple_data.blueprint_metadata_event
            (blueprint_course_reference_number, metadata_etag)
);

CREATE TABLE ple_data.blueprint_change_proposal_acceptance (
    proposal_id uuid PRIMARY KEY REFERENCES ple_data.blueprint_change_proposal(proposal_id),
    actor_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    accepted_at timestamp with time zone NOT NULL,
    decision jsonb NOT NULL CHECK (jsonb_typeof(decision) = 'object'),
    target_reference_number bigint NOT NULL,
    resulting_revision_number bigint NOT NULL,
    resulting_metadata_etag uuid NOT NULL,
    FOREIGN KEY (target_reference_number, resulting_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_reference_number, blueprint_revision_number),
    FOREIGN KEY (target_reference_number, resulting_metadata_etag)
        REFERENCES ple_data.blueprint_metadata_event
            (blueprint_course_reference_number, metadata_etag)
);

CREATE TABLE ple_data.blueprint_course_star (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, instructor_account_id)
);



-- A Watch is private: there is deliberately no watcher count, list, or
-- identity projection. C409/C423 own the separately authorized Star
-- projection and the in-app notification projection.
CREATE TABLE ple_data.blueprint_course_watch (
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course (reference_number),
    instructor_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (blueprint_course_reference_number, instructor_account_id)
);

SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.blueprint_course_watch_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    recipient_account_id uuid NOT NULL REFERENCES ple_private.account(account_id),
    blueprint_course_reference_number bigint NOT NULL
        REFERENCES ple_data.blueprint_course(reference_number),
    event_kind text NOT NULL CHECK (event_kind IN (
        'revision', 'published', 'archived', 'restored'
    )),
    source_event_id bigint NOT NULL CHECK (source_event_id > 0),
    occurred_at timestamptz NOT NULL,
    UNIQUE (recipient_account_id, event_kind, source_event_id)
);

SET LOCAL ROLE ple_data_owner;

-- Atomic creation of a new Blueprint Course from current reusable Course structure.



-- This relation is source provenance, not daughter-Course adoption provenance.
-- A Course Instance remains unchanged and never acquires a parent Blueprint
-- when its reusable structure is copied into a new lineage.
CREATE TABLE ple_data.blueprint_course_instance_source (
    blueprint_course_reference_number bigint PRIMARY KEY
        REFERENCES ple_data.blueprint_course (reference_number),
    source_course_id uuid NOT NULL REFERENCES ple_data.course_instance (course_id),
    recorded_at timestamp with time zone NOT NULL
);

COMMENT ON TABLE ple_data.blueprint_revision_question_pin IS 'role: revision, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_revision_module IS 'role: revision, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_revision_event IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_metadata_event IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_course_create_receipt IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_course_save_receipt IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_course_fork_receipt IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_change_proposal IS 'role: current state, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_change_proposal_acceptance IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_course_star IS 'role: current state, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

COMMENT ON TABLE ple_data.blueprint_course_watch IS 'role: current state, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_private_owner;

COMMENT ON TABLE ple_private.blueprint_course_watch_notification IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_data_owner;

COMMENT ON TABLE ple_data.blueprint_course_instance_source IS 'role: current state, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

