// Selector contract: assignment_editor_page.tsx exposes Question ID, Replace, Check Question ID, and Reload assignment.
import { expect, test, type Route } from "@playwright/test";
import { publishedProblemFixture } from "../../generated/fixtures/published_problem";

const course = publishedProblemFixture.course;
const original = publishedProblemFixture.assignment;
const assignmentPath = `/instructor/courses/${course.reference}/assignments/${original.reference}/edit`;
const replacementId = "7K4-M9QP";
const replacement = {
  ...publishedProblemFixture.catalogProblem,
  questionId: replacementId,
  metadata: {
    ...publishedProblemFixture.catalogProblem.metadata,
    title: "Replacement peptide geometry",
  },
};
const disclosurePolicy = {
  score: "afterSubmit",
  perItemCorrectness: "afterSubmit",
  feedbackText: "afterSubmit",
  solution: "afterSubmit",
  classStatistics: "never",
} as const;
const teachingSettings = {
  timeZone: "America/Chicago",
  lifecycle: "published",
  instructions: "Read the structures before starting.",
  availableAt: null,
  dueAt: null,
  closesAt: "2026-12-01T23:59:00.000",
  timeLimitSeconds: null,
  attemptLimit: null,
  lateSubmission: "accept",
  deadlineBehavior: "autoSubmit",
} as const;
const currentState = {
  state: "closed",
  closedAt: "2026-12-01T23:59:00.000",
} as const;

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
  let assignment = { ...structuredClone(original), disclosurePolicy };
  let revision = 1;
  let teachingConflict = false;
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
    if (path === `/api/navigation/${course.reference}`)
      return await response(route, { kind: "course", courseId: course.id });
    if (path === `/api/navigation/${original.reference}`)
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
      return await response(route, { ...assignment, teachingSettings, currentState }, 200, headers);
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
        disclosurePolicy?: typeof disclosurePolicy;
      };
      expect(body.items?.every((item) => !item.id.startsWith("new-"))).toBeTruthy();
      expect(body.disclosurePolicy).toEqual({
        score: "afterDue",
        perItemCorrectness: "duringAttempt",
        feedbackText: "afterClose",
        solution: "never",
        classStatistics: "afterSubmit",
      });
      assignment = {
        ...assignment,
        title: body.title ?? assignment.title,
        disclosurePolicy: body.disclosurePolicy ?? assignment.disclosurePolicy,
      };
      revision += 1;
      return await response(route, { ...assignment, teachingSettings, currentState }, 200, {
        "cache-control": "no-store",
        etag: `"${revision}"`,
      });
    }
    if (
      path === `/api/courses/${course.id}/assignments/${original.id}/teaching-settings` &&
      request.method() === "PUT"
    ) {
      if (request.headers()["if-match"] !== '"2"')
        return await response(route, { error: "assignment changed; reload it" }, 412, {
          "cache-control": "no-store",
        });
      const body = request.postDataJSON() as { dueAt?: string | null };
      if (body.dueAt === "2026-11-01T01:30:00.000")
        return await response(
          route,
          {
            error: "assignmentTeachingSettingsInvalid",
            field: "dueAt",
            reason: "ambiguousLocalTime",
            message: "Choose a local time outside the daylight-saving repeat hour.",
          },
          422,
          { "cache-control": "no-store" },
        );
      if (!teachingConflict) {
        teachingConflict = true;
        return await response(route, { error: "assignment changed; reload it" }, 412, {
          "cache-control": "no-store",
        });
      }
      revision += 1;
      return await response(route, { ...assignment, teachingSettings, currentState }, 200, {
        "cache-control": "no-store",
        etag: `"${revision}"`,
      });
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
      return await response(route, { ...assignment, teachingSettings, currentState }, 200, {
        "cache-control": "no-store",
        etag: '"3"',
      });
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
        return await response(route, { ...assignment, teachingSettings, currentState }, 200, {
          "cache-control": "no-store",
          etag: '"4"',
        });
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
  await expect(page.getByTestId("assignment-current-state")).toHaveText(
    "Published, closed since 2026-12-01 23:59 America/Chicago.",
  );
  const studentVisibility = page.getByRole("group", { name: "What students can see" });
  await expect(studentVisibility).toBeVisible();
  const fields = [
    "Score",
    "Per-item correctness",
    "Feedback text",
    "Correct answer or solution",
    "Class statistics",
  ];
  for (const label of fields) {
    const select = studentVisibility.getByRole("combobox", { name: label, exact: true });
    await expect(select).toHaveValue(label === "Class statistics" ? "never" : "afterSubmit");
    await expect(select.locator("option")).toHaveText([
      "While they work",
      "After they submit",
      "After the due time",
      "After the close time",
      "Never",
    ]);
  }
  await studentVisibility.getByRole("combobox", { name: "Score", exact: true }).focus();
  await expect(
    studentVisibility.getByRole("combobox", { name: "Score", exact: true }),
  ).toBeFocused();
  await studentVisibility
    .getByRole("combobox", { name: "Score", exact: true })
    .selectOption("afterDue");
  await expect(studentVisibility.getByRole("combobox", { name: "Score", exact: true })).toHaveValue(
    "afterDue",
  );
  await studentVisibility
    .getByRole("combobox", { name: "Per-item correctness", exact: true })
    .selectOption("duringAttempt");
  await studentVisibility
    .getByRole("combobox", { name: "Feedback text", exact: true })
    .selectOption("afterClose");
  await studentVisibility
    .getByRole("combobox", { name: "Correct answer or solution", exact: true })
    .selectOption("never");
  await studentVisibility
    .getByRole("combobox", { name: "Class statistics", exact: true })
    .selectOption("afterSubmit");
  await expect(
    page.getByText("Reuse questions from an existing assignment", { exact: true }),
  ).toHaveCount(0);
  await page.getByLabel("Assignment title").fill("Saved title without changing Question ID");
  await page.getByRole("button", { name: "Save title, order, and settings" }).click();
  await expect(page.getByText("Assignment title, order, and settings saved.")).toBeVisible();
  await expect(studentVisibility.getByRole("combobox", { name: "Score", exact: true })).toHaveValue(
    "afterDue",
  );
  await expect(
    studentVisibility.getByRole("combobox", { name: "Per-item correctness", exact: true }),
  ).toHaveValue("duringAttempt");
  await expect(
    studentVisibility.getByRole("combobox", { name: "Feedback text", exact: true }),
  ).toHaveValue("afterClose");
  await expect(
    studentVisibility.getByRole("combobox", { name: "Class statistics", exact: true }),
  ).toHaveValue("afterSubmit");
  await expect(
    studentVisibility.getByRole("combobox", { name: "Correct answer or solution", exact: true }),
  ).toHaveValue("never");
  const teaching = page.getByRole("region", { name: "Teaching operations" });
  await expect(teaching).toBeVisible();
  await page.getByLabel("Assignment title").fill("Local content draft survives teaching save");
  await teaching.getByLabel("Due").fill("2026-11-01T01:30");
  await teaching.getByRole("button", { name: "Save teaching operations" }).focus();
  await page.keyboard.press("Enter");
  await expect(teaching.getByRole("alert")).toContainText("daylight-saving repeat hour");
  await expect(teaching.getByLabel("Due")).toBeFocused();
  await expect(teaching.getByLabel("Due")).toHaveValue("2026-11-01T01:30");
  await teaching.getByLabel("Due").fill("2026-11-02T10:00");
  await teaching.getByRole("button", { name: "Save teaching operations" }).click();
  await expect(
    teaching.getByRole("button", { name: "Adopt latest teaching operations" }),
  ).toBeVisible();
  await expect(teaching.getByLabel("Due")).toHaveValue("2026-11-02T10:00");
  await teaching.getByRole("button", { name: "Adopt latest teaching operations" }).click();
  await expect(page.getByLabel("Assignment title")).toHaveValue(
    "Local content draft survives teaching save",
  );
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
  await expect(questionId).toBeFocused();
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
  await expect(
    page.getByRole("alert").filter({ hasText: "A newer assignment revision" }),
  ).toContainText("A newer assignment revision is available.");
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
