// Assignment editor selectors: src/pages/assignment_editor_page.tsx accessible labels and buttons.

import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import { tabTo } from "./simulator/keyboard_walkthrough";

const courseId = publishedProblemFixture.course.id;
const assignmentId = publishedProblemFixture.assignment.id;
const assignmentProblems = publishedProblemFixture.assignment.items
  .filter((item) => item.deliveryState === "active")
  .sort((left, right) => left.position - right.position)
  .map((item) => item.reference);
const editPath = `/instructor/courses/${courseId}/assignments/${assignmentId}/edit`;
const createPath = `/instructor/courses/${courseId}/assignments/new`;
const appearance = { theme: "grass", revision: "1", banner: null } as const;
const untimedAssignmentEditor = {
  ...publishedProblemFixture.assignment,
  assignmentTiming: { timeLimitSeconds: null },
} as const;
const secondCatalogProblem = {
  ...publishedProblemFixture.catalogProblem,
  problem: "0198e000-0000-7000-8000-000000000097",
  publicId: publishedProblemFixture.catalogProblem.publicId + 1,
  version: "0198e000-0000-7000-8000-000000000096",
  metadata: {
    ...publishedProblemFixture.catalogProblem.metadata,
    title: "Second immutable peptide version",
  },
};

function json(
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

function session(roles: ReadonlyArray<string>): unknown {
  return {
    authenticated: true,
    tenant: publishedProblemFixture.course.tenant,
    user: { id: "0198e000-0000-7000-8000-000000000014", displayName: "Editor", roles },
  };
}

function appearanceJson(route: Route): Promise<void> {
  return json(route, appearance, 200, { "cache-control": "no-store", etag: '"1"' });
}

async function useProductionTransport(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
}

async function openEditor(page: Page, path = editPath): Promise<void> {
  // `tools/mock_preview_server.mjs` is a static artifact server without an SPA
  // fallback; it returns 404 for this direct deep link. The managed deployment
  // must provide that fallback. This proves the built router after its normal
  // index entry point, without claiming that the static test server is one.
  await useProductionTransport(page);
  await page.goto("/");
  await page.evaluate((path: string) => {
    history.pushState({}, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, path);
}

async function routeCourseAssignments(
  page: Page,
  roles: ReadonlyArray<string>,
  courseRole: "instructor" | "student",
  requests: Array<{ readonly method: string; readonly path: string }>,
): Promise<void> {
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    requests.push({ method: request.method(), path });
    if (path === "/api/auth/session") return await json(route, session(roles));
    if (path === "/api/courses") {
      return await json(route, {
        items: [{ ...publishedProblemFixture.course, role: courseRole }],
        nextCursor: null,
      });
    }
    if (path === `/api/courses/${courseId}`) {
      return await json(route, { ...publishedProblemFixture.course, role: courseRole });
    }
    if (path === `/api/courses/${courseId}/appearance`) return await appearanceJson(route);
    if (path === `/api/courses/${courseId}/assignments`) {
      return await json(route, { items: [], nextCursor: null });
    }
    return await json(route, { error: "unexpected course assignment request" }, 500);
  });
}

async function openCourseAssignments(page: Page): Promise<void> {
  await useProductionTransport(page);
  await page.goto("/");
  const openCourse = page.getByRole("link", { name: "Open course", exact: true });
  await expect(openCourse).toBeVisible();
  await tabTo(page, openCourse);
  await expect(openCourse).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Assignments", exact: true })).toBeVisible();
}

async function routeStudentCourse(page: Page, requests: string[]): Promise<void> {
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    requests.push(path);
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === `/api/courses/${courseId}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "student" });
    }
    if (path === `/api/courses/${courseId}/appearance`) return await appearanceJson(route);
    return await json(route, { error: "unexpected assignment editor request" }, 500);
  });
}

test("course instructor reaches New assignment from the visible course page by keyboard", async ({
  page,
}) => {
  const requests: Array<{ readonly method: string; readonly path: string }> = [];
  await routeCourseAssignments(page, ["instructor"], "instructor", requests);
  await openCourseAssignments(page);

  const newAssignment = page.getByRole("link", { name: "New assignment", exact: true });
  await expect(newAssignment).toBeVisible();
  await expect(newAssignment).toHaveAttribute("href", createPath);
  await tabTo(page, newAssignment);
  await expect(newAssignment).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(new RegExp(`${courseId}/assignments/new$`, "u"));
  await expect(page.getByRole("heading", { name: "Create assignment" })).toBeVisible();
  expect(requests.some((request) => request.method === "POST")).toBe(false);
});

test("course learner cannot see the New assignment entry", async ({ page }) => {
  const requests: Array<{ readonly method: string; readonly path: string }> = [];
  await routeCourseAssignments(page, ["student"], "student", requests);
  await openCourseAssignments(page);

  await expect(page.getByRole("link", { name: "New assignment", exact: true })).toHaveCount(0);
  expect(requests.some((request) => request.method === "POST")).toBe(false);
});

test("student and global instructor who is a course learner see no assignment editor transport", async ({
  page,
}) => {
  const requests: string[] = [];
  await routeStudentCourse(page, requests);
  await openEditor(page);
  await expect(
    page.getByRole("heading", { name: "Assignment editing is not available for this account" }),
  ).toBeVisible();
  expect(requests).toContain("/api/auth/session");
  expect(requests).toContain(`/api/courses/${courseId}`);
  expect(
    requests.some(
      (path) =>
        path.includes("assignments") || path.includes("problems") || path.includes("workspaces"),
    ),
  ).toBe(false);
});

test("direct student route stops before the course, assignment, and catalog clients", async ({
  page,
}) => {
  const requests: string[] = [];
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    requests.push(path);
    if (path === "/api/auth/session") return await json(route, session(["student"]));
    return await json(route, { error: "student editor transport must not run" }, 500);
  });
  await openEditor(page);
  await expect(
    page.getByRole("heading", { name: "Assignment editing is not available for this account" }),
  ).toBeVisible();
  expect(requests).toEqual(["/api/auth/session"]);
});

test("hostile cross-tenant assignment detail is rejected before editor state adopts its revision", async ({
  page,
}) => {
  const requests: string[] = [];
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    requests.push(path);
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === `/api/courses/${courseId}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${courseId}/appearance`) return await appearanceJson(route);
    if (path === `/api/assignments/${assignmentId}`) {
      return await json(
        route,
        { ...untimedAssignmentEditor, tenant: "0198e000-0000-7000-8000-000000000088" },
        200,
        { etag: '"7"' },
      );
    }
    return await json(route, { error: "unexpected request" }, 500);
  });
  await openEditor(page);
  await expect(
    page.getByRole("heading", { name: "This assignment could not be opened" }),
  ).toBeVisible();
  expect(requests).toContain("/api/auth/session");
  expect(requests).toContain(`/api/courses/${courseId}`);
  expect(requests).toContain(`/api/assignments/${assignmentId}`);
  expect(
    requests.some(
      (path) =>
        path.includes("problems") ||
        (path.includes("assignments") && path !== `/api/assignments/${assignmentId}`),
    ),
  ).toBe(false);
});

test("direct question-ID lookup keeps the draft and pasted ID after access or service failures", async ({
  page,
}) => {
  await page.route("**/api/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === `/api/courses/${courseId}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${courseId}/appearance`) return await appearanceJson(route);
    if (path === `/api/assignments/${assignmentId}`) {
      return await json(route, untimedAssignmentEditor, 200, { etag: '"7"' });
    }
    if (
      path ===
      `/api/problems/${publishedProblemFixture.catalogProblem.problem}/versions/${publishedProblemFixture.catalogProblem.version}/detail`
    ) {
      return await json(route, {
        summary: publishedProblemFixture.catalogProblem,
        prompt: [],
        statistics: "unavailable",
      });
    }
    if (path === "/api/problems/by-id/P-2-v1") {
      return await json(route, { error: "not shared with this instructor" }, 403);
    }
    if (path === "/api/problems/by-id/P-3-v1") {
      return await json(route, { error: "temporary catalog outage" }, 503);
    }
    return await json(route, { error: "unexpected request" }, 500);
  });

  await openEditor(page);
  const selected = page.locator(".assignment-editor-list");
  const questionIds = page.getByLabel("Question IDs");
  const add = page.getByRole("button", { name: "Add questions by ID" });
  await expect(selected).toContainText("P-1-v1");

  await questionIds.fill("P-2-v1");
  await add.click();
  await expect(page.getByRole("alert")).toHaveText(
    "You do not have access to P-2-v1. Ask its owner to publish or share it.",
  );
  await expect(questionIds).toHaveValue("P-2-v1");
  await expect(selected).toHaveText(/P-1-v1/u);

  await questionIds.fill("P-3-v1");
  await add.click();
  await expect(page.getByRole("alert")).toHaveText(
    "Could not look up P-3-v1. Your pasted IDs and assignment are unchanged. Try again.",
  );
  await expect(questionIds).toHaveValue("P-3-v1");
  await expect(selected).toHaveText(/P-1-v1/u);
});

test("authorized editor saves exact immutable refs with CAS, retains all violations, and recovers from conflict", async ({
  page,
}) => {
  const requests: Array<{
    readonly path: string;
    readonly method: string;
    readonly body: unknown;
    readonly revision: string | null;
  }> = [];
  let assignmentReads = 0;
  let saves = 0;
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const body = request.postDataJSON() as unknown;
    requests.push({
      path,
      method: request.method(),
      body,
      revision: request.headers()["if-match"] ?? null,
    });
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === `/api/courses/${courseId}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${courseId}/appearance`) return await appearanceJson(route);
    if (path === `/api/assignments/${assignmentId}`) {
      assignmentReads += 1;
      return await json(route, untimedAssignmentEditor, 200, {
        etag: assignmentReads > 1 ? '"8"' : '"7"',
      });
    }
    if (
      path ===
      `/api/problems/${publishedProblemFixture.catalogProblem.problem}/versions/${publishedProblemFixture.catalogProblem.version}/detail`
    ) {
      return await json(route, {
        summary: publishedProblemFixture.catalogProblem,
        prompt: [],
        statistics: "unavailable",
      });
    }
    if (path === "/api/problems/search") {
      return await json(route, {
        items: [publishedProblemFixture.catalogProblem, secondCatalogProblem],
        nextCursor: null,
        facets: {
          taxonomy: [],
          capabilities: [],
          licenses: [],
          statistics: { available: 0, unavailable: 1 },
        },
      });
    }
    if (path === "/api/problems/by-id/P-2-v1") return await json(route, secondCatalogProblem);
    if (path === "/api/problems/by-id/P-99-v1") {
      return await json(route, { error: "problem reference not found" }, 404);
    }
    if (
      path === `/api/courses/${courseId}/assignments/${assignmentId}` &&
      request.method() === "PUT"
    ) {
      saves += 1;
      if (saves === 1) {
        const reference = assignmentProblems[0];
        return await json(
          route,
          {
            error: "assignment configuration is not supported",
            violations: [
              {
                title: "Peptide bond resonance and planarity",
                reference,
                capability: "serverGrading",
              },
              {
                title: "Peptide bond resonance and planarity",
                reference,
                capability: "perQuestionTiming",
              },
            ],
          },
          422,
        );
      }
      if (saves === 2) return await json(route, { error: "assignment changed; reload it" }, 409);
      return await json(
        route,
        {
          ...untimedAssignmentEditor,
          title: (body as { title: string }).title,
          policies: (body as { policies: unknown }).policies,
        },
        200,
        { etag: '"9"' },
      );
    }
    return await json(route, { error: "unexpected request" }, 500);
  });

  await page.setViewportSize({ width: 420, height: 820 });
  await openEditor(page);
  const title = page.getByLabel("Assignment title");
  await expect(title).toBeVisible();
  await expect(title).toBeFocused();
  await expect(page.getByRole("radio", { name: "Untimed", exact: true })).toBeChecked();
  await expect(page.getByLabel("Minutes per practice run")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Save assignment" })).toBeVisible();
  const selectedProblems = page.locator(".assignment-editor-list");
  await expect(selectedProblems).toContainText(
    `P-${publishedProblemFixture.catalogProblem.publicId}-v${publishedProblemFixture.catalogProblem.versionNumber}`,
  );
  await expect(selectedProblems).not.toContainText(publishedProblemFixture.catalogProblem.problem);
  await expect(selectedProblems).not.toContainText(publishedProblemFixture.catalogProblem.version);

  const questionIds = page.getByLabel("Question IDs");
  await questionIds.fill("P-1-v1");
  await page.getByRole("button", { name: "Add questions by ID" }).click();
  await expect(page.getByRole("alert")).toContainText("P-1-v1 is already in this assignment");
  await expect(questionIds).toHaveValue("P-1-v1");
  await questionIds.fill("P-2");
  await page.getByRole("button", { name: "Add questions by ID" }).click();
  await expect(page.getByRole("alert")).toContainText("P-2 is not an exact question ID");
  await expect(questionIds).toHaveValue("P-2");
  await questionIds.fill("P-2-v1, P-99-v1");
  await page.getByRole("button", { name: "Add questions by ID" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "P-99-v1 is not an available published question",
  );
  await expect(questionIds).toHaveValue("P-2-v1, P-99-v1");
  await expect(page.getByText("Second immutable peptide version")).toHaveCount(0);

  // The instructor obtains the identifier from a visible catalog result, then uses the
  // same clipboard and keyboard path available in the real editor. No UUID is entered.
  const search = page.getByLabel("Search published problems");
  await search.fill("Second immutable peptide version");
  await page.getByRole("button", { name: "Search catalog" }).click();
  const secondCatalogRow = page.locator(".assignment-editor-catalog-results article", {
    has: page.getByRole("heading", { name: "Second immutable peptide version", exact: true }),
  });
  const copiedId = await secondCatalogRow.locator("code").innerText();
  await expect(secondCatalogRow.locator("code")).toHaveText("P-2-v1");
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"], {
    origin: new URL(page.url()).origin,
  });
  const copyId = secondCatalogRow.getByRole("button", {
    name: `Copy question ID ${copiedId}`,
  });
  await copyId.focus();
  await expect(copyId).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(secondCatalogRow.getByRole("status")).toHaveText(`Copied ${copiedId}.`);
  await questionIds.focus();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.press("ControlOrMeta+V");
  await expect(questionIds).toHaveValue(copiedId);
  const addSecond = page.getByRole("button", { name: "Add questions by ID" });
  await addSecond.focus();
  await expect(addSecond).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByText("Second immutable peptide version").first()).toBeVisible();
  await expect(page.locator(".assignment-editor-import-success")).toContainText(
    "Added P-2-v1 to the unsaved selection",
  );
  await expect(page.locator(".assignment-editor-actions")).toContainText(
    "Unsaved assignment changes.",
  );
  await expect(questionIds).toHaveValue("");
  expect(requests.filter((request) => request.method === "PUT")).toEqual([]);
  await title.fill("Edited peptide practice");
  await page.getByRole("button", { name: "Save assignment" }).click();
  await expect(page.getByRole("heading", { name: "Fix these assignment settings" })).toBeFocused();
  await expect(page.getByText("cannot provide server grading.")).toBeVisible();
  await expect(page.getByText("cannot provide per-question timing.")).toBeVisible();
  await expect(title).toHaveValue("Edited peptide practice");

  await page.getByRole("button", { name: "Save assignment" }).click();
  await expect(page.getByRole("button", { name: "Reload newest assignment" })).toBeVisible();
  await expect(title).toHaveValue("Edited peptide practice");
  await page.getByRole("button", { name: "Reload newest assignment" }).click();
  await expect(title).toBeFocused();
  await title.fill("Saved after reload");
  await page.getByRole("button", { name: "Save assignment" }).click();
  await expect(page.locator(".assignment-editor-actions span")).toHaveText(
    "Assignment saved. This assignment is untimed.",
  );

  const puts = requests.filter((request) => request.method === "PUT");
  expect(puts).toHaveLength(3);
  expect(puts[0]?.revision).toBe('"7"');
  expect(puts[0]?.body).toEqual({
    title: "Edited peptide practice",
    problems: [
      ...assignmentProblems,
      { problem: secondCatalogProblem.problem, version: secondCatalogProblem.version },
    ],
    policies: publishedProblemFixture.assignment.policies,
    assignmentTiming: { timeLimitSeconds: null },
  });
  expect(puts[2]?.revision).toBe('"8"');
  expect(puts[2]?.body).toEqual({
    title: "Saved after reload",
    problems: assignmentProblems,
    policies: publishedProblemFixture.assignment.policies,
    assignmentTiming: { timeLimitSeconds: null },
  });
  expect(JSON.stringify(puts[2]?.body)).not.toMatch(
    /workspace|source|prompt|response|grading|answerKey|capabilit/i,
  );
});

test("instructor creates a Mastery assignment from public immutable catalog tuples by keyboard", async ({
  page,
}) => {
  const requests: Array<{
    readonly method: string;
    readonly path: string;
    readonly body: unknown;
  }> = [];
  const createdId = "0198e000-0000-7000-8000-000000000060";
  let created = false;
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const body = request.postDataJSON() as unknown;
    requests.push({ method: request.method(), path, body });
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === `/api/courses/${courseId}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${courseId}/appearance`) return await appearanceJson(route);
    if (path === "/api/problems/search") {
      return await json(route, {
        items: [publishedProblemFixture.catalogProblem],
        nextCursor: null,
        facets: {
          taxonomy: [],
          capabilities: [],
          licenses: [],
          statistics: { available: 0, unavailable: 0 },
        },
      });
    }
    if (path === "/api/problems/by-id/P-1-v1") {
      return await json(route, publishedProblemFixture.catalogProblem);
    }
    if (path === `/api/courses/${courseId}/assignments` && request.method() === "POST") {
      created = true;
      return await json(
        route,
        {
          ...untimedAssignmentEditor,
          id: createdId,
          title: (body as { readonly title: string }).title,
          policies: (body as { readonly policies: unknown }).policies,
          assignmentTiming: (body as { readonly assignmentTiming: unknown }).assignmentTiming,
        },
        201,
        { etag: '"1"' },
      );
    }
    if (path === `/api/assignments/${createdId}` && created) {
      return await json(route, { ...untimedAssignmentEditor, id: createdId });
    }
    return await json(route, { error: "unexpected request" }, 500);
  });

  await openEditor(page, createPath);
  const title = page.getByLabel("Assignment title");
  await expect(page.getByRole("heading", { name: "Create assignment" })).toBeVisible();
  await expect(title).toBeFocused();
  await expect(page.getByLabel("Completion requirement")).toHaveValue("allCorrect");
  await expect(page.getByLabel("Grade policy")).toHaveValue("highest");
  await expect(page.getByLabel("Continued practice")).toHaveValue("unlimited");
  await expect(page.getByLabel("Variation policy")).toHaveValue("newSeeds");
  await expect(page.getByRole("radio", { name: "Timed", exact: true })).toBeChecked();
  const runMinutes = page.getByLabel("Minutes per practice run");
  await expect(runMinutes).toHaveValue("15");
  await runMinutes.fill("0");
  await expect(page.getByRole("alert")).toHaveText(
    "Enter a positive number of minutes, such as 15.",
  );
  await expect(page.getByRole("button", { name: "Create assignment" })).toBeDisabled();
  await runMinutes.fill("1.5");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.getByRole("radio", { name: "Untimed", exact: true }).check();
  await expect(runMinutes).toHaveCount(0);
  await page.getByRole("radio", { name: "Timed", exact: true }).check();
  await expect(runMinutes).toHaveValue("1.5");
  await runMinutes.fill("15");

  await title.fill("Fall pilot peptide mastery");
  await page.getByLabel("Question IDs").fill("P-1-v1");
  const add = page.getByRole("button", { name: "Add questions by ID" });
  await add.focus();
  await page.keyboard.press("Enter");
  await expect(page.locator(".assignment-editor-import-success")).toHaveText(
    "Added P-1-v1 to the unsaved selection.",
  );
  await page.getByRole("button", { name: "Create assignment" }).focus();
  await page.keyboard.press("Enter");

  const open = page.getByRole("link", { name: "Open Fall pilot peptide mastery" });
  await expect(open).toBeVisible();
  await expect(open).toHaveAttribute("href", `/courses/${courseId}/assignments/${createdId}`);
  const posts = requests.filter((request) => request.method === "POST");
  expect(posts).toHaveLength(1);
  expect(posts[0]).toEqual({
    method: "POST",
    path: `/api/courses/${courseId}/assignments`,
    body: {
      title: "Fall pilot peptide mastery",
      problems: [assignmentProblems[0]],
      policies: {
        completion: { kind: "allCorrect" },
        grade: "highest",
        continuedPractice: { kind: "unlimited" },
        variation: "newSeeds",
      },
      assignmentTiming: { timeLimitSeconds: 900 },
    },
  });
  expect(JSON.stringify(posts[0]?.body)).not.toMatch(/source|answer|grading|prompt|response/i);
});

test("a loaded non-terminating minute value survives an unrelated save exactly", async ({
  page,
}) => {
  const storedSeconds = 2_147_483_647;
  const saves: unknown[] = [];
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === `/api/courses/${courseId}`) {
      return await json(route, { ...publishedProblemFixture.course, role: "instructor" });
    }
    if (path === `/api/courses/${courseId}/appearance`) return await appearanceJson(route);
    if (path === `/api/assignments/${assignmentId}` && request.method() === "GET") {
      return await json(
        route,
        {
          ...publishedProblemFixture.assignment,
          assignmentTiming: { timeLimitSeconds: storedSeconds },
        },
        200,
        { etag: '"7"' },
      );
    }
    if (
      path ===
      `/api/problems/${publishedProblemFixture.catalogProblem.problem}/versions/${publishedProblemFixture.catalogProblem.version}/detail`
    ) {
      return await json(route, {
        summary: publishedProblemFixture.catalogProblem,
        prompt: [],
        statistics: "unavailable",
      });
    }
    if (
      path === `/api/courses/${courseId}/assignments/${assignmentId}` &&
      request.method() === "PUT"
    ) {
      const body = request.postDataJSON() as unknown;
      saves.push(body);
      const requestBody = body as {
        readonly title: string;
        readonly problems: unknown;
        readonly policies: unknown;
        readonly assignmentTiming: unknown;
      };
      return await json(
        route,
        {
          ...publishedProblemFixture.assignment,
          title: requestBody.title,
          policies: requestBody.policies,
          assignmentTiming: requestBody.assignmentTiming,
        },
        200,
        { etag: '"8"' },
      );
    }
    if (path === "/api/problems/search") {
      return await json(route, {
        items: [],
        nextCursor: null,
        facets: {
          taxonomy: [],
          capabilities: [],
          licenses: [],
          statistics: { available: 0, unavailable: 0 },
        },
      });
    }
    return await json(route, { error: "unexpected request" }, 500);
  });

  await openEditor(page);
  const minutes = page.getByLabel("Minutes per practice run");
  await expect(minutes).toHaveValue("35791394.11666667");
  await page.getByLabel("Assignment title").fill("Renamed without changing timing");
  await page.getByRole("button", { name: "Save assignment" }).click();
  expect(saves).toHaveLength(1);
  expect(saves[0]).toMatchObject({
    title: "Renamed without changing timing",
    assignmentTiming: { timeLimitSeconds: storedSeconds },
  });
});
