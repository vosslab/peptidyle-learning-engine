# WP-G1 harness independence independent review

## Verdict

ACCEPTED for the WP-G1 fast static boundary. This accepts only its offline
source-policy contract. It does not change the status of blocked onboarding,
unwalked journeys, scoring, or final M7 closeout.

## Contract evidence

- The historical walkthrough plan required
  denial of product imports, SQL, local-account substitution, private score
  calls, pointer actions, answer-bearing assertions, and hidden pass
  conversion.
- The historical workstream owned only the simulator, runner, arranger/reporter,
  live configuration, and keyboard-journey source surface.
- The scanner rejects direct and aliased browser-control members after a narrow
  allowlist: only literal root `page.goto("/")`, literal platform keys, and
  the shared active-element observation survive. It also rejects dynamic
  imports, SQL/CTE/PRAGMA/client forms, browser request/state/cookie routes,
  private endpoints, body-text assertions, and hidden pass conversion.
- Normalized and zero-padded J9/J10 filenames cannot reuse the local credential
  form or session boundary as a canonical-account fallback.

## Adversarial replay

The prior residual probes now all fail with an intended violation:

```typescript
await expect(page).toContainText("nitrogen");
await page.keyboard.type("1");
const press = radios.nth(0).press;
await press("ArrowDown");
const go = page.goto;
await go("/runs/private");
const click = page.locator("button").click;
await click();
await page.$eval("body", (body) => body.textContent);
```

The real owned source tree returns an empty violation list. The declaration-time
live skip, visible local credential entry, literal root navigation, and shared
`document.activeElement` observation remain allowed and are covered by the
hostile/allow tests.

## Readiness assertion review

J1, J2, and J4 now use `expect(submit).toBeEnabled()` after visible native
radio selection rather than reading `Response format` status body text. This
is an appropriate visible semantic assertion: the shared `Actions` component
binds `Submit answer` disabled state to `!controller.canSubmit()` or pending
submission, and the multiple-choice widget validates a changed selection before
passing that state into `Actions`. It preserves the exact user-visible submit
readiness contract without asserting answer-bearing content.

## Validation

```bash
source source_me.sh && python -m pytest \
  tests/test_ui_walkthrough_harness_independence.py \
  tests/test_pyflakes_code_lint.py \
  tests/test_markdown_links.py \
  tests/test_ascii_compliance.py -q
npx tsc --noEmit -p tsconfig.lint.json
npx eslint tests/playwright/ui_walkthrough_keyboard_j1.spec.ts \
  tests/playwright/ui_walkthrough_keyboard_j2.spec.ts \
  tests/playwright/ui_walkthrough_keyboard_j4.spec.ts
npx prettier --check tests/playwright/ui_walkthrough_keyboard_j1.spec.ts \
  tests/playwright/ui_walkthrough_keyboard_j2.spec.ts \
  tests/playwright/ui_walkthrough_keyboard_j4.spec.ts \
  docs/active_plans/audits/wp_g1_harness_independence_independent_review.md
npx playwright test tests/playwright/ui_walkthrough_keyboard_j1.spec.ts \
  tests/playwright/ui_walkthrough_keyboard_j2.spec.ts \
  tests/playwright/ui_walkthrough_keyboard_j4.spec.ts --reporter=line
git diff --check
```

Results: 1,002 focused fast checks passed; TypeScript, ESLint, Prettier, and
`git diff --check` passed. The three live-only Playwright declarations compiled
and skipped without configured live inputs, as intended. No live stack was
started.

## Scope boundary

This static policy gate makes no coverage claim beyond its owned files. New
harness source or an intentionally expanded interaction path requires a new
source-policy review rather than an exact-count update.
