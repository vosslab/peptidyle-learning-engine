// run_completion_summary.spec.ts - production RunPage completion-policy coverage.

import { expect, test, type Page } from "@playwright/test";
import { build, type Plugin } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const RUN_ID = "0198e000-0000-7000-8000-000000000840";

const routerPlugin = {
  name: "run-completion-summary-router",
  setup(buildApi): void {
    buildApi.onResolve({ filter: /^@solidjs\/router$/ }, () => ({
      path: "run-completion-summary-router",
      namespace: "run-completion-summary-router",
    }));
    buildApi.onLoad({ filter: /.*/, namespace: "run-completion-summary-router" }, () => ({
      contents: `
        import { createResource } from "solid-js";
        export function useParams() { return { runId: "${RUN_ID}" }; }
        export function useNavigate() { return () => {}; }
        export function createAsync(loader) { const [value] = createResource(loader); return value; }
        export function query(callback, key) {
          callback.key = key;
          callback.keyFor = () => key;
          return callback;
        }
      `,
      loader: "js",
      resolveDir: process.cwd(),
    }));
  },
} satisfies Plugin;

let fixtureScript = "";

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    platform: "browser",
    plugins: [solidPlugin(), routerPlugin],
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { publishedProblemFixture } from "./generated/fixtures/published_problem.ts";
        import { issuedEnvelopeForAttempt } from "./src/api/mock/handlers.ts";
        import { createHttpApiClient } from "./src/api/http_client.ts";
        import { createApiRuntime, ApiRuntimeProvider } from "./src/api/runtime.tsx";
        import { CourseThemeRouteContext } from "./src/features/course_appearance/course_theme_context.ts";
        import { RunPage } from "./src/pages/run_page.tsx";
        import { WasmRuntimeProvider } from "./src/wasm/context.tsx";

        const runId = "${RUN_ID}";
        const templateAttempt = publishedProblemFixture.attempts[0];
        const attempt = {
          ...templateAttempt,
          id: "0198e000-0000-7000-8000-000000000841",
          run: runId,
          response: null,
          result: null,
          timer: { ...templateAttempt.timer, submittedAt: null },
        };
        const run = {
          ...publishedProblemFixture.runs[0],
          id: runId,
          completedAt: null,
          score: null,
        };
        const screen = {
          course: {
            summary: publishedProblemFixture.course,
            appearance: { theme: "grass", revision: "1", banner: null },
          },
          assignment: publishedProblemFixture.assignment,
          run,
          attempt,
          issuedQuestion: issuedEnvelopeForAttempt(attempt),
        };
        const root = document.createElement("div");
        root.id = "run-completion-summary-fixture";
        document.body.append(root);
        let dispose = () => {};
        let staleRunScreenQueryCalls = 0;
        let freshRunScreenCalls = 0;
        let rejectStaleRunScreenQuery = false;

        function json(value) {
          return new Response(JSON.stringify(value), {
            headers: { "content-type": "application/json" },
          });
        }

        function mount(practiceAllowed) {
          dispose();
          staleRunScreenQueryCalls = 0;
          freshRunScreenCalls = 0;
          rejectStaleRunScreenQuery = false;
          const transport = async (input, init) => {
            const request = new Request(new URL(String(input), window.location.origin), init);
            const path = new URL(request.url).pathname;
            if (request.method === "POST" && path === "/api/attempts/" + attempt.id + "/prefetch-next") {
              return json(null);
            }
            if (request.method === "POST" && path === "/api/submissions/" + attempt.id) {
              return json({
                accepted: true,
                attempt: { ...attempt, response: { kind: "multipleChoice", selected: ["carbonyl"] } },
                feedback: { correctness: true },
                nextIssued: null,
              });
            }
            if (request.method === "GET" && path === "/api/runs/" + runId) {
              freshRunScreenCalls += 1;
              return json(run);
            }
            if (request.method === "GET" && path === "/api/enrollments/" + run.enrollment) {
              return json({
                enrollment: publishedProblemFixture.enrollment,
                summary: publishedProblemFixture.summary,
              });
            }
            if (request.method === "GET" && path === "/api/runs/" + runId + "/attempts") {
              return json({ items: [attempt], nextCursor: null });
            }
            if (request.method === "GET" && path === "/api/assignments/" + screen.assignment.id) {
              return json(screen.assignment);
            }
            if (request.method === "GET" && path === "/api/courses/" + screen.course.summary.id) {
              return json(screen.course.summary);
            }
            if (request.method === "GET" && path === "/api/courses/" + screen.course.summary.id + "/appearance") {
              return new Response(JSON.stringify(screen.course.appearance), {
                headers: {
                  "cache-control": "no-store",
                  "content-type": "application/json",
                  etag: '"1"',
                },
              });
            }
            if (request.method === "GET" && path === "/api/attempts/" + attempt.id + "/question") {
              return json(screen.issuedQuestion);
            }
            if (request.method === "GET" && path === "/api/runs/" + runId + "/summary") {
              if (practiceAllowed === null) return new Promise(() => {});
              return json({
                course: screen.course,
                run: { ...run, completedAt: 1786000004300, score: 1 },
                summary: publishedProblemFixture.summary,
                practiceAllowed,
                outcomes: { items: [], nextCursor: null },
              });
            }
            return json({ error: "unhandled fixture request" });
          };
          const client = createHttpApiClient({ fetch: transport });
          const runtime = createApiRuntime(client);
          runtime.queries.runScreen = async (requestedRunId) => {
            if (!rejectStaleRunScreenQuery) return client.getRunScreen(requestedRunId);
            staleRunScreenQueryCalls += 1;
            throw new Error("Continue must not use the router run-screen query");
          };
          dispose = render(
            () => createComponent(ApiRuntimeProvider, {
              runtime,
              get children() {
                return createComponent(WasmRuntimeProvider, {
                  formatFallback: async () => ({ violations: [] }),
                  timerFallback: async () => "open",
                  capabilityFallback: async () => [],
                  get children() {
                    return createComponent(CourseThemeRouteContext.Provider, {
                      value: { kind: "runAttempt", screen },
                      get children() { return createComponent(RunPage, {}); },
                    });
                  },
                });
              },
            }),
            root,
          );
        }
        window.__runCompletionSummaryFixture = {
          mount,
          armStaleRunScreenQuery: () => {
            rejectStaleRunScreenQuery = true;
          },
          staleRunScreenQueryCalls: () => staleRunScreenQueryCalls,
          freshRunScreenCalls: () => freshRunScreenCalls,
        };
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "run_completion_summary_fixture.tsx",
    },
    write: false,
  });
  fixtureScript = result.outputFiles[0]?.text ?? "";
});

interface RunCompletionSummaryFixture {
  mount(practiceAllowed: boolean | null): void;
  armStaleRunScreenQuery(): void;
  staleRunScreenQueryCalls(): number;
  freshRunScreenCalls(): number;
}

declare global {
  interface Window {
    __runCompletionSummaryFixture: RunCompletionSummaryFixture;
  }
}

async function mountCompletedRun(page: Page, practiceAllowed: boolean | null): Promise<void> {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });
  await page.evaluate(
    (allowed) => window.__runCompletionSummaryFixture.mount(allowed),
    practiceAllowed,
  );
  const fixture = page.locator("#run-completion-summary-fixture");
  await expect(fixture.getByText(/^Practice run/)).toBeVisible();
  await page.evaluate(() => window.__runCompletionSummaryFixture.armStaleRunScreenQuery());
  await fixture.locator('input[type="radio"]').first().check();
  await fixture.getByRole("button", { name: /submit answer/i }).click();
  await fixture.getByRole("button", { name: "Continue" }).click();
}

test("production RunPage shows neutral completion until the summary policy loads", async ({
  page,
}) => {
  await mountCompletedRun(page, null);
  const fixture = page.locator("#run-completion-summary-fixture");
  await expect(fixture.getByRole("heading", { name: "Run complete" })).toBeVisible();
  expect(
    await page.evaluate(() => window.__runCompletionSummaryFixture.staleRunScreenQueryCalls()),
  ).toBe(0);
  expect(
    await page.evaluate(() => window.__runCompletionSummaryFixture.freshRunScreenCalls()),
  ).toBe(0);
  await expect(fixture.getByRole("button", { name: "Start another practice run" })).toHaveCount(0);
  await expect(fixture.getByRole("button", { name: "Back to assignment" })).toBeVisible();
});

test("production RunPage offers fresh practice only when the summary allows it", async ({
  page,
}) => {
  await mountCompletedRun(page, true);
  const fixture = page.locator("#run-completion-summary-fixture");
  await expect(
    fixture.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible();
  await expect(fixture.getByRole("button", { name: "Start another practice run" })).toBeVisible();
  await expect(fixture.getByRole("button", { name: "Back to assignment" })).toBeVisible();
  expect(
    await page.evaluate(() => window.__runCompletionSummaryFixture.freshRunScreenCalls()),
  ).toBe(0);
});

test("production RunPage keeps a closed run neutral and preserves the Back action", async ({
  page,
}) => {
  await mountCompletedRun(page, false);
  const fixture = page.locator("#run-completion-summary-fixture");
  await expect(fixture.getByRole("heading", { name: "This run is complete" })).toBeVisible();
  await expect(
    fixture.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toHaveCount(0);
  await expect(fixture.getByRole("button", { name: "Start another practice run" })).toHaveCount(0);
  await expect(fixture.getByRole("button", { name: "Back to assignment" })).toBeVisible();
  expect(
    await page.evaluate(() => window.__runCompletionSummaryFixture.freshRunScreenCalls()),
  ).toBe(0);
});
