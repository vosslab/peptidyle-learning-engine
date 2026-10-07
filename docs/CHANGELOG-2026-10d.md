## 2026-10-05

### Behavior or Interface Changes

- Aligned Blueprint Course Revision Tuple types, fields, and JSON names across Rust, TypeScript,
  generated contracts, and active documentation. Focused implementation checks and scoped independent
  specification and quality reviews passed; M02 milestone acceptance remains pending M01 live
  dependency evidence. See the [M02 verification report](active_plans/reports/question_spec_m02_tuple_names.md)
  and [execution ledger](active_plans/reports/question_spec_implementation_ledger.md).

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

- Added a linked uncertainty log for unresolved Question-spec decisions; review found no new entries.
- Saved the approved 29-milestone implementation plan and ledger for all 35 Question-spec TODOs;
  acceptance remains pending.
- Recorded Luna 6 for workers and independent preparation after contracts/environment are ready. M02
  reviews passed; M01 live evidence and dependent acceptance remain pending.
- Resumed implementation tracking, mapping decision-review and fork-audit findings to milestones or
  no-code dispositions. M01 service/runtime and overall acceptance remain pending.
- Clarified that current API names are implementation evidence, not compatibility requirements; added
  naming alignment across schema, code, JSON, consumers, tests, Library, and Blueprint tuples to TODO.
- Standardized Blueprint Course Revision Tuple wording in Rust, JavaScript tests, contract registry,
  and identified historical docs.
- Aligned reviewer follow-up with HG on publication Type, fork fields, Watch wording, and Pool contents.
  All 1,237 checklist bullets align; 451 documentation checks passed.
- Aligned supporting specs with HG on complete Revisions, reusable Pools, Assessment counts, Library
  Objects, metadata ownership, and Sysadmin access. Citation format and initial search default remain open.
- Restored Library Object wording for shared Published Question/Pool rules and Instructor workflows.
  Checklist matches HG at 1,237 bullets; 451 documentation checks passed.
- Kept Blueprint Course Revision Tuple and Library Object; clarified editor/search terms, Pool contents
  versus Assessment selection count, and metadata ownership. HG/checklist match at 1,237; 451 checks passed.
- Kept citation format deferred and labeled existing URL/text bounds as implementation details; HG
  retains the simple term citation.
- Simplified Question, Pool fork, Edit Number, and statistics wording; removed unsupported current-state
  Pool claims. HG/checklist match at 1,238 bullets; 451 documentation checks passed.
- Finalized HG wording for Question, metadata, Pool selection/Save/problems, forks, statistics, and
  Blueprint tuples; aligned specs/TODO. Checklist matches at 1,238; 451 checks passed.
- Simplified HG terms for interfaces, Pools, Revisions, Backends, retention, and Assessments; kept
  unresolved choices tentative. Checklist matches at 1,241; 451 checks passed.
- Qualified HG wording for publication, exact Revisions, and Pool inclusion; aligned Library search,
  Assessment editing, terminology, and checklists. Documentation checks passed.
- Documented Sysadmin confirmation and audited FERPA-sensitive Student access; aligned Revision and
  Blueprint tuple terminology, Native JSON, and Published Question metadata. HG/checklist match; 451 checks passed.
- Reviewed settled specs against HG; corrected publication scope, Type/Bloom evidence, Pool mismatch,
  and feedback exception. Separated rules from implementation assumptions; code/schema unchanged.
- Kept Archive behavior explicit in HG: read-only, hidden from discovery, references preserved,
  restorable, and forkable; updated checklists.
- Applied Neil's final Archive rule across current specs and recorded Q39: read-only, hidden from normal
  discovery, references retained, restorable, and forkable.
- Clarified Archive as preserving Questions that cannot safely be deleted; existing references remain,
  active discovery excludes them, and new-use rules stay separate. Search default remains tentative.
- Traced stored notices from HG through SQL/API implementation and removal; recorded evidence and
  rationale in the authority audit. Static history review; no code/runtime claim.
- Recorded Neil's fork-and-fix answer: removed manual notices from Watch/specs, retained notices for
  Revisions, forks, and Pool edits, and queued code removal in TODO. Docs only.
- Rechecked uncertainty entries; corrected Assessment-owned Pool and Archive claims, labeled Bloom sorting
  as implementation evidence, and recorded the remaining notice question in the authority audit.
- Corrected fork licensing: forks inherit the source license, with no choice during fork. Removed the
  proposed compatibility change/TODO, withdrew F06, and aligned HG, specs, contracts, and audits.
- Corrected three authority findings: metadata belongs to the Question record; Native JSON duplication is
  implementation evidence; forks preserve attribution/licensing. Type detection remains deferred; docs/contracts aligned.
- Traced the Assessment-owned Pool model's history and rationale in the fork audit; this is historical
  evidence, not a new product decision.
- Audited fork storage across SQL, Rust, TypeScript, tests, fixtures, and docs; recorded schema/design
  gaps and TODOs in [the fork audit](active_plans/reports/FORK_MODEL_CODE_AUDIT_2026_10_05.md). Static only;
  450 doc checks passed, with no code/schema/runtime change.
- Recorded Q36's four agreed credit measures and aligned HG/Library specs; removed unsupported metrics
  and queued code review. HG/checklist align at 1,232; 449 documentation checks passed.
- Recorded Q35: average earned Question credit and contributing graded-response count are in scope;
  full/zero-credit percentages remain undecided. No code change; docs checks passed.
- Recorded Q34: Question statistics use stored Question credit; Assessment settings determine points.
  Removed an invented metric name; proposed aggregates remain undecided. Docs checks passed.
- Recorded Q33: Library statistics should show Question deliveries and earned credit; exact measures
  remain open, with no difficulty rating or cross-Revision summary approved. Docs checks passed.
- Recorded Q32's rejected premise: define usage and Student-performance measures before considering
  cross-Revision statistics. No reporting option approved; docs checks passed.
- Recorded Q31: shared metadata belongs to the Question record; Native JSON stores Backend display,
  response, and grading content. Docs checks passed; no source-format or code changes.
- Recorded Neil's default: new Assessments enable partial credit. Updated HG, checklist, scoring specs,
  decision log, and TODO; code unchanged. Checklist aligns at 1,230; 449 checks passed.
- Renamed the Multiple Answer and ORDER scoring specs, updated references, and kept Native JSON scope
  explicit; another Backend may adopt either model. Rules/code unchanged; 449 documentation checks passed.
- Applied reviewer cleanup to Pool/Search links, Bloom API wording, required-Question claims, and QTI
  converter assumptions. Aligned search guidance without new HG decisions; code unchanged, 449 checks passed.
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

- Clarified Question metadata and fork fields; separated the Pool tuple-set definition from its
  one-Revision-per-Question rule. Aligned HG wording and checklists; 451 doc checks passed.

- Reconciled all Question decision-review findings and the fork audit with concrete implementation
  TODOs. Clarified Pool Save, release checks, nullable Bloom, metadata ownership, and statistics;
  corrected stale API/default/storage claims. Docs only; 451 documentation checks passed.

- Updated the Library API gate to test global paging separately from eight title-located Pilot
  Questions; assertions now propagate and cleanup restores the archived Question. Rebuilt-demo API
  journey passes.
### Developer Tests and Notes

- Planned 31 Question specs, ten BiologyProblems.org course/source specs, and a Blueprint import spec
  (42 files total with the shared import API doc), each with a clear rule owner. Recorded course
  completeness, syllabus colors and tentative Theme mappings, authenticated API import requirements,
  and the current loader's direct database path. The plan's link check passed; spec writing remains
  separate. See [the plan](active_plans/active/question_specs_documentation_plan.md).
- Audited Library Object fields and filters across docs, SQL, API, browser, and Assessment selection.
  Recorded inconsistencies and a proposed doc structure in the
  [audit](active_plans/audits/library_object_documentation_audit_2026_10_05.md); its link passed.
  Full link check had 404 passes and three stale links in an older audit. Static only; no runtime change.
- Documented shared-search boundaries and the three-display rule. HG/checklist match at 1,196; the new
  item remains open pending mixed Library/picker acceptance. Isolated browser probes found no storage
  writes and confirmed late-response ordering.
- M5 review fixed Browse reset leaving its warning armed and Blueprint inputs overlapping results.
  Live checks use semantic locators and the demo's actual 49 Questions.
