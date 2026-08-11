# WP-S2 J5 and J8 HCI review

## Acceptance addendum

The required runner integration and independent replay subsequently passed on
2026-08-11. The corrected local no-email pilot now accepts this J5/J8 boundary:
J5 visibly confirms Best `100%`, Latest `100%`, Completed `2`, and two
completed history rows after the two keyboard student runs; J8 contributes only
its closed cross-actor vocabulary to the schema-v2 report. Email/canonical
onboarding, J6/J7, all-family, multi-learner, and release work remain outside
this acceptance.

## Scope and method

Independent read-only cognitive walkthrough of the proposed J5 instructor
tail and J8 public cross-actor binding. The scenario is a local instructor
checking the exact corpus-backed assignment after a learner has completed two
keyboard-only Mastery runs. The review inspected visible-navigation selectors,
keyboard controls, receipt projection, and the focused policy gates. No
Podman stack, application code, database, or retained volume was changed.

## Verdict

**ACCEPTED OFFLINE.** The isolated J5/J8 source tail now meets its interaction,
privacy, and receipt-order requirements. This is not a live or final-report
acceptance: the next integration work must wire J5, J8, and the schema-v2
report child into the retained-stack runner.

The prior receipt-order blocker is repaired. J5 creates its closed evidence
only after all visible assertions, then calls
`closeThenAppendV2J5State(..., () => context.close())`. That helper awaits
successful context closure before it can append. Its new regression proves a
close rejection leaves the exact J11--J4 protected prefix untouched.

## Accepted interaction findings pending repair

- J5 starts from the allowed root entry, signs in through the labelled local
  form, opens the exact public course link, and binds the exact assignment via
  its rendered course-card heading before navigating to Gradebook. It does not
  use an old page, route shortcut, API, storage, cookie, history, or pointer
  action.
- Visible pagination is used both for assignment cards and gradebook rows. The
  row is scoped by the card-derived exact heading, then proves only its Best
  `100%`, Latest `100%`, and Completed `2` cells. Its expanded, controlled
  history region has exactly two ordered entries: `Run 1: Completed` and
  `Run 2: Completed`.
- The keyboard path is Tab/Enter only in J5. The narrow source scanner allows
  precisely those row-scoped score and history assertions; it rejects generic
  title, identity, answer, feedback, body-text, and private-browser access.
- J8 is not a browser action. It consumes the already validated J11--J5
  public-ID prefix, verifies course/assignment binding, and appends only
  course ID, assignment ID, bounded elapsed time, fixed codes, and empty
  diagnostics. The public renderer removes both IDs and emits only J8's closed
  cross-actor vocabulary, so it exposes no score, learner identity, title,
  response, run detail, credential, or email.
- The J8 codes align across the append state and public renderer:
  `visible_instructor_gradebook`, `visible_learner_completion`, and
  `visible_shared_assignment`. They describe the binding of independently
  visible earlier evidence, rather than claiming a new browser interaction.

## Focused evidence

| Check | Result |
| --- | --- |
| `python3 -m pytest tests/test_ui_walkthrough_harness_independence.py tests/test_ui_walkthrough_runner.py -q` | PASS: 46 tests, 6 subtests |
| focused J5/J8/report Playwright simulator specs | PASS: 15 tests |
| `npx tsc --noEmit` | PASS |
| focused Prettier check | PASS |
| `git diff --check` | PASS |

## Historical next step

At the time of this review, the next step was to wire J5, J8, and the
schema-v2 report child into the runner in fixed order, then run the retained
stack and independent replay. That integration and replay are now accepted by
the addendum above. Email remains outside this walkthrough.
