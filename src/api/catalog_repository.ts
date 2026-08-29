// catalog_repository.ts - converts the generated catalog search contract for the library UI.

import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { CatalogAuthorship } from "../../generated/api/CatalogAuthorship";
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
const BACKENDS = ["native", "webwork", "qti", "h5p", "imathas"] as const;
const RESPONSE_FAMILIES = [
  "numeric",
  "multipleChoice",
  "shortText",
  "multiBlank",
  "matching",
  "ordering",
  "hotspot",
  "fileUpload",
  "externalTool",
] as const;

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

function selectedBackend(value: string | null): CatalogSearchQuery["backends"] {
  if (value === null) return [];
  const selected = BACKENDS.find((candidate) => candidate === value);
  if (selected === undefined) throw new Error("Catalog backend selection is invalid");
  return [selected];
}

function selectedResponseFamily(value: string | null): CatalogSearchQuery["response_families"] {
  if (value === null) return [];
  const selected = RESPONSE_FAMILIES.find((candidate) => candidate === value);
  if (selected === undefined) throw new Error("Catalog response family selection is invalid");
  return [selected];
}

function selectedPublicText(value: string | null): Array<string> {
  return value === null ? [] : [value];
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

function evidenceFilter(value: string | null): CatalogSearchQuery["evidence"] {
  return value === "available" || value === "unavailable" ? value : "any";
}

function facets(
  page: Awaited<ReturnType<ApiClient["searchCatalog"]>>,
): ReadonlyArray<CatalogFacetAggregate> {
  return [
    ...page.facets.bylines.map((facet) => ({
      group: "byline" as const,
      value: facet.byline,
      count: facet.count,
    })),
    ...page.facets.backends.map((facet) => ({
      group: "backend" as const,
      value: facet.backend,
      count: facet.count,
    })),
    ...page.facets.tags.map((facet) => ({
      group: "tag" as const,
      value: facet.tag,
      count: facet.count,
    })),
    ...page.facets.responseFamilies.map((facet) => ({
      group: "responseFamily" as const,
      value: facet.responseFamily,
      count: facet.count,
    })),
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
    { group: "evidence" as const, value: "available", count: page.facets.evidence.available },
    {
      group: "evidence" as const,
      value: "unavailable",
      count: page.facets.evidence.unavailable,
    },
    { group: "usedInMyCourses" as const, value: "used", count: page.facets.usedInMyCourses.used },
  ];
}

/** Builds the one closed catalog request used by Library and source-aware pickers. */
export function catalogSearchRequest(
  query: CatalogBrowseQuery,
  cursor: string | null,
  authorship: CatalogAuthorship = "any",
): CatalogSearchQuery {
  return {
    text: query.search === "" ? null : query.search,
    bylines: selectedPublicText(query.byline),
    backends: selectedBackend(query.backend),
    tags: selectedPublicText(query.tag),
    response_families: selectedResponseFamily(query.responseFamily),
    taxonomy: taxonomyFilter(query.taxonomy),
    capabilities: selectedCapability(query.capability),
    licenses: selectedLicense(query.license),
    evidence: evidenceFilter(query.evidence),
    used_in_my_courses: query.usedInMyCourses === "used" ? "used" : "any",
    authorship,
    cursor,
    page_size: CATALOG_PAGE_SIZE,
  };
}

/** The only production bridge from the generated client into the virtual catalog surface. */
export function createCatalogRepository(
  client: ApiClient,
  authorship: CatalogAuthorship = "any",
): CatalogBrowseRepository {
  return {
    async search(query: CatalogBrowseQuery, cursor: string | null): Promise<unknown> {
      const search = catalogSearchRequest(query, cursor, authorship);
      const page = await client.searchCatalog(search);
      return {
        items: page.items.map((item) => ({
          displayId: item.summary.questionId,
          title: item.summary.metadata.title,
          summary: `Published ${item.summary.backend} problem.`,
          byline: item.summary.byline.names,
          taxonomy: item.summary.metadata.taxonomy.map((term) => `${term.scheme}:${term.code}`),
          capabilities: item.summary.capabilities,
          license: item.summary.metadata.license.kind,
          evidence:
            item.evidence.state === "available"
              ? {
                  state: "available" as const,
                  observedCourseCount: item.evidence.observedCourseCount,
                  independentLearnerObservationCount:
                    item.evidence.independentLearnerObservationCount,
                  difficultyIndex: item.evidence.difficultyIndex,
                  discriminationIndex: item.evidence.discriminationIndex ?? undefined,
                }
              : { state: "insufficientEvidence" as const },
        })),
        nextCursor: page.nextCursor,
        aggregates: facets(page),
      } satisfies CatalogBrowsePage;
    },
  };
}
