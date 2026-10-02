# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-01

### Fixes and Maintenance

- Reflowed the 2026-09-29 changelog day to about 100 characters so that day stays under 1000
  lines, then archived it in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md). The entries stay on
  that date. [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md) holds 2026-09-28, because the two days
  together would pass 1000 lines.
- Repaired the October 1 audit findings across Pool concurrency/import/provenance, shared-statistic
  privacy, atomic Discipline fulfillment, navigation identity, answer validation, stale
  classification context, and Genetics content. Removed the no-op Watch materializer, unused
  StudentWork aggregate, brittle guidance and SSR label gates, duplicate schema comments, and
  obsolete adapter behavior. Temporary proofs were removed after validation. Focused PostgreSQL,
  browser, Node, and Rust proofs passed. The final getter-backed save-status fix passed independent
  review, fresh runtime proof, and `check_codebase.sh`; typecheck, lint, format, and all 531 Node
  tests passed after removing two brittle SSR label/status tests and their helper while retaining
  13 controller tests. Final verification passed in separate receipts: Rust workspace checks,
  all-feature Clippy, tests, doctests, and WebAssembly checks; the TypeScript lane and all 531 Node
  tests (`/private/tmp/ple_repair_fast_final.log`); and full pytest with 10,058 passed, nine
  warnings, in 8.29 seconds (`/private/tmp/ple_fast_pytest.log`). This was not one aggregate
  command. The earlier link and generated-target cache failures were fixed, with the
  target directory brought to 1.2 GB within its limit. Closeout changes stayed within existing
  tests, fixture data, helper placement, and documentation: two brittle SQL source-string gates and
  two brittle SSR label/status tests were removed, a Rust fixture initializer was corrected for
  Clippy, and existing Pool GET/PUT token assertions were aligned to tokens 4/5 without behavior
  change. All 246 fresh screenshots remain verified. The repair is ready for human review. The final
  fresh capture published all 246 images; manifest, receipt, and current-file checks passed with
  representative image review. See
  the [repair ledger](active_plans/audits/fresh_audit_repair_status_2026_10_01.md),
  [screenshot atlas](SCREENSHOT_ATLAS.md), and
  [capture receipt](screenshots/current_capture_receipt.json).

### Developer Tests and Notes

- Completed a deeper static review of 515 reference-changed paths with per-file evidence and
  limits. Confirmed Pool import source-version loss and stale classification creation context,
  refined Pool concurrency ownership, and withdrew the earlier SQL helper privilege finding.
  See the [deeper audit](active_plans/audits/human_guidance_deeper_code_review_2026_10_01.md).
  Only audit documentation changed; no tests, builds, services, or compliance gates ran.
- Completed a fresh static whole-codebase audit with 2,654 path assessments, explicit review
  depths, source hashes, preserved designs, and prioritized corrections. See the
  [audit report](active_plans/audits/fresh_codebase_audit_2026_10_01.md). No implementation edits,
  tests, builds, services, or compliance gates were performed for this review.
- An active Sysadmin has platform-administration authority over PLE. The check uses that active
  role and does not grant Course membership. A disposable database executed the contract, denied
  an ordinary Sysadmin session, and was removed. Scoped Course, Student, and content repair stays
  separate. No Live Demo stack was started.
- The Sysadmin menu and rare-task navigation stay open for Neil. Accounts, Instructors, and Courses
  are already easy to find. System configuration either needs its own destination once
  installation-wide settings exist, or the menu is complete without one. Rare tasks either stay on
  the Sysadmin home until the Ribbon layout locks, or they need a secondary Ribbon row now. Neither
  reading was implemented. No Live Demo stack was started. No PostgreSQL proof was run.
- Sysadmins manage Courses by creating a Course Instance for an Instructor who teaches it. The
  Sysadmin home form sends that Instructor, and course creation assigns the membership. Inspection
  stays separate and does not change a Course. Retention transitions stay with the background
  process. No Live Demo stack was started. No PostgreSQL proof was run.
- Sysadmins can find and inspect Courses across the installation. The list and the Course page show
  the Instructor display name, activity, and retention state. Search stays in the request body. The
  pages do not deactivate, delete, or change a Course. No Live Demo stack was started. No
  PostgreSQL proof was run.
- Instructor approval and the vocabulary primary keys stay unresolved for Neil. Approval is either
  the immutable pre-account vetting decision, with later access changed by deactivate and
  reactivate, or a distinct Account status a Sysadmin can find and change. Theme and provided
  avatar keys either stay durable text tokens or become native UUID primary keys. Neither reading
  was implemented. No Live Demo stack was started. No PostgreSQL proof was run.
- Question inspection and Draft authoring keep the teaching content central. The published Question
  prompt leads its description and metadata. The description stays flat. The Draft editor places
  the student preview and Question Library metadata beside the student-facing prompt. The check
  covers those two workflows and does not measure every authoring or inspection workflow. No Live
  Demo stack was started. No PostgreSQL proof was run.
- Assessment authoring follows the teaching task. The Course editor leads with Assessments. Create
  Assessment names the start choice without a padded box. The Question editor puts Ordered
  Assessment Entries ahead of adding Questions. Those sections keep a subtle radius without card
  padding, fill, or shadow. Property groups stay flat. The check covers those three workflows and
  does not measure every Instructor page. No Live Demo stack was started. No PostgreSQL proof was
  run.
- Question Pool review uses the same flat groups as Question Library browse, on both the Library
  page and the shared create dialog. At 1280 by 800 two Library records and two Course roster rows
  stay in view, share one font, and have no corner radius or shadow. Their action heights match.
  The review columns stay within 24 pixels. The check does not measure every page. No Live Demo
  stack was started. No PostgreSQL proof was run.
- Assessment Template and Question Library groups keep a readable gap. Choose a Template sits
  above its message, and each classification name sits above its field instead of against it. A
  Question record stacks its title, description, facts, and action. Bloom counts stay on one row
  with their names, and the Template editor stays beside the overview. At 1280 by 800 the stacked
  gaps stay within 16 pixels, and each Bloom count keeps at least 8 pixels from its name. The
  check does not measure every page. No Live Demo stack was started. No PostgreSQL proof was run.
- Assessment Template columns, the opening Question Library search, and the Library result region
  no longer use dashboard cards. The Template overview and editor share one workspace without
  fill, radius, or shadow. The opening search has no hero padding, search tips are flat text, and
  the result rows sit in a scroll region without a floating panel. At 1280 by 800 the Template
  columns stay within 24 pixels. The check does not measure every page. No Live Demo stack was
  started. No PostgreSQL proof was run.
- Question Library classification filters, Bloom filters, Bloom count lists, and the bulk-action
  strip are no longer rounded cards. Each group stays flat because it only gathers nearby fields
  or buttons. A facet choice keeps its border and radius, and the selected Tag keeps its pill. At
  1280 by 800 those groups had no border, radius, shadow, or fill. The check does not measure
  every page. No Live Demo stack was started. No PostgreSQL proof was run.
- Question Library browse groups use a heading and a bottom divider. Subjects, Tags, and Question
  Types no longer sit in separate cards. At 1280 by 800 the headings share one column, and the
  choice list inside each group is not another box. The check does not measure every page. No Live
  Demo stack was started. No PostgreSQL proof was run.
- Question Library filters stack in their own column, so the result window no longer leaves a
  spacious hole between filter groups. At 1280 by 800 those groups stay 8 to 16 pixels apart, and
  the space inside one record stays within that. The Course roster keeps its rows together and
  uses a larger gap before the import tools. The check does not measure every page. No Live Demo
  stack was started. No PostgreSQL proof was run.
- Question Library browse and the Course roster prefer record text over empty padding at 1280 by
  800. The result region has no inset padding and visible records fill it. Each visible record's
  title, description, facts, and action outweigh the row padding. The roster section ends at its
  table, and each Student cell's text outweighs its padding. Leftover viewport under the short
  roster is the end of the page. The check does not measure every page. No Live Demo stack was
  started. No PostgreSQL proof was run.
- Question Library browse and the Course roster stay scannable at 1280 by 800. Two Library records
  show their titles and facts on one line, and two roster rows keep the Student and state columns
  aligned. The rows stay square and separated by a divider. The check does not measure every page.
  No Live Demo stack was started. No PostgreSQL proof was run.
- Question Library search filters exclude one nonmatching Question in PostgreSQL. Sixty-five
  matching Questions stay in the shared-filter result. The outsider's tag returns only that
  Question, and excluding the matching text keeps the outsider. The proof uses 66 Questions and
  does not time a 13,000 Question library. No Live Demo stack was started.
- Question Library browse sizes its result region to the viewport, the Template empty state sizes to
  its message, and a Question preview adopts a bounded resize report instead of a tall fixed height.
  Headless Chromium checked those shipped surfaces at 1280 by 800. The check does not measure every
  page. No Live Demo stack was started. No PostgreSQL proof was run.
- Question Library browse keeps two result records, with their titles and facts, inside a 1280 by
  800 viewport. The Course roster keeps two Student rows inside that same viewport. The browser
  proof rendered those shipped pages. No Live Demo stack was started. No PostgreSQL proof was run.
- Bloom Classification sorts mixed fixed and Pool Assessment Entries, and Save sends that order.
  A later save conflict asks for a reload, and the reload shows the saved order. Question Library
  browse search receives the selected Cognitive Process and Knowledge Dimension. The browser proof
  called those pages through a test double. No Live Demo stack was started. No PostgreSQL proof was
  run.
- Question Library search is the collection workflow. Archive stays one Published Question on its
  detail control, and rendering that control does not archive it. The library search render did not
  load 13,000 Questions. No Live Demo stack was started. No PostgreSQL proof was run.

## 2026-09-30

### Developer Tests and Notes

- Recorded a static design review of the 378 changed-file entries against `e67eacab`, with
  strengths, defects, refinements, and per-file coverage in the
  [deep review](active_plans/audits/human_guidance_deep_code_review_2026_09_30.md).
  This review ran no tests or builds and changed no implementation files.
