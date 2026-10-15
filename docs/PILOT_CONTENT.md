# Chapter 1 pilot content

The first teaching set contains two Assessments and eight Questions total. Genetics Chapter 1
and Biochemistry Chapter 1 each contain exactly:

- one algorithmic WeBWorK multiple-choice question;
- one algorithmic WeBWorK matching question;
- one static PLE PLE Question JSON multiple-choice question; and
- one static PLE PLE Question JSON matching question.

The authoritative human-readable inventory is
[`content/pilot/chapter_1_assessments.yaml`](../content/pilot/chapter_1_assessments.yaml). It uses
question slugs, titles, subject and course display names, point values, Question Types, and source paths
rather than UUIDs. The publication seed reads that validated inventory instead of maintaining a
second human-visible question list. The Instructor UI presents one canonical `AAAA-ZBBB` Crockford
Question ID for each question. Server-side checksum validation and Account authorization
resolve that exact selected Question subject to lifecycle policy; internal UUIDs
and the minimum backend/Revision evidence remain hidden.

## Source and review boundary

The source material came from `vosslab/biology-problems-website` revision
`11f9ff635bd20d8fa334c360a8cba86bb0ab6527`. The manifest records each copied file's checksum and,
for the four static Questions, the selected Blackboard source-item code and compiled PLE Question JSON
checksum.

Neil R. Voss is the named author and Roosevelt University is the named affiliation. Educational
content is CC BY 4.0. PGML code portions retain LGPL-3.0-or-later. The two Biochemistry PGML copies
are marked as adaptations in the manifest. The review corrected `Hydroxl` to `Hydroxyl`, clarified
the positive-charge and protein-cross-link descriptions, corrected source-description grammar, and
removed wording that implied seven matching rows were displayed when the question displays four.

The four static payloads are deliberately curated questions, not an unreviewed bulk import of the
199-row source banks. Their prompts use plain text and parallel wording, and the matching questions
provide four distinct descriptions without relying on color.

## Current installation

This small curated teaching set remains source material. Its miniature publisher
has been retired. Fresh installation imports complete BPW collections and bundled
Genetics PGML through [the shared Python loader](../launchers/import_bpw.py),
then adopts full Biochemistry through ordinary HTTP workflows. See
[LIVE_DEMO_SPEC.md](LIVE_DEMO_SPEC.md) for representative Student Work.
