// Compile-time contract checks for the shared RecordList content boundary.

import type { RecordFact, RecordListProps } from "../../src/components/record_list/record_list";

type Row = { readonly id: string };

const shared = {
  rows: [] as const satisfies ReadonlyArray<Row>,
  recordId: (row: Row): string => row.id,
  state: { kind: "ready" } as const,
  ariaLabel: "Contract records",
  emptyState: { title: "No records" },
};

const semantic: RecordListProps<Row> = {
  ...shared,
  content: () => ({ title: "Record", details: [], actions: [] }),
};

const courseClassificationFact: RecordFact = {
  kind: "courseClassification",
  value: {
    disciplineUuid: "discipline-molecular-biology",
    subjectUuid: null,
    topicUuid: null,
    subtopicUuid: null,
    tags: [],
  },
};

// @ts-expect-error RecordList requires semantic content.
const missingContent: RecordListProps<Row> = shared;

void [semantic, courseClassificationFact, missingContent];
