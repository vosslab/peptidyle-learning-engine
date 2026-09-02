# WP-W2 retry until correct

## Status

**ACCEPTED.** WP-W2 covers J2 only: an existing seeded student visibly retries the
arranged Mastery work through keyboard controls. It does not accept M5 as a whole,
J3-J5/J8, onboarding, all-family coverage, scoring reconstruction, or answer-key access.

## Delivered path

- J2 opens the exact visible Mastery href through Tab/Shift+Tab and native Enter.
- It uses the rendered Start/resume result: visible radios proceed directly, while a visible
  `Start another practice Assignment Attempt` button is reached by Tab and activated with Space.
- The private native retry lifecycle now issues an immutable server-authorized successor attempt;
  the browser refreshes the current screen and remounts response state for that successor.
- J2 selects exactly two visible unchecked radios through bounded native forward/reverse tab entry,
  uses Space and the visible Submit button, waits only for rendered retry/final states, and never
  reads correctness, feedback body, answer text, score, source, storage, or an API response.
- Python owns the mode-0700 private state root and mode-0600 report. J2 appends a bounded public
  fragment through a no-follow descriptor; private Playwright artifacts are confined to a sibling
  directory and removed with the state root.

## Acceptance evidence

- Manager and independent fresh-build runs passed:

  ```bash
  bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build
  ```

- The redacted schema-v1 report has ordered J1/J2 PASS rows, five public arrangement records, empty
  diagnostics, a mode-0700 parent, and a mode-0600 report. Cleanup left no selected Podman
  container, runner private-state directory, trace, screenshot, video, or error-context artifact.
- [Native retry lifecycle review](../audits/wp_w2_native_retry_lifecycle_review.md),
  [frontend refresh review](../audits/wp_w2_frontend_refresh_review.md),
  [response lifecycle review](../audits/wp_w2_response_lifecycle_review.md), and
  [receipt-authoritative completion review](../audits/wp_w2_receipt_authoritative_completion_review.md)
  accept the server and frontend lifecycle repairs.
- The independent [HCI review](../audits/wp_w2_retry_until_correct_hci_review.md) and
  [report review](../audits/wp_w2_retry_until_correct_report_review.md) accept the keyboard path,
  redacted report, private-state boundary, and forced-build cleanup inspection.

## Retained-volume diagnosis

- Earlier retained-volume replays exposed stale completion, delayed retry controls, native radio
  group focus order, summary refresh latency, and historical summary headings. The final path waits
  only on visible controls and semantic headings, uses bounded native tab navigation, and fails
  closed for rendered retry, error, feedback, neutral, closed, or timeout states.
