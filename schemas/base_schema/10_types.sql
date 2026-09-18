-- Enums and domains. One definition per vocabulary.

SET LOCAL ROLE ple_data_owner;

-- Exact-Revision Bloom metadata foundation; publication/AI admission cutover is separate.
-- ASVS 2.2.1/2.2.2: both closed dimensions are required on each attached pair.
CREATE TYPE ple_data.bloom_cognitive_process AS ENUM (
    'Remember', 'Understand', 'Apply', 'Analyze', 'Evaluate', 'Create'
);

CREATE TYPE ple_data.bloom_knowledge_dimension AS ENUM (
    'Factual Knowledge', 'Conceptual Knowledge', 'Procedural Knowledge',
    'Metacognitive Knowledge'
);

SET LOCAL ROLE ple_private_owner;




-- AI preparation must finish before the short publication transaction begins.
-- This deliberately tiny, one-use receipt is not a queue, provider record, or
-- browser capability. Its fingerprint binds the prepared pair to the target
-- kind and exact immutable candidate content consumed by a Revision.
CREATE TYPE ple_private.bloom_preparation_target_kind AS ENUM (
    'question_revision', 'question_pool_revision'
);

