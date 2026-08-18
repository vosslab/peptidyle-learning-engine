// GradebookPage visible cursor paging through an injected production transport.

import { expect, test, type Page } from "@playwright/test";
import { build, type Plugin } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { tabTo } from "./simulator/keyboard_walkthrough";

const COURSE_ID = "0198e000-0000-7000-8000-000000000014";
const COURSE_B_ID = "0198e000-0000-7000-8000-000000000015";

const routerPlugin = {
  name: "gradebook-pagination-router",
  setup(buildApi): void {
    buildApi.onResolve({ filter: /^@solidjs\/router$/ }, () => ({
      path: "gradebook-pagination-router",
      namespace: "gradebook-pagination-router",
    }));
    buildApi.onLoad({ filter: /.*/, namespace: "gradebook-pagination-router" }, () => ({
      loader: "js",
      resolveDir: process.cwd(),
      contents: `
        import { createSignal } from "solid-js";
        const [courseId, setCourseId] = createSignal("${COURSE_ID}");
        window.__gradebookRoute = { setCourseId, courseId };
        export function A(props) {
          const anchor = document.createElement("a");
          anchor.href = props.href;
          anchor.className = props.class ?? "";
          anchor.textContent = String(props.children ?? "");
          return anchor;
        }
        export function useParams() { return { get courseId() { return courseId(); } }; }
        export function query(callback, key) {
          callback.key = key;
          callback.keyFor = () => key;
          return callback;
        }
      `,
    }));
  },
} satisfies Plugin;

let fixtureScript = "";

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    outdir: "/tmp/ple-gradebook-pagination-fixture",
    platform: "browser",
    plugins: [solidPlugin(), routerPlugin],
    stdin: {
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "gradebook_pagination_fixture.tsx",
      contents: `
        import { createComponent, Show } from "solid-js";
        import { render } from "solid-js/web";
        import { publishedProblemFixture } from "./generated/fixtures/published_problem.ts";
        import { createHttpApiClient } from "./src/api/http_client.ts";
        import { createApiRuntime, ApiRuntimeProvider } from "./src/api/runtime.tsx";
        import { CourseThemeRouteContext } from "./src/features/course_appearance/course_theme_context.ts";
        import { GradebookPage } from "./src/pages/gradebook_page.tsx";

        const courseId = "${COURSE_ID}";
        const courseBId = "${COURSE_B_ID}";
        const cursor = "opaque gradebook cursor + /?=";
        const template = publishedProblemFixture.gradebook[0];
        const row = (number) => {
          const assignmentId = "0198e000-0000-7000-8000-" + String(number).padStart(12, "0");
          const enrollmentId = "0198e000-0000-7001-8000-" + String(number).padStart(12, "0");
          return {
            ...template,
            assignmentId,
            enrollmentId,
            assignmentTitle: "Visible assignment " + number,
            summary: { ...template.summary, enrollment: enrollmentId },
          };
        };
        const firstPage = Array.from({ length: 50 }, (_, index) => row(index + 1));
        const target = row(51);
        const courseBRow = {
          ...row(52),
          courseId: courseBId,
          assignmentTitle: "Course B initial assignment",
        };
        const courseBNextRow = {
          ...row(53),
          courseId: courseBId,
          assignmentTitle: "Course B continued assignment",
        };
        const root = document.createElement("div");
        root.id = "gradebook-pagination-fixture";
        const main = document.createElement("main");
        main.id = "gradebook-pagination-fixture-main";
        main.tabIndex = -1;
        main.append(root);
        document.body.append(main);
        let dispose = () => {};
        let mode = "success";
        let nextRequests = [];
        let gradebookRequests = [];
        let failOnce = true;
        let releaseNext = undefined;
        let releaseDelayedA = undefined;

        const json = (value, status = 200) => new Response(JSON.stringify(value), {
          status,
          headers: { "content-type": "application/json" },
        });
        const mount = (nextMode) => {
          dispose();
          mode = nextMode;
          nextRequests = [];
          gradebookRequests = [];
          failOnce = true;
          releaseNext = undefined;
          releaseDelayedA = undefined;
          window.__gradebookRoute.setCourseId(courseId);
          const transport = async (input, init) => {
            const request = new Request(new URL(String(input), window.location.origin), init);
            const url = new URL(request.url);
            if (request.method === "GET" && url.pathname === "/api/courses/" + courseBId + "/gradebook") {
              const next = url.searchParams.get("cursor");
              gradebookRequests.push({ path: url.pathname, cursor: next });
              return next === null
                ? json({ items: [courseBRow], nextCursor: "course-b-next-cursor" })
                : json({ items: [courseBNextRow], nextCursor: null });
            }
            if (request.method === "GET" && url.pathname === "/api/courses/" + courseId + "/gradebook") {
              const next = url.searchParams.get("cursor");
              if (mode === "delayedA" && next === null)
                return new Promise((resolve) => {
                  releaseDelayedA = () => resolve(json({ items: firstPage, nextCursor: cursor }));
                });
              if (next === null) return json({ items: firstPage, nextCursor: cursor });
              nextRequests.push(next);
              if (mode === "failOnce" && failOnce) { failOnce = false; return json({ error: "temporary" }, 503); }
              if (mode === "loop") return json({ items: [firstPage[49]], nextCursor: "different-loop-cursor" });
              if (mode === "hold") return new Promise((resolve) => { releaseNext = () => resolve(json({ items: [firstPage[49], target], nextCursor: null })); });
              return json({ items: [firstPage[49], target], nextCursor: null });
            }
            if (request.method === "GET" && url.pathname === "/api/enrollments/" + target.enrollmentId + "/runs") {
              return json({ items: publishedProblemFixture.runs, nextCursor: null });
            }
            return json({ error: "unhandled " + request.method + " " + url.pathname }, 404);
          };
          const runtime = createApiRuntime(createHttpApiClient({ fetch: transport }));
          dispose = render(
            () => createComponent(ApiRuntimeProvider, {
              runtime,
              get children() {
                return createComponent(Show, {
                  get when() { return window.__gradebookRoute.courseId(); },
                  keyed: true,
                  children: (selectedCourseId) =>
                    createComponent(CourseThemeRouteContext.Provider, {
                      value: {
                        kind: "course",
                        course: {
                          summary: {
                            ...publishedProblemFixture.course,
                            id: selectedCourseId,
                            publicId: selectedCourseId === courseBId ? 2 : 1,
                            role: "instructor",
                          },
                          appearance: { theme: "grass", revision: "1", banner: null },
                        },
                      },
                      get children() { return createComponent(GradebookPage, {}); },
                    }),
                });
              }
            }),
            root,
          );
        };
        window.__gradebookPaginationFixture = {
          mount,
          nextRequests: () => nextRequests,
          gradebookRequests: () => gradebookRequests,
          releaseNext: () => { if (releaseNext === undefined) return false; releaseNext(); releaseNext = undefined; return true; },
          releaseDelayedA: () => { if (releaseDelayedA === undefined) return false; releaseDelayedA(); releaseDelayedA = undefined; return true; },
          cursor,
          targetTitle: target.assignmentTitle,
        };
      `,
    },
    write: false,
  });
  const output = result.outputFiles.find((candidate) => candidate.path.endsWith(".js"));
  if (output === undefined) {
    throw new Error("Gradebook pagination fixture bundle was not produced.");
  }
  fixtureScript = output.text;
});

interface GradebookPaginationFixture {
  mount(mode: "success" | "failOnce" | "loop" | "hold" | "delayedA"): void;
  nextRequests(): ReadonlyArray<string>;
  gradebookRequests(): ReadonlyArray<{ readonly path: string; readonly cursor: string | null }>;
  releaseNext(): boolean;
  releaseDelayedA(): boolean;
  readonly cursor: string;
  readonly targetTitle: string;
}

declare global {
  interface Window {
    __gradebookPaginationFixture: GradebookPaginationFixture;
    __gradebookRoute: { setCourseId(courseId: string): void; courseId(): string };
  }
}

async function mount(
  page: Page,
  mode: "success" | "failOnce" | "loop" | "hold" | "delayedA",
  waitForTable = true,
): Promise<void> {
  await page.goto("/gradebook-pagination-fixture-blank");
  await page.addScriptTag({ content: fixtureScript });
  await page.evaluate((nextMode) => window.__gradebookPaginationFixture.mount(nextMode), mode);
  if (waitForTable)
    await expect(page.locator("#gradebook-pagination-fixture").getByRole("table")).toBeVisible();
}

async function reachPaginationActionThroughNativeSkipLink(
  page: Page,
  actionName: string,
): Promise<void> {
  const main = page.locator("#gradebook-pagination-fixture-main");
  const skip = page.getByRole("link", { name: "Skip to load more gradebook records", exact: true });
  const pagination = page.locator("#gradebook-pagination");
  const action = page.getByRole("button", { name: actionName, exact: true });

  await main.focus();
  await expect(main).toBeFocused();
  await tabTo(page, skip, "forward");
  await expect(skip).toBeFocused();
  await expect(skip).toHaveAttribute("target", "_self");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/#gradebook-pagination$/u);
  await expect(pagination).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(action).toBeFocused();
}

test("gradebook reports a pending keyboard request with an unavailable duplicate control", async ({
  page,
}) => {
  await mount(page, "hold");
  const root = page.locator("#gradebook-pagination-fixture");
  const loadMore = root.getByRole("button", { name: "Load more gradebook records", exact: true });
  await tabTo(page, loadMore, "backward");
  await expect(loadMore).toBeFocused();
  await page.keyboard.press("Space");
  const loading = root.getByRole("button", {
    name: "Loading more gradebook records...",
    exact: true,
  });
  await expect(loading).toBeDisabled();
  await expect(root.getByRole("region", { name: "Gradebook records" })).toHaveAttribute(
    "aria-busy",
    "true",
  );
  expect(await page.evaluate(() => window.__gradebookPaginationFixture.releaseNext())).toBe(true);
  await expect(root.locator("tr.gradebook-row")).toHaveCount(51);
});

test("course navigation replaces gradebook state and ignores a delayed former-course response", async ({
  page,
}) => {
  await mount(page, "delayedA", false);
  const root = page.locator("#gradebook-pagination-fixture");

  await page.evaluate((courseId) => window.__gradebookRoute.setCourseId(courseId), COURSE_B_ID);
  await expect(root.getByText("Course B initial assignment", { exact: true })).toBeVisible();
  await expect(root.getByText("Visible assignment 1", { exact: true })).toHaveCount(0);
  await expect(
    root.getByRole("button", { name: "Load more gradebook records", exact: true }),
  ).toBeVisible();
  await reachPaginationActionThroughNativeSkipLink(page, "Load more gradebook records");
  await page.keyboard.press("Space");
  await expect(root.getByText("Course B continued assignment", { exact: true })).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => window.__gradebookPaginationFixture.gradebookRequests()))
    .toEqual([
      { path: `/api/courses/${COURSE_B_ID}/gradebook`, cursor: null },
      { path: `/api/courses/${COURSE_B_ID}/gradebook`, cursor: "course-b-next-cursor" },
    ]);

  expect(await page.evaluate(() => window.__gradebookPaginationFixture.releaseDelayedA())).toBe(
    true,
  );
  await expect(root.getByText("Course B initial assignment", { exact: true })).toBeVisible();
  await expect(root.getByText("Course B continued assignment", { exact: true })).toBeVisible();
  await expect(root.getByText("Visible assignment 1", { exact: true })).toHaveCount(0);
  await expect(root.locator("tr.gradebook-row")).toHaveCount(2);
});

test("gradebook appends a keyboard-requested 51st row once, transfers focus, and keeps run history available", async ({
  page,
}) => {
  await mount(page, "success");
  const root = page.locator("#gradebook-pagination-fixture");
  const target = await page.evaluate(() => window.__gradebookPaginationFixture.targetTitle);
  const cursor = await page.evaluate(() => window.__gradebookPaginationFixture.cursor);
  const loadMore = root.getByRole("button", { name: "Load more gradebook records", exact: true });

  await expect(root.getByText(target, { exact: true })).toHaveCount(0);
  await reachPaginationActionThroughNativeSkipLink(page, "Load more gradebook records");
  await expect(loadMore).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(root.getByText(target, { exact: true })).toHaveCount(1);
  await expect(
    root.getByRole("button", { name: "Load more gradebook records", exact: true }),
  ).toHaveCount(0);
  await expect(
    root.getByRole("link", { name: "Skip to load more gradebook records", exact: true }),
  ).toHaveCount(0);
  await expect(
    root.getByText("Loaded 51 gradebook records.", { exact: true }),
  ).toBeVisible();
  await expect(
    root
      .locator('button[id^="gradebook-history-control-"]')
      .filter({ hasText: "View run history" }),
  ).toHaveCount(51);
  await expect(root.locator("button:focus")).toHaveCount(1);
  await expect(root.locator("button:focus")).toHaveText("View run history");
  await expect
    .poll(() => page.evaluate(() => window.__gradebookPaginationFixture.nextRequests()))
    .toEqual([cursor]);

  const targetRow = root.locator("tr.gradebook-row", { hasText: target });
  const history = targetRow.getByRole("button", { name: "View run history", exact: true });
  await tabTo(page, history);
  await expect(history).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(root.getByRole("region", { name: /run history for learner/i })).toBeVisible();
});

test("gradebook preserves visible records and retries the exact opaque cursor after a transport failure", async ({
  page,
}) => {
  await mount(page, "failOnce");
  const root = page.locator("#gradebook-pagination-fixture");
  const cursor = await page.evaluate(() => window.__gradebookPaginationFixture.cursor);
  const loadMore = root.getByRole("button", { name: "Load more gradebook records", exact: true });
  await reachPaginationActionThroughNativeSkipLink(page, "Load more gradebook records");
  await expect(loadMore).toBeFocused();
  await page.keyboard.press("Space");
  await expect(
    root
      .getByRole("alert")
      .getByText(/Could not load more gradebook records\. The 50 gradebook records already visible/),
  ).toBeVisible();
  await expect(root.locator("tr.gradebook-row")).toHaveCount(50);
  await expect(root.locator("#gradebook-pagination")).toHaveCount(1);
  await expect(
    root.getByRole("link", { name: "Skip to load more gradebook records", exact: true }),
  ).toBeVisible();
  const retry = root.getByRole("button", {
    name: "Try loading more gradebook records again",
    exact: true,
  });
  await expect(retry).toBeFocused();
  await tabTo(page, retry);
  await page.keyboard.press("Enter");
  await expect(root.locator("tr.gradebook-row")).toHaveCount(51);
  await expect
    .poll(() => page.evaluate(() => window.__gradebookPaginationFixture.nextRequests()))
    .toEqual([cursor, cursor]);
});

test("gradebook cursor-loop failure stops visibly without a misleading retry", async ({ page }) => {
  await mount(page, "loop");
  const root = page.locator("#gradebook-pagination-fixture");
  const loadMore = root.getByRole("button", { name: "Load more gradebook records", exact: true });
  await reachPaginationActionThroughNativeSkipLink(page, "Load more gradebook records");
  await expect(loadMore).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(root.getByRole("alert")).toContainText("pagination stopped");
  await expect(root.getByRole("button", { name: /loading more gradebook records/i })).toHaveCount(
    0,
  );
  await expect(
    root.getByRole("button", { name: /try loading more gradebook records again/i }),
  ).toHaveCount(0);
  await expect(root.getByRole("button", { name: "Reload gradebook", exact: true })).toBeFocused();
  await reachPaginationActionThroughNativeSkipLink(page, "Reload gradebook");
  await expect(root.locator("tr.gradebook-row")).toHaveCount(50);
});
