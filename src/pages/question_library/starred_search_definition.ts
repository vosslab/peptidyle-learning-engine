// Search state and browser mapping for one Instructor's private Starred collection.

import type { LibraryFacetField } from "../../../generated/api/LibraryFacetField";
import type { RecordFilterChoices } from "../../components/data_sheet/data_sheet_fields";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type {
  SearchDefinition,
  SearchPage,
  SearchPageRequest,
} from "../../components/search_block/search_definition";
import type { ApiClient } from "../../api/client";
import type { StarredQuestionSort } from "../../api/question_star";
import {
  libraryFacetAggregates,
  libraryObjectSearchRequest,
  questionLibraryRowFromResult,
} from "../../api/question_library_repository";
import {
  type LibraryObjectQuestionRow,
  type QuestionLibraryBrowseQuery,
  type QuestionLibraryFilterCounts,
  type LibrarySearchPage,
  type QuestionLibraryPageSize,
} from "../../api/question_library_projection";
import {
  EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  normalizeQuestionLibraryBrowseQuery,
} from "../../features/question_library/library_query";
import { questionLibraryContent } from "../../features/question_library/question_library_search_definition";

export interface StarredBrowseQuery extends QuestionLibraryBrowseQuery {
  readonly starredSort: StarredQuestionSort;
}

export interface StarredQuestionRow extends LibraryObjectQuestionRow {
  readonly starredAtMicros: number;
}

export const INITIAL_STARRED_QUERY: StarredBrowseQuery = {
  ...EMPTY_QUESTION_LIBRARY_BROWSE_QUERY,
  kind: "questions",
  questions: "all",
  sort: "titleAscending",
  starredSort: "starredDateDescending",
};

export function normalizedStarredQuery(query: StarredBrowseQuery): StarredBrowseQuery {
  return {
    ...normalizeQuestionLibraryBrowseQuery({
      ...query,
      kind: "questions",
      questions: "all",
      sort: "titleAscending",
    }),
    starredSort: query.starredSort,
  };
}

export function starredQuestionContent(
  row: StarredQuestionRow,
  onUnstar: (questionId: string) => void,
): RecordContent {
  const content = questionLibraryContent(row);
  const date = new Date(Math.floor(row.starredAtMicros / 1000));
  return {
    ...content,
    actions: [
      {
        id: "open",
        kind: "link",
        label: "Open",
        href: `/library/${encodeURIComponent(row.displayId)}?revisionNumber=${row.publishedQuestionRevisionTuple.revisionNumber}`,
      },
      {
        id: "unstar",
        kind: "command",
        label: "Unstar",
        onClick: () => onUnstar(row.displayId),
      },
    ],
    details: [
      ...content.details,
      {
        kind: "time",
        label: "Starred",
        value: date.toLocaleString(),
        dateTime: date.toISOString(),
      },
    ],
  };
}

/** The private endpoint uses the public Library filter vocabulary and fixed server scope. */
export function starredSearchRepository(client: ApiClient): {
  search: (
    query: QuestionLibraryBrowseQuery,
    page: SearchPageRequest,
  ) => Promise<LibrarySearchPage>;
  listFacetChoices: (
    field: LibraryFacetField,
    query: QuestionLibraryBrowseQuery,
    search: string,
    cursor: string | null,
  ) => Promise<RecordFilterChoices>;
} {
  return {
    async search(
      query: QuestionLibraryBrowseQuery,
      pageRequest: SearchPageRequest,
    ): Promise<LibrarySearchPage> {
      const starredQuery = query as StarredBrowseQuery;
      const request = libraryObjectSearchRequest(
        { ...starredQuery, kind: "questions", questions: "all", sort: "titleAscending" },
        pageRequest,
        "any",
      );
      const page = await client.listStarredQuestions(
        request,
        starredQuery.starredSort,
        pageRequest,
      );
      const rawRows = page.items.map((item) =>
        questionLibraryRowFromResult({ kind: "question", ...item }),
      );
      return {
        items: rawRows.map((row, index) => ({
          ...row,
          starredAtMicros: page.items[index]!.starredAtMicros,
        })),
        pageNumber: page.pageNumber,
        pageSize: page.pageSize as QuestionLibraryPageSize,
        totalCount: page.totalCount,
        aggregates: libraryFacetAggregates(page),
        facetTruncation: {
          authorNames: page.facets.authorNamesTruncated,
          tags: page.facets.tagsTruncated,
          subjects: page.facets.subjectsTruncated,
          topics: page.facets.topicsTruncated,
        },
      } satisfies LibrarySearchPage;
    },
    async listFacetChoices(
      field: LibraryFacetField,
      query: QuestionLibraryBrowseQuery,
      search: string,
      cursor: string | null,
    ): Promise<RecordFilterChoices> {
      const starredQuery = query as StarredBrowseQuery;
      const request = {
        ...libraryObjectSearchRequest(
          { ...starredQuery, kind: "questions", questions: "all", sort: "titleAscending" },
          null,
          "any",
        ),
        cursor,
      };
      return client.listStarredFacetChoices(field, request, starredQuery.starredSort, search);
    },
  };
}

export function starredSearchDefinition(
  client: ApiClient,
  onUnstar: (questionId: string) => void,
): SearchDefinition<StarredBrowseQuery, StarredQuestionRow, QuestionLibraryFilterCounts> {
  const repository = starredSearchRepository(client);
  return {
    initialQuery: INITIAL_STARRED_QUERY,
    cleanup: normalizedStarredQuery,
    isEligible: () => true,
    getText: (query) => query.search,
    setText: (query, search) => ({ ...query, search }),
    rowId: (row) => row.displayId,
    content: (row) => starredQuestionContent(row, onUnstar),
    fetchPage: async (
      query,
      pageRequest,
    ): Promise<SearchPage<StarredQuestionRow, QuestionLibraryFilterCounts>> => {
      const page = await repository.search(query, pageRequest);
      if (page.items.some((row) => row.kind !== "question"))
        throw new Error("Starred page contained a Pool");
      return {
        items: page.items as ReadonlyArray<StarredQuestionRow>,
        pageNumber: page.pageNumber,
        pageSize: page.pageSize,
        totalCount: page.totalCount,
        filterCounts: { aggregates: page.aggregates, facetTruncation: page.facetTruncation },
      };
    },
  };
}
