# Question Type specification

## Purpose and authority

A Question Type states the educational interaction a Published Question assesses. It is distinct
from its Question Backend, source format, and browser control. This specification follows
[Human Guidance](../HUMAN_GUIDANCE.md). The field table in
[QUESTION_LIBRARY_METADATA_SPEC.md](QUESTION_LIBRARY_METADATA_SPEC.md) owns common metadata.

## Required value

Every Published Question has exactly one supported Question Type. A Draft can be created, imported,
and saved without a Type. Publication requires a supported Type. Native JSON
declares it through the authored `response.kind`, which maps exactly to this vocabulary; it has no
redundant independent `questionType` source field. An importer may map a known typed QTI item to a
Type. For other Backends, Type is editable classification metadata supplied by the author or
importer. Assign WeBWorK Question Type manually for now. Automatic PG/PGML Type detection is
deferred. Backend rendering and grading remain
owned by the Backend rather than controlled by that classification tag.

The current API vocabulary is listed below. It implements the eight Types named in HG; the list
is not a rule that a Backend may support only these internal interactions.

| API value | Meaning | Native JSON response kind |
| --- | --- | --- |
| `multipleChoice` | Select exactly one choice. | `singleChoice` |
| `multipleAnswer` | Select one or more choices. | `multipleAnswer` |
| `fillInBlank` | Enter one short text answer. | `fillIn` |
| `multipleFillInBlank` | Enter several named short text answers. | `multiFillIn` |
| `numeric` | Enter one numeric answer. | `numeric` |
| `matching` | Match prompts to choices. | `matching` |
| `ordering` | Put all listed items in order. | `ordering` |
| `hotspot` | Select one or more labeled image regions. | `hotspot` |

`multipleChoice` is not a synonym for every choice-based Question, and `multipleAnswer` is not a
fallback for an unknown interaction. A backend-owned document may use an arbitrary internal
control while still declaring one of these Types for Library discovery.

## Grading

Each Question Backend grades its own Questions. Native JSON Matching credit
is the number of correct pairs divided by the total number of prompts.
Wrong and unanswered pairs earn zero without an additional deduction. PLE applies the
Assessment's partial-credit setting and point value to the stored fraction; the Backend always
calculates the earned fraction. See
[QUESTION_BACKEND_SPEC.md](QUESTION_BACKEND_SPEC.md#stored-credit-and-awarded-points).

Native JSON MC and HOTSPOT use all-or-nothing grading. NUM uses the declared tolerance.
FIB accepts alternative answers and regular expressions. MULTI-FIB combines independently graded
FIBs; each blank has equal weight. The source and grading
rules belong to [NATIVE_JSON_SPEC.md](NATIVE_JSON_SPEC.md). Native JSON ORDER averages the correct-position and correctly ordered-pair fractions; see
[ORDER_SCORING_SPEC.md](ORDER_SCORING_SPEC.md).

### Multiple Answer partial credit

Question Backends grade their own Questions. Native JSON's Multiple Answer formula, worked
examples, and scoring principles are owned by
[MULTIPLE_ANSWER_SCORING_SPEC.md](MULTIPLE_ANSWER_SCORING_SPEC.md).
Assessment Instructors decide whether to award partial credit.

## Pools, revisions, and changes

All members of a Pool share one Question Type. Correcting a Backend Question's Type tag in place
can cause a Pool mismatch; apply the rules for Questions in a Pool. Native JSON's Type follows its
source interaction, so changing that interaction creates a new Revision. A new Revision normally
inherits the preceding record's attributes except for the published changes. Existing Attempts
retain their issued content and continue normally. See
[QUESTION_REVISION_SPEC.md](QUESTION_REVISION_SPEC.md).

A classification change uses the existing Library metadata rules. The Backend continues to own
its interaction and grading. Implementing a new Native JSON interaction requires corresponding
source validation, presentation, and grading; that work is distinct from adding or correcting a
classification label for a Backend that already supports the interaction. Import/export support
is reported according to the external converter's capabilities.
