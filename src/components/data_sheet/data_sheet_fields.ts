import type { JSX } from "solid-js";

/** Values stay separate from rendered labels so query bindings use domain data. */
export type RecordFieldScalar = string | number | boolean;
export type RecordFieldValue = RecordFieldScalar | ReadonlyArray<RecordFieldScalar> | null;

type RecordFieldBase<SortKey extends string, FilterKey extends string> = {
  /** Stable across pages, result refreshes, and presentation modes. */
  readonly id: string;
  readonly label: string;
  readonly sortKey?: SortKey;
  readonly filterKey?: FilterKey;
  readonly defaultColumn: {
    /** Lower numbers remain visible longer as the available width shrinks. */
    readonly priority: number;
    readonly width?: string;
    /** True retains the column at any width; false hides it; omitted fits responsively.
     * Explicit user Show/Hide choices take precedence.
     */
    readonly visible?: boolean;
  };
};

export type RecordField<Row, SortKey extends string, FilterKey extends string> = RecordFieldBase<
  SortKey,
  FilterKey
> &
  (
    | {
        readonly role?: "data";
        readonly value: (row: Row) => RecordFieldValue;
        /** Optional semantic display for a value; never used to derive query keys. */
        readonly render?: (row: Row) => JSX.Element;
      }
    | {
        /** The canonical title occupies the one semantic row-header column. */
        readonly role: "identity";
        readonly value: (row: Row) => string;
        readonly render?: never;
      }
  );

/** Validate stable column IDs and locate the sole title source. */
export function recordIdentityField<Row, SortKey extends string, FilterKey extends string>(
  fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>,
): Extract<RecordField<Row, SortKey, FilterKey>, { readonly role: "identity" }> | undefined {
  const ids = new Set<string>();
  let identity:
    Extract<RecordField<Row, SortKey, FilterKey>, { readonly role: "identity" }> | undefined;
  for (const field of fields) {
    if (
      field.id.length === 0 ||
      field.id === "record_identity" ||
      field.id === "record_actions" ||
      field.id === "record_details" ||
      ids.has(field.id)
    ) {
      throw new Error(`RecordList field ID ${field.id} is empty, reserved, or duplicated.`);
    }
    ids.add(field.id);
    if (field.role === "identity") {
      if (identity) throw new Error("RecordList accepts only one identity field.");
      identity = field;
    }
  }
  return identity;
}

export type RecordSort<SortKey extends string> = {
  readonly key: SortKey;
  readonly direction: "ascending" | "descending";
};

export type RecordSortBinding<SortKey extends string> = {
  readonly value: RecordSort<SortKey> | null;
  readonly onChange: (sort: RecordSort<SortKey>) => void;
  /** Valid query orders without a sortable column, such as recently published. */
  readonly choices?: ReadonlyArray<{ readonly label: string; readonly sort: RecordSort<SortKey> }>;
};

export type RecordFilterBinding<FilterKey extends string> = {
  readonly values: ReadonlyMap<FilterKey, ReadonlyArray<string>>;
  readonly onChange: (key: FilterKey, values: ReadonlyArray<string>) => void;
  /** Clears column filters as one query-owner transition, preserving other criteria. */
  readonly onClear: () => void;
  /** Choices cover matches, including selected values outside this page. */
  readonly choices: (
    key: FilterKey,
    search: string,
    cursor: string | null,
    signal: AbortSignal,
  ) => Promise<RecordFilterChoices>;
};

export type RecordFilterChoice = {
  readonly key: string;
  readonly label: string;
  readonly count: number;
};

export type RecordFilterChoices = {
  readonly choices: ReadonlyArray<RecordFilterChoice>;
  readonly selected: ReadonlyArray<RecordFilterChoice>;
  readonly nextCursor: string | null;
  /** Explanation for choices unavailable until another selection is made. */
  readonly unavailableReason?: string;
};

/** Query owners apply sorting and filtering to the complete collection before paging. */
export type DataSheetQuery<SortKey extends string, FilterKey extends string> = {
  readonly sort?: RecordSortBinding<SortKey>;
  readonly filters?: RecordFilterBinding<FilterKey>;
};
