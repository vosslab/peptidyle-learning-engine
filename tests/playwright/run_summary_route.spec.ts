import { expect, test } from "@playwright/test";
import { build, type Plugin } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const RUN_ID = "0198e000-0000-7000-8000-000000000023";

/** The fixture supplies only routing context; the mounted page and HTTP client are production code. */
const runSummaryFixtureRouterPlugin = {
  name: "run-summary-fixture-router",
  setup(buildApi): void {
    buildApi.onResolve({ filter: /^@solidjs\/router$/ }, () => ({
      path: "run-summary-fixture-router",
      namespace: "run-summary-fixture-router",
    }));
    buildApi.onLoad({ filter: /.*/, namespace: "run-summary-fixture-router" }, () => ({
      contents: `
        export function useParams() { return { runId: "0198e000-0000-7000-8000-000000000023" }; }
        export function useNavigate() {
          return (path) => { window.__runSummaryBrowserFixture.navigations.push(path); };
        }
        export function query(callback, key) {
          callback.key = key;
          callback.keyFor = () => key;
          return callback;
        }
      `,
      loader: "js",
    }));
  },
} satisfies Plugin;

interface RunSummaryFixtureRequest {
  readonly method: string;
  readonly path: string;
  readonly cursor: string | null;
  readonly pageSize: string | null;
}

interface RunSummaryFixtureEvidence {
  readonly requests: ReadonlyArray<RunSummaryFixtureRequest>;
  readonly maximumConcurrentRequests: number;
  readonly activeRequests: number;
  readonly navigations: ReadonlyArray<string>;
}

interface RunSummaryFixtureWindow {
  readonly __runSummaryBrowserFixture: {
    readonly requests: ReadonlyArray<RunSummaryFixtureRequest>;
    readonly maximumConcurrentRequests: () => number;
    readonly activeRequests: () => number;
    readonly navigations: ReadonlyArray<string>;
  };
}

let runSummaryFixture = "";

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    plugins: [solidPlugin(), runSummaryFixtureRouterPlugin],
    platform: "browser",
    outdir: "/tmp/ple-run-summary-browser-fixture",
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { publishedProblemFixture } from "./generated/fixtures/published_problem.ts";
        import { createHttpApiClient } from "./src/api/http_client.ts";
        import { ApiRuntimeProvider } from "./src/api/runtime.tsx";
        import { CourseThemeRouteContext } from "./src/features/course_appearance/course_theme_context.ts";
        import { RunSummaryPage } from "./src/pages/run_summary_page.tsx";

        const runId = "0198e000-0000-7000-8000-000000000023";
        const nextCursor = "summary:30";
        const requests = [];
        const navigations = [];
        let activeRequests = 0;
        let maximumConcurrentRequests = 0;
        let failSecondPageOnce = true;
        let failStartOnce = true;
        const run = {
          ...publishedProblemFixture.runs[1],
          id: runId,
          completedAt: 1786000004300,
          score: 1,
        };
        const outcomes = Array.from({ length: 31 }, (_, index) => ({
          attempt: "0198e000-0000-7000-8000-" + String(800 + index).padStart(12, "0"),
          assignmentPosition: index,
          submittedAt: 1786000004400 + index,
          response: publishedProblemFixture.attempts[0].response,
          feedback: null,
        }));

        function summaryPage(cursor) {
          const course = {
            summary: publishedProblemFixture.course,
            appearance: { theme: "grass", revision: "1", banner: null },
          };
          if (cursor === null) {
            return {
              course,
              run,
              summary: publishedProblemFixture.summary,
              practiceAllowed: true,
              outcomes: { items: outcomes.slice(0, 30), nextCursor },
            };
          }
          if (cursor === nextCursor) {
            return {
              course,
              run,
              summary: publishedProblemFixture.summary,
              practiceAllowed: true,
              outcomes: { items: outcomes.slice(30), nextCursor: null },
            };
          }
          return null;
        }

        const transport = async (input, init) => {
          const url = new URL(String(input), window.location.origin);
          const method = init?.method ?? "GET";
          const cursor = url.searchParams.get("cursor");
          requests.push({ method, path: url.pathname, cursor, pageSize: url.searchParams.get("pageSize") });
          activeRequests += 1;
          maximumConcurrentRequests = Math.max(maximumConcurrentRequests, activeRequests);
          await new Promise((resolve) => window.setTimeout(resolve, 4));
          activeRequests -= 1;
          if (method === "GET" && url.pathname === "/api/runs/" + runId + "/summary") {
            if (cursor === nextCursor && failSecondPageOnce) {
              failSecondPageOnce = false;
              return new Response(JSON.stringify({ error: "temporary summary page failure" }), { status: 503 });
            }
            const page = summaryPage(cursor);
            return page === null
              ? new Response(JSON.stringify({ error: "unknown cursor" }), { status: 400 })
              : new Response(JSON.stringify(page), { headers: { "content-type": "application/json" } });
          }
          if (method === "GET" && url.pathname === "/api/enrollments/" + run.enrollment) {
            return new Response(JSON.stringify({ enrollment: publishedProblemFixture.enrollment, summary: publishedProblemFixture.summary }), {
              headers: { "content-type": "application/json" },
            });
          }
          if (method === "POST" && url.pathname === "/api/runs") {
            if (failStartOnce) {
              failStartOnce = false;
              return new Response(JSON.stringify({ error: "temporary start failure" }), { status: 503 });
            }
            return new Response(JSON.stringify({
              ...run,
              id: "0198e000-0000-7000-8000-000000000024",
              reference: "R-5",
              runNumber: 5,
              startedAt: 1786000005000,
              completedAt: null,
              score: null,
              mode: "practice",
            }), { headers: { "content-type": "application/json" } });
          }
          return new Response(JSON.stringify({ error: "fixture route not found" }), { status: 404 });
        };

        window.__runSummaryBrowserFixture = {
          requests,
          maximumConcurrentRequests: () => maximumConcurrentRequests,
          activeRequests: () => activeRequests,
          navigations,
        };
        const client = createHttpApiClient({ fetch: transport });
        const mount = document.createElement("div");
        mount.id = "run-summary-fixture";
        document.body.appendChild(mount);
        void client.getRunSummary(runId, undefined, 30).then((initialSummary) => {
          render(() => createComponent(ApiRuntimeProvider, {
            runtime: { client },
            get children() {
              return createComponent(CourseThemeRouteContext.Provider, {
                value: { kind: "runSummary", response: initialSummary },
                get children() { return createComponent(RunSummaryPage, {}); },
              });
            },
          }), mount);
        });
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "run_summary_browser_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles.find((candidate) => candidate.path.endsWith(".js"));
  if (output === undefined) throw new Error("Run-summary browser fixture bundle was not produced.");
  runSummaryFixture = output.text;
});

test("direct run-summary route appends the bounded 31st outcome without duplicates", async ({
  page,
}) => {
  const requests: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/summary")) requests.push(request.url());
  });
  await page.goto("/");
  await page.evaluate((runId) => {
    history.pushState({}, "", `/runs/${runId}/summary`);
    dispatchEvent(new PopStateEvent("popstate"));
  }, "R-4");
  await expect(page.getByRole("heading", { name: "Run summary" })).toBeVisible();
  await expect(page.locator(".feedback-panel")).toHaveCount(30);
  await page.getByRole("button", { name: "Load more responses" }).click();
  await expect(page.locator(".feedback-panel")).toHaveCount(31);
  await expect(page.getByRole("button", { name: "Load more responses" })).toHaveCount(0);
  expect(requests).toEqual([]);
  const labels = await page
    .locator(".feedback-panel")
    .evaluateAll((panels) => panels.map((panel) => panel.textContent));
  expect(new Set(labels).size).toBeGreaterThan(1);
});

test("built run summary retries a bounded cursor and a fresh-practice start without losing outcomes", async ({
  page,
}) => {
  const pageErrors: Array<string> = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.goto("/");
  await page.addScriptTag({ content: runSummaryFixture });

  const fixture = page.locator("#run-summary-fixture");
  await expect(fixture).toBeAttached();
  expect(pageErrors).toEqual([]);
  await expect(fixture.getByRole("heading", { name: "Run summary" })).toBeVisible();
  await expect(fixture.locator(".feedback-panel")).toHaveCount(30);
  await expect(fixture.locator(".feedback-panel button")).toHaveCount(0);
  const feedbackHeadingIds = await fixture
    .locator(".feedback-panel__heading")
    .evaluateAll((headings) => headings.map((heading) => heading.id));
  expect(new Set(feedbackHeadingIds).size).toBe(feedbackHeadingIds.length);
  await expect(fixture.getByRole("button", { name: "Start fresh practice" })).toBeVisible();
  await expect(fixture.getByText("Feedback is not available yet.").first()).toBeVisible();

  await fixture.getByRole("button", { name: "Load more responses" }).click();
  await expect(fixture.getByText("Could not load more responses")).toBeVisible();
  await expect(fixture.locator(".feedback-panel")).toHaveCount(30);
  await fixture.getByRole("button", { name: "Retry", exact: true }).click();
  await expect(fixture.locator(".feedback-panel")).toHaveCount(31);
  await expect(fixture.getByRole("button", { name: "Load more responses" })).toHaveCount(0);

  const outcomeStatuses = await fixture
    .locator(".feedback-panel [role='status']")
    .allTextContents();
  expect(outcomeStatuses).toHaveLength(31);
  expect(new Set(outcomeStatuses).size).toBe(1);

  await fixture.getByRole("button", { name: "Start fresh practice" }).click();
  await expect(fixture.getByText("Could not start a fresh practice run")).toBeVisible();
  await expect(fixture.locator(".feedback-panel")).toHaveCount(31);
  await fixture.getByRole("button", { name: "Retry starting practice" }).click();
  await page.waitForFunction(() => {
    const fixtureWindow = window as unknown as RunSummaryFixtureWindow;
    return fixtureWindow.__runSummaryBrowserFixture.navigations.length === 1;
  });

  const fixtureEvidence = await page.evaluate((): RunSummaryFixtureEvidence => {
    const fixtureWindow = window as unknown as RunSummaryFixtureWindow;
    return {
      requests: fixtureWindow.__runSummaryBrowserFixture.requests,
      maximumConcurrentRequests:
        fixtureWindow.__runSummaryBrowserFixture.maximumConcurrentRequests(),
      activeRequests: fixtureWindow.__runSummaryBrowserFixture.activeRequests(),
      navigations: fixtureWindow.__runSummaryBrowserFixture.navigations,
    };
  });
  const summaryRequests = fixtureEvidence.requests.filter(
    (request) => request.path === `/api/runs/${RUN_ID}/summary`,
  );
  expect(summaryRequests).toEqual([
    { method: "GET", path: `/api/runs/${RUN_ID}/summary`, cursor: null, pageSize: "30" },
    {
      method: "GET",
      path: `/api/runs/${RUN_ID}/summary`,
      cursor: "summary:30",
      pageSize: "30",
    },
    {
      method: "GET",
      path: `/api/runs/${RUN_ID}/summary`,
      cursor: "summary:30",
      pageSize: "30",
    },
  ]);
  expect(fixtureEvidence.maximumConcurrentRequests).toBe(1);
  expect(fixtureEvidence.activeRequests).toBe(0);
  expect(fixtureEvidence.requests.every((request) => !request.path.includes("offset"))).toBe(true);
  expect(
    fixtureEvidence.requests.filter((request) => request.path.includes("feedback-release")),
  ).toEqual([]);
  expect(fixtureEvidence.navigations).toEqual(["/runs/R-5"]);
});
