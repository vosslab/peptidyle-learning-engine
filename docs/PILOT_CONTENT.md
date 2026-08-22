# Chapter 1 pilot content

The first teaching corpus contains two assignments and eight questions total. Genetics Chapter 1
and Biochemistry Chapter 1 each contain exactly:

- one algorithmic WeBWorK multiple-choice question;
- one algorithmic WeBWorK matching question;
- one static PLE flat-question multiple-choice question; and
- one static PLE flat-question matching question.

The authoritative human-readable inventory is
[`content/pilot/chapter_1_assignments.yaml`](../content/pilot/chapter_1_assignments.yaml). It uses
question slugs, titles, subject and course display names, point values, families, and source paths
rather than UUIDs. The publication seed reads that validated inventory instead of maintaining a
second human-visible catalog. The instructor UI presents one canonical `AAA-BBBB` Crockford
Question ID for each question. Server-side checksum validation and tenant/actor authorization
resolve that exact assigned question subject to lifecycle policy; internal UUIDs and immutable
snapshots remain hidden for grading and provenance.

## Source and review boundary

The source material came from `vosslab/biology-problems-website` revision
`11f9ff635bd20d8fa334c360a8cba86bb0ab6527`. The manifest records each copied file's checksum and,
for the four static questions, the selected Blackboard source-item code and compiled PLE flat JSON
checksum.

Neil R. Voss is the named author and Roosevelt University is the named institution. Educational
content is CC BY 4.0. PGML code portions retain LGPL-3.0-or-later. The two Biochemistry PGML copies
are marked as adaptations in the manifest. The review corrected `Hydroxl` to `Hydroxyl`, clarified
the positive-charge and protein-cross-link descriptions, corrected source-description grammar, and
removed wording that implied seven matching rows were displayed when the question displays four.

The four static payloads are deliberately curated questions, not an unreviewed bulk import of the
199-row source banks. Their prompts use plain text and parallel wording, and the matching questions
provide four distinct descriptions without relying on color.

## Validation

Run the tracked corpus validator from the repository root:

```bash
cargo tools pilot-content
```

It proves the durable eight-question matrix, unique human slugs, source and payload checksums,
manifest-to-flat-payload title agreement, supported point values,
selected MC/MATCH Blackboard record shape, strict PLE flat v2 compilation, CC BY metadata,
answer-free public definitions, private-key binding, and correct-versus-wrong server grading for
all four static questions.

The fixed live-demo seed/manifest and Rust behavior tests own Chapter One
publication, exact rerun, and grading semantics. The one disposable live-demo
lifecycle installs that reviewed baseline into PostgreSQL and MinIO; Chapter
One no longer creates a separately named stack or browser owner. Historical
pilot evidence used `P-...-v1` references, but current instructor-facing IDs use
canonical `AAA-BBBB` Crockford Question IDs while immutable snapshots stay
internal for grading and provenance.

Browser learner behavior is selected only through the canonical wrapper:

```bash
./run_playwright_tests.sh --build
```

That wrapper owns the production `dist/` build, HTTPS stack, browser selection,
fixed owner lease, and exact cleanup.
