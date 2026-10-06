## 2026-10-03

### Behavior or Interface Changes

- Applied the actionable specialist review findings: widened Tier 1 shoulder
  curves, softened its upper corners, and simplified the content sheet edge.
  Ribbon overflow chevrons now scroll their own row by pointer or keyboard
  without changing selection. Blueprint detail puts primary teaching actions
  and reusable structure ahead of stewardship and classification; return and
  classification actions use compact, aligned placement.

- Strengthened active/inactive surface separation in both Ribbon tiers across
  all themes and display modes. A small shared surface stylesheet keeps the
  value step adjustable while preserving curved Tier 1 and square Tier 2 joins.
  Inactive labels retain their contrast; keyboard focus now stays clear against
  both surfaces. Human Guidance records surface distinction separately from text
  accessibility.

- Refined the Ribbon and content as one composition: broader borderless Tier 1
  shoulders, square open Tier 2 joins, better label spacing, and tinted bar
  surfaces. Added a quiet outer content rail, removed the competing breadcrumb
  rule, and softened Course section dividers. Connected surfaces repaint
  together during theme changes. Navigation and row reservations stay stable.
- Real-page review exposed an empty media column squeezing text-only records.
  The shared grid now creates a media track only for an actual image; facts wrap
  between labels and values. Temporary browser evidence verifies the row width.
  Primary record links/buttons now reuse the shared primary-action treatment
  instead of a color-only local rule.

- Corrected Tier 1 to a continuous colored Ribbon with one curved selected
  file-folder tab flowing into its lower strip. Unselected choices stay
  integrated into the bar. Tier 2 flows into content with square corners.
  Both tier treatments remain modular and selection keeps stable geometry.

- Ribbon polish shades inactive folders consistently in light and dark,
  gives desktop folder labels more space, softens their corners, and quiets
  the Tier 2 divider. Selected tabs retain their stronger labels and edges.
  Both tier treatments stay independently owned by their small stylesheets.

### Developer Tests and Notes

- Public Blueprint search results show the owner's verified display name as
  Author. Institution stays off that row because one global installation has no
  institution boundaries.
- `devel/human_guidance_checklist.py --evidence` checks the assembled checklist.
  Every verified bullet needs a locating source, test, or runtime symbol, and a
  runtime-required bullet also needs test or runtime evidence.
- The implementation checklist now matches Human Guidance at 1065 verified
  bullets, 27 open bullets, and 54 items that are not product behavior, across
  1146 bullets. Blob `3da3a20cb34ed9c01346835de5b55a6da92a98e8`. The duplicate
  breadcrumb probe left the permanent suite; the ribbon contract test keeps
  that behavior. A later re-audit found no added, removed, or changed bullets.
  All nine part gates exited 0, and `--diff` and `--consistency` exited 0 twice.
  The role-badge open row now states both current readings: badge left of the
  logo, or logo then badge, with phones omitting the product name.

- Review fixes pass existing theme/density, responsive, style-ownership,
  TypeScript, and focused lint gates. Responsive coverage now checks actual
  pointer/keyboard overflow navigation. One-time live checks verify narrow
  Instructor pages and Blueprint editor access. Regenerated all 246 screenshots
  directly and rebuilt the Instructor crop stack after capture completion. See
  [review fixes](active_plans/reports/ribbon_review_fixes_2026_10_03.md).

- Added separate skill-guided Instructor reviews: an
  [HCI review](active_plans/reports/ribbon_hci_review_2026_10_03.md)
  covering task orientation and navigation, and a
  [CSS creative review](active_plans/reports/ribbon_css_creative_review_2026_10_03.md)
  covering composition, tab silhouettes, and surface relationships. Each
  distinguishes screenshot evidence from behavior requiring runtime validation
  and gives concrete implementation and acceptance recommendations.

- Two independent image reviewers assessed the current Instructor screenshot
  corpus, all 30 theme/mode samples, representative full pages, and controlled
  narrow-screen examples. Their separate [review A](active_plans/reports/ribbon_independent_image_review_a_2026_10_03.md)
  and [review B](active_plans/reports/ribbon_independent_image_review_b_2026_10_03.md)
  preserve their differing aesthetic judgments. This pass changes review
  documentation only.

- Reviewed all 30 theme/mode combinations and narrow/full-page compositions for
  the Ribbon state contrast change. Existing density, text contrast, focus,
  reduced-motion, responsive, and style-ownership checks pass. The existing
  all-theme check now protects meaningful surface separation instead of mere
  color inequality. Refreshed all 246 real-app screenshots directly in
  `docs/screenshots` and rebuilt the 66-image Instructor crop stack. See the
  [contrast review](active_plans/reports/ribbon_state_contrast_2026_10_03.md).

- Regenerated all 246 screenshots into staging from a fresh Live Demo and ran
  the existing Instructor crop-stack script there: 66 images, 1280 by 11748
  pixels. Capture paths match the manifest. The reused demo had altered Student
  score state; a fresh seed passed all 27 scenarios. This first run used staging
  because automatic approval review enforced the earlier publishing boundary.
- Following the user's destination correction, regenerated all 246 images directly
  into `docs/screenshots` and rebuilt the Instructor crop stack there after all
  66 Instructor captures completed. Manifest, receipt, and galleries validate.
  Screenshot discovery now ignores regular Finder `.DS_Store` files at every
  corpus directory level; other unexpected entries remain rejected. Existing
  screenshot contract tests pass. No Git actions were taken.

- Surface refinement passes all-theme density/contrast, responsive, routed-shell,
  style-ownership, TypeScript, and focused lint checks. Local design comparisons
  are in the [surface review](active_plans/reports/ribbon_surface_refinement_2026_10_03.md).
  Screenshots for this pass use staging only; publishing and Git are out of scope.

- Replaced the earlier miniature-folder assertions with the clarified bar/tab
  surface contract. All-theme contrast, focus, forced colors, reduced motion,
  responsive, routed-shell, and offline checks pass. See the
  [file-tab review](active_plans/reports/ribbon_file_tabs_2026_10_03.md).
- Refreshed all 246 real-app captures on a fresh demo. Final publication resumed
  after removing Finder metadata from the corpus root; receipt/gallery checks pass.

- Rotated October 1 and September 30 history into
  [CHANGELOG-2026-10a.md](CHANGELOG-2026-10a.md), retaining the latest two dates.
- All-theme light/dark density, responsive, and routed-shell checks pass for
  the polish. Rendered comparisons and final capture evidence are recorded in
  [ribbon_polish_2026_10_03.md](active_plans/reports/ribbon_polish_2026_10_03.md).

## 2026-10-02

### Behavior or Interface Changes

- Ribbon tiers use separate small stylesheets: Tier 1 folders float on the
  Ribbon plane, and Tier 2 rectangular tabs sit on the content edge. The open
  folder shares its surrounding surface, with shaded siblings; the selected
  Tier 2 tab joins content. Selection preserves control and row geometry.
  Phone role badges leave room for the selected folder.
- One breadcrumb implementation composes the selected Ribbon hierarchy and
  object ancestry directly. Instructor Course descendants retain My Active
  Courses or My Inactive Courses from stored lifecycle. Authored labels are
  preserved, and Attempt links carry their required navigation state.

### Developer Tests and Notes

- Ribbon and breadcrumb model checks passed 30 tests. Live Instructor checks
  verified active and inactive Course, Students, and Appearance routes, plus
  keyboard breadcrumb reachability and light/dark navigation accessibility at
  320, 393, 768, and 1280px. Density, responsive, shell, and style-ownership
  evidence passed. The clean 246-image corpus and required acceptance stages
  passed; final review and test-fixture repairs are recorded in
  [ribbon_review_2026_10_02.md](active_plans/reports/ribbon_review_2026_10_02.md).
- Screenshot invitation setup confirms roster revocation through its existing
  modal. Instructor crop stacking runs in its own directory and excludes its
  previous output. Submitted-Attempt captures select the latest Attempt when
  a synthetic account has history from earlier replays.
- Database support-repair assertions inspect append-only audit effects under
  their table owner inside the rolled-back test transaction, restoring forced
  row filtering before further API calls. Production access remains unchanged.
- The connected Unrelease fixture mints its Student Account ID so preceding
  acceptance cases cannot collide with the permanent public-ID registry.
- The Blueprint promotion fixture uses the existing Account ID-minting
  placeholder instead of an invalid fixed Sysadmin ID.
- The cross-store acceptance script uses its own repository-root variable so
  loading `source_me.sh` reaches the PostgreSQL/MinIO tests.
- Screenshot publication keeps `docs/screenshots/instructor/crop-stack.sh` and
  `stacked-screenshot.webp` beside the Instructor captures. Those review files
  are not corpus images, and a file in another role folder is still rejected.
- Ribbon Tier 1 tabs keep one folder box, and the selected tab opens into the
  Tier 2 surface. Tier 2 controls are smaller separated tabs. A phone-width
  Tier 2 row uses the full width and scrolls, with an edge fade so a label is
  not sliced at the scrollport.
- Ribbon Tier 1 uses folder tabs and Tier 2 uses quieter compact tabs. A
  descendant page keeps the ancestor Tier 1 tab and Tier 2 control selected.
  Breadcrumbs follow that hierarchy. Adjacent crumbs collapse only when they
  show the same name. Course summary reads now include `course_lifecycle_state`
  so active and inactive Courses select the matching Tier 2 parent.
- Human Guidance closeout records 1055 verified bullets, 28 open bullets, and 54
  items that are not product behavior, across 1137 bullets. The Human Guidance
  blob is `d0614eba06400c7ae00dfcdb73ca0a9e1ed03efc`. The compliance plan now
  lives at `docs/archive/human_guidance_implementation_compliance_plan.md`. The
  checklist stays in `docs/active_plans/audits/`.
- Breadcrumb trails keep the Course and Assessment steps. `presentBreadcrumbs`
  adds the selected Ribbon tier when a descendant page omitted that destination,
  and it drops an adjacent crumb that opens the same href.
- `devel/run_playwright_tests.sh` exited 0 on the canonical Live Demo before the
  breadcrumb ancestor change. The four registered scenarios passed, and so did
  Assessment release, Assessment Attempt including WeBWorK reload, Instructor
  Accounts, support repair, invitation export, and the seeded course browser.
- Live Demo invitation creation reads `courseInstance.id`. The seeded course
  browser checks the released Assessment record, the Gradebook table, roster
  names, and the Unit Review label. The course-seed journey passed on the
  running Live Demo.
- The Live Demo support journey reads `support_repair_capability_id` and stamps
  the copied roster profile at the transaction clock so `updated_at` stays at
  or after `created_at`. The support journey passed on the running Live Demo.
- The signed-in top bar is logo, product name, role badge, then Tier 1.
  Light/Dark and the Profile image stay at the far end. Phones omit the product
  name and keep logo, then badge. The Profile menu offers Profile settings and
  Sign Out. A Ribbon contract test checks that order.
- A verbatim Human Guidance relative link in the implementation checklist resolves
  from docs/. The same link outside those copies still resolves from the file that
  contains it. Markdown link and ASCII checks passed.
- Offline fast checks passed. The empty recorded-JavaScript CDN list is consulted
  by author-script validation. `cargo clean` brought target/ under 10 GiB.
- Canonical Live Demo started. The four registered Playwright scenarios passed.
  The Assessment Attempt journey still fails while reloading a WeBWorK saved
  response. The Live Demo stack was left running.
- The Human Guidance checklist records a product question or the unlocked Sysadmin
  Ribbon layout on every open row. Implementation compliance reports summarize 1041
  verified bullets, 27 open bullets, and 54 items that are not product behavior.
  No Live Demo stack was started.
- Frequent Instructor teaching tasks stay in the Courses, Questions, and Assessments
  groups. Those tasks are linked. Teaching Operations, Blueprint Updates, Course Setup,
  and Grade Settings stay future destinations and are not usable links. No Live Demo
  stack was started.
- Table shape and clocks follow the schema style rules. Support repair resource class is
  the enum ple_data.support_repair_resource_class instead of a repeated text check.
  Source rules report no findings. A disposable database installed the schema and was
  removed. No Live Demo stack was started.
- Large Question Library collections stay scannable rows. Each row shows the title, Question
  ID, discipline, and author together. Search, filters, and title sort run on 13,000
  Questions, and excluding one term narrows 12,000 to 6,000. No Live Demo stack was started.
- Expert Question Library syntax narrows a large library. `enzyme -inhibitor` keeps the
  enzyme term and excludes inhibitor. A disposable database held 12,000 matching Questions
  and the exclusion left 6,000 within 15 seconds. The database was removed. No Live Demo
  stack was started.
- Profile and Sign out stay together in the Profile menu. Ribbon navigation does not
  repeat Sign out for Student, Instructor, or Sysadmin. A headless browser opened the
  shipped Ribbon and checked each role. No Live Demo stack was started.
- A fresh install provisions the Live Demo and the Genetics example by default. The
  migrator command created Course BCHM 301 and public Blueprint Genetics, Fall Genetics,
  with 9 topics and 41 Questions. The Live Demo launcher was not started. Disposable
  Postgres, object storage, and the API were removed.
- Public identities reject an internal UUID. Account, Course, Assessment, Blueprint, and
  Question parsers refuse a UUID. Profile hides one. PostgreSQL rejects a UUID Account ID
  and keeps UABCDEFGM. A disposable database ran the proof and was removed. No Live Demo
  stack was started.
- Object storage, its hash, and the retention log keep the canonical public ID. Question
  ABCD-XEFG is the object path and JSON. Course CIABCDEFGS is the record path and the
  failure log. A different canonical ID changes the hash. No Live Demo stack was started.
- Parsing, JSON, routes, and PostgreSQL keep a canonical public ID unchanged. Account
  UABCDEFGM was stored and reread as that string. A mint placeholder became a different
  canonical Account ID. Lowercase Account and Question values were rejected. A disposable
  database ran the proof and was removed. No Live Demo stack was started.
- Generators and JSON store and transmit only the canonical public ID. Profile displays and
  copies Account ID U0000035E and hides a lowercase value. A Rust test generated the five
  typed IDs and rejected lowercase and bad-checksum Account JSON. A browser test copied only
  the canonical Account ID. No Live Demo stack was started.
- PLE and WeBWorK issue and grade through one Question Backend interface. Each adapter keeps
  its source and renderer details. A native Question graded at credit 1 and a WeBWorK Question
  graded at credit 0.5 through that interface. Deferred iMathAS and H5P stay outside it. No
  Live Demo stack was started.
- Question Library stewardship keeps a private Watch and delivers improvement threads and impact
  notices to current watchers. A new thread records the same timestamp for creation and update,
  and the discussion read uses the stored thread id. A disposable database executed the Watch
  inbox contract and was removed. No Live Demo stack was started.
- Public ID routes check canonical syntax and the embedded checksum before a database lookup.
  A bad checksum for a Question, Question Pool, Course Instance, Assessment, Blueprint Course,
  or Account is concealed, and the Store is not called. No Live Demo stack was started.
- Question Library cleanup of a large import uses shipped search, filters, title sort, and one
  bulk metadata command. A disposable database loaded 13,000 imported Questions, narrowed them
  by text, tag, Question type, and license, sorted by title, and retagged the first 1,000. The
  remaining imported Questions kept their tag. The database was removed. No Live Demo stack was
  started.
- The Question Library PostgreSQL acceptance fixture mints Account IDs through the account
  placeholder. `question_library_search_filters_and_pages_in_postgresql` passed on a disposable
  database, including both Bloom dimension counts. The database was removed. No Live Demo stack
  was started.
- Question Library search keeps both Bloom dimensions useful. Each filter narrows the
  whole Library, and the report lists every guide value. The Library browser test selects
  Remember and Factual Knowledge and shows both teaching meanings. No Live Demo stack was
  started.
- Library Bloom search shows the guide's teaching meaning for each Cognitive Process and
  Knowledge Dimension. The database accepts only those six and four values. AI
  assignment stays deferred and does not block publication. A disposable database
  executed the vocabulary contract and was removed. No Live Demo stack was started.
- Reference stays the name for an indirect locator. Current object identities remain Ids
  and Tuples. A WeBWorK source location remains a Binding. The Question and Blueprint
  Tuple tests refuse a legacy reference wrapper. No Reference type was added. No Live
  Demo stack was started.
- An Instructor can issue a one-hour content support capability for one Course
  Assessment. The Sysadmin read returns that Assessment's identity, type, title, and
  status. It omits instructions, Questions, answers, and Student Work. Issuer
  deactivation conceals the read, and the Sysadmin does not become a Course member.
  A disposable database executed the contract and was removed. No Live Demo stack
  was started.
- An Instructor can issue a one-hour Course support capability to a Sysadmin. The read
  returns Course identity, term, activity, retention, and Instructor display names. It
  records the use, rechecks the original Instructor, and does not create Course
  membership. Content repair stays rejected. A disposable database executed the contract
  and was removed. No Live Demo stack was started.
