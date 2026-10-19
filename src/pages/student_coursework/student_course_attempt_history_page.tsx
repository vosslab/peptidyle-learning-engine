// One global, self-only Attempt History query across enrolled Courses.

import { useNavigate } from "@solidjs/router";
import { createMemo, createResource, type JSX } from "solid-js";

import type {
  StudentAttemptHistoryEntry,
  StudentAttemptHistoryFacetField,
  StudentAttemptHistoryQuery,
  StudentAttemptHistorySort,
} from "../../api/student_course_attempt_history";
import { useApplicationApi } from "../../api/application_api";
import type {
  RecordField,
  RecordFilterChoices,
  DataSheetQuery,
  RecordSort,
} from "../../components/data_sheet/data_sheet_fields";
import type { RecordContent } from "../../components/data_sheet/record_content";
import { PageFrame } from "../../components/page_frame/page_frame";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import type {
  SearchDefinition,
  SearchPageRequest,
} from "../../components/search_block/search_definition";
import { createDisplayDateTimeFormatter } from "../../format_datetime";
import { studentCrossCourseLabel } from "./student_collection_fields";
import {
  assessmentAttemptPath,
  assessmentAttemptRouteState,
} from "../../navigation/assessment_attempt_route";
import { assessmentAttemptRouteId, parseCourseInstanceId } from "../../navigation/public_route";

type AttemptSortField = "title" | "course" | "started";

const ORDERS: ReadonlyArray<{
  readonly value: StudentAttemptHistorySort;
  readonly sort: RecordSort<AttemptSortField>;
  readonly label: string;
}> = [
  {
    value: "startedNewest",
    sort: { key: "started", direction: "descending" },
    label: "Newest first",
  },
  {
    value: "startedOldest",
    sort: { key: "started", direction: "ascending" },
    label: "Oldest first",
  },
  {
    value: "titleAscending",
    sort: { key: "title", direction: "ascending" },
    label: "Title A to Z",
  },
  {
    value: "titleDescending",
    sort: { key: "title", direction: "descending" },
    label: "Title Z to A",
  },
  {
    value: "courseAscending",
    sort: { key: "course", direction: "ascending" },
    label: "Course A to Z",
  },
  {
    value: "courseDescending",
    sort: { key: "course", direction: "descending" },
    label: "Course Z to A",
  },
];

const INITIAL_QUERY: StudentAttemptHistoryQuery = {
  search: "",
  state: "all",
  courseIds: [],
  sort: "startedNewest",
};

function attemptFields(
  formatDateTime: () => ReturnType<typeof createDisplayDateTimeFormatter> | undefined,
): ReadonlyArray<
  RecordField<StudentAttemptHistoryEntry, AttemptSortField, StudentAttemptHistoryFacetField>
> {
  return [
    {
      id: "title",
      label: "Coursework",
      role: "identity",
      sortKey: "title",
      value: (attempt) => attempt.assessmentTitle,
      defaultColumn: { priority: 0, width: "20rem" },
    },
    {
      id: "course",
      label: "Course",
      sortKey: "course",
      filterKey: "course",
      value: (attempt) => attempt.courseInstanceId,
      render: (attempt) =>
        studentCrossCourseLabel({
          id: attempt.courseInstanceId,
          shortName: attempt.courseShortName,
          longName: attempt.courseLongName,
        }),
      defaultColumn: { priority: 1, width: "14rem" },
    },
    {
      id: "state",
      label: "Status",
      filterKey: "state",
      value: (attempt) => (attempt.submittedAt === undefined ? "inProgress" : "submitted"),
      render: (attempt) => (attempt.submittedAt === undefined ? "In progress" : "Submitted"),
      defaultColumn: { priority: 2, width: "10rem" },
    },
    {
      id: "started",
      label: "Started",
      sortKey: "started",
      value: (attempt) => attempt.startedAt,
      render: (attempt) => formatDateTime()?.(attempt.startedAt) ?? "Time unavailable",
      defaultColumn: { priority: 3, width: "12rem" },
    },
  ];
}

function scoreLabel(attempt: StudentAttemptHistoryEntry): string {
  if (attempt.submittedAt === undefined) return "No score yet";
  const score = attempt.assessmentScore;
  if (score === undefined) return "Score pending";
  const format = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });
  return `Score: ${format.format(score.pointsEarned)} of ${format.format(score.pointsPossible)} points`;
}

/** The Student's complete authorized history uses one server-owned result order. */
export function StudentAttemptHistoryPage(): JSX.Element {
  const api = useApplicationApi();
  const navigate = useNavigate();
  const [accountSettings] = createResource(() => api.client.getAccountSettings());
  const formatDateTime = createMemo(() => {
    const timeZone = accountSettings()?.timeZone;
    return timeZone === undefined ? undefined : createDisplayDateTimeFormatter(timeZone);
  });
  const courseNames = new Map<string, string>();

  function content(attempt: StudentAttemptHistoryEntry): RecordContent {
    const format = formatDateTime();
    const route = attempt.submittedAt === undefined ? "attempt" : "review";
    const details: Array<RecordContent["details"][number]> = [
      {
        kind: "text",
        label: "Course",
        value: studentCrossCourseLabel({
          id: attempt.courseInstanceId,
          shortName: attempt.courseShortName,
          longName: attempt.courseLongName,
        }),
      },
      {
        kind: "text",
        value: `Attempt ${attempt.assessmentAttemptNumber} · ${route === "attempt" ? "In progress" : "Submitted"}`,
      },
      {
        kind: "time",
        value: `Started ${format?.(attempt.startedAt) ?? "Time unavailable"}`,
        dateTime: new Date(attempt.startedAt).toISOString(),
      },
    ];
    if (attempt.submittedAt !== undefined) {
      details.push({
        kind: "time",
        value: `Submitted ${format?.(attempt.submittedAt) ?? "Time unavailable"}`,
        dateTime: new Date(attempt.submittedAt).toISOString(),
      });
    }
    details.push({ kind: "text", value: scoreLabel(attempt) });
    return {
      title: attempt.assessmentTitle,
      details,
      actions: [
        {
          id: "open-attempt",
          kind: "command",
          label: route === "attempt" ? "Open Attempt" : "Review Attempt",
          onClick: () =>
            navigate(assessmentAttemptPath(route, attempt.courseInstanceId), {
              state: assessmentAttemptRouteState(
                assessmentAttemptRouteId(attempt.assessmentAttemptId),
              ),
            }),
        },
      ],
    };
  }

  const definition: SearchDefinition<StudentAttemptHistoryQuery, StudentAttemptHistoryEntry, null> =
    {
      initialQuery: INITIAL_QUERY,
      maximumPageSize: 100,
      cleanup: (query) => ({
        ...query,
        search: query.search.trim().slice(0, 200),
        courseIds: [...new Set(query.courseIds)].sort(),
      }),
      isEligible: () => true,
      getText: (query) => query.search,
      setText: (query, search) => ({ ...query, search }),
      rowId: (attempt) => attempt.assessmentAttemptId,
      content,
      fetchPage: async (query, requestPage: SearchPageRequest) => {
        const page = await api.client.listStudentAttemptHistory(
          query,
          requestPage.pageNumber,
          requestPage.pageSize,
        );
        if (page.pageSize !== requestPage.pageSize) {
          throw new Error("Attempt History returned a different page size than requested");
        }
        return {
          items: page.items,
          pageNumber: page.pageNumber,
          pageSize: requestPage.pageSize,
          totalCount: page.totalCount,
          filterCounts: null,
        };
      },
    };
  const state = createSearchState(definition, { runOnMount: true });

  const recordQuery: DataSheetQuery<AttemptSortField, StudentAttemptHistoryFacetField> = {
    sort: {
      get value() {
        return ORDERS.find(({ value }) => value === state.query().sort)?.sort ?? null;
      },
      onChange: (sort) => {
        const order = ORDERS.find(
          ({ sort: candidate }) =>
            candidate.key === sort.key && candidate.direction === sort.direction,
        );
        if (order !== undefined) {
          void state.sort((query) => ({ ...query, sort: order.value }));
        }
      },
    },
    filters: {
      get values() {
        const query = state.query();
        return new Map<StudentAttemptHistoryFacetField, ReadonlyArray<string>>([
          ["course", query.courseIds],
          ["state", query.state === "all" ? [] : [query.state]],
        ]);
      },
      onChange: (field, values) => {
        if (field === "course") {
          const courseIds = values
            .map(parseCourseInstanceId)
            .filter((course): course is NonNullable<typeof course> => course !== null);
          void state.apply((query) => ({ ...query, courseIds }));
          return;
        }
        const selected = values.filter((value) => value === "submitted" || value === "inProgress");
        const next = selected.length === 1 ? selected[0] : "all";
        void state.apply((query) => ({
          ...query,
          state: next === "submitted" || next === "inProgress" ? next : "all",
        }));
      },
      onClear: () => void state.apply((query) => ({ ...query, state: "all", courseIds: [] })),
      choices: async (field, search, cursor, signal): Promise<RecordFilterChoices> => {
        const page = await api.client.listStudentAttemptHistoryFacetChoices(
          state.query(),
          field,
          search,
          cursor ?? undefined,
        );
        if (signal.aborted) throw new DOMException("Facet request canceled", "AbortError");
        if (field === "course") {
          for (const choice of page.choices) courseNames.set(choice.key, choice.label);
        }
        const selectedKeys =
          field === "course"
            ? state.query().courseIds
            : state.query().state === "all"
              ? []
              : [state.query().state];
        const selected = selectedKeys.map(
          (key) =>
            page.choices.find((choice) => choice.key === key) ?? {
              key,
              label: field === "course" ? (courseNames.get(key) ?? key) : key,
              count: 0,
            },
        );
        return { choices: page.choices, selected, nextCursor: page.nextCursor ?? null };
      },
    },
  };

  return (
    <PageFrame routeSurface="studentAttemptHistory">
      <SearchBlock
        layout="above"
        state={state}
        controls={<SearchControls state={state} textLabel="Search Attempt History" />}
        collection={{
          ariaLabel: "Attempt History",
          fields: attemptFields(formatDateTime),
          query: recordQuery,
          emptyState: {
            title: "No Attempts match your search.",
            message: "Started Coursework appears here for your enrolled Courses.",
          },
        }}
      />
    </PageFrame>
  );
}
