ALTER TABLE public.course
    ADD COLUMN term_start_date date NOT NULL,
    ADD COLUMN term_end_date date NOT NULL,
    ADD COLUMN time_zone text NOT NULL,
    ADD CONSTRAINT course_term_start_date_bounds_check CHECK (
        term_start_date BETWEEN DATE '0001-01-01' AND DATE '9999-12-31'
    ),
    ADD CONSTRAINT course_term_end_date_bounds_check CHECK (
        term_end_date BETWEEN DATE '0001-01-01' AND DATE '9999-12-31'
    ),
    ADD CONSTRAINT course_term_order_check CHECK (term_start_date <= term_end_date),
    ADD CONSTRAINT course_time_zone_shape_check CHECK (
        char_length(time_zone) BETWEEN 1 AND 255
        AND time_zone = btrim(time_zone)
        AND time_zone !~ '[[:space:]]'
    );
