// frontend_contract.spec.ts - built-artifact proof for the WP-C9 reference slice.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";
import fs from "node:fs";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import { respondCatalog } from "../../src/api/mock/handlers/catalog";
import { ROUTE_CONTRACT, type RouteContract } from "../../src/route_contract";

declare global {
  interface Window {
    __PLE_ROUTE_FAILURE_TEST__?: () => boolean;
  }
}

const IDS = {
  course: "C-1",
  assignment: "A-1",
  run: "R-4",
  problem: "7K3-M9QP",
  workspace: "W-1",
} as const;

const SAVED_ATTEMPT_KEY =
  "ple:attempt:0198e000-0000-7000-8000-000000000001:0198e000-0000-7000-8000-000000000023:0198e000-0000-7000-8000-000000000033";
const SAVED_ATTEMPT_BUFFER = JSON.stringify({
  response: { kind: "multipleChoice", selected: ["0002"] },
  idempotencyKey: "saved-response-key",
});

async function navigateWithinSpa(page: Page, pathname: string): Promise<void> {
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, pathname);
}

async function tabTo(page: Page, target: ReturnType<Page["locator"]>, limit = 20): Promise<void> {
  for (let index = 0; index < limit; index += 1) {
    if (await target.evaluate((element) => document.activeElement === element)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`Tab did not reach ${await target.getAttribute("aria-label")}`);
}

async function expectNoBlockingAxeViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).include("main").analyze();
  const blocking = results.violations
    .filter((violation) => violation.impact === "critical" || violation.impact === "serious")
    .map((violation) => ({
      id: violation.id,
      impact: violation.impact,
      targets: violation.nodes.flatMap((node) => node.target),
    }));
  expect(blocking).toEqual([]);
}

function json(route: Route, value: unknown, headers: Record<string, string> = {}): Promise<void> {
  return route.fulfill({
    status: 200,
    contentType: "application/json",
    headers,
    body: JSON.stringify(value),
  });
}

async function fulfillResponse(route: Route, response: Response): Promise<void> {
  await route.fulfill({
    status: response.status,
    headers: Object.fromEntries(response.headers.entries()),
    body: await response.text(),
  });
}

function materializeRoute(route: RouteContract): string {
  return route.path
    .replace(":courseRef", IDS.course)
    .replace(":assignmentRef", IDS.assignment)
    .replace(":runRef", IDS.run)
    .replace(":problemRef", IDS.problem)
    .replace(":workspaceRef", IDS.workspace);
}

function isInstructorTransport(pathname: string): boolean {
  return (
    pathname.startsWith("/api/navigation/") ||
    pathname.startsWith("/api/problems") ||
    pathname.startsWith("/api/workspaces") ||
    /^\/api\/courses\/[^/]+\/(?:appearance|assignments|gradebook|roster)(?:\/|$)/u.test(pathname)
  );
}

interface StudentBoundaryApi {
  readonly instructorRequests: string[];
  readonly unexpectedRequests: string[];
}

async function installStudentBoundaryApi(page: Page): Promise<StudentBoundaryApi> {
  const instructorRequests: string[] = [];
  const unexpectedRequests: string[] = [];
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.route("**/api/**", async (route) => {
    const pathname = new URL(route.request().url()).pathname;
    if (pathname === "/api/auth/session") {
      await json(route, {
        authenticated: true,
        tenant: publishedProblemFixture.course.tenant,
        user: {
          id: publishedProblemFixture.enrollment.user,
          displayName: "Fixture Student",
          roles: ["student"],
        },
      });
      return;
    }
    if (pathname === "/api/auth/account/presentation") {
      await json(route, { contrast: "standard" });
      return;
    }
    if (pathname === "/api/courses") {
      await json(route, { items: [], nextCursor: null });
      return;
    }
    if (isInstructorTransport(pathname)) {
      instructorRequests.push(pathname);
    } else {
      unexpectedRequests.push(pathname);
    }
    await route.fulfill({ status: 500, body: "unexpected route-boundary transport" });
  });
  return { instructorRequests, unexpectedRequests };
}

test("preview serves the SPA shell only for declared browser document routes", async ({
  request,
}) => {
  const documentHeaders = { accept: "text/html" };
  const declaredRoute = await request.get(`/library/${IDS.problem}`, {
    headers: documentHeaders,
  });
  expect(declaredRoute.status()).toBe(200);
  expect(declaredRoute.headers()["content-type"]).toContain("text/html");
  const shell = await declaredRoute.text();
  expect(shell).not.toContain("<base");
  expect(shell).toMatch(/src="\/main\.js\?v=[a-f0-9]+"/);
  expect(shell).toMatch(/href="\/main\.css\?v=[a-f0-9]+"/);
  expect(shell).toMatch(/href="\/style\.css\?v=[a-f0-9]+"/);

  const refusals = await Promise.all([
    request.get("/unknown-learning-space", { headers: documentHeaders }),
    request.get("/missing-static.js", { headers: documentHeaders }),
    request.get("/api/missing", { headers: documentHeaders }),
    request.get("/api/assets/missing.svg", { headers: documentHeaders }),
    request.get(`/library/${IDS.problem}`, { headers: { accept: "application/json" } }),
    request.post(`/library/${IDS.problem}`, { headers: documentHeaders }),
  ]);
  expect(refusals.map((response) => response.status())).toEqual([404, 404, 404, 404, 404, 404]);
});

test("all product routes resolve inside the persistent shell", async ({ page }) => {
  const routes = [
    { path: "/", surface: "courses" },
    { path: "/sign-in", surface: "signIn" },
    { path: "/auth/email/complete", surface: "emailAuthenticationComplete" },
    { path: "/auth/account/email/complete", surface: "emailChangeComplete" },
    { path: "/course-invitations/redeem", surface: "courseInvitation" },
    { path: "/account/security", surface: "accountSecurity" },
    {
      path: "/account/co-instructor-invitations",
      surface: "accountPendingInvitations",
    },
    { path: `/courses/${IDS.course}`, surface: "courseAssignments" },
    {
      path: `/courses/${IDS.course}/assignments/${IDS.assignment}`,
      surface: "assignmentOverview",
    },
    { path: `/runs/${IDS.run}`, surface: "runAttempt" },
    { path: `/runs/${IDS.run}/summary`, surface: "runSummary" },
    { path: "/library", surface: "library", restricted: true },
    {
      path: `/library/${IDS.problem}`,
      surface: "problemDetail",
      restricted: true,
    },
    { path: "/workspace", surface: "workspaceList", restricted: true },
    {
      path: `/workspace/${IDS.workspace}`,
      surface: "workspaceEditor",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/assignments/new`,
      surface: "assignmentCreate",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/assignments/${IDS.assignment}/edit`,
      surface: "assignmentEditor",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/assignments/${IDS.assignment}/access`,
      surface: "assignmentAccess",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/gradebook`,
      surface: "gradebook",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/grade-settings`,
      surface: "courseGradeSettings",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/appearance`,
      surface: "courseAppearance",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/students`,
      surface: "courseRoster",
      restricted: true,
    },
    {
      path: `/instructor/courses/${IDS.course}/teaching-operations`,
      surface: "teachingOperations",
      restricted: true,
    },
  ];

  await page.goto("/");
  await expect(page.getByRole("link", { name: "Peptidyle home" })).toBeVisible();

  for (const route of routes) {
    await navigateWithinSpa(page, route.path);
    if (route.restricted === true) {
      const denial = page.locator('[data-route-surface="routeAccessDenied"]');
      await expect(denial).toBeVisible();
      await expect(denial).toHaveAttribute("data-denied-route", route.surface);
    } else {
      await expect(page.locator(`[data-route-surface="${route.surface}"]`)).toBeVisible();
    }
    await expect(page.locator("header.site-header")).toBeVisible();
  }
});

test("an instructor can invite a student through the platform keyboard path", async ({ page }) => {
  const requests: string[] = [];
  page.on("request", (request) => requests.push(new URL(request.url()).pathname));
  const course = { ...publishedProblemFixture.course, role: "instructor" };
  const invitation = {
    invitationId: "0198e000-0000-7000-8000-000000000601",
    email: "new.student@mail.roosevelt.edu",
    rosterId: "900123457",
    status: "pending",
    expiresAt: 1_755_411_600_000,
  } as const;
  let pendingInvitations: ReadonlyArray<typeof invitation> = [];
  const roster = (): unknown => ({
    rosterMode: "emailEnrollment",
    members: [
      {
        memberId: "0198e000-0000-7000-8000-000000000602",
        displayName: "Fixture Student",
        rosterEmail: "student@mail.roosevelt.edu",
        rosterId: "900123456",
        role: "student",
        status: "active",
      },
    ],
    pendingInvitations,
    allowedEmailDomains: [{ domain: "mail.roosevelt.edu", includeSubdomains: false }],
    signupPosture: "invitationOnly",
    nextCursor: null,
    rosterRevision: pendingInvitations.length + 1,
  });
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
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
    if (path === `/api/courses/${course.id}/assignments`) {
      const { id, reference, title } = publishedProblemFixture.assignment;
      return await json(route, { items: [{ id, reference, title }], nextCursor: null });
    }
    if (path === `/api/courses/${course.id}/roster`) return await json(route, roster());
    if (path === `/api/courses/${course.id}/invitations` && request.method() === "POST") {
      pendingInvitations = [invitation];
      return await json(
        route,
        {
          invitation,
          redemptionPath: `/course-invitations/redeem#token=${"A".repeat(43)}`,
          emailDelivery: "queued",
        },
        { "cache-control": "no-store" },
      );
    }
    return await route.fulfill({ status: 404, body: "not found" });
  });
  await page.goto("/");
  await navigateWithinSpa(page, `/instructor/courses/${IDS.course}/students`);
  const email = page.getByLabel("Institutional email");
  await expect(email).toBeVisible();
  await expect(page.getByRole("button", { name: /Add .*Fake Student/u })).toHaveCount(0);
  await expect(page.getByText("Add local teaching student", { exact: true })).toHaveCount(0);
  expect(requests.some((path) => path.endsWith("/local-teaching-members"))).toBe(false);
  await tabTo(page, email, 30);
  await page.keyboard.type("new.student@mail.roosevelt.edu");
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("Institutional student ID")).toBeFocused();
  await page.keyboard.type("900123457");
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "Create invitation" })).toBeFocused();
  await page.keyboard.press("Enter");

  await expect(
    page.getByRole("cell", { name: "new.student@mail.roosevelt.edu", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("900123457")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Share this invitation" })).toBeVisible();
  await expect(page.getByLabel("Invitation link")).toHaveValue(
    /^http:\/\/[^/]+\/course-invitations\/redeem#token=[A-Za-z0-9_-]{43}$/u,
  );
  await expect(page.getByRole("button", { name: "Copy invitation link" })).toBeVisible();
  await expectNoBlockingAxeViolations(page);
});

test("student navigation hides restricted controls and focuses main content", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("link", { name: "Workspace" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Library" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Courses" })).toBeVisible();

  await page.getByRole("link", { name: "Account" }).click();
  await expect(page.locator("#main-content")).toBeFocused();
  await expect(page.locator('[data-route-surface="accountSecurity"]')).toBeVisible();

  await page.getByRole("link", { name: "Skip to learning content" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();
});

test("manual student workspace navigation mounts no authoring transport", async ({ page }) => {
  const requests: string[] = [];
  page.on("request", (request) => requests.push(new URL(request.url()).pathname));

  await page.goto("/");
  requests.length = 0;
  await navigateWithinSpa(page, `/workspace/${IDS.workspace}`);

  await expect(page.locator('[data-route-surface="routeAccessDenied"]')).toHaveAttribute(
    "data-denied-route",
    "workspaceEditor",
  );
  await expect(page.locator('[data-route-surface="workspaceEditor"]')).toHaveCount(0);
  expect(
    requests.filter(
      (path) => path.startsWith("/api/workspaces") || path.includes("author-preview"),
    ),
  ).toEqual([]);
});

test("manual student catalog-detail navigation mounts no catalog transport", async ({ page }) => {
  const requests: string[] = [];
  page.on("request", (request) => requests.push(new URL(request.url()).pathname));

  await page.goto("/");
  requests.length = 0;
  await navigateWithinSpa(page, `/library/${IDS.problem}`);

  await expect(page.locator('[data-route-surface="routeAccessDenied"]')).toHaveAttribute(
    "data-denied-route",
    "problemDetail",
  );
  await expect(page.locator('[data-route-surface="problemDetail"]')).toHaveCount(0);
  expect(requests.filter((path) => path.startsWith("/api/problems/by-id/"))).toEqual([]);
});

test("student deep links never mount or transport instructor route surfaces", async ({ page }) => {
  test.setTimeout(60_000);
  const failedAssets: string[] = [];
  page.on("response", (response) => {
    const resourceType = response.request().resourceType();
    if ((resourceType === "script" || resourceType === "stylesheet") && !response.ok()) {
      failedAssets.push(`${response.status()} ${new URL(response.url()).pathname}`);
    }
  });
  const api = await installStudentBoundaryApi(page);
  const protectedRoutes = ROUTE_CONTRACT.filter((route) => route.requiredRoles.length > 0);

  for (const route of protectedRoutes) {
    api.instructorRequests.length = 0;
    api.unexpectedRequests.length = 0;
    const pathname = materializeRoute(route);
    const response = await page.goto(pathname);
    expect(response?.status(), route.id).toBe(200);
    const denial = page.locator('[data-route-surface="routeAccessDenied"]');
    await expect(denial).toHaveAttribute("data-denied-route", route.id);
    await expect(
      page.locator('[data-route-surface]:not([data-route-surface="routeAccessDenied"])'),
    ).toHaveCount(0);
    await expect(page.locator("[data-course-theme]")).toHaveCount(0);
    expect(api.instructorRequests, `${route.id} page.goto`).toEqual([]);
    expect(api.unexpectedRequests, `${route.id} page.goto`).toEqual([]);

    api.instructorRequests.length = 0;
    api.unexpectedRequests.length = 0;
    const reloadResponse = await page.reload();
    expect(reloadResponse?.status(), route.id).toBe(200);
    await expect(denial).toHaveAttribute("data-denied-route", route.id);
    await expect(
      page.getByRole("heading", { name: "This page is available to instructors only" }),
    ).toBeFocused();
    expect(api.instructorRequests, `${route.id} reload`).toEqual([]);
    expect(api.unexpectedRequests, `${route.id} reload`).toEqual([]);
  }

  const recoveryLink = page.getByRole("link", { name: "Return to courses" });
  await expect(page.getByRole("alert")).toContainText("available to instructors only");
  await page.keyboard.press("Tab");
  await expect(recoveryLink).toBeFocused();
  await expectNoBlockingAxeViolations(page);
  await page.keyboard.press("Enter");
  await expect(page.locator('[data-route-surface="courses"]')).toBeVisible();
  expect(failedAssets).toEqual([]);
});

test("unknown SPA navigation recovers safely without protected transport", async ({ page }) => {
  const api = await installStudentBoundaryApi(page);
  await page.goto("/");
  await expect(page.locator('[data-route-surface="courses"]')).toBeVisible();
  api.instructorRequests.length = 0;
  api.unexpectedRequests.length = 0;

  await navigateWithinSpa(page, "/unknown-learning-space");
  await expect(page.locator('[data-route-surface="notFound"]')).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "That page is not part of this learning space" }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "Return to courses" })).toBeVisible();
  await expect(page.locator("[data-course-theme]")).toHaveCount(0);
  expect(api.instructorRequests).toEqual([]);
  expect(api.unexpectedRequests).toEqual([]);
});

test("sign-out unmounts a protected surface and prevents remount transport", async ({ page }) => {
  const catalogRequests: string[] = [];
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const pathname = new URL(request.url()).pathname;
    if (pathname === "/api/auth/session") {
      await json(route, {
        authenticated: true,
        tenant: publishedProblemFixture.course.tenant,
        user: {
          id: publishedProblemFixture.enrollment.user,
          displayName: "Catalog instructor",
          roles: ["instructor"],
        },
      });
      return;
    }
    if (pathname === "/api/auth/account/presentation") {
      await json(route, { contrast: "standard" });
      return;
    }
    if (pathname === "/api/auth/logout" && request.method() === "POST") {
      await json(route, { authenticated: false });
      return;
    }
    if (pathname === "/api/problems" || pathname.startsWith("/api/problems/")) {
      catalogRequests.push(pathname);
      await fulfillResponse(
        route,
        respondCatalog(
          new Request(request.url(), {
            method: request.method(),
            headers: request.headers(),
          }),
        ),
      );
      return;
    }
    await route.fulfill({ status: 404, body: "unexpected session-transition request" });
  });

  await page.goto("/library");
  await expect(page.locator('[data-route-surface="library"]')).toBeVisible();
  const requestsBeforeSignOut = catalogRequests.length;
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.locator('[data-route-surface="signIn"]')).toBeVisible();
  await expect(page.locator('[data-route-surface="library"]')).toHaveCount(0);

  await navigateWithinSpa(page, "/library");
  await expect(page.locator('[data-session-state="signedOut"]')).toBeVisible();
  await expect(page.locator('[data-route-surface="library"]')).toHaveCount(0);
  expect(catalogRequests).toHaveLength(requestsBeforeSignOut);
});

test("a route failure keeps the shell usable and omits raw exception details", async ({ page }) => {
  await page.addInitScript(() => {
    let shouldFail = true;
    window.__PLE_ROUTE_FAILURE_TEST__ = (): boolean => {
      const requested = shouldFail;
      shouldFail = false;
      return requested;
    };
  });
  await page.goto("/");

  await expect(page.locator("header.site-header")).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary navigation" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Try this page again" })).toBeVisible();
  await expect(page.getByText("route-boundary-test-sentinel")).toHaveCount(0);

  await page.getByRole("button", { name: "Try this page again" }).click();
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
});

test("header navigation leaves a failed route and renders the selected surface", async ({
  page,
}) => {
  await page.addInitScript(() => {
    let shouldFail = true;
    window.__PLE_ROUTE_FAILURE_TEST__ = (): boolean => {
      const requested = shouldFail;
      shouldFail = false;
      return requested;
    };
  });
  await page.goto("/");

  await expect(page.getByRole("button", { name: "Try this page again" })).toBeVisible();
  await page.getByRole("link", { name: "Account" }).click();
  await expect(page.locator("header.site-header")).toBeVisible();
  await expect(page.locator('[data-route-surface="accountSecurity"]')).toBeVisible();
});

test("a student reaches, validates, submits, and advances through the generated reference response", async ({
  page,
}) => {
  const failedAssetRequests: string[] = [];
  page.on("response", (response) => {
    if (new URL(response.url()).pathname.startsWith("/api/assets/") && !response.ok()) {
      failedAssetRequests.push(`${response.status()} ${response.url()}`);
    }
  });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();

  await page.getByRole("link", { name: "Open course" }).click();
  await expect(page.getByRole("heading", { name: "Assignments" })).toBeVisible();
  await page.getByRole("link", { name: "Start assignment" }).click();
  await expect(page.getByRole("heading", { name: "Peptide bond mastery" })).toBeVisible();
  await page.getByRole("button", { name: "Start or continue practice" }).click();

  await expect(
    page.getByRole("heading", { name: "Peptide bond resonance and planarity", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Question", exact: true })).toBeVisible();
  await expect(page.locator('[data-route-surface="runAttempt"]')).not.toContainText(
    "Question content ready.",
  );
  const images = page.locator("img.question-renderer__image");
  await expect(images).toHaveCount(2);
  await expect
    .poll(async () =>
      images.evaluateAll((nodes) =>
        nodes.every((image) => (image as HTMLImageElement).naturalWidth > 0),
      ),
    )
    .toBe(true);
  expect(failedAssetRequests).toEqual([]);
  const radios = page.getByRole("radio");
  await expect(radios).toHaveCount(3);
  await radios.nth(1).check();
  await expect(radios.nth(1)).toBeChecked();
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );

  if (process.env["PLE_CAPTURE_VISUALS"] === "1") {
    fs.mkdirSync("generated/ui", { recursive: true });
    await page.screenshot({ path: "generated/ui/wp_c9_run_desktop.png", fullPage: true });
  }

  const selectedTarget = radios.nth(1).locator("xpath=ancestor::label");
  const box = await selectedTarget.boundingBox();
  expect(box?.height).toBeGreaterThanOrEqual(44);

  await page.getByRole("button", { name: "Submit answer" }).click();
  // Feedback focus timing is covered by the component acceptance fixture; the
  // integrated run flow verifies the panel mounts and remains actionable.
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(
    page.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Start another practice" })).toBeVisible();
});

test("a student completes the primary platform-key course-to-answer path without a pointer", async ({
  page,
}) => {
  await page.goto("/");

  await tabTo(page, page.getByRole("link", { name: "Skip to learning content" }));
  await expect(page.getByRole("link", { name: "Skip to learning content" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const openCourse = page.getByRole("link", { name: "Open course" });
  await tabTo(page, openCourse);
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const reviewAssignment = page.getByRole("link", { name: "Start assignment" });
  await tabTo(page, reviewAssignment);
  await page.keyboard.press("Enter");
  await expect(page.locator("#main-content")).toBeFocused();

  const start = page.getByRole("button", { name: "Start or continue practice" });
  await tabTo(page, start);
  await page.keyboard.press("Space");
  await expect(page.locator("#main-content")).toBeFocused();

  const radios = page.getByRole("radio");
  await tabTo(page, radios.first());
  await page.keyboard.press("Space");
  await expect(radios.first()).toBeChecked();
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );

  const submit = page.getByRole("button", { name: "Submit answer" });
  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(radios.first()).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Space");

  const continueButton = page.getByRole("button", { name: "Continue" });
  await tabTo(page, continueButton);
  await page.keyboard.press("Space");
  await expect(
    page.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible();

  const back = page.getByRole("button", { name: "Back to assignment" });
  await tabTo(page, back);
  await page.keyboard.press("Space");
  await expect(page.getByRole("heading", { name: "Peptide bond mastery" })).toBeVisible();
});

test("student question and feedback surfaces have no serious or critical axe violations", async ({
  page,
}) => {
  await page.goto("/");
  await navigateWithinSpa(page, `/runs/${IDS.run}`);
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();
  await expectNoBlockingAxeViolations(page);

  const radios = page.getByRole("radio");
  await radios.nth(1).check();
  await page.getByRole("button", { name: "Submit answer" }).click();
  await expect(page.getByRole("heading", { name: "Feedback", exact: true })).toBeVisible();
  await expectNoBlockingAxeViolations(page);
});

test("session recovery stays editable, never writes the attempt to local storage, and clears on exit", async ({
  page,
}) => {
  await page.addInitScript(
    ({ key, buffer }) => {
      sessionStorage.setItem(key, buffer);
    },
    { key: SAVED_ATTEMPT_KEY, buffer: SAVED_ATTEMPT_BUFFER },
  );

  await page.goto("/");
  await navigateWithinSpa(page, `/runs/${IDS.run}`);
  const radios = page.getByRole("radio");
  await expect(radios.nth(1)).toBeChecked();
  const restoredStorage = await page.evaluate((key) => {
    return {
      localAttempt: localStorage.getItem(key),
      sessionAttempt: sessionStorage.getItem(key),
    };
  }, SAVED_ATTEMPT_KEY);
  expect(restoredStorage).toEqual({
    localAttempt: null,
    sessionAttempt: SAVED_ATTEMPT_BUFFER,
  });

  await radios.nth(2).check();
  await expect(radios.nth(2)).toBeChecked();
  await expect(radios.nth(1)).not.toBeChecked();
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );
  const editedStorage = await page.evaluate((key) => {
    return {
      localAttempt: localStorage.getItem(key),
      sessionAttempt: sessionStorage.getItem(key),
    };
  }, SAVED_ATTEMPT_KEY);
  expect(editedStorage.localAttempt).toBeNull();
  expect(editedStorage.sessionAttempt).not.toBeNull();
  expect(editedStorage.sessionAttempt).not.toBe(SAVED_ATTEMPT_BUFFER);
  expect(editedStorage.sessionAttempt).toContain('"selected":["0003"]');
  expect(editedStorage.sessionAttempt).toContain('"idempotencyKey":');

  await navigateWithinSpa(page, "/");
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
  const exitedStorage = await page.evaluate((key) => {
    return {
      localAttempt: localStorage.getItem(key),
      sessionAttempt: sessionStorage.getItem(key),
    };
  }, SAVED_ATTEMPT_KEY);
  expect(exitedStorage).toEqual({ localAttempt: null, sessionAttempt: null });
});

test("the reference response remains usable at the 320 CSS-pixel baseline", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.goto("/");
  await navigateWithinSpa(page, `/runs/${IDS.run}`);
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();

  const hasHorizontalOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  expect(hasHorizontalOverflow).toBe(false);
  await expect(page.getByRole("button", { name: "Submit answer" })).toBeVisible();
  if (process.env["PLE_CAPTURE_VISUALS"] === "1") {
    fs.mkdirSync("generated/ui", { recursive: true });
    await page.screenshot({ path: "generated/ui/wp_c9_run_320.png", fullPage: true });
  }
});
