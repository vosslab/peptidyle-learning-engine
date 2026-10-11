# Vendored QPM audit

## Later autonomous closeout

After this audit, the user directed removal of human-dependent plan gates. A separate independent
agent assessed supported-scope closure, and a fresh actual-WASM check verified all 18 hashes,
generated both real QTI package types, and observed `unsupportedDirection` on read. The plan now
accepts supported-bank behavior plus explicit unsupported-format handling. This resolves the
completion-path finding without claiming the original successful QTI imports occurred. The
original findings below remain the historical audit result. Reader implementation and unrelated
dependency-snapshot refresh remain separate work; neither waits for human approval in this plan.

## Findings and disposition

1. **Blocker for unconditional plan acceptance: two QTI import cases are unmet.**
   The original milestone 4 in [vendor_qpm_plan.md](../vendor_qpm_plan.md) requires Canvas QTI
   and Blackboard QTI ZIP imports. QPM exposes these engines as writers only; the successful
   Blackboard export ZIP check covers a different format. Plan and Test classified this as a
   blocker; Docs classified the completion-claim drift as high severity.
   The plan and [vendor_qpm_implementation.md](../reports/vendor_qpm_implementation.md) now state
   that these two cases remain unmet and deferred. The plan's existing scope-control rule defers
   discovered limitations and requires approval for stable QPM engine changes. That authorizes
   recording the limitation, not claiming successful QTI acceptance. New readers remain separate
   upstream work in [TODO.md](../../TODO.md#future-product-capabilities). Restoring the unused
   PLE adapter would not create an integrated import path and would contradict the removal scope.

2. **Medium: retired adapter dependencies remained in the audit snapshot. Fixed for this change.**
   [dependency_freshness_snapshot.json](../../../devel/dependency_freshness_snapshot.json) still
   listed the deleted QTI manifest and its exclusive `html5ever` and `xmlparser` dependencies.
   Removed those two records and the QTI provenance from seven shared dependency records.
   Legacy rated the failing freshness check high severity. Coordinator verification found that
   the snapshot already had unrelated manifest and version drift in `HEAD`; the introduced
   removal residue is fixed, while the broader freshness check remains failing. See residual risks.

3. **Medium: image-binding documentation mixed current behavior and deferred design. Fixed.**
   [QUESTION_IMPORT_SPEC.md](../../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md),
   [NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md](../../QUESTION_SPECS/NATIVE_JSON_CONVERTER_HANDOFF_SPEC.md),
   and an older entry in [DESIGN_DECISIONS.md](../../DESIGN_DECISIONS.md) used present-tense binding
   claims beside the current skip behavior. Docs rated this high severity; the coordinator treats
   it as documentation drift because runtime and TODO already agree. These sections now identify
   binding as future work, explain per-question skips, and retain PLE's image-identity ownership
   and complete HOTSPOT authoring boundary.

4. **Medium: refresh source depended on hidden shell configuration. Fixed.**
   [vendor_qpm_wasm.py](../../../devel/vendor_qpm_wasm.py) used a custom `QPM_ROOT` environment
   variable despite [PYTHON_STYLE.md](../../PYTHON_STYLE.md#environment-variables). Removed that
   control and its import/documentation. The sibling checkout is the default; `-q/--qpm-root`
   explicitly selects another checkout. Also corrected the low-severity standard-library import
   ordering finding. No vendored bytes were rebuilt or changed for this cleanup.

5. **Low: proposed removal of the selector comment was declined.**
   Comment proposed deleting the source-path header in
   `question_import.spec.ts` because it
   could become stale. [PLAYWRIGHT_TEST_STYLE.md](../../PLAYWRIGHT_TEST_STYLE.md#selectors)
   explicitly requires selector source references. Retained the comment and added missing line
   references. The test's behavior and contract/failure-plan comments are unchanged.

## Review coverage

All six passes used fresh independent reviewers and completed before fixes began. Every pass
raised at least one finding; there were no no-finding passes. No reviewer edited production files.

| Pass | Result |
| --- | --- |
| Plan | Original QTI acceptance remains unmet; other gates and bounded architecture accepted. |
| Test | Same QTI evidence gap; keep the single seven-kind journey and its scenario registration. |
| Style | Remove custom environment configuration and correct Python import ordering. |
| Docs | Reconcile QTI acceptance status and deferred image-binding wording. |
| Legacy | Remove retired dependency snapshot records; no other live adapter consumers found. |
| Comment | Selector-comment removal proposed; retained under the more specific Playwright rule. |

Boundary: current QPM integration source, vendored artifacts, removed adapter/helper and exclusive
tests, build/lint configuration, permanent browser journey, current docs, and historical-link
repairs against baseline `e46342c735a5c0e8ec59eee99101eb9a09738d51`. The audit also checked the
dependency snapshot affected by removal. Unrelated agent configuration and Git index state were
outside the review/fix boundary. Generated distribution comments remain upstream-owned.

## Verification evidence

- Fresh Test-pass browser-scenario contract check: 5 passed.
- Fresh Style-pass ESLint and Prettier checks on the managed changed sources: passed.
- Fresh Docs-pass historical blob checks: all newly pinned targets exist at their revisions.
- After fixes, focused Markdown links, scenario contract, import style, shebang, ASCII, and
  whitespace checks: 7,132 passed. ESLint, Prettier on the edited TypeScript, and diff hygiene also
  passed after cleanup. One-time vendor CLI checks covered the sibling default,
  explicit checkout, and removal of custom environment precedence. The snapshot probe confirmed
  that edits remove only retired QTI provenance and exclusive dependencies. All 18 vendor hashes
  remain unchanged. No new permanent tests were added.
  The final focused-check log is `test-results/qpm-import/ple_qpm_audit_focused_final.log`.
- Prior implementation evidence was read from `test-results/qpm-import/`: passing fast checks,
  complete `all_test.sh`, seven-kind production-browser journey, publication, shared-image and
  individual-failure probes. The three retained screenshots were inspected during this audit.
  These are earlier runtime results, not a fresh full-suite or browser rerun after audit cleanup.

The permanent journey is justified by the stable QPM-to-Draft preview contract. Publication,
failure injection, image checks, and CLI probes remain one-time evidence. The audit did not add
permanent coverage or turn unsupported QTI imports into an acceptance success.

## Residual risks

- Successful Canvas QTI 1.2 and Blackboard QTI 2.1 import acceptance remains outstanding; current
  QPM has no readers. Inline-image binding and broader HTML rendering remain deferred.
- `source source_me.sh && python3 devel/dependency_freshness_audit.py --check` still fails on
  pre-existing snapshot drift: retired `aws-config`, absent `regex-lite`/`resvg` records, an old
  `uuid` manifest list, and multiple dated versions no longer in the lockfile. The version drift
  was checked against `HEAD:Cargo.lock`, and the relevant manifest differences also predate this
  change. The September 14 registry date was preserved. A separate registry/dependency refresh is
  needed; replacing historical registry values with installed versions would not prove freshness.
- The six passes reviewed their focused boundaries; this is not a new upstream QPM engine audit
  or a fresh live-stack acceptance run. The supported importer runtime was unchanged by the fixes.
