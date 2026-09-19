-- blueprint_course tables. CREATE TABLE and COMMENT ON only.
-- Functions, policies, and privileges live in later layers.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.blueprint_course (
    blueprint_course_id ple_data.blueprint_course_id PRIMARY KEY,
    owner_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL CHECK (char_length(btrim(short_name)) BETWEEN 1 AND 500),
    long_name text NOT NULL CHECK (char_length(btrim(long_name)) BETWEEN 1 AND 500),
    content_discipline_id uuid NOT NULL REFERENCES ple_data.content_discipline(content_discipline_id),
    content_subject_id uuid,
    content_topic_id uuid,
    content_subtopic_id uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    CHECK (content_topic_id IS NULL OR content_subject_id IS NOT NULL),
    CHECK (content_subtopic_id IS NULL OR content_topic_id IS NOT NULL),
    FOREIGN KEY (content_subject_id, content_discipline_id)
        REFERENCES ple_data.content_subject_discipline(content_subject_id, content_discipline_id),
    FOREIGN KEY (content_subject_id, content_topic_id)
        REFERENCES ple_data.content_topic(content_subject_id, content_topic_id),
    FOREIGN KEY (content_topic_id, content_subtopic_id)
        REFERENCES ple_data.content_subtopic(content_topic_id, content_subtopic_id),
    availability ple_data.blueprint_availability NOT NULL DEFAULT 'private',
    promoted boolean NOT NULL DEFAULT false,
    blueprint_edit_number bigint NOT NULL CHECK (blueprint_edit_number > 0),
    current_blueprint_revision_number integer NOT NULL DEFAULT 1
        CHECK (current_blueprint_revision_number > 0),
    created_at timestamp with time zone NOT NULL,
    updated_on date NOT NULL DEFAULT CURRENT_DATE
);



CREATE TABLE ple_data.blueprint_course_revision (
    blueprint_course_id ple_data.blueprint_course_id NOT NULL
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    blueprint_revision_number integer NOT NULL CHECK (blueprint_revision_number > 0),
    content jsonb NOT NULL CHECK (jsonb_typeof(content) = 'object'),
    content_checksum bytea NOT NULL CHECK (octet_length(content_checksum) = 32),
    saved_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_id, blueprint_revision_number)
);

CREATE TABLE ple_data.blueprint_revision_question_pin (
    blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    blueprint_revision_number integer NOT NULL,
    content_path text NOT NULL CHECK (char_length(content_path) BETWEEN 1 AND 500),
    published_question_id ple_data.question_family_id NOT NULL,
    question_revision_number integer NOT NULL CHECK (question_revision_number > 0),
    PRIMARY KEY (
        blueprint_course_id, blueprint_revision_number, content_path
    ),
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number),
    FOREIGN KEY (published_question_id, question_revision_number)
        REFERENCES ple_data.question_revision (published_question_id, revision_number),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);




-- Module and Assessment References are durable identities across Revisions.
-- Their positions and Module membership belong to each immutable Revision.
CREATE TABLE ple_data.blueprint_revision_module (
    blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    blueprint_revision_number integer NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    module_position integer NOT NULL CHECK (module_position BETWEEN 1 AND 1024),
    PRIMARY KEY (
        blueprint_course_id, blueprint_revision_number,
        blueprint_module_reference
    ),
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number),
    UNIQUE (blueprint_course_id, blueprint_revision_number, module_position),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


CREATE TABLE ple_data.blueprint_revision_assessment (
    blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    blueprint_revision_number integer NOT NULL,
    blueprint_module_reference uuid NOT NULL,
    blueprint_assessment_reference uuid NOT NULL,
    assessment_position integer NOT NULL CHECK (assessment_position BETWEEN 1 AND 1024),
    PRIMARY KEY (
        blueprint_course_id, blueprint_revision_number,
        blueprint_assessment_reference
    ),
    FOREIGN KEY (
        blueprint_course_id, blueprint_revision_number,
        blueprint_module_reference
    ) REFERENCES ple_data.blueprint_revision_module (
        blueprint_course_id, blueprint_revision_number,
        blueprint_module_reference
    ),
    UNIQUE (
        blueprint_course_id, blueprint_revision_number,
        blueprint_module_reference, assessment_position
    ),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


CREATE TABLE ple_data.blueprint_revision_event (
    blueprint_revision_event_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    blueprint_revision_number integer NOT NULL,
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (blueprint_course_id, blueprint_revision_number),
    UNIQUE (blueprint_course_id, actor_account_id, request_checksum),
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number)
);

CREATE TABLE ple_data.blueprint_metadata_event (
    blueprint_metadata_event_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    blueprint_course_id ple_data.blueprint_course_id NOT NULL
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    short_name text NOT NULL,
    long_name text NOT NULL,
    -- Immutable exact snapshots, not joins to today's vocabulary associations.
    content_discipline_id uuid NOT NULL,
    content_subject_id uuid,
    content_topic_id uuid,
    content_subtopic_id uuid,
    tags text[] NOT NULL CHECK (ple_data.course_classification_tags_are_valid(tags)),
    availability ple_data.blueprint_availability NOT NULL,
    blueprint_edit_number bigint NOT NULL CHECK (blueprint_edit_number > 0),
    occurred_at timestamp with time zone NOT NULL,
    UNIQUE (blueprint_course_id, blueprint_edit_number)
);


CREATE TABLE ple_data.blueprint_course_create_receipt (
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    blueprint_revision_number integer NOT NULL,
    blueprint_edit_number bigint NOT NULL CHECK (blueprint_edit_number > 0),
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum),
    FOREIGN KEY (blueprint_course_id, blueprint_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number)
);

CREATE TABLE ple_data.blueprint_course_save_receipt (
    blueprint_course_id ple_data.blueprint_course_id NOT NULL
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    resulting_blueprint_revision_number integer NOT NULL CHECK (
        resulting_blueprint_revision_number > 0
    ),
    changed boolean NOT NULL,
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (blueprint_course_id, actor_account_id, request_checksum)
);

CREATE TABLE ple_data.blueprint_course_fork (
    blueprint_course_fork_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    blueprint_course_id ple_data.blueprint_course_id NOT NULL UNIQUE
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    source_blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    source_blueprint_revision_number integer NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    forked_at timestamp with time zone NOT NULL,
    CHECK (blueprint_course_id <> source_blueprint_course_id),
    FOREIGN KEY (
        source_blueprint_course_id, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_id, blueprint_revision_number
    )
);



-- An idempotent fork request is distinct from ordinary Blueprint creation:
-- the receipt preserves the source fact and prevents a retry from creating a
-- second child lineage.
CREATE TABLE ple_data.blueprint_course_fork_receipt (
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    request_checksum bytea NOT NULL CHECK (octet_length(request_checksum) = 32),
    blueprint_course_id ple_data.blueprint_course_id NOT NULL
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    source_blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    source_blueprint_revision_number integer NOT NULL CHECK (
        source_blueprint_revision_number > 0
    ),
    blueprint_edit_number bigint NOT NULL CHECK (blueprint_edit_number > 0),
    accepted_at timestamp with time zone NOT NULL,
    PRIMARY KEY (actor_account_id, request_checksum),
    FOREIGN KEY (
        source_blueprint_course_id, source_blueprint_revision_number
    ) REFERENCES ple_data.blueprint_course_revision (
        blueprint_course_id, blueprint_revision_number
    )
);

SET LOCAL ROLE ple_data_owner;
COMMENT ON TABLE ple_data.blueprint_course IS 'role: current state, Stable reusable Blueprint Course lineage with names, availability, Edit Number, and current Revision.';

COMMENT ON TABLE ple_data.blueprint_course_revision IS 'role: revision, Immutable complete Blueprint Revision; exact references remain valid after later Saves or archive.';

COMMENT ON TABLE ple_data.blueprint_revision_assessment IS 'role: revision, Durable Blueprint Assessment identity and Revision membership retained for provenance and comparison.';

COMMENT ON TABLE ple_data.blueprint_course_fork IS 'role: event, Exact immutable source Blueprint Revision for one independent child Blueprint lineage.';

-- Immutable source-copy Proposal evidence; canonical JSON is derived from exact
-- Revision and metadata-event pins by the existing trusted domain exporter.
CREATE TABLE ple_data.blueprint_change_proposal (
    blueprint_change_proposal_id uuid PRIMARY KEY,
    proposer_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    source_blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    source_revision_number integer NOT NULL,
    source_blueprint_edit_number bigint NOT NULL CHECK (source_blueprint_edit_number > 0),
    target_blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    target_revision_number integer NOT NULL,
    target_blueprint_edit_number bigint NOT NULL CHECK (target_blueprint_edit_number > 0),
    created_at timestamp with time zone NOT NULL,
    CHECK (source_blueprint_course_id <> target_blueprint_course_id),
    FOREIGN KEY (source_blueprint_course_id, source_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number),
    FOREIGN KEY (target_blueprint_course_id, target_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number),
    FOREIGN KEY (source_blueprint_course_id, source_blueprint_edit_number)
        REFERENCES ple_data.blueprint_metadata_event
            (blueprint_course_id, blueprint_edit_number),
    FOREIGN KEY (target_blueprint_course_id, target_blueprint_edit_number)
        REFERENCES ple_data.blueprint_metadata_event
            (blueprint_course_id, blueprint_edit_number),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp(),
    CHECK (updated_at >= created_at)
);


CREATE TABLE ple_data.blueprint_change_proposal_acceptance (
    blueprint_change_proposal_id uuid PRIMARY KEY REFERENCES ple_data.blueprint_change_proposal(blueprint_change_proposal_id),
    actor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    accepted_at timestamp with time zone NOT NULL,
    decision jsonb NOT NULL CHECK (jsonb_typeof(decision) = 'object'),
    target_blueprint_course_id ple_data.blueprint_course_id NOT NULL,
    resulting_revision_number integer NOT NULL,
    resulting_blueprint_edit_number bigint NOT NULL CHECK (resulting_blueprint_edit_number > 0),
    FOREIGN KEY (target_blueprint_course_id, resulting_revision_number)
        REFERENCES ple_data.blueprint_course_revision
            (blueprint_course_id, blueprint_revision_number),
    FOREIGN KEY (target_blueprint_course_id, resulting_blueprint_edit_number)
        REFERENCES ple_data.blueprint_metadata_event
            (blueprint_course_id, blueprint_edit_number)
);

CREATE TABLE ple_data.blueprint_course_star (
    blueprint_course_id ple_data.blueprint_course_id NOT NULL
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    instructor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    starred_at timestamptz NOT NULL,
    PRIMARY KEY (blueprint_course_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);




-- A Watch is private: there is deliberately no watcher count, list, or
-- identity projection. C409/C423 own the separately authorized Star
-- projection and the in-app notification projection.
CREATE TABLE ple_data.blueprint_course_watch (
    blueprint_course_id ple_data.blueprint_course_id NOT NULL
        REFERENCES ple_data.blueprint_course (blueprint_course_id),
    instructor_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account (account_id),
    watched_at timestamptz NOT NULL,
    PRIMARY KEY (blueprint_course_id, instructor_account_id),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);


SET LOCAL ROLE ple_private_owner;

CREATE TABLE ple_private.blueprint_course_watch_notification (
    notification_id uuid PRIMARY KEY DEFAULT pg_catalog.gen_random_uuid(),
    recipient_account_id ple_data.account_id NOT NULL REFERENCES ple_private.account(account_id),
    blueprint_course_id ple_data.blueprint_course_id NOT NULL
        REFERENCES ple_data.blueprint_course(blueprint_course_id),
    event_kind ple_data.watch_notification_event_kind NOT NULL,
    source_event_id uuid NOT NULL,
    occurred_at timestamptz NOT NULL,
    UNIQUE (recipient_account_id, event_kind, source_event_id)
);


SET LOCAL ROLE ple_data_owner;

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

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.blueprint_course_watch_notification IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_data_owner;




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
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

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.blueprint_course_watch_notification IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_data_owner;




SET LOCAL ROLE ple_private_owner;


SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
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

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.blueprint_course_watch_notification IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_data_owner;




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
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

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.blueprint_course_watch_notification IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_data_owner;





SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
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

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.blueprint_course_watch_notification IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_data_owner;




SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
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

SET LOCAL ROLE ple_private_owner;
COMMENT ON TABLE ple_private.blueprint_course_watch_notification IS 'role: event, deleted by none for published Blueprints. HUMAN_GUIDANCE.md Blueprint Courses.';

SET LOCAL ROLE ple_data_owner;

SET LOCAL ROLE ple_data_owner;
COMMENT ON COLUMN ple_data.blueprint_course.content_subject_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.blueprint_course.content_topic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.blueprint_course.content_subtopic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.blueprint_metadata_event.content_subject_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.blueprint_metadata_event.content_topic_id IS 'NULL means this optional fact is absent.';
COMMENT ON COLUMN ple_data.blueprint_metadata_event.content_subtopic_id IS 'NULL means this optional fact is absent.';

