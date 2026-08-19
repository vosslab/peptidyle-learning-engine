// Built-app student perspective and fail-closed instructor-route evidence.
// Selector contract: src/app.tsx owns Primary navigation; src/route_access_boundary.tsx owns
// routeAccessDenied; src/pages/assignment_overview_page.tsx owns assignmentOverview.

import { expect, test, type Page, type Route } from "@playwright/test";

import type { LearnerAssignmentProgress } from "../../generated/api/LearnerAssignmentProgress";
import type { LearnerAssignmentSummary } from "../../generated/api/LearnerAssignmentSummary";
import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import { ROUTE_CONTRACT, type RouteContract } from "../../src/route_contract";
import { captureDocumentationScreenshot } from "./docs_screenshot_capture";
import {
  CORPUS_VIEWPORT_SIZES,
  surfaceByName,
  type CorpusArtifact,
  type CorpusSurface,
} from "./ui_corpus_manifest";

const outputDirectory = process.env["PLE_STUDENT_ACCESS_VISUALS_DIR"];
const course = {
  ...publishedProblemFixture.course,
  title: "BIOC 301: Protein Structure",
  role: "student" as const,
};
const assignment: LearnerAssignmentSummary = {
  id: publishedProblemFixture.assignment.id,
  reference: publishedProblemFixture.assignment.reference,
  title: "Protein structure practice",
  items: publishedProblemFixture.assignment.items,
  selectionGroups: publishedProblemFixture.assignment.selectionGroups,
};
const progress: LearnerAssignmentProgress = {
  scoreState: "available",
  currentScore: 0.75,
  bestScore: 1,
  latestScore: 0.75,
  completedRunCount: 2,
  totalQuestionAttempts: 5,
  lastActivityAt: 1_786_000_004_100,
  classStatistics: {
    state: "insufficientEvidence",
  },
};

const INTERNAL_UUID_PATTERN =
  /[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}/iu;
const PRIVILEGED_COPY_PATTERN =
  /answer key|disclosure policy|effective policy|tenant|gradebook|roster|student records/iu;

interface StrictStudentApi {
  readonly instructorRequests: string[];
  readonly unexpectedRequests: string[];
}

function json(route: Route, value: unknown, status = 200): Promise<void> {
  return route.fulfill({
    status,
    contentType: "application/json",
    body: JSON.stringify(value),
  });
}

function isInstructorTransport(pathname: string): boolean {
  return (
    pathname.startsWith("/api/navigation/") ||
    pathname.startsWith("/api/problems") ||
    pathname.startsWith("/api/workspaces") ||
    /^\/api\/courses\/[^/]+\/(?:appearance|assignments|gradebook|roster)(?:\/|$)/u.test(pathname)
  );
}

async function installStrictStudentApi(page: Page): Promise<StrictStudentApi> {
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
    const request = route.request();
    const pathname = new URL(request.url()).pathname;
    if (isInstructorTransport(pathname)) instructorRequests.push(pathname);
    if (pathname === "/api/auth/session") {
      return await json(route, {
        authenticated: true,
        tenant: course.tenant,
        user: {
          id: publishedProblemFixture.enrollment.user,
          displayName: "Avery Student",
          roles: ["student"],
        },
      });
    }
    if (pathname === "/api/auth/account/presentation") {
      return await json(route, { contrast: "standard" });
    }
    if (pathname === "/api/auth/passkeys") {
      return await json(route, [
        {
          id: "0198e000-0000-7000-8000-000000000901",
          label: "Student laptop",
          createdAtMillis: 1_780_000_000_000,
          lastUsedAtMillis: 1_786_000_000_000,
        },
      ]);
    }
    if (pathname === "/api/courses") {
      return await json(route, { items: [course], nextCursor: null });
    }
    if (pathname === `/api/navigation/${course.reference}`) {
      return await json(route, { kind: "course", courseId: course.id });
    }
    if (pathname === `/api/navigation/${assignment.reference}`) {
      return await json(route, {
        kind: "assignment",
        courseId: course.id,
        assignmentId: assignment.id,
      });
    }
    if (pathname === `/api/courses/${course.id}`) return await json(route, course);
    if (pathname === `/api/courses/${course.id}/appearance`) {
      return await route.fulfill({
        status: 200,
        headers: {
          "content-type": "application/json",
          "cache-control": "no-store",
          etag: '"1"',
        },
        body: JSON.stringify({ theme: "grass", revision: "1", banner: null }),
      });
    }
    if (pathname === `/api/assignments/${assignment.id}/learner`) {
      return await json(route, assignment);
    }
    if (pathname === `/api/assignments/${assignment.id}/summary`) {
      return await json(route, progress);
    }
    unexpectedRequests.push(`${request.method()} ${pathname}`);
    return await json(route, { error: "Unexpected student fixture request" }, 500);
  });
  return { instructorRequests, unexpectedRequests };
}

function manifestSurface(name: string): CorpusSurface {
  const surface = surfaceByName(name);
  if (surface === undefined) throw new Error(`student visual surface is missing: ${name}`);
  return surface;
}

function requiredSurface(name: string): CorpusSurface {
  const surface = manifestSurface(name);
  if (surface.requiredViewports === undefined) {
    throw new Error(`student visual surface is missing its required matrix: ${name}`);
  }
  return surface;
}

function materializePath(routePath: string): string {
  return routePath
    .replace(":courseRef", course.reference)
    .replace(":assignmentRef", assignment.reference)
    .replace(":problemRef", publishedProblemFixture.catalogProblem.questionId)
    .replace(":workspaceRef", "W-1");
}

function materializeRoute(route: RouteContract): string {
  return materializePath(route.path);
}

function requiredRoute(routeId: RouteContract["id"]): RouteContract {
  const route = ROUTE_CONTRACT.find((candidate) => candidate.id === routeId);
  if (route === undefined) throw new Error(`${routeId} route is missing`);
  return route;
}

async function expectStudentNavigation(page: Page): Promise<void> {
  const navigation = page.getByRole("navigation", { name: "Primary navigation" });
  await expect(navigation.getByRole("link")).toHaveText(["Courses", "Account"]);
  await expect(navigation.getByRole("link", { name: "Library" })).toHaveCount(0);
  await expect(navigation.getByRole("link", { name: "Workspace" })).toHaveCount(0);
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const dimensions = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth);
}

async function expectPublicStudentPixels(
  page: Page,
  allowInstructorDenial: boolean,
): Promise<void> {
  const visible = await page.locator("body").innerText();
  expect(visible).not.toMatch(INTERNAL_UUID_PATTERN);
  expect(visible).not.toMatch(PRIVILEGED_COPY_PATTERN);
  if (!allowInstructorDenial) {
    expect(visible).not.toMatch(/instructor tools|instructors only/iu);
  }
  expect(new URL(page.url()).pathname).not.toMatch(INTERNAL_UUID_PATTERN);
  await expectNoHorizontalOverflow(page);
}

async function stabilize(page: Page): Promise<void> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.evaluate(async () => {
    await document.fonts.ready;
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
}

async function captureArtifact(page: Page, artifact: CorpusArtifact): Promise<void> {
  await stabilize(page);
  await captureDocumentationScreenshot(
    page,
    artifact.path.slice(artifact.path.lastIndexOf("/") + 1),
    undefined,
    undefined,
    outputDirectory,
  );
}

test("student assignment overview uses only the learner projection across required viewports", async ({
  page,
}) => {
  test.setTimeout(60_000);
  const api = await installStrictStudentApi(page);
  const surface = requiredSurface("studentAllowedAssignmentOverview");
  const route = materializePath(surface.route);
  expect(surface.route).toBe(requiredRoute("assignmentOverview").path);

  for (const artifact of surface.artifacts) {
    await page.setViewportSize(CORPUS_VIEWPORT_SIZES[artifact.viewport]);
    await page.goto(route);
    await expect(page.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
    await expect(page.getByRole("heading", { name: assignment.title })).toBeVisible();
    await expect(page.locator(".assignment-facts").getByRole("status")).toContainText(
      "Score available",
    );
    await expect(page.getByText("Not enough evidence to show class statistics.")).toBeVisible();
    await expectStudentNavigation(page);
    await expectPublicStudentPixels(page, false);
    expect(api.unexpectedRequests).toEqual([]);
    await captureArtifact(page, artifact);
  }
});

test("student account remains available through the shared student navigation", async ({
  page,
}) => {
  const api = await installStrictStudentApi(page);
  const surface = manifestSurface("accountSecurity");
  const artifact = surface.artifacts[0];
  if (artifact === undefined) throw new Error("account security artifact is missing");
  await page.setViewportSize(CORPUS_VIEWPORT_SIZES[artifact.viewport]);
  await page.goto(surface.route);
  await expect(page.locator('[data-route-surface="accountSecurity"]')).toBeVisible();
  await expect(page.getByText("Student laptop", { exact: true })).toBeVisible();
  await expectStudentNavigation(page);
  await expectPublicStudentPixels(page, false);
  expect(api.instructorRequests).toEqual([]);
  expect(api.unexpectedRequests).toEqual([]);
  await captureArtifact(page, artifact);
});

test("student direct instructor routes deny before component transport and cover roster and gradebook", async ({
  page,
}) => {
  test.setTimeout(60_000);
  const api = await installStrictStudentApi(page);
  const protectedRoutes = ROUTE_CONTRACT.filter((route) => route.requiredRoles.length > 0);

  for (const route of protectedRoutes) {
    api.instructorRequests.length = 0;
    api.unexpectedRequests.length = 0;
    await page.goto(materializeRoute(route));
    const denial = page.locator('[data-route-surface="routeAccessDenied"]');
    await expect(denial).toBeVisible();
    await expect(denial).toHaveAttribute("data-denied-route", route.id);
    await expect(
      page.locator('[data-route-surface]:not([data-route-surface="routeAccessDenied"])'),
    ).toHaveCount(0);
    await expect(page.locator("[data-course-theme]")).toHaveCount(0);
    await expectStudentNavigation(page);
    await expectPublicStudentPixels(page, true);
    expect(api.instructorRequests, route.id).toEqual([]);
    expect(api.unexpectedRequests, route.id).toEqual([]);
  }

  expect(protectedRoutes.some((route) => route.id === "gradebook")).toBe(true);
  expect(protectedRoutes.some((route) => route.id === "courseRoster")).toBe(true);

  const surface = requiredSurface("studentInstructorRouteDenial");
  const representativeRoute = materializePath(surface.route);
  expect(surface.route).toBe(requiredRoute("gradebook").path);
  for (const artifact of surface.artifacts) {
    api.instructorRequests.length = 0;
    api.unexpectedRequests.length = 0;
    await page.setViewportSize(CORPUS_VIEWPORT_SIZES[artifact.viewport]);
    await page.goto(representativeRoute);
    await expect(page.locator('[data-route-surface="routeAccessDenied"]')).toHaveAttribute(
      "data-denied-route",
      "gradebook",
    );
    await expect(page.locator("[data-course-theme]")).toHaveCount(0);
    await expectPublicStudentPixels(page, true);
    expect(api.instructorRequests).toEqual([]);
    expect(api.unexpectedRequests).toEqual([]);
    await captureArtifact(page, artifact);
  }
});
