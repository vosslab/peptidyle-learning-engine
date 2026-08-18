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

async function targetIsFocused(target: Locator): Promise<boolean> {
  if ((await target.count()) !== 1) return false;
  return target.evaluate((element) => element.ownerDocument.activeElement === element);
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
  if (await targetIsFocused(target)) return;
  for (let step = 0; step < maxSteps; step += 1) {
    if (direction === "forward") await page.keyboard.press("Tab");
    else await page.keyboard.press("Shift+Tab");
    if (await targetIsFocused(target)) return;
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
  readonly itemName: "assignments" | "courses" | "gradebook records";
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
  if (itemName === "courses") {
    return {
      loadMore: "Load more courses",
      loading: "Loading more courses...",
      loaded: /^Loaded \d+ more courses\. \d+ courses visible\.$/u,
      complete: /^Loaded \d+ course(?:s)?\.$/u,
      error:
        /^Could not load more courses\. The \d+ course(?:s)? already visible (?:is|are) still available\./u,
      retry: "Try loading more courses again",
      skipToLoadMore: "Skip to load more courses",
      fragmentTargetId: "course-pagination",
      protocolError:
        /^The list (?:returned a repeated page marker|did not add new records), so loading stopped safely\./u,
      reload: "Reload courses",
    };
  }
  if (itemName === "assignments") {
    return {
      loadMore: "Load more assignments",
      loading: "Loading more assignments...",
      loaded: /^Loaded \d+ more assignment(?:s)?\. \d+ assignment(?:s)? visible\.$/u,
      complete: /^Loaded \d+ assignment(?:s)?\.$/u,
      error:
        /^Could not load more assignments\. The \d+ assignment(?:s)? already visible (?:is|are) still available\./u,
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
    loaded: /^Loaded \d+ more gradebook record(?:s)?\. \d+ gradebook record(?:s)? visible\.$/u,
    complete: /^Loaded \d+ gradebook record(?:s)?\.$/u,
    error:
      /^Could not load more gradebook records\. The \d+ gradebook record(?:s)? already visible (?:is|are) still available\./u,
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
  const loading = page
    .getByRole("button", { name: copy.loading, exact: true })
    .and(page.locator("button:disabled"));
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
    await expect
      .poll(
        async () => {
          const targetCount = await target.count();
          if (targetCount > 1) return "duplicate-target";
          if (targetCount === 1 && (await target.isVisible())) return "target";
          if (await error.isVisible()) return "error";
          if (await protocolError.isVisible()) return "protocol-error";
          if (await complete.isVisible()) return "complete";
          if (await loadMore.isVisible()) return "load-more";
          return "pending";
        },
        { timeout: 30_000 },
      )
      .not.toBe("pending");

    const targetCount = await target.count();
    if (targetCount === 1) {
      await expect(target).toBeVisible();
      await tabToPaginationTarget(page, keyboardTarget);
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
