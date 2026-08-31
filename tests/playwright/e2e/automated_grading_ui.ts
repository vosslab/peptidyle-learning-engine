// Visible automated-grading helpers for connected student journeys.
//
// Selector contract:
// - src/pages/assignment_attempt_page.tsx owns the Assignment Attempt identity surface.
// - src/pages/assignment_attempt_page.tsx owns successor retry.
// - src/pages/assignment_attempt_page.tsx owns pending status and its visible refresh action.
import { expect, type Locator, type Page } from "@playwright/test";

const AUTOMATED_GRADING_DEADLINE_MS = 150_000;
const AUTOMATED_GRADING_POLL_MS = 2_000;

export async function waitForAutomatedFeedback(page: Page): Promise<Locator> {
  const feedback = page.getByRole("heading", { name: "Feedback", exact: true }).locator("..");
  const pending = page
    .getByRole("heading", { name: "Response received", exact: true })
    .locator("..");
  const nonCurrentFeedback = page.getByRole("heading", {
    name: /^(Score is being updated|Score update needs attention)$/u,
    exact: true,
  });

  // Accept either a visible pending action or feedback because the worker may
  // complete between the submission click and the first browser observation.
  await expect
    .poll(
      async () => {
        if (await feedback.isVisible()) return "feedback";
        const checkStatus = pending.getByRole("button", {
          name: "Check grading status",
          exact: true,
        });
        return (await pending.isVisible()) && (await checkStatus.isVisible())
          ? "pending"
          : "waiting";
      },
      { timeout: AUTOMATED_GRADING_DEADLINE_MS, intervals: [AUTOMATED_GRADING_POLL_MS] },
    )
    .toMatch(/^(feedback|pending)$/u);

  // Status refresh is an idempotent visible action; keep driving it until the
  // server-owned terminal feedback replaces the pending projection.
  await expect
    .poll(
      async () => {
        if ((await feedback.isVisible()) && !(await nonCurrentFeedback.isVisible())) return true;
        const checkStatus = page.getByRole("button", {
          name: /^(Check grading status|Check for updated score)$/u,
          exact: true,
        });
        if ((await checkStatus.isVisible()) && (await checkStatus.isEnabled())) {
          await checkStatus.click();
        }
        return (await feedback.isVisible()) && !(await nonCurrentFeedback.isVisible());
      },
      { timeout: AUTOMATED_GRADING_DEADLINE_MS, intervals: [AUTOMATED_GRADING_POLL_MS] },
    )
    .toBe(true);
  return feedback;
}

export async function advanceToNextIssuedQuestion(page: Page): Promise<void> {
  const runSurface = page.locator('[data-route-surface="runAttempt"]');
  const predecessor = await runSurface.getAttribute("data-attempt-id");
  if (predecessor === null || predecessor.length === 0) {
    throw new Error("run-attempt surface is missing its stable attempt identity");
  }
  const advance = page.getByRole("button", {
    name: /^(Continue|Refresh for the next question)$/u,
  });
  await expect(advance).toBeVisible();
  await advance.click();

  // Successor titles may legitimately repeat. The server-issued attempt ID is
  // the stable identity proving that the visible workflow actually advanced.
  await expect
    .poll(
      async () => {
        if ((await runSurface.getAttribute("data-attempt-id")) !== predecessor) return true;
        const retry = page.getByRole("button", { name: "Retry next question", exact: true });
        if ((await retry.isVisible()) && (await retry.isEnabled())) await retry.click();
        return false;
      },
      { timeout: AUTOMATED_GRADING_DEADLINE_MS, intervals: [AUTOMATED_GRADING_POLL_MS] },
    )
    .toBe(true);
}
