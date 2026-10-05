# HG repair six-pass audit — 2026-10-04

All six fresh independent review passes completed: Plan, Test, Style, Docs,
Legacy, and Comment. Each reported at least one finding. The review used the
current HG repair plan and source, not an assumption that every dirty file belongs
to this task. Duplicate findings below are combined. Reviews were read-only.

## Clarification after review

Neil clarified: Students do not get avatars. The successful Student check below
means a signed-in Student could view an Instructor's Profile image. It does not
mean a Student uploaded or received an image. The older HG gallery-avatar rules
for Students conflict with this direction; reconciling those rules and the
implementation remains follow-up work.

## Findings, in priority order

1. **High: active test lanes still invoke removed Instructor vetting.**
   `tests/playwright/test_course_entry_banner.mjs` and its approval harness are
   called by the fast UI launcher; `tests/e2e/e2e_question_star_name_privacy.sh`
   is called by the E2E aggregate. Both retain the deleted vetting endpoint or
   fields. This breaks relevant validation and protects rejected behavior.
   Reuse current Sysadmin creation and retain meaningful account/Star access
   assertions; remove obsolete approval-only checks.
2. **High: current docs contradict settled account and email behavior.**
   Authorization, classification, installation, local-stack, FAQ, and terminology
   docs retain an internal vetted/approved tier. TODO and cookbook text call
   email-code delivery unbuilt. These claims can cause future work to restore
   rejected behavior or duplicate implemented authentication. Describe active
   Instructors and outside-PLE vetting; retain only external provider/inbox
   acceptance as blocked.
3. **Medium: feedback and Pool-notice documentation overstates decisions.**
   `docs/STUDENT_GUIDE.md` chooses optional-feedback timing that HG defers.
   `docs/DESIGN_DECISIONS.md` withdraws Pool administration wording more broadly
   than the unresolved impact-notice authority permits. Remove the feedback
   timing claim and distinguish removed forums from deferred notice authorship.
4. **Medium: screenshot privacy checks reject a valid policy value.**
   `tests/playwright/screenshot_corpus/privacy_profiles.ts` omits `after_due`
   from known answer timing values, incorrectly treating that policy as leaked
   answer content. Add the valid timing and amend the existing boundary test.
5. **Medium: Blueprint sort proof cannot distinguish key behaviors.**
   `crates/learning-data-access/tests/blueprint_course_postgres/discovery.rs`
   uses identical Star/Watch counts on one record; its recent-edit assertion
   does not separate creation order from edit order. Distinct fixtures in the
   existing connected test should make those assertions meaningful.
6. **Medium: retired Instructor-tier wording remains in current source.**
   SQL errors, Rust comments, and TypeScript contract comments retain
   "Verified Instructor" or an internal vetted identity. Actual predicates
   check active Instructor status and a Profile name. Correct wording while
   preserving authorization; historical records remain historical.
7. **Medium: Profile acceptance and evidence need precise limits.**
   The plan promises Question-author/Pool-owner links, while the Profile report explicitly
   omits a link to the Question Pool owner's Profile. Keep that item open. The same report
   still calls the public Profile screenshot deferred despite its published
   manifest/receipt; update this evidence without treating a screenshot as
   cross-account uploaded-image proof.
8. **Medium, follow-up: shared email delivery has a narrow name.**
   `InstructorSetupEmailDelivery` and `PLE_INSTRUCTOR_SETUP_*` also support
   ordinary Student/Instructor sign-in codes. The configuration is documented,
   but the name obscures its broader responsibility. Consider a neutral name
   in a bounded follow-up; no behavior or configuration rename is part of this
   audit cleanup.
9. **Low: the plan overstates spreadsheet architecture completion.**
   This repair proves shared result presentation and navigation. The modular
   spreadsheet backend concern remains a separate architecture review.
10. **Low, advisory: composition is approaching its size limit.**
    `crates/server/src/composition.rs` is about 910 lines, below the 1000-line
    limit. A focused email assembly module may help its next substantive change;
    this audit does not require a refactor solely to satisfy an advisory.

## Evidence and limits

The full Rust gate, TypeScript/lint/format and 536 Node tests, 10219 Python tests,
fresh PostgreSQL schema/fairness proof, and 1177-bullet checklist gates passed
before audit cleanup. Published screenshot evidence contains 247 images.
Account lifecycle and Question Library navigation/forum-removal live checks pass.
Final Quiz/Exam policy replay now passes: actual Quiz controls, absence of a score
delay, saved answer override/API readback, and Exam API defaults. An Instructor could upload and replace a Profile image, and a signed-in Student
could view that Instructor's image. Requests without sign-in were denied, and
cleanup of replaced images passed. A later check failed when a Sysadmin tried to
upload their own Profile image: the server returned "not found" (HTTP 404).
That failure remains unresolved. Real email provider/inbox acceptance awaits the user's
email-account setup.

No new permanent test was proposed. The Test pass recommends amending existing
meaningful boundary tests. The six passes do not constitute a new audit of every
one of the 1177 HG bullets, nor an exhaustive review of unrelated dirty files.

## Cleanup status

The obvious terminology, documentation, stale-test, and existing-test corrections
are complete. The obsolete approval-only browser checks and unused harness were
removed; the Course banner browser check still passes. The test for who can see the names of Instructors who Star a Question now creates
its test Instructor using email, first name, last name, and affiliation. The
script and account-creation request were checked, but the complete test was not
run again against the application and database. That behavior still needs
verification after this change.

The privacy boundary test passes with `after_due` accepted and actual answer
text still rejected. The strengthened Blueprint test passes on fresh isolated
PostgreSQL 17 with different Stars/Watch leaders and a real later edit moving an
older Course ahead of newer ones.
No new permanent test file or test case was added by this cleanup.

After cleanup, TypeScript, lint, formatting, 536 Node tests, schema checks, and
10217 Python tests pass. The full Rust gate passed the final functional repairs;
later Rust changes only corrected terminology and strengthened the existing
connected Blueprint test, which compiled and passed separately. The Python
storage gates initially caught excess build cache; removing obsolete untagged
images and completed old test binaries restored both budgets without deleting
running containers or data.

The remaining follow-ups include:

- Make a Question Pool owner's displayed name link to their Instructor Profile.
- Resolve who can write notices about changes to a Question Pool. Removing
  discussion threads did not settle this separate question.
- Check whether the search backend provides the reusable spreadsheet-style
  interface Neil requested. Shared compact, list, and poster displays alone do
  not establish that.
- Give the shared sign-in email code a name that reflects use by both Students
  and Instructors.
 The composition-size advisory is
not treated as a blocking defect or a reason for an unrelated refactor.

## Search and Pool source check after Neil's clarification

Neil specified that PLE should not become a major messaging platform. Whether
his latest wording also removes the previously requested Stars and Watches is
awaiting clarification. No removal of those features is assumed. He also
specified that Question Pools are designed to be forked often and that the
Instructor who owns a Pool is its owner.

The search implementation is only partly shared:

- `src/pages/library_browse_rows.tsx` and
  `src/pages/blueprint_course_search_page.tsx` use the shared `RecordList` and
  `SearchResultDisplay` controls for compact, list, and poster displays.
- `src/pages/library_pool_discovery.tsx` uses `RecordList`, but does not use that
  display selector. It also opens Pool inspection within the page.
- Question, Pool, and Blueprint searches each manage their own filters,
  requests, and paging. Shared classification and paging controls exist.
- `src/components/record_list/record_table.tsx` provides named table columns,
  but none of these three search pages uses it. It does not provide a shared
  column-filter system.

Conclusion: reusable display components exist, but the requested shared
spreadsheet-style search is unfinished. Separate domain queries are reasonable;
the gap is the shared interface for searching, filtering, and comparing results.
This is source inspection, not a new browser or performance test.

Pool forking is already explicit in HG and implemented by
`schemas/base_schema/50_functions/assessment_pool_forks.sql`, which calls
`ple_data.fork_question_pool` when importing a Pool. The common fork function in
`question_pools.sql` creates a new Pool ID at Edit Number 1 and copies the exact
member Question Revisions. Editing the fork therefore has a separate member
list. No new runtime check was performed here.

The current Pool table has no explicit owner Account field. It records the
Instructor who attested that the member Questions are interchangeable and keeps
separate authorship records. Those facts should not silently be called ownership.
The owner's identity needs checking before adding the requested Profile link.

The retained `crates/server/src/library_discussion.rs` routes allow written
notices about Questions and Pools to be created, edited, and cancelled. They are
more than a Star button or Watch subscription. Their necessity needs review
against Neil's instruction to keep PLE out of the messaging-platform business.

### Earlier implementation remains in place

The September 25 implementation was not dropped. Commit `967a842b` records
completion of the shared record/page framework and database search work. The
current standardization ledger marks those packages complete. Current code
retains shared record layouts, page layout, sort controls, and paging controls.
`crates/learning-data-access/src/postgres/question_library/search.rs` still sends
filters, sorting, and a page limit to `ple_api.search_question_library_entries`.
That backend work is present; the earlier description of "only row display"
understated it.

The original plan deliberately left query state and domain-specific behavior
with pages. The missing additional reuse is the search-page form and coordination
of filters, requests, and results. Neil clarified that shared rows with custom
pages are acceptable, though further page reuse makes sense. Treat that as an
improvement to the existing implementation, not evidence that the two-day effort
was discarded or a reason to replace its working components.
