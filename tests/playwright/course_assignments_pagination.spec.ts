// Production assignment pagination: keyboard discovery, bounded request, focus, and recovery.

import { expect, test } from "@playwright/test";
import { build, type Plugin } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";

const COURSE_ID = "0198e000-0000-7000-8000-000000000014";

const fixtureRouterPlugin = {
  name: "assignment-pagination-fixture-router",
  setup(buildApi): void {
    buildApi.onResolve({ filter: /^@solidjs\/router$/ }, () => ({
      path: "assignment-pagination-fixture-router",
      namespace: "assignment-pagination-fixture-router",
    }));
    buildApi.onLoad({ filter: /.*/, namespace: "assignment-pagination-fixture-router" }, () => ({
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
        export function useParams() { return {}; }
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
    plugins: [solidPlugin(), fixtureRouterPlugin],
    platform: "browser",
    outdir: "/tmp/ple-course-assignment-pagination-fixture",
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { ApiRuntimeProvider } from "./src/api/runtime.tsx";
        import { AssignmentList } from "./src/pages/course_assignments_page.tsx";

        const courseId = "${COURSE_ID}";
        const firstPage = Array.from({ length: 50 }, (_, index) => ({
          id: "0198e000-0000-7000-8000-" + String(index + 100).padStart(12, "0"),
          publicId: index + 100,
          courseId,
          title: "Assignment " + String(index + 1),
          items: [{ deliveryState: "active" }],
          selectionGroups: [{ drawCount: 2 }],
        }));
        const secondPage = Array.from({ length: 50 }, (_, index) => ({
          id: "0198e000-0000-7000-8000-" + String(index + 200).padStart(12, "0"),
          publicId: index + 200,
          courseId,
          title: "Assignment " + String(index + 51),
          items: [{ deliveryState: "active" }],
          selectionGroups: [{ drawCount: 2 }],
        }));
        const target = {
          id: "0198e000-0000-7000-8000-000000000999",
          publicId: 999,
          courseId,
          title: "Assignment 101",
          items: [{ deliveryState: "active" }],
          selectionGroups: [{ drawCount: 2 }],
        };
        const terminalTarget = {
          ...target,
          id: "0198e000-0000-7000-8000-000000001000",
          publicId: 1000,
          title: "Assignment 102",
        };
        const requests = [];
        let scenario = "success";
        let reloads = 0;
        const studentSummary = {
          tenant: "tenant",
          enrollment: "enrollment",
          currentScore: 0.72,
          bestScore: 0.83,
          latestScore: 0.76,
          completedRunCount: 2,
          totalQuestionAttempts: 5,
          lastActivityAt: null,
        };
        const client = {
          listAssignments: async (_course, cursor) => {
            requests.push(cursor);
            await new Promise((resolve) => window.setTimeout(resolve, 20));
            if (scenario === "transport") throw new Error("temporary failure");
            if (scenario === "protocol") return { items: [target], nextCursor: cursor };
            if (cursor === "after-50") return { items: secondPage, nextCursor: "after-100" };
            if (cursor === "after-100") return { items: [target], nextCursor: null };
            return { items: [terminalTarget], nextCursor: null };
          },
          getAssignmentSummary: async () => studentSummary,
        };
        document.body.replaceChildren();
        const skipToMain = document.createElement("a");
        skipToMain.href = "#main-content";
        skipToMain.textContent = "Skip to learning content";
        const mount = document.createElement("main");
        mount.id = "main-content";
        mount.tabIndex = -1;
        document.body.append(skipToMain, mount);
        render(() => createComponent(ApiRuntimeProvider, {
          runtime: { client },
          get children() {
            return createComponent(AssignmentList, {
              courseId,
              courseReference: "C-1",
              initialPage: { items: firstPage, nextCursor: "after-50" },
              reloadAssignments: async () => { reloads += 1; },
              canCreateAssignment: false,
            });
          },
        }), mount);
        window.__assignmentPaginationFixture = {
          requests,
          setScenario: (next) => { scenario = next; },
          reloads: () => reloads,
        };
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "course_assignments_pagination_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles.find((candidate) => candidate.path.endsWith(".js"));
  if (output === undefined)
    throw new Error("Assignment pagination fixture bundle was not produced.");
  fixtureBundle = output.text;
});

declare global {
  interface Window {
    __assignmentPaginationFixture: {
      readonly requests: string[];
      readonly setScenario: (scenario: "success" | "transport" | "protocol") => void;
      readonly reloads: () => number;
    };
  }
}

async function mountFixture(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("about:blank");
  await page.addScriptTag({ content: fixtureBundle });
  await expect(page.getByRole("button", { name: "Load more assignments" })).toBeVisible();
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
}

test("the production list helper reaches the exact 101st assignment through two keyboard pages", async ({
  page,
}) => {
  test.setTimeout(2_000);
  await mountFixture(page);
  const firstCard = page.locator(".course-card").first();
  await expect(firstCard).toContainText("3 questions in each new run.");
  await expect(firstCard).toContainText(
    "Progress: Current 72%, Latest 76%, Best 83%, 2 completed runs.",
  );
  const targetCard = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: "Assignment 101", exact: true }),
  });
  const target = targetCard.getByRole("link", { name: "Start assignment", exact: true });
  await expect(target).toHaveCount(0);
  await tabToTargetThroughVisiblePagination(page, {
    target,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page
        .locator(".course-card")
        .nth(index)
        .getByRole("link", { name: "Start assignment", exact: true }),
    itemName: "assignments",
  });
  await expect(target).toHaveCount(1);
  await expect(target).toBeFocused();
  await expect(page.locator(".course-card")).toHaveCount(101);
  await expect(page.getByText("Loaded 101 assignments.", { exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.__assignmentPaginationFixture.requests)).toEqual([
    "after-50",
    "after-100",
  ]);
});

test("transport and cursor protocol recovery controls receive focus after keyboard activation", async ({
  page,
}) => {
  await mountFixture(page);
  await page.evaluate(() => window.__assignmentPaginationFixture.setScenario("transport"));
  const loadMore = page.getByRole("button", { name: "Load more assignments" });
  const skipPagination = page.getByRole("link", { name: "Skip to load more assignments" });
  await tabTo(page, skipPagination);
  await expect(skipPagination).toBeFocused();
  await page.keyboard.press("Enter");
  const pagination = page.getByRole("region", { name: "Assignment pagination", exact: true });
  await expect(pagination).toHaveAttribute("id", "assignment-pagination");
  await expect(pagination).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(loadMore).toBeFocused();
  await page.keyboard.press("Enter");
  const retry = page.getByRole("button", { name: "Try loading more assignments again" });
  await expect(retry).toBeFocused();
  await expect(skipPagination).toBeVisible();
  await expect(skipPagination).toHaveAttribute("href", "#assignment-pagination");
  await expect(skipPagination).toHaveAttribute("target", "_self");
  await expect(retry).not.toHaveAttribute("id");
  await page.evaluate(() => window.__assignmentPaginationFixture.setScenario("success"));
  await retry.press("Enter");
  await expect(page.getByText("Assignment 51", { exact: true })).toBeVisible();

  await page.goto("about:blank");
  await page.addScriptTag({ content: fixtureBundle });
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
  await page.evaluate(() => window.__assignmentPaginationFixture.setScenario("protocol"));
  const protocolLoadMore = page.getByRole("button", { name: "Load more assignments" });
  const protocolSkipPagination = page.getByRole("link", { name: "Skip to load more assignments" });
  await tabTo(page, protocolSkipPagination);
  await expect(protocolSkipPagination).toBeFocused();
  await page.keyboard.press("Enter");
  const protocolPagination = page.getByRole("region", {
    name: "Assignment pagination",
    exact: true,
  });
  await expect(protocolPagination).toHaveAttribute("id", "assignment-pagination");
  await expect(protocolPagination).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(protocolLoadMore).toBeFocused();
  await page.keyboard.press("Enter");
  const reload = page.getByRole("button", { name: "Reload assignments" });
  await expect(reload).toBeFocused();
  await expect(protocolSkipPagination).toBeVisible();
  await expect(protocolSkipPagination).toHaveAttribute("href", "#assignment-pagination");
  await expect(protocolSkipPagination).toHaveAttribute("target", "_self");
  await expect(reload).not.toHaveAttribute("id");
  await reload.press("Enter");
  expect(await page.evaluate(() => window.__assignmentPaginationFixture.reloads())).toBe(1);
});
