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
- The version-first Question Library object usage statistics defined below, which hold counts
  and credit sums per Published Question Revision and cannot identify or
  reconstruct a Student's activity.

This is a content-and-relationship boundary, not a blanket label for every
file or table. Independent global Account data remains personal and
authentication-sensitive, and is not automatically public. Likewise,
"not FERPA" does not mean public: Answer Keys, private authoring, credentials,
and private backend state retain their own access restrictions.
A Published Question reused by reference does not become Student Work; its
Student-linked delivery or selection record remains protected.

## Question Library object usage statistics

PLE keeps one global, version-first usage statistic per Published Question Revision so
Instructors can judge a Question's difficulty in the Library. This section states exactly what
that statistic collects, stores, and shows. The schema shape lives in
[DATABASE_STYLE.md](DATABASE_STYLE.md) ("Every table has a clock") and
`schemas/base_schema/20_tables/statistics.sql`.

Collected once per Issued Question when its Assessment Attempt is submitted (an Attempt that is
Unreleased or deleted before submission contributes nothing):

- the exact `(question_id, revision_number)` issued;
- whether a saved response exists (blank Questions are never sent to a Question Backend), and
  for a graded response its credit fraction.

Every Attempt counts, including practice Attempts after full credit; the pedagogy treats each
Attempt as one observation.

Stored in two places with two lifetimes:

- The per-observation receipt is Student Work. It lives in `ple_private`, is rooted in the Issued
  Question and its submission, exists so each observation increments the totals exactly once, and
  is purged with the Attempt.
- The aggregate is global content, keyed by `(question_id, revision_number)`, holding exactly:
  `issued_count`, `blank_count`, `answered_count`, `correct_count` (credit = 1), `partial_count`
  (0 < credit < 1), `incorrect_count` (credit = 0), `credit_sum`, `credit_sum_sq`, the date
  the Revision was published (`created_on`), and the calendar date of the most recent
  increment (`updated_on`). `issued_count = blank_count + answered_count` and
  `answered_count = correct_count + partial_count + incorrect_count`; mean and standard deviation
  of credit derive from the sums. Unrelease and FERPA deletion leave these totals unchanged; PLE
  never rebuilds them from Student Work.
- For example, when a Student scores `credit = 0.6` on a Question, its
  `(question_id, revision_number)` row is updated with `issued_count += 1`,
  `answered_count += 1`, `partial_count += 1`, `credit_sum += 0.6`, `credit_sum_sq += 0.36`,
  and `updated_on = <today>`. A full-credit response instead adds `correct_count += 1`,
  `credit_sum += 1`, `credit_sum_sq += 1`. A blank Question adds only `issued_count += 1`,
  `blank_count += 1`, and the date; the sums are untouched because no credit was graded.

Each Question Pool keeps two stored counters: an `issued_count` for the Pool, and a
`selected_count` per member Published Question, removed with the member. Its difficulty is
derived at read time from its current members' Question rows; nothing about outcomes is stored
per Pool. Question storage stays separate per Revision; every bulk
view shows the rollup across all Revisions, and the Question detail page is the one place that
adds the per-Revision breakdown.

The aggregate row is built from those columns and those only. Re-identification comes from
linkage, so the following stay in Student Work: any Course, Student, Account, Attempt,
submission, or roster reference; any time of day; any point value, timing, ordering, seed,
selected choice, or free-text response; any per-Course or per-term breakdown. The row records
global totals and a last-increment calendar date, so an observer with a roster learns at most that
some Student somewhere answered that day.

Shown to Instructors in the Question Library with the count of observations beside every rate.
Students see Course-scoped class statistics through the Assessment feedback policy instead;
those are projections of protected Student Work for their Course, governed by the Assessment's
feedback settings, and purged with the Course.

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
Student-linked evidence. The Question Library object usage statistics defined above survive
because they hold only per-Revision counts and credit sums. An ID, URL, cache
key, reference, or claimed role
is not authority; the server derives access from authenticated context and
stored relationships.
