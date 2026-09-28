-- Shared Theme vocabulary. CREATE TABLE and COMMENT ON only.

SET LOCAL ROLE ple_data_owner;

CREATE TABLE ple_data.theme (
    theme_id text PRIMARY KEY CHECK (
        theme_id = btrim(theme_id)
        AND theme_id ~ '^[a-z][a-z0-9-]{0,31}$'
    ),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.transaction_timestamp()
);

INSERT INTO ple_data.theme (theme_id) VALUES
    ('tundra'), ('forest'), ('desert'), ('grass'), ('arctic'),
    ('ocean'), ('tropical'), ('coral-reef'), ('swamp'), ('underground'),
    ('salt-marsh'), ('wetland'), ('sea-floor'), ('magma'), ('beach');

COMMENT ON TABLE ple_data.theme IS
    'role: vocabulary, reviewed Course and Instructor appearance palettes. Deleted by: none.';
