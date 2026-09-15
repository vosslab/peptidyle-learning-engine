# Student Work records

This document applies the current Assessment and Student Work model from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md). The canonical vocabulary is in
[TERMINOLOGY_CONTRACT.md](TERMINOLOGY_CONTRACT.md).

PLE separates current teaching configuration from retained Student Work. A
Course Instance Assessment is one current aggregate. An Assessment Attempt is
one occurrence of that Assessment for one Student. Published Questions,
published Question Pools, and Blueprint Courses have immutable Revisions;
Student Work merely retains their exact evidence.

## Assessment current state

An Assessment has a stable identity within its Course Instance and one current
set of teaching settings, including:

- title, instructions, and Assessment Type;
- availability and deadline values;
- Attempt limit, late-work behavior, and feedback behavior;
- Question ordering, navigation, display, and randomization behavior;
- ordered fixed Questions or Question Pool selections; and
- current point values.

An Assessment Edit Number may protect concurrent current-state saves. It is not
an Assessment Revision or historical snapshot.

An Assessment is Unreleased or Released. Release is explicit. A deadline may
make a released Assessment unavailable for new work without adding Closed or
Archived lifecycle states. Accepted edits govern future behavior and current
score calculations as described below; they do not silently replace the exact
Question or Pool Revision already selected for an open Attempt.

An Assessment copied from a Blueprint Assessment may retain the exact Blueprint
Revision and stable Blueprint Assessment reference as provenance. This does not
create an Assessment Revision or prevent later Course-local editing.

## Assessment composition

Each ordered Assessment position uses either:

- one exact Published Question Revision; or
- a Question Pool selection with an exact Pool Revision and selection count.

Starting an Attempt records the exact Questions selected from a Pool and the
backend state needed to render them. Later Question or Pool publication does not
change an existing Attempt.

## Student Work hierarchy

| Record | Owns or retains |
| --- | --- |
| Student record | One global Student Account's FERPA-protected data in one Course Instance |
| Assessment Attempt | One occurrence of an Assessment for that Student |
| Question selection | Exact Question or Pool Revision evidence and fixed position |
| Backend state | Opaque state needed to render and interpret that selected Question |
| Saved response | The Student's replaceable complete response while the Attempt is open |
| Finalized response evidence | The saved response retained as part of the submitted whole Assessment Attempt |
| Credit outcome | The Question Backend's immutable credit fraction and permitted feedback |

Every child remains constrained to the same Student, Course Instance,
Assessment, and Attempt. Browser identifiers do not establish those
relationships.

## Retained Attempt evidence

An Attempt retains only what is needed to interpret Student Work correctly:

- the Student, Course, Assessment, Attempt number, and timestamps;
- the fixed Assessment timing that applies to that Attempt;
- the exact Question or Pool Revision selections;
- randomization seed or opaque backend state;
- the Student's saved response and its whole-Attempt finalization evidence;
- the immutable credit fraction returned by each backend; and
- the feedback-disclosure condition that applies.

This is not an Assessment snapshot family, software-version archive, rendered
page archive, or general replay service. Supporting implementation evidence may
be more specific when a real backend needs it, but it may not create another
product revision concept.

## Response and submission behavior

A Student can move among all Questions in an open Attempt. A complete response
can be saved and replaced. An incomplete response is not saved as complete and
is not graded.

Saving changes only the working response. The whole Assessment Attempt is
submitted in one Student action or automatically at its deadline. That action
finalizes all saved complete responses together; other positions remain
unanswered. After submission, Student responses are immutable.

The Question Backend may evaluate a complete saved response early, but PLE does
not expose a Student-visible grading outcome until the whole Attempt is
submitted. The backend's credit fraction is immutable. Score readers multiply
that fraction by the Assessment Question's current point value, so changing
point values recalculates scores without regrading.

Human Guidance allows multiple Attempts according to Assessment policy but does
not define which Attempt contributes to a Course grade. This model therefore
does not impose highest, latest, first, or average selection.

## Authorization and concurrency

A Student works only through the exact active Student Course relationship and
Student record. Every co-Instructor has equal authority through a current
Instructor Course relationship. The Course creator or first Instructor has no
extra privilege.

Attempt start/resume, response save, whole-Assessment submission, automatic
deadline submission, and Assessment Unrelease must serialize at the Assessment
and Attempt boundary so two operations cannot create conflicting Student Work.
The implementation may use database locks or compare-and-swap values; those are
mechanisms, not product lifecycle states.

## Assessment Unrelease

Unrelease is a high-consequence Instructor action. The interface requires the
exact Assessment title as typed confirmation. The successful operation:

1. verifies equal co-Instructor authority and the current Released state;
2. changes the Assessment to Unreleased;
3. deletes all Student Work owned by that Assessment; and
4. preserves the Assessment definition, Course relationships, and shared
   Published Questions and Pools.

The action must be all-or-nothing. Human Guidance does not require a generic
audit-event architecture for every action. Any retained evidence beyond the
necessary destructive-action result must be justified by the security or
support boundary, not invented here.

## Retention

Student Work follows the Course retention clock. Removing or deactivating the
Student's Course access does not delete the global Account or Student Work. See
[RETENTION_POLICY.md](RETENTION_POLICY.md) for notice, archive, recovery-period,
and permanent-deletion rules.

## Related boundaries

- [ASSESSMENT_LIFECYCLE.md](ASSESSMENT_LIFECYCLE.md) maps the end-to-end path.
- [ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md) defines the
  Student transport boundary.
- [QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md) defines opaque
  backend ownership and credit fractions.
- [DATABASE_AUTHORIZATION.md](DATABASE_AUTHORIZATION.md) describes the database
  enforcement boundary.
