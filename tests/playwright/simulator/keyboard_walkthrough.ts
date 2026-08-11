// keyboard_walkthrough.ts - shared visible-focus helper for keyboard-only walkthroughs.

import { expect, type Locator, type Page } from "@playwright/test";

const MAX_TAB_STEPS = 40;
const MAX_PAGINATION_STEPS = 20;
const MAX_PAGINATION_TAB_STEPS = 80;

export type TabDirection = "forward" | "backward";

/** Reaches a rendered target only through the browser's native Tab order. */
export async function tabTo(
  page: Page,
  target: Locator,
  direction: TabDirection = "forward",
): Promise<void> {
  await tabToWithinBound(page, target, direction, MAX_TAB_STEPS);
}

/** Keeps the wider 50-row pagination traversal private to this helper module. */
async function tabToPaginationTarget(page: Page, target: Locator): Promise<void> {
  await tabToWithinBound(page, target, "forward", MAX_PAGINATION_TAB_STEPS);
}

async function tabToWithinBound(
  page: Page,
  target: Locator,
  direction: TabDirection,
  maxSteps: number,
): Promise<void> {
  if (await target.evaluate((element) => element === document.activeElement)) return;
  for (let step = 0; step < maxSteps; step += 1) {
    if (direction === "forward") await page.keyboard.press("Tab");
    else await page.keyboard.press("Shift+Tab");
    if (await target.evaluate((element) => element === document.activeElement)) return;
  }
  throw new Error("visible keyboard target was not reached through bounded native tab navigation");
}

export interface VisiblePaginationTarget {
  /** The one rendered link, row, or card the public walkthrough needs next. */
  readonly target: Locator;
  /** The focusable public control associated with target when target itself is a row or card. */
  readonly keyboardTarget?: Locator;
  /** Every currently rendered assignment card or gradebook row. */
  readonly renderedItems: Locator;
  /** Returns the first public control added after one successful page request. */
  readonly firstAppendedControl: (previousCount: number) => Locator;
  /** The noun phrase used in the product's public pagination controls. */
  readonly itemName: "assignments" | "gradebook records";
}

function paginationCopy(itemName: VisiblePaginationTarget["itemName"]): {
  readonly loadMore: string;
  readonly loading: string;
  readonly loaded: RegExp;
  readonly complete: RegExp;
  readonly error: RegExp;
  readonly retry: string;
  readonly skipToLoadMore: string;
  readonly fragmentTargetId: string;
  readonly protocolError: RegExp;
  readonly reload: string;
} {
  if (itemName === "assignments") {
    return {
      loadMore: "Load more assignments",
      loading: "Loading more assignments...",
      loaded: /^Loaded \d+ more assignments\. \d+ assignments shown\.$/u,
      complete: /^All \d+ assignments are shown\.$/u,
      error: /^Could not load more assignments\. The \d+ already shown are still available\./u,
      retry: "Try loading more assignments again",
      skipToLoadMore: "Skip to load more assignments",
      fragmentTargetId: "assignment-pagination",
      protocolError:
        /^The list (?:returned a repeated page marker|did not add new records), so loading stopped safely\./u,
      reload: "Reload assignments",
    };
  }
  return {
    loadMore: "Load more gradebook records",
    loading: "Loading more gradebook records...",
    loaded: /^Loaded \d+ more gradebook records\. \d+ records shown\.$/u,
    complete: /^All \d+ gradebook records are shown\.$/u,
    error: /^Could not load more gradebook records\. The \d+ already shown are still available\./u,
    retry: "Try loading more gradebook records again",
    skipToLoadMore: "Skip to load more gradebook records",
    fragmentTargetId: "gradebook-pagination",
    protocolError: /^Gradebook pagination stopped because the next page was not distinct\./u,
    reload: "Reload gradebook",
  };
}

/**
 * Finds one public target through only visible native pagination controls.
 *
 * Every activation proves keyboard focus, a pending state, a visible success state,
 * and an increased rendered result count. It fails rather than guessing when the
 * target is absent at the announced terminal state or a recoverable error appears.
 */
export async function tabToTargetThroughVisiblePagination(
  page: Page,
  {
    target,
    keyboardTarget = target,
    renderedItems,
    firstAppendedControl,
    itemName,
  }: VisiblePaginationTarget,
): Promise<void> {
  const copy = paginationCopy(itemName);
  const loadMore = page.getByRole("button", { name: copy.loadMore, exact: true });
  const loading = page.getByRole("button", { name: copy.loading, exact: true });
  const loaded = page.getByRole("status").filter({ hasText: copy.loaded });
  const complete = page.getByRole("status").filter({ hasText: copy.complete });
  const error = page.getByRole("alert").filter({ hasText: copy.error });
  const retry = page.getByRole("button", { name: copy.retry, exact: true });
  const skipToLoadMore = page.getByRole("link", { name: copy.skipToLoadMore, exact: true });
  const fragmentTarget = page.locator(`#${copy.fragmentTargetId}`);
  const protocolError = page.getByRole("alert").filter({ hasText: copy.protocolError });
  const reload = page.getByRole("button", { name: copy.reload, exact: true });
  let loadedPages = 0;

  for (let step = 0; step < MAX_PAGINATION_STEPS; step += 1) {
    const targetCount = await target.count();
    if (targetCount === 1) {
      await expect(target).toBeVisible();
      if (loadedPages === 0) await tabTo(page, keyboardTarget, "backward");
      else await tabToPaginationTarget(page, keyboardTarget);
      await expect(keyboardTarget).toBeFocused();
      return;
    }
    if (targetCount > 1) {
      throw new Error(
        `visible pagination rendered the requested ${itemName} target more than once`,
      );
    }

    if (await error.isVisible()) {
      await expect(retry).toBeVisible();
      throw new Error(
        `visible ${itemName} pagination reported a recoverable error before the target`,
      );
    }
    if (await protocolError.isVisible()) {
      await expect(reload).toBeVisible();
      throw new Error(`visible ${itemName} pagination stopped safely after a protocol error`);
    }
    if (await complete.isVisible()) {
      throw new Error(
        `visible ${itemName} pagination reached its terminal state before the target`,
      );
    }

    await expect(loadMore).toHaveCount(1);
    await expect(loadMore).toBeVisible();
    const before = await renderedItems.count();
    if (loadedPages === 0) {
      await tabTo(page, skipToLoadMore);
      await expect(skipToLoadMore).toBeFocused();
      await page.keyboard.press("Enter");
      await expect(fragmentTarget).toBeFocused();
      await page.keyboard.press("Tab");
    } else {
      await tabToPaginationTarget(page, loadMore);
    }
    await expect(loadMore).toBeFocused();
    await page.keyboard.press("Space");
    await expect(loading).toBeVisible();
    await expect(loading).toBeDisabled();

    await expect
      .poll(
        async () =>
          (await error.isVisible()) ||
          (await protocolError.isVisible()) ||
          (await loaded.isVisible()) ||
          (await complete.isVisible()),
        { timeout: 30_000 },
      )
      .toBe(true);

    if (await error.isVisible()) {
      await expect(retry).toBeVisible();
      throw new Error(
        `visible ${itemName} pagination reported a recoverable error after activation`,
      );
    }
    if (await protocolError.isVisible()) {
      await expect(reload).toBeVisible();
      throw new Error(`visible ${itemName} pagination stopped safely after a protocol error`);
    }
    const after = await renderedItems.count();
    if (after <= before) {
      throw new Error(
        `visible ${itemName} pagination announced success without adding a rendered item`,
      );
    }
    await expect(firstAppendedControl(before)).toBeFocused();
    loadedPages += 1;
  }
  throw new Error(
    `visible ${itemName} pagination exceeded ${MAX_PAGINATION_STEPS} keyboard activations`,
  );
}
