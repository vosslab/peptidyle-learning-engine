// catalog_repository.ts - converts the generated catalog search contract for the library UI.

import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { Capability } from "../../generated/api/Capability";
import type { CatalogLicenseValue } from "../../generated/api/CatalogLicenseValue";
import type { ApiClient } from "./client";
import type {
  CatalogBrowsePage,
  CatalogBrowseQuery,
  CatalogBrowseRepository,
  CatalogFacetAggregate,
} from "../pages/library_page_model";

const CATALOG_PAGE_SIZE = 50;
const CAPABILITIES = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "perQuestionTiming",
  "printExport",
  "offlinePreview",
] as const satisfies ReadonlyArray<Capability>;
const LICENSES = [
  "allRightsReserved",
  "ccBy",
  "ccBySa",
  "ccByNc",
  "cc0",
  "other",
] as const satisfies ReadonlyArray<CatalogLicenseValue>;

function selectedCapability(value: string | null): Array<Capability> {
  if (value === null) {
    return [];
  }
  const selected = CAPABILITIES.find((candidate) => candidate === value);
  if (selected === undefined) {
    throw new Error("Catalog capability selection is invalid");
  }
  return [selected];
}

function selectedLicense(value: string | null): Array<CatalogLicenseValue> {
  if (value === null) {
    return [];
  }
  const selected = LICENSES.find((candidate) => candidate === value);
  if (selected === undefined) {
    throw new Error("Catalog license selection is invalid");
  }
  return [selected];
}

function taxonomyFilter(value: string | null): CatalogSearchQuery["taxonomy"] {
  if (value === null) {
    return [];
  }
  const separator = value.indexOf(":");
  if (separator < 1 || separator === value.length - 1) {
    throw new Error("Catalog taxonomy selection is invalid");
  }
  return [{ scheme: value.slice(0, separator), code: value.slice(separator + 1) }];
}

function statisticsFilter(value: string | null): CatalogSearchQuery["statistics"] {
  return value === "available" || value === "unavailable" ? value : "any";
}

function facets(
  page: Awaited<ReturnType<ApiClient["searchCatalog"]>>,
): ReadonlyArray<CatalogFacetAggregate> {
  return [
    ...page.facets.taxonomy.map((facet) => ({
      group: "taxonomy" as const,
      value: `${facet.term.scheme}:${facet.term.code}`,
      count: facet.count,
    })),
    ...page.facets.capabilities.map((facet) => ({
      group: "capability" as const,
      value: facet.capability,
      count: facet.count,
    })),
    ...page.facets.licenses.map((facet) => ({
      group: "license" as const,
      value: facet.license,
      count: facet.count,
    })),
    { group: "statistic" as const, value: "available", count: page.facets.statistics.available },
    {
      group: "statistic" as const,
      value: "unavailable",
      count: page.facets.statistics.unavailable,
    },
  ];
}

/** The only production bridge from the generated client into the virtual catalog surface. */
export function createCatalogRepository(client: ApiClient): CatalogBrowseRepository {
  return {
    async search(query: CatalogBrowseQuery, cursor: string | null): Promise<unknown> {
      const search: CatalogSearchQuery = {
        text: query.search === "" ? null : query.search,
        taxonomy: taxonomyFilter(query.taxonomy),
        capabilities: selectedCapability(query.capability),
        licenses: selectedLicense(query.license),
        statistics: statisticsFilter(query.statistic),
        cursor,
        pageSize: CATALOG_PAGE_SIZE,
      };
      const page = await client.searchCatalog(search);
      return {
        items: page.items.map((item) => ({
          displayId: item.questionId,
          problemId: item.problem,
          versionId: item.version,
          title: item.metadata.title,
          summary: `Published ${item.backend} problem.`,
          taxonomy: item.metadata.taxonomy.map((term) => `${term.scheme}:${term.code}`),
          capabilities: item.capabilities,
          license: item.metadata.license.kind,
        })),
        nextCursor: page.nextCursor,
        aggregates: facets(page),
      } satisfies CatalogBrowsePage;
    },
  };
}
