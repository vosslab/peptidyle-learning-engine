# Changelog

> **Historical implementation evidence.** Changelog entries preserve what was changed and believed
> at the time. They are not product authority. Current intent comes from [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md), which supersedes old Assignment,
> Blueprint, lifecycle, grading, role, retention, and UI models.

> October 11 entries are archived in [CHANGELOG-2026-10e.md](CHANGELOG-2026-10e.md).
> September 29 entries are archived in [CHANGELOG-2026-09r.md](CHANGELOG-2026-09r.md).
> September 28 entries are archived in [CHANGELOG-2026-09s.md](CHANGELOG-2026-09s.md).

## 2026-10-13

### Behavior or Interface Changes

- Implemented WeBWorK autosave and save before Question navigation, leaving an Attempt, and Submit.
  Save failures retain editable input and expose Retry; successful saves proceed quietly. Accepted
  saves use an atomic revision check so delayed older requests cannot replace newer work. Save
  acknowledgements preserve the mounted Question, and a lost submission acknowledgement triggers
  an authoritative progress check.
- Added completed Question-and-recorded-response review independently of correct-answer disclosure.
  WeBWorK replays the retained source and seed, with retained prompt context when replay is unavailable.
  Native review includes original prompts, choices, and retained isolated author content. Removed
  Instructor timing controls for the Student's own response and correctness while preserving optional
  answer and teaching-feedback settings. The complete repository gate passed with 9,090 pytest
  cases and fresh PostgreSQL acceptance; representative renderer replay, browser fault journeys,
  and all 257 live screenshot captures passed. Inspected the changed Student review views.
- Fixed a readiness race found during full screenshot verification: navigating away from a visible
  WeBWorK form now waits for its bridge's ready message before capture. The existing deadline keeps
  missing messages retryable. A browser regression check verifies that an untouched Question stays
  unanswered when navigation begins before the frame finishes loading.

### Fixes and Maintenance

- Completed six fresh independent Student-work reviews: Plan, Test, Style, Docs, Legacy, and
  Comment. No production defect was confirmed. A suspected first-save race was withdrawn after
  tracing the existing parent-row locks. Added selector-owner references to the browser scenario
  and shortened a source banner; focused formatting passed. One recommendation remains for human
  review: strengthen the existing completed-review scenario with a direct protected-answer-content
  assertion. This audit reused the completed implementation's full-suite and browser evidence.
- Added the [WeBWorK Student work plan](active_plans/decisions/webwork_student_work_plan.md)
  with the selected approach: autosave while working and save before navigation, with recovery
  actions for failed saves and no routine confirmation prompts. Excluded crash recovery at the
  human's direction. Local ADAPT inspection found open-ended text autosave and submitted WeBWorK
  session restoration, but no matching WeBWorK autosave/save-before-navigation path. Recorded the human
  requirements for stable MC/MA/MATCH positions within an Attempt, new seeds only on new Attempts,
  and Question/recorded-response/correctness review even when correct answers are hidden.
  Included a focused FIB typing, immediate-navigation, and clearing check alongside MC/MA/MATCH.
  Grounded acceptance in Student-visible behavior and representative cases; tool-version changes
  alone do not block working replay, and byte/pixel equivalence is not an acceptance requirement.
  Clarified adaptability over speculative edge-case handling, excluded unsaved browser-crash work
  from implementation and acceptance, and removed visibility/shutdown save handling from the plan.
  Documentation checks passed for the planning change; implementation is recorded above.
- Refocused launch readiness on actual Student work in the
  [Student launch audit](active_plans/audits/STUDENT_LAUNCH_READINESS_AUDIT_2026_10_13.md).
  Reproduced WeBWorK answer loss on navigation, native answer loss after leaving a failed save,
  disabled WeBWorK save recovery, and false submission status after a lost acknowledgement;
  recorded native review's missing Question context. Confirmed saved-response restoration, 2X
  accommodations, background deadline submission, and sampled accessibility checks. This is audit
  evidence at the reviewed baseline; implementation and acceptance are recorded above.
- Added the [launch readiness audit](active_plans/audits/LAUNCH_READINESS_AUDIT_2026_10_13.md)
  with six independent review passes, verified hosting/email configuration gaps, first real
  administrator bootstrap requirements, and conditional course-content scope. Classroom production
  launch remains on hold; stale Bloom and grade-export blockers are identified as documentation drift.
- Recorded passing offline gates (including 9,072 pytest cases), a separate fresh PostgreSQL
  acceptance rerun after releasing an orphaned demo lease, all 257 live screenshot captures, and
  actual CSV/TSV grade downloads. Kept tracked screenshots and production code unchanged; dependency
  scans found one informational Rust YAML-parser maintenance advisory and no Node or direct Python
  runtime advisories.

## 2026-10-12

### Behavior or Interface Changes

- Renamed PLE's QPM WASM consumer contract to the project-owned `qpm-wasm` package and
  `qpm_wasm` generated artifacts. The importer and vendor tool now load the producer's QPM
  namespace; QTI remains the name of the assessment interoperability standard. Removed the unused
  QTI runtime environment setting from the browser-test composition.
- Rotated the completed October 6 and October 7 change blocks to
  [CHANGELOG-2026-10e.md](CHANGELOG-2026-10e.md).
