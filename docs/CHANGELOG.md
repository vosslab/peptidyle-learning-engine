# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-05

### Behavior or Interface Changes

- Named Instructor screenshots by visible PLE location and state: removed redundant laptop
  suffixes from both picker filenames, renamed the Profile capture to `profile_preferences.webp`,
  and removed shared-search implementation wording from Library result filenames and captions.
  Question and Pool detail pages now belong to Browse Question Library in the Ribbon, breadcrumbs,
  return links, and capture workflows. Browse initially presents categories without a false
  no-matches message. Updated existing navigation checks and screenshot links; no permanent tests
  were added. TypeScript, lint, formatting, 534 frontend checks, retained UI checks, and all
  10,468 Python checks pass. Refreshed all 256 screenshots from a clean Demo, preserving 76
  laptop-only Instructor captures; corpus replay and artifact checks pass. A one-time browser
  probe also verifies initial Browse failure, retry, and recovery.

- Corrected the shared-search drift audit findings: removed the obsolete standalone Pool-list
  API/store/SQL/client path; enforced Instructor Pool ownership with a role-typed composite
  foreign key and covering index; replaced shared synthetic Library fixtures with local inputs;
  and corrected combined-Library labels. A second set of six independent audit passes completed.
  Rust checks, all three connected acceptance lanes, and the clean-schema
  Instructor/Student/Sysadmin ownership probe pass. The broader
  Library Object result-contract design remains explicitly deferred.

- Restored Human Guidance's laptop-only Instructor and Sysadmin screenshot scope. Removed
  the Library phone capture, both square picker captures, and their unauthorized Python/Node
  test exceptions. Regenerated the manifest, receipt, atlas, and galleries for the remaining
  256 captures using their existing fresh images. Static artifact verification, 20 corpus
  tests, 405 viewport/link checks, TypeScript, lint, and formatting pass.

- Applied the permanent-test checklist to shared-search additions. Removed duplicate browser and
  accessibility harnesses, repeated API/filter matrices, and generic decoder inventories instead
  of retaining implementation proof as blocking tests. Net growth under `tests/` falls from
  3,019 to 965 lines; Rust test additions fall by another 366 lines. Retained coverage owns the
  session, real Library journey, exact pins/edit tokens, and database integrity once per boundary.
  Requested screenshot scenes remain; documentation records one-time reviews separately.
  Fast checks, the retained browser suite, live Library, connected database/SQL oracles,
  documentation checks, and independent review pass.

- Refreshed the complete repository screenshot corpus from a newly initialized Live Demo.
  The picker capture now owns its Pool fixture, and the shared fixture helper invokes its
  browser-side setup correctly. A live create/reuse check confirms exactly one fixture Pool;
  TypeScript, lint, formatting, and corpus checks pass. All 259 captures are published;
  live verification passes manifest closure, privacy, and artifact-integrity checks.

- Shared search M16 completes the architecture guide, aligned documentation, one shared page-size
  constant, and the refreshed screenshot corpus. Live captures fixed the Assessment picker's
  dialog surface; publication, verification, accessibility, live Library, parity, and independent
  review pass. The full aggregate passes 549 Node tests, 10,330 Python checks, and all three
  real-service acceptance lanes after repairing stale policy and shared-ID test fixtures.
  Temporary probes are removed; the separate Pool validation handoff remains open.

- Shared search M15 passes independent usability and visual review, axe on all five surfaces,
  and keyboard/paging checks. Compact now retains descriptions in every display mode, with a
  permanent parity regression. Final review also restricts Pool Type text matching to the
  approved Question-only grammar while preserving structured Pool Type filtering, verified
  in fresh PostgreSQL. The complete offline and browser gates pass; temporary probes are
  removed after retaining their evidence.
- Shared search M14 replaces the separate Assessment Pool picker with one Question/Pool
  content picker. Blueprint draft appends and Course explicit fork imports preserve their
  existing workflows and exact concurrency tokens. Full frontend gates, independent review,
  rebuilt real Blueprint/Course import journeys, and Library integration pass.
- Shared search M13 moves the Question picker onto shared search while retaining exact pins,
  inspection, and the ordered tray. Pool workflows constrain candidates by authoritative
  classification, Type, and Backend, including eligible Questions already in another Pool.
  The scoped Blueprint reader supplies required Pool eligibility. Full frontend/Rust/schema
  gates, fresh PostgreSQL proof, browser eligibility checks, and independent review pass.
- Shared search M12 unifies Question and Pool Library results with Both/no-Pool defaults, a
  visible removable membership chip, Owner filtering, URL state, and all three displays. One
  capped selection feeds kind-specific editors and preserves the untouched kind after saving.
  The separate Pool panel is removed; Question-only pickers retain All Questions. Full frontend
  gates, mixed live journeys, speed replay, and independent review pass.
- Shared search M11 routes both Questions and Pools through `/library/{id}`, with object titles
  in breadcrumbs and Pool links from discovery and the Watch inbox. Pool detail preserves
  exact member credits and Instructor controls. Frontend gates, rebuilt live detail/retry
  journeys, and independent review pass.
- Shared search M10 combines Questions and Pools in one authorized database query, with kind,
  membership, owner, global cursor paging, and strict tagged results. Kind lookup shares Library
  visibility rules. Connected mixed/fork/filter/paging proofs and rebuilt Live Demo API/browser
  checks pass. A measured function-local JIT correction reduces HTTP medians to 15.6-17.6 ms;
  independent review is clean. The Library interface remains Questions-only until M12.
- Shared search M9 calculates a required Pool collection license from exact member Revisions
  on creation, member replacement, and forks. Manual Pool provenance is removed; member license
  and authorship remain visible. All seven supported combinations, exact pins, downgrade, forks,
  and internal-helper privileges pass fresh PostgreSQL proofs. Full Rust, schema, TypeScript,
  and independent review pass.
- Shared search M8 stores each Pool's Type and Backend from its first member, keeps that pair
  immutable, and permits each Question only once. Creation, member edits, and forks enforce the
  same rules. Full Rust/TypeScript gates, connected rejection/fork proofs, the rebuilt Live Demo,
  and independent fixture/audit review pass. The live repair set was empty before and after.
- Shared search M7 removes obsolete Pool authorship and attribution from the base schema and
  SQL credit functions. Pool owner, source-Pool link, and member authorship remain intact;
  manual license replacement remains M9. Fresh PostgreSQL lifecycle, schema, focused Rust,
  and the full codebase checks pass. Current Rust/browser contracts already omitted these fields.
- Shared search M5 gives Library Search, Browse, and Blueprint one responsive page shell.
  Automatic URL/Browse searches leave directly; user searches and drafts warn before leaving;
  Clear and Browse Start over reset that warning. Pool creation preserves the mounted search.
  Full frontend and live Library gates, independent review, and desktop/narrow captures pass.
- Shared search M4 moves Public Blueprint Course search onto the same state, controls, and
  results as Library. Name and Tag remain local until one Search/Enter submission; filters
  apply immediately, display changes make no request, and Clear returns to idle. The full
  frontend gate, independent review, and current-source live Blueprint screenshot pass.

### Fixes and Maintenance

- The live Library API gate now checks global pagination separately from the eight known Pilot
  Questions, which it locates by exact title. It no longer assumes the Pilot is the entire
  Library. Assertion failures propagate correctly, and an exit handler restores the Question
  used for archive/restore evidence. The API journey passes on the rebuilt demo.

### Developer Tests and Notes

- Planned a dedicated 31-file Question specification set with narrow responsibilities, existing
  PLE terms, explicit field and operation rules, and one owning document per rule. The plan uses
  the existing Rust QTI Package Maker for conversion work and records WASM as future direction.
  A separate ten-file BiologyProblems.org set covers common source/import rules, the importer,
  and six Course-specific inventories, Assessment mappings, and content gaps. Added explicit
  Question import and Blueprint Course import API specifications; the complete planned set has
  42 files, including the shared Blueprint Course import API document outside those folders.
  Recorded Biochemistry and Genetics as complete source courses, and Molecular Biology,
  Biostatistics, and Laboratory as partial courses, separately from pilot selection and PLE import status.
  Recorded the syllabus Course color preferences and required each Course specification to map
  them to PLE Themes. Added the user's magenta preference for Molecular Biology and teal-green
  preference for Laboratory; their exact RGB values and Theme matches remain unspecified.
  Proposed Ocean for Genetics, Grassland for Biostatistics, Magma for Biotechnology, and tentative
  Tundra for Biochemistry; its lavender surfaces and green accents are only an approximate purple match.
  Required content importing and Blueprint Course assembly through authenticated PLE APIs using
  returned IDs. Source inspection found that current pilot creation already generates Question
  and Blueprint IDs but calls database-backed routines directly, leaving the API path untested
  by that loader.
  [question_specs_documentation_plan.md](active_plans/active/question_specs_documentation_plan.md)
  passes its focused Markdown link check; writing the specifications remains a separate task.
- Audited the shared Library Object boundary across documentation, SQL search, API types,
  browser rows, and Assessment selection. Recorded field-loss and filter inconsistencies plus
  a proposed Question documentation structure in
  [library_object_documentation_audit_2026_10_05.md](active_plans/audits/library_object_documentation_audit_2026_10_05.md).
  The new report passes its link check. The full link check reports 404 passes and one older
  audit with three links to renamed Pool files; this source audit changes no runtime behavior.
- Documented the implemented shared-search boundary and the approved three-display rule.
  Human Guidance and its generated checklist match at 1,196 bullets; the new checklist item
  remains open until mixed Library and picker acceptance. Preliminary isolated-browser checks
  found no search storage writes and confirmed late-response ordering.
- M5 review corrected Browse reset leaving its warning armed and rendered Blueprint inputs
  extending into the results column. Live browser checks now use semantic region/list locators
  and a bounded page assertion: the current demo has 49 Questions, not an assumed 50.

## 2026-10-04

### Additions and New Features

- Shared search M1 adds a content-defined request session with ordered responses, cursor paging,
  exact retry, bounded selection, and disposal. Replacement searches clear stale rows and facet
  counts; failed paging preserves the current page. Eight session regressions and the full
  TypeScript, lint, formatting, and 544-test Node gate pass. Implementation and remaining gates
  are tracked in [SHARED_SEARCH_IMPLEMENTATION.md](active_plans/reports/SHARED_SEARCH_IMPLEMENTATION.md).

### Behavior or Interface Changes

- Shared search M2 moves Library requests into the shared session. Typing stays local until
  Search or Enter; facet counts follow the applied query. Focused browser checks and the full
  codebase and fast UI gates pass.
- Shared search M3 gives Library shared controls, results, display switching, selection, and
  removable filter chips. Library filters and bulk editing have focused modules; Browse still
  waits for exact filters, and successful bulk edits retain their confirmation while refreshing.
  The obsolete row renderer is removed. The full frontend gate, independent review, import
  boundary check, and rendered filter-layout inspection pass.
- Shared search M6 records an immutable Pool owner for creation and every fork. Initial Course
  adoption uses the assigned Instructor; later append and Apply use the authorized actor.
  Connected PostgreSQL tests prove distinct adopter and co-Instructor ownership. They also
  exposed and fixed Apply passing a Blueprint ID to the Course-specific Assessment save path.
  Fresh schema/lifecycle checks and independent review pass.
- Implemented the authorized repairs from the priority HG audit. Assessment score
  readers now share current-point handling for retired Pools; a fresh disposable
  PostgreSQL replay changed an existing Attempt from 2/2 to 1/1 after removal.
  Trusted writes enforce post-issue content limits and protect issued Pool
  members. Automated score timing controls were removed, and Quiz/Exam answer
  timing is an overridable default. Focused validation and remaining acceptance
  work are tracked in [HG_COMPLIANCE_FIXES_2026_10_04.md](active_plans/HG_COMPLIANCE_FIXES_2026_10_04.md).
- Replaced search restoration with separate result tabs and discard confirmation,
  added shared search display modes, and expanded Blueprint comparison fields.
  The shared RecordList browser regression passed after its reorder checks were
  extracted into a focused helper to meet the source-file size limit. Account,
  public Profile, and forum-removal code is integrated. Fresh publication captured
  247 screenshots with a matching receipt. The complete Rust workspace gate
  passes, including strict Clippy, test-support and all-feature tests, doctests,
  and WebAssembly. TypeScript, lint, formatting, 536 Node tests, and 10219 Python
  tests pass; live interaction acceptance is recorded in the repair plan.
- Recorded Neil's direction to follow GitHub's repository model for Blueprint
  Watches. Watch counts represent notification subscribers and support sorting;
  search does not expose watcher identities. The interview record distinguishes
  his instruction from the implementation mapping. Neil confirmed real email
  delivery awaits his email-account setup; local validation does not certify an
  externally configured provider.

### Fixes and Maintenance

- Aligned active Pool/search terminology, model, API/data descriptions, Assessment lifecycle,
  architecture, Instructor guide, Bloom guide, and design decisions with current HG. Documented
  unordered membership, one Type/Backend, Pool-owned metadata, compatible licensing, organized
  combined search, and Pool mismatch behavior. Marked current SQL/API gaps rather than claiming
  implementation completion. Added current-guidance notices to the completed RecordList plans.
- Clarified required non-NULL metadata, including Question Type, and separated it from optional
  metadata and pending AI-assigned Bloom Classification. Neil withdrew the proposed 24-hour
  deadline: no time limit is enforced. Recorded why automatic assignment and publication checks
  reduce dependence on Instructors completing every field manually.
- Documentation validation passed: 392 guidance-format/link checks, HG/checklist equality at
  1195 bullets, status consistency, and whitespace checks. Replaced dead links in historical
  audits with labeled references to removed files. No application or database code changed.

- Recorded Questions in no Pool as a search filter and Neil's preferred default of those
  Questions plus Pools, reducing redundant member results while retaining member-inclusive
  search. Clarified that already-released Assessments continue as-is after a Pool mismatch.
  Updated the manager handoff; HG/checklist diff and consistency pass at 1192 bullets.

- Replaced provisional stale-Pool wording with Pool mismatch: the Pool no longer satisfies
  its current requirements. Documented specific causes and updated the shared search handoff.
  HG/checklist diff and consistency pass at 1188 bullets; whitespace check passes.

- Corrected Pool membership to an unordered set of distinct Questions, with spreadsheet-style
  sorting for the editor display. Recorded stale-Pool release blocking after classification
  mismatches and the requirement to retain enough members for Assessment selection. Removed
  speculative Assessment-overlap restrictions from the handoff. HG and checklist match at
  1186 bullets; consistency and whitespace checks pass. Documentation only.

- Made the original combined Question-and-Pool search requirement explicit in
  HG, with Both, Questions only, and Pools only filters. Corrected the schematic
  to require combined server sorting/paging and recorded the existing picker
  rules without reopening them as product decisions.

- Recorded automatic Pool license calculation from member licenses, with
  incompatible combinations rejected. Updated the shared search schematic;
  current NC and ND deferrals remain in place.

- Replaced optional Pool authorship in HG with Pool owner and source-Pool link,
  as selected by Neil. The Pool has no separate Author field; member ownership
  and authorship remain intact. Updated the shared search schematic.

- Recorded Pool keyword search as matching only Pool text and metadata, with no
  member-Question search expansion. Updated the shared search schematic.

- Recorded Pool-only Topic/Subtopic, Tags, and Bloom filtering. Deferred NC
  content alongside ND and moved the NC compatibility example into deferred
  guidance. Updated the shared search schematic with the settled Pool rules
  and the remaining search decisions.

- Recorded NC Question license support as current scope and ND support as
  deferred in HG. Future ND support must restrict Question forking; the current
  search work does not enable ND content.

- Added Neil's concrete CC BY-NC-SA Pool compatibility example to HG and the
  checklist. Recorded missing CC BY-NC and CC BY-NC-SA support across the
  current Question license contract as implementation work.

- Recorded Pool-specific Tags and both Bloom dimensions in HG and its checklist,
  alongside the Pool's own Topic/Subtopic. Member metadata remains separate;
  search matching behavior is still being clarified.

- Made the Pool license required in HG: one license compatible with every member,
  never a Mixed label. Preserved individual Question licenses and removed the
  older optional Pool-license wording; implementation checks remain open.

- Corrected Pool membership guidance to preserve each Question's owner as well
  as its authors. Both may differ across members; Pool ownership is separate.

- Recorded separate Pool Topic/Subtopic and allowance for different member
  authors in HG and its checklist. Distinguished Pool metadata, member metadata,
  and shared membership requirements in the interview record; filter behavior
  and Pool-specific Tags remain proposals.

- Clarified Pool classification membership in HG: Discipline and Subject match;
  Topic and Subtopic may differ. Recorded Neil's explicit choice and retained
  the separate same-Type and same-Backend requirements.

- Recorded one Question Type and one Backend per Pool in HG after Neil's
  explicit Backend choice. Updated checklist evidence to keep enforcement
  pending and remove obsolete mixed-Backend evidence from the new requirement.

- Recorded Neil's direction toward one Question Type per Pool, including no
  mixing Matching and Multiple Choice. The interview record supersedes the
  proposed mixed-type search behavior and keeps Backend uniformity separate.

- Recorded compatible member licenses as the settled minimum for Question Pools
  in HG and its evidence checklist. A stricter identical-license requirement
  remains open; implementation verification remains pending.

- Recorded the Pool member-license discussion as undecided: Neil prefers to
  avoid mixed licenses but has not chosen identical licenses over compatible
  licenses. No new membership restriction was implemented.

- Added [SHARED_SEARCH_PAGE_SCHEMATIC.md](active_plans/active/SHARED_SEARCH_PAGE_SCHEMATIC.md)
  proposing shared search controls, request handling, and result displays on top
  of the existing database queries. It separates shared behavior from content
  fields/actions and describes a serial migration with behavior checks.

- Traced the September 25 shared-interface plan to its completion ledger,
  commit, and current search code. Confirmed that shared layouts, controls,
  and database filtering/sorting/paging remain implemented. Recorded Neil's
  clarification that custom search pages are acceptable and further reuse
  should build on that work.

- Replaced invented Question Pool curator wording with owner in current HG and
  review records, and recorded that Pools are designed to be forked often.
  Checked the search code: shared result displays exist, but the shared
  spreadsheet-style search remains unfinished. Recorded the source evidence,
  existing Pool-fork behavior, and unresolved Pool owner identity in the audit.

- Clarified the HG audit report: Students viewed Instructor Profile images in
  the successful check. Recorded Neil's direction that Students do not get
  avatars and the conflicting older HG gallery rules for follow-up. Reworded
  the remaining Profile links, Pool notices, search design, and Question Star
  test gaps in plain product language.

- Completed the independent Plan, Test, Style, Docs, Legacy, and Comment review
  of the HG repairs. Corrected active Instructor/email/feedback guidance and
  source wording, removed obsolete approval-only browser checks, and updated
  the Star privacy fixture. Strengthened the existing screenshot privacy and
  Blueprint sort tests. After cleanup, 536 Node and 10217 Python tests pass;
  the isolated Blueprint PostgreSQL proof passes. Remaining findings and the
  Sysadmin avatar-upload failure are recorded in
  [HG_SIX_PASS_AUDIT_2026_10_04.md](active_plans/reports/HG_SIX_PASS_AUDIT_2026_10_04.md).
- Corrected the Profile-image upload and deletion queries to read the SQL
  functions' `object_record_id` column. The wrong column name caused live uploads
  to return HTTP 503 before reaching object storage. Finalization now uses the
  same mapping, and Profile work creation uses transaction-consistent timestamps
  so its creation/update constraint holds.
- Added the missing `after_all_students_complete` workspace decoder case. SQL
  created new Quizzes correctly, but the reader rejected their answer default
  and returned HTTP 422; a focused regression now protects that boundary.
- Gave the bundled-curriculum publishing Account its required Instructor Profile.
  A missing publisher name and affiliation caused the live Blueprint list to
  reject all results; source authorship remains in the curriculum manifest.
- Fixed Live Demo startup with email delivery unconfigured and updated its
  Assessment seed call after removing score timing. Runtime installation SQL and
  Course content are now copied after Rust compilation in the container build;
  SQL-only repairs no longer change those Rust compilation inputs. SQLx forward
  migrations and embedded RDKit assets remain compile-time inputs.

### Decisions and Failures

- Audited current HG implementation before fixes across Assessment fairness,
  score/answer visibility, Instructor onboarding/Profile access, and search/navigation.
  Reconciled all nine checklist parts to 1177 current bullets (81 added/reworded,
  50 removed/reworded), reopened inherited verification, and preserved prior evidence
  and annotations. The prioritized report is
  [HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md](active_plans/reports/HG_IMPLEMENTATION_COMPLIANCE_2026_10_04.md).
  Before the repair, a disposable PostgreSQL probe confirmed that removing a Pool leaves its earned
  and possible points in an existing Attempt. Further findings include unrestricted
  post-issue edits, score withholding, an unchangeable Quiz/Exam answer gate, and
  the rejected onboarding and search-restoration workflows. Seventy-five focused
  Rust/Node tests and the normal saved-response PostgreSQL E2E passed; several tests
  protect obsolete behavior. Checklist diff/consistency/evidence and all nine part
  gates passed; 381 guidance-format/link checks passed. Live browser acceptance
  was not run because the configured Live Demo control receipt was absent.
  Deferred decisions remain deferred; production code was not changed by this audit.
- Cleaned supporting Question, Assessment lifecycle/payload/activity, repeated-practice,
  security, identity, and object-storage documents against settled HG. Removed the
  duplicate Quiz collaboration rule and speculative authoring collaboration;
  corrected obsolete Assessment Type names, score disclosure, deferred feedback,
  and Instructor terminology. Recast future H5P details as deferred design work
  rather than recorded human decisions. Guidance-format and Markdown-link tests
  pass (377 tests); production code and generated schema evidence remain unchanged.
- Continued the HG language audit through API, authorization, backend, and
  terminology contracts. Removed stale score-withholding and mandatory answer
  timing rules; kept optional feedback timing deferred. Clarified Native JSON
  format versus Question Revisions, Student Ribbon status, and Blueprint result
  fields. Removed the unsupported Instructor-grant requirement for Sysadmin
  repairs following Neil's correction, while deferring much of the unproven
  Sysadmin functionality beyond Account creation. Targeted documentation checks
  pass (46 tests); no production behavior was changed.
- Reconciled design, terminology, and contracts with HG's existing equal-Instructor
  rule; removed wording implying a Verified Instructor permission tier. Corrected
  Star-list-only identity restrictions, Instructor image visibility, score withholding,
  broad post-issue edits, required classification, and the incorrect Pool Revision
  model. Withdrew general discussion-thread and Sysadmin-only Pool moderation rules.
  Recorded Change Proposals terminology, GitHub-style Stars, and the close Blueprint
  Course/repository analogy with HG-defined differences. Targeted documentation
  checks pass (46 tests); production reconciliation remains follow-up.
- Extended the bounded design-authority review: Pool discussion/impact-notice
  administration is restricted to Sysadmins without explicit HG support. Logged
  the question without choosing new permissions. Verified that bulk-metadata SQL
  already includes Discipline and Subtopic despite stale design wording.
  Interview follow-up Markdown links pass; no production behavior changed.
- Made cautious Pool removal firm: individual members can be removed only before
  issue to any Student in the Assessment. Remove a bad Pool as a whole and exclude
  its earned and possible points from every Attempt; fairness takes precedence.
  Guidance-format tests pass (2 tests); implementation remains follow-up.
- Recorded a tentative limit on individual Assessment Pool-member removal: the
  Question must not have been issued to any Student in that Assessment. This
  avoids changing already-issued work or introducing different scoring remedies
  for individual Students. Guidance-format tests pass (2 tests).
- Settled the score effect of complete Assessment Question/Pool removal: remove
  earned and possible points from all Attempts for fairness to every Student.
  Reaffirmed existing destructive Unrelease behavior; individual Pool-member
  removal effects remain open. Guidance-format tests pass (2 tests).
- Recorded limited post-issue Assessment content edits: point changes, reordering,
  and Pool-member removal. Complete Question/Pool removal remains tentative;
  effects on already-issued Attempts need clarification. Guidance-format tests pass.
- Traced the remembered structural-edit restriction through older implementation
  history and the September 12 current-state redesign. Current code preserves
  started Attempt facts while allowing edits for future Attempts; whether the
  broader restriction was intentionally removed remains unresolved. Recorded
  this and bounded design-authority findings in the interview follow-up note.
  Its Markdown link check passes; no runtime or production changes were made.
- Recorded that search prompts and results must never be stored permanently,
  with database size and bloat as the stated reason. Guidance-format tests pass.
- Deferred the Question Feedback timing decision until Neil reviews how feedback
  is used in PLE. Removed wording that treated a separate timing control as
  settled; optional feedback content itself remains in scope. Guidance-format
  tests pass.
- Recorded the decision not to store old search results for restoration, closing
  that proposed feature. Separate tabs for list items and search-discard
  confirmation remain required. Guidance-format tests pass.
- Confirmed the Quiz answer default waits for every Student to finish, with
  Instructors changing the setting for absent Students. Recorded the accepted
  need for closer Quiz oversight; score visibility remains immediate. Optional
  feedback timing is still under discussion. Guidance-format tests pass.
- Removed score withholding and separate posting from HG for automatically graded
  Attempts: Question scores and totals appear after submission and grading without
  waiting for classmates. Reconciled older Student-interface score-release wording
  and recorded Neil's reasons. Targeted documentation checks pass; production
  behavior has not changed during this interview.
- Reviewed interview wording against existing HG: Native JSON corrections follow
  existing owner/revision rules, and backend Question Feedback follows its own
  release setting. Logged the unresolved overlap between uncontrolled Question
  scores and older score-release wording. Targeted format and link checks pass.
- Clarified that only a Native JSON Question's owner can correct it; other
  Instructors can change its point value in their Assessment. Guidance-format
  tests pass.
- Deferred Native JSON regrading for now, preserving Neil's tentative wording.
  Future regrading replaces the previous result rather than retaining superseded
  grades. Corrected the interview's unsupported grading-history assumption;
  guidance-format tests pass.
- Recorded conditional support for rechecking Native JSON responses after a key
  correction, with infrastructure cost unresolved. Narrow source inspection
  confirms one immutable grading result per Question Attempt; no implementation
  or runtime validation was performed. Documentation whitespace check passes.
- Recorded zero Assessment points as the usual remedy for flawed Questions,
  with Native JSON correction as an alternative. No separate grade-correction
  workflow was requested. Guidance-format tests pass.
- Made Assessment-type answer-release rules Instructor-changeable defaults and
  recorded Neil's tentative preference for flexibility. Guidance-format tests pass.
- Recorded the Blackboard Ultra results-timing model supplied in the interview,
  excluding controls for Question score visibility or timing. The meaning of
  all-grades timing for absent Students remains open. Guidance-format tests pass.
- Clarified the Sysadmin's Instructor Account fields: email address, first name,
  last name, and affiliation. Recorded the request to focus interview questions
  on consequential behavior. Both guidance-format tests pass.
- Recorded interview decisions in Human Guidance: public-within-PLE Instructor
  Profiles, all named Blueprint sorts, Student-count wording, shared spreadsheet
  search and multiple display modes, tooltips, new tabs/windows for list items,
  and confirmation before discarding an existing search. Removed the unwanted
  Quiz collaboration sentence. No production code changed in this interview.
- Added reasons and expressed decision strength to the interview follow-up note,
  distinguishing firm rules, settled choices, flexible references, open questions,
  and delegated engineering details. The broader design interview remains open;
  the old audit's 27 bullets are not a complete design inventory.
- Recorded Neil's concern about shared spreadsheet-style search in
  [HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md](active_plans/decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md)
  for investigation after the guidance interview. Shared list/table components
  remain; the limited history check does not establish whether a fuller
  spreadsheet interface was removed.
- Moved System-wide settings into Human Guidance's deferred product behavior
  section at Neil's request; the settings themselves remain undefined.
- Neil clarified Instructor onboarding: a Sysadmin vets the Instructor outside
  PLE before creating an Account with an email address. PLE sends its setup email.
  Deactivate/reactivate controls later access; PLE has no approval workflow or
  approval status. Updated Human Guidance and the audit reports. The current
  code's required vetting decision remains an implementation gap.

### Developer Tests and Notes

- Shared search M1/M2 independent review found no blocker; all 22 focused session/Library tests
  passed. Integration review fixed obsolete cursor reuse after a failed page-size change and
  kept UI callback errors outside transport retry handling. Library submit, Bloom filtering,
  and Pool-task isolation browser checks pass. M2's broad browser gate remains open while a
  Student navigation test failure is investigated.

- Both guidance-format tests pass. The implementation checklist and its counts
  remain the pre-interview snapshot; no runtime compliance is claimed for the
  clarified onboarding behavior.

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
