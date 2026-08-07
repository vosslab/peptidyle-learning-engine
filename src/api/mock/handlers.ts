// handlers.ts - dependency-free mock API backed by the typed WP-C7 corpus.

import {
  publishedProblemAssetBodies,
  publishedProblemFixture,
} from "../../../generated/fixtures/published_problem";

/** Planned server route groups that UI work may use before routes exist. */
export const MOCK_ROUTE_GROUPS = ["auth", "catalog", "course", "run", "asset"] as const;

export type MockRouteGroup = (typeof MOCK_ROUTE_GROUPS)[number];

/** Fetch-compatible function implemented entirely in the browser or Node. */
export type MockFetch = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

/** One route-group handler in the mock API. */
export interface MockApiHandler {
  readonly group: MockRouteGroup;
  readonly canHandle: (request: Request) => boolean;
  readonly respond: (request: Request) => Response;
}

const MOCK_ORIGIN = "https://mock.peptidyle.invalid";

function requestFrom(input: RequestInfo | URL, init?: RequestInit): Request {
  if (input instanceof Request) {
    return new Request(input, init);
  }
  const url = new URL(input.toString(), MOCK_ORIGIN);
  return new Request(url, init);
}

function pathSegments(request: Request): ReadonlyArray<string> {
  return new URL(request.url).pathname.split("/").filter(Boolean);
}

function routeResource(request: Request): string | undefined {
  const segments = pathSegments(request);
  if (segments[0] !== "api") {
    return undefined;
  }
  return segments[1];
}

function handlesResource(request: Request, resources: ReadonlyArray<string>): boolean {
  const resource = routeResource(request);
  return resource !== undefined && resources.includes(resource);
}

function jsonResponse(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

function routeNotFound(request: Request): Response {
  const url = new URL(request.url);
  return jsonResponse({ error: `No mock route for ${request.method} ${url.pathname}` }, 404);
}

function methodNotAllowed(request: Request): Response {
  return jsonResponse(
    { error: `Method ${request.method} is not supported by this mock route` },
    405,
  );
}

function canHandleAuth(request: Request): boolean {
  return handlesResource(request, ["auth"]);
}

function respondAuth(request: Request): Response {
  const segments = pathSegments(request);
  const action = segments[2];
  if (request.method === "POST" && action === "logout") {
    return jsonResponse({ authenticated: false });
  }
  if (
    (request.method === "POST" && action === "login") ||
    (request.method === "GET" && action === "session")
  ) {
    return jsonResponse({
      authenticated: true,
      tenant: publishedProblemFixture.enrollment.tenant,
      user: {
        id: publishedProblemFixture.enrollment.student,
        displayName: "Fixture Student",
      },
    });
  }
  return routeNotFound(request);
}

function canHandleCatalog(request: Request): boolean {
  return handlesResource(request, ["problems", "taxonomy"]);
}

function respondCatalog(request: Request): Response {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (request.method === "GET" && resource === "problems" && segments.length === 2) {
    return jsonResponse({ items: [publishedProblemFixture.publishedProblem], nextCursor: null });
  }
  if (
    request.method === "GET" &&
    resource === "problems" &&
    segments[2] === publishedProblemFixture.publishedProblem.problem &&
    segments[3] === "versions" &&
    segments[4] === publishedProblemFixture.publishedProblem.version
  ) {
    return jsonResponse(publishedProblemFixture.publishedProblem);
  }
  if (request.method === "POST" && resource === "problems" && segments[3] === "publish") {
    return jsonResponse(publishedProblemFixture.publishedProblem, 201);
  }
  if (request.method === "GET" && resource === "taxonomy") {
    return jsonResponse({ items: publishedProblemFixture.publishedProblem.metadata.taxonomy });
  }
  return routeNotFound(request);
}

function canHandleCourse(request: Request): boolean {
  return handlesResource(request, ["courses", "assignments", "enrollments"]);
}

function respondCourse(request: Request): Response {
  if (request.method !== "GET") {
    return methodNotAllowed(request);
  }
  const segments = pathSegments(request);
  const resource = segments[1];
  if (resource === "courses" && segments.length === 2) {
    return jsonResponse({ items: [publishedProblemFixture.course], nextCursor: null });
  }
  if (
    resource === "courses" &&
    segments[2] === publishedProblemFixture.course.id &&
    segments[3] === "assignments"
  ) {
    return jsonResponse({ items: [publishedProblemFixture.assignment], nextCursor: null });
  }
  if (resource === "assignments" && segments[2] === publishedProblemFixture.assignment.id) {
    return jsonResponse(publishedProblemFixture.assignment);
  }
  if (resource === "enrollments" && segments[2] === publishedProblemFixture.enrollment.id) {
    return jsonResponse({
      enrollment: publishedProblemFixture.enrollment,
      summary: publishedProblemFixture.summary,
    });
  }
  return routeNotFound(request);
}

function canHandleRun(request: Request): boolean {
  return handlesResource(request, ["runs", "attempts", "submissions", "grading"]);
}

function responseForRun(request: Request, runId: string | undefined): Response {
  if (request.method === "POST" && runId === undefined) {
    const inProgress = publishedProblemFixture.runs.find((run) => run.completedAt === null);
    return inProgress === undefined
      ? jsonResponse({ error: "Fixture has no in-progress run" }, 500)
      : jsonResponse(inProgress, 201);
  }
  if (request.method !== "GET" || runId === undefined) {
    return methodNotAllowed(request);
  }
  const run = publishedProblemFixture.runs.find((candidate) => candidate.id === runId);
  if (run === undefined) {
    return jsonResponse({ error: `Unknown fixture run ${runId}` }, 404);
  }
  return jsonResponse(run);
}

function responseForAttempt(attemptId: string | undefined): Response {
  const attempt = publishedProblemFixture.attempts.find((candidate) => candidate.id === attemptId);
  return attempt === undefined
    ? jsonResponse({ error: `Unknown fixture attempt ${attemptId ?? ""}` }, 404)
    : jsonResponse(attempt);
}

function respondRun(request: Request): Response {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (resource === "runs" && segments[3] === "attempts" && request.method === "GET") {
    const runAttempts = publishedProblemFixture.attempts.filter(
      (attempt) => attempt.run === segments[2],
    );
    return jsonResponse({ items: runAttempts, nextCursor: null });
  }
  if (resource === "runs") {
    return responseForRun(request, segments[2]);
  }
  if (resource === "attempts" && request.method === "GET") {
    return responseForAttempt(segments[2]);
  }
  if (resource === "submissions" && request.method === "POST") {
    const inProgress = publishedProblemFixture.attempts.find((attempt) => attempt.result === null);
    return inProgress === undefined
      ? jsonResponse({ error: "Fixture has no in-progress attempt" }, 500)
      : jsonResponse({ accepted: true, attempt: inProgress });
  }
  if (
    resource === "grading" &&
    segments[2] === "summaries" &&
    segments[3] === publishedProblemFixture.enrollment.id &&
    request.method === "GET"
  ) {
    return jsonResponse(publishedProblemFixture.summary);
  }
  return routeNotFound(request);
}

function canHandleAsset(request: Request): boolean {
  return handlesResource(request, ["assets"]);
}

function respondAsset(request: Request): Response {
  if (request.method !== "GET") {
    return methodNotAllowed(request);
  }
  const assetId = pathSegments(request)[2];
  const asset = publishedProblemFixture.assets.find((candidate) => candidate.id === assetId);
  const body = assetId === undefined ? undefined : publishedProblemAssetBodies[assetId];
  if (asset === undefined || body === undefined) {
    return jsonResponse({ error: `Unknown fixture asset ${assetId ?? ""}` }, 404);
  }
  return new Response(body, {
    status: 200,
    headers: {
      "content-type": asset.mediaType,
      etag: `"${asset.sha256}"`,
    },
  });
}

/** One handler per planned API route group. */
export const mockApiHandlers: ReadonlyArray<MockApiHandler> = [
  { group: "auth", canHandle: canHandleAuth, respond: respondAuth },
  { group: "catalog", canHandle: canHandleCatalog, respond: respondCatalog },
  { group: "course", canHandle: canHandleCourse, respond: respondCourse },
  { group: "run", canHandle: canHandleRun, respond: respondRun },
  { group: "asset", canHandle: canHandleAsset, respond: respondAsset },
];

/** Creates an isolated fetch replacement with no network fallback. */
export function createMockFetch(): MockFetch {
  function mockFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
    const request = requestFrom(input, init);
    const handler = mockApiHandlers.find((candidate) => candidate.canHandle(request));
    const response = handler === undefined ? routeNotFound(request) : handler.respond(request);
    return Promise.resolve(response);
  }

  return mockFetch;
}
