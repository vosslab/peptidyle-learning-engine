import type { JSX } from "solid-js";

/** Semantic purpose of one consistently aligned record region. */
export type RecordRegionRole = "identity" | "metadata" | "status" | "actions";

/** Horizontal alignment within a region's shared grid track. */
export type RecordRegionAlign = "start" | "center" | "end" | "stretch";

/**
 * Narrow-screen retention order. Required regions remain visible; lower priorities are removed
 * before higher priorities as the available inline space decreases.
 */
export type RecordRegionPriority = "required" | "high" | "medium" | "low";

type RecordRegionBase<Row> = {
  readonly id: string;
  readonly width: string;
  readonly align: RecordRegionAlign;
  /** Optional visible heading; row content remains responsible for accessible region names. */
  readonly header?: JSX.Element;
  readonly content: (row: Row) => JSX.Element;
};

type RetainedRecordRegion<Row> = RecordRegionBase<Row> & {
  readonly role: "identity" | "actions";
  readonly priority: "required";
};

type SupplementalRecordRegion<Row> = RecordRegionBase<Row> & {
  readonly role: "metadata" | "status";
  readonly priority: RecordRegionPriority;
};

/**
 * One semantic region shared by every row in a RecordList.
 *
 * Identity and actions are always retained so a record and its primary action survive narrow
 * screens. Metadata and status can opt into a lower responsive priority.
 */
export type RecordRegion<Row> = RetainedRecordRegion<Row> | SupplementalRecordRegion<Row>;
