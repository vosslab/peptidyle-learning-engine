// question_renderer_security.spec.ts - browser proof for inert prompt and MathML projection.

import { expect, test } from "@playwright/test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const ASSET_ID = "00000000-0000-0000-0000-000000000001";

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
        import {
          QuestionRenderer,
          projectServerSanitizedMarkup,
        } from "./src/components/question_renderer.tsx";

        const assetId = "${ASSET_ID}";
        const hostile = [
          "<script>globalThis.rendererExecuted = true</script>",
          '<p onclick="globalThis.rendererExecuted = true">bad</p>',
          '<img src="https://attacker.example/pixel">',
          '<a ping="https://attacker.example/ping">bad</a>',
          '<iframe src="https://attacker.example/frame"></iframe>',
        ];
        const rejected = hostile.every((markup) => {
          try {
            projectServerSanitizedMarkup(markup);
            return false;
          } catch {
            return true;
          }
        });
        const safeMarkup = projectServerSanitizedMarkup(
          '<p>Safe supplied <strong>markup</strong>.</p><img alt="Diagram" data-asset-id="' + assetId + '">',
        );
        const envelope = {
          version: "00000000-0000-0000-0000-000000000002",
          seed: 7,
          prompt: [
            { kind: "math", latex: "\\\\frac{1}{2}", description: "one half" },
            { kind: "text", markdown: "Safe supplied markup." },
          ],
          response: { kind: "numeric", tolerance: { kind: "exact" }, unit: null },
        };
        const fixture = document.createElement("div");
        fixture.id = "renderer-fixture";
        fixture.dataset.hostileRejected = String(rejected);
        document.body.appendChild(fixture);
        render(
          () => createComponent(QuestionRenderer, {
            presentation: envelope,
            suppliedMarkup: [{
              promptIndex: 1,
              markup: safeMarkup,
              assets: new Map([[assetId, { asset: assetId, checksum: "a".repeat(64) }]]),
            }],
            assetUrl: (asset) => new URL("/api/assets/" + encodeURIComponent(asset.asset), location.origin),
          }),
          fixture,
        );
        const invalidMathFixture = document.createElement("div");
        invalidMathFixture.id = "invalid-math-fixture";
        document.body.appendChild(invalidMathFixture);
        render(
          () => createComponent(QuestionRenderer, {
            presentation: {
              ...envelope,
              prompt: [{ kind: "math", latex: "\\\\frac{1}", description: "broken fraction" }],
            },
            assetUrl: (asset) => new URL("/api/assets/" + encodeURIComponent(asset.asset), location.origin),
          }),
          invalidMathFixture,
        );
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "question_renderer_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("Renderer fixture bundle was not produced.");
  fixtureScript = output.text;
});

test("renderer projects hostile markup inertly and mounts semantic MathML", async ({ page }) => {
  const requests: string[] = [];
  const pageErrors: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.route(`**/api/assets/${ASSET_ID}`, (route) =>
    route.fulfill({
      body: "",
      contentType: "image/svg+xml",
    }),
  );
  await page.goto("/");
  requests.length = 0;

  await page.addScriptTag({ content: fixtureScript });

  const fixture = page.locator("#renderer-fixture");
  await expect.poll(() => pageErrors).toEqual([]);
  await expect(fixture).toHaveAttribute("data-hostile-rejected", "true");
  await expect(fixture.locator("math")).toHaveAttribute("aria-label", "one half");
  await expect(fixture.locator("math > mfrac > mn")).toHaveCount(2);
  await expect(fixture.getByText("Safe supplied markup.")).toBeVisible();
  await expect(page.locator("#invalid-math-fixture").getByRole("alert")).toContainText(
    "Please ask the instructor to correct its TeX.",
  );
  await expect(fixture.locator(`img[data-asset-id="${ASSET_ID}"]`)).toHaveAttribute(
    "src",
    `http://127.0.0.1:4173/api/assets/${ASSET_ID}`,
  );
  await expect.poll(() => requests).toContain(`http://127.0.0.1:4173/api/assets/${ASSET_ID}`);
  expect(requests.every((url) => new URL(url).origin === "http://127.0.0.1:4173")).toBe(true);
  await expect(page).not.toHaveURL(/attacker\.example/u);
  expect(
    await page.evaluate(
      () => (globalThis as typeof globalThis & { rendererExecuted?: boolean }).rendererExecuted,
    ),
  ).toBeUndefined();
});
