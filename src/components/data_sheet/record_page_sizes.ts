/** Shared bounded page sizes for record queries and their controls. */
export const RECORD_PAGE_SIZES = [50, 100, 250] as const;
export type RecordPageSize = (typeof RECORD_PAGE_SIZES)[number];
