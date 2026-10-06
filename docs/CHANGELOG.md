# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-05

### Documentation

- Corrected the search specs' treatment of current API names: they are implementation evidence,
  not compatibility requirements. Added naming alignment to TODO across schema, Rust, TypeScript,
  JSON, API consumers, and tests, including Library search and Blueprint Course Revision Tuple.

- Standardized Blueprint Course Revision Tuple in Rust comments and assertion text, JavaScript
  test names, the semantic-contract registry, and the historical docs identified by Neil.

- Checked the reviewer follow-up against current HG. Clarified publication Type requirements,
  copied fork fields, Watch wording, and the Questions a Pool contains. Kept Library Object and
  Blueprint Course Revision Tuple; retained notice removal as implementation follow-up.
  All 1,237 HG/checklist bullets align; 451 documentation checks passed.

- Aligned supporting Question, identity, lifecycle, architecture, database, privacy, and security
  docs with HG: complete Revision records, ordinary reusable Pools, separate Assessment count
  problems, Library Object terminology, metadata ownership, and confirmed Sysadmin access.
  Kept citation format and the initial search default unsettled. Recorded the alignment in the
  settled-decision review; existing TODO items retain the implementation work.

- Restored Library Object wording wherever rules apply equally to Published Questions and
  Question Pools, including Instructor workflows. The terminology rule now expands the names
  only when defining Library Object or distinguishing its two kinds. All 1,237 checklist bullets
  match HG; 451 documentation checks passed.

- Kept Blueprint Course Revision Tuple and Library Object as established terms. Clarified
  Instructor-facing editor/search wording, removed the duplicate Pool classification-problem
  bullet, separated Pool contents from the number each Assessment selects, and simplified the
  metadata ownership rule. All 1,237 checklist bullets match HG; 451 documentation checks passed.

- Kept citation format deferred in the Question metadata spec and open-question log. Labeled
  existing URL/text bounds as implementation details; HG retains the simple term citation.

- Simplified Question definitions, Pool fork wording, Edit Number guidance, and statistics wording.
  Replaced vague source information with citation. Removed current-state phrasing for Pools
  from HG and supporting specifications; the tuple set and Save rules describe the Pool.
  All 1,238 checklist bullets match HG; 451 documentation checks passed.

- Applied the final HG wording corrections: defined Question, kept metadata on Draft and Published
  Question records, simplified Pool selection/Save/problem reporting, removed extra fork recipes,
  included Pool statistics, and adopted Blueprint Course Revision Tuple. Aligned supporting docs
  and TODO; all 1,238 checklist bullets match HG and 451 documentation checks passed.

- Simplified awkward HG wording across interfaces, Pools, revisions, Backends, retention, and
  Assessments. Used direct wording for settled permissions and kept unresolved choices tentative.
  Used Library Objects for shared Question/Pool rules and aligned the terminology guidance.
  Preserved checklist evidence; all 1,241 bullets match HG and 451 documentation checks passed.

- Qualified Question wording in HG where publication, exact Revisions, or Pool inclusion matters.
  Library search and Assessment editing name Published Questions and Question Pools; runtime
  wording stays generic. Simplified the Pool Save and release guidance into three plain bullets.
  Updated terminology and checklists; 451 documentation checks passed.

- Documented Sysadmin confirmation and audited access to FERPA-sensitive Student data, with
  implementation follow-up in TODO. Clarified shared Revision rules and used Blueprint Revision
  Tuple for exact Blueprint identities. Simplified Native JSON wording and the Question definition,
  and specified Published Question metadata in HG and its checklists. Kept Revision publication
  authority separate from the rules defining which changes create a Revision.
  HG/checklists match; 451 documentation checks passed.

- Reviewed settled Question-spec claims against exact HG wording. Corrected publication scope,
  Type/backend generalization, Bloom transport requirements, Pool mismatch wording, and the
  feedback exception. Separated Pool attestation, Native JSON limits, and statistics counters
  from approved product rules; closed stale Q32. Recorded the four-way authority review and
  focused implementation follow-up. Application code and schema are unchanged.

- Kept HG Question Archive behavior explicit: read-only, removed from normal discovery, existing
  references preserved, restorable, and forkable. Placed the shared Question/Blueprint simplicity
  principle immediately after it, with GitHub as the general model. Updated checklists.

- Applied Neil's final short Archive rule: an ordinary Question becomes read-only and leaves
  normal discovery, preserves existing references, and can be restored or forked. Replaced
  intermediate Archive interpretations in current specs and recorded the final rule in Q39.

- Clarified Archive as preservation of Questions that cannot safely be deleted, with normal
  active discovery excluding archived Questions and existing references retained. Kept new-use
  restrictions outside implied Archive behavior. Made the search default explicitly tentative
  and removed unapproved cross-Revision statistics requirements from terminology, design, and
  data-policy docs; recorded implementation follow-up in TODO.

- Traced stored notices from August 29 HG wording through September 17 SQL/API implementation
  and October 4 thread removal that retained notices. Recorded commit evidence, the expanded
  design rationale, and current code locations in the Question-spec authority audit. Static
  history review; no application changes or runtime acceptance claim.

- Recorded Neil's fork-and-fix answer for Question/Pool problems. Removed manually written
  notices from HG Watch events and current specifications; retained notifications for Revisions,
  forks, and Pool membership edits. Added removal of existing notice code to TODO. Docs only.

- Rechecked the Question uncertainty log before further interviewing. Corrected residual
  Assessment-owned Pool wording, removed unsupported certainty about Archive blocking new
  selection, and labeled the existing Bloom sorting restriction as implementation evidence.
  Recorded the remaining notice-feature question and source evidence in the authority audit.

- Corrected the mistaken fork-license finding: a Question fork starts with its source license,
  with no license choice during forking. Removed the proposed compatibility change and its code
  TODO, withdrew F06, and aligned HG, the fork spec, contracts, and audit records.

- Corrected the three remaining Question-spec authority findings. Shared metadata belongs to the
  complete Question record; Native JSON duplicate fields are labeled current implementation
  evidence. Question fork rules preserve attribution and compatible licensing; the exact-license
  SQL lock remains in TODO. Recorded Neil's further direction in HG: WeBWorK Type is assigned
  manually for now and automatic detection is deferred. Aligned specs, contracts, examples,
  checklists, and audit status. Application code and schema are unchanged.

- Traced the Assessment-owned Pool model from September 14 HG wording through the September 15
  agent-authored SQL restrictions, September 16 Blueprint reuse restrictions, and September 18
  current-state conversion. Added commit references, session timestamps, recorded reasons, and
  limits on attribution to the fork audit. This is historical evidence, not a new product decision.

- Audited fork storage and creation across SQL, Rust, TypeScript, tests, fixtures, and docs.
  Recorded five fork-named tables and confirmed special Assessment-owned Pool constraints,
  forced Pool copies, Blueprint reuse restrictions, and matching API/domain types. Question and
  Blueprint creation use ordinary records and child Revision 1 in the inspected paths. Added
  [FORK_MODEL_CODE_AUDIT_2026_10_05.md](active_plans/reports/FORK_MODEL_CODE_AUDIT_2026_10_05.md)
  and focused TODOs for a direct pre-production schema correction, parent fields, retry records,
  Pool metadata copying, and tests. All 450 documentation checks pass. Static audit only;
  application code and schema are unchanged, and no runtime tests were run.

- Recorded Q36 and aligned HG and the Library spec with four agreed credit measures:
  graded-response count, average stored credit, full-credit percentage, and zero-credit
  percentage. Removed unsupported answer-choice statistics and cross-Revision summaries from
  the Library spec, retained usage frequency and privacy requirements, and queued code review
  in TODO. Recorded Neil's interview rule: derive consequences from existing PLE concepts and
  settled rules before asking about genuine alternatives. All 449 documentation checks pass;
  HG/checklist align at 1,232 bullets. Application code is unchanged.

- Recorded Q35: include average earned Question credit and its contributing graded-response
  count. The proposed full-credit and zero-credit percentages remain undecided. All 449
  documentation checks pass; application code is unchanged.

- Recorded Q34 as a clarification of existing responsibilities: Question statistics use
  stored Question credit; Assessment settings determine awarded points separately. Withdrew
  the invented performance-summary name and kept proposed aggregate measures undecided.
  All 449 documentation checks pass; application code is unchanged.

- Recorded Q33: Question Library statistics should show how often Students received a
  Question and how much credit they earned. Exact measures remain under discussion; the
  choice approves no derived difficulty rating or cross-Revision summary. All 449 documentation
  checks pass; application code is unchanged.

- Recorded the rejected statistics interview premise as Q32: define useful usage and Student
  performance measures before asking about combined Revision statistics. Neither reporting
  option was approved; unsupported agent additions remain queued for documentation cleanup.
  All 449 documentation checks pass. Product rules and application code are unchanged.

- Recorded Q31: shared Question metadata belongs to the complete Question record; Native JSON
  holds Backend rendering, response, and grading material. Captured Neil's duplication concern
  and explicit confirmation, queued the spec cleanup, and extended the focused implementation
  TODO. All 449 documentation checks pass. No source-format or application changes made.

- Recorded Neil's explicit default: new Assessments start with partial credit enabled.
  Updated HG, its checklist, the Backend and scoring specs, the decision log, and implementation
  TODO. Instructor control and stored-fraction scoring remain unchanged. All 449 documentation
  checks pass; HG/checklist remain aligned at 1,230 bullets. Application code is unchanged.

- Renamed the dedicated scoring specs to
  [MULTIPLE_ANSWER_SCORING_SPEC.md](QUESTION_SPECS/MULTIPLE_ANSWER_SCORING_SPEC.md) and
  [ORDER_SCORING_SPEC.md](QUESTION_SPECS/ORDER_SCORING_SPEC.md), updated documentation references,
  and kept their current Native JSON scope explicit. Another Backend may adopt either model
  explicitly. Scoring rules and application code are unchanged. All 449 documentation checks pass.

- Applied the post-consolidation reviewer cleanup: removed duplicate Pool/Search links and stale
  Bloom API wording. Removed required Question language from product rules while retaining the
  actual schema/Native JSON constraint as labeled implementation evidence with a TODO. QTI now
  leaves its converter handoff undecided, removes the unsupported exact Native JSON mapping,
  and separates Question content conversion from deferred feedback timing. Aligned active search
  guidance and interview summaries with per-Assessment Pool-use mismatch. Recorded the findings
  without adding new HG decisions. All 449 documentation checks pass; HG/checklist remain at
  1,230 bullets. Code is unchanged.

- Consolidated Question import workflow and API evidence in
  [QUESTION_IMPORT_SPEC.md](QUESTION_SPECS/QUESTION_IMPORT_SPEC.md), Browse into
  [QUESTION_LIBRARY_SEARCH_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_SEARCH_SPEC.md), and deferred
  bulk-edit workflow/API evidence in
  [QUESTION_LIBRARY_BULK_EDIT_SPEC.md](QUESTION_SPECS/QUESTION_LIBRARY_BULK_EDIT_SPEC.md). Removed
  the three redundant files and updated README, cross-references, plans, and coverage records.
  Filter semantics, cross-kind behavior, combinations, and count scope remain owned by the filter
  spec; Search references those rules. Existing implementation details are labeled separately.
  The prior Pool consolidation remains intact. One-time reference checks and all 449 documentation
  checks pass; HG/checklist remain aligned at 1,230 bullets. No product redesign or code changes.

- Consolidated all six Pool specs into [QUESTION_POOL_SPEC.md](QUESTION_SPECS/QUESTION_POOL_SPEC.md),
  removed the five narrower files, and updated their references. Recorded Neil's replacement
  model: Assessment entries reference existing Pools and store their own selection counts;
  explicit forks create regular Instructor-owned Pools. Current edits affect future selections,
  while existing Attempts retain selected Questions. Aligned HG, its checklist, Library and
  Course/Blueprint summaries, terminology, contracts, and active handoffs. Recorded Q28/Q29 and
  the reasons, superseding the earlier automatic-fork answer. Unreleased editing state can be
  saved with a specific Pool-use mismatch; release validates completeness, and post-issue limits
  remain. Copied Pool metadata includes existing Bloom fields. Code/API/schema/UI/test work is
  recorded in one TODO item. All 452 documentation checks pass; HG/checklist match at 1,230
  bullets. This pass changes documentation only.

- Recorded Neil's acceptance of Draft autosave with a visible saved status so Instructors can
  return to unfinished work. Updated HG, its checklist, the Draft spec, Q27, and implementation
  TODO. Automated abandoned-Draft cleanup remains deferred, with no expiration period set.
  All 457 documentation checks pass; HG/checklist match at 1,217 bullets. Code is unchanged.

- Restored automatic Pool forking when an Instructor adds a Pool to an Assessment. The new
  Pool belongs to the Instructor and its parent field points to the original, allowing independent
  editing. Removed the invented reuse-versus-fork choice from the Library and Pool fork specs;
  aligned HG and its checklist, resolved audit F01 and Q21, and updated implementation TODO.
  Recorded Draft Save versus autosave as undecided; expiration remains unspecified and automated
  cleanup deferred. All 457 documentation checks pass; HG/checklist match at 1,216 bullets.
  Application code is unchanged.

- Recorded Neil's clarification that Drafts have no content or metadata requirements. Aligned HG
  and the Draft, Backend, Type, metadata, Native JSON, WeBWorK, and import specs: empty, incomplete,
  or broken content can be created, imported, and saved; complete source and required metadata
  are checked at publication. Closed audit finding F02 in documentation, recorded Q26, and added
  implementation verification to TODO. All 457 documentation checks pass; HG/checklist match at
  1,216 bullets. Application behavior remains unchanged.

- Audited the Question specification refactor against HG and Neil's explicit decisions. Recorded
  seven remaining conflicts or unsupported choices in
  [QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md](active_plans/reports/QUESTION_SPEC_AUTHORITY_AUDIT_2026_10_05.md),
  including the agent's overreach when changing automatic Pool forking during an ownership
  correction. Linked the findings from earlier reports and the decision log. This pass changes
  audit records only; product rules and code remain unchanged. All 457 documentation checks pass;
  HG/checklist match at 1,215 bullets. The findings remain open.

- Corrected the Question record and Pool model after Neil's review. Each Question Revision is a
  complete record with permitted in-place metadata edits. Backend Type classification and Bloom
  use ordinary metadata editing; Native JSON Type follows its source. Pool forks are regular
  reusable Pools with Instructor owners and parent pointers. Pool saves replace current state,
  with no undo; Edit Numbers serve concurrency only. Aligned HG, the Question specs, terminology,
  concurrency, and API summaries. Recorded schema/application work in TODO and set aside the
  archive edge case. All 456 documentation checks pass; HG/checklist match at 1,215 bullets.

- Removed the three obsolete Question Model, Question ID, and Question Backend navigation
  files at Neil's request. Updated Markdown links to the current QUESTION_SPECS owners.
  Recorded that Backends always calculate and PLE stores earned credit; Assessment partial-credit
  changes recalculate all Attempts from stored fractions without regrading. Clarified saving valid
  MATCH and MULTI-FIB responses with unanswered parts. Implementation remains in TODO.
  All 456 documentation checks pass; HG/checklist match at 1,211 bullets. Application code is unchanged.

- Added [ORDER_SCORING_SPEC.md](QUESTION_SPECS/ORDER_SCORING_SPEC.md)
  for Neil's equal-weight position and relative-order scoring rule. Included the pair comparison,
  inversion explanation, permutation and swap tables, and exact reversed-order behavior.
  Closed Q20, aligned HG and related specs, and recorded implementation work in TODO.
  One-time arithmetic checks reproduced the tables across 5,910 permutations. All 459
  documentation checks pass; HG/checklist match at 1,207 bullets. Application code is unchanged.

- Recorded the Native JSON grading map: MC and HOTSPOT all-or-nothing, NUM tolerance, FIB
  accepted answers with regex support, and MULTI-FIB as independently graded FIBs. Closed Q19
  and opened the relative-order question for ORDER. Regex support and MULTI-FIB partial-credit
  implementation remain in TODO. All 458 documentation checks pass; HG/checklist match at
  1,206 bullets.

- Continued the Question-spec decision review: scoped the accepted Matching formula to
  Native JSON with Assessment partial credit enabled, corrected stale MA decision-log wording,
  and opened Q19 about MULTI-FIB grading. Existing all-or-nothing code remains evidence of
  current behavior rather than an approved teaching rule. All 458 documentation checks pass;
  HG/checklist match at 1,202 bullets.

- Settled Native JSON Multiple Answer partial credit with the linear choice-count multiplier
  on both too-few and too-many selections. Updated the dedicated spec, current references,
  decision log, and implementation TODO. All 20 supplied examples pass; one-time arithmetic
  checks cover 45,880 valid count combinations and confirm score bounds and selection behavior.
  All 458 documentation checks pass; HG/checklist remain aligned at 1,202 bullets.

- Created the dedicated Native JSON Multiple Answer scoring spec for Neil's supplied
  cancellation-plus-squared-penalty rule. Recorded Backend ownership and Assessment
  Instructor control of partial credit. Verified the worked examples and flagged two
  conflicts: adding a correct selection can lower credit, and selecting everything can
  earn 72% for nine correct out of ten. Formula correction remains open. All 458
  documentation checks pass; HG/checklist match at 1,202 bullets.

- Updated the preferred Multiple Answer formula to the symmetric average penalty, retaining
  Neil's "probably the strongest candidate so far" qualification. Recorded unequal answer-key
  examples and the change from zero credit to less than 50% for selecting everything when
  incorrect choices exist. One-time arithmetic checks cover the six supplied examples and
  1,640 answer-key compositions. All 457 documentation checks pass; HG/checklist match at
  1,201 bullets. Grading implementation remains in TODO.

- Reopened the Multiple Answer grading decision in the uncertainty log. Recorded Neil's two
  candidate deductions and the uneven-answer-key examples; the final formula remains open.

- Aligned Sysadmin authority across HG, authorization, security, terminology, API, and data
  docs: full administrative access includes Course and Student records; support work is
  recorded for audit. Removed conflicting scoped-access wording and retained implementation
  follow-up in TODO. All 457 documentation checks pass; HG/checklist match at 1,201 bullets.

- Kept HG focused on positive product guidance: removed rejected-filter commentary and a
  redundant search-syntax warning, and recorded the writing rule. The decision log retains
  the removal reason. All 457 documentation checks pass; HG/checklist match at 1,203 bullets.

- Recorded Multiple Answer partial credit: fraction of correct choices selected minus fraction of
  incorrect choices selected, with a minimum of zero. Neil's five-correct-among-ten example earns
  zero when all ten are selected. Implementation remains in TODO. Documentation checks pass;
  HG and its checklist match at 1,205 bullets.

- Settled proportional Matching grading: equal credit per correct pair, wrong or blank pairs
  zero, no additional deduction. Replaced the unsupported all-or-nothing requirement and recorded
  Native JSON grader and incomplete-response changes in TODO. Other Types remain separate
  decisions. All 457 documentation checks pass; HG and its checklist match at 1,204 bullets.

- Clarified Published Question permissions: every Instructor may read, add to an Assessment, or
  fork; the owning Instructor and Sysadmins may edit. Recorded Sysadmins' full administrative
  authority and removed conflicting metadata-only, read-only, and critical-flaw restrictions
  from current specs and authority docs. Unproven tools remain deferred, with implementation
  follow-up in TODO. All 457 documentation checks pass; HG and its checklist match at 1,203 bullets.

- Recorded Neil's metadata-editing rule: Question owner and Sysadmin, with AI editing tentative
  because Instructors will do the bare minimum of metadata writing. Replaced the unsupported
  reference to deferred bulk-edit permissions. Neil clarified that the owning Instructor corrects
  Bloom; other Instructors may fork Questions, not edit them. Corrected the broader documented
  Bloom permissions and recorded code follow-up in TODO. Also removed
  lingering Draft revision-history and HTTP-import claims, and restored HG's archive confirmation.
  All 457 documentation checks pass; HG and its checklist agree on 1,204 bullets.

- Clarified the common Question workflow in HG, the Draft and import specs, and the implementation
  TODO: import or write a Draft, preview it, test it, refine it, add the metadata, then publish.
  All 457 documentation checks pass; HG and its checklist agree on 1,203 bullets.

- Reviewed agent-added assumptions in the Question specs. Made backend-agnostic Draft authoring
  and testing explicit, corrected fork-Draft revision wording, removed unsupported import tracking
  and transport requirements, and closed the unnecessary Blueprint Theme question. Current source
  inspection confirms PG/PGML Draft creation and publication services already exist; the browser
  authoring and Draft-testing gaps are recorded in TODO.md. No application code changed. All 457
  documentation checks pass; HG and its checklist agree on 1,203 bullets; whitespace checks pass.

- Revised the first-delivery set to Genetics, Biotechnology, and Biochemistry. Neil currently
  favors deferring largely incomplete Biostatistics, and explicitly retained Biotechnology.
  BP.org and Neil confirm only Genetics and Biochemistry have complete source-course coverage.
  Updated Q08/Q09, the pilot, and Course specs; all 456 documentation checks pass.

- Settled first-delivery Blueprint scope: Genetics, Biotechnology, Biostatistics, and Biochemistry;
  Molecular Biology and Laboratory come later. Closed Q08 and aligned the pilot and Course specs.
  All 456 guidance-format and Markdown-link checks pass; this follow-up changes documentation only.
- Rotated October 2-3 entries into [CHANGELOG-2026-10b.md](CHANGELOG-2026-10b.md) at the size limit.

- Closed the unapproved "Used in my Courses" question and removed its UI/API/SQL support.
  Offline Rust, frontend, schema, and documentation checks pass. Neil chose to retain these edits;
  further code work and live verification are tracked in [TODO.md](TODO.md) during the docs review.

- Kept recent HG additions close to Neil's stated intent; moved Pool export packaging and fork
  search consequences into Design Decisions, and marked the older bulk-edit design deferred.
  HG/checklist alignment and all 455 guidance-format and Markdown-link checks pass.

- Reworded the latest HG additions closer to Neil's own words and removed the repeated
  Pool-filter explanation. Forks use ordinary Pool rules. Product decisions are unchanged;
  HG/checklist diff, consistency, and whitespace checks pass.

- Recorded the Question-spec interview decisions: Pool forks remain Pools, Instructor bulk
  editing is deferred, base Assessments start from website topics, and PLE does not track later
  BiologyProblems.org changes. Question export targets another LMS and uses the external
  `qti-package-maker-rs` library. Pool members stay separate same-Type Questions within the
  export package. Updated HG, its checklist, focused specs, and the decision log with reasons
  and firmness. HG diff/consistency, Markdown links, and whitespace checks pass; code is unchanged.

- Replaced competing Question definitions with 31 focused Question specifications, ten
  BiologyProblems.org source/Course specifications, and a Blueprint import specification.
  Established shared Library Object rules for Questions and Pools, preserved the canonical
  hyphen/public SHA-256 ID rule, and separated Native JSON from QTI interchange. Old rule-owner
  paths now link to their replacements. Recorded specification conflicts, implementation gaps,
  and unresolved choices without designing import recovery. HG remains unchanged. Markdown
  links, JSON example syntax, and whitespace checks pass; no runtime compliance is claimed.
  A further HG fidelity review restored omitted MATCH, Pool creation, archive, and Profile-image
  rules; removed unsupported fork-ID allocation timing and Assessment-Pool filter claims; and
  recorded the unresolved filter scope with its authority evidence.

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

- Clarified Question metadata and fork fields; separated the Pool tuple-set definition from its
  one-Revision-per-Question rule. Aligned HG wording and checklists; 451 doc checks passed.

- Reconciled all Question decision-review findings and the fork audit with concrete implementation
  TODOs. Clarified Pool Save, release checks, nullable Bloom, metadata ownership, and statistics;
  corrected stale API/default/storage claims. Docs only; 451 documentation checks passed.

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
