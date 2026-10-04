-- Guide vocabulary: the six Cognitive Process values and four Knowledge
-- Dimension values are the only Bloom pair the database accepts.
BEGIN;
SET LOCAL ROLE ple_private_owner;
DO $$
DECLARE
    cognitive text;
    knowledge text;
BEGIN
    FOREACH cognitive IN ARRAY ARRAY[
        'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create'
    ] LOOP
        FOREACH knowledge IN ARRAY ARRAY[
            'Factual Knowledge', 'Conceptual Knowledge',
            'Procedural Knowledge', 'Metacognitive Knowledge'
        ] LOOP
            PERFORM ple_private.validate_bloom_pair(cognitive, knowledge);
        END LOOP;
    END LOOP;
    BEGIN
        PERFORM ple_private.validate_bloom_pair('Synthesize', 'Factual Knowledge');
        RAISE EXCEPTION 'invalid Bloom cognitive process was accepted';
    EXCEPTION WHEN invalid_parameter_value THEN
        NULL;
    END;
    BEGIN
        PERFORM ple_private.validate_bloom_pair('Remember', 'Strategic Knowledge');
        RAISE EXCEPTION 'invalid Bloom knowledge dimension was accepted';
    EXCEPTION WHEN invalid_parameter_value THEN
        NULL;
    END;
END
$$;
SELECT 'bloom vocabulary proof passed' AS bloom_vocabulary_proof;
ROLLBACK;
