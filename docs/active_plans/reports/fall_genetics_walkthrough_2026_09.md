# Genetics Blueprint Course: scope-correction receipt

> **Dated implementation evidence.** This report records repository state and
> conclusions at its stated time. It is not current product authority;
> [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) supersedes conflicting product
> vocabulary, lifecycle, grading, role, authorization, and interface claims below.

Reviewed September 13, 2026.

Status: M4 is complete as of 2026-09-14. The fresh canonical runtime at
`https://localhost:55390` published `BP-2`, Revision 1, in session 53170 with the r11 hashes:
11 topics, 119 banks, and 20,579 rows. The reviewed r5 browser proof verified exact Blueprint
reload, throwaway Course/Assignment creation, visible saved-radio restoration after reload and a
new authenticated Mary session, direct same-Attempt navigation, UI wrong/correct submission, and
Student-history/Gradebook agreement. Root inspected the fresh Topic 11 screenshot as legible.
Fresh outage and GET receipts establish recovery and document-read non-mutation below. The proof
runtime is intentionally disposable and makes no production-deployment claim; the published
Blueprint is stored, Available, and reusable for the installation lifetime. Authorized closeout
deleted only `tests/_temp/genetics_bank_port` (208,331 files in 1,490 directories, about 3.59 GiB),
with no archive or permanent fixture. The original 11-topic Biology Problems Website Genetics
source, runtime/volumes/`BP-2`, and Git index were untouched.

## Correct deliverable

M4 prepares one reusable Genetics Blueprint Course for the semester. The ordered `topic01` through
`topic11` entries in `OTHER_REPOS/biology-problems-website/topics_metadata.yml` become ordered
Assignments. The current `bbq-*-questions.txt` files in each topic directory define source-bank
membership. The work retains curriculum content and provenance as product data; it is not a
fixture exercise and does not make counts, checksums, generated downloads, or cached indexes into
recurring CI gates.

Biochemistry is not part of this curriculum delivery. It may inform later content work but is not
a second M4 course.

## Source and model evidence

- The website advertises 139 generated problem sets, while current Genetics topic directories
  contain 119 `bbq-*-questions.txt` banks and about 20,579 rows. Topic 3, 4, 8, and 11 generated
  lists are stale against that current membership. These are review-time source discrepancies,
  not permanent acceptance counts; current bank membership is authoritative.
- Existing Blueprint Pools contain exact published Question entries and can set `selectionCount`;
  a bank maps to a Pool using its source selection policy, defaulting to one only when no source
  policy exists.
- All six observed BBQ response shapes now use the same faithful opaque path: source rows compile
  to static WeBWorK PG. This preserves rich HTML tables, labels, response semantics, and
  backend-owned grading without a PLE-native control parser.
- The r11 staging corpus retains all 11 topics, 119 banks, and 20,579 source rows as static PG
  documents. This corpus is private, temporary production-preparation evidence, not a fixture or
  a recurring count/checksum gate.

## Conversion and renderer evidence

The r11 staging corpus compiled each source row to a static PG document and preserved source
topic/bank/row provenance. Private renderer evidence passed all 20,579 generated documents with
the supported opaque renderer path. It also exercised the six source response shapes and targeted
correct/wrong checks, including every numeric multiple-choice row affected by the explicit PG
choice-index rule. The evidence intentionally contains no answer recipe in this report.

Renderer success proves conversion and backend rendering/grading behavior only. Separate ordinary
publication, browser, outage, and GET proof establishes the disposable Blueprint behavior below.
It does not claim a production deployment.

## Evidence-led execution path

1. Completed: session 53170 published fresh `BP-2`, Revision 1, with the r11 hashes and retained
   source topic/bank grouping, exact published Question revisions, and Pool membership: 11 topics,
   119 banks, and 20,579 rows.
2. Completed: r5 reloaded `BP-2`, Revision 1, through the ordinary browser path and verified its
   11 ordered Assignments, 119 Pools, exact pins, and source selection policy.
3. Completed behavior proof: r5 created throwaway Course/Assignment records, visibly restored a
   saved radio selection after reload and a new Mary session, directly navigated to the same
   Attempt, submitted wrong (`0`) then correct (`1`) through the UI, and verified Student
   history/Gradebook agreement. Root inspected the fresh Topic 11 screenshot as legible. The r4
   outage proof recorded actual POST `503`/plain copy, restoration before and after renderer
   recovery, accepted UI submission, and a healthy renderer. GET receipt R6 records public API
   observations and reviewed SQL evidence of no Student Work/outcome writes; it does not claim a
   private database snapshot. Session 38324 separately covers no-browser expiry. Session 78713's
   aggregate passed before fresh M4 startup. Authorized closeout removed the temporary corpus/probe
   tree without an archive or permanent fixture.

The smallest backend-owned restore correction is now implemented and independently approved. The
existing authorized WeBWorK document read carries the private saved opaque input and exact Question
Attempt source pins to create an ephemeral resumed document; original issued HTML remains
immutable. It creates no API/table, expiry policy, GET write, outcome persistence, control parser,
queue, or submission/grade action. The renderer transport continues to reject answer/preview,
process, and source-URL overrides. Temporary backend-only proof passed real r11 MC, MA, FIB, NUM,
and duplicate-MA cases with visible controls restored and no feedback. The r5 receipt subsequently
completed the real PLE integration proof. Temporary corpus/probe closeout remains the only M4
recording task.

Diagnostic probes were temporary and have been removed. A rich presentation or grading semantic
that lacks a faithful supported path is refused and repaired in scope; it is never flattened,
stripped, or dropped. Any recurring test must separately satisfy
[PYTEST_STYLE.md](../../PYTEST_STYLE.md) and [TEST_EVIDENCE_MODEL.md](../../TEST_EVIDENCE_MODEL.md).

## Assignment Attempt authority

[HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) defines recovery as server-owned auto-submission of
saved responses at expiry. Saving, reconnecting, and reloading maintain continuity before expiry.
Instructors do not grade, retry, or regrade Student work. A transient backend failure preserves
saved responses. Students may submit while within the existing time limit; expired Attempts remain
closed to edits and are finalized through the ordinary submission path when the backend is
available. It creates no public grading lifecycle or retry concept.
