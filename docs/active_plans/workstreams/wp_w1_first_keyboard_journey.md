# WP-W1 first keyboard journey

## Status

**ACCEPTED.** The manager and independent forced-build runs passed the repaired J1 path through
`bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build`. J1 uses the exact visible Mastery
href, bounded Tab/Shift+Tab, native Enter, and Space without a pointer action or answer
reconstruction. J2 is accepted separately in
[wp_w2_retry_until_correct.md](wp_w2_retry_until_correct.md). This workstream claims only J1; it
does not accept later student journeys, scoring verification, or answer-key access.

## Supersession note

The dated record below mentions `PLAYWRIGHT_NO_COPY_PROMPT=1`. It was an unsupported hidden custom
flag and has been removed. Privacy now relies on Playwright's default-off trace, screenshot, and
video recording plus private explicit inputs; this note does not add live acceptance evidence.

## Scope

- `tests/playwright/ui_walkthrough_keyboard_j1.spec.ts` signs in through the rendered local form,
  then uses Tab, Shift+Tab, Enter, and Space to select the arranged course and Mastery assignment.
- The spec observes only rendered focus, visible response readiness, visible feedback, and the
  visible completion state. It never invokes a browser API, clicks or focuses a platform control,
  injects session state, routes directly after login, or reads answer-bearing material.
- A bounded private state fragment records J1 PASS only after the visible start, response, submit,
  feedback, and completion milestones occur.
- The fixed live Playwright child sets `PLAYWRIGHT_NO_COPY_PROMPT=1`; trace, screenshot, and video
  remain off, so a failed visible-feedback state does not persist an AI page snapshot.

## Evidence

- The manager and independent live runs each passed through the Python runner using IPv4 localhost.
  The runner used AUTO build selection and its fixed local stack arrangement.
- J1 used rendered controls only: Tab and Shift+Tab for focus, native Enter for sign-in and links,
  and Space for native buttons and radios. It neither used a pointer action nor reconstructed an
  answer or score.
- The replay fixes are covered by no-pointer source regressions. They await a new clean live replay;
  this document makes no renewed live acceptance claim.
- The final report is mode 0600 in a mode-0700 directory. No trace, screenshot, video, copied page
  context, temporary state, or selected Podman container remained after normal no-volume cleanup.
