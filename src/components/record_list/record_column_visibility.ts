import type { RecordField } from "./record_fields";

export type ColumnChoice = "show" | "hide";
export const RECORD_ACTIONS_COLUMN_WIDTH = "14rem";
const RECORD_ACTIONS_COLUMN_MINIMUM_WIDTH = 112;

/** Default widths guide automatic visibility; explicit resizing keeps its own width. */
function columnWidth(width: string | undefined, rootFontSize: number): number {
  if (width?.endsWith("rem")) {
    const rem = Number.parseFloat(width);
    if (Number.isFinite(rem)) return rem * rootFontSize;
  }
  if (width?.endsWith("px")) {
    const pixels = Number.parseFloat(width);
    if (Number.isFinite(pixels)) return pixels;
  }
  return 160;
}

/**
 * Actions remain reachable on a narrow compact table without consuming the
 * identity column. A user-resized action column continues to take precedence.
 */
export function defaultRecordActionsWidth(availableWidth: number, rootFontSize: number): number {
  const preferred = columnWidth(RECORD_ACTIONS_COLUMN_WIDTH, rootFontSize);
  if (availableWidth <= 0) return preferred;
  return Math.max(RECORD_ACTIONS_COLUMN_MINIMUM_WIDTH, Math.min(preferred, availableWidth * 0.4));
}

export function visibleRecordFields<Row, SortKey extends string, FilterKey extends string>(
  fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>,
  choices: ReadonlyMap<string, ColumnChoice>,
  availableWidth: number,
  rootFontSize: number,
  hasActions: boolean,
  identityWidth = 240,
): ReadonlyArray<RecordField<Row, SortKey, FilterKey>> {
  const dataFields = fields.filter((field) => field.role !== "identity");
  const size = (field: RecordField<Row, SortKey, FilterKey>): number =>
    columnWidth(field.defaultColumn.width, rootFontSize);
  // A user resize can make the table scroll; it does not silently hide a field.
  let used =
    identityWidth + (hasActions ? defaultRecordActionsWidth(availableWidth, rootFontSize) : 0);
  const visible = new Set<string>();

  for (const field of dataFields) {
    if (choices.get(field.id) !== "show") continue;
    visible.add(field.id);
    used += size(field);
  }

  const automatic = dataFields
    .filter((field) => choices.get(field.id) === undefined && field.defaultColumn.visible !== false)
    .map((field, index) => ({ field, index }))
    .sort(
      (left, right) =>
        left.field.defaultColumn.priority - right.field.defaultColumn.priority ||
        left.index - right.index,
    );
  for (const { field } of automatic) {
    const width = size(field);
    if (availableWidth === 0 || used + width <= availableWidth) {
      visible.add(field.id);
      used += width;
    }
  }

  return dataFields.filter((field) => visible.has(field.id));
}

export function recordColumnWidth(width: string | undefined, rootFontSize: number): number {
  return width === undefined ? 240 : columnWidth(width, rootFontSize);
}

export function visibleTableWidth<Row, SortKey extends string, FilterKey extends string>(
  fields: ReadonlyArray<RecordField<Row, SortKey, FilterKey>>,
  widths: ReadonlyMap<string, number>,
  availableWidth: number,
  rootFontSize: number,
  hasActions: boolean,
  identityId = "record_identity",
  identityWidth = 240,
): number {
  return (
    (widths.get(identityId) ?? identityWidth) +
    (hasActions
      ? (widths.get("record_actions") ?? defaultRecordActionsWidth(availableWidth, rootFontSize))
      : 0) +
    fields.reduce(
      (sum, field) =>
        sum + (widths.get(field.id) ?? columnWidth(field.defaultColumn.width, rootFontSize)),
      0,
    )
  );
}
