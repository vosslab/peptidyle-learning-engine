// Visible instructor course creation through the production HTTP component boundary.

import { expect, test, type Page, type Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";

const createdCourse = {
  ...publishedProblemFixture.course,
  id: "0198e000-0000-7000-8000-000000000099",
  publicId: 2,
  title: "BIOC 301: Biochemistry",
  role: "instructor",
};

function session(roles: ReadonlyArray<string>): unknown {
  return {
    authenticated: true,
    tenant: publishedProblemFixture.course.tenant,
    user: {
      id: "0198e000-0000-7000-8000-000000000114",
      displayName: "Course instructor",
      roles,
    },
  };
}

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

async function useProductionTransport(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
}

test("an instructor creates a course by keyboard and opens its real course link", async ({
  page,
}) => {
  test.setTimeout(2_000);
  const requests: Array<{
    readonly method: string;
    readonly path: string;
    readonly body: unknown;
  }> = [];
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    requests.push({ method: request.method(), path, body: request.postDataJSON() as unknown });
    if (path === "/api/auth/session") return await json(route, session(["instructor"]));
    if (path === "/api/courses" && request.method() === "GET") {
      return await json(route, { items: [publishedProblemFixture.course], nextCursor: null });
    }
    if (path === "/api/courses" && request.method() === "POST")
      return await json(route, createdCourse, 201);
    if (path === `/api/navigation/C-${createdCourse.publicId}`) {
      return await json(route, { kind: "course", courseId: createdCourse.id });
    }
    if (path === `/api/courses/${createdCourse.id}`) return await json(route, createdCourse);
    if (path === `/api/courses/${createdCourse.id}/appearance`) {
      return await json(route, { theme: "grass", revision: "1", banner: null }, 200, {
        "cache-control": "no-store",
        etag: '"1"',
      });
    }
    if (path === `/api/courses/${createdCourse.id}/assignments`) {
      return await json(route, { items: [], nextCursor: null });
    }
    return await json(route, { error: `unexpected request ${path}` }, 500);
  });
  await useProductionTransport(page);
  await page.goto("/");

  const title = page.getByLabel("Course title");
  await expect(title).toBeVisible();
  await title.fill(createdCourse.title);
  await title.press("Enter");
  const newLink = page.locator(`a[href="/courses/C-${createdCourse.publicId}"]`);
  const newCourse = page.locator("article.course-card").filter({ has: newLink });
  await expect(newCourse.getByRole("heading", { name: createdCourse.title })).toBeVisible();
  await expect(newLink).toBeFocused();
  expect(requests.filter((request) => request.method === "POST")).toEqual([
    { method: "POST", path: "/api/courses", body: { title: createdCourse.title } },
  ]);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Assignments", exact: true })).toBeVisible();
});

test("a student sees no course creation control and cannot call its endpoint", async ({ page }) => {
  const requests: Array<{ readonly method: string; readonly path: string }> = [];
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    requests.push({ method: request.method(), path });
    if (path === "/api/auth/session") return await json(route, session(["student"]));
    if (path === "/api/courses")
      return await json(route, { items: [publishedProblemFixture.course], nextCursor: null });
    return await json(route, { error: `unexpected request ${path}` }, 500);
  });
  await useProductionTransport(page);
  await page.goto("/");

  await expect(page.getByLabel("Course title")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Create course" })).toHaveCount(0);
  expect(
    requests.some((request) => request.method === "POST" && request.path === "/api/courses"),
  ).toBe(false);
});

test("a recoverable creation failure preserves the typed title and exposes retry guidance", async ({
  page,
}) => {
  test.setTimeout(2_000);
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/auth/session") return await json(route, session(["sysadmin"]));
    if (path === "/api/courses" && request.method() === "GET") {
      return await json(route, { items: [], nextCursor: null });
    }
    if (path === "/api/courses" && request.method() === "POST") {
      return await json(route, { error: "title rejected" }, 422);
    }
    return await json(route, { error: `unexpected request ${path}` }, 500);
  });
  await useProductionTransport(page);
  await page.goto("/");

  const title = page.getByLabel("Course title");
  await expect(title).toBeVisible();
  await title.fill("BIOC 301: Biochemistry");
  await title.press("Enter");
  await expect(
    page.getByRole("status").filter({ hasText: "We could not create that course" }),
  ).toBeVisible();
  await expect(title).toHaveValue("BIOC 301: Biochemistry");
  await expect(page.getByRole("button", { name: "Create course" })).toBeEnabled();
});
