// keyboard_walkthrough.spec.ts - behavior proof for visible keyboard pagination.

import { expect, test, type Locator, type Page } from "@playwright/test";

import { tabToTargetThroughVisiblePagination } from "./keyboard_walkthrough";

interface FixtureOptions {
  readonly pages: readonly (readonly string[])[];
  readonly failAtPage?: number;
  readonly initialLoadDelayMs?: number;
  readonly protocolAtPage?: number;
  readonly itemName?: "assignments" | "courses" | "gradebook records";
}

function fixtureMarkup({
  pages,
  failAtPage,
  initialLoadDelayMs = 0,
  protocolAtPage,
  itemName = "assignments",
}: FixtureOptions): string {
  const copy =
    itemName === "courses"
      ? {
          complete: "courses",
          error: "Could not load more courses. The ",
          fragmentId: "course-pagination",
          load: "Load more courses",
          loaded: "more courses. ",
          loading: "Loading more courses...",
          retry: "Try loading more courses again",
          shown: "courses",
          skip: "Skip to load more courses",
        }
      : itemName === "gradebook records"
        ? {
            complete: "gradebook records",
            error: "Could not load more gradebook records. The ",
            fragmentId: "gradebook-pagination",
            load: "Load more gradebook records",
            loaded: "more gradebook records. ",
            loading: "Loading more gradebook records...",
            retry: "Try loading more gradebook records again",
            shown: "records",
            skip: "Skip to load more gradebook records",
          }
        : {
            complete: "assignments",
            error: "Could not load more assignments. The ",
            fragmentId: "assignment-pagination",
            load: "Load more assignments",
            loaded: "more assignments. ",
            loading: "Loading more assignments...",
            retry: "Try loading more assignments again",
            shown: "assignments",
            skip: "Skip to load more assignments",
          };
  return `
    <a href="#main-content">Skip to main content</a>
    <main id="main-content" tabindex="-1">
      <p id="pagination-status" role="status" aria-live="polite"></p>
      <a href="#${copy.fragmentId}">${copy.skip}</a>
      <section id="items"></section>
      <section id="${copy.fragmentId}" tabindex="-1"></section>
    </main>
    <script>
      const pages = ${JSON.stringify(pages)};
      const failAtPage = ${JSON.stringify(failAtPage ?? null)};
      const initialLoadDelayMs = ${JSON.stringify(initialLoadDelayMs)};
      const protocolAtPage = ${JSON.stringify(protocolAtPage ?? null)};
      let pageIndex = 0;
      const items = document.querySelector("#items");
      const controls = document.querySelector("#${copy.fragmentId}");
      const status = document.querySelector("#pagination-status");
      const shownCount = () => items.querySelectorAll("article.course-card").length;
      const appendPage = (page) => {
        let firstLink = null;
        for (const title of pages[page]) {
          const card = document.createElement("article");
          card.className = "course-card";
          const link = document.createElement("a");
          link.id = "assignment-" + title;
          link.href = "/courses/demo/assignments/" + title;
          link.textContent = title;
          if (title === "target") link.dataset.target = "true";
          card.append(link);
          items.append(card);
          if (firstLink === null) firstLink = link;
        }
        return firstLink;
      };
      const renderControl = () => {
        controls.replaceChildren();
        if (pageIndex >= pages.length) {
          status.textContent = "All " + shownCount() + " ${copy.complete} are shown.";
          return;
        }
        const button = document.createElement("button");
        button.setAttribute("type", "button");
        button.textContent = "${copy.load}";
        button.addEventListener("click", () => {
          button.disabled = true;
          button.textContent = "${copy.loading}";
          status.textContent = "${copy.loading}";
          window.setTimeout(() => {
            if (failAtPage === pageIndex) {
              const alert = document.createElement("div");
              alert.setAttribute("role", "alert");
              alert.textContent = "${copy.error}" + shownCount() + " already shown are still available.";
              const retry = document.createElement("button");
              retry.setAttribute("type", "button");
              retry.textContent = "${copy.retry}";
              alert.append(retry);
              controls.replaceChildren(alert);
              return;
            }
            if (protocolAtPage === pageIndex) {
              const alert = document.createElement("div");
              alert.setAttribute("role", "alert");
              alert.textContent = "Gradebook pagination stopped because the next page was not distinct. The " + shownCount() + " already shown are still available.";
              const reload = document.createElement("button");
              reload.setAttribute("type", "button");
              reload.textContent = "Reload gradebook";
              alert.append(reload);
              controls.replaceChildren(alert);
              return;
            }
            const before = shownCount();
            const firstAppended = appendPage(pageIndex);
            pageIndex += 1;
            status.textContent = "Loaded " + (shownCount() - before) + " ${copy.loaded}" + shownCount() + " ${copy.shown} shown.";
            firstAppended?.focus();
            renderControl();
          }, 15);
        });
        controls.append(button);
      };
      const renderInitialPage = () => {
        appendPage(pageIndex);
        pageIndex += 1;
        renderControl();
      };
      if (initialLoadDelayMs > 0) window.setTimeout(renderInitialPage, initialLoadDelayMs);
      else renderInitialPage();
    </script>
  `;
}

async function loadFixture(page: Page, options: FixtureOptions): Promise<void> {
  await page.setContent(fixtureMarkup(options));
  const skipToMain = page.getByRole("link", { name: "Skip to main content", exact: true });
  const main = page.locator("#main-content");
  await page.keyboard.press("Tab");
  await expect(skipToMain).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(main).toBeFocused();
}

function visibleTarget(page: Page): Locator {
  return page.locator("a[data-target=true]");
}

function visibleCards(page: Page): Locator {
  return page.locator("article.course-card");
}

function firstAppendedControl(page: Page, previousCount: number): Locator {
  return visibleCards(page).nth(previousCount).getByRole("link");
}

test("keyboard pagination reaches a page-one target forward from focused main content", async ({
  page,
}) => {
  await loadFixture(page, { pages: [["target"]] });
  await expect(page.locator("#main-content")).toBeFocused();

  await tabToTargetThroughVisiblePagination(page, {
    target: visibleTarget(page),
    renderedItems: visibleCards(page),
    firstAppendedControl: (index) => firstAppendedControl(page, index),
    itemName: "assignments",
  });

  await expect(visibleTarget(page)).toBeFocused();
});

test("keyboard pagination waits for an initially loading terminal-page target", async ({
  page,
}) => {
  await loadFixture(page, { initialLoadDelayMs: 25, pages: [["target"]] });
  await expect(page.locator("#main-content")).toBeFocused();

  await tabToTargetThroughVisiblePagination(page, {
    target: visibleTarget(page),
    renderedItems: visibleCards(page),
    firstAppendedControl: (index) => firstAppendedControl(page, index),
    itemName: "assignments",
  });

  await expect(visibleTarget(page)).toBeFocused();
  await expect(
    page.getByRole("button", { name: "Load more assignments", exact: true }),
  ).toHaveCount(0);
});

test("course pagination reaches a third-page course through visible keyboard controls", async ({
  page,
}) => {
  const pageOne = Array.from({ length: 50 }, (_, index) => `course-${index}`);
  const pageTwo = Array.from({ length: 50 }, (_, index) => `course-${index + 50}`);
  await loadFixture(page, { itemName: "courses", pages: [pageOne, pageTwo, ["target"]] });

  await tabToTargetThroughVisiblePagination(page, {
    target: visibleTarget(page),
    renderedItems: visibleCards(page),
    firstAppendedControl: (index) => firstAppendedControl(page, index),
    itemName: "courses",
  });

  await expect(visibleTarget(page)).toBeFocused();
});

test("gradebook pagination fails closed with the visible protocol reload action", async ({
  page,
}) => {
  await loadFixture(page, {
    itemName: "gradebook records",
    pages: [["first"], ["target"]],
    protocolAtPage: 1,
  });

  await expect(
    tabToTargetThroughVisiblePagination(page, {
      target: visibleTarget(page),
      renderedItems: visibleCards(page),
      firstAppendedControl: (index) => firstAppendedControl(page, index),
      itemName: "gradebook records",
    }),
  ).rejects.toThrow("protocol error");
  await expect(page.getByRole("button", { name: "Reload gradebook", exact: true })).toBeVisible();
});

test("keyboard pagination fails at an announced terminal state when the target is absent", async ({
  page,
}) => {
  await loadFixture(page, { pages: [["first"]] });

  await expect(
    tabToTargetThroughVisiblePagination(page, {
      target: visibleTarget(page),
      renderedItems: visibleCards(page),
      firstAppendedControl: (index) => firstAppendedControl(page, index),
      itemName: "assignments",
    }),
  ).rejects.toThrow("terminal state before the target");
  await expect(page.getByRole("status")).toContainText("All 1 assignments are shown.");
});

test("keyboard pagination fails closed on a visible recoverable error", async ({ page }) => {
  await loadFixture(page, { pages: [["first"], ["target"]], failAtPage: 1 });

  await expect(
    tabToTargetThroughVisiblePagination(page, {
      target: visibleTarget(page),
      renderedItems: visibleCards(page),
      firstAppendedControl: (index) => firstAppendedControl(page, index),
      itemName: "assignments",
    }),
  ).rejects.toThrow("recoverable error");
  await expect(page.getByRole("alert")).toContainText(
    "Could not load more assignments. The 1 already shown are still available.",
  );
  await expect(
    page.getByRole("button", { name: "Try loading more assignments again" }),
  ).toBeVisible();
});

test("keyboard pagination rejects a duplicate exact target instead of choosing one", async ({
  page,
}) => {
  await loadFixture(page, { pages: [["target", "target"]] });

  await expect(
    tabToTargetThroughVisiblePagination(page, {
      target: visibleTarget(page),
      renderedItems: visibleCards(page),
      firstAppendedControl: (index) => firstAppendedControl(page, index),
      itemName: "assignments",
    }),
  ).rejects.toThrow("more than once");
});
