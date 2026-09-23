/**
 * Selects a contiguous RecordList slice without owning its markup or record state.
 *
 * A caller measures mounted rows by record id and passes those measurements back on the next
 * calculation. Unmeasured rows use the supplied estimate, so the spacer always represents the
 * whole list while measurements converge.
 */

export type RecordListWindowMeasurements = ReadonlyMap<string, number>;

export type RecordListWindowMetrics<Row> = {
  readonly records: ReadonlyArray<Row>;
  readonly recordId: (record: Row) => string;
  readonly estimatedRecordHeightPx: number;
  readonly measuredRecordHeightsPx?: RecordListWindowMeasurements;
};

export type RecordListWindowOptions<Row> = RecordListWindowMetrics<Row> & {
  readonly scrollTopPx: number;
  readonly viewportHeightPx: number;
  readonly overscanPx: number;
  /** Keep this record mounted when it owns keyboard focus. */
  readonly focusedRecordId?: string;
};

export type RecordListWindow<Row> = {
  /** The original records, in their original order, for the mounted contiguous slice. */
  readonly records: ReadonlyArray<Row>;
  readonly firstRecordIndex: number;
  readonly lastRecordIndexExclusive: number;
  readonly topSpacerHeightPx: number;
  readonly bottomSpacerHeightPx: number;
  readonly totalHeightPx: number;
};

export type RecordListWindowScrollAlignment = "start" | "center" | "end" | "nearest";

type RecordListWindowLayout<Row> = {
  readonly records: ReadonlyArray<Row>;
  readonly recordIds: ReadonlyArray<string>;
  readonly recordStartsPx: ReadonlyArray<number>;
  readonly recordHeightsPx: ReadonlyArray<number>;
  readonly totalHeightPx: number;
};

function finiteNonnegative(value: number): number {
  if (!Number.isFinite(value)) return 0;
  const normalized = Math.max(0, value);
  return normalized;
}

function validEstimatedHeight(estimatedRecordHeightPx: number): number {
  if (!Number.isFinite(estimatedRecordHeightPx) || estimatedRecordHeightPx <= 0) {
    throw new Error("estimatedRecordHeightPx must be a positive finite number");
  }
  return estimatedRecordHeightPx;
}

function measuredHeightOrEstimate(
  recordId: string,
  estimatedRecordHeightPx: number,
  measuredRecordHeightsPx: RecordListWindowMeasurements | undefined,
): number {
  const measuredHeightPx = measuredRecordHeightsPx?.get(recordId);
  if (
    measuredHeightPx === undefined ||
    !Number.isFinite(measuredHeightPx) ||
    measuredHeightPx <= 0
  ) {
    return estimatedRecordHeightPx;
  }
  return measuredHeightPx;
}

function recordListWindowLayout<Row>(
  metrics: RecordListWindowMetrics<Row>,
): RecordListWindowLayout<Row> {
  const estimatedRecordHeightPx = validEstimatedHeight(metrics.estimatedRecordHeightPx);
  const recordIds: string[] = [];
  const recordStartsPx: number[] = [];
  const recordHeightsPx: number[] = [];
  const knownRecordIds = new Set<string>();
  let totalHeightPx = 0;

  for (const record of metrics.records) {
    const recordId = metrics.recordId(record);
    if (knownRecordIds.has(recordId)) {
      throw new Error(`RecordList window requires unique record ids; found ${recordId}`);
    }
    knownRecordIds.add(recordId);
    recordIds.push(recordId);
    recordStartsPx.push(totalHeightPx);
    const recordHeightPx = measuredHeightOrEstimate(
      recordId,
      estimatedRecordHeightPx,
      metrics.measuredRecordHeightsPx,
    );
    recordHeightsPx.push(recordHeightPx);
    totalHeightPx += recordHeightPx;
  }

  return {
    records: metrics.records,
    recordIds,
    recordStartsPx,
    recordHeightsPx,
    totalHeightPx,
  };
}

function firstRecordAtOrAfter(layout: RecordListWindowLayout<unknown>, positionPx: number): number {
  for (let recordIndex = 0; recordIndex < layout.records.length; recordIndex += 1) {
    const recordEndPx = layout.recordStartsPx[recordIndex]! + layout.recordHeightsPx[recordIndex]!;
    if (recordEndPx > positionPx) return recordIndex;
  }
  return layout.records.length;
}

function firstRecordAfter(layout: RecordListWindowLayout<unknown>, positionPx: number): number {
  for (let recordIndex = 0; recordIndex < layout.records.length; recordIndex += 1) {
    if (layout.recordStartsPx[recordIndex]! >= positionPx) return recordIndex;
  }
  return layout.records.length;
}

function focusedRecordIndex(
  layout: RecordListWindowLayout<unknown>,
  focusedRecordId: string | undefined,
): number {
  if (focusedRecordId === undefined) return -1;
  const recordIndex = layout.recordIds.indexOf(focusedRecordId);
  return recordIndex;
}

/**
 * Returns the records that should stay mounted for a scroll position.
 *
 * The result is always one original contiguous slice. If a focused record falls outside the
 * viewport, the slice expands to include it, keeping that DOM node mounted until focus moves.
 */
export function recordListWindow<Row>(
  options: RecordListWindowOptions<Row>,
): RecordListWindow<Row> {
  const layout = recordListWindowLayout(options);
  if (layout.records.length === 0) {
    return {
      records: [],
      firstRecordIndex: 0,
      lastRecordIndexExclusive: 0,
      topSpacerHeightPx: 0,
      bottomSpacerHeightPx: 0,
      totalHeightPx: 0,
    };
  }

  const scrollTopPx = finiteNonnegative(options.scrollTopPx);
  const viewportHeightPx = finiteNonnegative(options.viewportHeightPx);
  const overscanPx = finiteNonnegative(options.overscanPx);
  const firstVisiblePositionPx = Math.max(0, scrollTopPx - overscanPx);
  const lastVisiblePositionPx = Math.min(
    layout.totalHeightPx,
    scrollTopPx + viewportHeightPx + overscanPx,
  );
  let firstRecordIndex = firstRecordAtOrAfter(layout, firstVisiblePositionPx);
  let lastRecordIndexExclusive = firstRecordAfter(layout, lastVisiblePositionPx);

  if (firstRecordIndex === layout.records.length) {
    firstRecordIndex = layout.records.length - 1;
  }
  if (lastRecordIndexExclusive <= firstRecordIndex) {
    lastRecordIndexExclusive = firstRecordIndex + 1;
  }

  const focusedIndex = focusedRecordIndex(layout, options.focusedRecordId);
  if (focusedIndex >= 0) {
    firstRecordIndex = Math.min(firstRecordIndex, focusedIndex);
    lastRecordIndexExclusive = Math.max(lastRecordIndexExclusive, focusedIndex + 1);
  }

  const topSpacerHeightPx = layout.recordStartsPx[firstRecordIndex]!;
  const bottomSpacerHeightPx =
    layout.totalHeightPx -
    layout.recordStartsPx[lastRecordIndexExclusive - 1]! -
    layout.recordHeightsPx[lastRecordIndexExclusive - 1]!;
  const records = layout.records.slice(firstRecordIndex, lastRecordIndexExclusive);

  return {
    records,
    firstRecordIndex,
    lastRecordIndexExclusive,
    topSpacerHeightPx,
    bottomSpacerHeightPx,
    totalHeightPx: layout.totalHeightPx,
  };
}

/** Computes a valid scroll position that places one record inside a window viewport. */
export function recordListWindowScrollTopForRecord<Row>(
  metrics: RecordListWindowMetrics<Row>,
  targetRecordId: string,
  viewportHeightPx: number,
  alignment: RecordListWindowScrollAlignment = "nearest",
  currentScrollTopPx: number = 0,
): number | undefined {
  const layout = recordListWindowLayout(metrics);
  const targetRecordIndex = layout.recordIds.indexOf(targetRecordId);
  if (targetRecordIndex < 0) return undefined;

  const viewportHeight = finiteNonnegative(viewportHeightPx);
  const currentScrollTop = finiteNonnegative(currentScrollTopPx);
  const targetStartPx = layout.recordStartsPx[targetRecordIndex]!;
  const targetEndPx = targetStartPx + layout.recordHeightsPx[targetRecordIndex]!;
  let requestedScrollTopPx = targetStartPx;

  if (alignment === "center") {
    requestedScrollTopPx =
      targetStartPx - (viewportHeight - layout.recordHeightsPx[targetRecordIndex]!) / 2;
  } else if (alignment === "end") {
    requestedScrollTopPx = targetEndPx - viewportHeight;
  } else if (alignment === "nearest") {
    const viewportEndPx = currentScrollTop + viewportHeight;
    if (targetStartPx < currentScrollTop) {
      requestedScrollTopPx = targetStartPx;
    } else if (targetEndPx > viewportEndPx) {
      requestedScrollTopPx = targetEndPx - viewportHeight;
    } else {
      requestedScrollTopPx = currentScrollTop;
    }
  }

  const maximumScrollTopPx = Math.max(0, layout.totalHeightPx - viewportHeight);
  const scrollTopPx = Math.min(Math.max(0, requestedScrollTopPx), maximumScrollTopPx);
  return scrollTopPx;
}
