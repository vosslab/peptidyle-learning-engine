# Ribbon file-tab correction

The user clarified that Tier 1 is a continuous colored bar with one selected
file-folder tab emerging from it. The previous raised shoulder on each enclosed
rectangle depicted miniature folders. That interpretation is superseded.

Tier 1 now paints a single compact selected silhouette with rounded top corners
and outward-curving feet. Its face flows into a narrow Ribbon strip separating
it from Tier 2. Resting choices have no individual enclosure. The bar uses the
theme secondary surface; the selected face and strip use the ordinary surface.
This preserves the surface relationship across light and dark without fixing
selection to a brightness direction. Tier 2 retains its content connection and
now has strictly square corners.

The [curved-tab reference](https://www.geeksforgeeks.org/javascript/how-to-make-curved-active-tab-in-navigation-menu-using-html-css-javascript/)
informed the silhouette and continuous-bar relationship. Its sample styling and
JavaScript are not used. Each tier remains owned by its small stylesheet; shared
navigation, row reservations, and breadcrumb ancestry remain intact.

## Rendered review

Fresh current-source application-shell renders use synthetic Instructor Course
data and production theme tokens. They cover 320, 393, 768, and 1280px in both
modes. These are visual evidence, not Live Demo preference-persistence tests.

- [Desktop light](ribbon_file_tabs_2026_10_03/final_1280_light.png)
- [Desktop dark](ribbon_file_tabs_2026_10_03/final_1280_dark.png)
- [Phone light](ribbon_file_tabs_2026_10_03/final_393_light.png)
- [Phone dark](ribbon_file_tabs_2026_10_03/final_393_dark.png)

## Verification

- Final Ribbon density evidence passed across all 15 themes in both modes,
  with navigation text contrast at least 5.5:1, stable selection geometry,
  visible keyboard focus, forced-colors selection, and reduced motion.
- Final responsive evidence and routed application-shell evidence passed.
  Phone labels retain internal space around the selected curved edge.
- Offline aggregate checks passed, including Rust/Wasm, frontend checks,
  and 10,125 Python tests. Documentation and source-hygiene checks passed
  4,483 tests with nine existing near-limit advisories.
- The first capture on the reused demo stopped at an unreleased-score
  checkpoint. The fresh owned-demo replay passed all 246 captures, including
  that checkpoint. A Finder `.DS_Store` at the corpus root blocked final
  publication. After removing it, the existing publication function resumed
  from the completed manifest and images; receipt/gallery generation and
  static corpus verification passed. The wrapper itself exited nonzero at
  the metadata boundary, so this is capture plus resumed-publication evidence.
- Final real-app review covered the Instructor roster, Tundra dark Course,
  and Student phone Course. The Live Demo remains running for inspection.

Real-app examples:

- [Instructor light](../../screenshots/instructor/courses-students-course_roster_active.webp)
- [Instructor dark](../../screenshots/instructor/theme_sample-tundra-dark.webp)
- [Student phone](../../screenshots/student/phone/course_landing.webp)

The full service acceptance from the earlier Ribbon work remains recorded in
[ribbon_review_2026_10_02.md](ribbon_review_2026_10_02.md); this CSS-only correction
uses offline, focused browser, and live screenshot gates.
