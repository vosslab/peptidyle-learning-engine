// Public Blueprint Course rules supplied to the shared search interface.
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type {
  BlueprintCourseClassificationSearch,
  BlueprintCourseClient,
  BlueprintCourseListSort,
} from "../api/blueprint_course";
import type { SearchDefinition, SearchPage } from "../features/search/search_session";
import { publicBlueprintContent } from "./blueprint_course_search_result";
import { emptyBlueprintClassificationSearch } from "./blueprint_course_search_classification";

/** One committed Public Blueprint Course search, owned beside its server definition. */
export type BlueprintCourseSearchQuery = {
  readonly query: string;
  readonly promotedOnly: boolean;
  readonly classification: BlueprintCourseClassificationSearch;
  readonly classificationDescription: string;
  readonly sort: BlueprintCourseListSort;
  readonly tag: string;
};

export function emptyBlueprintSearch(): BlueprintCourseSearchQuery {
  return {
    query: "",
    promotedOnly: false,
    classification: emptyBlueprintClassificationSearch(),
    classificationDescription: "",
    sort: "name",
    tag: "",
  };
}

export function blueprintCourseSearchDefinition(
  client: Pick<BlueprintCourseClient, "listBlueprintCourses">,
): SearchDefinition<
  BlueprintCourseSearchQuery,
  BlueprintCourseSummaryView,
  undefined,
  BlueprintCourseListSort
> {
  return {
    initialQuery: emptyBlueprintSearch(),
    cleanup: (query) => ({ ...query, query: query.query.trim(), tag: query.tag.trim() }),
    getText: (query) => query.query,
    setText: (query, text) => ({ ...query, query: text }),
    rowId: (course) => course.id,
    content: publicBlueprintContent,
    fetchPage: async (
      query,
      cursor,
      pageSize,
    ): Promise<SearchPage<BlueprintCourseSummaryView, undefined>> => {
      const page = await client.listBlueprintCourses(
        cursor ?? undefined,
        pageSize,
        false,
        query.query,
        true,
        query.promotedOnly,
        query.classification,
        query.sort,
        query.tag,
      );
      return { items: page.items, nextCursor: page.nextCursor, filterCounts: undefined };
    },
    sort: {
      label: "Sort Public Blueprint Courses",
      options: [
        { value: "name", label: "Name" },
        { value: "adoptions", label: "Adoptions" },
        { value: "students", label: "Students having taken the course" },
        { value: "stars", label: "Stars" },
        { value: "watches", label: "Watches" },
        { value: "recentEdits", label: "Most recently edited" },
      ],
      get: (query) => query.sort,
      set: (query, sort) => ({ ...query, sort }),
    },
    appliedFilters: (query) => [
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
      ...(query.tag !== ""
        ? [
            {
              id: "tag",
              label: `Tag: ${query.tag}`,
              clear: (current: BlueprintCourseSearchQuery) => ({ ...current, tag: "" }),
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
