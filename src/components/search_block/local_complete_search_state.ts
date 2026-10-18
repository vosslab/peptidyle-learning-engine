import { createEffect, createMemo, untrack, type Accessor } from "solid-js";

import type { RecordContent } from "../data_sheet/record_content";
import type { RecordListQuery, RecordSort } from "../data_sheet/record_fields";
import {
  localCompleteFilterChoices,
  searchLocalCompletePage,
  type LocalCompleteQuery,
  type LocalCompleteSearchOptions,
} from "./local_complete_search";
import { createSearchState, type SearchStateController } from "./search_state";
import type { SearchDefinition, SearchState } from "./search_session";

export type LocalCompleteSource<Row> = {
  /** Undefined means undelivered; an empty array is a delivered empty collection. */
  readonly rows: Accessor<ReadonlyArray<Row> | undefined>;
  readonly loading?: Accessor<boolean>;
  readonly error?: Accessor<unknown>;
  readonly refetchSource?: () => unknown;
};

export type LocalCompleteSearchBindings<Row, SortKey extends string, FilterKey extends string> = {
  readonly state: SearchStateController<LocalCompleteQuery<SortKey, FilterKey>, Row, number>;
  readonly query: RecordListQuery<SortKey, FilterKey>;
};

/** Adapts a complete reactive collection to the standard search/session contracts. */
export function createLocalCompleteSearch<Row, SortKey extends string, FilterKey extends string>(
  source: LocalCompleteSource<Row>,
  options: LocalCompleteSearchOptions<Row, SortKey, FilterKey> & {
    readonly content: (row: Row) => RecordContent;
    readonly selectionMaximum?: number;
    readonly beforeQueryChange?: () => Promise<boolean>;
  },
): LocalCompleteSearchBindings<Row, SortKey, FilterKey> {
  type Query = LocalCompleteQuery<SortKey, FilterKey>;
  let authorizedRows: ReadonlyArray<Row> = [];
  let delivered = false;
  const initialQuery: Query = { text: "", sort: null, filters: new Map() };
  const definition: SearchDefinition<Query, Row, number> = {
    initialQuery,
    cleanup: (query) => ({ ...query, text: query.text.trim() }),
    isEligible: () => true,
    getText: (query) => query.text,
    setText: (query, text) => ({ ...query, text }),
    fetchPage: (query, page) =>
      Promise.resolve(searchLocalCompletePage(authorizedRows, query, page, options)),
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
    const error = source.error?.();
    if (error !== undefined) {
      return {
        kind: "error",
        rows: current.rows,
        filterCounts: current.filterCounts,
        pageNumber: current.pageNumber,
        pageSize: current.pageSize,
        totalCount: current.totalCount,
        error,
      };
    }
    if (source.loading?.()) {
      return {
        kind: "loading",
        rows: current.rows,
        filterCounts: current.filterCounts,
        pageNumber: current.pageNumber,
        pageSize: current.pageSize,
        totalCount: current.totalCount,
      };
    }
    return current;
  }

  const state = createMemo(projectedState);
  const controller: SearchStateController<Query, Row, number> = {
    ...base,
    state,
    retry: async (): Promise<void> => {
      if (source.error?.() !== undefined && source.refetchSource) await source.refetchSource();
      else await base.retry();
    },
    refresh: async (): Promise<void> => {
      if (source.refetchSource) await source.refetchSource();
      else await base.refresh();
    },
  };

  createEffect(() => {
    if (source.error?.() !== undefined) return;
    const rows = source.rows();
    if (rows === undefined) return;
    untrack(() => {
      const ids = new Set(rows.map(options.rowId));
      authorizedRows = rows;
      base.deselectIds([...base.selectedIds()].filter((id) => !ids.has(id)));
      if (delivered || base.state().kind !== "initial") {
        delivered = true;
        void base.refresh();
      } else {
        delivered = true;
        const draft = base.hasTypedDraft() ? base.typedText() : undefined;
        void base.open(base.query());
        if (draft !== undefined) base.setTypedText(draft);
      }
    });
  });

  const query: RecordListQuery<SortKey, FilterKey> = {
    kind: "paged",
    sort: {
      get value(): RecordSort<SortKey> | null {
        return base.query().sort;
      },
      onChange: (sort): void => {
        void base.sort((current) => ({ ...current, sort }));
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
