import { expect, test, type Page } from "@playwright/test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

interface ExternalToolFixture {
  readonly attemptId: string;
  readonly launchUrl: string;
  readonly launchCalls: () => number;
  readonly submissionRequests: () => ReadonlyArray<{
    readonly path: string;
    readonly body: string;
    readonly idempotencyKey: string | null;
  }>;
  readonly changes: () => ReadonlyArray<{ readonly kind: "externalTool" }>;
  readonly setMode: (mode: "ready" | "outage" | "unsafe") => void;
}

declare global {
  interface Window {
    externalToolFixture: ExternalToolFixture;
  }
}

let fixtureScript = "";

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    plugins: [solidPlugin()],
    platform: "browser",
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { createHttpApiClient } from "./src/api/http_client.ts";
        import { mockExternalToolSubmissionReceipt } from "./src/api/mock/handlers.ts";
        import { ResponseWidget } from "./src/components/response_widget.tsx";

        const root = document.createElement("div");
        root.id = "external-tool-fixture";
        document.documentElement.append(root);
        let launchCalls = 0;
        let mode = "ready";
        const submissionRequests = [];
        const changes = [];
        const attemptId = "0198e000-0000-7000-8000-000000000034";
        const launchUrl = "/api/attempts/" + attemptId + "/external-tool/launch";
        const validator = {
          mode: "wasm",
          validateResponseFormat: async () => ({ violations: [] }),
        };
        const launch = async () => {
          launchCalls += 1;
          if (mode === "outage") throw new Error("broker unavailable");
          if (mode === "unsafe") return { launchUrl: "https://provider.example/launch" };
          return { launchUrl };
        };
        const client = createHttpApiClient({
          fetch: async (input, init) => {
            const request = new Request(new URL(input.toString(), window.location.origin), init);
            submissionRequests.push({
              path: new URL(request.url).pathname,
              body: await request.text(),
              idempotencyKey: request.headers.get("idempotency-key"),
            });
            return new Response(JSON.stringify(mockExternalToolSubmissionReceipt()), {
              status: 200,
              headers: { "content-type": "application/json" },
            });
          },
        });
        render(() => createComponent(ResponseWidget, {
          attemptId,
          definition: { kind: "externalTool" },
          validator,
          getExternalToolLaunch: launch,
          onResponseChange: (response) => changes.push(response),
          onSubmit: async (response) => client.submitResponse(attemptId, response, "mounted-external-key"),
          onEscape: () => {},
        }), root);
        window.externalToolFixture = {
          attemptId,
          launchUrl,
          launchCalls: () => launchCalls,
          submissionRequests: () => submissionRequests,
          changes: () => changes,
          setMode: (next) => { mode = next; },
        };
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "external_tool_response_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("External-tool fixture bundle was not produced.");
  fixtureScript = output.text;
});

async function mountFixture(page: Page): Promise<void> {
  await page.goto("/");
  await page.waitForTimeout(100);
  await page.addScriptTag({ content: fixtureScript });
}

async function dispatchReadiness(
  page: Page,
  data: unknown,
  origin: string,
  fromFrame: boolean,
): Promise<void> {
  await page.evaluate(
    ({ message, eventOrigin, useFrame }) => {
      const frame = document.querySelector<HTMLIFrameElement>("iframe.external-tool-frame");
      const source = useFrame ? (frame?.contentWindow ?? null) : window;
      window.dispatchEvent(
        new MessageEvent("message", { data: message, origin: eventOrigin, source }),
      );
    },
    { message: data, eventOrigin: origin, useFrame: fromFrame },
  );
}

test("external-tool activation is local, readiness is strict, and submission is marker-only", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#external-tool-fixture");
  const attemptId = "0198e000-0000-7000-8000-000000000034";

  await expect(fixture.getByRole("button", { name: "Open learning tool" })).toBeVisible();
  await expect(fixture.locator("iframe")).toHaveCount(0);
  expect(await page.evaluate(() => window.externalToolFixture.launchCalls())).toBe(0);

  await fixture.getByRole("button", { name: "Open learning tool" }).click();
  await expect(fixture.locator("iframe.external-tool-frame")).toHaveAttribute(
    "src",
    `/api/attempts/${attemptId}/external-tool/launch`,
  );
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeDisabled();
  expect(await page.evaluate(() => window.externalToolFixture.launchCalls())).toBe(1);
  expect(await page.evaluate(() => window.externalToolFixture.changes())).toEqual([
    { kind: "externalTool" },
  ]);

  await dispatchReadiness(
    page,
    { kind: "ple.externalTool.ready", attemptId },
    page.url().replace(/\/$/, ""),
    false,
  );
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeDisabled();

  await dispatchReadiness(
    page,
    { kind: "ple.externalTool.ready", attemptId },
    "https://foreign.example",
    true,
  );
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeDisabled();

  await dispatchReadiness(
    page,
    { kind: "ple.externalTool.ready", attemptId, score: 1 },
    page.url().replace(/\/$/, ""),
    true,
  );
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeDisabled();

  await dispatchReadiness(
    page,
    { kind: "ple.externalTool.ready", attemptId },
    page.url().replace(/\/$/, ""),
    true,
  );
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeEnabled();
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeFocused();
  await fixture.getByRole("button", { name: "Submit answer" }).click();
  await expect
    .poll(async () => page.evaluate(() => window.externalToolFixture.submissionRequests().length))
    .toBe(1);
  expect(await page.evaluate(() => window.externalToolFixture.submissionRequests())).toEqual([
    {
      path: `/api/attempts/${attemptId}/external-tool/launch/submission`,
      body: JSON.stringify({ response: { kind: "externalTool" } }),
      idempotencyKey: "mounted-external-key",
    },
  ]);
});

test("external-tool broker outage remains local and retries only after user action", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#external-tool-fixture");
  await page.evaluate(() => window.externalToolFixture.setMode("outage"));

  await fixture.getByRole("button", { name: "Open learning tool" }).click();
  await expect(fixture.getByRole("status")).toContainText("learning tool is unavailable");
  await expect(fixture.locator("iframe")).toHaveCount(0);
  await expect(fixture.getByRole("button", { name: "Retry learning tool" })).toBeVisible();
  expect(await page.evaluate(() => window.externalToolFixture.launchCalls())).toBe(1);
});

test("external-tool surface refuses an off-origin launch value", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#external-tool-fixture");
  await page.evaluate(() => window.externalToolFixture.setMode("unsafe"));

  await fixture.getByRole("button", { name: "Open learning tool" }).click();
  await expect(fixture.getByRole("status")).toContainText("route was not safe");
  await expect(fixture.locator("iframe")).toHaveCount(0);
  await expect(fixture.getByRole("button", { name: "Submit answer" })).toBeDisabled();
});
