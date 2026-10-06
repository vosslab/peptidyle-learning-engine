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
and matrix position. A Pool's values describe its intended work as a whole rather than an average
or maximum of its members' values.

A Pool fork copies the parent Pool's current Bloom fields with its other metadata, as specified in
[QUESTION_POOL_SPEC.md](QUESTION_POOL_SPEC.md#forking).

Initial AI assignment remains deferred. Bloom may be NULL while awaiting assignment, with no time
limit. Missing Bloom permits publication and Library entry. The owning Instructor can correct
either dimension; Sysadmins have the same administrative editing authority as for other metadata.

## Ordinary metadata editing

A Question's Bloom fields belong to its complete Question Revision record. Correcting Bloom on
the current record updates those fields in place and preserves its Revision Number. A new Question
Revision carries forward the record's attributes, including Bloom, subject to the changes being
published. A Pool stores its own Bloom fields in its current record.

Use the ordinary record save and concurrency checks for metadata edits. A correction preserves
the other dimension when it is unchanged. Bloom uses the same editing model as Title; it has no
separate classification counter or historical object. AI assignment should preserve an Instructor's
correction when that deferred feature is implemented.

The existing dedicated Bloom routes, classification counter, and broader editing permissions are
implementation drift to reconcile in [TODO.md](../TODO.md#question-spec-implementation-follow-up).
Their presence in code does not define a separate product workflow. The current API requires a
complete pair in its request; that transport choice does not require an Instructor to supply the
other dimension before editing an ordinary nullable metadata field.

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
