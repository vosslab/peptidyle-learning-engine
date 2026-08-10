// Assignment editor selectors: src/pages/assignment_editor_page.tsx accessible labels and buttons.

import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";

const courseId = publishedProblemFixture.course.id;
const assignmentId = publishedProblemFixture.assignment.id;
const assignmentProblems = publishedProblemFixture.assignment.items
  .filter((item) => item.deliveryState === "active")
  .sort((left, right) => left.position - right.position)
  .map((item) => item.reference);
const editPath = `/instructor/courses/${courseId}/assignments/${assignmentId}/edit`;
const appearance = { theme: "grass", revision: "1", banner: null } as const;
const secondCatalogProblem = {
  ...publishedProblemFixture.catalogProblem,
  problem: "0198e000-0000-7000-8000-000000000097",
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

async function openEditor(page: Page): Promise<void> {
  // `tools/mock_preview_server.mjs` is a static artifact server without an SPA
  // fallback; it returns 404 for this direct deep link. The managed deployment
  // must provide that fallback. This proves the built router after its normal
  // index entry point, without claiming that the static test server is one.
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.goto("/");
  await page.evaluate((path: string) => {
    history.pushState({}, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, editPath);
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

test("student and global instructor who is a course learner see no assignment editor transport", async ({
  page,
}) => {
  const requests: string[] = [];
  await routeStudentCourse(page, requests);
  await openEditor(page);
  await expect(
    page.getByRole("heading", { name: "Assignment editing is not available for this account" }),
  ).toBeVisible();
  expect(new Set(requests)).toEqual(
    new Set([
      "/api/auth/session",
      `/api/courses/${courseId}`,
      `/api/courses/${courseId}/appearance`,
    ]),
  );
  expect(requests.some((path) => path.includes("assignments") || path.includes("problems"))).toBe(
    false,
  );
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
        { ...publishedProblemFixture.assignment, tenant: "0198e000-0000-7000-8000-000000000088" },
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
  expect(new Set(requests)).toEqual(
    new Set([
      "/api/auth/session",
      `/api/courses/${courseId}`,
      `/api/courses/${courseId}/appearance`,
      `/api/assignments/${assignmentId}`,
    ]),
  );
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
      return await json(route, publishedProblemFixture.assignment, 200, {
        etag: assignmentReads > 1 ? '"8"' : '"7"',
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
          ...publishedProblemFixture.assignment,
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
  await expect(page.getByRole("button", { name: "Save assignment" })).toBeVisible();

  await page.getByLabel("Search published problems").fill("peptide");
  await page.getByRole("button", { name: "Search catalog" }).click();
  await expect(page.getByRole("button", { name: "Already selected" })).toBeVisible();
  const addSecond = page.getByRole("button", { name: "Add published version" });
  await addSecond.focus();
  await expect(addSecond).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByText("Second immutable peptide version").first()).toBeVisible();
  expect(requests.filter((request) => request.method === "PUT")).toEqual([]);
  await title.focus();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: /Move Peptide bond resonance and planarity later/ }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: /Remove Peptide bond resonance and planarity/ }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: /Move Second immutable peptide version earlier/ }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("button", { name: /Remove Second immutable peptide version/ }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("Completion requirement")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("Grade policy")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("Continued practice")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("Variation policy")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("Search published problems")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "Search catalog" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "Save assignment" })).toBeFocused();
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
  await expect(page.locator(".assignment-editor-actions span")).toHaveText("Assignment saved.");

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
  });
  expect(puts[2]?.revision).toBe('"8"');
  expect(puts[2]?.body).toEqual({
    title: "Saved after reload",
    problems: assignmentProblems,
    policies: publishedProblemFixture.assignment.policies,
  });
  expect(JSON.stringify(puts[2]?.body)).not.toMatch(
    /workspace|source|prompt|response|grading|answerKey|capabilit/i,
  );
});
