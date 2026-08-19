// MOD-UI-BROWSE routed mock proof: safe query, facets, immutable detail navigation.

import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import { respondCatalog } from "../../src/api/mock/handlers/catalog";

const libraryPath = "/library";

async function openLibrary(page: Page): Promise<void> {
  await page.goto("/");
  await page.evaluate((path: string) => {
    history.pushState({}, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, libraryPath);
}

function json(route: Route, value: unknown): Promise<void> {
  return route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify(value),
  });
}

async function fulfill(route: Route, response: Response): Promise<void> {
  await route.fulfill({
    status: response.status,
    headers: Object.fromEntries(response.headers.entries()),
    body: await response.text(),
  });
}

/** Authorize the full-app journey while reusing the canonical catalog mock handler. */
async function installInstructorCatalogFixture(page: Page): Promise<void> {
  await page.addInitScript(() =>
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    }),
  );
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") {
      return await json(route, {
        authenticated: true,
        tenant: publishedProblemFixture.course.tenant,
        user: {
          id: "0198e000-0000-7000-8000-000000000120",
          displayName: "Catalog instructor",
          roles: ["instructor"],
        },
      });
    }
    if (path === "/api/auth/account/presentation")
      return await json(route, { contrast: "standard" });
    if (path === "/api/problems" || path.startsWith("/api/problems/")) {
      const request = route.request();
      return await fulfill(
        route,
        respondCatalog(
          new Request(request.url(), {
            method: request.method(),
            headers: request.headers(),
          }),
        ),
      );
    }
    return await route.fulfill({ status: 404, body: "Unexpected catalog fixture request" });
  });
}

test("library uses one bounded search request, server facet counts, keyboard controls, and immutable detail", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await installInstructorCatalogFixture(page);
  await openLibrary(page);

  await expect(page.locator('[data-route-surface="library"]')).toBeVisible();
  const search = page.getByLabel("Search published questions");
  await search.focus();
  await expect(search).toBeFocused();
  await expect(page.getByLabel("Topic")).toBeVisible();
  await expect(page.getByLabel("Capability")).toBeVisible();
  await expect(page.getByLabel("License")).toBeVisible();
  await expect(page.getByLabel("Evidence")).toBeVisible();
  await expect(
    page
      .locator('[data-route-surface="library"]')
      .getByLabel("Published by", { exact: true })
      .first(),
  ).toHaveText("By Fixture Instructor");
  await page.getByLabel("Topic").selectOption("Peptidyle:BIOCHEM.PEPTIDE_BOND");
  await page.getByLabel("Capability").selectOption("serverGrading");
  await page.getByLabel("License").selectOption("ccBy");
  await page.getByLabel("Evidence").selectOption("available");
  await expect(page.getByRole("link", { name: "Open question" })).toBeVisible();

  const openQuestion = page.getByRole("link", { name: "Open question" });
  await openQuestion.focus();
  await expect(openQuestion).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator('[data-route-surface="problemDetail"]')).toBeVisible();
  await expect(
    page
      .locator('[data-route-surface="problemDetail"] .eyebrow')
      .getByText("Published question", { exact: true }),
  ).toBeVisible();
  await expect(
    page
      .locator('[data-route-surface="problemDetail"]')
      .getByLabel("Published by", { exact: true }),
  ).toHaveText("By Fixture Instructor");
  await expect(page.getByRole("heading", { name: "Anonymous learning evidence" })).toBeVisible();
  await expect(page.getByText("48 learners")).toBeVisible();
  await expect(page.getByText("Difficulty (mean score)")).toBeVisible();
  await expect(page.getByText("67.5%")).toBeVisible();
  await expect(page.getByText("1.4 attempts")).toBeVisible();
  await expect(page.getByText("2 min")).toBeVisible();
  await expect(page.getByText("0.42")).toBeVisible();
  await expect(page.locator("body")).not.toContainText(
    /answerKey|correctResponse|grading|sourceLocator/i,
  );

  const returnToLibrary = page.getByRole("link", { name: "Return to problem library" });
  await returnToLibrary.focus();
  await expect(returnToLibrary).toBeFocused();
  await page.keyboard.press("Enter");
  await page.setViewportSize({ width: 800, height: 1280 });
  await page.getByLabel("Evidence").selectOption("unavailable");
  const openSuppressedQuestion = page.getByRole("link", { name: "Open question" });
  await openSuppressedQuestion.focus();
  await expect(openSuppressedQuestion).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Insufficient evidence" })).toBeVisible();
  await expect(
    page.getByText(
      "There is not enough anonymous learning evidence to display measures for this question.",
    ),
  ).toBeVisible();
  await expect(page.locator("body")).not.toContainText(
    /48 learners|67\.5%|1\.4 attempts|2 min|0\.42/i,
  );
});

test("library keeps intentional empty and narrow responsive states", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await installInstructorCatalogFixture(page);
  await openLibrary(page);
  await page.getByLabel("Search published questions").fill("not-a-catalog-title");
  await expect(
    page.getByRole("heading", { name: "No published questions match these filters" }),
  ).toBeVisible();
  await expect(page.getByLabel("Search published questions")).toBeVisible();
  expect(
    await page.locator("html").evaluate((element) => element.scrollWidth <= element.clientWidth),
  ).toBe(true);
});
