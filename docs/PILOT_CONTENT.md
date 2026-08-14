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
resolve the current assignable question; internal UUIDs and immutable snapshots remain hidden for
grading and provenance.

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

Run the disposable publication oracle separately:

```bash
bash tests/e2e/e2e_chapter_one_pilot.sh
```

It starts a uniquely named PostgreSQL and MinIO project, publishes the eight immutable current
questions and two four-item Mastery assignments, then reruns the seed without mutation. Historical
pilot evidence used `P-...-v1` references; that superseded public identity is not the current
contract. Current instructor-facing IDs use canonical `AAA-BBBB` Crockford Question IDs, while
immutable snapshots stay internal for grading and provenance. The reviewed WeBWorK sources directly
admit immediate correctness without disclosing answer material. The oracle checks the
four-native/four-WeBWorK inventory, source artifacts, and absence of synthetic predecessors. It
removes only its own disposable containers and volumes. The normal local launcher uses this same
host-only seed path and writes its answer-free manifest to
`containers/local-chapter-one-pilot.json`.

Run the complete built-browser learner gate separately:

```bash
bash tests/e2e/e2e_chapter_one_browser.sh
```

It builds the browser artifacts, starts a uniquely named complete PLE stack, publishes
the same two assignments, and has the local student complete all eight questions through visible
keyboard controls without consulting answer keys. After each submission it requires the visible
feedback region to show either released feedback or the policy-correct recorded-response state. It
also requires the fresh-practice control after each four-question chapter, then removes only its own
stack and volumes. The canonical UI walkthrough visibly constructs the Genetics assignment in J13
but does not run this all-eight learner sweep; this complete two-chapter gate remains the release oracle.
