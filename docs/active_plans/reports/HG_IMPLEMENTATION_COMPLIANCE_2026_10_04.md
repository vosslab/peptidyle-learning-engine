# Human Guidance implementation compliance audit: 2026-10-04

**The implementation does not yet comply with the revised Human Guidance.** The
highest-impact gaps affect Assessment fairness and automated score visibility.
Several existing tests pass because they protect behavior that HG now rejects.
This is the pre-fix gap report; no production fixes were made during the audit.

Current product authority: [Human Guidance](../../HUMAN_GUIDANCE.md). Reasons and
strength of the interview decisions remain in the
[interview record](../decisions/HUMAN_GUIDANCE_INTERVIEW_FOLLOWUP.md).

## Prioritized findings

Priority here is implementation order, not a count of failed tests.

| Order | Finding | Consequence | Evidence |
| --- | --- | --- | --- |
| 1 | A-04: removing a whole Pool retires its entry but leaves its points in existing Attempts. | Earlier and later Students can receive different score treatment. Both earned and possible points must be excluded from every Attempt. | [Assessment audit](HG_ASSESSMENT_COMPLIANCE_2026_10_04.md#a-04--removing-a-pool-stops-future-delivery-but-does-not-remove-its-score-from-existing-attempts) |
| 2 | A-02 / A-03: post-issue saves permit new/replaced Questions and Pool-member removal without checking whether a member was issued. | The accepted limits on changing issued Assessments are absent at the trusted write boundaries. | [Assessment audit](HG_ASSESSMENT_COMPLIANCE_2026_10_04.md) |
| 3 | A-01: automated scores can still be delayed or hidden. | Students can finish graded work without seeing their Question or Assessment scores. | [Assessment audit](HG_ASSESSMENT_COMPLIANCE_2026_10_04.md#a-01--score-release-controls-still-hide-automated-scores) |
| 4 | A-05: Quiz/Exam answers always wait for all current Students to finish, despite the selected timing. | Instructors cannot use the agreed override when a Student goes AWOL. | [Assessment audit](HG_ASSESSMENT_COMPLIANCE_2026_10_04.md) |
| 5 | HG-ACC-01: account creation requires an internal vetting receipt, lacks the requested name/affiliation fields, and has no setup-email dispatch in the inspected path. | The ordinary Instructor setup path differs from the settled workflow. This is more than label cleanup. | [Access audit](HG_ACCESS_COMPLIANCE_2026_10_04.md) |
| 6 | HG-SRCH-01 / 04: results open in the current tab and save old-result snapshots; search pages lack discard confirmation. | Opening an item replaces the search, and Ribbon navigation can discard it without the requested warning. | [Search audit](HG_SEARCH_COMPLIANCE_2026_10_04.md) |
| 7 | HG-SRCH-03: the assembled search lacks the shared spreadsheet-style exploration/results interface and its required display modes. | Existing shared components do not yet provide the requested search workflow. This finding does not prescribe a new custom page or a component architecture. | [Search audit](HG_SEARCH_COMPLIANCE_2026_10_04.md) |
| 8 | HG-SRCH-02 / 05: Blueprint search lacks Stars, Watches, and most-recent-edit sorts; result metadata omits Institution and Star/Watch signals. | Instructors cannot sort and compare using all the fields HG names. The Author row is present. | [Search audit](HG_SEARCH_COMPLIANCE_2026_10_04.md) |
| 9 | HG-ACC-02 / 04: Instructor Profiles/images are restricted to their owner; Star lists retain the old name-only projection. | Other Accounts, including Students, cannot see the required Instructor Profile/image representation. | [Access audit](HG_ACCESS_COMPLIANCE_2026_10_04.md) |
| 10 | HG-ACC-05: general Question and Pool discussion routes remain implemented. | A rejected forum feature remains part of the product. It must not be relabeled as the accepted Blueprint Change Proposal workflow. | [Access audit](HG_ACCESS_COMPLIANCE_2026_10_04.md) |

The code does **not** establish a separate capability-bearing Verified Instructor
role among admitted Instructors. It does retain an internal vetting workflow,
verified-name fields, and related identity restrictions. Those are the evidenced
problems; the audit does not invent a second role to explain them.

The Blueprint Student count is an aggregate of historical enrollment per adopted
Course Instance. HG does not define a completion test or how withdrawals/repeats
should count. This audit does not require a new completion-tracking system.
Likewise, the inspected search return paths use browser memory/history: their
restoration behavior conflicts with HG, but no database search-storage defect was
demonstrated.

## What already supports the intended behavior

- Current-point recalculation rescales stored credit without asking the Question
  Backend to grade again. The focused unit test passed. This is a useful foundation
  for the fairness fix; retirement alone currently fails to use it correctly.
- The Account model has one immutable role per Account. Instructor deactivation
  and reactivation append state to the same Account without deleting content.
  These statements have fresh source evidence, not new end-to-end certification.
- Question Revision publication checks the current owner. A Native JSON correction
  does not require a new ownership workflow. The source guard was inspected;
  regrading previous submissions remains deferred.
- Blueprint results carry the real author display name. Search has ordinary text
  and classification filters, and shared record components already exist. The
  required combined spreadsheet/mode workflow is still missing.

## Deferred and unsettled boundaries

- Optional Question Feedback timing and Native JSON regrading remain deferred.
- Broader Sysadmin support and System-wide settings remain deferred. HG-ACC-03
  records the existing Instructor-issued support-grant conflict, but does not
  authorize building a replacement support system now. Any future support must
  honor Sysadmin authority, task scope, audit, and Course-membership boundaries.
- Complete standalone Question removal after issue remains tentative. Whole-Pool
  removal and the fair score effect of any complete removal are settled.
- Question Change Proposal terminology is settled; a complete Question proposal
  workflow was not selected in this interview. General Pool discussions are not
  an accepted substitute. Blueprint Change Proposals remain in scope.

## Source reconciliation and checklist status

The current HG contains **1177** checklist bullets, compared with **1146** in the
old audit. There are **81 added or reworded occurrences** and **50 removed or
reworded occurrences**. These are text changes, not 81 separate product decisions.
The [reconciliation record](hg_compliance_2026_10_04/reconciliation.json) preserves
removed text, evidence, and former open questions. Additional
[prior annotations](hg_compliance_2026_10_04/prior_annotations.json) retain the old
audit's reasons and qualifications as history, not current decisions.

HG SHA-256: `7e6676929036d0408259a2f8fb9e2a4bf4daf4b6027d13431b319a05d9ed3519`.
Git HEAD: `470498665acad40875c8fed986a1fc03deb67329`; the audit reads the working
tree, including changes already present at audit start. HEAD alone does not identify
that tree. Existing unrelated implementation edits were preserved.

The [living checklist](../audits/human_guidance_implementation_checklist.md) and all
nine source parts now follow current HG text and order. Earlier verification marks
were reopened, retaining previous evidence with a stale-evidence label. Only fresh,
named evidence restores a mark. A pending row is **not necessarily a defect**;
this prevents the old audit's 1065 verified rows from becoming a false current claim.

Current marks: **14 verified**, **1090 pending or mismatched**, and **73 N/A**.
The verified rows name whether their fresh evidence is source or test based.
The 1090 total is not a defect count: most are earlier claims deliberately left
uncertified in this focused pass. There are 34 checklist occurrences linked to confirmed mismatches; repeated HG
wording means this is not a count of distinct defects. The prioritized findings
above group the actual work.

This pass reviews the priority areas agreed before the audit. It does not repeat
all historic security, retention, backend, responsive-layout, or accessibility
checks across the full 1177-bullet inventory. Those claims remain pending fresh
validation, rather than silently inherited.

## Validation

| Check | Observed result | Limit |
| --- | --- | --- |
| HG checklist text/order, duplicate consistency, evidence locators, nine part gates | Passed | Checks audit integrity, not product behavior. |
| Guidance format and Markdown links | 381 passed | Documentation checks only. |
| Domain score-disclosure tests | 16 passed | Protect the old configurable timing; these are evidence of A-01, not HG compliance. |
| Current-point recalculation unit test | 1 passed | Proves stored-credit scaling; does not test Pool removal. |
| Assessment editor/progress/history Node tests | 13 passed | Include the obsolete "Score not released" behavior. |
| Account/discussion decoder and image-crop Node tests | 9 passed | Do not test setup email or cross-account Profile access. |
| Search/Blueprint Node tests | 36 passed | Include rejected return restoration and Institution omission. |
| Disposable PostgreSQL saved-response lifecycle E2E | Passed | Normal save/finalization/grading and Pool construction; not the new fairness boundaries. |

The [checklist validation log](hg_compliance_2026_10_04/checklist_validation.log) and
[documentation validation log](hg_compliance_2026_10_04/documentation_validation.log)
record the final audit-integrity checks.

A fresh disposable PostgreSQL probe reproduced A-04 through the normal save API:
an Attempt containing one one-point fixed Question and one one-point Pool stayed
at **2/2 after Pool removal**, rather than the required **1/1**. This is direct
runtime evidence of the defect, not a passing compliance result.

No live browser flow was run. The required local Live Demo control receipt was
absent; the audit did not start or rebuild the stack. Browser interaction, email
delivery, cross-account image delivery, and the full Unrelease lifecycle remain
without fresh end-to-end proof here. A passing old-behavior test must be replaced or revised when the accepted
behavior is implemented; it cannot be used to waive the mismatch.

## Delivery recommendation

Fix Assessment fairness, immediate score visibility, and the correct-answer
override first. Reuse current-point
recalculation and enforce the accepted edit limits at server/database boundaries.
Validate two Students whose Attempts started before and after the edit, including
an in-progress Attempt and a removed Pool. Verify both earned and possible points.

Then address onboarding and the shared search/navigation work, with focused browser
checks. Keep deferred features out of these fixes. Revisit the remaining pending
checklist rows by coherent behavior area; do not treat a large inherited test count
or a successful checklist-format gate as release acceptance.
