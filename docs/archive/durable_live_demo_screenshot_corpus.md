# Durable Live Demo Screenshot Corpus

## Summary

Replace the current eight-capture script with a manifest-driven Live Demo atlas covering all
currently supported major role surfaces. The initial target is 52 captures: every supported
route for Public, Instructor, Student, and Sysadmin, plus meaningful workflow states and
responsive variants.

Flatten active screenshots to exactly four folders:

```text
docs/screenshots/
|-- public/
|-- instructor/
|-- student/
|-- sysadmin/
|-- current_capture_manifest.json
`-- current_capture_receipt.json
```

Use semantic filenames such as `assignment_release_preview_laptop.png`; ordering and grouping
belong in the manifest and generated gallery, not nested directories or numeric filename
prefixes. Remove duplicate, retired, and unsupported screenshots from the active corpus; Git
history retains them.

## Repository Findings

| Role       | Tracked now | Automated now | Durable target |
| ---------- | ----------: | ------------: | -------------: |
| Public     |           1 |             1 |              3 |
| Instructor |          37 |             2 |             19 |
| Student    |          28 |             4 |             23 |
| Sysadmin   |           3 |             1 |              7 |
| Total      |          69 |             8 |             52 |

- The route contract declares 32 route IDs. Current server composition supports 20 of them,
  producing 22 role-owned route surfaces because `/` has distinct Instructor, Student, and
  Sysadmin presentations.
- The present manifest is descriptive metadata only. The Playwright script independently
  hardcodes all eight paths, viewport selection, scenarios, and Ribbon expectations.
- Current `--verify` only checks that declared paths exist and begin with a PNG signature. It
  does not start PLE, execute workflows, validate dimensions, detect unmanaged screenshots, or
  prove manifest-to-scenario closure.
- One Instructor library pair is byte-identical. The four Student preview-denial images are
  byte-identical to their corresponding instructor-route-denial images.
- Several retained families represent unavailable product behavior: pending teaching
  invitations, the legacy delivery-check route, legacy assignment workspace tabs, Course
  Appearance, curriculum adoption/update, Grade Settings, Teaching Operations, item-pool
  delivery, passkeys, and institution-wide Sysadmin question projection.
- The current automated tablet and phone sizes, 768x1024 and 390x844, conflict with the durable
  viewport decision. Use laptop 1280x800, tablet 800x1280, phone 393x852, and square 800x800.
- The default Live Demo already provides excellent reusable state: Elena's course, library,
  blueprint, roster, release workspace, and Gradebook; Mary completed; Jack in progress; Avery
  not started; and Morgan's Sysadmin surfaces.
- Existing real-browser workflows already exercise authoring, Blueprint/Course creation, roster
  invitation and claim, assignment release, Student submission and recovery, Instructor Account
  lifecycle, and scoped support. Their visible application actions should become shared workflow
  helpers.

## Target Coverage

- Public, 3 captures: sign-in at laptop and phone; expired-session renewal at laptop.
- Instructor, 19 captures:
  - Course list, Course Assignment workspace, active roster, pending invitation, and Gradebook.
  - Question Library default and filtered states, published question detail, draft list, saved
    editor, publication review, and published result.
  - Blueprint list, Blueprint detail, and question picker.
  - Assignment creation, draft release workspace, answer-free preview, and released workspace.
- Student, 23 captures:
  - Course list at laptop and phone; invitation index, invitation detail, and accepted state.
  - Not-started, in-progress, and completed Course landings, plus a phone landing.
  - Unanswered Assignment overview at laptop and tablet, plus a resumed in-progress overview.
  - Unanswered Question at laptop, phone, and square; selected response; response received;
    graded; graded state restored after reload; numerical response; and hotspot response.
  - Representative authorization denial at laptop and phone.
- Sysadmin, 7 captures:
  - Role-specific Courses page.
  - Instructor Accounts initial, creation validation, created, and deactivated states; the workflow
    also verifies that reactivation persists without publishing a duplicate Active-state image.
  - Scoped Support entry form and successfully authorized roster projection.

The coverage ledger must mark the remaining 12 route IDs and future Ribbon destinations as
deferred with the missing product capability. Screenshot work must not restore retired
application behavior merely to reproduce old PNGs. WebWork and a full eight-format
response-control survey remain follow-on coverage because they require special fixtures beyond
the default Live Demo and are not separate major pages.

## Architecture and Interfaces

- Promote
  [current_capture_manifest.json](../screenshots/current_capture_manifest.json) to schema version 2. Each capture declares:
  - stable `id`, role-flat `path`, `role`, `routeId`, product `area`, workflow `state`;
  - `scenario`, `checkpoint`, canonical `viewport`, and closed `privacyProfile`;
  - gallery caption/order and optional featured status.
- Add a machine-readable coverage ledger to the manifest. Every route contract and user-facing
  Ribbon destination must be either `captured`, `covered_by` another target, or `deferred` with a
  concrete capability reason. Verification rejects unaccounted additions or silent coverage
  removal.
- Keep executable behavior in a typed TypeScript scenario registry under
  `tests/playwright/screenshot_corpus/`. The registry exposes scenario IDs and named checkpoints;
  JSON never contains selectors, arbitrary actions, or scripts.
- Extract visible application actions into reusable Playwright workflow modules shared by
  behavioral specs and screenshot scenarios. Tests retain behavioral assertions and never depend
  on screenshot paths; the capture runner owns publication and visual metadata.
- Prepare persisted scenario state once, then reuse it across checkpoints and viewport-specific
  browser contexts. Fixed Live Demo state remains read-only; mutating workflows use deterministic
  namespaced data created through normal UI and HTTP contracts.
- Replace blanket Student restrictions with closed privacy profiles for unanswered work, the
  fictional Student's selected response, self-only aggregate status, Instructor answer-free
  views, and Sysadmin scoped projections. Every profile continues to reject Answer Keys, correct
  answers, private source, credentials, tokens, internal bindings, and unrelated Student records.
- Generate `docs/SCREENSHOT_ATLAS.md` from the manifest. Organize it by role, product area,
  workflow, state, and viewport, with linked thumbnails for all captures. The generator also
  produces a temporary atlas beside each verification replay.
- Generate `current_capture_receipt.json` during publication with the manifest digest, exact path
  set, dimensions, and per-image SHA-256. This binds tracked PNGs to the last successful
  publication without imposing cross-run pixel equality.

## Canonical Command Behavior

[devel/capture_screenshots.sh](../../devel/capture_screenshots.sh) remains the only operator entry
point and filesystem-anchors itself without Git.

- Default invocation:
  1. Clear and start the owned Developer Browser Suite from a clean state.
  2. Provision the normal Live Demo baseline.
  3. Capture every manifest record into `test-results/screenshot-corpus/staging/`.
  4. Validate the complete staged corpus.
  5. Promote all role folders, the receipt, and the generated atlas only after full success.
  6. Remove active PNGs not declared by the manifest and stop/clean the owned stack.
- `--verify`:
  1. Validate manifest schema, coverage closure, scenario/checkpoint registration, tracked PNG
     set, receipt, gallery, and canonical dimensions.
  2. Start a clean Live Demo and replay every declared capture into
     `test-results/screenshot-corpus/verify/`.
  3. Apply the same navigation, route, semantic-state, privacy, page-error, and same-origin
     assertions used for publication.
  4. Require every declaration to produce exactly one correctly sized PNG and reject undeclared
     output.
  5. Verify that tracked screenshot files were not modified and leave the temporary atlas
     available for launch-readiness review.
  6. Stop the owned stack and prove cleanup.
- Do not use exact or threshold-based pixel comparison as a pass/fail gate. The repository rejects
  arbitrary pixel-equivalence acceptance. Report byte differences between tracked and replayed
  images for human review, while machine success rests on reproducibility of the declared semantic
  states.

## Dispatchable Milestones

1. Corpus foundation and flat-tree cutover
   Ownership: manifest model, validator, scenario registry, publisher, gallery, receipt, and shell
   orchestration. Port the existing eight captures, correct canonical viewports, flatten their
   paths, remove the other historical PNGs, and update documentation references. Success: default
   rebuild and clean live `--verify` both pass with exactly eight active PNGs.
2. Public and Instructor families
   Ownership: public authentication, seeded Instructor survey, authoring, and Course/Assignment
   construction workflows. Reach 3 Public and 19 Instructor captures using the fixed baseline plus
   deterministic visible mutations. Success: all supported Instructor routes and named workflow
   checkpoints are represented, privacy-safe, and reproducible from one clean run.
3. Student survey and delivery family
   Ownership: Mary/Jack/Avery baseline states, invitation/claim workflow, delivery progression,
   responsive contexts, authorization denial, and Student privacy profiles. Reach 23 Student
   captures. Success: all supported Student routes, the four canonical viewport profiles, and
   selected/pending/graded/reloaded states replay without exposing protected grading data.
4. Sysadmin families and corpus closure
   Ownership: Instructor Account lifecycle, scoped support preparation, final coverage ledger, and
   documentation reconciliation. Reach seven Sysadmin captures and 52 total. Success: all 22
   currently supported role-owned route surfaces are captured; every unsupported route or Ribbon
   destination has a specific deferred reason; no unmanaged or duplicate active PNG remains.

Each milestone updates `docs/CHANGELOG.md` only after its focused gate passes.

## Verification and Acceptance

- Add focused offline tests for manifest decoding, path/role rules, viewport resolution, route and
  Ribbon coverage closure, registry/checkpoint closure, privacy-profile selection, PNG IHDR
  dimensions, receipt validation, and deterministic gallery generation. Assert relationships, not
  a permanent fixed count.
- Run the affected existing browser scenarios after workflow extraction, then the complete serial
  production-browser suite before final acceptance.
- Final gates:
  - focused Node tests;
  - `./check_codebase.sh`;
  - `./devel/capture_screenshots.sh`;
  - `./devel/capture_screenshots.sh --verify`;
  - Markdown link validation and `git diff --check`;
  - generation of a scan-oriented 52-image atlas for the subsequent human design review.
- Capture-system implementation completes when the machine gates and clean replay pass. Human
  visual review is the next product-assessment activity and records launch-readiness observations
  separately; a passing `--verify` does not itself declare the visual design launch-ready.

## Completion record

The capture system completed on 2026-09-09. The active corpus contains 52 flat PNGs with role
counts `3/19/23/7`, a schema-version-2 manifest, a bound publication receipt, a generated atlas,
and closed accounting for all 32 route IDs and 26 Ribbon destinations. The final replay retained
52 temporary PNGs and its generated atlas under `test-results/screenshot-corpus/verify/`.

The final verifier summary was:

```text
Static screenshot corpus verification passed.
Live replay passed; 5 tracked images differ byte-for-byte and remain available for human review in test-results/screenshot-corpus/verify/.
Developer browser stopped: ple-live-demo-browser
Screenshot corpus complete; the owned Live Demo stack is clean.
```

Useful implementation findings:

- Normal Live Demo routes, visible controls, persisted application state, and ordinary HTTP
  contracts produce the corpus. No screenshot-only route, mocked response, UI, or backend state
  was added. Scoped Support preparation uses its existing authorized HTTP contract because token
  issuance has no visible product action; the Sysadmin enters and uses that capability through the
  normal visible form.
- A completed Mary Assignment legitimately offers another Assignment Attempt. The atlas therefore
  uses Jack's resumed in-progress Assignment as the distinct overview state instead of labeling a
  repeat opportunity as completed.
- The supported durable recovery is graded state after reload. A new browser context does not own
  the prior session-storage pointer, so the corpus does not fabricate a fresh-session restoration.
- Instructor Account reactivation is executed, reloaded, and verified. Its visible active state is
  byte-identical to the created state, so publication retains the distinct validation state rather
  than a duplicate reactivated PNG.
- Portable filesystems do not offer one atomic replacement for the four role folders, receipt, and
  atlas. Publication validates staging first, holds a complete recovery backup during replacement,
  rolls back ordinary failures, and refuses to overwrite an interrupted backup.
- The JavaScript entry remains a thin compatibility wrapper, the TypeScript registry remains the
  sole scenario/checkpoint dispatch map, and the shell command remains the single operator entry.
  Future capture additions should be small manifest records that reuse an existing scenario unless
  a genuinely new product workflow requires executable code.
- The next activity is a separate human review of the 52-image atlas. Visual observations from the
  implementation run do not justify expanding this completed screenshot architecture.

Closeout gates passed:

- `node --import tsx --test tests/test_screenshot_corpus.mjs`: 9 passed.
- `./check_codebase.sh`: strict TypeScript, ESLint, Prettier, and 359 Node tests passed.
- `./devel/capture_screenshots.sh`: all 52 captures published from a clean Live Demo.
- `./devel/capture_screenshots.sh --verify`: all 52 captures replayed from another clean Live Demo;
  published PNGs remained unchanged and temporary review evidence was retained.
- `./devel/run_playwright_tests.sh`: the complete serial production-browser suite passed.
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py tests/test_ascii_compliance.py tests/test_whitespace.py -q`:
  1,886 passed.
- `source source_me.sh && ./launchers/all_test.sh`: 416 generated Rust-owned TypeScript contracts,
  three fixture contracts, Rust formatting/checks/strict Clippy/tests/doctests, the browser Wasm
  target, 359 Node tests, 6,004 Python tests, PostgreSQL migration/authority/persistence acceptance,
  and PostgreSQL-plus-MinIO Course Appearance coherence all passed. Disposable resources were
  removed and the command ended with `PASS: complete live acceptance is green.`
- `git diff HEAD --check`: passed.

### Post-completion audit

Six independent Plan, Test, Style, Documentation, Legacy, and Comment passes found no blocker or
high-severity issue. Their concrete findings were closed with the smallest matching changes:

- the canonical Node lane now discovers the nine focused corpus tests;
- active-tree closure, privacy-profile selection, receipt tampering, and an injected mid-role
  publication failure have direct offline coverage;
- publication rejects root and role-folder cruft, while ordinary replacement failure restores the
  previous corpus and retains its recovery backup;
- state-based grading polling replaces a fixed sleep, and exact response-completion waits prevent
  privacy inspection from racing the two intentional immediate navigations;
- scenario selector contracts, scoped-support setup intent, screenshot troubleshooting, and an old
  direct capture command are current.

The final post-audit replay produced the same verifier summary recorded above: all 52 semantic and
privacy checkpoints passed, five byte differences remained non-gating review evidence, the tracked
corpus was unchanged, and the owned Live Demo stack was clean. The separately staged
`docs/active_plans/screenshot_llm_thoughts.md` was not rewritten or relocated as incidental audit
cleanup.
