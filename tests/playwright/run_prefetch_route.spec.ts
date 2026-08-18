import { expect, test, type Page } from "@playwright/test";
import { build, type Plugin } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { classifyPostStartSurface } from "./simulator/post_start_surface";

const routerPlugin = {
  name: "prefetch-fixture-router",
  setup(api): void {
    api.onResolve({ filter: /^@solidjs\/router$/ }, () => ({
      path: "prefetch-fixture-router",
      namespace: "prefetch-fixture-router",
    }));
    api.onLoad({ filter: /.*/, namespace: "prefetch-fixture-router" }, () => ({
      loader: "js",
      resolveDir: process.cwd(),
      contents: `
        import { createResource } from "solid-js";
        export function useParams() { return { runId: window.__prefetchFixture.runId }; }
        export function useNavigate() { return () => {}; }
        export function createAsync(loader) { const [value] = createResource(loader); return value; }
        export function query(callback, key) { callback.key = key; callback.keyFor = () => key; return callback; }
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
    platform: "browser",
    plugins: [solidPlugin(), routerPlugin],
    stdin: {
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "run_prefetch_fixture.tsx",
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { publishedProblemFixture } from "./generated/fixtures/published_problem.ts";
        import { issuedQuestionWireForAttempt } from "./src/api/mock/handlers.ts";
        import { createHttpApiClient } from "./src/api/http_client.ts";
        import { createApiRuntime, ApiRuntimeProvider } from "./src/api/runtime.tsx";
        import { CourseThemeRouteContext } from "./src/features/course_appearance/course_theme_context.ts";
        import { WasmRuntimeProvider } from "./src/wasm/context.tsx";
        import { RunPage } from "./src/pages/run_page.tsx";

        const ids = { run30: "0198e000-0000-7000-8000-000000000730", run31: "0198e000-0000-7000-8000-000000000731", a30: "0198e000-0000-7000-8000-000000000732", b30: "0198e000-0000-7000-8000-000000000733", a31: "0198e000-0000-7000-8000-000000000734", b31: "0198e000-0000-7000-8000-000000000735" };
        const hash = (n) => String(n).repeat(64);
        const submissionCountKey = "prefetch-fixture-submission-count";
        const requests = [];
        const held = new Map();
        let heldSuccessorScreen = null;
        let runId = ids.run30;
        let active = { [ids.run30]: 0, [ids.run31]: 0 };
        let mode = window.__prefetchMode ?? "match";
        let screenMode = "exact";
        let submitted = false;
        const templateAttempt = publishedProblemFixture.attempts[0];
        const templateRun = publishedProblemFixture.runs[0];
        const makeRun = (id, number) => ({ ...templateRun, id, runNumber: number, completedAt: null, score: null });
        const attempt = (run, position) => ({ ...templateAttempt, id: run === ids.run30 ? (position === 0 ? ids.a30 : ids.b30) : (position === 0 ? ids.a31 : ids.b31), run, assignmentPosition: position, seed: run === ids.run30 ? 30 + position : 130 + position, response: null, result: null, timer: { ...templateAttempt.timer, submittedAt: null }, provenance: { ...templateAttempt.provenance, renderedQuestionSha256: hash(position === 0 ? "a" : "b") } });
        const screenAttempt = () => {
          const next = attempt(runId, active[runId]);
          if (!submitted) return next;
          if (screenMode === "same") return attempt(runId, 0);
          if (screenMode === "wrongId") return { ...next, id: runId === ids.run30 ? ids.a30 : ids.a31 };
          if (screenMode === "wrongRun") return { ...next, run: runId === ids.run30 ? ids.run31 : ids.run30 };
          if (screenMode === "wrongPosition") return { ...next, assignmentPosition: 0 };
          if (screenMode === "wrongVersion") return { ...next, questionVersion: "0198e000-0000-7000-8000-000000000799" };
          if (screenMode === "wrongSeed") return { ...next, seed: next.seed + 1 };
          if (screenMode === "wrongDeadline") return { ...next, timer: { ...next.timer, deadline: 99 } };
          if (screenMode === "wrongHash") return { ...next, provenance: { ...next.provenance, renderedQuestionSha256: hash("c") } };
          return next;
        };
        const envelope = (value) => { const base = issuedQuestionWireForAttempt(value); return { ...base, title: "Position " + (value.assignmentPosition + 1) + " / " + value.run.slice(-3), prompt: value.assignmentPosition === 1 ? [...base.prompt, ...Array.from({length: 14}, (_, i) => ({ kind: "image", asset: { asset: "0198e000-0000-7000-8000-" + String(900 + i).padStart(12, "0"), checksum: hash("a") }, description: "Warm asset " + i }))] : base.prompt }; };
        const json = (value, status = 200) => new Response(JSON.stringify(value), { status, headers: { "content-type": "application/json" } });
        const transport = async (input, init) => {
          const request = new Request(new URL(String(input), window.location.origin), init);
          const url = new URL(request.url); const path = url.pathname; const method = request.method;
          requests.push({ method, path, runId, body: method === "GET" ? null : await request.clone().text(), aborted: request.signal.aborted });
          if (method === "POST" && path.startsWith("/api/submissions/")) {
            const prior = Number(sessionStorage.getItem(submissionCountKey) ?? "0");
            sessionStorage.setItem(submissionCountKey, String(prior + 1));
          }
          const currentRun = makeRun(runId, runId === ids.run30 ? 30 : 31);
          const current = screenAttempt();
          if (method === "GET" && path === "/api/runs/" + runId) {
            if (submitted && screenMode === "hold") {
              return new Promise((resolve) => { heldSuccessorScreen = () => resolve(json(currentRun)); });
            }
            return json(currentRun);
          }
          if (method === "GET" && path === "/api/runs/" + runId + "/attempts") return json({ items: [current], nextCursor: null });
          if (method === "GET" && path === "/api/attempts/" + current.id + "/question") return json(envelope(current));
          if (method === "GET" && path === "/api/courses/" + publishedProblemFixture.course.id) return json(publishedProblemFixture.course);
          if (method === "GET" && path === "/api/courses/" + publishedProblemFixture.course.id + "/appearance") return new Response(JSON.stringify({ theme: "grass", revision: "1", banner: null }), { status: 200, headers: { "content-type": "application/json", "cache-control": "no-store", "etag": '"1"' } });
          if (method === "GET" && path === "/api/assignments/" + publishedProblemFixture.assignment.id) return json(publishedProblemFixture.assignment);
          if (method === "GET" && path === "/api/enrollments/" + publishedProblemFixture.enrollment.id) return json({ enrollment: publishedProblemFixture.enrollment, summary: publishedProblemFixture.summary });
          if (method === "POST" && path === "/api/attempts/" + current.id + "/prefetch-next") {
            if (mode === "outage") return json({ error: "temporary" }, 503);
            if (mode === "hold") return new Promise((resolve) => { held.set(runId, { resolve, signal: request.signal, next: attempt(runId, 1) }); });
            const next = attempt(runId, 1); const value = { predecessor: current.id, run: next.run, assignmentPosition: 1, questionVersion: next.questionVersion, seed: next.seed, renderedQuestionSha256: hash("b"), envelope: envelope(next) };
            if (mode === "wrongRun") value.run = ids.run31;
            if (mode === "wrongHash") value.renderedQuestionSha256 = hash("c");
            if (mode === "wrongVersion") value.questionVersion = "0198e000-0000-7000-8000-000000000799";
            if (mode === "wrongSeed") value.seed += 1;
            return json(value);
          }
          if (method === "POST" && path === "/api/submissions/" + current.id) {
            const next = attempt(runId, 1); active[runId] = 1; submitted = true;
            const nextIssued = mode === "pending" ? null : { id: next.id, run: next.run, questionVersion: next.questionVersion, seed: next.seed, deadline: null, assignmentPosition: 1, renderedQuestionSha256: hash("b") };
            return json({ accepted: true, attempt: { ...current, response: { kind: "multipleChoice", selected: ["0001"] } }, feedback: { correctness: true }, nextIssued, nextPending: mode === "pending" });
          }
          if (method === "GET" && path.startsWith("/api/assets/")) return new Response("asset", { status: 200 });
          return json({ error: "unhandled " + method + " " + path }, 404);
        };
        globalThis.fetch = transport;
        const client = createHttpApiClient({ fetch: transport });
        const runtime = createApiRuntime(client);
        let staleRunScreenQueryCalls = 0;
        let rejectStaleRunScreenQuery = false;
        runtime.queries.runScreen = async (requestedRunId) => {
          if (!rejectStaleRunScreenQuery) return client.getRunScreen(requestedRunId);
          staleRunScreenQueryCalls += 1;
          throw new Error("Continue must not use the router run-screen query");
        };
        const root = document.createElement("div"); root.id = "run-prefetch-fixture"; document.body.append(root);
        const mount = async () => {
          const routeScreen = await client.getRunScreen(runId);
          return render(() => createComponent(ApiRuntimeProvider, {
            runtime,
            get children() {
              return createComponent(WasmRuntimeProvider, {
                formatFallback: async () => ({ violations: [] }),
                timerFallback: async () => "open",
                capabilityFallback: async () => [],
                get children() {
                  return createComponent(CourseThemeRouteContext.Provider, {
                    value: { kind: "runAttempt", screen: routeScreen },
                    get children() { return createComponent(RunPage, {}); },
                  });
                },
              });
            },
          }), root);
        };
        let dispose = () => {};
        void mount().then((nextDispose) => { dispose = nextDispose; });
        window.__prefetchFixture = { get runId() { return runId; }, requests: () => requests, submissionCount: () => Number(sessionStorage.getItem(submissionCountKey) ?? "0"), armStaleRunScreenQuery: () => { rejectStaleRunScreenQuery = true; }, staleRunScreenQueryCalls: () => staleRunScreenQueryCalls, setMode: (value) => { mode = value; }, setScreenMode: (value) => { screenMode = value; }, releaseSuccessorScreen: () => { const release = heldSuccessorScreen; heldSuccessorScreen = null; if (release === null) return false; screenMode = "exact"; release(); return true; }, switchRun: (next) => { dispose(); runId = next; active[next] = 0; mode = "match"; screenMode = "exact"; submitted = false; void mount().then((nextDispose) => { dispose = nextDispose; }); }, settleHeld: (run) => { const entry = held.get(run); if (!entry) return false; const next = entry.next; entry.resolve(json({ predecessor: run === ids.run30 ? ids.a30 : ids.a31, run: next.run, assignmentPosition: 1, questionVersion: next.questionVersion, seed: next.seed, renderedQuestionSha256: hash("b"), envelope: envelope(next) })); return entry.signal.aborted; }, ids };
      `,
    },
    write: false,
  });
  fixtureScript = result.outputFiles[0]?.text ?? "";
});

interface PrefetchRequest {
  readonly method: string;
  readonly path: string;
  readonly body: string | null;
}
interface PrefetchFixture {
  readonly runId: string;
  readonly ids: { readonly run30: string; readonly run31: string };
  readonly requests: () => ReadonlyArray<PrefetchRequest>;
  readonly submissionCount: () => number;
  readonly armStaleRunScreenQuery: () => void;
  readonly staleRunScreenQueryCalls: () => number;
  readonly setMode: (mode: string) => void;
  readonly setScreenMode: (mode: string) => void;
  readonly releaseSuccessorScreen: () => boolean;
  readonly switchRun: (run: string) => void;
  readonly settleHeld: (run: string) => boolean;
}

declare global {
  interface Window {
    __prefetchFixture: PrefetchFixture;
    __prefetchMode?: string;
  }
}

async function mount(page: Page, mode = "match"): Promise<void> {
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.addInitScript((initialMode) => {
    window.__prefetchMode = initialMode;
    if (sessionStorage.getItem("prefetch-fixture-mounted") !== "true") {
      sessionStorage.removeItem("prefetch-fixture-submission-count");
      sessionStorage.setItem("prefetch-fixture-mounted", "true");
    }
  }, mode);
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });
  const heading = page
    .locator("#run-prefetch-fixture")
    .getByRole("heading", { name: /Position 1/ });
  await expect
    .poll(async () => {
      if (pageErrors[0] !== undefined) throw new Error(pageErrors[0]);
      return await heading.count();
    })
    .toBe(1);
}

test("a pending successor keeps feedback visible and offers a refresh without another submission", async ({
  page,
}) => {
  await mount(page, "pending");
  const root = page.locator("#run-prefetch-fixture");
  await root.locator('input[type="radio"]').first().check();
  await root.getByRole("button", { name: /submit answer/i }).click();
  await expect(root.getByRole("heading", { name: "Correct" })).toBeVisible();
  const refresh = root.getByRole("button", { name: "Refresh for the next question" });
  await expect(refresh).toBeVisible();
  const submissionsBeforeRefresh = await page.evaluate(() =>
    window.__prefetchFixture.submissionCount(),
  );
  expect(submissionsBeforeRefresh).toBe(1);

  const reloaded = page.waitForNavigation();
  await refresh.click();
  await reloaded;
  expect(
    await page.evaluate(() => sessionStorage.getItem("prefetch-fixture-submission-count")),
  ).toBe("1");
});

async function submitAndContinue(page: Page): Promise<void> {
  const root = page.locator("#run-prefetch-fixture");
  await root.locator('input[type="radio"]').first().check();
  await root.getByRole("button", { name: /submit answer/i }).click();
  await root.getByRole("button", { name: "Continue" }).click();
}

test("matching prefetch warms at most twelve assets and Continue advances without a next-screen fetch", async ({
  page,
}) => {
  await mount(page);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          window.__prefetchFixture.requests().filter((r) => r.path.endsWith("/prefetch-next"))
            .length,
      ),
    )
    .toBe(1);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          window.__prefetchFixture
            .requests()
            .filter((request) => request.path.startsWith("/api/assets/")).length,
      ),
    )
    .toBe(12);
  const warmed = await page.evaluate(() =>
    window.__prefetchFixture
      .requests()
      .filter((request) => request.path.startsWith("/api/assets/")),
  );
  expect(new Set(warmed.map((request) => request.path)).size).toBe(12);
  const browserStorage = await page.evaluate(() => ({
    local: Object.values(localStorage),
    session: Object.values(sessionStorage),
  }));
  expect(JSON.stringify(browserStorage)).not.toMatch(/prefetch|envelope|provider|provenance/i);
  const before = await page.evaluate(() => window.__prefetchFixture.requests().length);
  const root = page.locator("#run-prefetch-fixture");
  await root.locator('input[type="radio"]').first().check();
  await root.getByRole("button", { name: /submit answer/i }).click();
  await expect(root.getByRole("heading", { name: "Correct" })).toBeVisible();
  await root.getByRole("button", { name: "Continue" }).click();
  await expect(
    page.locator("#run-prefetch-fixture").getByRole("heading", { name: /Position 2/ }),
  ).toBeVisible();
  const evidence = await page.evaluate(() => window.__prefetchFixture.requests());
  const after = evidence.slice(before);
  expect(after.some((r) => r.path === "/api/runs/0198e000-0000-7000-8000-000000000730")).toBe(
    false,
  );
  expect(after.filter((r) => r.path.startsWith("/api/submissions/")).length).toBe(1);
  expect(JSON.stringify(evidence)).not.toMatch(/answer|key|provider|provenance/i);
});

test("an online event retries a failed prefetch and the recovered cache avoids fallback", async ({
  page,
}) => {
  await mount(page, "outage");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          window.__prefetchFixture.requests().filter((r) => r.path.endsWith("/prefetch-next"))
            .length,
      ),
    )
    .toBe(1);
  await page.evaluate(() => {
    window.__prefetchFixture.setMode("match");
    window.dispatchEvent(new Event("online"));
  });
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          window.__prefetchFixture.requests().filter((r) => r.path.endsWith("/prefetch-next"))
            .length,
      ),
    )
    .toBe(2);
  const before = await page.evaluate(() => window.__prefetchFixture.requests().length);
  await submitAndContinue(page);
  const after = (await page.evaluate(() => window.__prefetchFixture.requests())).slice(before);
  expect(
    after.some((request) => request.path === "/api/runs/0198e000-0000-7000-8000-000000000730"),
  ).toBe(false);
});

for (const mode of ["wrongRun", "wrongHash", "wrongVersion", "wrongSeed", "outage"]) {
  test(`${mode} prefetch falls back to the issued screen without losing feedback`, async ({
    page,
  }) => {
    await mount(page, mode);
    await submitAndContinue(page);
    await expect(
      page.locator("#run-prefetch-fixture").getByRole("heading", { name: /Position 2/ }),
    ).toBeVisible();
    const evidence = await page.evaluate(() => window.__prefetchFixture.requests());
    expect(
      evidence.filter((r) => r.path === "/api/runs/0198e000-0000-7000-8000-000000000730").length,
    ).toBeGreaterThan(1);
  });
}

test("fallback reads the fresh client screen, not a stale router query", async ({ page }) => {
  await mount(page, "outage");
  await page.evaluate(() => window.__prefetchFixture.armStaleRunScreenQuery());
  const initialRunScreenCalls = await page.evaluate(
    () =>
      window.__prefetchFixture
        .requests()
        .filter((request) => request.path === "/api/runs/0198e000-0000-7000-8000-000000000730")
        .length,
  );
  await submitAndContinue(page);
  await expect(
    page.locator("#run-prefetch-fixture").getByRole("heading", { name: /Position 2/ }),
  ).toBeVisible();
  expect(await page.evaluate(() => window.__prefetchFixture.staleRunScreenQueryCalls())).toBe(0);
  expect(
    await page.evaluate(
      () =>
        window.__prefetchFixture
          .requests()
          .filter((request) => request.path === "/api/runs/0198e000-0000-7000-8000-000000000730")
          .length,
    ),
  ).toBe(initialRunScreenCalls + 1);
});

test("a distinct retry attempt resets choice entry while the active attempt keeps its selection", async ({
  page,
}) => {
  await mount(page, "outage");
  const root = page.locator("#run-prefetch-fixture");
  const firstAttemptRadio = root.locator('input[type="radio"]').first();
  await firstAttemptRadio.check();
  await expect(firstAttemptRadio).toBeChecked();
  await root.getByRole("button", { name: /submit answer/i }).click();
  await root.getByRole("button", { name: "Continue" }).click();
  const retryRadio = root.locator('input[type="radio"]').first();
  await expect(retryRadio).not.toBeChecked();
});

test("fallback hides the submitted response until the successor screen is issued", async ({
  page,
}) => {
  await mount(page, "outage");
  await page.evaluate(() => window.__prefetchFixture.setScreenMode("hold"));
  const root = page.locator("#run-prefetch-fixture");
  const submittedRadio = root.locator('input[type="radio"]').first();
  await submittedRadio.check();
  await root.getByRole("button", { name: /submit answer/i }).click();
  await root.getByRole("button", { name: "Continue" }).click();

  const runSurface = root.locator("[data-route-surface=runAttempt]");
  await expect(runSurface).toHaveAttribute("aria-busy", "true");
  await expect(
    root.getByRole("status").filter({ hasText: "Loading the next question..." }),
  ).toBeVisible();
  await expect(root.locator('input[type="radio"]')).toHaveCount(0);
  await expect(root.getByRole("button", { name: /submit answer/i })).toHaveCount(0);
  expect(
    classifyPostStartSurface({
      radios: await root.getByRole("radio").count(),
      freshPractice: await root
        .getByRole("button", { name: "Start another practice" })
        .isVisible(),
      inlineErrors: await root.locator(".inline-error:visible").count(),
    }),
  ).toBe("pending");

  await expect
    .poll(() => page.evaluate(() => window.__prefetchFixture.releaseSuccessorScreen()))
    .toBe(true);
  await expect(root.getByRole("heading", { name: /Position 2/ })).toBeVisible();
  await expect(runSurface).toHaveAttribute("aria-busy", "false");
  const successorRadios = root.locator('input[type="radio"]');
  expect(await successorRadios.count()).toBeGreaterThan(1);
  expect(
    classifyPostStartSurface({
      radios: await root.getByRole("radio").count(),
      freshPractice: await root
        .getByRole("button", { name: "Start another practice" })
        .isVisible(),
      inlineErrors: await root.locator(".inline-error:visible").count(),
    }),
  ).toBe("run");
  for (const radio of await successorRadios.all()) await expect(radio).not.toBeChecked();
});

for (const descriptor of [
  "same",
  "wrongId",
  "wrongRun",
  "wrongPosition",
  "wrongVersion",
  "wrongSeed",
  "wrongDeadline",
  "wrongHash",
]) {
  test(`a ${descriptor} fresh screen recovers without replacing the submitted attempt`, async ({
    page,
  }) => {
    await mount(page, "outage");
    await page.evaluate((nextMode) => window.__prefetchFixture.setScreenMode(nextMode), descriptor);
    const root = page.locator("#run-prefetch-fixture");
    const firstAttemptRadio = root.locator('input[type="radio"]').first();
    await firstAttemptRadio.check();
    await root.getByRole("button", { name: /submit answer/i }).click();
    await root.getByRole("button", { name: "Continue" }).click();
    await expect(root.getByRole("button", { name: "Retry next question" })).toBeVisible();
    await expect(root.getByRole("heading", { name: /Position 1/ })).toBeVisible();
    await expect(firstAttemptRadio).toBeChecked();
  });
}

test("late run-30 prefetch is aborted on teardown and cannot affect fresh run 31", async ({
  page,
}) => {
  await mount(page, "hold");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          window.__prefetchFixture.requests().filter((r) => r.path.endsWith("/prefetch-next"))
            .length,
      ),
    )
    .toBe(1);
  await page.evaluate(() => window.__prefetchFixture.switchRun(window.__prefetchFixture.ids.run31));
  await expect(
    page.locator("#run-prefetch-fixture").getByRole("heading", { name: /731/ }),
  ).toBeVisible();
  expect(
    await page.evaluate(() =>
      window.__prefetchFixture.settleHeld(window.__prefetchFixture.ids.run30),
    ),
  ).toBe(true);
  await submitAndContinue(page);
  await expect(
    page.locator("#run-prefetch-fixture").getByRole("heading", { name: /Position 2 \/ 731/ }),
  ).toBeVisible();
});
