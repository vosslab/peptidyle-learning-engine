// handlers.ts - dependency-free mock API backed by the typed WP-C7 corpus.

import { canHandleAsset, respondAsset } from "./handlers/assets";
import { canHandleAuth, respondAuth } from "./handlers/auth";
import { canHandleCatalog, respondCatalog } from "./handlers/catalog";
import {
  canHandleCourse,
  createMockCourseAppearanceState,
  respondCourse,
} from "./handlers/courses";
import { createMockAssignmentState } from "./handlers/authoring";
import { canHandleRun, respondRun } from "./handlers/runs";
import { requestFrom, routeNotFound } from "./handlers/shared";

/** Route groups with implemented mock capability owners. */
export const MOCK_ROUTE_GROUPS = ["auth", "catalog", "course", "run", "asset"] as const;

export type MockRouteGroup = (typeof MOCK_ROUTE_GROUPS)[number];

/** Fetch-compatible function implemented entirely in the browser or Node. */
export type MockFetch = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

/** One route-group handler in the mock API. */
export interface MockApiHandler {
  readonly group: MockRouteGroup;
  readonly canHandle: (request: Request) => boolean;
  readonly respond: (request: Request) => Response | Promise<Response>;
}

export {
  issuedEnvelopeForAttempt,
  issuedQuestionWireForAttempt,
  externalToolFixtureAttempt,
  mockAttemptById,
  mockExternalToolSubmissionReceipt,
  mockPrefetchedNextQuestion,
  mockPrefetchSubmissionReceipt,
  prefetchFixtureAttempt,
  prefetchedFixtureAttempt,
} from "./handlers/runs";
export { mockCourseAppearance, secondaryMockCourse } from "./handlers/courses";

/** One handler per implemented API route group. */
export const mockApiHandlers: ReadonlyArray<MockApiHandler> = [
  { group: "auth", canHandle: canHandleAuth, respond: respondAuth },
  { group: "catalog", canHandle: canHandleCatalog, respond: respondCatalog },
  { group: "course", canHandle: canHandleCourse, respond: respondCourse },
  { group: "run", canHandle: canHandleRun, respond: respondRun },
  { group: "asset", canHandle: canHandleAsset, respond: respondAsset },
];

/** Creates an isolated fetch replacement with no network fallback. */
export function createMockFetch(): MockFetch {
  const assignmentState = createMockAssignmentState();
  const appearanceState = createMockCourseAppearanceState();

  async function mockFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
    const request = requestFrom(input, init);
    const handler = mockApiHandlers.find((candidate) => candidate.canHandle(request));
    return handler === undefined
      ? routeNotFound(request)
      : handler.group === "course"
        ? await respondCourse(request, assignmentState, appearanceState)
        : await handler.respond(request);
  }

  return mockFetch;
}
