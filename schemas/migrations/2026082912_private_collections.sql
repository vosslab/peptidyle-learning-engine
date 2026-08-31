-- SD1 private Instructor collections and saved catalog-search roots.

SET LOCAL ROLE ple_data_owner;
GRANT USAGE ON SCHEMA ple_data TO ple_private_owner;
GRANT REFERENCES ON TABLE ple_data.published_question TO ple_private_owner;
RESET ROLE;
SET LOCAL ROLE ple_private_owner;
CREATE TABLE ple_private.question_collection (
    collection_id uuid PRIMARY KEY,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    title text NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 300),
    edit_number integer NOT NULL CHECK (edit_number > 0),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL CHECK (updated_at >= created_at)
);
CREATE TABLE ple_private.question_collection_entry (
    collection_id uuid NOT NULL REFERENCES ple_private.question_collection (collection_id),
    question_id text NOT NULL REFERENCES ple_data.published_question (question_id),
    added_at timestamp with time zone NOT NULL,
    PRIMARY KEY (collection_id, question_id)
);
CREATE TABLE ple_private.saved_question_search (
    search_id uuid PRIMARY KEY,
    owner_account_id uuid NOT NULL REFERENCES ple_private.account (account_id),
    edit_number integer NOT NULL CHECK (edit_number > 0),
    filter jsonb NOT NULL CHECK (jsonb_typeof(filter) = 'object'),
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL CHECK (updated_at >= created_at)
);
ALTER TABLE ple_private.question_collection ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_collection FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_collection_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.question_collection_entry FORCE ROW LEVEL SECURITY;
ALTER TABLE ple_private.saved_question_search ENABLE ROW LEVEL SECURITY;
ALTER TABLE ple_private.saved_question_search FORCE ROW LEVEL SECURITY;
REVOKE ALL PRIVILEGES ON TABLE ple_private.question_collection, ple_private.question_collection_entry, ple_private.saved_question_search FROM PUBLIC;
COMMENT ON TABLE ple_private.question_collection IS 'Private ordered Question Collection of shared Question lineages; not a visibility grant.';
RESET ROLE;
