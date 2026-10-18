// Instructor-only lineage endorsements and self-only Watch activity.
import { Show, createEffect, createResource, createSignal, type JSX } from "solid-js";
import type {
  BlueprintWatchEvent,
  BlueprintWatchEventKind,
  BlueprintWatchPage,
  BlueprintWatchQuery,
  BlueprintStewardshipClient,
} from "../../api/blueprint_stewardship";
import "./blueprint_stewardship.css";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type {
  RecordField,
  RecordFilterChoices,
  DataSheetQuery,
} from "../../components/data_sheet/data_sheet_fields";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { createSearchState } from "../../components/search_block/search_state";
import type {
  SearchPageRequest,
  SearchDefinition,
  SearchPage,
} from "../../components/search_block/search_definition";

type StarredInstructorRow = { readonly id: string; readonly displayName: string };
const instructorFields: ReadonlyArray<RecordField<StarredInstructorRow, "name", "name">> = [
  {
    id: "name",
    label: "Instructor",
    role: "identity",
    sortKey: "name",
    filterKey: "name",
    value: (row) => row.displayName,
    defaultColumn: { priority: 0, width: "18rem" },
  },
];

interface Props {
  readonly client: BlueprintStewardshipClient;
  readonly blueprintCourseId: string;
  readonly formatDateTime: (timestamp: number | Date) => string;
}

function eventLabel(kind: BlueprintWatchEvent["kind"]): string {
  switch (kind) {
    case "revision":
      return "New Blueprint Revision";
    case "archived":
      return "Blueprint archived";
    case "restored":
      return "Blueprint restored";
  }
}

const INITIAL_WATCH_QUERY: BlueprintWatchQuery = {
  q: "",
  eventKinds: [],
  direction: "descending",
};

type WatchSort = "occurredAt";
type WatchFilter = "eventKind";

function watchFields(
  formatDateTime: Props["formatDateTime"],
): ReadonlyArray<RecordField<BlueprintWatchEvent, WatchSort, WatchFilter>> {
  return [
    {
      id: "activity",
      label: "Activity",
      role: "identity",
      filterKey: "eventKind",
      value: (row) => eventLabel(row.kind),
      defaultColumn: { priority: 0, width: "16rem" },
    },
    {
      id: "occurredAt",
      label: "Occurred",
      sortKey: "occurredAt",
      value: (row) => row.occurredAt,
      render: (row) => (
        <time dateTime={new Date(row.occurredAt).toISOString()}>
          {formatDateTime(row.occurredAt)}
        </time>
      ),
      defaultColumn: { priority: 1, width: "15rem" },
    },
  ];
}

function cleanWatchQuery(query: BlueprintWatchQuery): BlueprintWatchQuery {
  return {
    q: query.q.trim().toLowerCase().slice(0, 120),
    eventKinds: [...new Set(query.eventKinds)].sort(),
    direction: query.direction,
  };
}

function watchDefinition(
  load: (query: BlueprintWatchQuery, page: SearchPageRequest) => Promise<BlueprintWatchPage>,
  content: (row: BlueprintWatchEvent) => RecordContent,
): SearchDefinition<BlueprintWatchQuery, BlueprintWatchEvent, BlueprintWatchPage> {
  return {
    initialQuery: INITIAL_WATCH_QUERY,
    maximumPageSize: 100,
    cleanup: cleanWatchQuery,
    isEligible: () => true,
    getText: (query) => query.q,
    setText: (query, q) => ({ ...query, q }),
    rowId: (row) => row.eventId,
    content,
    fetchPage: async (
      query,
      requestedPage,
    ): Promise<SearchPage<BlueprintWatchEvent, BlueprintWatchPage>> => {
      const page = await load(query, requestedPage);
      if (page.pageSize !== requestedPage.pageSize)
        throw new Error("Blueprint Watch response page size differs from its request");
      return { ...page, pageSize: requestedPage.pageSize, filterCounts: page };
    },
    appliedFilters: (query) =>
      query.eventKinds.map((kind) => ({
        id: `event-${kind}`,
        label: `Activity: ${eventLabel(kind)}`,
        clear: (current: BlueprintWatchQuery): BlueprintWatchQuery => ({
          ...current,
          eventKinds: current.eventKinds.filter((value) => value !== kind),
        }),
      })),
  };
}

function watchFacetChoices(
  page: BlueprintWatchPage | undefined,
  selected: ReadonlyArray<BlueprintWatchEventKind>,
  search: string,
): RecordFilterChoices {
  const values: ReadonlyArray<BlueprintWatchEventKind> = ["revision", "archived", "restored"];
  const choices = values.map((value) => ({
    key: value,
    label: eventLabel(value),
    count: page?.eventKindFacets.find((facet) => facet.value === value)?.count ?? 0,
  }));
  const term = search.trim().toLowerCase();
  return {
    choices: choices.filter((choice) => choice.label.toLowerCase().includes(term)),
    selected: choices.filter((choice) => selected.includes(choice.key)),
    nextCursor: null,
  };
}

function watchEventContent(
  formatDateTime: Props["formatDateTime"],
  event: BlueprintWatchEvent,
): RecordContent {
  return {
    title: eventLabel(event.kind),
    details: [
      {
        kind: "time",
        label: "Occurred",
        value: formatDateTime(event.occurredAt),
        dateTime: new Date(event.occurredAt).toISOString(),
      },
    ],
    actions: [],
  };
}

/** The surrounding Instructor detail route mounts this for an accessible Blueprint. */
export function BlueprintStewardship(props: Props): JSX.Element {
  const [busy, setBusy] = createSignal(false);
  const [notice, setNotice] = createSignal("");
  const [failed, setFailed] = createSignal(false);
  const [state, { refetch }] = createResource(
    () => props.blueprintCourseId,
    async (blueprintCourseId) => {
      const [star, watch] = await Promise.all([
        props.client.getBlueprintStar(blueprintCourseId),
        props.client.getBlueprintWatch(blueprintCourseId),
      ]);
      return { star, watch };
    },
  );
  const [instructors, { refetch: refetchInstructors }] = createResource(
    () => props.blueprintCourseId,
    (blueprintCourseId) => props.client.getBlueprintStarredInstructors(blueprintCourseId),
  );
  const [activityRefreshRevision, setActivityRefreshRevision] = createSignal(0);
  const instructorSearch = createLocalCompleteSearch(
    {
      rows: () => {
        const names = instructors();
        if (names === undefined) return undefined;
        const occurrences = new Map<string, number>();
        return names.map(({ displayName }) => {
          const occurrence = occurrences.get(displayName) ?? 0;
          occurrences.set(displayName, occurrence + 1);
          // The restricted projection identifies names, not individual Accounts.
          return { id: JSON.stringify([displayName, occurrence]), displayName };
        });
      },
      loading: () => instructors.loading,
      error: (): unknown => instructors.error,
      refetchSource: refetchInstructors,
    },
    {
      fields: instructorFields,
      rowId: (row) => row.id,
      keyword: (row) => row.displayName,
      content: (row) => ({ title: row.displayName, details: [], actions: [] }),
    },
  );

  async function toggle(kind: "star" | "watch"): Promise<void> {
    const current = state();
    if (current === undefined || busy() || state.loading || state.error !== undefined) return;
    setBusy(true);
    setNotice("");
    setFailed(false);
    try {
      if (kind === "star")
        await props.client.setBlueprintStar(
          props.blueprintCourseId,
          !current.star.viewerHasStarred,
        );
      else await props.client.setBlueprintWatch(props.blueprintCourseId, !current.watch.watching);
      // Refresh the preference that owns the control before confirming the action.
      await Promise.all([refetch(), ...(kind === "star" ? [refetchInstructors()] : [])]);
      if (kind === "star" && (state.error !== undefined || instructors.error !== undefined))
        throw new Error("Blueprint Star stewardship could not be refreshed");
      setNotice(
        kind === "star"
          ? "Your Star preference is saved."
          : "Your private Watch preference is saved.",
      );
    } catch {
      setFailed(true);
      setNotice("Your preference could not be confirmed. Reload stewardship before trying again.");
    } finally {
      setBusy(false);
    }
  }

  async function reload(): Promise<void> {
    setFailed(false);
    setNotice("");
    const results = await Promise.allSettled([refetch(), refetchInstructors()]);
    setActivityRefreshRevision((revision) => revision + 1);
    if (results[0]?.status === "rejected") setFailed(true);
  }

  return (
    <section
      class="blueprint-stewardship"
      aria-label="Blueprint stewardship"
      aria-busy={busy() || state.loading}
    >
      <h2>Stars and Watch activity</h2>
      <p>A Star is a visible endorsement. Your Watch preference and activity are private.</p>
      <Show when={state.error !== undefined || failed()}>
        <p role="alert">
          Blueprint stewardship could not be confirmed. Reload to see its current state.
        </p>
      </Show>
      <Show when={state.loading}>
        <p role="status">Loading Blueprint stewardship...</p>
      </Show>
      <Show when={state.error === undefined && state()}>
        {(current) => (
          <>
            <div class="blueprint-stewardship__actions">
              <button
                type="button"
                class="quiet-action"
                aria-pressed={current().star.viewerHasStarred}
                disabled={busy() || state.loading}
                onClick={() => void toggle("star")}
              >
                {current().star.viewerHasStarred ? "Starred" : "Star"}
              </button>
              <span>
                {current().star.starCount} {current().star.starCount === 1 ? "Star" : "Stars"}
              </span>
              <button
                type="button"
                class="quiet-action"
                aria-pressed={current().watch.watching}
                disabled={busy() || state.loading}
                onClick={() => void toggle("watch")}
              >
                {current().watch.watching ? "Watching" : "Watch"}
              </button>
            </div>
            <details>
              <summary>Instructors who Starred this Blueprint</summary>
              {/* ASVS 1.2.1, 8.2.3: exact server-projected names rendered as text, without identity links. */}
              <SearchBlock
                layout="above"
                controls={
                  <SearchControls state={instructorSearch.state} textLabel="Instructor name" />
                }
                state={instructorSearch.state}
                collection={{
                  ariaLabel: "Instructors who starred this Blueprint",
                  ...instructorSearch.collection,
                  emptyState: { title: "No Instructor Stars to show." },
                }}
              />
            </details>
            <details>
              <summary>Your private Watch activity</summary>
              <Show when={props.blueprintCourseId} keyed>
                {(blueprintCourseId) => (
                  <WatchActivity
                    client={props.client}
                    blueprintCourseId={blueprintCourseId}
                    formatDateTime={props.formatDateTime}
                    refreshRevision={activityRefreshRevision()}
                  />
                )}
              </Show>
            </details>
          </>
        )}
      </Show>
      <button
        type="button"
        class="quiet-action"
        disabled={busy() || state.loading}
        onClick={() => void reload()}
      >
        Reload stewardship
      </button>
      <Show when={notice()}>
        <p role={failed() ? "alert" : "status"}>{notice()}</p>
      </Show>
    </section>
  );
}

function WatchActivity(props: Props & { readonly refreshRevision: number }): JSX.Element {
  const load = (query: BlueprintWatchQuery, page: SearchPageRequest): Promise<BlueprintWatchPage> =>
    props.client.getBlueprintWatchEvents(
      props.blueprintCourseId,
      query,
      page.pageNumber,
      page.pageSize,
    );
  const state = createSearchState(
    watchDefinition(load, (row) => watchEventContent(props.formatDateTime, row)),
    { runOnMount: true },
  );
  let previousRefreshRevision = props.refreshRevision;
  createEffect(() => {
    const revision = props.refreshRevision;
    if (revision === previousRefreshRevision) return;
    previousRefreshRevision = revision;
    void state.refresh();
  });
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
          ["eventKind", state.query().eventKinds],
        ]);
      },
      onChange: (_key, values) =>
        void state.apply((current) => ({
          ...current,
          eventKinds: values.filter(
            (value): value is BlueprintWatchEventKind =>
              value === "revision" || value === "archived" || value === "restored",
          ),
        })),
      onClear: () => void state.apply((current) => ({ ...current, eventKinds: [] })),
      choices: async (_key, search) => {
        const query = state.query();
        const page = await load({ ...query, eventKinds: [] }, { pageNumber: 1, pageSize: 50 });
        return watchFacetChoices(page, query.eventKinds, search);
      },
    },
  };
  return (
    <SearchBlock
      layout="above"
      state={state}
      controls={
        <SearchControls
          state={state}
          textLabel="Search Watch activity"
          textPlaceholder="Activity"
        />
      }
      collection={{
        ariaLabel: "Your private Watch activity",
        emptyState: {
          title: "No matching Watch activity",
          message: "Try another activity or filter.",
        },
        fields: watchFields(props.formatDateTime),
        query: recordQuery,
      }}
    />
  );
}
