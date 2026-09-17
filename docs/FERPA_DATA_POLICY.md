# FERPA data policy

PLE collects Student educational data reluctantly, uses it deliberately, exposes
it only to authorized users, and purges it predictably. This is PLE's protective
product classification, not a categorical legal ruling, legal advice, or
institutional certification.

## Related data policies

These policies intentionally overlap where FERPA classification, authorization,
and retention meet. This policy is the primary FERPA reference and owns the
Student educational-record boundary. [RETENTION_POLICY.md](RETENTION_POLICY.md)
owns retention-lifecycle details, and
[DATA_CLASSIFICATION.md](DATA_CLASSIFICATION.md) owns the broader
data-classification and handling model.

User decisions and [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) remain the overall
authority.

## Protected Student records

For PLE, a protected Student educational record is Course-scoped evidence of a
Student's educational activity or outcome. It includes:

- Identifiable enrollment and Course-membership history.
- Assessment Attempts and their state.
- Saved and final Question responses.
- Grades, scores, credit, and grading outcomes.
- Student-specific feedback.
- Student-linked timing, deadlines, accommodations, and activity.
- Issued Question and Pool selections, including the exact selected Revisions.
- Interpretation evidence needed to understand or grade delivered work.
- Student analytics.
- Opaque or pseudonymous IDs that can be linked to any protected evidence.

**Student Work** is the umbrella for protected Course Instance evidence:
Assessment Attempts, saved and final responses, results, and the interpretation
evidence needed to understand submitted work. It is Course-scoped, not global
Account data. The underlying records retain their own identities and purposes.

Removing a name, removing a Student from a Course, or deactivating an Account
does not remove this sensitivity. The protected portion of an otherwise
reusable object remains protected when its content or relationship embeds this
Student-linked evidence.

## Records outside this boundary

The following are not Student educational records when genuinely independent
of identifiable or linkable individual Student evidence:

- Reusable Course metadata, Assessment definitions, configuration, and settings.
- Questions, Question Pools, Blueprint Courses, and their Revisions.
- Instructor authoring content.
- Independent global Account data.
- Genuinely privacy-safe, version-first aggregates from accepted graded
  Attempts, correct counts, and eligible choice counts, with clearly labelled
  Question rollups, only when they cannot identify or reconstruct a Student's
  activity.

This is a content-and-relationship boundary, not a blanket label for every
file or table. Independent global Account data remains personal and
authentication-sensitive, and is not automatically public. Likewise,
"not FERPA" does not mean public: Answer Keys, private authoring, credentials,
and private backend state retain their own access restrictions.
A Published Question reused by reference does not become Student Work; its
Student-linked delivery or selection record remains protected.

## Access and recovery

In normal access, authorized Students use only their own protected Course
records. Current Course Instructors use only the authorized teaching
projections for their Course; co-Instructors have equal authority. The Sysadmin
Product Role has no ambient FERPA access. Support access is specific to a task,
scoped to it, and recorded.

After Instructor notice, archived records leave ordinary Student and Instructor
browsing. An authorized current Course Instructor may explicitly recover an
archived record before permanent deletion. Accidental or premature archival
does not remove that recovery until the original deletion deadline. Recovery is
not ordinary browsing, preserves the Course authorization boundary, and neither
resets nor extends the original retention deadline. Its delivery mechanism is
intentionally not invented here.

## Deletion and survivors

At the permanent-deletion milestone, PLE deletes protected Student educational
records and does not retain a hidden product archive through logs, backups,
caches, or similar paths. [RETENTION_POLICY.md](RETENTION_POLICY.md) specifies
the notice, archive, recovery, and deletion timing.

Reusable teaching content and independent global Accounts persist separately
from Course Student Work, but a surviving portion cannot contain protected
Student-linked evidence. Genuinely privacy-safe, version-first aggregate
rollups of accepted graded Attempts, correct counts, and eligible choice counts
may survive only when clearly labelled and unable to identify or reconstruct
individual Student activity. An ID, URL, cache key, reference, or claimed role
is not authority; the server derives access from authenticated context and
stored relationships.
