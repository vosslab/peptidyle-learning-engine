// Public Blueprint Course rules supplied to the shared search interface.
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { Accessor } from "solid-js";
import type {
  BlueprintCourseClassificationSearch,
  BlueprintCourseClient,
  BlueprintCourseListSort,
  BlueprintCourseFacetFilters,
  BlueprintCourseFacetField,
} from "../api/blueprint_course";
import type { SearchDefinition, SearchPage } from "../features/search/search_session";
import { blueprintCourseContent } from "./blueprint_course_search_result";
import { emptyBlueprintClassificationSearch } from "./blueprint_course_search_classification";

/** One committed Public Blueprint Course search, owned beside its server definition. */
export type BlueprintCourseSearchQuery = {
  readonly query: string;
  readonly promotedOnly: boolean;
  readonly classification: BlueprintCourseClassificationSearch;
  readonly classificationDescription: string;
  readonly sort: BlueprintCourseListSort;
  readonly includeArchived: boolean;
  readonly facets: BlueprintCourseFacetFilters;
};

export function emptyBlueprintFacets(): BlueprintCourseFacetFilters {
  return { disciplineUuids: [], subjectUuids: [], topicUuids: [], subtopicUuids: [], tags: [] };
}

export function emptyBlueprintSearch(includeArchived = false): BlueprintCourseSearchQuery {
  return {
    query: "",
    promotedOnly: false,
    classification: emptyBlueprintClassificationSearch(),
    classificationDescription: "",
    sort: "name",
    includeArchived,
    facets: emptyBlueprintFacets(),
  };
}

export function blueprintCourseSearchDefinition(
  client: Pick<BlueprintCourseClient, "listBlueprintCourses">,
  formatTimestamp: Accessor<(timestamp: number | Date) => string>,
  scope: "public" | "all" = "public",
): SearchDefinition<BlueprintCourseSearchQuery, BlueprintCourseSummaryView, undefined> {
  return {
    initialQuery: emptyBlueprintSearch(scope === "all"),
    cleanup: (query) => ({ ...query, query: query.query.trim() }),
    isEligible: (query) =>
      scope === "all" ||
      query.query.length > 0 ||
      query.promotedOnly ||
      query.classification.disciplineUuid !== null ||
      Object.values(query.facets).some((values) => values.length > 0),
    getText: (query) => query.query,
    setText: (query, text) => ({ ...query, query: text }),
    rowId: (course) => course.id,
    content: (course) => blueprintCourseContent(course, formatTimestamp()),
    fetchPage: async (
      query,
      cursor,
      pageSize,
    ): Promise<SearchPage<BlueprintCourseSummaryView, undefined>> => {
      const page = await client.listBlueprintCourses(
        cursor ?? undefined,
        pageSize,
        scope === "all" && query.includeArchived,
        query.query,
        scope === "public",
        query.promotedOnly,
        query.classification,
        query.sort,
        "",
        query.facets,
      );
      return { items: page.items, nextCursor: page.nextCursor, filterCounts: undefined };
    },
    appliedFilters: (query) => [
      ...(
        ["disciplineUuids", "subjectUuids", "topicUuids", "subtopicUuids", "tags"] as const
      ).flatMap((field: BlueprintCourseFacetField) =>
        query.facets[field].length === 0
          ? []
          : [
              {
                id: field,
                label: `${field === "tags" ? "Tags" : field.replace("Uuids", "")}: ${query.facets[field].length}`,
                clear: (current: BlueprintCourseSearchQuery) => ({
                  ...current,
                  facets: { ...current.facets, [field]: [] },
                }),
              },
            ],
      ),
      ...(query.promotedOnly
        ? [
            {
              id: "promoted",
              label: "Promoted only",
              clear: (current: BlueprintCourseSearchQuery) => ({
                ...current,
                promotedOnly: false,
              }),
            },
          ]
        : []),
      ...(scope === "all" && query.includeArchived
        ? [
            {
              id: "archived",
              label: "Include Archived",
              clear: (current: BlueprintCourseSearchQuery) => ({
                ...current,
                includeArchived: false,
              }),
            },
          ]
        : []),
      ...(query.classificationDescription !== ""
        ? [
            {
              id: "classification",
              label: query.classificationDescription,
              clear: (current: BlueprintCourseSearchQuery) => ({
                ...current,
                classification: emptyBlueprintClassificationSearch(),
                classificationDescription: "",
              }),
            },
          ]
        : []),
    ],
  };
}
