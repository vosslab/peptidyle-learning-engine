# Page and decoder consolidation

## Outcome

Group related browser pages under stable feature areas and make DTO decoders reuse the canonical
domain decoders. Route behavior and feature ownership stay unchanged, except the shared Question
statistics panel belongs to `question_library`.

## Baseline and implementation

- Baseline: 79 page files lived directly in `src/pages/`.
- 75 page files were moved with `git mv` into focused directories: `administration`,
  `assessment_attempt`, `assessment_overview`, `assessment_templates`, `authentication`,
  `blueprint_course`, `course`, `profile`, `question_drafts`, `question_library`, and
  `student_coursework`. The Course roster and gradebook helper is named
  `course_roster_gradebook_fields.ts`; Watch notifications stay with the Question Library route.
- Three cross-cutting page compositions remain at the `pages/` root. No forwarding wrappers were
  added, and route behavior stayed intact. The unused Student Work inspection stylesheet was
  removed after repository-wide consumer checks found no reference.
- Question statistics presentation is now shared feature code in
  `src/features/question_library/question_statistics_panel.tsx` and its stylesheet.
- Three DTO decoders reuse canonical `QuestionType`, positive-integer, and Bloom enum decoders.
  This removes duplicate validation rules while retaining DTO-specific shape checks.

## Milestones

1. **Page grouping and import recovery:** accepted. The remaining seven Library/Course files were
   grouped and the helper renamed. Import recovery repaired 3,240 static import specifiers across
   470 files while preserving all other source bytes; TypeScript and the application build passed.
2. **Decoder consolidation:** accepted. Canonical QuestionType, positive-integer, and Bloom enum
   behavior was confirmed by SPEC review.
3. **Quality and integration:** accepted. Fresh QUALITY and final integration reviews passed.
   Integration confirmed current page ownership, routes, decoder values, and registry references.

## Acceptance record

- Markdown link and guidance-format gates passed: 582 tests. The first full fast run found stale
  path labels in the generated Ribbon destination ledger; all 21 labels now match their current
  source targets.
- Full fast checks passed: 9,427 Python tests plus Rust, TypeScript, lint, and format checks.
  Browser build passed separately. No fresh browser acceptance is claimed.
- SPEC accepted after the manager corrected four stale qualified pointers to
  `BlueprintCourseClient.listBlueprintCourses` and `ApiClient.searchLibraryObjects`. QUALITY
  accepted after the clarification; no source fix remained. Final integration accepted with no
  blocking gap.
