# Biology course specification handoff

## Delivered documentation

This work created the ten planned BiologyProblems.org specifications in
`docs/BIOLOGY_PROBLEMS_SPECS/` and the reusable
[BLUEPRINT_COURSE_IMPORT_API_SPEC.md](../../BLUEPRINT_COURSE_IMPORT_API_SPEC.md). They define:

- source selection, provenance, PG/PGML preservation, native JSON conversion, QTI reuse, and
  source-to-PLE mapping records;
- API-based Question import and Course assembly using generated identities and readback;
- source inventories for all six requested Courses;
- the settled distinction between complete Biochemistry/Genetics, partial Molecular Biology,
  Biostatistics, and Laboratory, and unclassified Biotechnology; and
- the user-selected Course color families and only the already proposed Theme matches.

## Evidence used

- [HUMAN_GUIDANCE.md](../../HUMAN_GUIDANCE.md) and
  [FALL_2026_PILOT.md](../../FALL_2026_PILOT.md) are the product and pilot authorities.
- [PILOT_CONTENT.md](../../PILOT_CONTENT.md) documents the narrow eight-Question Chapter 1
  teaching subset. It is not the full Course inventory.
- Local topic indexes under `OTHER_REPOS/biology-problems-website/site_docs/` supplied every
  listed source topic and description.
- `crates/server/src/authoring.rs` and `crates/server/src/blueprint_course.rs`, plus
  `crates/question_model/src/blueprint_course/`, supplied the current Draft and Blueprint route,
  identifier, ordering, revision, and idempotency evidence.

## Settled requirements not yet delivered

- **Use normal authenticated APIs for production import and system tests.** Current pilot tooling
  already receives generated Question, Blueprint Course, and Assessment IDs, but it calls
  database-backed stores directly. The production importer must call the running HTTP API and read
  its results back. This is an implementation gap, not a question about hard-coded IDs.
- **Preserve algorithmic source.** Each canonical PG or PGML source remains one WeBWorK Published
  Question. Each suitable static source becomes a reviewed native JSON Published Question. The
  current Draft HTTP route accepts only PLE Question JSON; implementing the missing authenticated
  PG/PGML import operation is engineering work required for a complete production importer.
- **Use PLE-generated identities.** The Course API provides `POST /api/course-blueprints`,
  `PUT`/`GET` by Blueprint Course ID, revision readback, and publication. It accepts an initial
  complete tree and generates Course, Module, and Assessment IDs. This is enough to assemble a
  Course after Question import, but only after the Question import API supports every selected
  source format.

## Deferred topics

- **Question-license support.** NC and ND support is deferred in the Question specifications. The
  known Chapter 1 pilot material is CC BY, so this does not currently block that subset.
- **QTI conversion coverage.** QTI remains an optional interchange path for suitable static
  Questions. Reuse `qti-package-maker-rs` when its actual reader supports the input. Do not add a
  second converter or present QTI as the PLE runtime format.
- **Theme palette choice.** Molecular Biology is magenta and Laboratory is teal-green by settled
  user preference, but exact Theme matches are deferred. Biochemistry's Tundra and the proposed
  Ocean, Grassland, and Magma matches require Light/Dark visual review.

## Central question log

The central [question_specs_open_questions.md](../decisions/question_specs_open_questions.md)
contains the only active later-grill entries. Its priorities are the correct review order:

- **Q01:** Repeat-import source changes affect Question identity and Course mappings.
- **Q02:** Course Assessment grouping and Pool choices determine usable base-Course content.
- **Q05:** Blueprint color preference needs a settled relationship to Course Instance Theme.
- **Q08:** Partial Molecular Biology, Biostatistics, and Laboratory source material needs a
  release-scope decision.
- **Q09:** Biotechnology source completeness needs classification before it becomes a Course claim.

The missing PG/PGML HTTP operation is an implementation gap. It is not a question for the user to
design at the route level. The importer needs an authenticated operation that preserves canonical
source and supplies the backend's required registered source reference.
