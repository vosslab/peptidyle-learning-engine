-- Copyable catalog locators use positive 31-bit components.  This is far more
-- capacity than the product-scoped catalog needs, while keeping every value
-- lossless in PostgreSQL, Rust, JSON, and the browser's safe-integer number
-- representation.

ALTER TABLE ONLY public.problem
    ADD CONSTRAINT problem_public_id_display_range_check
    CHECK (public_id BETWEEN 1 AND 2147483647);

ALTER TABLE ONLY public.problem_version
    ADD CONSTRAINT problem_version_number_display_range_check
    CHECK (version_number BETWEEN 1 AND 2147483647);

-- This projection is normally populated only by its owning trigger, but it is
-- also a catalog response source and must retain the same wire-safe invariant.
ALTER TABLE ONLY public.catalog_search_document
    ADD CONSTRAINT catalog_search_document_display_range_check
    CHECK (
        public_id BETWEEN 1 AND 2147483647
        AND version_number BETWEEN 1 AND 2147483647
    );

ALTER SEQUENCE public.problem_public_id_seq MAXVALUE 2147483647;
