// Lazy, read-only inspection of exact saved Blueprint content and recorded metadata.
import { Show, createMemo, createSignal, onCleanup, type JSX } from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintHistoryEntryView } from "../../../generated/api/BlueprintHistoryEntryView";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type {
  BlueprintCourseClient,
  BlueprintHistoryPage,
  BlueprintHistoryQuery,
} from "../../api/blueprint_course";
import { ApiRequestError } from "../../api/http_client";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type {
  RecordField,
  RecordFilterChoices,
  DataSheetQuery,
} from "../../components/data_sheet/data_sheet_fields";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import type { SearchDefinition } from "../../components/search_block/search_definition";
import { BlueprintHistoryContent } from "./blueprint_history_content";

interface HistoryProps {
  readonly client: BlueprintCourseClient;
  readonly view: BlueprintCourseView;
  readonly formatDateTime: (timestamp: number | Date) => string;
}

function historyError(error: unknown): string {
  // ASVS 16.5.1: unexpected failures never expose server details or raw error messages.
  if (error instanceof ApiRequestError) {
    if (error.status === 401)
      return "Your session ended. Sign in again to inspect Blueprint history.";
    if (error.status === 403 || error.status === 404)
      return "Blueprint history is unavailable for your Account.";
  }
  return "Blueprint history could not load. Try again.";
}

function historyEntryId(entry: BlueprintHistoryEntryView): string {
  return entry.kind === "savedRevision"
    ? `revision-${entry.revisionNumber}`
    : `metadata-${entry.blueprintEditNumber}`;
}

/** Opening history never grants editing or changes the ordinary latest-Revision workspace. */
export function BlueprintHistory(props: HistoryProps): JSX.Element {
  const identity = createMemo(
    () =>
      `${props.view.id}:${props.view.current_revision_tuple.revisionNumber}:${props.view.blueprint_edit_number}`,
  );
  return (
    <Show when={identity()} keyed>
      {(_identity) => (
        <HistoryPanel
          client={props.client}
          view={props.view}
          formatDateTime={props.formatDateTime}
        />
      )}
    </Show>
  );
}

function HistoryPanel(props: HistoryProps): JSX.Element {
  const [revisionOpen, setRevisionOpen] = createSignal(false);
  const [metadataOpen, setMetadataOpen] = createSignal(false);
  const [selectedRevision, setSelectedRevision] = createSignal<string>();
  const [revision, setRevision] = createSignal<BlueprintRevisionView>();
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string>();
  let request = 0;
  onCleanup(() => request++);

  async function inspect(number: string): Promise<void> {
    const currentRequest = ++request;
    setSelectedRevision(number);
    setRevision(undefined);
    setError(undefined);
    setBusy(true);
    try {
      const result = await props.client.getBlueprintRevision(props.view.id, number);
      if (currentRequest !== request) return;
      if (
        result.blueprintCourseRevisionTuple.blueprintCourseId !== props.view.id ||
        result.blueprintCourseRevisionTuple.revisionNumber !== number
      )
        throw new Error("Unexpected Blueprint Revision");
      setRevision(result);
    } catch (failure: unknown) {
      if (currentRequest === request) setError(historyError(failure));
    } finally {
      if (currentRequest === request) setBusy(false);
    }
  }

  function closeInspection(): void {
    request++;
    setSelectedRevision(undefined);
    setRevision(undefined);
    setError(undefined);
    setBusy(false);
  }

  return (
    <section class="blueprint-course-inspection" aria-label="Blueprint history">
      <h2>Blueprint history</h2>
      <p>
        Latest saved content: Revision {props.view.current_revision_tuple.revisionNumber}. History
        is read-only; inspecting it does not replace your current content or local edits.
      </p>
      <details onToggle={(event) => setRevisionOpen(event.currentTarget.open)}>
        <summary>Revision history</summary>
        <Show when={revisionOpen()}>
          <HistoryPage
            client={props.client}
            blueprintCourseId={props.view.id}
            kind="revisions"
            currentRevisionNumber={props.view.current_revision_tuple.revisionNumber}
            formatDateTime={props.formatDateTime}
            onInspect={(number) => void inspect(number)}
          />
        </Show>
      </details>
      <details onToggle={(event) => setMetadataOpen(event.currentTarget.open)}>
        <summary>Recorded metadata changes</summary>
        <Show when={metadataOpen()}>
          <p>Recorded names and availability are separate from saved content Revisions.</p>
          <HistoryPage
            client={props.client}
            blueprintCourseId={props.view.id}
            kind="metadata"
            currentRevisionNumber={props.view.current_revision_tuple.revisionNumber}
            formatDateTime={props.formatDateTime}
            onInspect={(number) => void inspect(number)}
          />
        </Show>
      </details>
      <Show when={selectedRevision()}>
        {(number) => (
          <section
            class="blueprint-course-content-card"
            aria-label={`Read-only Blueprint Revision ${number()}`}
          >
            <h3>
              {number() === props.view.current_revision_tuple.revisionNumber
                ? "Latest saved"
                : "Historical"}{" "}
              Revision {number()} - read-only
            </h3>
            <button type="button" class="quiet-action" onClick={closeInspection}>
              Close Revision inspection
            </button>
            <Show when={busy()}>
              <p role="status">Loading exact saved Revision {number()}.</p>
            </Show>
            <Show when={error()}>
              {(message) => (
                <>
                  <p role="alert">{message()}</p>
                  <button type="button" onClick={() => void inspect(number())}>
                    Retry loading Revision {number()}
                  </button>
                </>
              )}
            </Show>
            <Show when={revision()} keyed>
              {(saved) => <BlueprintHistoryContent client={props.client} revision={saved} />}
            </Show>
          </section>
        )}
      </Show>
    </section>
  );
}

interface HistoryPageProps {
  readonly client: BlueprintCourseClient;
  readonly blueprintCourseId: string;
  readonly kind: "revisions" | "metadata";
  readonly currentRevisionNumber: string;
  readonly formatDateTime: (timestamp: number | Date) => string;
  readonly onInspect: (revisionNumber: string) => void;
}

type HistorySortKey = "sequence" | "recordedAt" | "name";
type HistoryFilterKey = "availability";

type HistoryRow = BlueprintHistoryEntryView;

function historyFields(
  kind: "revisions" | "metadata",
  formatDateTime: (timestamp: number | Date) => string,
): ReadonlyArray<RecordField<HistoryRow, HistorySortKey, HistoryFilterKey>> {
  if (kind === "revisions")
    return [
      {
        id: "revision",
        label: "Revision",
        role: "identity",
        sortKey: "sequence",
        value: (row) => (row.kind === "savedRevision" ? `Revision ${row.revisionNumber}` : ""),
        defaultColumn: { priority: 0, width: "12rem" },
      },
      {
        id: "savedAt",
        label: "Saved",
        sortKey: "recordedAt",
        value: (row) => (row.kind === "savedRevision" ? row.savedAt : null),
        render: (row) =>
          row.kind === "savedRevision" ? (
            <time dateTime={new Date(row.savedAt).toISOString()}>
              {formatDateTime(row.savedAt)}
            </time>
          ) : null,
        defaultColumn: { priority: 1, width: "15rem" },
      },
    ];
  return [
    {
      id: "name",
      label: "Long name",
      role: "identity",
      sortKey: "name",
      value: (row) => (row.kind === "metadataChange" ? row.longName : ""),
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "shortName",
      label: "Short name",
      value: (row) => (row.kind === "metadataChange" ? row.shortName : null),
      defaultColumn: { priority: 1, width: "12rem" },
    },
    {
      id: "availability",
      label: "Availability",
      filterKey: "availability",
      value: (row) => (row.kind === "metadataChange" ? row.availability : null),
      defaultColumn: { priority: 2, width: "10rem" },
    },
    {
      id: "recordedAt",
      label: "Recorded",
      sortKey: "recordedAt",
      value: (row) => (row.kind === "metadataChange" ? row.recordedAt : null),
      render: (row) =>
        row.kind === "metadataChange" ? (
          <time dateTime={new Date(row.recordedAt).toISOString()}>
            {formatDateTime(row.recordedAt)}
          </time>
        ) : null,
      defaultColumn: { priority: 3, width: "15rem" },
    },
    {
      id: "theme",
      label: "Theme",
      value: (row) => (row.kind === "metadataChange" ? row.theme : null),
      defaultColumn: { priority: 4, width: "10rem", visible: false },
    },
  ];
}

/** Each independent history stream owns a complete-result query and stable keyset. */
function HistoryPage(props: HistoryPageProps): JSX.Element {
  const initialQuery: BlueprintHistoryQuery = {
    q: "",
    sort: "sequence",
    direction: "descending",
    availability: [],
  };
  const content = (entry: HistoryRow): RecordContent => {
    if (entry.kind === "savedRevision")
      return {
        title: `Revision ${entry.revisionNumber}${entry.revisionNumber === props.currentRevisionNumber ? " (latest saved)" : " (historical)"}`,
        details: [{ kind: "text", value: `Saved ${props.formatDateTime(entry.savedAt)}` }],
        actions: [
          {
            id: "inspect-revision",
            kind: "command",
            label: `Inspect Revision ${entry.revisionNumber}`,
            onClick: () => props.onInspect(entry.revisionNumber),
          },
        ],
      };
    return {
      title: `${entry.longName} (${entry.shortName}) - ${entry.availability}`,
      details: [
        { kind: "text", value: `Recorded ${props.formatDateTime(entry.recordedAt)}` },
        { kind: "text", value: `Theme: ${entry.theme}` },
        { kind: "courseClassification", value: entry.classification },
      ],
      actions: [],
    };
  };
  const definition: SearchDefinition<BlueprintHistoryQuery, HistoryRow, BlueprintHistoryPage> = {
    initialQuery,
    maximumPageSize: 100,
    cleanup: (query) => ({
      q: query.q.trim().toLowerCase().slice(0, 120),
      sort: props.kind === "revisions" && query.sort === "name" ? "sequence" : query.sort,
      direction: query.direction,
      availability: props.kind === "metadata" ? [...new Set(query.availability)].sort() : [],
    }),
    isEligible: () => true,
    getText: (query) => query.q,
    setText: (query, q) => ({ ...query, q }),
    rowId: historyEntryId,
    content,
    fetchPage: async (query, requestedPage) => {
      const page = await props.client.listBlueprintHistory(
        props.blueprintCourseId,
        props.kind,
        query,
        requestedPage.pageNumber,
        requestedPage.pageSize,
      );
      if (page.pageSize !== requestedPage.pageSize)
        throw new Error("Blueprint history response page size differs from its request");
      return { ...page, pageSize: requestedPage.pageSize, filterCounts: page };
    },
    appliedFilters: (query) =>
      query.availability.map((value) => ({
        id: value,
        label: `Availability: ${value}`,
        clear: (current: BlueprintHistoryQuery): BlueprintHistoryQuery => ({
          ...current,
          availability: current.availability.filter((item) => item !== value),
        }),
      })),
  };
  const state = createSearchState(definition, { runOnMount: true });
  const recordQuery: DataSheetQuery<HistorySortKey, HistoryFilterKey> = {
    sort: {
      get value() {
        return { key: state.query().sort, direction: state.query().direction };
      },
      onChange: (sort) =>
        void state.sort((current) => ({ ...current, sort: sort.key, direction: sort.direction })),
    },
    ...(props.kind === "metadata"
      ? {
          filters: {
            get values(): ReadonlyMap<HistoryFilterKey, ReadonlyArray<string>> {
              return new Map<HistoryFilterKey, ReadonlyArray<string>>([
                ["availability", state.query().availability],
              ]);
            },
            onChange: (_key: HistoryFilterKey, values: ReadonlyArray<string>) =>
              void state.apply((current) => ({
                ...current,
                availability: values.filter(
                  (value): value is "active" | "archived" =>
                    value === "active" || value === "archived",
                ),
              })),
            onClear: () => void state.apply((current) => ({ ...current, availability: [] })),
            choices: async (
              _key: HistoryFilterKey,
              search: string,
            ): Promise<RecordFilterChoices> => {
              const query = state.query();
              const page = await props.client.listBlueprintHistory(
                props.blueprintCourseId,
                props.kind,
                { ...query, availability: [] },
                1,
                1,
              );
              const choices = (["active", "archived"] as const).map((key) => ({
                key,
                label: key === "active" ? "Active" : "Archived",
                count:
                  key === "active"
                    ? page.availabilityFacets.activeCount
                    : page.availabilityFacets.archivedCount,
              }));
              const term = search.trim().toLowerCase();
              return {
                choices: choices.filter((choice) => choice.label.toLowerCase().includes(term)),
                selected: choices.filter((choice) => query.availability.includes(choice.key)),
                nextCursor: null,
              };
            },
          },
        }
      : {}),
  };
  return (
    <SearchBlock
      layout="above"
      state={state}
      controls={
        <SearchControls
          state={state}
          textLabel={props.kind === "revisions" ? "Search Revisions" : "Search metadata changes"}
          textPlaceholder={props.kind === "revisions" ? "Revision number" : "Name or theme"}
        />
      }
      collection={{
        ariaLabel: props.kind === "revisions" ? "Saved Revisions" : "Recorded metadata changes",
        emptyState: {
          title:
            props.kind === "revisions" ? "No matching Revisions" : "No matching metadata changes",
          message: "Try another search or filter.",
        },
        fields: historyFields(props.kind, props.formatDateTime),
        query: recordQuery,
      }}
    />
  );
}
