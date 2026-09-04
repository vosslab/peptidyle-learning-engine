# PLE Question JSON instructor editor implementation

> **Historical workstream record.** This package is retained as implementation evidence, not
> current task direction. Current authority is the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

Date: 2026-08-09

## Completed scope

The instructor PLE Question JSON editor is complete. It is a focused feature under
`src/features/ple_question_json_authoring/`, rather than an addition to the large
generic editor page or decoder. Small modules own the exact schema-version-2 source,
strict codec, defaults, answer-free Student Question Preview, protected client,
revision repository, reducer, compact field components, styles, and page
composition. The existing generic editor remains available for non-PLE Question JSON drafts.

The one intentional answer-bearing browser contract is the authenticated,
author-role instructor's own canonical-source route:

```text
GET/PUT /api/workspaces/{workspace}/ple-question-json
```

It has `Cache-Control: no-store`, a strong ETag, no signed source URL, and no
checksum or object metadata. It is not a general browser source capability.
Ordinary browser contracts, local student/student preview, Wasm, public draft
and publication DTOs, and student routes remain answer-free. The protected
workspace route falls back to the legacy generic editor only when this source
route returns 404; any other protected-load failure is shown as an error.

## Instructor flow

The list page can create a complete default PLE Question JSON Question, persist it, and
open its workspace. An existing PLE Question JSON workspace loads the protected canonical
source. The author can edit the title, prompt, points, choices, exact one
correct semantic choice, per-choice teaching feedback, outcome feedback,
policies, and metadata. Each correct-choice native radio has a distinct
per-choice accessible name.

The local student preview is deliberately answer-free and makes no request.
The explicit instructor answer check is shown only after a successful private
save on the private author page. Save uses the source ETag. A stale save keeps
the author's local input visible and offers a deliberate reload of the newest
saved draft; successful reload restores focus to the editor heading.

Per-workspace operation generations discard late save/review work, and the UI
locks duplicate saves while one is in flight. Publication first obtains normal
answer-free readiness and diff data for the same saved revision. The author
then chooses scope and confirms a scope-only publish request. On success the
page shows the immutable Published Question and Question Revision link. An edit or changed
revision invalidates a pending review rather than publishing stale work.

The responsive author layout stacks without horizontal overflow at 375 px, and
the native controls remain usable with the keyboard.

## Evidence

Focused TypeScript/model checks passed, as did the visible fixture acceptance:

```text
npx tsc --noEmit
node --import tsx --test tests/test_ple_question_json_authoring.mjs \
  tests/test_ple_question_json_editor_model.mjs tests/test_editor_ui.mjs  # 28 passed
bash run_playwright_tests.sh tests/playwright/ple_question_json_editor.spec.ts # 2 passed
bash run_playwright_tests.sh tests/playwright/editor_mock_surface.spec.ts # 7 passed
bash run_playwright_tests.sh --build \
  tests/playwright/ple_question_json_editor.spec.ts \
  tests/playwright/editor_mock_surface.spec.ts # production rebuild; 9 passed
./check_codebase.sh  # all 11 stages; 167 Node tests
```

The independent re-review passed. The browser harness renders the real component,
client, and repository with same-origin protected-route responses. It proves
the authoring contract and visible behavior, but does not claim a deployed
authentication journey or browser walkthrough against a production server.

## Next dependency-ordered package

Implement bounded Canvas and Blackboard QTI profile mappings. Preserve original
package provenance and explicit unsupported-feature records; map only the
supported PLE Question JSON-supported subset into the established source/compiler boundary.
