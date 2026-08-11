# WP-I4 instructor setup security review

## Verdict

ACCEPTED for the bounded WP-I4 instructor-only slice. The manager's elevated
seed-42 live command completed, and this review independently verified its
retained redacted report, permissions, and cleanup state. This is not full
walkthrough or schema-v2 report acceptance; WP-E1 owns that later, nine-journey
public-report contract.

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42 --build --instructor-setup-only
```

The command was run against an empty Podman inventory. It may publish the retry
corpus and visibly create one local course, student membership, and Mastery
assignment. The runner owns cleanup of only its launched Compose containers via
`down --remove-orphans`, never volumes. It left the final private report mode
0600 in a mode-0700 directory and removed the private state directory.

## Closed findings

### Descriptor-safe private state

`instructor_setup_state.ts` now opens the 0700 parent with directory and
no-follow flags, `fstat`s that descriptor, opens the child with no-follow flags,
`fstat`s the 0600 regular child, and compares the currently named parent device
and inode with the opened parent before returning the child descriptor. The
deterministic parent-replacement hook renames the parent between those operations
and proves a fail-closed result. The state path is also closed to the exact
`journeys.json` child name.

### Atomic public-ID handoff

The spec constructs J11, J12, and J13 after their visible assertions, then makes
one post-J13 commit. A J12/J13 failure therefore leaves the protected state
empty. The schema requires exactly three canonical-JSON fragments in J11, J12,
J13 order, exact key sets, empty diagnostics, bounded nonnegative elapsed time,
fixed visible outcome lists, and lower-case canonical UUIDs. It rejects private
fields, duplicate JSON members, upper-case UUIDs, oversized data, symlinks, and
reordered fragments.

### Fixed instructor-only process boundary

The direct runner test proves the instructor-only failure branch accepts only the
one retry-corpus arrangement and invokes exactly
`tests/playwright/ui_walkthrough_instructor_setup.spec.ts`. It does not run the
student, gradebook, cross-actor, or renderer children. The bounded alias is a
mode-0600 ASCII file in the mode-0700 private state directory; its value is
supplied only at the visible J12 action and is absent from the report and captured
child output. A child failure preserves only `playwright_instructor_setup`, removes
private state, and performs one no-volume Compose down for a runner-owned stack.

## Validation evidence

| Check | Result |
| --- | --- |
| Focused Python runner, harness, and baseline tests | PASS: 48 passed |
| Focused Playwright state/config/arrangement tests | PASS: 14 passed, 1 intended live-only skip |
| Node hostile report and journey tests | PASS: 27 passed |
| `npx tsc --noEmit -p tsconfig.json` | PASS |
| `npx tsc --noEmit -p tsconfig.lint.json` | PASS |
| Focused ESLint and Prettier | PASS |
| Shell syntax, ASCII, Markdown links, diff whitespace | PASS; docs checks 958 passed |
| `podman ps --all` and `podman pod ls` | Empty before authorization |

## Live evidence inspection

- The only retained instructor-run report is
  `test-results/ui_walkthrough/ui_walkthrough_seed_42.json`. It is a regular
  mode-0600 file under a mode-0700 directory and is 213 ASCII bytes.
- Its canonical compact payload is PASS, master seed 42, and `complete`, with
  exactly one `api-retry-corpus-publication` arrangement and its public problem
  and version IDs. It contains no credential, email, learner alias/name/ID,
  answer, score, response, child stdout, stderr, course ID, or assignment ID.
- `test-results/.last-run.json` records `status: passed` and an empty
  `failedTests` list. No private `ple-ui-walkthrough-*` directory or Playwright
  artifact remains; `podman ps --all` and `podman pod ls` are empty after cleanup.
- Source inspection confirms J11, J12, and J13 are constructed only after their
  visible assertions, then written by the sole post-J13
  `commitInstructorSetupState` call. No partial handoff is committed.

## Instructor-only report scope

The absence of J11/J12/J13 rows from this instructor-only public report is an
accepted slice limitation, not a WP-I4 acceptance blocker. WP-I4's contract is a
protected public-ID handoff to fixed later children; the runner deliberately
returns after its one instructor child and removes that private state at cleanup.
The report is therefore an execution/redaction/cleanup receipt, not durable
journey evidence.

The plan assigns the public schema-v2 report with ordered J11, J12, J13, J1,
J2, J3, J4, J5, and J8 rows to WP-E1, after WP-I4, WP-S1, and WP-S2. Calling
this small report a full walkthrough report would be misleading. Its `complete`
stage means the requested instructor-only command completed, not that the full
teaching-loop charter completed.

## Remaining evidence boundary

The instructor-only live evidence now exists. The remaining acceptance work is
the separately owned student, gradebook, schema-v2 report, and final walkthrough
packages. An HCI reviewer is independently evaluating the same slice; no second
live stack was started by this security review, avoiding concurrent Podman runs.
