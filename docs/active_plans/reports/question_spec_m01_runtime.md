# Question Spec M01 Runtime Report

Date: 2026-10-05

M01 is accepted on its literal criteria. This report records local runner and dependency evidence;
fresh-database, Live Demo, stack run 61546, and refreshed API runtime remain pending. The temporary
fixture correction passed SPEC and QUALITY review, but its runtime rerun has not occurred. Target
size is 9.55 GiB.

## Local tooling

- TypeScript setup completed through the existing bootstrap. `output_question_spec/m01/setup_typescript.log`
  records 174 packages installed. Its npm audit summary reports six critical vulnerabilities;
  details are in `output_question_spec/m01/npm_audit.json`. This is dependency evidence only; no
  dependency updates were made.
- Playwright setup completed, and its standalone Chromium launch passed after the sandbox-blocked
  default launch. Evidence is in `setup_playwright_escalated.log` and
  `playwright_launch_escalated.log`. The default attempt in `playwright_launch.log` ended in
  Chromium `SIGTRAP` after macOS denied Mach port bootstrap access. No browser journey was run.
- The pre-generated-file Node question baseline recorded 37 tests, 32 passing and 5 failing because
  generated API constants were absent (`node_question_baseline.log`). After generated contracts were
  available, the question-focused Node run recorded 76 passing and 0 failing
  (`node_question_generated_baseline.log`).
- The pre-M02 native model baseline recorded 239 passing Rust tests and no failures across its
  nonzero suites (`native_model_baseline.log`).

## Fresh database baseline

The authorized runner is:

```bash
source ./source_me.sh && tests/e2e/e2e_database_baseline.sh
```

The original process acquired its fixed browser-suite lease, built the `database-migrator` image for
project `ple-live-demo-browser`, started PostgreSQL, and ran `cargo run -p project-tools -- database
verify`. It exited nonzero before the SQL install/compile phase because the concurrent M03 Rust tree
did not compile: `crates/server/src/question_metadata.rs` imported
`learning_data_access::SavedQuestionMetadata`, which was not yet exported. There was also one
`unused_imports` warning for `AccountId` in `support_capability.rs`. This is a failed runtime attempt,
not an accepted fresh-database result and not evidence of an SQL schema failure. The M03 API owner
confirmed those Rust types were still being registered when the baseline ran.

The M03 owner reported SQL ready for a fresh compile gate, but the failed build began before that
handoff and read a mutable checkout, so its result cannot establish which schema snapshot it
consumed. After exit, the controller reported `ple-live-demo-browser` absent with empty container,
network, and volume inventories; the owner completed cleanup.

The original runner's redirected log is `output_question_spec/m01/fresh_database_baseline.log`.
An attempted parallel retry was rejected by the held lease, but it opened that same log with shell
truncation first. The final exception above remained visible after the original process exited, but
the log is not a complete transcript and cannot establish all preceding command output or the
original shell exit code. The controller's final cleanup status is available from the post-exit
read-only status query recorded above. Do not retry this profile until the M03 API/schema work reaches
a coherent handoff.

## Runner coverage and runtime limits

- The repository has an owned PostgreSQL plus MinIO cross-store runner at
  `tests/e2e/e2e_course_appearance.sh`; its owner controls the disposable stack and runs the ignored
  MinIO object-store conformance test plus the PostgreSQL/MinIO saga test. It was inspected but not
  started while the exclusive profile lease and M03 schema changes were active.
- The existing WeBWorK renderer evidence tool is `devel/webwork_render_probe.py`. It accepts an
  explicit PG/PGML source or matrix, renderer origin, output directory, and optional browser check.
  The pilot WeBWorK PGML source is available at
  `content/pilot/webwork/which_hydrophobic-simple.pgml`. No renderer was running during this
  inspection, so no render result is claimed.
- Production-browser scenarios run through
  `tests/e2e/e2e_live_demo_production_browser.py --running --scenario <name>` against the
  fixed HTTPS Live Demo owner. `instructor_authoring` and `auth_authorization` are suitable focused
  M01 browser journeys. Neither was run because the fresh profile owns the lease and the M03 schema
  handoff is pending.
- The default `local_stack.py status` showed the unrelated `containers` project absent. An explicit
  read-only query for `ple-live-demo-browser` showed only its healthy PostgreSQL container while the
  baseline owner was building the migrator; the complete Live Demo is therefore not ready.

## Next bounded gates

1. Wait for the M03 API and SQL owners to confirm a coherent handoff, then rerun the fresh database
   baseline so its result is tied to the intended schema. Preserve the complete final output in the
   M01 evidence directory.
2. Once the schema handoff is stable, start the fixed Live Demo through
   `source ./source_me.sh && ./launchers/run_live_demo.sh --headless`, retain the ready stack, and
   run the two focused synthetic-account browser scenarios.
3. Use the owned course-appearance runner for MinIO proof and the explicit renderer probe or existing
   WeBWorK browser journey for render proof; record each command and status separately.
