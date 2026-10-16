import { createEffect, createMemo, untrack, type Resource } from "solid-js";

import type { RecordContent } from "../../components/record_list/record_list";
import type { RecordListQuery, RecordSort } from "../../components/record_list/record_fields";
import {
  localCompleteFilterChoices,
  searchLocalCompletePage,
  type LocalCompleteQuery,
  type LocalCompleteSearchOptions,
} from "./local_complete_search";
import { createSearchState, type SearchStateController } from "./search_state";
import type { SearchDefinition, SearchState } from "./search_session";

type LocalCompleteSource<Source, Row> = {
  readonly resource: Resource<Source>;
  readonly refetchSource: () => unknown;
  readonly rows: (value: Source) => ReadonlyArray<Row>;
};

export type LocalCompleteSearchBindings<Row, SortKey extends string, FilterKey extends string> = {
  readonly state: SearchStateController<LocalCompleteQuery<SortKey, FilterKey>, Row, number>;
  readonly query: RecordListQuery<SortKey, FilterKey>;
};

/** Adapts an existing complete resource to the standard search/session contracts. */
export function createLocalCompleteSearch<
  Row,
  SortKey extends string,
  FilterKey extends string,
  Source = ReadonlyArray<Row>,
>(
  source: LocalCompleteSource<Source, Row>,
  options: LocalCompleteSearchOptions<Row, SortKey, FilterKey> & {
    readonly content: (row: Row) => RecordContent;
    readonly selectionMaximum?: number;
    readonly beforeQueryChange?: () => Promise<boolean>;
  },
): LocalCompleteSearchBindings<Row, SortKey, FilterKey> {
  type Query = LocalCompleteQuery<SortKey, FilterKey>;
  let authorizedRows: ReadonlyArray<Row> = [];
  let opened = false;
  let previousIds = new Set<string>();
  const initialQuery: Query = { text: "", sort: null, filters: new Map() };
  const definition: SearchDefinition<Query, Row, number> = {
    initialQuery,
    cleanup: (query) => ({ ...query, text: query.text.trim() }),
    isEligible: () => true,
    getText: (query) => query.text,
    setText: (query, text) => ({ ...query, text }),
    fetchPage: (query, cursor, pageSize) =>
      Promise.resolve(searchLocalCompletePage(authorizedRows, query, cursor, pageSize, options)),
    rowId: options.rowId,
    content: options.content,
    ...(options.selectionMaximum === undefined
      ? {}
      : { selection: { maximum: options.selectionMaximum } }),
    appliedFilters: (query) =>
      [...query.filters].flatMap(([key, values]) =>
        values.map((value) => ({
          id: `${key}:${value}`,
          label: `${options.fields.find((field) => field.filterKey === key)?.label ?? key}: ${value}`,
          clear: (current: Query): Query => {
            const filters = new Map(current.filters);
            const next = (filters.get(key) ?? []).filter((selected) => selected !== value);
            if (next.length === 0) filters.delete(key);
            else filters.set(key, next);
            return { ...current, filters };
          },
        })),
      ),
  };
  const base = createSearchState(definition, { beforeQueryChange: options.beforeQueryChange });

  function projectedState(): SearchState<Row, number> {
    const current = base.state();
    if (source.resource.state === "errored") {
      return {
        kind: "error",
        rows: current.rows,
        filterCounts: current.filterCounts,
        nextCursor: current.nextCursor,
        error: source.resource.error,
      };
    }
    if (source.resource.loading) {
      return {
        kind: "loading",
        rows: current.rows,
        filterCounts: current.filterCounts,
        nextCursor: current.nextCursor,
      };
    }
    return current;
  }

  const state = createMemo(projectedState);
  const controller: SearchStateController<Query, Row, number> = {
    ...base,
    state,
    retry: async (): Promise<void> => {
      if (source.resource.state === "errored") await source.refetchSource();
      else await base.retry();
    },
    refresh: async (): Promise<void> => {
      await source.refetchSource();
    },
  };

  createEffect(() => {
    const status = source.resource.state;
    if (status !== "ready") return;
    const rows = source.rows(source.resource());
    untrack(() => {
      const ids = new Set(rows.map(options.rowId));
      const removed = [...previousIds].some((id) => !ids.has(id));
      authorizedRows = rows;
      previousIds = ids;
      if (!opened) {
        opened = true;
        void base.open(initialQuery);
        return;
      }
      base.deselectIds([...base.selectedIds()].filter((id) => !ids.has(id)));
      void (async (): Promise<void> => {
        await base.refresh();
        while (removed && base.state().kind === "empty" && base.hasPrevious()) {
          await base.previous();
        }
      })();
    });
  });

  const query: RecordListQuery<SortKey, FilterKey> = {
    kind: "paged",
    sort: {
      get value(): RecordSort<SortKey> | null {
        return base.query().sort;
      },
      onChange: (sort): void => {
        void base.apply((current) => ({ ...current, sort }));
      },
    },
    filters: {
      get values(): ReadonlyMap<FilterKey, ReadonlyArray<string>> {
        return base.query().filters;
      },
      onChange: (key, values): void => {
        void base.apply((current) => {
          const filters = new Map(current.filters);
          if (values.length === 0) filters.delete(key);
          else filters.set(key, [...new Set(values)]);
          return { ...current, filters };
        });
      },
      onClear: (): void => {
        void base.apply((current) => ({ ...current, filters: new Map() }));
      },
      choices: (key, search, cursor, signal) => {
        if (signal.aborted) throw new DOMException("Aborted", "AbortError");
        return Promise.resolve(
          localCompleteFilterChoices(authorizedRows, base.query(), key, search, cursor, options),
        );
      },
    },
  };
  return { state: controller, query };
}
