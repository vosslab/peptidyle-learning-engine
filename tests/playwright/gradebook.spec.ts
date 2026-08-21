// MOD-UI-GRADEBOOK browser proof for accessible compact and expanded states.

import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";

const gradebookPath = "/instructor/courses/C-1/gradebook";

function json(route: Route, value: unknown, headers: Record<string, string> = {}): Promise<void> {
  return route.fulfill({
    status: 200,
    contentType: "application/json",
    headers,
    body: JSON.stringify(value),
  });
}

async function openGradebook(
  page: Page,
  scoringStatus: "current" | "recalculating" | "failed" = "current",
): Promise<void> {
  const course = { ...publishedProblemFixture.course, role: "instructor" };
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") {
      return await json(route, {
        authenticated: true,
        tenant: course.tenant,
        user: {
          id: publishedProblemFixture.enrollment.user,
          displayName: "Course instructor",
          roles: ["instructor"],
        },
      });
    }
    if (path === "/api/auth/account/presentation") {
      return await json(route, { contrast: "standard" });
    }
    if (path === "/api/courses") {
      return await json(route, { items: [course], nextCursor: null });
    }
    if (path === "/api/navigation/C-1") {
      return await json(route, { kind: "course", courseId: course.id });
    }
    if (path === `/api/courses/${course.id}`) return await json(route, course);
    if (path === `/api/courses/${course.id}/appearance`) {
      return await json(
        route,
        { theme: "grass", revision: "1", banner: null },
        { "cache-control": "no-store", etag: '"1"' },
      );
    }
    if (path === `/api/courses/${course.id}/gradebook`) {
      return await json(route, {
        items: publishedProblemFixture.gradebook.map((row) => ({ ...row, scoringStatus })),
        nextCursor: null,
      });
    }
    if (path.startsWith("/api/enrollments/") && path.endsWith("/runs")) {
      return await json(route, { items: publishedProblemFixture.runs, nextCursor: null });
    }
    return await route.fulfill({ status: 404, body: "not found" });
  });
  await page.goto("/");
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, gradebookPath);
}

test("gradebook presents compact progress and loads history only when requested", async ({
  page,
}) => {
  await openGradebook(page);

  await expect(page.locator('[data-route-surface="gradebook"]')).toBeVisible();
  await expect(page.getByRole("heading", { name: "Gradebook" })).toBeVisible();
  await expect(page.getByRole("table")).toBeVisible();
  await expect(page.getByRole("cell", { name: "100%" }).first()).toBeVisible();
  await expect(page.getByText("Peptide bond mastery", { exact: true })).toBeVisible();
  await expect(page.getByRole("region", { name: /run history/i })).toHaveCount(0);

  const historyButton = page.getByRole("button", { name: "View run history" });
  await historyButton.focus();
  await page.keyboard.press("Enter");
  await expect(historyButton).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("region", { name: /run history for learner/i })).toBeVisible();
  await expect(page.getByText("Run 1", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("status").filter({ hasText: /Run history updated|Loading run history/u }),
  ).toBeVisible();
});

test("gradebook reflows into labeled records on a narrow viewport", async ({ page }) => {
  await page.setViewportSize({ width: 420, height: 900 });
  await openGradebook(page);

  await expect(page.locator(".gradebook-row")).toBeVisible();
  await expect(page.locator(".gradebook-table th[data-label='Assignment']")).toContainText(
    "Peptide bond mastery",
  );
  await expect(page.getByRole("button", { name: "View run history" })).toBeVisible();
});

for (const scoringStatus of ["recalculating", "failed"] as const) {
  test(`gradebook keeps run-history scores neutral while ${scoringStatus}`, async ({ page }) => {
    await openGradebook(page, scoringStatus);
    await expect(
      page
        .getByRole("cell", {
          name: scoringStatus === "recalculating" ? "Recalculating" : "Unavailable",
        })
        .first(),
    ).toBeVisible();
    await page.getByRole("button", { name: "View run history" }).press("Enter");
    const history = page.getByRole("region", { name: /run history for learner/i });
    await expect(history).toContainText(
      scoringStatus === "recalculating"
        ? "Completed, recalculating"
        : "Completed, score unavailable",
    );
    await expect(history).not.toContainText("100%");
  });
}
