# Question model

PLE separates private working Questions, reusable Published Question lineages,
immutable source Revisions, published Question Pools, and the exact Question
evidence retained in Student Work. Product intent comes from
[HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md).

## Question identity

A Question has one canonical title; compact interfaces may truncate it without
creating another title. Questions are subject agnostic, and properly tagged
content from every subject belongs in the same global Question Library.

A public Question ID identifies one stable Published Question lineage. Storage
uses eight uppercase Crockford Base32 characters; display uses `AAAA-ZBBB`.
Seven characters are cryptographically random identity, and the first character
after the hyphen is an HMAC-derived check character. It detects mistyping but is
not authorization.

A Question Revision Reference combines that stable Question ID with a positive
Revision Number. It identifies one exact immutable source-bearing Revision.
Internal UUIDs remain hidden implementation identities.

## Draft Questions

A Draft Question is private, mutable, unpublished, and unversioned. Saving
replaces its working state. An Edit Number may refuse stale saves; it does not
create a Draft Revision.

An Instructor may delete an unneeded Draft. Abandoned Drafts may be cleaned up
after appropriate warning and a recovery period. Cleanup must not damage
content already copied into a Published Question Revision.

## Publication and Revisions

Publishing a new Question creates a stable lineage and Revision 1. Publishing a
change to source, answers, grading rules, feedback, or Question assets creates
the next immutable Revision under the same ID when the Question remains the
same lineage.

Title, description, tags, subject, topic, and other search metadata belong to
the lineage and do not create a Revision. A substantive fork creates a private
Draft and, after validation, a new Published Question ID with attribution.

Published Questions are available to all vetted Instructors through the one
Question Library. Students receive Question content only through authorized
Coursework. Archive removes a Published Question from ordinary discovery/new
selection but preserves exact Revisions used by Assessments and Student Work.

Forced corrections are rare recorded Sysadmin actions for critical flaws. They
create replacement immutable Revision evidence; they do not rewrite prior
Student Work. Human Guidance does not require a generalized correction worker,
remediation manifest, or public regrading workflow.

## Question Pools

A Question Pool is an independently reusable published object with its own
public `AAAA-ZBBB` ID and immutable Pool Revisions. It has no Draft state.

Importing a Pool into a new Assessment automatically forks it for that
Assessment. The fork preserves Published Questions by public ID and can change
without mutating the source Pool.

Starting an Attempt makes fresh Pool selections. Returning to the same Attempt
preserves its selections. Student Work keeps the exact Pool Revision and
Published Question Revision delivered.

## Bloom classification

One exact Published Question Revision or Question Pool Revision has two independent
Bloom dimensions. The Cognitive Process is one of Remember, Understand, Apply,
Analyze, Evaluate, or Create. The Knowledge Dimension is one of Factual Knowledge,
Conceptual Knowledge, Procedural Knowledge, or Metacognitive Knowledge. Their ordered
pair determines the derived classification label and matrix position; neither is
a third stored field. Classification describes full-credit cognitive work, not
Question Difficulty. A Pool describes its intended work as a whole, not an inherited
or highest member classification. [BLOOM_TAXONOMY_GUIDE.md](BLOOM_TAXONOMY_GUIDE.md)
owns teaching interpretation.

Dedicated SQL relations attach a non-null checked pair to the exact composite
Question or Pool Revision key. A separate positive classification Edit Number starts
at 1. Active Instructors with current exact Library read access can correct either
or both dimensions by supplying the complete pair and expected classification Edit
Number. Keeping the unchanged dimension in that pair does not couple their meanings.
SQL locks the pair, rejects stale edits, and advances this counter only for a changed
pair. An exact no-op preserves the counter; a stale no-op still refuses. Corrections
do not change source, content Revision Numbers, member pins, lineage metadata tokens,
Assessment references, or Student Work.

The trusted initialization seam inserts once and cannot overwrite a correction.
It validates storage values, not semantic correctness or model origin. AI initial
assignment during publication and required classification before Library entry remain
the product contract, not completed runtime behavior. Current Question and Pool
Library reads project the required pair and its independent Edit Number for the exact
Revision requested, including the exact Pool Revision route. A Question summary's
legacy `latestQuestionRevision` field carries that exact requested Revision on an
exact-detail route; it does not trigger a second latest-Revision lookup.

The B2 source boundary corrects the exact Question or Pool Revision through its
dedicated `/bloom` route. An active vetted Instructor sends the complete pair and
expected classification Edit Number; authority comes from current exact Library
read access, not ownership. The pair is locked and checked for staleness before
no-op handling. A stale request receives `412`; a current no-op retains the Edit
Number and a changed pair advances it once. Sysadmins remain read-only. The browser
does not retry or merge: it reloads the same exact Revision, keeps the draft pair,
and requires explicit resave. No correction creates a content Revision.

Question Library Browse accepts the two Bloom values as independent optional exact predicates.
When either is present, it combines with every other normalized search predicate; neither changes
the existing `titleAscending` or `publishedNewest` sorts. A saved `QuestionSearchFilter`, URL
handoff, and opaque cursor retain the same normalized Bloom pair and sort; a cursor cannot continue
a different normalized query. The server returns whole-matching-set Bloom facets in guide order:
six Cognitive Process values and four Knowledge Dimension values, including zero counts and an
empty result.

Question Pool discovery applies the same two independent optional exact values to the current
Pool Revision's own classification. Both values are part of the Pool cursor's normalized query
binding. PostgreSQL applies every Pool-owned predicate before its page limit and returns the page
with complete six-plus-four matching-set counts from that one filtered relation. Counts therefore
remain available for an empty page and never derive from loaded rows or member Questions. Returning
from exact Pool inspection retains the applied Pool filters, counts, results, and position; a
classification correction refreshes the applied query before those results are shown again.

The B1/B2 source paths still need connected two-Instructor, denied-role, and
browser proof. Configured AI execution, complete publication orchestration, and
legacy classification remain open product work. The source-approved Question and Pool
Library filter/report paths still need connected multi-page, role, and browser
proof. Forks need their own whole-Pool judgment; corrected source classification
is not an automatic AI-assigned target classification. No default classification
or provider is selected.

## Assessment selection and Student Work

An Assessment contains ordered positions, each holding a Published Question or
Question Pool. The Assessment records exact Revision evidence. A new Question
or Pool Revision never advances an Assessment implicitly.

An Assessment Attempt retains:

- the Assessment position;
- exact Question Revision;
- exact Pool Revision and selected Question when applicable;
- backend-native randomization or opaque state when applicable;
- the saved response and its whole-Attempt finalization evidence;
- immutable credit fraction and protected feedback; and
- only other evidence required to interpret the work.

Native PLE Question JSON is static and receives no random seed. It may randomize
answer-choice order according to authored Question behavior. A backend such as
WeBWorK owns its parameterization and state.

Saving a complete response changes the Student's working response. The Student
may replace it while the Assessment Attempt is open. Whole-Assessment
submission finalizes all saved responses together as Student Work.

## Backend ownership

Question Type is immutable author-declared educational metadata on a Published
Question Revision. Search, filters, labels, and presentation use it. PLE never
infers it from backend controls.

The selected Question Backend owns source interpretation, rendering,
interaction, response interpretation, grading, feedback, and backend state.
PLE owns authorization, IDs, Revision selection, Assessment/Attempt workflow,
stored outcomes, score calculation, and retention.

The browser receives an answer-free native presentation or opaque backend-owned
document. It never receives Answer Keys, private source, grading input,
credentials, or raw provider results.

## Native PLE Question JSON

The native format is private, unpublished, unversioned, static, and strictly
validated. It supports MC, MA, FIB, MULTI-FIB, NUM, MATCH, ORDER, and HOTSPOT.
Stored native Questions and readers are upgraded together when the internal
shape changes.

Author JavaScript may render or support interaction in an isolated untrusted
environment. It receives no random seed, application credentials, privileged
state, or grading authority. Its only dependency declaration is the closed
`libraries` enum (currently `rdkit`); native source cannot name a dependency
URL, CDN, local path, package version, or asset digest. A separate
server-owned reviewed registry selects any runtime assets.

QTI is an import, export, and archival interchange boundary, not PLE's internal
source or runtime model. Importers translate supported external items into
PLE-managed Draft Questions without creating a second Question lifecycle.

## Assets and presentation

Question assets bind to the exact Published Question Revision. Private source,
Answer Keys, and grading inputs remain in private storage. Only answer-free
authorized renditions cross the Student boundary.

Presentation-specific references or checksums may detect stale/mismatched
valid state. They do not authenticate a Student, grant access, or decide
correctness.

## Statistics and stewardship

Published Questions can be starred and watched. Vetted Instructors may see Star
counts and which vetted Instructors starred. Watch state is private to the
watcher and drives notifications for Revisions, forks, improvement threads, and
impact notices. Students and anonymous users receive neither Instructor
identity lists nor Watch information.

Question Statistics are Revision-specific privacy-safe aggregates. They may
include accepted graded-Attempt, correct, incorrect, partial-credit, unanswered,
and eligible answer-choice counts. Clearly labeled Question-level rollups may
combine Revisions. Statistics may survive Student-record deletion only after
privacy thresholds ensure that individual Students cannot reasonably be
identified. Course-local or small-cell analyses remain FERPA-protected.

Unanswered remains a visible response status and is also counted as incorrect
for scoring and aggregate correctness. It does not create a response or backend
evaluation.

## Revision boundary

Published Questions, published Question Pools, and Blueprint Courses have
immutable Revisions. Revision families do not extend to Assessment current
state, Attempts, responses, corrections, metadata, or retention.

See [QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md),
[QUESTION_BACKEND_CONTRACTS.md](QUESTION_BACKEND_CONTRACTS.md),
[ASSESSMENT_PAYLOAD_DESIGN.md](ASSESSMENT_PAYLOAD_DESIGN.md), and
[AUTHORIZATION_CONTRACTS.md](AUTHORIZATION_CONTRACTS.md).
