# Active-plan cleanup

The user requested archival of old planning content and complete manager/subagent execution paths
without human-dependent milestones. Classification used current authority, status, later ledgers,
source/Changelog evidence, creation dates, and independent agent assessments.

## Disposition

- Moved 215 historical files from `docs/active_plans/` to `docs/archive/`, preserving plans,
  audits, decision history, reports, and associated visual artifacts. Used `git mv` for 214 tracked
  files and a filesystem rename for the untracked QPM audit. Existing implementation changes and
  unrelated Git index entries were preserved.
- Retained six existing files: two active UI plans and four audit/checklist inputs with open work.
  Replaced the stale release plan with a concise
  [release_readiness_followup.md](../../active_plans/active/release_readiness_followup.md) and added
  [current_work.md](../../active_plans/active/current_work.md). Final independent review identified
  the retained checklist's missing execution owner; added
  [human_guidance_reconciliation_plan.md](../../active_plans/active/human_guidance_reconciliation_plan.md)
  to carry its current gaps through repair and verification. The active tree now contains nine files.
- Archived Question-spec, shared-search, record/frame, fast-UI, theme, and legacy UI work with
  their supporting reports. The iMathAS proposal is archived as deferred under Human Guidance,
  not as implemented. Interview records and unresolved historical findings are preserved.
- Corrected the QPM completion path after independent assessment: current readable formats are
  offered, while writer-only Canvas/Blackboard QTI engines stay unavailable and return the explicit
  unsupported result. A fresh actual-WASM probe verified both real exported package types and
  all 18 artifact hashes. The original unmet successful-QTI-import expectation remains recorded;
  no new reader is claimed. See [vendor_qpm_plan.md](../vendor_qpm_plan.md).
- Removed the retained plans' human sign-off/setup dependencies. Agent-run baseline repair,
  browser behavior tests, disposable services, and independent visual/architecture assessment
  provide a complete local path. The setup-email checklist now names a local SMTP acceptance
  task rather than waiting for a human email account. Its unchecked status remains honest.

## Review judgments

One classification reviewer suggested keeping the iMathAS decision and missing-destination plan.
Current Human Guidance defers iMathAS, while current My Questions/Starred source and the September
29 receipts establish those destinations. They were archived with explicit dispositions. The old
release plan's unverified current outcomes remain owned by the new follow-up, rather than being
silently closed. Open usability audits remain active.

The QPM assessor suggested a second permanent Blackboard journey and another full runtime replay.
The approved plan retains one permanent seven-kind journey; other focused cases are implementation
evidence. No browser code or vendored bytes changed in this documentation cleanup. The existing
live receipts, fresh capability/hash proof, and independent assessment support the corrected scope
record without expanding the permanent suite or relabeling old runtime evidence as fresh.

## Verification

- All 557 Markdown-link checks passed after the final plan addition.
- Both generated-checklist checks passed with 1,248 Human Guidance and checklist bullets.
- Relocation integrity passed for all 215 moved files; all 10 moved visual artifacts match their
  original tracked bytes. The active inventory contains nine files.
- Repository hygiene checks passed: 6,032 checks with 10 existing source-length advisories.
  Working-tree and staged diff whitespace checks passed.
- Fresh independent closeout review found and resolved the checklist execution-owner gap. A
  different fresh reviewer accepted the resulting agent-owned repair, verification, and closeout
  path without human dependencies.

The move receipt and command logs are kept under ignored `test-results/active-plan-cleanup/`.
These documentation checks do not renew historical runtime claims or execute the four remaining
plans.
