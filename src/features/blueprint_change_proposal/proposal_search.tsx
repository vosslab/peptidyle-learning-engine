import { createEffect, type JSX } from "solid-js";
import type { BlueprintChangeProposalSummaryView } from "../../../generated/api/BlueprintChangeProposalSummaryView";
import type {
  BlueprintChangeProposalClient,
  BlueprintProposalListQuery,
} from "../../api/blueprint_change_proposal";
import type { RecordField, DataSheetQuery } from "../../components/data_sheet/data_sheet_fields";
import type { RecordContent } from "../../components/data_sheet/record_content";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import type { SearchDefinition } from "../../components/search_block/search_definition";

type ProposalSortKey = "created";
type ProposalFilterKey = "state";
type ProposalStatus = BlueprintProposalListQuery["status"][number];

function selectedStatuses(values: ReadonlyArray<string>): ProposalStatus[] {
  return [
    ...new Set(
      values.filter(
        (value): value is ProposalStatus =>
          value === "proposed" || value === "stale" || value === "accepted",
      ),
    ),
  ].sort();
}

function proposalFields(
  formatDateTime: (timestamp: number | Date) => string,
): ReadonlyArray<
  RecordField<BlueprintChangeProposalSummaryView, ProposalSortKey, ProposalFilterKey>
> {
  return [
    {
      id: "names",
      label: "Change Proposal",
      role: "identity",
      value: (row) => `${row.sourceNames.longName} to ${row.targetNames.longName}`,
      defaultColumn: { priority: 0, width: "22rem" },
    },
    {
      id: "source",
      label: "Source",
      value: (row) => row.sourceNames.shortName,
      defaultColumn: { priority: 1, width: "10rem" },
    },
    {
      id: "target",
      label: "Target",
      value: (row) => row.targetNames.shortName,
      defaultColumn: { priority: 2, width: "10rem" },
    },
    {
      id: "state",
      label: "State",
      filterKey: "state",
      value: (row) =>
        row.accepted !== null ? "accepted" : row.targetIsStale ? "stale" : "proposed",
      render: (row) =>
        row.accepted !== null ? "Accepted" : row.targetIsStale ? "Older target basis" : "Proposed",
      defaultColumn: { priority: 3, width: "9rem" },
    },
    {
      id: "created",
      label: "Created",
      sortKey: "created",
      value: (row) => row.createdAt,
      render: (row) => (
        <time dateTime={new Date(row.createdAt).toISOString()}>
          {formatDateTime(row.createdAt)}
        </time>
      ),
      defaultColumn: { priority: 4, width: "11rem" },
    },
  ];
}

export function ProposalSearch(props: {
  readonly client: BlueprintChangeProposalClient;
  readonly target?: string;
  readonly content: (row: BlueprintChangeProposalSummaryView) => RecordContent;
  readonly formatDateTime: (timestamp: number | Date) => string;
}): JSX.Element {
  const displayFields = proposalFields(props.formatDateTime);
  const initialQuery: BlueprintProposalListQuery = {
    text: "",
    sort: "createdDescending",
    status: [],
  };
  const definition: SearchDefinition<
    BlueprintProposalListQuery,
    BlueprintChangeProposalSummaryView,
    undefined
  > = {
    maximumPageSize: 100,
    initialQuery,
    cleanup: (query) => ({
      ...query,
      text: query.text.trim(),
      status: selectedStatuses(query.status),
    }),
    isEligible: () => true,
    getText: (query) => query.text,
    setText: (query, text) => ({ ...query, text }),
    rowId: (row) => row.proposalId,
    content: props.content,
    fetchPage: async (query, requestedPage) => {
      const response =
        props.target === undefined
          ? await props.client.listMyBlueprintChangeProposals(
              query,
              requestedPage.pageNumber,
              requestedPage.pageSize,
            )
          : await props.client.listBlueprintChangeProposalsForTarget(
              props.target,
              query,
              requestedPage.pageNumber,
              requestedPage.pageSize,
            );
      if (response.pageSize !== requestedPage.pageSize)
        throw new Error("Blueprint Change Proposal response page size differs from its request");
      return { ...response, pageSize: requestedPage.pageSize, filterCounts: undefined };
    },
    appliedFilters: (query) =>
      query.status.map((status) => ({
        id: `status:${status}`,
        label: `State: ${status}`,
        clear: (current) => ({
          ...current,
          status: current.status.filter((value) => value !== status),
        }),
      })),
  };
  const state = createSearchState(definition, { runOnMount: true });
  let previousTarget = props.target;
  createEffect(() => {
    const target = props.target;
    if (target === previousTarget) return;
    previousTarget = target;
    void state.open(initialQuery);
  });
  const recordQuery: DataSheetQuery<ProposalSortKey, ProposalFilterKey> = {
    sort: {
      get value() {
        return {
          key: "created" as const,
          direction:
            state.query().sort === "createdAscending"
              ? ("ascending" as const)
              : ("descending" as const),
        };
      },
      onChange: (sort) =>
        void state.sort((query) => ({
          ...query,
          sort: sort.direction === "ascending" ? "createdAscending" : "createdDescending",
        })),
    },
    filters: {
      get values() {
        return new Map<ProposalFilterKey, ReadonlyArray<string>>([["state", state.query().status]]);
      },
      onChange: (_key, values) =>
        void state.apply((query) => ({
          ...query,
          status: selectedStatuses(values),
        })),
      onClear: () => void state.apply((query) => ({ ...query, status: [] })),
      choices: async (_key, search, cursor, signal) => {
        if (cursor !== null) return { choices: [], selected: [], nextCursor: null };
        const choices = await props.client.listBlueprintProposalStatusChoices(
          props.target,
          state.query().text,
          state.query().status,
        );
        if (signal.aborted) throw new DOMException("Aborted", "AbortError");
        return {
          ...choices,
          choices: choices.choices.filter((choice) =>
            choice.label.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase()),
          ),
        };
      },
    },
  };
  return (
    <SearchBlock
      layout="above"
      controls={
        <SearchControls state={state} textLabel="Proposal source or target">
          <label>
            State{" "}
            <select
              value={
                state.query().status.length === 0
                  ? "all"
                  : state.query().status.length === 1
                    ? state.query().status[0]
                    : "multiple"
              }
              onChange={(event) =>
                void state.apply((query) => ({
                  ...query,
                  status: selectedStatuses([event.currentTarget.value]),
                }))
              }
            >
              <option value="all">All</option>
              <option value="multiple" disabled>
                Multiple states
              </option>
              <option value="proposed">Proposed</option>
              <option value="stale">Older target basis</option>
              <option value="accepted">Accepted</option>
            </select>
          </label>
        </SearchControls>
      }
      state={state}
      collection={{
        ariaLabel: "Blueprint Change Proposals",
        fields: displayFields,
        query: recordQuery,
        emptyState: {
          title: "No Change Proposals match.",
          message:
            props.target === undefined
              ? "Clear filters or open a Blueprint Course to propose a change."
              : "Clear filters or choose a saved Blueprint source above.",
        },
      }}
    />
  );
}
