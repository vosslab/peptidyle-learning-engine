// Selector contract: assignment_editor_page.tsx exposes Question ID, Replace, Check Question ID, and Reload assignment.
import { expect, test, type Route } from "@playwright/test";
import { publishedProblemFixture } from "../../generated/fixtures/published_problem";

const course = publishedProblemFixture.course;
const original = publishedProblemFixture.assignment;
const assignmentPath = `/instructor/courses/C-${course.publicId}/assignments/A-${original.publicId}/edit`;
const replacementId = "7K4-M9QP";
const replacement = {
  ...publishedProblemFixture.catalogProblem,
  questionId: replacementId,
  metadata: {
    ...publishedProblemFixture.catalogProblem.metadata,
    title: "Replacement peptide geometry",
  },
};

function response(
  route: Route,
  value: unknown,
  status = 200,
  headers: Record<string, string> = {},
): Promise<void> {
  return route.fulfill({
    status,
    contentType: "application/json",
    headers,
    body: JSON.stringify(value),
  });
}

test("instructor replaces an assigned Question ID with visible issued-work consequences and stale recovery", async ({
  page,
}) => {
  let assignment = structuredClone(original);
  let revision = 1;
  await page.addInitScript(() =>
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", { get: () => false }),
  );
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const headers = { "cache-control": "no-store", etag: `"${revision}"` };
    if (path === "/api/auth/session")
      return await response(route, {
        authenticated: true,
        tenant: course.tenant,
        user: {
          id: "0198e000-0000-7000-8000-000000000014",
          displayName: "Professor",
          roles: ["instructor"],
        },
      });
    if (path === "/api/auth/account/presentation")
      return await response(route, { contrast: "standard" });
    if (path === `/api/navigation/C-${course.publicId}`)
      return await response(route, { kind: "course", courseId: course.id });
    if (path === `/api/navigation/A-${original.publicId}`)
      return await response(route, {
        kind: "assignment",
        courseId: course.id,
        assignmentId: original.id,
      });
    if (path === `/api/courses/${course.id}`)
      return await response(route, { ...course, role: "instructor" });
    if (path === `/api/courses/${course.id}/appearance`)
      return await response(route, { theme: "grass", revision: "1", banner: null }, 200, {
        "cache-control": "no-store",
        etag: '"1"',
      });
    if (path === `/api/assignments/${original.id}`)
      return await response(
        route,
        { ...assignment, assignmentTiming: { timeLimitSeconds: null } },
        200,
        headers,
      );
    if (path === `/api/problems/by-id/${replacementId}`) return await response(route, replacement);
    if (path === `/api/problems/by-id/${replacementId}/detail`)
      return await response(route, { summary: replacement, prompt: [], statistics: "unavailable" });
    if (path === `/api/problems/search`)
      return await response(route, {
        items: [replacement],
        nextCursor: null,
        facets: {
          taxonomy: [],
          capabilities: [],
          licenses: [],
          statistics: { available: 0, unavailable: 1 },
        },
      });
    if (
      path === `/api/courses/${course.id}/assignments/${original.id}` &&
      request.method() === "PUT"
    ) {
      const body = request.postDataJSON() as {
        title?: string;
        items?: ReadonlyArray<{ id: string }>;
      };
      expect(body.items?.every((item) => !item.id.startsWith("new-"))).toBeTruthy();
      assignment = { ...assignment, title: body.title ?? assignment.title };
      revision += 1;
      return await response(
        route,
        { ...assignment, assignmentTiming: { timeLimitSeconds: null } },
        200,
        { "cache-control": "no-store", etag: `"${revision}"` },
      );
    }
    if (
      path === `/api/courses/${course.id}/assignments/${original.id}/items` &&
      request.method() === "POST"
    ) {
      expect(request.postData()).toBe(JSON.stringify({ questionId: replacementId, position: 1 }));
      assignment = {
        ...assignment,
        items: [
          ...assignment.items,
          {
            id: "0198e000-0000-7000-8000-000000000502",
            questionId: replacementId,
            title: replacement.metadata.title,
            backend: replacement.backend,
            capabilities: replacement.capabilities,
            position: 1,
            pointsPossible: "1",
            deliveryState: "active",
            scoringMode: "normal",
          },
        ],
      };
      revision = 3;
      return await response(
        route,
        { ...assignment, assignmentTiming: { timeLimitSeconds: null } },
        200,
        { "cache-control": "no-store", etag: '"3"' },
      );
    }
    if (path.endsWith("/question") && request.method() === "PUT") {
      if (request.postData() !== JSON.stringify({ questionId: replacementId }))
        return await response(route, { error: "wrong QID" }, 422);
      if (request.headers()["if-match"] === '"3"') {
        revision = 4;
        assignment = {
          ...assignment,
          items: assignment.items.map((candidate, index) =>
            index === 0
              ? {
                  ...candidate,
                  id: "0198e000-0000-7000-8000-000000000501",
                  questionId: replacementId,
                  title: replacement.metadata.title,
                  backend: replacement.backend,
                  capabilities: replacement.capabilities,
                }
              : candidate,
          ),
        };
        return await response(
          route,
          { ...assignment, assignmentTiming: { timeLimitSeconds: null } },
          200,
          { "cache-control": "no-store", etag: '"4"' },
        );
      }
      return await response(route, { error: "assignment changed; reload it" }, 409, {
        "cache-control": "no-store",
      });
    }
    return await response(route, { error: `unexpected ${request.method()} ${path}` }, 500);
  });
  await page.goto("/");
  await page.evaluate((path) => {
    history.pushState({}, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, assignmentPath);
  await expect(page.getByRole("heading", { name: "Assignment editor" })).toBeVisible();
  await expect(
    page.getByText("Reuse questions from an existing assignment", { exact: true }),
  ).toHaveCount(0);
  await page.getByLabel("Assignment title").fill("Saved title without changing Question ID");
  await page.getByRole("button", { name: "Save title, order, and settings" }).click();
  await expect(page.getByText("Assignment title, order, and settings saved.")).toBeVisible();
  await page.getByText("Add by Question ID", { exact: true }).click();
  await page
    .getByRole("textbox", { name: "Direct import Question ID", exact: true })
    .fill(replacementId);
  await page.getByRole("button", { name: "Add Question ID", exact: true }).click();
  await expect(
    page.getByText("Question added. Add and remove are available before student work begins."),
  ).toBeVisible();
  await page.getByRole("button", { name: "Replace", exact: true }).first().click();
  await expect(
    page.getByText(
      "Future runs use the replacement. Already issued work stays with its original question.",
    ),
  ).toBeVisible();
  const questionId = page.getByRole("textbox", { name: "Replacement Question ID", exact: true });
  await questionId.focus();
  await page.keyboard.type(replacementId);
  await page.getByRole("button", { name: "Check Question ID" }).click();
  await expect(page.locator(".success-state .copyable-question-id code")).toHaveText(replacementId);
  await expect(page.locator(".success-state")).toContainText(replacement.metadata.title);
  await expect(page.locator(".success-state")).toContainText("PLE native");
  await page.getByRole("button", { name: "Replace with selected question" }).click();
  await expect(
    page.getByText(
      `Replacement saved. Future runs use the replacement; issued work stays with its original question.`,
    ),
  ).toBeVisible();
  await expect(page.getByText(replacementId, { exact: true }).first()).toBeVisible();
  await page.getByRole("button", { name: "Replace", exact: true }).first().click();
  await questionId.fill(replacementId);
  await page.getByRole("button", { name: "Check Question ID" }).click();
  await page.getByRole("button", { name: "Replace with selected question" }).click();
  await expect(page.getByRole("button", { name: "Reload assignment" })).toBeVisible();
  await expect(page.getByRole("alert")).toContainText("A newer assignment revision is available.");
  await expect(questionId).toHaveValue(replacementId);
  await expect(page.locator("body")).not.toContainText(/0198e000-0000-7000-8000/u);
  expect(
    await page
      .locator(".assignment-editor-grid")
      .evaluate((element) => element.scrollWidth <= element.clientWidth),
  ).toBeTruthy();
  for (const viewport of [
    { width: 800, height: 1280 },
    { width: 320, height: 800 },
  ]) {
    await page.setViewportSize(viewport);
    expect(
      await page.locator("body").evaluate((element) => element.scrollWidth <= window.innerWidth),
    ).toBeTruthy();
  }
});
