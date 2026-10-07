# Question Specification Runtime Gate Preparation

## Status and scope

Historical source-preparation record. Its start instructions and readiness status are superseded by the
[implementation ledger](question_spec_implementation_ledger.md) and the
[approved implementation plan](../active/question_spec_implementation_plan.md). Use those documents
for current ownership and gate status. The evidence below is retained as a record of the earlier
preparation; it does not describe the active runtime attempt or establish acceptance.

This report records source-only preparation for the approved 29-milestone Question implementation. The canonical baseline selector was extended with the existing ignored raw-source PostgreSQL oracle. M12 DTO/publication and the M28 helper remain in flight, so the root-owned compile-only gate is deferred until their source owners report readiness. This marker is not that gate or the root start signal. No runtime, container, build, or full-test command was run here. **No runtime acceptance is claimed.**

## Canonical command and coverage matrix

| Slice | Current source evidence and command | Status |
| --- | --- | --- |
| Canonical baseline | `source ./source_me.sh && python3 local_stack.py acceptance` runs the baseline owner, installation data, and Course appearance. The baseline preserves the existing broad ignored grading lifecycle selector and its other selectors. | Prepared; runtime pending |
| Raw Draft sources | Existing ignored `authoring_raw_source_postgres::raw_empty_incomplete_and_broken_sources_keep_their_binding_and_edit_cas` now runs serially with `--ignored --exact --test-threads=1`, using the same app-role `DATABASE_URL` and runtime manifest as other authoring selectors. The oracle covers empty/malformed Native JSON, broken PG, incomplete PGML, unresolved Native HOTSPOT source, reopen/readback, binding preservation, and stale Edit Number CAS rejection. | Newly registered; not run |
| Question fork from archived source | Existing `blueprint_course_postgres_question_fork_parents::question_fork_parents_copy_snapshots_and_keep_immediate_parent` is registered. Its fixture archives the exact source before forking and checks the source remains unchanged. | Already registered; not run |
| Blueprint Theme behavior (M27) | Existing `blueprint_course_postgres_lifecycle::revision_only_blueprint_lifecycle_is_atomic_immutable_and_current_head_safe` is registered. It checks Blueprint `Forest`, adopted Course `Forest`, independent Course change to `Ocean`, and Blueprint remains `Forest`. | Already registered; not run |
| Installation-data lane (M28) | Exact standalone command: `source ./source_me.sh && bash tests/e2e/e2e_installation_data.sh`. The canonical acceptance runner also invokes installation data between baseline and Course appearance. | Source prepared; connected status pending |
| M29 authenticated production-browser journeys | Run separately, after the runtime is available, using the commands below and the current [M29 browser acceptance preparation](../workstreams/question_spec_m29_browser_acceptance_prep.md). This includes the existing valid Native/PG/PGML execution journey plus the focused unfinished-source and lifecycle journeys. They are not invoked by `local_stack.py acceptance`. | Prepared; not run |
| Live Demo authoring and screenshots | The Live Demo script and screenshot launcher are separate commands below; screenshot output still requires independent image assessment. | Prepared; not run |
| M29 completion | Requires fresh DB, full checks, production-browser journeys, Live Demo, screenshots, all 35 TODO/audit items, and fresh integration review. | Pending |

The raw-source selector is inserted after M04 Draft binding and before the Student Account time-zone selector. Student Account time zone, Assessment Access, Course lifecycle, the verification and SQL checks, grading lifecycle, Unrelease, Blueprint lifecycle, discovery, Question Library search, Blueprint fork lineage, and Question fork parents all run before M03 Question Revision metadata. The later Watch selectors are retained after M03. M03 is the final mutator of the shared Question metadata fixture, but the first two later Watch selectors consume it: the Question-fork Watch case, and the Pool-member Watch case through its fork helper, read/fork the seeded Question Revision 1 after M03's metadata changes. Those Watch assertions target Watch events and do not assert the original Question metadata. This is a source-level ordering dependency to keep visible during runtime review; it is not lost M03 coverage or runtime evidence. M03 is not the final selector in the stream.

The full selector order remains: M04 Draft binding; raw Draft source binding; Student Account time zone; Assessment Access; Course lifecycle, Active-lifetime rejection, and summary; application DB verification and catalog/expiry SQL checks; broad grading lifecycle suite; Unrelease; Blueprint lifecycle; discovery; Question Library search; Blueprint fork lineage; Question fork parents; M03 Revision metadata; then the three existing Watch selectors. No existing selector was removed or reordered. In particular, the raw-source selector does not run immediately before M03.

## Commands after source readiness

Canonical fresh-database acceptance:

```bash
source ./source_me.sh && python3 local_stack.py acceptance
```

The canonical runner currently sequences database baseline, installation data, and Course appearance. The installation-data slice can also be invoked directly as:

```bash
source ./source_me.sh && bash tests/e2e/e2e_installation_data.sh
```

Production-browser journeys are separate from that command:

```bash
source source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --running --scenario valid_draft_execution
source source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --running --scenario instructor_authoring
source source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --running --scenario pool_editor
source source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --running --scenario sysadmin_student_data_access
source source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --running --scenario blueprint_theme_inheritance
source source_me.sh && bash tests/e2e/e2e_live_demo_authoring.sh --publish
source source_me.sh && ./devel/capture_screenshots.sh
```

The browser commands follow the current M29 browser preparation report. The screenshots are a one-time artifact gate, followed by independent image assessment; neither is runtime evidence until run and reviewed.

## M09 failure and runtime prerequisite

`output_question_spec/m09_save_grant_completion_cli.log` records the failed disposable M09 install before oracle 09. The migration principal failed creating a type in `ple_private`; execution stopped before oracle 09. The old diagnosis that `ple_app` needed `USAGE` on `ple_data` is superseded by the approved thin `ple_api.save_question_pool_members(...)` wrapper decision in [ROOT_POOL_SAVE_CALLABLE_DECISION.md](output_question_spec/ROOT_POOL_SAVE_CALLABLE_DECISION.md), which grants `ple_app` only the wrapper. A cleanup trap was authored, but no post-failure disposable-resource inventory or cleanup receipt was found. Root must obtain and inspect that evidence before any canonical run. This report does not assert that cleanup completed.

The M09 connected DB/API/browser path remains unverified. The successful-call contract is in source, but this failed run is not evidence of connected acceptance. Do not treat this readiness report or `RUNTIME_GATE_READY.md` as the separate root start signal.

## Source-only verification evidence

| Check | Exact command or source correspondence | Result |
| --- | --- | --- |
| Python syntax | `source ./source_me.sh && python3 -m py_compile local_stack_control/database_baseline_owner.py` | Exit 0; syntax valid |
| Ignored test registration and invocation | Source: `crates/learning-data-access/tests/authoring_raw_source_postgres.rs::raw_empty_incomplete_and_broken_sources_keep_their_binding_and_edit_cas`, marked `#[ignore = "requires the disposable PostgreSQL acceptance runtime"]`. Owner invocation: `--test authoring_raw_source_postgres raw_empty_incomplete_and_broken_sources_keep_their_binding_and_edit_cas -- --ignored --exact --test-threads=1`; it uses `authoring_environment`, the same app-role environment as surrounding authoring selectors. | Exact test name, ignored registration, and owner invocation correspond |
| Scoped `git diff --check` | `git diff --check -- docs/active_plans/reports/QUESTION_SPEC_RUNTIME_GATE_PREPARATION.md output_question_spec/RUNTIME_GATE_READY.md` | Exit 0; these artifacts are untracked in this checkout, so Git's regular diff does not include their contents |
| Both artifact files, direct whitespace scan | `rg -n '[[:blank:]]+$' docs/active_plans/reports/QUESTION_SPEC_RUNTIME_GATE_PREPARATION.md output_question_spec/RUNTIME_GATE_READY.md` | Exit 1; no trailing whitespace matches |
| Both artifact files, no-index diff whitespace check | `git diff --no-index --check /dev/null docs/active_plans/reports/QUESTION_SPEC_RUNTIME_GATE_PREPARATION.md` and `git diff --no-index --check /dev/null output_question_spec/RUNTIME_GATE_READY.md` | Each exits 1 because each file differs from `/dev/null`; neither emitted whitespace diagnostics |

The source order was checked in `local_stack_control/database_baseline_owner.py`: raw Draft source binding follows M04 Draft binding and precedes Student Account time zone; M03 follows the earlier selectors, and all three Watch selectors follow M03. The fork-parent, Blueprint lifecycle/Theme, and M27/M28/M29 matrix above remains unchanged. The Watch-after-M03 dependency is recorded above. `local_stack.py acceptance` sequences baseline, installation data, and Course appearance; production-browser journeys remain separately invoked. No runtime, container, build, test, or full gate was run, and no runtime acceptance is claimed.
