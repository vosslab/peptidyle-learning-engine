import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { Accessor } from "solid-js";
import type { ContentClassificationClient } from "../../api/content_classification";
import type {
  BlueprintCourseClient,
  BlueprintCourseFacetField,
  BlueprintCourseListSort,
} from "../../api/blueprint_course";
import type {
  RecordField,
  RecordFilterChoices,
  DataSheetQuery,
  RecordSort,
} from "../../components/data_sheet/data_sheet_fields";
import type { BlueprintCourseSearchQuery } from "./blueprint_course_search_definition";
import { blueprintVocabularyCell } from "./blueprint_course_vocabulary_cell";

export type BlueprintRecordSortKey =
  "name" | "adoptions" | "students" | "stars" | "watches" | "recentEdits";

const orders: ReadonlyArray<{
  readonly order: BlueprintCourseListSort;
  readonly sort: RecordSort<BlueprintRecordSortKey>;
}> = [
  { order: "name", sort: { key: "name", direction: "ascending" } },
  { order: "nameDescending", sort: { key: "name", direction: "descending" } },
  { order: "adoptions", sort: { key: "adoptions", direction: "descending" } },
  { order: "adoptionsAscending", sort: { key: "adoptions", direction: "ascending" } },
  { order: "students", sort: { key: "students", direction: "descending" } },
  { order: "studentsAscending", sort: { key: "students", direction: "ascending" } },
  { order: "stars", sort: { key: "stars", direction: "descending" } },
  { order: "starsAscending", sort: { key: "stars", direction: "ascending" } },
  { order: "watches", sort: { key: "watches", direction: "descending" } },
  { order: "watchesAscending", sort: { key: "watches", direction: "ascending" } },
  { order: "recentEdits", sort: { key: "recentEdits", direction: "descending" } },
  { order: "recentEditsAscending", sort: { key: "recentEdits", direction: "ascending" } },
];

/** Compact fields and stable IDs shared by both Blueprint collections. */
export function blueprintCourseSearchFields(
  client: ContentClassificationClient,
  scope: "active" | "all",
  formatTimestamp: Accessor<(timestamp: number | Date) => string>,
): ReadonlyArray<
  RecordField<BlueprintCourseSummaryView, BlueprintRecordSortKey, BlueprintCourseFacetField>
> {
  const vocabulary = blueprintVocabularyCell(client);
  return [
    {
      id: "name",
      label: "Blueprint Course",
      role: "identity",
      sortKey: "name",
      value: (course) => course.long_name,
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "discipline",
      label: "Discipline",
      filterKey: "disciplineUuids",
      value: (course) => course.classification.disciplineUuid,
      render: (course) => vocabulary("discipline", null, course.classification.disciplineUuid),
      defaultColumn: { priority: 1, width: "10rem" },
    },
    {
      id: "subject",
      label: "Subject",
      filterKey: "subjectUuids",
      value: (course) => course.classification.subjectUuid,
      render: (course) =>
        vocabulary(
          "subject",
          course.classification.disciplineUuid,
          course.classification.subjectUuid,
        ),
      defaultColumn: { priority: 2, width: "10rem" },
    },
    {
      id: "owner",
      label: "Author",
      value: (course) => course.owner_display_name,
      defaultColumn: { priority: 3, width: "11rem" },
    },
    {
      id: "institution",
      label: "Institution",
      value: (course) => course.owner_affiliation,
      defaultColumn: { priority: 4, width: "12rem", visible: scope === "active" },
    },
    {
      id: "adoptions",
      label: "Adoptions",
      sortKey: "adoptions",
      value: (course) => course.total_adoptions,
      defaultColumn: { priority: 5, width: "7rem", visible: scope === "active" },
    },
    {
      id: "students",
      label: "Students",
      sortKey: "students",
      value: (course) => course.total_students_ever_enrolled,
      defaultColumn: { priority: 6, width: "7rem", visible: scope === "active" },
    },
    {
      id: "stars",
      label: "Stars",
      sortKey: "stars",
      value: (course) => course.star_count,
      defaultColumn: { priority: 6, width: "6rem", visible: false },
    },
    {
      id: "watches",
      label: "Watches",
      sortKey: "watches",
      value: (course) => course.watcher_count,
      defaultColumn: { priority: 7, width: "7rem", visible: false },
    },
    {
      id: "recent_edits",
      label: "Last edited",
      sortKey: "recentEdits",
      value: (course) => course.last_edited_at_millis,
      render: (course) => formatTimestamp()(course.last_edited_at_millis),
      defaultColumn: { priority: 7, width: "9rem", visible: scope === "all" },
    },
    {
      id: "revision",
      label: "Revision",
      value: (course) => course.current_revision_tuple.revisionNumber,
      defaultColumn: { priority: 4, width: "6rem", visible: scope === "all" },
    },
    {
      id: "availability",
      label: "Availability",
      value: (course) => (course.availability === "archived" ? "Archived" : "Active"),
      defaultColumn: { priority: 8, width: "8rem", visible: scope === "all" },
    },
    {
      id: "topic",
      label: "Topic",
      filterKey: "topicUuids",
      value: (course) => course.classification.topicUuid,
      render: (course) =>
        vocabulary("topic", course.classification.subjectUuid, course.classification.topicUuid),
      defaultColumn: { priority: 9, width: "9rem", visible: false },
    },
    {
      id: "subtopic",
      label: "Subtopic",
      filterKey: "subtopicUuids",
      value: (course) => course.classification.subtopicUuid,
      render: (course) =>
        vocabulary("subtopic", course.classification.topicUuid, course.classification.subtopicUuid),
      defaultColumn: { priority: 10, width: "9rem", visible: false },
    },
    {
      id: "tags",
      label: "Tags",
      filterKey: "tags",
      value: (course) => course.classification.tags,
      defaultColumn: { priority: 11, width: "9rem", visible: scope === "active" },
    },
  ];
}

/** The endpoint applies every order/filter to the whole collection before paging. */
export function blueprintCourseSearchRecordQuery(props: {
  readonly query: () => BlueprintCourseSearchQuery;
  readonly apply: (
    query: (current: BlueprintCourseSearchQuery) => BlueprintCourseSearchQuery,
  ) => Promise<void>;
  readonly sort: (
    query: (current: BlueprintCourseSearchQuery) => BlueprintCourseSearchQuery,
  ) => Promise<void>;
  readonly client: Pick<BlueprintCourseClient, "listBlueprintCourseFacetChoices">;
  readonly scope: "active" | "all";
}): DataSheetQuery<BlueprintRecordSortKey, BlueprintCourseFacetField> {
  return {
    sort: {
      get value(): RecordSort<BlueprintRecordSortKey> | null {
        return orders.find((entry) => entry.order === props.query().sort)?.sort ?? null;
      },
      onChange: (sort): void => {
        const order = orders.find(
          (entry) => entry.sort.key === sort.key && entry.sort.direction === sort.direction,
        )?.order;
        if (order !== undefined && order !== props.query().sort)
          void props.sort((current) => ({ ...current, sort: order }));
      },
    },
    filters: {
      get values(): ReadonlyMap<BlueprintCourseFacetField, ReadonlyArray<string>> {
        return new Map<BlueprintCourseFacetField, ReadonlyArray<string>>(
          (Object.keys(props.query().facets) as BlueprintCourseFacetField[]).map((field) => [
            field,
            props.query().facets[field],
          ]),
        );
      },
      onChange: (field, values): void => {
        void props.apply((current) => ({
          ...current,
          facets: { ...current.facets, [field]: [...values] },
        }));
      },
      onClear: (): void => {
        void props.apply((current) => ({
          ...current,
          facets: {
            disciplineUuids: [],
            subjectUuids: [],
            topicUuids: [],
            subtopicUuids: [],
            tags: [],
          },
        }));
      },
      choices: async (field, search, cursor, signal): Promise<RecordFilterChoices> => {
        const current = props.query();
        const choices = await props.client.listBlueprintCourseFacetChoices(
          field,
          {
            includeArchived: props.scope === "all" && current.includeArchived,
            activeOnly: props.scope === "active",
            promotedOnly: current.promotedOnly,
            query: current.query,
            classification: current.classification,
            sort: current.sort,
            tag: "",
            facets: current.facets,
          },
          search,
          cursor,
        );
        return signal.aborted ? { choices: [], selected: [], nextCursor: null } : choices;
      },
    },
  };
}
