# Ribbon and breadcrumb review

Work follows [improve-ribbon-and-crumb.md](../../archive/improve-ribbon-and-crumb.md).
Completed October 3, 2026: focused checks, required live acceptance stages, and the clean
246-image screenshot replay pass. The final rendered navigation was reviewed against the
clarified visual contract; this records agent review, not human aesthetic signoff.
The later [visual polish review](ribbon_polish_2026_10_03.md) records the subsequent surface and
spacing refinements and refreshes the shared screenshot corpus.

## Corrections to the draft

- Earlier guidance and tests incorrectly joined Tier 1 to Tier 2. The clarified contract uses
  two distinct relationships: Tier 1 folders float within the Ribbon, while Tier 2 rectangular
  tabs sit directly on the content edge. Every folder has a small shoulder and space around it.
  The selected folder shares the Ribbon plane with a defined edge and stronger label; shaded
  siblings recede. The selected Tier 2 shares the content surface and opens into it. Resting
  labels use readable muted text. Selection preserves each tab's size and row reservation.
- Tier 1 and Tier 2 styles live in separate small stylesheets. Shared row, overflow, focus,
  touch, and breadcrumb behavior remain in the existing shell. There are no runtime style modes.
- Breadcrumbs previously built an old route trail, then removed ancestors by matching labels.
  That could also remove an authored Course named `Courses`. One shared implementation now
  composes Home, selected Ribbon tiers, and object/page ancestry directly. Only adjacent
  identical names collapse; sharing a URL does not remove a level.
- Breadcrumb links now preserve the history state required by existing Attempt shortcuts.
  This repairs links within the shared breadcrumb model; it adds no Student navigation model.
- At 320px the Instructor badge left too little room for the selected folder and overflow cue.
  Phone badges use compact typography while retaining the full role name.
- The screenshot invitation setup omitted the visible roster-removal confirmation. The setup
  now confirms that action before waiting for the invitation row to disappear.
- The Instructor crop script now runs in its own directory and excludes its prior stacked
  output, so rerunning it does not recursively include that output.
- The Courses tab and breadcrumb consistently link directly to the role home, including from
  Course routes. Instructor navigation no longer detours through the root role resolver.

## Instructor ancestry audit

| Surface | Required ancestry | Evidence |
| --- | --- | --- |
| Active Course, Students, Appearance | Home / Courses / My Active Courses / Course / section | Real app direct-load checks and model tests |
| Inactive Course, Students, Appearance | Home / Courses / My Inactive Courses / Course / section | Real app direct-load checks, collection link navigation, and model tests |
| Owned Blueprint detail | Home / Courses / My Blueprint Courses / Blueprint | Existing authorized owner label publication and model tests |
| Public Blueprint detail | Home / Courses / Search Public Blueprint Courses / Blueprint | Existing authorized read-access publication and model tests |
| Blueprint proposal detail | Home / Courses / My Blueprint Courses / My Change Proposals / Change Proposal | Model test verifies both collection links |
| Published Question and Draft editor | Home / Questions / applicable Tier 2 / Question | Route declarations and model tests |
| Assessment workspace and Gradebook | Home / Assessments / Course / Assessment or section | Plan explicitly assigns these routes to Assessments, whose Tier 2 has no Course parent |

Course lifecycle comes from the stored database value through the Rust summary and generated
API contract to the strict browser decoder. The running database's summary function was inspected
and included `course_lifecycle_state`. Unresolved lifecycle shows neither an assumed Tier 2
selection nor an assumed collection crumb.

Both roles use normal breadcrumbs and the same rules. Students' Courses Tier 2 is their current
enrollments. Active/inactive collections belong to Instructors.

## Rendered evidence

The css-creative-expert review uses a visual-craft and layout contract: Tier 1 folders float on
the Ribbon plane, selected Tier 2 merges into content, selection reserves its geometry,
and narrow rows scroll without widening the page. The targets are all three roles,
320-1280px viewports, light/dark themes, and unresolved/resolved ancestry.

The cascade is component-owned: `app_ribbon.tsx` imports `app_ribbon.css`,
`ribbon_tier_one.css`, `ribbon_tier_two.css`, then `app_ribbon_density.css`. Tier-local size and
surface tokens keep future visual changes independent. Shared shell aliases provide surface
and spacing tokens to breadcrumbs. The density sheet owns common phone, coarse-pointer,
forced-color, and reduced-motion overrides. The Tier 2 contract follows the user's
[connected-tab reference](https://uicookies.com/wp-content/uploads/2019/03/tabs.jpg).
Screenshot inspection and computed-style checks verify the shared content color, open bottom,
defined top edge, readable muted labels, and unchanged geometry. Both display modes are measured
for every supported theme. Tier 1 is checked for surrounding space and its own Ribbon surface.

The final current-source routed-shell previews use synthetic Course data and the production
theme tokens. They verify paint and layout, separately from real-service capture and preference
persistence:

- [Desktop light](ribbon_review_2026_10_02/final_1280_light.png)
- [Desktop dark](ribbon_review_2026_10_02/final_1280_dark.png)
- [Phone light](ribbon_review_2026_10_02/final_393_light.png)
- [Phone dark](ribbon_review_2026_10_02/final_393_dark.png)

The inactive proof used a new empty Course created through the Instructor UI. The existing
retention function advanced that review-owned Course to its inactive cutoff. No Students were
enrolled in it. Direct loads verified all three Instructor Course routes against the real API.
These ancestry captures precede the final connected-tab styling above.

- [inactive_1280_light.png](ribbon_review_2026_10_02/inactive_1280_light.png)
- [inactive_1280_dark.png](ribbon_review_2026_10_02/inactive_1280_dark.png)
- [inactive_320_light.png](ribbon_review_2026_10_02/inactive_320_light.png)
- [inactive_393_dark.png](ribbon_review_2026_10_02/inactive_393_dark.png)
- [review.json](ribbon_review_2026_10_02/review.json)

The earlier ancestry review confirms the inactive ancestor,
compact Course name on narrow screens, and stable row spacing. At 320px the breadcrumb trail
scrolls; keyboard focus reveals each ancestor and the current page. The selected Tier 2 label
stays visible after the row's reveal animation settles. Initial capture during motion produced
misleading clipping and contrast evidence; final captures wait for visibility and finish finite
paint transitions.

Live checks at 320, 393, 768, and 1280px in light and dark found no document horizontal overflow
and no axe violations in the Ribbon or breadcrumb region. Density evidence checks the theme
matrix, forced colors, text contrast, unchanged selection geometry, and compact controls.
Responsive evidence passed. These results cover the navigation regions, not every page control.

The final clean real-service corpus supersedes the interrupted earlier replays. Review included
the Instructor roster, Blueprint detail, deep Assessment Properties, light/dark theme samples,
Student phone Course navigation, Sysadmin navigation, and the rebuilt Instructor comparison strip.
Selected folders remain separate from the second row; selected rectangular tabs open into the
content surface. Instructor collection ancestry and the shallower Student trail remain visible.

- [Instructor Students](../../screenshots/instructor/courses-students-course_roster_active.webp)
- [Instructor dark theme](../../screenshots/instructor/theme_sample-tundra-dark.webp)
- [Student phone](../../screenshots/student/phone/course_landing.webp)
- [Screenshot atlas](../../SCREENSHOT_ATLAS.md)

## Verification status

- Focused Ribbon and breadcrumb model tests: 30 passed.
- Offline aggregate: passed, including Rust workspace checks, 542 frontend tests, and
  10,122 Python tests. Fast checks also passed.
- After archiving the plan, documentation links, guidance format, ASCII, and source/shell
  line-limit checks passed 4,480 tests. `git diff --check` passed.
- Ribbon density, responsive, and routed-shell browser evidence: passed. The shell check verifies
  unresolved/resolved Instructor ancestry, stable content placement, narrow layout, keyboard
  traversal, and theme switching. Obsolete loading, theme-hook, palette, and flattened-trail
  expectations were repaired against the current owning contracts.
- Component style-ownership evidence passed with the same production theme supplied to both
  comparison surfaces. Its former hard-coded/missing-token comparison was not a valid cascade
  comparison. Page-local navigation retains its presentation after the tier-style split.
- The final `capture_screenshots.sh --fresh` run exited zero and published all 246 images.
  Instructor crop stacking completed, followed by successful static corpus verification.
  The workflow stopped its owned Live Demo after capture. Static verification establishes
  corpus consistency; browser behavior and accessibility evidence are reported separately above.
- The completed temporary browser probes were removed after retaining their evidence.
- Full aggregate passed the disposable database baseline and installation replay. Although it
  exited zero, the cross-store child printed a readonly `REPO_ROOT` collision while sourcing
  the repository environment, before running its tests. Its local variable is now distinct;
  its full isolated rerun exited zero with both `minio_object_store_conforms` and
  `course_banner_saga_is_durable_authorized_and_cross_store` passing (one test each). All
  required acceptance stages now have passing evidence. The screenshot-owned Live Demo was
  cleaned up before this gate.
- The live aggregate passed the Course lifecycle database test, then found two audit assertions
  running as the restricted migrator. The audit table is append-only with no read policy.
  Assertions now select its owner and temporarily remove forced row filtering only while
  inspecting audit effects; FORCE is restored before further API calls, within the rolled-back
  test transaction. Application grants and production policies are unchanged. The corrected
  oracle and the complete disposable database baseline passed against PostgreSQL.
- The following connected Unrelease case reused a fixed Student public ID already issued by
  a preceding case. It now uses the existing Account ID-minting placeholder.
- Blueprint promotion exposed an invalid fixed Sysadmin Account ID; that fixture now uses the
  existing ID-minting placeholder as well. Both repaired cases pass in the full database baseline.
