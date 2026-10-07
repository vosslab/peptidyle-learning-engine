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

- M29 `valid_draft_execution` now previews and tests Native HOTSPOT image content
  and a distinct checked-in PGML question, checks loaded image state, exact saved
  Edit Numbers and preview/test seeds, and publishes a changed Native correction
  under the same Question ID. Focused format, lint, both TypeScript configs, and
  scenario-registry checks pass. The Playwright CLI and connected runtime still
  require the separate real-stack owner input; no runtime acceptance is claimed.
