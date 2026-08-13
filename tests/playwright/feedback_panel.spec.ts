// feedback_panel.spec.ts - mounted disclosure, ordering, inertness, and keyboard acceptance proof.

import { expect, test } from "@playwright/test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

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
        import { FeedbackPanel } from "./src/components/feedback_panel.tsx";

        const assetUrl = (asset) => new URL("/api/assets/" + asset.asset, window.location.origin);
        const learnerResponse = [{ kind: "text", markdown: "<img src=x onerror=window.injected=true>My response" }];
        const full = {
          correctness: false,
          pointsEarned: 2,
          pointsPossible: 3,
          hint: [{ kind: "text", markdown: "Review peptide planarity." }],
          correctResponse: [{ kind: "text", markdown: "A planar peptide bond." }],
          rationale: [{ kind: "text", markdown: "Resonance restricts rotation." }],
        };
        const mount = document.createElement("div");
        mount.id = "feedback-fixture";
        document.body.appendChild(mount);
        const escapeFocus = document.createElement("button");
        escapeFocus.id = "escape-focus";
        escapeFocus.type = "button";
        escapeFocus.textContent = "Keep my focus here";
        document.body.appendChild(escapeFocus);
        render(() => createComponent(FeedbackPanel, {
          disclosure: { kind: "released", feedback: full },
          learnerResponse,
          assetUrl,
          focusAdvanceDelayMs: 1000,
          onAdvance: () => { document.body.dataset.advanceCount = "1"; },
        }), mount);
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "feedback_panel_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("Feedback fixture bundle was not produced.");
  fixtureScript = output.text;
});

test("full released feedback has a pedagogical reading order and inert learner text", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });

  const panel = page.locator("#feedback-fixture");
  await expect(panel.getByRole("heading", { name: "Feedback" })).toBeFocused();
  await expect(panel.getByRole("status")).toContainText("Feedback released. Not quite.");
  await expect(
    panel.getByText("<img src=x onerror=window.injected=true>My response"),
  ).toBeVisible();
  await expect(panel.locator("img")).toHaveCount(0);
  await expect(panel).toContainText("Score: 2 / 3");

  const sectionTitles = await panel.locator("h3").allTextContents();
  expect(sectionTitles).toEqual([
    "Your response",
    "Not quite",
    "Hint",
    "Correct response",
    "Why this works",
  ]);
  expect(await page.evaluate(() => "injected" in globalThis)).toBe(false);
});

test("a released panel focuses its heading, then its advance after the announcement delay", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({
    content: fixtureScript.replace("focusAdvanceDelayMs: 1000", "focusAdvanceDelayMs: 25"),
  });

  const panel = page.locator("#feedback-fixture");
  await expect(panel.getByRole("heading", { name: "Feedback" })).toBeFocused();
  await expect(panel.getByRole("button", { name: "Continue" })).toBeFocused();
});

test("a learner focus move during the announcement delay is never stolen", async ({ page }) => {
  await page.goto("/");
  await page.clock.install();
  await page.addScriptTag({
    content: fixtureScript.replace("focusAdvanceDelayMs: 1000", "focusAdvanceDelayMs: 100"),
  });

  const panel = page.locator("#feedback-fixture");
  await expect(panel.getByRole("heading", { name: "Feedback" })).toBeFocused();
  const escapeFocus = page.locator("#escape-focus");
  await escapeFocus.focus();
  await page.clock.runFor(150);
  await expect(escapeFocus).toBeFocused();
});

test("correctness-only release cannot show an answer or rationale and the advance is keyboard safe", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({
    content: fixtureScript.replace(
      'disclosure: { kind: "released", feedback: full },',
      'disclosure: { kind: "released", feedback: { correctness: true } },',
    ),
  });

  const panel = page.locator("#feedback-fixture");
  await expect(panel.getByRole("heading", { name: "Correct", exact: true })).toBeVisible();
  await expect(panel.getByRole("heading", { name: "Correct response" })).toHaveCount(0);
  await expect(panel.getByRole("heading", { name: "Why this works" })).toHaveCount(0);
  await expect(panel.getByText("No additional feedback was provided.")).toBeVisible();

  const advance = panel.getByRole("button", { name: "Continue" });
  await advance.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("body")).toHaveAttribute("data-advance-count", "1");
});
