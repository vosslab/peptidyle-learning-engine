# WP-I4 instructor setup HCI review

## Acceptance addendum

The handoff integration that was pending in this review subsequently passed in
the corrected local no-email pilot. Manager and independent seed-42
retained-stack `--build` runs used the visible J11/J12/J13 setup before the
student keyboard and instructor-gradebook journeys, then emitted the canonical
schema-v2 report. This accepts the bounded local setup only, not email,
canonical onboarding, J6/J7, all-family, multi-student, or release work.

## Scope and verdict

Independently reviewed WP-I4 against the historical walkthrough plan. This was a
read-only HCI, SolidJS-surface, and TypeScript evidence review of the J11/J12/J13
specification, protected state, runner/configuration boundary, the relevant
instructor pages, and focused offline gates. No implementation, plan, changelog,
index, Podman machine, or live stack was changed.

**CHANGES REQUIRED.** The underlying product surfaces have labelled native
controls, visible success/error states, focused course/roster outcomes, and a
public-only protected-state schema. The proposed J11/J12/J13 evidence does not
prove the required keyboard-only instructor journey, is not protected by the
walkthrough source scanner, persists partial state before the full visible
sequence completes, and currently fails strict TypeScript compilation.

## Task model

The user is a local instructor preparing one real course before a remote student
works in it. Completion means the instructor can, without a pointer or hidden
state access, sign in locally; create and open a uniquely named course; open
Students and visibly activate the configured student; return through a rendered
control; create a Mastery assignment from the public catalog; and observe its
real course assignment link. The only retained handoff is the exact public
course, assignment, problem, and version IDs after this whole sequence has
passed.

The review used a cognitive walkthrough: each transition needs a labelled,
visible target; native Tab/Shift+Tab and Enter or Space activation; an asserted
focused target; a visible completion or recovery state; and a bounded timeout.

## Findings

### P1 - J11/J12/J13 is pointer-driven, not keyboard-only evidence

- **Location:** `tests/playwright/ui_walkthrough_instructor_setup.spec.ts:57-120`.
- **Evidence:** The test uses `.click()` for local sign-in, Create course,
  Open course, Students, Add active student, New assignment, Search catalog,
  Add published version, and Create assignment. It also uses `page.goBack()` at
  line 103. There are no `page.keyboard.press()` calls, no `tabTo` helper, and
  only the post-create link and roster row receive focus assertions.
- **Impact:** A passing child would prove that the rendered controls exist, but
  it would not prove that an instructor with no pointer can discover, focus, and
  activate them. Browser history is also a prohibited navigation shortcut after
  the allowed root entry.
- **Required repair:** Follow the established J5 pattern. Start only with
  `page.goto("/")`; use the visible labelled credential input, then reach each
  link/button with `tabTo`, assert focus, and activate it with native Enter or
  Space. Replace `goBack()` with a rendered course-management route/link reached
  in that same way. Preserve ordinary text-value setup only at its visible
  labelled input boundary. Assert focus on the destination main content or the
  next meaningful labelled target after every route/change.

### P1 - The permanent scanner omits the instructor setup child

- **Location:** `tests/test_ui_walkthrough_harness_independence.py:25-39,149-159`.
- **Evidence:** `harness_sources()` includes only
  `ui_walkthrough_keyboard_j*.spec.ts`, simulator sources, runner sources, and
  live config. It does not include
  `tests/playwright/ui_walkthrough_instructor_setup.spec.ts`. The policy test
  therefore passed despite the direct click and history operations above.
- **Impact:** Pointer, direct-route/history, request/session/storage, answer,
  score, feedback, or private-identity shortcuts can regress in the prerequisite
  child without a permanent offline failure.
- **Required repair:** Add the fixed instructor setup spec to the scanner's
  owned sources and apply the same root-only navigation, native-key allowlist,
  no-pointer, no-direct-focus, no-request/API/session/storage/cookie, and
  no-answer/score/feedback/private-identity-read checks as the platform
  journeys. Add hostile fixtures that fail for `.click()`, `goBack()`,
  `context.request`, storage/cookie access, and body/private text reads.

### P1 - State is appended before all visible assertions pass

- **Location:** `tests/playwright/ui_walkthrough_instructor_setup.spec.ts:80,
  100,139` and plan acceptance criterion 4.
- **Evidence:** J11 is appended immediately after course creation/opening and
  J12 immediately after roster activation. J13 catalog, policy, and real-link
  assertions can still fail after those writes. The plan requires the spec to
  append public IDs only after all visible assertions pass.
- **Impact:** The retained state may claim an incomplete instructor setup and
  violates the all-or-nothing evidence boundary.
- **Required repair:** Keep validated J11/J12/J13 fragments in memory and make
  one protected append/commit only after the J13 success-link assertion. Retain
  exact ordering and reject any partial prefix at the runner handoff boundary.
  Add a focused failure test that causes a post-J12 assertion failure and proves
  the state file remains empty.

### P1 - Strict TypeScript gate is red

- **Location:** `tests/playwright/ui_walkthrough_instructor_setup.spec.ts:40`.
- **Evidence:** `npx tsc --noEmit -p tsconfig.lint.json` reports TS2322:
  `string | undefined` is returned where `string` is required. The length test
  does not narrow indexed RegExp-match access under
  `noUncheckedIndexedAccess`.
- **Impact:** The review target does not meet the repository's mandatory strict
  type gate, and a malformed visible reference lacks a compiler-enforced
  fail-closed extraction path.
- **Required repair:** Narrow each match into a local checked string before
  return (or use one checked parser returning the exact public pair), then rerun
  both root and lint TypeScript configurations.

### P2 - J12/J13 targets are ambiguous and lack recovery evidence

- **Location:** `tests/playwright/ui_walkthrough_instructor_setup.spec.ts:66,
  89,110-123`.
- **Evidence:** The course link and catalog row use `.first()`. The roster
  assertion identifies a generic visible `Local pilot` cell rather than the
  alias entered in the labelled J12 form. The child asserts normal success only;
  it does not prove the labelled recovery/error states that preserve the course
  title, student alias, or assignment draft, nor does it set child-specific
  timeouts beyond Playwright's global 30 seconds.
- **Impact:** A changed ordering or preexisting local row can select the wrong
  resource, and a failure gives limited instructor-oriented recovery evidence.
- **Required repair:** Use exact unique, scoped visible targets (the entered
  course title; the entered configured alias or a non-identifying active-row
  relationship; and an exact selected public catalog item). Require count one
  before keyboard activation. Add focused component-level recovery tests for
  create course, local roster activation, catalog search/create assignment, and
  document bounded live-child timeouts with the user-facing failure/retry
  checkpoint.

## Confirmed boundaries

- The child starts at `/`; it does not use `context.request`, `page.request`,
  storage-state injection, cookie operations, `page.evaluate`, direct focus,
  or a non-root `goto`.
- Local instructor credentials and the configured student alias are read from
  regular non-symlink mode-0600 files at their visible form action boundaries.
  They are not serialized into `InstructorSetupFragment`.
- The runner's instructor-only arrangement publishes only the public retry
  corpus and rejects an unexpected arrangement shape. It creates a 0700 private
  directory and 0600 state/alias files, and invokes only the fixed instructor
  spec.
- `InstructorSetupFragment` is a closed schema with only fixed journey/status
  codes, elapsed time, and public UUIDs. Its reader rejects extra private
  fields, reordered fragments, unsafe state metadata, and symlinks.
- Product source inspection confirms labelled Course title, Configured student
  alias, Assignment title, and public catalog controls; it also confirms
  visible product recovery copy for course creation, roster activation, and
  catalog failures. This does not substitute for the missing keyboard journey
  evidence.

## Validation

| Check | Result |
| --- | --- |
| `python3 -m pytest -q tests/test_ui_walkthrough_harness_independence.py tests/test_ui_walkthrough_runner.py` | PASS: 36 tests plus 3 subtests. The pass exposes the scanner-coverage gap; it does not validate this omitted child. |
| Targeted ESLint | PASS with no diagnostics. |
| Targeted Prettier | PASS: all matched files use Prettier code style. |
| `npx tsc --noEmit -p tsconfig.lint.json` | FAIL: TS2322 at instructor setup line 40. |
| Focused offline Playwright course/editor/config/state/instructor setup selection | INCONCLUSIVE for browser bodies: Chromium failed before test bodies with macOS Mach-port permission denied. Eight config/state tests passed and the instructor live-only test skipped as designed; ten component browser tests could not launch. No Podman or live stack was started. |

## Exact live checklist

Run only after the four P1 repairs and focused offline gates pass.

- Start the fixed retained-stack instructor child; do not use an API arrangement
  except the named public corpus publication.
- Confirm `podman ps` is empty before the run, then use the normal fixed runner
  with `--build --instructor-setup-only`.
- Observe J11 start at `/`, reach the visible local credential control by Tab,
  enter the configured instructor credential, Tab to the rendered sign-in
  button, and activate with Enter. Create a unique course through its labelled
  form using keyboard controls only; require the unique visible Open course
  link, focus it, and activate it with Enter.
- Confirm the opened course surface/main target receives focus. Reach the exact
  rendered Students link by Tab and activate it with Enter; do not use browser
  history, direct navigation, pointer action, request context, direct focus,
  storage, or cookies.
- In J12, reach the labelled configured-student field by Tab, supply only the
  configured local alias, focus and activate Add active student with Enter or
  Space, and require exactly one matching visible active student outcome with
  an announced completion/recovery state.
- Return through a rendered visible control, not history. Reach New assignment
  through Tab/Enter, fill the labelled assignment title/search fields, and use
  Tab plus native Enter/Space for search, exact public catalog selection, and
  Create assignment. Verify the visible Mastery values (All questions correct,
  Highest run score, Allow unlimited practice) and the exact real course
  assignment link.
- Confirm no test source or trace reads an answer, feedback body, score,
  student identity beyond the configured visible input/action boundary, or a
  private browser/session value. Confirm stdout/stderr and credentials are
  discarded.
- After every J11/J12/J13 visible assertion passes, inspect the protected state
  reader/output: it contains one ordered all-or-nothing prefix with only exact
  public course, assignment, problem, and version UUIDs, fixed codes, elapsed
  values, and empty diagnostics.
- Confirm normal cleanup removes the selected containers and private temporary
  runner state; `podman ps` is empty after the run.

This review does not accept a live instructor setup run, M10, or the complete
instructor-to-student walkthrough.

## Re-review - 2026-08-11

**ACCEPTED OFFLINE.** The repaired WP-I4 child closes all four P1 findings from
this review. This verdict accepts the static keyboard-evidence, scanner,
protected-state, configuration, and focused offline-test boundaries only. It
does not accept a retained-stack run, M10, or the end-to-end teaching loop.

### Accepted repairs

- J11/J12/J13 now starts only at `/`. The instructor reaches the labelled local
  credential field, Course title, Students, configured student field, Back to
  course, New assignment, search, catalog action, and create action with
  `tabTo`, asserts focus, and activates native controls with Enter. The
  rendered Back to course link replaces the prior browser-history shortcut.
- The unique course card scopes Open course and requires one match. Students,
  Back to course, and New assignment each require exactly one visible rendered
  link before keyboard activation. The created assignment link is visible
  before the evidence is retained. The global Playwright timeout remains a
  bounded 30 seconds.
- The harness scanner now owns
  `ui_walkthrough_instructor_setup.spec.ts` and applies the platform keyboard
  rule set. Its hostile probe rejects pointer click, history navigation,
  request/session/cookie shortcuts, storage access, body-text reads, and DOM
  evaluation.
- J11, J12, and J13 fragments remain in memory until all J13 catalog, Mastery
  policy, and visible assignment-link assertions complete. One closed
  `commitInstructorSetupState` call then validates and writes the ordered
  complete public-ID prefix. The focused post-J12-failure test proves that no
  write occurs before this boundary.
- Public catalog IDs are now read from scoped rendered public attributes and
  checked as UUIDs. Both TypeScript configurations compile with strict indexed
  access enabled.

### Re-review validation

| Check | Result |
| --- | --- |
| `npx tsc --noEmit -p tsconfig.json` and `npx tsc --noEmit -p tsconfig.lint.json` | PASS |
| `python3 -m pytest -q tests/test_ui_walkthrough_harness_independence.py tests/test_ui_walkthrough_runner.py` | PASS: 38 tests plus 3 subtests |
| Focused ESLint, Prettier, and `git diff --check` | PASS |
| `PW_PORT=4341 npx playwright test tests/playwright/ui_walkthrough_live_config.spec.ts tests/playwright/simulator/instructor_setup_state.spec.ts tests/playwright/ui_walkthrough_instructor_setup.spec.ts --reporter=line` | PASS: 10 passed, 1 intended live-only skip |

### Updated live checklist

- Run the normal fixed `--build --instructor-setup-only` child with an empty
  Podman container list before it begins.
- Observe only Tab, Shift+Tab if needed, and native Enter/Space from root login
  through unique course creation/opening, Students, configured local student
  activation, rendered Back to course, public catalog selection, Mastery
  policy verification, assignment creation, and its real course link.
- Confirm every route change has a visible/focused destination and no pointer,
  history, direct route, direct focus, request/API, storage, cookie, answer,
  score, feedback, or private-identity shortcut occurs.
- Confirm the sole post-J13 state commit contains only the exact ordered public
  course, assignment, problem, and version UUIDs, fixed visible codes, elapsed
  time, and empty diagnostics; credentials and child stdout/stderr remain
  absent.
- Confirm normal cleanup leaves `podman ps` empty and removes private runner
  state.

## Live re-review - 2026-08-11

**LIVE HCI SLICE PASSED; WP-I4 HANDOFF INTEGRATION BLOCKED.** I independently
ran exactly:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build --instructor-setup-only
```

The live child completed successfully and normal cleanup left no containers,
private temporary state, or Playwright artifact directory. The refreshed public
report is PASS for seed 42, has one public corpus-publication arrangement, and
is mode 0600 in a mode-0700 directory. This is credible live evidence that the
fixed browser child, local stack lifecycle, and cleanup contract work together.

The WP-I4 acceptance criterion is nevertheless not met yet. In
`tests/e2e/e2e_ui_walkthrough.py`, the instructor-only branch runs the fixed
Playwright specification and returns immediately. It does not call
`readInstructorSetupPrefix`, validate the complete J11/J12/J13 handoff, extract
the public IDs for fixed later children, or render those journeys into the
report. The observed PASS report therefore contains only:

```json
{"status":"PASS","masterSeed":42,"stage":"complete","arrangements":[{"label":"api-retry-corpus-publication",...}]}
```

It contains no J11/J12/J13 milestones or public course/assignment/problem
handoff evidence.

### Decision on report omission

The absence of J11/J12/J13 public report rows is acceptable only as a narrowly
labelled instructor-only browser execution slice: E1 owns the final schema-v2
report containing all ordered milestone rows. It is **not** acceptable as the
WP-I4 completion claim, because WP-I4 itself requires the runner to validate
the protected fragment and supply its IDs only to the fixed later children.
The present early return prevents that contract from being exercised at all.

### Required integration repair

- After the instructor-only Playwright child succeeds, fail closed unless the
  private file decodes as exactly one complete ordered J11/J12/J13 prefix with
  the same course ID and only validated public UUIDs/fixed visible codes.
- Make that validated public handoff available only to the fixed J1/J5/J8
  children in the non-instructor-only path; do not emit credentials, student
  identity, answer, score, feedback, child stdout, or private file contents.
- E1 must render the final ordered J11, J12, J13, J1, J2, J3, J4, J5, and J8
  report. Until then, label the instructor-only report as a narrow live child
  result rather than complete WP-I4 evidence.

### Independent live evidence

| Check | Result |
| --- | --- |
| Exact elevated seed-42 command | PASS |
| Public report | PASS; one public corpus arrangement; no J11/J12/J13 rows |
| Report modes | PASS: directory 0700, report 0600 |
| Post-run Podman state | PASS: `podman ps --all` empty |
| Post-run private state/artifacts | PASS: no `ple-ui-walkthrough-*` directory or journey-artifacts directory remains |

No manual cleanup, volume reset, implementation change, or staging action was
performed for this re-review.
