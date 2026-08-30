-- SD1 private Instructor collections and saved catalog-search roots.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.published_question TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.instructor_collection (
    collection_id uuid PRIMARY KEY,
    owner_user_id uuid NOT NULL REFERENCES ple_private.account (user_id),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 300),
    revision integer NOT NULL CHECK (revision > 0),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL CHECK (updated_at >= created_at)
);
CREATE TABLE ple_private.instructor_collection_question (
    collection_id uuid NOT NULL REFERENCES ple_private.instructor_collection (collection_id),
    question_id text NOT NULL REFERENCES ple_data.published_question (question_id),
    added_at timestamp with time zone NOT NULL,
    PRIMARY KEY (collection_id, question_id)
);
CREATE TABLE ple_private.saved_catalog_search (
    search_id uuid PRIMARY KEY,
    owner_user_id uuid NOT NULL REFERENCES ple_private.account (user_id),
    revision integer NOT NULL CHECK (revision > 0),
    filter jsonb NOT NULL CHECK (jsonb_typeof(filter) = 'object'),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL CHECK (updated_at >= created_at)
);
ALTER TABLE ple_private.instructor_collection ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.instructor_collection FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.instructor_collection_question ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.instructor_collection_question FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.saved_catalog_search ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.saved_catalog_search FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.instructor_collection, ple_private.instructor_collection_question, ple_private.saved_catalog_search FROM PUBLIC;
COMMENT ON TABLE ple_private.instructor_collection IS 'Private Instructor organization of shared published QuestionIds; not a visibility grant.';
RESET ROLE;
