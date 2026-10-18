import type {
  RecordField,
  RecordFieldScalar,
  RecordFilterChoices,
  RecordSort,
} from "../data_sheet/data_sheet_fields";
import type { SearchPage, SearchPageRequest } from "./search_definition";

/** A query over one complete, already authorized source array. */
export type LocalCompleteQuery<SortKey extends string, FilterKey extends string> = {
  readonly text: string;
  readonly sort: RecordSort<SortKey> | null;
  readonly filters: ReadonlyMap<FilterKey, ReadonlyArray<string>>;
};

export type LocalCompleteSearchOptions<Row, SortKey extends string, FilterKey extends string> = {
  readonly fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>;
  readonly rowId: (row: Row) => string;
  readonly keyword: (row: Row) => string;
  /** Apply a fixed domain scope before user-controlled matching. */
  readonly inScope?: (row: Row) => boolean;
  /** Domain comparison for values that cannot safely use the generic scalar order. */
  readonly compare?: (key: SortKey, left: Row, right: Row) => number | undefined;
};

function fieldValues<Row, SortKey extends string, FilterKey extends string>(
  field: RecordField<Row, SortKey, FilterKey>,
  row: Row,
): ReadonlyArray<RecordFieldScalar> {
  const value = field.value(row);
  if (value === null) return [];
  return Array.isArray(value)
    ? (value as ReadonlyArray<RecordFieldScalar>)
    : [value as RecordFieldScalar];
}

function matchingRows<Row, SortKey extends string, FilterKey extends string>(
  rows: ReadonlyArray<Row>,
  query: LocalCompleteQuery<SortKey, FilterKey>,
  options: LocalCompleteSearchOptions<Row, SortKey, FilterKey>,
  excludedFilter?: FilterKey,
): Row[] {
  const filterFields = new Map<FilterKey, RecordField<Row, SortKey, FilterKey>>();
  for (const field of options.fields) {
    if (field.filterKey !== undefined) filterFields.set(field.filterKey, field);
  }
  const words = query.text.trim().toLocaleLowerCase().split(/\s+/u).filter(Boolean);
  return rows.filter((row) => {
    if (options.inScope !== undefined && !options.inScope(row)) return false;
    const haystack = options.keyword(row).toLocaleLowerCase();
    if (!words.every((word) => haystack.includes(word))) return false;
    for (const [key, selected] of query.filters) {
      if (key === excludedFilter || selected.length === 0) continue;
      const field = filterFields.get(key);
      if (field === undefined) throw new Error(`Unknown local filter field: ${key}`);
      if (!fieldValues(field, row).some((value) => selected.includes(String(value)))) return false;
    }
    return true;
  });
}

function compareScalar(
  left: RecordFieldScalar | undefined,
  right: RecordFieldScalar | undefined,
): number {
  if (left === undefined) return right === undefined ? 0 : 1;
  if (right === undefined) return -1;
  if (typeof left === "number" && typeof right === "number") return left - right;
  if (typeof left === "boolean" && typeof right === "boolean") return Number(left) - Number(right);
  return String(left).toLocaleLowerCase().localeCompare(String(right).toLocaleLowerCase());
}

function sortedRows<Row, SortKey extends string, FilterKey extends string>(
  rows: Row[],
  sort: RecordSort<SortKey> | null,
  options: LocalCompleteSearchOptions<Row, SortKey, FilterKey>,
): Row[] {
  if (sort === null) return rows;
  const field = options.fields.find((candidate) => candidate.sortKey === sort.key);
  if (field === undefined) throw new Error(`Unknown local sort field: ${sort.key}`);
  const direction = sort.direction === "ascending" ? 1 : -1;
  return rows.sort((left, right) => {
    const compared =
      options.compare?.(sort.key, left, right) ??
      compareScalar(fieldValues(field, left)[0], fieldValues(field, right)[0]);
    return compared === 0
      ? options.rowId(left).localeCompare(options.rowId(right))
      : direction * compared;
  });
}

function offsetFromCursor(cursor: string | null): number {
  if (cursor === null) return 0;
  if (!/^(0|[1-9][0-9]*)$/u.test(cursor)) throw new Error("Invalid local page cursor.");
  const offset = Number(cursor);
  if (!Number.isSafeInteger(offset)) throw new Error("Invalid local page cursor.");
  return offset;
}

/** Match and order the whole array before taking a page. */
export function searchLocalCompletePage<Row, SortKey extends string, FilterKey extends string>(
  rows: ReadonlyArray<Row>,
  query: LocalCompleteQuery<SortKey, FilterKey>,
  page: SearchPageRequest,
  options: LocalCompleteSearchOptions<Row, SortKey, FilterKey>,
): SearchPage<Row, number> {
  const matched = sortedRows(matchingRows(rows, query, options), query.sort, options);
  const totalCount = matched.length;
  const totalPages = Math.max(1, Math.ceil(totalCount / page.pageSize));
  const pageNumber = Math.min(Math.max(1, page.pageNumber), totalPages);
  const offset = (pageNumber - 1) * page.pageSize;
  return {
    items: matched.slice(offset, offset + page.pageSize),
    pageNumber,
    pageSize: page.pageSize,
    totalCount,
    filterCounts: totalCount,
  };
}

/** Count each facet on the whole matching array with its own filter excluded. */
export function localCompleteFilterChoices<Row, SortKey extends string, FilterKey extends string>(
  rows: ReadonlyArray<Row>,
  query: LocalCompleteQuery<SortKey, FilterKey>,
  key: FilterKey,
  search: string,
  cursor: string | null,
  options: LocalCompleteSearchOptions<Row, SortKey, FilterKey>,
): RecordFilterChoices {
  const field = options.fields.find((candidate) => candidate.filterKey === key);
  if (field === undefined) throw new Error(`Unknown local filter field: ${key}`);
  const counts = new Map<string, number>();
  for (const row of matchingRows(rows, query, options, key)) {
    for (const value of new Set(fieldValues(field, row).map(String))) {
      counts.set(value, (counts.get(value) ?? 0) + 1);
    }
  }
  const selected = (query.filters.get(key) ?? []).map((value) => ({
    key: value,
    label: value,
    count: counts.get(value) ?? 0,
  }));
  const term = search.trim().toLocaleLowerCase();
  const choices = [...counts.entries()]
    .filter(([value]) => value.toLocaleLowerCase().includes(term))
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([value, count]) => ({ key: value, label: value, count }));
  const offset = offsetFromCursor(cursor);
  const pageSize = 50;
  return {
    choices: choices.slice(offset, offset + pageSize),
    selected,
    nextCursor: offset + pageSize < choices.length ? String(offset + pageSize) : null,
  };
}
