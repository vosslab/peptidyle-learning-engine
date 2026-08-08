// feedback_submission_flow.spec.ts - browser proof that server disclosure drives the mounted panel.

import { expect, test, type Page } from "@playwright/test";
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
        import { createEffect, createSignal, createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { createMockApiClient } from "./src/api/mock/client.ts";
        import { mockAttemptById } from "./src/api/mock/handlers.ts";
        import { createAttemptStateMachine } from "./src/features/attempt/attempt_state.ts";
        import { FeedbackPanel } from "./src/components/feedback_panel.tsx";

        const root = document.createElement("div");
        root.id = "submission-flow";
        // The application shell owns the body and can replace it during route
        // commits; the independent acceptance fixture must survive that work.
        document.documentElement.append(root);
        const [feedback, setFeedback] = createSignal(null);
        let disposePanel;
        createEffect(() => {
          const state = feedback();
          if (state === null) return;
          disposePanel?.();
          const disclosure = state.feedback.kind === "released"
            ? { kind: "released", feedback: state.feedback.feedback }
            : { kind: "awaiting", feedback: null };
          disposePanel = render(() => createComponent(FeedbackPanel, {
            disclosure,
            assetUrl: (asset) => new URL("/api/assets/" + asset.asset, window.location.origin),
            onAdvance: () => {},
            focusAdvanceDelayMs: 1,
          }), root);
        });
        window.mountSubmissionFeedback = async (attemptId) => {
          const attempt = mockAttemptById(attemptId);
          if (!attempt) throw new Error("missing fixture attempt");
          const client = createMockApiClient();
          const machine = createAttemptStateMachine({
            context: {
              tenantId: attempt.tenant,
              runId: attempt.run,
              attemptId: attempt.id,
              questionVersion: attempt.questionVersion,
              seed: attempt.seed,
              deadline: attempt.timer.deadline,
            },
            storage: { getItem: () => null, setItem: () => {}, removeItem: () => {} },
            clock: { now: () => Date.now() },
            network: { isOnline: () => true },
            generateIdempotencyKey: () => "mounted-flow-key",
            submitResponse: client.submitResponse,
            isSessionExpired: () => false,
            onStateChange: (state) => {
              if (state.phase === "feedback") setFeedback(state);
            },
          });
          machine.start();
          machine.setResponse({ kind: "externalTool" }, { valid: true, message: null });
          await machine.submit();
        };
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "feedback_submission_flow_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("Submission-flow fixture bundle was not produced.");
  fixtureScript = output.text;
});

async function mountScenario(page: Page, attemptId: string): Promise<void> {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  // Let the application shell finish its first route commit before mounting an
  // independent fixture root beside it.
  await page.waitForTimeout(100);
  await page.addScriptTag({ content: fixtureScript });
  if (errors.length > 0) throw new Error(errors.join("\n"));
  await page.evaluate(
    async (id) =>
      (
        window as unknown as { mountSubmissionFeedback: (attemptId: string) => Promise<void> }
      ).mountSubmissionFeedback(id),
    attemptId,
  );
}

test("correctness-only mock feedback permits its hint but no full-disclosure sections", async ({
  page,
}) => {
  await mountScenario(page, "0198e000-0000-7000-8000-000000000030");
  const panel = page.locator("#submission-flow");
  await expect(panel.getByRole("heading", { name: "Not quite", exact: true })).toBeVisible();
  await expect(panel.getByText("Review the prompt and try another variation.")).toBeVisible();
  await expect(panel.getByText(/Score:|Points earned:|Points possible:/)).toHaveCount(0);
  await expect(panel.getByRole("heading", { name: "Correct response" })).toHaveCount(0);
  await expect(panel.getByRole("heading", { name: "Why this works" })).toHaveCount(0);
});

for (const [label, attemptId] of [
  ["deferred", "0198e000-0000-7000-8000-000000000032"],
  ["on-release", "0198e000-0000-7000-8000-000000000033"],
] as const) {
  test(`${label} mock feedback remains explicitly awaiting`, async ({ page }) => {
    await mountScenario(page, attemptId);
    const panel = page.locator("#submission-flow");
    await expect(panel.getByRole("status")).toHaveText(
      "Your response was recorded. Feedback is not available yet.",
    );
    await expect(panel.getByRole("heading", { name: "Hint" })).toHaveCount(0);
    await expect(panel.getByRole("heading", { name: "Correct response" })).toHaveCount(0);
    await expect(panel.getByRole("heading", { name: "Why this works" })).toHaveCount(0);
  });
}
