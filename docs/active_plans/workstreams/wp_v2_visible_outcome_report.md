# WP-V2 visible outcome report

## Status

**ACCEPTED HISTORICAL EVIDENCE; SUPERSEDED AS THE ACTIVE BASELINE.** The
manager's successful M4 run and independent artifact inspection remain accepted
in the [WP-V2 review](../audits/wp_v2_visible_outcome_report_review.md). This
schema-v1 report records only J1 and does not claim later coverage. The
2026-08-11 accepted corrected local no-email pilot uses the separate schema-v2
baseline with J11/J12/J13/J1/J2/J3/J4/J5/J8; it does not rewrite this history.

## Scope

- `tests/playwright/simulator/visible_outcome_report.ts` owns typed validation, public-ID ordering,
  canonical rendering, bounded diagnostics, and the PASS/BLOCKED/NOT_APPLICABLE/FAIL vocabulary.
- J1 currently accepts only PASS or FAIL. PASS requires all five visible milestone codes and no
  diagnostics. FAIL requires one fixed redacted diagnostic and cannot carry the full PASS evidence.
- The Python runner remains the sole mode-0600 atomic report writer. It creates one mode-0700
  system-temporary state directory outside `test-results`, passes the fixed path only to fixed
  children, and removes that directory on success and failure.
- The runner preserves top-level `PASS` or `FAIL` and `stage: complete` compatibility. Future
  BLOCKED and NOT_APPLICABLE values belong to later journey rows, not process status.

## Evidence

- Pure Node tests pin canonical ordering, redaction, duplicate rejection, and J1 outcome rules.
- Focused Python tests pin private-state permissions, cleanup, bounded renderer output, and report
  compatibility. The manager and independent live evidence used
  `bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42`.
- The accepted report contains one J1 PASS row with exactly the five visible outcome codes, five
  separate public arrangement rows, an IPv4-only local-origin boundary, and no answer,
  score, credential, feedback body, trace, screenshot, video, or copied page context.
- The report directory is mode 0700 and its atomic report is mode 0600; no temporary state or
  selected Podman container remains after normal no-volume cleanup. Python AUTO reuses safe built
  output when present and builds only when it is absent.
