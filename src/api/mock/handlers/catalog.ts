import { publishedProblemFixture } from "../../../../generated/fixtures/published_problem";
import type { CatalogProblemDetail } from "../../../../generated/api/CatalogProblemDetail";
import type { CatalogProblemSummary } from "../../../../generated/api/CatalogProblemSummary";
import type { CatalogSearchPage } from "../../../../generated/api/CatalogSearchPage";
import { normalizeQuestionIdSyntax } from "../../../question_id";

import { handlesResource, jsonResponse, pathSegments, routeNotFound } from "./shared";

/** Mirrors the live `ProblemDisplayRef` parser so mock route status is useful evidence. */
function normalizeQuestionId(reference: string): string | null {
  return normalizeQuestionIdSyntax(reference);
}

export function canHandleCatalog(request: Request): boolean {
  return handlesResource(request, ["problems", "taxonomy"]);
}

const disclosedEvidenceSummary: CatalogProblemSummary = {
  ...publishedProblemFixture.catalogProblem,
  questionId: "7K4-M9QP",
  metadata: {
    ...publishedProblemFixture.catalogProblem.metadata,
    title: "Peptide bond geometry: anonymous evidence example",
  },
};

interface CatalogFixtureRecord {
  readonly summary: CatalogProblemSummary;
  readonly statistics: CatalogProblemDetail["statistics"];
}

const catalogFixtureRecords: ReadonlyArray<CatalogFixtureRecord> = [
  { summary: publishedProblemFixture.catalogProblem, statistics: "unavailable" },
  {
    summary: disclosedEvidenceSummary,
    statistics: {
      available: {
        cohortSize: 48,
        difficultyIndex: 0.675,
        attemptsMean: 1.4,
        timeMedianSecondsEstimate: 120,
        discriminationIndex: 0.42,
      },
    },
  },
];

function catalogDetailFixture(record: CatalogFixtureRecord): CatalogProblemDetail {
  return {
    summary: record.summary,
    prompt: publishedProblemFixture.publishedProblem.prompt,
    statistics: record.statistics,
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
  const normalizedText = (parameters.get("text") ?? "").trim().toLowerCase();
  const items = catalogFixtureRecords
    .filter((record) => {
      const summary = record.summary;
      const textMatches =
        normalizedText.length === 0 ||
        normalizedText === summary.questionId.toLowerCase() ||
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
      const statisticsMatch =
        statistics === "any" ||
        (statistics === "available" && record.statistics !== "unavailable") ||
        (statistics === "unavailable" && record.statistics === "unavailable");
      return (
        textMatches &&
        taxonomyMatches &&
        capabilitiesMatch &&
        (parameters.getAll("licenses").length === 0 || licensesMatch) &&
        statisticsMatch
      );
    })
    .map((record) => record.summary);
  return {
    items: parameters.get("cursor") === null ? items : [],
    nextCursor: null,
    facets: {
      taxonomy: publishedProblemFixture.catalogProblem.metadata.taxonomy.map((term) => ({
        term,
        count: catalogFixtureRecords.length,
      })),
      capabilities: publishedProblemFixture.catalogProblem.capabilities.map((capability) => ({
        capability,
        count: catalogFixtureRecords.length,
      })),
      licenses: [
        {
          license: publishedProblemFixture.catalogProblem.metadata.license.kind,
          count: catalogFixtureRecords.length,
        },
      ],
      statistics: { available: 1, unavailable: 1 },
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
    segments.length === 4 &&
    segments[2] === "by-id"
  ) {
    const reference = segments[3] ?? "";
    const normalized = normalizeQuestionId(reference);
    if (normalized === null) {
      return jsonResponse({ error: "invalid problem reference" }, 400);
    }
    const record = catalogFixtureRecords.find(
      (candidate) => candidate.summary.questionId === normalized,
    );
    return record === undefined
      ? jsonResponse({ error: "problem reference not found" }, 404)
      : jsonResponse(record.summary);
  }
  if (
    request.method === "GET" &&
    resource === "problems" &&
    segments.length === 5 &&
    segments[2] === "by-id" &&
    segments[4] === "detail"
  ) {
    const record = catalogFixtureRecords.find(
      (candidate) => candidate.summary.questionId === normalizeQuestionId(segments[3] ?? ""),
    );
    return record === undefined
      ? jsonResponse({ error: "problem reference not found" }, 404)
      : jsonResponse(catalogDetailFixture(record));
  }
  if (request.method === "GET" && resource === "problems" && segments.length === 2) {
    return jsonResponse({ items: [publishedProblemFixture.catalogProblem], nextCursor: null });
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
