# Question and Pool specification handoff

## Scope completed

The following approved specification files now exist under `docs/QUESTION_SPECS/`:

- `DRAFT_QUESTION_SPEC.md`
- `PUBLISHED_QUESTION_SPEC.md`
- `QUESTION_FORK_SPEC.md`
- `QUESTION_REVISION_SPEC.md`
- `QUESTION_POOL_SPEC.md`

They assign one focused owner for Draft Questions, Published Questions, Question Revisions and
forks, and current-state Pool behavior. They use the integrated Library Object model: Pools and
Published Questions were both part of the original design, have different object kinds, and share
discovery and the public ID namespace.

## Settled behavior carried into the specifications

- A Pool is an immediately discoverable, current-state Library Object with an unordered set of
  distinct Published Questions pinned to exact Revisions.
- Members share Discipline, Subject, Question Type, and Question Backend. They retain their own
  owners, authors, licenses, and detailed metadata.
- The Pool owns its metadata, owner, calculated license, and Pool Edit Number. It has no separate
  Author and no Revision family.
- A Pool fork has a new ID, Edit Number 1, source-Pool reference, independent state, and copied
  exact member Revisions. Forking is explicit; adding a Pool references the existing Pool ID and
  stores the selection count on the Assessment entry.
- Attempt evidence keeps Question ID, Question Revision, Pool ID, and Pool Edit Number. New
  Attempts select freshly; resumed Attempts retain their selections.
- A Pool mismatch names the violated membership rule. A Pool-use mismatch reports an Assessment
  entry asking for too many members. Save unreleased work and block affected release until
  resolved; established post-issue rules remain.
- Current license acceptance remains CC0, CC BY, and CC BY-SA. The Pool license is calculated from
  members, never selected manually. NC and ND are deferred.

## Unsettled product behavior kept out of normative rules

- Question Feedback is optional, but its timing and relation to answer visibility are deferred.
- Regrading prior submitted Native JSON responses after correcting an answer key is deferred.
  Assessment-level point changes remain the ordinary remedy for a flawed Question in issued work.
- NC acceptance and hosted-use policy remain deferred. The recorded future NC compatibility example
  is not current admission behavior. ND is also deferred.
- Automated cleanup of abandoned Drafts lacks the events, timing, warning, and recovery rules
  required for implementation.

## Implementation and evidence gaps to verify separately

This documentation pass did not certify runtime behavior. Source and prior audits identify these
follow-up checks:

- Confirm every Pool creation and membership write enforces one member per Published Question,
  exact Revision pins, common Discipline/Subject/Type/Backend, and calculated license
  compatibility.
- Confirm all mutable Pool saves advance and compare the appropriate Pool Edit Number; earlier
  audit evidence found a concurrency counter problem in some support and metadata paths, later
  reported repaired behavior should be rechecked against the final API path.
- Verify that mismatch detection identifies each concrete cause, blocks release before delivery,
  and does not disrupt an already released Assessment.
- Verify Assessment post-issue controls: no removal of an issued individual Pool member, whole-Pool
  removal recalculates earned and possible points for every Attempt, and Unrelease deletes Student
  Work before normal edits.
- Verify all Question and Pool search consumers use Pool-owned text and metadata, not member
  values, for Pool rows.
- Verify later imports, exports, generated APIs, database checks, and browser labels follow these
  rules. This handoff does not claim those alignment tasks are complete.

