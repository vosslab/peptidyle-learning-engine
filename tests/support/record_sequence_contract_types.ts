// Compile-time contract checks for the shared RecordSequence content boundary.

import type {
  RecordSequenceProps,
  RecordSequenceReorder,
} from "../../src/components/record_list/record_sequence";

type Row = { readonly id: string };

const shared = {
  rows: [] as const satisfies ReadonlyArray<Row>,
  recordId: (row: Row): string => row.id,
  state: { kind: "ready" } as const,
  ariaLabel: "Contract records",
  emptyState: { title: "No records" },
};

const semantic: RecordSequenceProps<Row> = {
  ...shared,
  content: () => ({ title: "Record", details: [], actions: [] }),
};

const reorder: RecordSequenceReorder<Row> = {
  onMove: (): void => undefined,
  recordLabel: (row: Row): string => row.id,
  isDisabled: (): boolean => false,
};

// @ts-expect-error RecordSequence requires semantic content.
const missingContent: RecordSequenceProps<Row> = shared;

void [semantic, reorder, missingContent];
