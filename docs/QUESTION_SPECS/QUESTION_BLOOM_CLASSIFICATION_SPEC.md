# Question Bloom Classification specification

Bloom describes the work needed for full credit on a Question, or the intended work of a Pool as
a whole. It is ordinary editable metadata, like Title. Authority:
[HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md#question-library-bloom-classification-metadata).
[BLOOM_TAXONOMY_GUIDE.md](../BLOOM_TAXONOMY_GUIDE.md) owns teaching interpretation and Assessment
sorting. Bloom describes the intended work, separately from observed Student credit statistics.

## Values and absence

| Dimension | Values, in guide order |
| --- | --- |
| Cognitive Process | Remember, Understand, Apply, Analyze, Evaluate, Create |
| Knowledge Dimension | Factual Knowledge, Conceptual Knowledge, Procedural Knowledge, Metacognitive Knowledge |

The two dimensions have independent meanings. Together they determine the classification label
and matrix position. A Pool uses the mode of each dimension among its exact member Revisions.
Calculate the dimensions independently, ignoring NULL values in each dimension. Ties use the
guide order above; a dimension with no classified members remains NULL. Recalculate on membership
edits. Pool classification does not call JEV.

A Pool fork copies the parent Pool's current Bloom fields with its other metadata, as specified in
[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md#forking). Forking does not recalculate Bloom. Changing
the fork's membership recalculates both dimensions, just as for any other Pool. Instructor
corrections remain ordinary metadata until a subsequent membership edit recalculates the modes.

JEV assigns both dimensions to a newly published Question through the API server's in-memory queue.
The pair is saved together using the existing metadata Edit Number. Bloom may remain NULL with no
time limit if JEV is unconfigured, unavailable, or a queued request is lost during restart. Missing Bloom permits publication and Library entry. The owning Instructor can correct
either dimension; Sysadmins have the same administrative editing authority as for other metadata.

## Ordinary metadata editing

A Question's Bloom fields belong to its complete Question Revision record. Correcting Bloom on
the current record updates those fields in place and preserves its Revision Number. A new Question
Revision carries forward the record's attributes, including Bloom, subject to the changes being
published. A Pool stores its own Bloom fields in its current record.

Use the ordinary record save and concurrency checks for metadata edits. A correction preserves
the other dimension when it is unchanged. Bloom uses the same editing model as Title; it has no
separate classification counter or historical object. A late AI response cannot replace an Instructor's correction. There are no pending flags,
periodic retries, or new job tables.

Current Bloom editing uses ordinary metadata saves. A Question save carries the exact Revision
Tuple and metadata Edit Number; the complete metadata replacement includes both Bloom fields, each
nullable. The Question editor loads their current values before saving, so changing one can leave
the other unchanged. Pool Bloom fields use the ordinary Pool metadata save. See
[the Question metadata route](../../crates/server/src/question_metadata.rs),
[the Question editor](../../src/features/question_metadata/question_metadata_editor.tsx),
[the Pool editor](../../src/components/question_pool_metadata_editor.tsx). The former Question and
Pool metadata client tests included a set value with the other dimension NULL; they were retired
during test pruning and are historical evidence rather than current coverage. These source paths
are implementation evidence. Connected Bloom database checks passed; see the
[Bloom integration evidence](../active_plans/reports/bloom_integration_evidence.md). Browser
acceptance was not part of this server-side change.

## Search and Assessment use

Each dimension is an independent exact filter. Both can combine with other Library filters.
A Question matches its own fields; a Pool matches its own fields. Unclassified objects remain
visible without a Bloom filter and do not match a requested value they lack.

Return six Cognitive Process counts and four Knowledge Dimension counts in guide order, including
zero values. Counts describe the whole matching Library Object set before paging.

Assessment sorting uses the fields of the Question Revision or Pool actually referenced by the
Assessment. A Pool's own Bloom values remain independent of member corrections.

For example, correcting Apply/Procedural Knowledge to Analyze/Procedural Knowledge edits the
Question record's Cognitive Process field. It preserves the Knowledge Dimension, Question ID,
Revision Number, source, and stored grading outcomes.

## Server integration

Configure `PLE_JEV_API_KEY` or `PLE_JEV_API_KEY_FILE` on the API server. Compose passes the optional
environment variable `PLE_JEV_API_KEY`; the file setting applies to direct server deployments.
OpenTofu's `enable_jev` injects the key from the existing API secret.
The key stays server-side. Missing credentials affect only new Question classification. Pools
calculate their modes without JEV credentials.

The API queues successful Question publication and Pool creation/membership edits. The queue is
bounded and best-effort: full queues, shutdown, failed evidence preparation, and JEV failures leave
metadata unchanged. There is no separate service or recurring sweep. Existing revision inheritance
continues to copy assigned Bloom fields; a successor with an inherited classification is not sent
to JEV. Instructor corrections are ordinary metadata saves.

The client uses BPW's `jev-1.14.0` model and version 2 rubric from
`bioproblems_site/bloom_rubric.py` at commit `4d5deb6142d2834b3a9fef97ab8a459ae52c3edb`.
The criteria are copied verbatim in `crates/server/src/bloom/rubric.json`; only question scope is
adapted. Update that documented copy deliberately when BPW changes its rubric. Native evidence
contains the typed prompt, response choices, and answer mappings. WeBWorK evidence uses the
existing answer-review renderer at seed 0. Text extraction retains math, table separators, image
labels, and answer controls; executable scripts and CSS are omitted. Visual-only information can
remain incomplete, as in BPW. This is approximate search metadata, not validated measurement.
