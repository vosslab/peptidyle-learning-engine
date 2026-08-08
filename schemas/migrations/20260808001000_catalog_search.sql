-- MOD-UI-BROWSE: hot-metadata catalog search and keyset traversal.
--
-- The payload table remains deliberately absent from every browse query.  The
-- partial keyset index is the continuation path; the expression index matches
-- the server's metadata-only text predicate.
CREATE INDEX problem_version_catalog_search_key_idx
    ON problem_version (problem_id, version_id)
    WHERE lifecycle = 'published';

CREATE INDEX problem_version_catalog_search_text_idx
    ON problem_version
    USING gin (to_tsvector('simple', title || ' ' || metadata::text));
