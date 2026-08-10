# Flat-question editor HCI and UI brief

> **Historical design input.** This brief is retained as evidence, not current task direction.
> Current authority is the [release completion plan](../active/release_completion_plan.md),
> [implementation status](../implementation_status.md), and [human guidance](../../HUMAN_GUIDANCE.md).

Status: **DONE_WITH_CONCERNS**

Scope used: `docs/active_plans/implementation_plan.md`, `docs/HUMAN_GUIDANCE.md`,
`src/route_contract.ts`, `src/routes.ts`, `src/pages/editor_live_pages.tsx`,
`src/pages/editor_page.tsx`, `src/pages/editor_page_model.ts`,
`src/pages/editor_workspace_repository.ts`, `src/pages/editor_instructor_preview.ts`,
`src/api/http_client.ts`, `src/pages/editor_page_styles.ts`, and Playwright/unit tests in
`tests/playwright/editor_mock_surface.spec.ts` and `tests/playwright/frontend_contract.spec.ts`
plus `tests/test_editor_instructor_preview.mjs` and `tests/test_editor_ui.mjs`.

## 1) Product goal and target flow (persistence-gated)

Target user: instructor with author role on `/workspace/:workspaceId`.

The first visible instructor draft editor is a persistence-aware authoring shell:

- loads an existing workspace draft list and selects a draft (or first draft),
- edits draft learner-facing fields,
- renders local key-free preview before submission,
- offers optional protected instructor preview tied to latest saved revision,
- performs capability checks for publication readiness,
- requires a confirmation diff before immutable publish.

Plan expectation says draft editor + preview includes learner and answer-key views with seeded generation in WASM
for immediate feedback (`implementation_plan.md:1035-1037`).

## 2) Current goal/action flow

### User flow (actual in current UI)

1. **Enter route** `/workspace/:workspaceId` or `/workspace`
   - route mount: `WorkspaceEditorLivePage`
   - role allow-list: instructor/publisher/administrator
   - denied surface shown if role fails.
2. **Load** page (`loading -> empty/ready/error`)
   - `load()` fetches draft summary then selected draft.
3. **Edit** title + prompt
   - title `<input>`, prompt `<textarea>` (first text block replacement semantics).
4. **Save draft**
   - local message states.
5. **Student preview**
   - seed is entered (numeric input),
   - local preview request through WASM facade only.
6. **Instructor preview** (explicit action)
   - "Load instructor answer preview" save-first + server request,
   - shows answer key + rationale in teacher-facing panel.
7. **Capabilities + publish review**
   - check/collect required capabilities via checkboxes,
   - request publish review (saves draft first),
   - shows `Version comparison`,
   - confirm publication scope (`institution`/`public`),
   - immutable publish call uses reviewed ETag.

## 3) Vocabulary and mapping to UI controls

- **Draft**: private working question (`EditorDraft`) with no published `problem/version`.
- **Revision**: server strong ETag string used for optimistic concurrency.
- **Seed**: per-preview randomization input for reproducible student variant.
- **Student preview**: local key-free derivation + response widget, no grading.
- **Instructor preview**: protected server derivation of `correctResponse` + optional rationale.
- **Capability**: policy dimension required before publish (`algorithmicGeneration`, `hints`, `perQuestionTiming`, `offlinePreview`).
- **Publication readiness failure**: capability/infra issue message shown in editor.
- **Stale conflict**: CAS mismatch; local unsaved content is preserved and can be reloaded.
- **Review state**: server-computed publish diff requiring confirmation.

## 4) State behavior matrix (actual evidence)

| Surface                         | State             | User-visible behavior                                                       | Evidence                                                         |
| ------------------------------- | ----------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Route access                    | Permission denied | "Workspace authoring is not available for this account" + no editor surface | `editor_live_pages.tsx:19-35`, `frontend_contract.spec.ts:57-63` |
| Workspace list/editor container | `loading`         | `role= status` with "Loading workspace drafts..."                           | `editor_page.tsx:500-504`                                        |
| Workspace list/editor container | `empty`           | panel: "No drafts yet"                                                      | `editor_page.tsx:505-510`                                        |
| Workspace list/editor container | `error`           | route error panel + retry action                                            | `editor_page.tsx:511-518`                                        |
| Draft edit state                | dirty local edits | "Save draft" available; "Draft saved..."/error message text region          | `editor_page.tsx:555-583`, `editor_page.tsx:496-498`             |
| Student preview                 | `idle`            | "Seed" input + Preview button                                               | `editor_page.tsx:649-665`                                        |
| Student preview                 | `loading`         | `role=status` "Generating preview..."                                       | `editor_page.tsx:673-675`                                        |
| Student preview                 | `ready`           | question and response widget rendered (same components as learner)          | `editor_page.tsx:691-709`                                        |
| Student preview                 | `error`           | error region with message                                                   | `editor_page.tsx:676-680`                                        |
| Instructor preview              | `idle`            | button available unless backend capability unavailable                      | `editor_page.tsx:713-733`                                        |
| Instructor preview              | `loading`         | `role=status` "Loading protected instructor preview..."                     | `editor_page.tsx:737-739`                                        |
| Instructor preview              | `available`       | question card + `Correct response` + optional `Why this works`              | `editor_page.tsx:754-777`                                        |
| Instructor preview              | `unavailable`     | saved status notice from backend reason text                                | `editor_page.tsx:740-746`                                        |
| Instructor preview              | `error`           | inline retry guidance, stale/conflict messages                              | `editor_page.tsx:747-753`                                        |
| Publish                         | `idle/error`      | "Review publication changes" action + status/error message                  | `editor_page.tsx:859-869`, `851-854`                             |
| Publish                         | `loadingDiff`     | `role=status` "Preparing publication review..."                             | `editor_page.tsx:856-858`                                        |
| Publish                         | `confirm`         | version comparison + proposed title + scope + confirm action                | `editor_page.tsx:795-837`                                        |
| Publish                         | `published`       | immutable version link + confirmation                                       | `editor_page.tsx:841-848`                                        |
| Conflict states                 | stale conflict    | inline recovery notice + "Reload newest draft"                              | `editor_page.tsx:584-595`, `304`-`309`, `471-474`                |

## 5) Error recovery model

### A) Save/publish conflicts (CAS gate)

- Save uses `if-match` and throws `WorkspaceConflictError` on mismatch.
- UI sets `staleConflict` and preserves unsaved edits.
- Explicit recovery action: **Reload newest draft**.
- Same preserved-edit pattern used for save/delete/publish conflicts.

### B) Instructor preview conflicts

- Instructor preview always saves draft first to obtain exact server revision.
- If revision mismatch returns conflict, UI shows "Reload...try again" and keeps current draft visible.
- If server returns unavailable, UI shows backend reason and does not inject secret fields.

### C) Publication capability refusal

- Publication review includes live capability check.
- Validation failures block confirmation and retain editability with clear violation text.
- If read-only backend unavailable, previous local violations remain visible.

### D) Network/service failure

- Any caught non-conflict error is surfaced in inline `inline-error` role text.
- No specialized offline badge is implemented; offline/unreachable routes appear through generic error text.

## 6) Keyboard and accessibility behavior

- **Forms and controls are labeled and semantic**: title/prompt inputs, seed input, checkboxes, buttons.
- **Announcements**:
  - `role="status"` for load, saving, prepare, loading states.
  - `role="alert"` for conflicts and failures.
  - `aria-live="polite"` on section/status container and instructor preview while loading.
- **Navigation and labels**:
  - landmarks (`aria-labelledby`) for major editor sections.
  - list active draft uses `aria-current="page"` for current selection.
- **Focus behavior**:
  - explicit focus restoration is not implemented in `EditorPage`.
  - user currently relies on browser native focus sequencing.
- **Keyboard-only concerns**:
  - All interactive controls are inherently keyboard-focusable (`button`, `input`, `textarea`, `checkbox`, `select`).
  - No custom key handlers are required for core operations; all workflows can be performed with Tab/Enter/Space.

## 7) Loading / empty / success / offline / permission evidence

- **Loading**: route and data operations use status text and placeholders.
- **Empty**: no selected draft -> "No drafts yet".
- **Success**:
  - local preview renders immediately with current draft values,
  - publish success shows immutable version link.
- **Offline**:
  - offline is not separately detected as a mode.
  - failure paths still return non-conflict error UI states.
  - tests assert student preview uses no extra network calls; instructor preview path only calls when explicitly triggered.
- **Permission**:
  - role-gated route denies non-author users before editor mount.
  - denied route preserves shell and does not call workspace/author-preview endpoints in student navigation path (`frontend_contract.spec.ts`).

## 8) Responsiveness and layout assumptions

- Primary grid is desktop two-column (`1fr + ~0.7fr`) and collapses to one column at `max-width: 48rem`.
- Section ordering is stable; on narrow screens sections stack vertically.
- No horizontal overflow test for this editor exists yet; only generic run-baseline viewport checks are present for other flows.

## 9) Route-level contract and persistence gate mapping

- Contract surface: `/workspace/:workspaceId` is "Draft editor, validation, and preview"
  (`route_contract.ts:41-46`).
- `WorkspaceEditorLivePage` only mounts for allowed roles and injects:
  - `createWorkspaceEditorRepository(client, createInstructorPreviewClient())`
  - `initialWorkspace` route param.
- Persistence gate:
  - workspace `GET/PUT/DELETE` paths are same-origin and no-store.
  - revision captured from `ETag` and required on save/delete/publish/preview.
  - publish can only proceed with revision from review diff.

## 10) Acceptance evidence (test-level)

### Editor contract and security boundaries

- **No network during student preview**
  - `editor_mock_surface.spec.ts` asserts local student preview has zero `/api`/`key` requests while clicking "Preview this variation".
- **Instructor preview is explicit + save-first + no save leakage**
  - `editor_mock_surface.spec.ts` live test checks first two calls are `PUT` save + `GET /author-preview`, and saved payload retains only allowed draft fields.
- **Offline + backend-availability errors**
  - tests assert unavailable/inconsistent instructor preview states with retry text.
- **Stale conflict recovery**
  - tests check local unsaved edits remain visible and stale answers are cleared in conflict/reload flow.
- **Publish diff scope and CAS**
  - live test checks publish calls are `POST /api/problems/{id}/publish` with JSON body only `{ "scope": "public" }` and changing `if-match`.
- **No answer/key/source leakage**
  - tests verify request payloads and decode boundaries exclude forbidden fields.
- **Compile-time boundary**
  - `editor_page_typecheck.ts` marks draft preview cannot satisfy published assignment/published envelope types.
- **Route permissions**
  - `frontend_contract.spec.ts` confirms denied message for workspace routes in non-author session and confirms no workspace routes are called.

## 11) Small ASCII wireframe (desktop)

```
+----------------------------------------------------------------------------------+
| Instructor workspace  (route surface: workspaceEditor)                               |
| h1 Draft, preview, and publish a learning question                                |
+----------------------------------------------------------------------------------+
| Sidebar                    | Main editor panel                               |
| - Draft list               | [Draft fields] Title + Prompt text                |
| - choose draft (aria-current) | [Save draft] [Delete] [message]                  |
| - Load more drafts          | [Policy checks] violations / readiness             |
|                           +--------------------------------------------------+
|                           | Student preview                                   |
|                           | Seed input + Preview button                         |
|                           | - question card                                   |
|                           | - response widget                                 |
|                           +--------------------------------------------------+
|                           | Instructor answer preview                          |
|                           | button: Load/Retry instructor answer preview        |
|                           | - Correct response (feedback/rationale)            |
|                           +--------------------------------------------------+
|                           | Publish panel                                    |
|                           | Review publication changes -> version diff           |
|                           | scope select + confirm immutable publication        |
+----------------------------------------------------------------------------------+
```

## 12) Nielsen / WCAG ledger

### Nielsen heuristics alignment

- **Visibility of system status**: strong use of inline status/error roles; revision/publish messages, loading states.
- **Error prevention**: conflict model preserves local edits and requires explicit recovery/reload.
- **User control and freedom**: explicit "Reload newest draft" and "Try again" affordances.
- **Match to real world**: vocabulary maps to "draft", "publication", "scope", "revision".
- **Consistency**: route surfaces and command style align with global shell conventions.
- **Error recovery help**: inline text and action buttons for conflict cases.

### WCAG/ARIA checklist (partial)

- **1.1.1** non-text content: renderer responsibility, not editor-specific.
- **2.1.1/2.1.2** keyboard operable controls: native controls used.
- **2.4.1** bypass / focus context: `aria-current`, landmarks and section headings present.
- **3.3.1/3.3.3** error identification/instructions: inline alerts and explicit messages present.
- **4.1.1/4.1.3** name/role/state/value: buttons/inputs/sections are semantic and include roles/states in status flow.

## 13) Concerns versus requested behavior

This brief is marked DONE_WITH_CONCERNS because the current visible editor is not yet a full flat-question
multi-choice authoring surface:

- It supports title + first-text prompt editing and does not expose explicit per-choice input fields
  for first-choice feedback/rationale creation.
- It does provide response-widget based student preview and instructor answer feedback display
  (`Correct response`/`Why this works`) as output, not authoring controls.
- If "create per-choice outcomes during initial authoring" is a hard requirement,
  that is a follow-on feature gap outside current implementation.
