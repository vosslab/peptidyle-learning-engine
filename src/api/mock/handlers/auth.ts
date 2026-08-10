import { publishedProblemFixture } from "../../../../generated/fixtures/published_problem";

import { handlesResource, jsonResponse, pathSegments, routeNotFound } from "./shared";

export function canHandleAuth(request: Request): boolean {
  return handlesResource(request, ["auth"]);
}

export function respondAuth(request: Request): Response {
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
        id: publishedProblemFixture.enrollment.user,
        displayName: "Fixture Student",
        roles: ["student"],
      },
    });
  }
  return routeNotFound(request);
}
