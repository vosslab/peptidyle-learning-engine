// CourseList visible cursor paging through an injected production transport.

import { expect, test, type Page } from "@playwright/test";
import { build, type Plugin } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const fixtureRouterPlugin = {
  name: "course-list-pagination-fixture-router",
  setup(buildApi): void {
    buildApi.onResolve({ filter: /^@solidjs\/router$/ }, () => ({
      path: "course-list-pagination-fixture-router",
      namespace: "course-list-pagination-fixture-router",
    }));
    buildApi.onLoad({ filter: /.*/, namespace: "course-list-pagination-fixture-router" }, () => ({
      contents: `
        export function A(props) {
          const anchor = document.createElement("a");
          anchor.href = props.href;
          anchor.className = props.class ?? "";
          anchor.id = props.id ?? "";
          anchor.target = props.target ?? "";
          anchor.textContent = props.children ?? "";
          if (typeof props.ref === "function") props.ref(anchor);
          return anchor;
        }
        export function createAsync() { return () => undefined; }
        export function revalidate() { return Promise.resolve(); }
        export function query(callback, key) { callback.key = key; callback.keyFor = () => key; return callback; }
      `,
      loader: "js",
    }));
  },
} satisfies Plugin;

let fixtureBundle = "";

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    outdir: "/tmp/ple-course-list-pagination-fixture",
    platform: "browser",
    plugins: [solidPlugin(), fixtureRouterPlugin],
    stdin: {
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "course_list_pagination_fixture.tsx",
      contents: `
        import { createComponent, createSignal } from "solid-js";
        import { render } from "solid-js/web";
        import { ApiRuntimeProvider } from "./src/api/runtime.tsx";
        import { CourseList } from "./src/pages/course_list_page.tsx";

        const role = "student";
        const course = (number) => ({
          id: "0198e000-0000-7000-8000-" + String(number).padStart(12, "0"),
          publicId: number,
          tenant: "0198e000-0000-7000-8000-000000000001",
          title: "Visible course " + number,
          role,
        });
        const firstPage = Array.from({ length: 50 }, (_, index) => course(index + 1));
        const secondPage = Array.from({ length: 50 }, (_, index) => course(index + 51));
        const target = course(101);
        const cursor50 = "opaque course cursor + /?=";
        const cursor100 = "course cursor after second page";
        const [createdCourses] = createSignal([firstPage[0]]);
        const requests = [];
        let scenario = "success";
        let reloads = 0;
        let failedOnce = false;
        const client = {
          listCourses: async (cursor) => {
            requests.push(cursor);
            await new Promise((resolve) => window.setTimeout(resolve, 20));
            if (scenario === "transport" && !failedOnce) {
              failedOnce = true;
              throw new Error("temporary failure");
            }
            if (scenario === "protocol") return { items: [target], nextCursor: cursor };
            if (cursor === cursor50) return { items: secondPage, nextCursor: cursor100 };
            if (cursor === cursor100) return { items: [target], nextCursor: null };
            throw new Error("unexpected cursor");
          },
        };
        const main = document.createElement("main");
        main.id = "main-content";
        main.tabIndex = -1;
        document.body.replaceChildren();
        const skipToMain = document.createElement("a");
        skipToMain.href = "#main-content";
        skipToMain.textContent = "Skip to learning content";
        document.body.append(skipToMain, main);
        render(() => createComponent(ApiRuntimeProvider, {
          runtime: { client },
          get children() {
            return createComponent(CourseList, {
              initialPage: { items: firstPage, nextCursor: cursor50 },
              createdCourses,
              reloadCourses: async () => { reloads += 1; },
              registerLink: () => undefined,
            });
          },
        }), main);
        window.__courseListPaginationFixture = {
          requests,
          cursor50,
          cursor100,
          setScenario: (next) => { scenario = next; failedOnce = false; },
          reloads: () => reloads,
        };
      `,
    },
    write: false,
  });
  const output = result.outputFiles.find((candidate) => candidate.path.endsWith(".js"));
  if (output === undefined)
    throw new Error("Course list pagination fixture bundle was not produced.");
  fixtureBundle = output.text;
});

declare global {
  interface Window {
    __courseListPaginationFixture: {
      readonly requests: Array<string | undefined>;
      readonly cursor50: string;
      readonly cursor100: string;
      readonly setScenario: (scenario: "success" | "transport" | "protocol") => void;
      readonly reloads: () => number;
    };
  }
}

async function tabTo(page: Page, target: import("@playwright/test").Locator): Promise<void> {
  for (let step = 0; step < 160; step += 1) {
    if (await target.evaluate((element) => element === document.activeElement)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(
    "visible course pagination control was not reached through native tab navigation",
  );
}

async function mountFixture(page: Page): Promise<void> {
  await page.goto("about:blank");
  await page.addScriptTag({ content: fixtureBundle });
  await expect(page.getByRole("button", { name: "Load more courses", exact: true })).toBeVisible();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
}

async function reachLoadMoreThroughSkipLink(page: Page): Promise<void> {
  const skip = page.getByRole("link", { name: "Skip to load more courses", exact: true });
  await tabTo(page, skip);
  await expect(skip).toBeFocused();
  await page.keyboard.press("Enter");
  const pagination = page.getByRole("region", { name: "Course pagination", exact: true });
  await expect(pagination).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "Load more courses", exact: true })).toBeFocused();
}

test("the production course list reaches the exact 101st course through two visible keyboard pages", async ({
  page,
}) => {
  await mountFixture(page);
  const target = page.getByRole("heading", { name: "Visible course 101", exact: true });
  await expect(target).toHaveCount(0);

  await reachLoadMoreThroughSkipLink(page);
  await page.keyboard.press("Space");
  await expect(
    page.getByText("Loaded 50 more courses. 100 courses shown.", { exact: true }),
  ).toBeVisible();
  const firstSecondPageCourse = page
    .getByRole("link", {
      name: "Open course",
      exact: true,
    })
    .nth(50);
  await expect(firstSecondPageCourse).toBeFocused();

  await reachLoadMoreThroughSkipLink(page);
  await page.keyboard.press("Enter");
  await expect(target).toBeVisible();
  const targetLink = page.locator(".course-card").filter({ has: target }).getByRole("link", {
    name: "Open course",
    exact: true,
  });
  await expect(targetLink).toBeFocused();
  await expect(page.locator(".course-card")).toHaveCount(101);
  await expect(page.getByText("All 101 courses are shown.", { exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.__courseListPaginationFixture.requests)).toEqual([
    await page.evaluate(() => window.__courseListPaginationFixture.cursor50),
    await page.evaluate(() => window.__courseListPaginationFixture.cursor100),
  ]);
});

test("course pagination preserves visible courses, retries the exact opaque cursor, and stops safely on a loop", async ({
  page,
}) => {
  await mountFixture(page);
  const cursor = await page.evaluate(() => window.__courseListPaginationFixture.cursor50);
  await page.evaluate(() => window.__courseListPaginationFixture.setScenario("transport"));
  await reachLoadMoreThroughSkipLink(page);
  await page.keyboard.press("Space");
  await expect(page.getByRole("alert")).toContainText(
    "Could not load more courses. The 50 already shown are still available.",
  );
  await expect(page.locator(".course-card")).toHaveCount(50);
  const retry = page.getByRole("button", { name: "Try loading more courses again", exact: true });
  await expect(retry).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator(".course-card")).toHaveCount(100);
  expect(await page.evaluate(() => window.__courseListPaginationFixture.requests)).toEqual([
    cursor,
    cursor,
  ]);

  await page.goto("about:blank");
  await page.addScriptTag({ content: fixtureBundle });
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await page.evaluate(() => window.__courseListPaginationFixture.setScenario("protocol"));
  await reachLoadMoreThroughSkipLink(page);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("alert")).toContainText("repeated page marker");
  await expect(page.locator(".course-card")).toHaveCount(50);
  await expect(page.getByRole("button", { name: "Load more courses", exact: true })).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Try loading more courses again", exact: true }),
  ).toHaveCount(0);
  const reload = page.getByRole("button", { name: "Reload courses", exact: true });
  await expect(reload).toBeFocused();
  await reload.press("Enter");
  expect(await page.evaluate(() => window.__courseListPaginationFixture.reloads())).toBe(1);
});
