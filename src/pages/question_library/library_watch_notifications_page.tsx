// Private, searchable in-app Watch activity for the signed-in Instructor.

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import type {
  LibraryWatchEventKind,
  LibraryWatchNotification,
  LibraryWatchPage,
  LibraryWatchQuery,
  LibraryWatchTargetKind,
} from "../../api/library_watch_notification";
import type { SearchPageRequest } from "../../components/search_block/search_definition";
import { useApplicationApi } from "../../api/application_api";
import { useSelectedDisplayDateTimeFormatter } from "../../selected_display_zone";
import { PageFrame } from "../../components/page_frame/page_frame";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type {
  RecordField,
  RecordFilterChoices,
  DataSheetQuery,
} from "../../components/data_sheet/data_sheet_fields";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import type { SearchDefinition, SearchPage } from "../../components/search_block/search_definition";

const INITIAL_QUERY: LibraryWatchQuery = {
  q: "",
  targetKinds: [],
  eventKinds: [],
  direction: "descending",
};

type WatchFilter = "targetKind" | "eventKind";
type WatchSort = "occurredAt";

function eventLabel(value: LibraryWatchNotification): string {
  switch (value.eventKind) {
    case "revision":
      return "New Revision";
    case "membersChanged":
      return "Questions changed";
    case "fork":
      return "New fork";
  }
}

function targetLabel(value: LibraryWatchNotification): string {
  return value.targetKind === "question" ? "Published Question" : "Question Pool";
}

function targetHref(value: LibraryWatchNotification): string {
  return `/library/${encodeURIComponent(value.targetPublicId)}`;
}

export function notificationContent(
  formatTimestamp: (timestamp: number | Date) => string,
): (notification: LibraryWatchNotification) => RecordContent {
  return (notification) => ({
    title: eventLabel(notification),
    description: `${targetLabel(notification)}: ${notification.targetPublicId}`,
    details: [
      {
        kind: "text",
        label:
          notification.targetKind === "questionPool"
            ? "Question Pool Edit Number"
            : "Question Revision Number",
        value: String(
          notification.targetKind === "questionPool"
            ? notification.questionPoolEditNumber
            : notification.questionRevisionNumber,
        ),
      },
      ...(notification.forkedPublicId === null
        ? []
        : [{ kind: "text" as const, label: "Fork ID", value: notification.forkedPublicId }]),
      {
        kind: "time",
        label: "Occurred",
        value: formatTimestamp(notification.occurredAt),
        dateTime: new Date(notification.occurredAt).toISOString(),
      },
    ],
    actions: [
      {
        id: "open-target",
        kind: "link",
        label: notification.targetKind === "question" ? "Open Question" : "Open Question Pool",
        href: targetHref(notification),
        primary: true,
      },
    ],
  });
}

function watchFields(
  formatTimestamp: () => (timestamp: number | Date) => string,
): ReadonlyArray<RecordField<LibraryWatchNotification, WatchSort, WatchFilter>> {
  return [
    {
      id: "activity",
      label: "Activity",
      role: "identity",
      filterKey: "eventKind",
      value: eventLabel,
      defaultColumn: { priority: 0, width: "12rem" },
    },
    {
      id: "target",
      label: "Library ID",
      value: (row) => row.targetPublicId,
      defaultColumn: { priority: 1, width: "10rem" },
    },
    {
      id: "targetKind",
      label: "Library object",
      filterKey: "targetKind",
      value: targetLabel,
      defaultColumn: { priority: 2, width: "12rem" },
    },
    {
      id: "occurredAt",
      label: "Occurred",
      sortKey: "occurredAt",
      value: (row) => row.occurredAt,
      render: (row) => (
        <time dateTime={new Date(row.occurredAt).toISOString()}>
          {formatTimestamp()(row.occurredAt)}
        </time>
      ),
      defaultColumn: { priority: 3, width: "12rem" },
    },
    {
      id: "forkedPublicId",
      label: "Fork ID",
      value: (row) => row.forkedPublicId,
      defaultColumn: { priority: 4, width: "10rem", visible: false },
    },
  ];
}

function cleanQuery(query: LibraryWatchQuery): LibraryWatchQuery {
  return {
    q: query.q.trim().toLowerCase().slice(0, 120),
    targetKinds: [...new Set(query.targetKinds)].sort(),
    eventKinds: [...new Set(query.eventKinds)].sort(),
    direction: query.direction,
  };
}

function watchDefinition(
  load: (query: LibraryWatchQuery, page: SearchPageRequest) => Promise<LibraryWatchPage>,
  content: (row: LibraryWatchNotification) => RecordContent,
): SearchDefinition<LibraryWatchQuery, LibraryWatchNotification, LibraryWatchPage> {
  return {
    initialQuery: INITIAL_QUERY,
    maximumPageSize: 100,
    cleanup: cleanQuery,
    isEligible: () => true,
    getText: (query) => query.q,
    setText: (query, q) => ({ ...query, q }),
    rowId: (row) => row.eventId,
    content,
    fetchPage: async (
      query,
      request,
    ): Promise<SearchPage<LibraryWatchNotification, LibraryWatchPage>> => {
      const page = await load(query, request);
      return {
        items: page.items,
        pageNumber: page.pageNumber,
        pageSize: page.pageSize as SearchPageRequest["pageSize"],
        totalCount: page.totalCount,
        filterCounts: page,
      };
    },
    appliedFilters: (query) => [
      ...query.targetKinds.map((value) => ({
        id: `target-${value}`,
        label: `Library object: ${value === "question" ? "Published Question" : "Question Pool"}`,
        clear: (current: LibraryWatchQuery): LibraryWatchQuery => ({
          ...current,
          targetKinds: current.targetKinds.filter((kind) => kind !== value),
        }),
      })),
      ...query.eventKinds.map((value) => ({
        id: `event-${value}`,
        label: `Event: ${value === "membersChanged" ? "Questions changed" : value}`,
        clear: (current: LibraryWatchQuery): LibraryWatchQuery => ({
          ...current,
          eventKinds: current.eventKinds.filter((kind) => kind !== value),
        }),
      })),
    ],
  };
}

function facetChoices(
  field: WatchFilter,
  page: LibraryWatchPage,
  selected: ReadonlyArray<string>,
  search: string,
): RecordFilterChoices {
  const labels: Record<string, string> = {
    question: "Published Question",
    questionPool: "Question Pool",
    revision: "New Revision",
    membersChanged: "Questions changed",
    fork: "New fork",
  };
  const counts = field === "targetKind" ? page.targetKindFacets : page.eventKindFacets;
  const values =
    field === "targetKind" ? ["question", "questionPool"] : ["revision", "membersChanged", "fork"];
  const choices = values.map((key) => ({
    key,
    label: labels[key] ?? key,
    count: counts.find((count) => count.value === key)?.count ?? 0,
  }));
  const term = search.trim().toLowerCase();
  return {
    choices: choices.filter((choice) => choice.label.toLowerCase().includes(term)),
    selected: choices.filter((choice) => selected.includes(choice.key)),
    nextCursor: null,
  };
}

/** Shows this active Instructor's entire snapshot-authorized Watch history. */
export function LibraryWatchNotificationsPage(): JSX.Element {
  const client = useApplicationApi().client;
  const formatTimestamp = useSelectedDisplayDateTimeFormatter();
  const load = (query: LibraryWatchQuery, page: SearchPageRequest): Promise<LibraryWatchPage> =>
    client.getLibraryWatchNotifications(query, page);
  const state = createSearchState(
    watchDefinition(load, (row) => notificationContent(formatTimestamp())(row)),
    {
      runOnMount: true,
    },
  );
  const recordQuery: DataSheetQuery<WatchSort, WatchFilter> = {
    sort: {
      get value() {
        return { key: "occurredAt" as const, direction: state.query().direction };
      },
      onChange: (sort) => void state.sort((current) => ({ ...current, direction: sort.direction })),
    },
    filters: {
      get values() {
        return new Map<WatchFilter, ReadonlyArray<string>>([
          ["targetKind", state.query().targetKinds],
          ["eventKind", state.query().eventKinds],
        ]);
      },
      onChange: (key, values) =>
        void state.apply((current) =>
          key === "targetKind"
            ? { ...current, targetKinds: values as ReadonlyArray<LibraryWatchTargetKind> }
            : { ...current, eventKinds: values as ReadonlyArray<LibraryWatchEventKind> },
        ),
      onClear: () =>
        void state.apply((current) => ({ ...current, targetKinds: [], eventKinds: [] })),
      choices: async (key, search) => {
        const query = state.query();
        const withoutOwn =
          key === "targetKind" ? { ...query, targetKinds: [] } : { ...query, eventKinds: [] };
        const page = await load(withoutOwn, { pageNumber: 1, pageSize: 50 });
        return facetChoices(
          key,
          page,
          key === "targetKind" ? query.targetKinds : query.eventKinds,
          search,
        );
      },
    },
  };

  return (
    <PageFrame routeSurface="libraryWatchNotifications">
      <SearchBlock
        layout="above"
        state={state}
        controls={
          <SearchControls
            state={state}
            textLabel="Search Watch activity"
            textPlaceholder="Library ID or event"
          />
        }
        collection={{
          ariaLabel: "Private Watch activity",
          emptyState: {
            title: "No matching Watch activity",
            message: "Try another Library ID, event, or filter.",
          },
          fields: watchFields(formatTimestamp),
          query: recordQuery,
        }}
      />
      <Show
        when={
          state.state().kind === "empty" &&
          state.query().q.length === 0 &&
          state.query().targetKinds.length === 0 &&
          state.query().eventKinds.length === 0
        }
      >
        <p>
          <A class="primary-link" href="/library">
            Open Question Library
          </A>
        </p>
      </Show>
    </PageFrame>
  );
}
