# Ribbon visual polish

This is historical evidence. The later [file-tab correction](ribbon_file_tabs_2026_10_03.md)
supersedes its Tier 1 shape and surface interpretation and refreshes the shared screenshots.

The two-tier structure was substantially improved by the prior implementation.
A further rendered review found three remaining sources of visual competition:
inactive folders became brighter in dark mode, an old desktop breakpoint squeezed
folder labels, and the full-width Tier 2 divider used the same strong color as selection.

The tier owners now shade resting folders from the Ribbon plane itself, use consistent
desktop padding, and keep a quiet content divider beneath the strong selected-tab edge.
Folder corners are slightly softer, and resting labels use less weight. Selected shapes,
row reservations, breadcrumb hierarchy, and the content connection remain stable.
Folder shoulders inherit the parent paint transition and reduced-motion behavior.

## Rendered review

Controlled comparisons use the actual application shell with synthetic Course data and
production theme tokens. The resulting dark view gives the open folder clear emphasis;
the light view has more room around labels and a quieter horizontal boundary.

- [Desktop light](ribbon_polish_2026_10_03/final_1280_light.png)
- [Desktop dark](ribbon_polish_2026_10_03/final_1280_dark.png)
- [Phone light](ribbon_polish_2026_10_03/final_393_light.png)
- [Phone dark](ribbon_polish_2026_10_03/final_393_dark.png)

## Verification

- Ribbon density: passed across all 15 themes in both display modes, including text contrast,
  stable selection geometry, forced colors, and reduced motion.
- Responsive evidence and current-source routed-shell evidence: passed.
- Fast checks: passed, including Rust, 542 frontend tests, and 10,124 Python checks.
  The initial run exceeded the build-cache budget; completed service-test artifacts were
  cleared and the complete offline gate then passed.
- The real-app capture completed and published all 246 screenshots. Final visual review included
  the Instructor roster and dark-theme Course page, plus Student phone Course navigation.
  These confirm the quieter resting folders, clearer selected outline, content-tab connection,
  and retained breadcrumb ancestry. The Live Demo remains running for human inspection.
- Instructor comparison-strip regeneration and static corpus verification passed. Final
  documentation, ASCII, and line-limit checks passed 4,482 tests; `git diff --check` passed.

Real-app examples:

- [Instructor light](../../screenshots/instructor/courses-students-course_roster_active.webp)
- [Instructor dark](../../screenshots/instructor/theme_sample-tundra-dark.webp)
- [Student phone](../../screenshots/student/phone/course_landing.webp)

This polish changes the two tier stylesheets. The previous full database and service acceptance
results are recorded in [ribbon_review_2026_10_02.md](ribbon_review_2026_10_02.md).
