# M3 arrangement integration

## Status

**Independently ACCEPTED.** M3 has accepted supported-API arrangement evidence;
it does not claim a keyboard journey, scoring, enrollment simulation, canonical
onboarding, all-family coverage, or release acceptance. See the independent
[M3 review](../audits/m3_arrangement_integration_review.md).

## Scope

- The Python runner starts the accepted local stack, then invokes only the fixed
  `tests/e2e/ui_walkthrough_arrange.ts` child through repository-local `tsx`.
- The child validates private mode-0600 login and launcher-manifest files,
  uses isolated instructor API contexts for WP-A1 then WP-A2, and emits one
  bounded public-ID object with separate launcher enrollment, launcher
  baseline, API corpus, API Mastery, and API Exam records.
- The runner validates that object before passing only the course and arranged
  assignment UUIDs plus the credential-file path to fixed Playwright specs.
- The browser signs in through the rendered local credential form and opens
  the visible course, Mastery, and Exam cards. It does not inject cookies,
  call browser APIs for arrangement, or use a direct route shortcut.

## Evidence

- `source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py -q`
  passes 20 focused offline runner checks.
- `npx playwright test tests/playwright/ui_walkthrough_live_config.spec.ts`
  validates opt-in live inputs and the action-time credential reader offline.
- `npx tsc --noEmit`, Prettier checks, and `git diff --check` pass.

## Live acceptance history

- The first live arrangement run passed and wrote five safe arrangement records.
  A retained-volume replay then failed closed when old and new Mastery cards
  shared a title. The fixed selector binds each current card's visible
  `Review assignment` link to its validated course and assignment ID.
- After that repair, two manager runs of the exact Python-backed command with
  `--master-seed 42` passed consecutively. The same seed and stable labels
  replayed; new supported-API UUIDs correctly differed between runs.
- An independent elevated run of the same command also passed. Each reviewed
  report was private (directory mode 0700, file mode 0600), redacted, and held
  only the five public arrangement records. No SQL, account, or enrollment
  fixture was used; Assignment Arrangement creation used the existing Student
  Course Membership.
- The normal runner cleanup is no-volume and left the selected project empty.
  During the independent check, a no-volume project `down --remove-orphans`
  raced normal shutdown and reported already-removed resources; the two prior
  clean manager passes are the unambiguous automatic-cleanup evidence.

## Limits

- The browser opened visible course, Mastery, and Exam cards after rendered
  local sign-in, but this arrangement evidence is not J1 or later journey,
  response, retry, score, or enrollment-simulation coverage. Live local
  origins remain IPv4 `127.0.0.1` or `localhost`; no IPv6 route is claimed.
