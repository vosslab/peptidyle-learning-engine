# Flat-question persistence/publication/runtime implementation

> **Historical workstream record.** This package is retained as implementation evidence, not
> current task direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

Date: 2026-08-09

## Completed package

The PLE flat-question JSON v1 persistence boundary is complete. It turns one
canonical private authoring document into three deliberately separate durable
forms:

```text
author saves canonical JSON
          |
          v
typed CAS workspace draft + private non-signable source
          |
          v
publication transaction
  /                 |                 \
immutable source   public model     typed private grader material
non-signable       answer-free      separate grader capability
```

`PUT /api/workspaces/{workspace}/flat-question` uses a strong ETag and
`Cache-Control: no-store`. It compiles bounded raw JSON before save and returns
only the answer-free draft. `POST /api/problems/{workspace}/flat-question-publish`
uses the same no-store/ETag discipline, rereads and recompiles the private
source, copies its exact canonical bytes to a distinct immutable non-signable
`ProblemSource`, then sends one catalog promotion command. Neither route emits
source bytes, checksums, answer keys, or feedback material.

`FlatQuestionStore` atomically advances the typed draft and its source metadata
in Memory and PostgreSQL. `FlatQuestionGradingStore` is separate: the runtime
receives its own grader capability, verifies the public/private checksum
binding, and evaluates feedback only on the server. PostgreSQL uses
`PostgresGraderStore` with the dedicated `ple_grading_reader` login; its only
private read is the constrained `SECURITY DEFINER` function owned by
`ple_automated_grading`. Forced RLS and direct table-grant denial remain in effect for
application and student roles.

The focused source, data-access, PostgreSQL, route, and test owners are all
below the repository's 1000-line limit. This worktree remains shared, dirty,
and uncommitted; this report does not claim a commit boundary.

## Validation evidence

The final focused validation and independent re-review passed:

```text
cargo fmt --check
cargo clippy -p adapter_native -p learning-data-access -p server_core --all-targets -- -D warnings
cargo test -p adapter_native flat_question                         # 8 passed
cargo test -p learning-data-access --test conformance flat_question # 1 passed
cargo test -p learning-data-access --features postgres --lib flat_question # 14 passed
cargo test -p server_core flat_question                            # 6 passed
source source_me.sh && python3 -m pytest -q tests/test_crate_boundaries.py # 5 passed
source source_me.sh && python3 -m pytest -q tests/test_markdown_links.py # 66 passed
git diff --check
```

The disposable PostgreSQL gate used a real compiled authoring document: it
stored canonical private bytes, retrieved them only through the dedicated
grader capability, and evaluated `blue` as correct/right and `red` as
incorrect/wrong. The complete static/package gates and the Podman-backed
baseline were rerun after the integrity remediation. The independent re-review
is retained in the accepted package record.

## Successor package

The instructor flat-question editor is complete; its implementation handoff is
[`flat_question_editor_implementation.md`](flat_question_editor_implementation.md).
It keeps author input recoverable, previews only answer-free content, and
requires a pre-publication version review. Bounded Canvas and Blackboard QTI
profile mappings are next; they must not reopen the completed persistence or
editor boundaries.
