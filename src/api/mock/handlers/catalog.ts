import { publishedProblemFixture } from "../../../../generated/fixtures/published_problem";
import type { CatalogProblemDetail } from "../../../../generated/api/CatalogProblemDetail";
import type { CatalogSearchPage } from "../../../../generated/api/CatalogSearchPage";

import { handlesResource, jsonResponse, pathSegments, routeNotFound } from "./shared";

export function canHandleCatalog(request: Request): boolean {
  return handlesResource(request, ["problems", "taxonomy"]);
}

function catalogDetailFixture(): CatalogProblemDetail {
  return {
    summary: publishedProblemFixture.catalogProblem,
    prompt: publishedProblemFixture.publishedProblem.prompt,
    statistics: "unavailable",
  };
}

function catalogSearchFixture(request: Request): CatalogSearchPage | Response {
  const parameters = new URL(request.url).searchParams;
  const allowed = new Set([
    "text",
    "taxonomy",
    "capabilities",
    "licenses",
    "statistics",
    "cursor",
    "pageSize",
  ]);
  for (const key of parameters.keys()) {
    if (!allowed.has(key)) {
      return jsonResponse({ error: `Unknown catalog search parameter ${key}` }, 400);
    }
  }
  const pageSize = parameters.get("pageSize");
  if (pageSize !== null && (!/^[1-9][0-9]*$/.test(pageSize) || Number(pageSize) > 100)) {
    return jsonResponse({ error: "Invalid catalog page size" }, 400);
  }
  const statistics = parameters.get("statistics") ?? "any";
  if (!["any", "available", "unavailable"].includes(statistics)) {
    return jsonResponse({ error: "Invalid catalog statistics filter" }, 400);
  }
  const summary = publishedProblemFixture.catalogProblem;
  const normalizedText = (parameters.get("text") ?? "").trim().toLowerCase();
  const textMatches =
    normalizedText.length === 0 ||
    summary.metadata.title.toLowerCase().includes(normalizedText) ||
    summary.metadata.tags.some((tag) => tag.toLowerCase().includes(normalizedText));
  const taxonomyMatches = parameters
    .getAll("taxonomy")
    .every((filter) =>
      summary.metadata.taxonomy.some((term) => `${term.scheme}:${term.code}` === filter),
    );
  const capabilitiesMatch = parameters
    .getAll("capabilities")
    .every((capability) => summary.capabilities.some((candidate) => candidate === capability));
  const licensesMatch = parameters
    .getAll("licenses")
    .some((license) => summary.metadata.license.kind === license);
  const statisticsMatch = statistics === "any" || statistics === "unavailable";
  const includesSummary =
    textMatches &&
    taxonomyMatches &&
    capabilitiesMatch &&
    (parameters.getAll("licenses").length === 0 || licensesMatch) &&
    statisticsMatch &&
    parameters.get("cursor") === null;
  return {
    items: includesSummary ? [summary] : [],
    nextCursor: null,
    facets: {
      taxonomy: summary.metadata.taxonomy.map((term) => ({ term, count: 1 })),
      capabilities: summary.capabilities.map((capability) => ({ capability, count: 1 })),
      licenses: [{ license: summary.metadata.license.kind, count: 1 }],
      statistics: { available: 0, unavailable: 1 },
    },
  };
}

export function respondCatalog(request: Request): Response {
  const segments = pathSegments(request);
  const resource = segments[1];
  if (request.method === "GET" && resource === "problems" && segments[2] === "search") {
    const page = catalogSearchFixture(request);
    return page instanceof Response ? page : jsonResponse(page);
  }
  if (
    request.method === "GET" &&
    resource === "problems" &&
    segments.length === 6 &&
    segments[2] === publishedProblemFixture.publishedProblem.problem &&
    segments[3] === "versions" &&
    segments[4] === publishedProblemFixture.publishedProblem.version &&
    segments[5] === "detail"
  ) {
    return jsonResponse(catalogDetailFixture());
  }
  if (request.method === "GET" && resource === "problems" && segments.length === 2) {
    return jsonResponse({ items: [publishedProblemFixture.catalogProblem], nextCursor: null });
  }
  if (
    request.method === "GET" &&
    resource === "problems" &&
    segments.length === 5 &&
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
    return jsonResponse({
      items: publishedProblemFixture.publishedProblem.metadata.taxonomy,
      nextCursor: null,
    });
  }
  return routeNotFound(request);
}
