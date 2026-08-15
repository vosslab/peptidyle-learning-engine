// MOD-UI-BROWSE routed mock proof: safe query, facets, immutable detail navigation.

import { expect, test, type Page } from "@playwright/test";
import { build, type Plugin } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const libraryPath = "/library";

/** The mounted fixture needs LibraryPage's link markup, not a second router tree. */
const catalogFixtureRouterPlugin = {
  name: "catalog-fixture-router",
  setup(buildApi): void {
    buildApi.onResolve({ filter: /^@solidjs\/router$/ }, () => ({
      path: "catalog-fixture-router",
      namespace: "catalog-fixture-router",
    }));
    buildApi.onLoad({ filter: /.*/, namespace: "catalog-fixture-router" }, () => ({
      contents: `
        export function A(props) {
          const link = document.createElement("a");
          link.className = props.class ?? "";
          link.href = props.href;
          link.textContent = String(props.children);
          return link;
        }
      `,
      loader: "js",
    }));
  },
} satisfies Plugin;

async function openLibrary(page: Page): Promise<void> {
  await page.goto("/");
  await page.evaluate((path: string) => {
    history.pushState({}, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, libraryPath);
}

test("library uses one bounded search request, server facet counts, keyboard controls, and immutable detail", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openLibrary(page);

  await expect(page.locator('[data-route-surface="library"]')).toBeVisible();
  const search = page.getByLabel("Search published questions");
  await search.focus();
  await expect(search).toBeFocused();
  await expect(page.getByLabel("Topic")).toBeVisible();
  await expect(page.getByLabel("Capability")).toBeVisible();
  await expect(page.getByLabel("License")).toBeVisible();
  await expect(page.getByLabel("Evidence")).toBeVisible();
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

let boundedCatalogFixture = "";

interface CatalogFixtureRequest {
  readonly path: string;
  readonly cursor: string | null;
  readonly text: string;
  readonly pageSize: string | null;
  readonly taxonomy: string | null;
  readonly capability: string | null;
  readonly license: string | null;
  readonly statistic: string | null;
}

interface CatalogFixtureEvidence {
  readonly requests: ReadonlyArray<CatalogFixtureRequest>;
  readonly maximumConcurrentRequests: number;
  readonly activeRequests: number;
}

interface CatalogFixtureWindow {
  readonly __catalogBrowserFixture: {
    readonly requests: ReadonlyArray<CatalogFixtureRequest>;
    readonly maximumConcurrentRequests: () => number;
    readonly activeRequests: () => number;
  };
}

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    plugins: [solidPlugin(), catalogFixtureRouterPlugin],
    platform: "browser",
    outdir: "/tmp/ple-catalog-browser-fixture",
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { publishedProblemFixture } from "./generated/fixtures/published_problem.ts";
        import { createHttpApiClient } from "./src/api/http_client.ts";
        import { createCatalogRepository } from "./src/api/catalog_repository.ts";
        import { LibraryPage } from "./src/pages/library_page.tsx";

        const totalRows = 10_000;
        const pageSize = 50;
        const requests = [];
        let activeRequests = 0;
        let maximumConcurrentRequests = 0;
        let failCursorFiftyOnce = true;

        function questionId(index) {
          const alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
          let value = index;
          let suffix = "";
          for (let digit = 0; digit < 4; digit += 1) {
            suffix = alphabet[value % alphabet.length] + suffix;
            value = Math.floor(value / alphabet.length);
          }
          return "CAT-" + suffix;
        }

        function pageFor(requestUrl) {
          const cursor = requestUrl.searchParams.get("cursor");
          const start = cursor === null ? 0 : Number(cursor);
          if (!Number.isSafeInteger(start) || start < 0 || start >= totalRows || start % pageSize !== 0) {
            return new Response(JSON.stringify({ error: "bad opaque cursor" }), { status: 400 });
          }
          const text = requestUrl.searchParams.get("text") ?? "old";
          if (start === pageSize && text === "new" && failCursorFiftyOnce) {
            failCursorFiftyOnce = false;
            return new Response(JSON.stringify({ error: "temporary later page failure" }), { status: 503 });
          }
          const end = Math.min(start + pageSize, totalRows);
          const items = Array.from({ length: end - start }, (_, offset) => {
            const index = start + offset + 1;
            return {
              ...publishedProblemFixture.catalogProblem,
              questionId: questionId(index),
              metadata: {
                ...publishedProblemFixture.catalogProblem.metadata,
                title: text + " catalog problem " + index,
              },
            };
          });
          return new Response(JSON.stringify({
            items,
            nextCursor: end < totalRows ? String(end) : null,
            facets: {
              taxonomy: [{ term: { scheme: "Peptidyle", code: "BIOCHEM.PEPTIDE_BOND", label: "Peptide bonds" }, count: totalRows }],
              capabilities: [{ capability: "serverGrading", count: totalRows }],
              licenses: [{ license: "ccBy", count: totalRows }],
              statistics: { available: 0, unavailable: totalRows },
            },
          }), { headers: { "content-type": "application/json" } });
        }

        const transport = async (input, init) => {
          const requestUrl = new URL(String(input), window.location.origin);
          const cursor = requestUrl.searchParams.get("cursor");
          const text = requestUrl.searchParams.get("text") ?? "old";
          requests.push({
            path: requestUrl.pathname,
            cursor,
            text,
            pageSize: requestUrl.searchParams.get("pageSize"),
            taxonomy: requestUrl.searchParams.get("taxonomy"),
            capability: requestUrl.searchParams.get("capabilities"),
            license: requestUrl.searchParams.get("licenses"),
            statistic: requestUrl.searchParams.get("statistics"),
          });
          activeRequests += 1;
          maximumConcurrentRequests = Math.max(maximumConcurrentRequests, activeRequests);
          const delay = text === "old" ? 80 : text === "new" ? 8 : 2;
          await new Promise((resolve) => window.setTimeout(resolve, delay));
          activeRequests -= 1;
          if (init?.method !== undefined && init.method !== "GET") throw new Error("catalog fixture only accepts GET");
          return pageFor(requestUrl);
        };

        window.__catalogBrowserFixture = {
          requests,
          maximumConcurrentRequests: () => maximumConcurrentRequests,
          activeRequests: () => activeRequests,
        };
        const client = createHttpApiClient({ fetch: transport });
        const mount = document.createElement("div");
        mount.id = "bounded-catalog-fixture";
        document.body.appendChild(mount);
        render(() => createComponent(LibraryPage, { repository: createCatalogRepository(client) }), mount);
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "catalog_bounded_browser_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles.find((candidate) => candidate.path.endsWith(".js"));
  if (output === undefined) throw new Error("Catalog browser fixture bundle was not produced.");
  boundedCatalogFixture = output.text;
});

test("built library keeps a 10,000-row catalog bounded, stale-safe, retryable, and filterable", async ({
  page,
}) => {
  const pageErrors: Array<string> = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto("/");
  await page.addScriptTag({ content: boundedCatalogFixture });

  const fixture = page.locator("#bounded-catalog-fixture");
  await expect(fixture).toBeAttached();
  expect(pageErrors).toEqual([]);
  const window = fixture.getByRole("region", { name: "Published questions" });
  const search = fixture.getByLabel("Search published questions");

  // Change the query before the deliberately delayed initial response can settle.
  await search.fill("new");
  await expect(fixture.getByText("new catalog problem 1", { exact: true })).toBeVisible();
  await expect(fixture.getByText("CAT-0001", { exact: true })).toBeVisible();
  await expect(fixture.getByText("old catalog problem 1", { exact: true })).toHaveCount(0);

  await window.evaluate((element) => {
    element.style.height = "400px";
    element.style.overflow = "auto";
    element.scrollTop = element.scrollHeight;
    element.dispatchEvent(new Event("scroll", { bubbles: true }));
  });
  await expect(fixture.getByRole("alert")).toContainText("The library could not load");
  await window.evaluate((element) => {
    element.scrollTop = 0;
    element.dispatchEvent(new Event("scroll", { bubbles: true }));
  });
  await expect(fixture.getByText("new catalog problem 1", { exact: true })).toBeVisible();
  await expect(fixture.getByLabel("Topic")).toContainText("Peptidyle:BIOCHEM.PEPTIDE_BOND (10000)");

  await fixture.getByRole("button", { name: "Try again" }).click();
  await page.waitForFunction(() => {
    const fixtureWindow = window as unknown as CatalogFixtureWindow;
    return (
      fixtureWindow.__catalogBrowserFixture.activeRequests() === 0 &&
      fixtureWindow.__catalogBrowserFixture.requests.filter(
        (request) => request.cursor === "50" && request.text === "new",
      ).length === 2
    );
  });
  await expect(fixture.getByRole("alert")).toHaveCount(0);
  await window.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
    element.dispatchEvent(new Event("scroll", { bubbles: true }));
  });
  await expect(fixture.getByText("new catalog problem 100", { exact: true })).toBeVisible();
  const renderedRowCount = await fixture.locator(".catalog-row").count();
  expect(renderedRowCount).toBeGreaterThan(0);
  expect(renderedRowCount).toBeLessThanOrEqual(15);

  const allRenderedTitles = await fixture.locator(".catalog-row h2").allTextContents();
  expect(new Set(allRenderedTitles).size).toBe(allRenderedTitles.length);

  await fixture.getByLabel("Topic").selectOption("Peptidyle:BIOCHEM.PEPTIDE_BOND");
  await fixture.getByLabel("Capability").selectOption("serverGrading");
  await fixture.getByLabel("License").selectOption("ccBy");
  await fixture.getByLabel("Evidence").selectOption("unavailable");
  await page.waitForFunction(() => {
    const fixtureWindow = window as unknown as CatalogFixtureWindow;
    return fixtureWindow.__catalogBrowserFixture.requests.some(
      (request) =>
        request.taxonomy === "Peptidyle:BIOCHEM.PEPTIDE_BOND" &&
        request.capability === "serverGrading" &&
        request.license === "ccBy" &&
        request.statistic === "unavailable",
    );
  });
  await window.evaluate((element) => {
    element.scrollTop = 0;
    element.dispatchEvent(new Event("scroll", { bubbles: true }));
  });
  await expect(fixture.getByRole("link", { name: "Open question" }).first()).toHaveAttribute(
    "href",
    "/library/CAT-0001",
  );

  const fixtureEvidence = await page.evaluate((): CatalogFixtureEvidence => {
    const fixtureWindow = window as unknown as CatalogFixtureWindow;
    return {
      requests: fixtureWindow.__catalogBrowserFixture.requests,
      maximumConcurrentRequests: fixtureWindow.__catalogBrowserFixture.maximumConcurrentRequests(),
      activeRequests: fixtureWindow.__catalogBrowserFixture.activeRequests(),
    };
  });
  expect(fixtureEvidence.activeRequests).toBe(0);
  expect(fixtureEvidence.maximumConcurrentRequests).toBe(1);
  expect(fixtureEvidence.requests[0]).toEqual({
    path: "/api/problems/search",
    cursor: null,
    text: "old",
    pageSize: "50",
    taxonomy: null,
    capability: null,
    license: null,
    statistic: null,
  });
  expect(
    fixtureEvidence.requests.filter((request) => request.cursor === "50" && request.text === "new"),
  ).toHaveLength(2);
  expect(fixtureEvidence.requests.every((request) => request.pageSize === "50")).toBe(true);
  expect(fixtureEvidence.requests.filter((request) => request.cursor !== null).length).toBeLessThan(
    10,
  );
  expect(
    fixtureEvidence.requests.some(
      (request) =>
        request.taxonomy === "Peptidyle:BIOCHEM.PEPTIDE_BOND" &&
        request.capability === "serverGrading" &&
        request.license === "ccBy" &&
        request.statistic === "unavailable",
    ),
  ).toBe(true);
  const finalRequest = fixtureEvidence.requests[fixtureEvidence.requests.length - 1];
  expect(finalRequest).toBeDefined();
  expect(finalRequest?.text).toBe("new");
});
