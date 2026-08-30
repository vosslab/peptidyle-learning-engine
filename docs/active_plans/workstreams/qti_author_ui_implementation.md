# QTI author UI implementation

> **Historical accepted package.** WP-QTI-1 through WP-QTI-12 are accepted history. Current
> dependency order and remaining QTI scope are in the [release completion plan](../active/release_completion_plan.md)
> and [implementation status](../implementation_status.md).

Status: complete and independently accepted on 2026-08-09. WP-QTI-11 and WP-QTI-12 subsequently
passed as well.

## Scope

WP-QTI-10 adds the author-side review and conversion surface for the accepted WP-QTI-9 routes. It
uses the existing `/workspace/:workspaceId` route and existing flat editor rather than creating a
second authoring route.

## Delivered behavior

- `src/features/qti_profile_import/` owns the feature-local answer-free TypeScript contract,
  strict decoder, same-origin `no-store` client, pure review state, Solid panel, and styles.
- The browser uploads the selected ZIP as opaque bytes. It never parses ZIP/XML, stores an archive or
  safe report outside component memory, receives answer mappings, or receives private answer material.
- The panel presents queued/processing state with manual refresh, recognized profile/default/warning
  detail, source-ordered accepted/rejected item cards, and explicit review acknowledgement.
- An all-rejected report has no conversion controls. Unsupported profiles, terminal failures, and
  ambiguous upload collisions provide an accessible next action. Exact retry reuses an import identity
  only after an ambiguous upload; terminal failure starts a new import.
- Conversion requires an acknowledged current report, selected accepted item, and the displayed clean
  strong draft revision. A 409/428 report or draft conflict preserves safe context and asks the author
  to refresh and review again.
- Successful conversion refetches the existing workspace route and focuses the existing flat editor.
  The stale editor is inert from conversion through refetch. If refetch fails after conversion commits,
  it remains locked and the panel offers a repeatable `Reload converted draft` action. That recovery
  neither repeats conversion nor creates a new import; the editor unlocks and receives focus only
  after the converted draft loads.

## Validation

- Permanent offline Node tests in `tests/test_qti_profile_import.mjs` cover strict safe DTO decoding,
  opaque ZIP/no-store transport, acknowledgement invalidation, clean-draft conversion gating, retry
  identity, and redacted conflict errors.
- Permanent real-route Playwright tests in `tests/playwright/qti_profile_import.spec.ts` cover four
  scenarios: ambiguous retry plus conversion handoff, all-rejected and unsupported recovery,
  changed-report/dirty/stale-revision recovery with keyboard and 375 px reflow, and committed
  conversion refetch failure/retry recovery.
- `npx playwright test tests/playwright/qti_profile_import.spec.ts --workers=1` passed 4 of 4
  Chromium scenarios against the rebuilt bundle.
- `./check_codebase.sh` passed 11 of 11 checks, including 173 Node and 184 server tests.
- Independent security and HCI re-reviews reported no P0/P1 findings.

## Historical successor

WP-QTI-11 was the immediate successor and accepted the disposable live PostgreSQL/RLS/profile-to-
native path, grading, archive/provenance checks, and cleanup. This UI handoff remains scoped to
WP-QTI-10 and does not retroactively claim the live gate or the separately completed WP-QTI-12
independent close-out.

WP-QTI-1 through WP-QTI-12 are now accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the QTI-JSONL contract, WP-RC5 owns families and
Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.

## Repository state

This handoff changes no Git index state. The shared worktree remains intentionally dirty; do not infer
that the displayed changes form a commit-ready selection.
