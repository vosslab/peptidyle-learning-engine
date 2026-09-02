# WP-W6 closed-exam completion wording review

## Verdict

**ACCEPTED.** The repaired focused browser test bundles the Assignment Attempt
completion view, completes a server-issued rendered response, and uses the
normal summary-client request to prove all three completion states. It observes
neutral unresolved wording, allowed fresh-practice wording and action, and
closed wording with no fresh-practice action; `Back to assignment` stays
visible.

This accepts the W6 completion-copy prerequisite only. It neither proves the policy engine nor
claims the later paired Mastery-versus-Exam live walkthrough, simulator report row, or browser
acceptance.

## Findings

| Priority    | Finding                                                | Evidence                                                                                                                                                                                                                                                                             | Resolution                                                                                                                       |
| ----------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| P1 resolved | Production completion behavior had not been covered.   | Historical evidence: `run_completion_summary.spec.ts` mounted former `RunPage` with route data, API runtime, and WASM provider; after a rendered response and Continue, its former `GET /api/runs/<id>/summary` path either remained pending or returned allowed/closed policy data. | The three tests asserted headings, the true-only historical `Start another practice run` action, and the persistent Back action. |
| P2 resolved | The workstream previously overstated current evidence. | The workstream identifies the historical `run_completion_summary.spec.ts`, rather than the prefetch test, and accurately records the former server-issued response, summary request, wording, action, and Back-control assertions.                                                   | Resolved.                                                                                                                        |

## Confirmed implementation properties

- Historical interface evidence: former `completionHeading(undefined)` returned
  the neutral `Run complete` heading.
- Historical interface evidence: `practiceAllowed: true` selected `Keep
practicing with a fresh variation`; `false` selected `This run is complete`.
- Historical interface evidence: the former page gated `Start another practice
run` on `runSummary()?.practiceAllowed` and retained the labelled `Back to
assignment` button outside the summary-dependent branches.
- The summary section has an `aria-labelledby` reference to the actual `h2` heading, so the
  wording remains a semantic, announced section label rather than decorative copy.
- The former UI condition consumed the server-provided
  `RunSummaryResponse.practiceAllowed`; it did not infer or prove the underlying
  policy engine.

## Re-review validation

- PASS: `npx tsc --noEmit` exited 0 with no diagnostics.
- Historical validation paths: PASS: `npx eslint src/pages/run_page.tsx tests/playwright/run_completion_summary.spec.ts` exited
  0 with no diagnostics.
- Historical validation paths: PASS: `npx prettier --check src/pages/run_page.tsx tests/playwright/run_completion_summary.spec.ts docs/active_plans/workstreams/wp_w6_exam_copy_fix.md` reported `All matched files use Prettier code style!`.
- PASS: `node --import tsx --test tests/test_frontend_contract.mjs` reported 18 passed, 0 failed.
- Historical test paths: PASS: `npx playwright test tests/playwright/run_completion_summary.spec.ts tests/playwright/run_prefetch_route.spec.ts --workers=1` reported 11 passed. It is focused offline component evidence, not a live policy-contrast claim.
- PASS: `source source_me.sh && python3 -m pytest tests/test_ascii_compliance.py tests/test_markdown_links.py -q` reported 958 passed.
- PASS: `git diff --check` exited 0.

No live stack, Podman, arranged Mastery/Exam assignment, simulator report row, or full walkthrough
was run for this review.
