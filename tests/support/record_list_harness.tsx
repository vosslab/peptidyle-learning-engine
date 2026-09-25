// record_list_harness.tsx - browser-only composition of the shared RecordList APIs.

import { For, createMemo, createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";

import "../../src/browser_environment";

import { RecordList } from "../../src/components/record_list/record_list";
import type { RecordCollectionState } from "../../src/components/record_list/record_collection_state";
import { RecordDetailList } from "../../src/components/record_list/record_detail_list";
import {
  RecordOutlineItem,
  RecordOutlineList,
} from "../../src/components/record_list/record_outline_list";
import {
  createRecordListPresentation,
  recordListPresentationSwitcher,
} from "../../src/components/record_list/record_list_presentation";
import {
  RecordListReorder,
  RecordListReorderControls,
  reorderedRecordListRows,
} from "../../src/components/record_list/record_list_reorder";
import { RecordSequence } from "../../src/components/record_list/record_sequence";
import { RecordTable } from "../../src/components/record_list/record_table";
import {
  recordListWindow,
  recordListWindowScrollTopForRecord,
} from "../../src/components/record_list/record_list_window";
import type { RecordRegion } from "../../src/components/record_list/region_spec";

type DemoRecord = {
  readonly id: string;
  readonly label: string;
  readonly heightPx: number;
};

type AlignmentRecord = {
  readonly id: string;
  readonly title: string;
  readonly metadata: string;
  readonly status: string;
  readonly note: string;
};

type RecordListHarness = {
  readonly setWindowFocusedRecord: (recordId: string | undefined) => void;
  readonly setWindowScrollTop: (scrollTopPx: number) => void;
};

const PRESENTATION_RECORDS: ReadonlyArray<DemoRecord> = [
  { id: "enzyme", label: "Enzyme kinetics", heightPx: 48 },
  { id: "genetics", label: "Genetics review", heightPx: 48 },
  { id: "proteins", label: "Protein structure", heightPx: 48 },
];

const REORDER_RECORDS: ReadonlyArray<DemoRecord> = [
  { id: "alpha", label: "Alpha", heightPx: 48 },
  { id: "bravo", label: "Bravo", heightPx: 48 },
  { id: "charlie", label: "Charlie", heightPx: 48 },
];

const WINDOW_RECORDS: ReadonlyArray<DemoRecord> = [
  { id: "alpha", label: "Alpha", heightPx: 32 },
  { id: "bravo", label: "Bravo", heightPx: 48 },
  { id: "charlie", label: "Charlie", heightPx: 64 },
  { id: "delta", label: "Delta", heightPx: 80 },
  { id: "echo", label: "Echo", heightPx: 96 },
  { id: "foxtrot", label: "Foxtrot", heightPx: 112 },
];

const WINDOW_HEIGHTS = new Map(WINDOW_RECORDS.map((record) => [record.id, record.heightPx]));

const ALIGNMENT_RECORDS: ReadonlyArray<AlignmentRecord> = [
  { id: "brief", title: "DNA", metadata: "Week 1", status: "Ready", note: "One item" },
  {
    id: "detailed",
    title: "Genome-wide association study preparation",
    metadata: "Week 12 independent reading",
    status: "Needs review",
    note: "A deliberately longer supplemental note",
  },
];

const ALIGNMENT_REGIONS: ReadonlyArray<RecordRegion<AlignmentRecord>> = [
  {
    id: "identity",
    role: "identity",
    priority: "required",
    width: "minmax(0, 1fr)",
    align: "start",
    content: (record) => <span>{record.title}</span>,
  },
  {
    id: "metadata",
    role: "metadata",
    priority: "high",
    width: "minmax(8rem, 12rem)",
    align: "start",
    content: (record) => <span>{record.metadata}</span>,
  },
  {
    id: "status",
    role: "status",
    priority: "medium",
    width: "minmax(7rem, 10rem)",
    align: "end",
    content: (record) => <span>{record.status}</span>,
  },
  {
    id: "note",
    role: "metadata",
    priority: "low",
    width: "minmax(8rem, 13rem)",
    align: "start",
    content: (record) => <span>{record.note}</span>,
  },
  {
    id: "actions",
    role: "actions",
    priority: "required",
    width: "max-content",
    align: "end",
    content: (record) => <button type="button">Open {record.title}</button>,
  },
];

const DETAIL_RECORDS = [
  { id: "attempt-one", title: "Question 1", feedback: "Explain why the DNA sequence changes." },
  { id: "attempt-two", title: "Question 2", feedback: "Compare the two protein structures." },
] as const;

function AlignmentCase(): JSX.Element {
  return (
    <section data-record-list-case="alignment">
      <h1>Record alignment</h1>
      <RecordList
        rows={ALIGNMENT_RECORDS}
        regions={ALIGNMENT_REGIONS}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Alignment records"
        emptyState={{ title: "No records" }}
      />
    </section>
  );
}

function recordRegions(
  selection: () => string | undefined,
  selectRecord: (recordId: string) => void,
): ReadonlyArray<RecordRegion<DemoRecord>> {
  return [
    {
      id: "identity",
      role: "identity",
      priority: "required",
      width: "minmax(14rem, 1fr)",
      align: "start",
      content: (record: DemoRecord): JSX.Element => (
        <span data-record-appearance={selection() === record.id ? "selected" : "ordinary"}>
          {record.label}
        </span>
      ),
    },
    {
      id: "actions",
      role: "actions",
      priority: "required",
      width: "max-content",
      align: "end",
      content: (record: DemoRecord) => (
        <button type="button" onClick={() => selectRecord(record.id)}>
          Select {record.label}
        </button>
      ),
    },
  ];
}

function PresentationCase(props: {
  readonly id: string;
  readonly variants: "one" | "two";
}): JSX.Element {
  const [selectedRecordId, setSelectedRecordId] = createSignal<string>("genetics");
  const presentation = createRecordListPresentation({
    variants:
      props.variants === "one"
        ? [{ id: "compact", label: "Compact" }]
        : [
            { id: "table", label: "Table" },
            { id: "cards", label: "Cards" },
          ],
  });
  const switcher = recordListPresentationSwitcher(presentation, "Presentation choices");
  const regions = recordRegions(selectedRecordId, setSelectedRecordId);

  return (
    <section data-record-list-case={props.id}>
      <h1>{props.id === "one-variant" ? "One presentation" : "Two presentations"}</h1>
      {switcher === undefined ? undefined : (
        <div role="group" aria-label={switcher.ariaLabel}>
          <For each={switcher.variants}>
            {(variant) => (
              <button
                type="button"
                aria-pressed={switcher.selectedVariant() === variant.id}
                onClick={() => switcher.selectVariant(variant.id)}
              >
                {variant.label}
              </button>
            )}
          </For>
        </div>
      )}
      <p data-record-list-selection={selectedRecordId()}>
        Selected: {PRESENTATION_RECORDS.find((record) => record.id === selectedRecordId())?.label}
      </p>
      <section data-record-list-presentation={presentation.variant()}>
        <RecordList
          rows={PRESENTATION_RECORDS}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel={`${props.id} records`}
          emptyState={{ title: "No records" }}
        />
      </section>
    </section>
  );
}

function ReorderCase(): JSX.Element {
  const [records, setRecords] = createSignal(REORDER_RECORDS);
  const [selectedRecordId, setSelectedRecordId] = createSignal<string>();
  const regions = [
    ...recordRegions(selectedRecordId, setSelectedRecordId).slice(0, 1),
    {
      id: "actions",
      role: "actions" as const,
      priority: "required" as const,
      width: "max-content",
      align: "end" as const,
      content: (record: DemoRecord): JSX.Element => (
        <RecordListReorderControls
          recordId={record.id}
          recordLabel={record.label}
          index={() => records().findIndex((candidate) => candidate.id === record.id)}
          count={() => records().length}
          disabled={false}
        />
      ),
    },
  ] satisfies ReadonlyArray<RecordRegion<DemoRecord>>;

  return (
    <section data-record-list-case="reorder">
      <h1>Reorder records</h1>
      <RecordListReorder
        onMove={(sourceIndex, destinationIndex) => {
          setRecords((currentRecords) =>
            reorderedRecordListRows(currentRecords, sourceIndex, destinationIndex),
          );
        }}
      >
        <RecordList
          rows={records()}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="Reorderable records"
          emptyState={{ title: "No records" }}
        />
      </RecordListReorder>
    </section>
  );
}

function PrimitiveStateCase(props: {
  readonly id: "ready" | "empty" | "loading" | "error";
  readonly state: RecordCollectionState;
}): JSX.Element {
  const state = (): RecordCollectionState => props.state;
  const rows = (): ReadonlyArray<DemoRecord> => (props.id === "ready" ? PRESENTATION_RECORDS : []);
  const [selectedRecordId, setSelectedRecordId] = createSignal<string>();
  const caseId = `primitive-${props.id}`;
  return (
    <section data-record-list-case={caseId}>
      <h1>{`Primitive ${props.state.kind}`}</h1>
      <RecordList
        rows={rows()}
        regions={recordRegions(selectedRecordId, setSelectedRecordId)}
        recordId={(record) => record.id}
        state={state()}
        ariaLabel={`${caseId} records`}
        emptyState={{ title: "No primitive records" }}
      />
    </section>
  );
}

function FamilyComponentsCase(): JSX.Element {
  const [selectedRecordId, setSelectedRecordId] = createSignal<string>();
  const regions = recordRegions(selectedRecordId, setSelectedRecordId);

  return (
    <section data-record-list-case="record-family">
      <h1>Record family</h1>
      <button type="button" data-record-family-tab-start>
        Start keyboard traversal
      </button>
      <section data-record-family-case="sequence">
        <h2>Ordered records</h2>
        <RecordSequence
          rows={PRESENTATION_RECORDS}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="Ordered course records"
          emptyState={{ title: "No ordered records" }}
        />
      </section>
      <section data-record-family-case="sequence-empty">
        <RecordSequence
          rows={[]}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="Empty ordered course records"
          emptyState={{ title: "No ordered records" }}
        />
      </section>
      <section data-record-family-case="sequence-loading">
        <RecordSequence
          rows={[]}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "loading", label: "Loading ordered course records..." }}
          ariaLabel="Loading ordered course records"
          emptyState={{ title: "No ordered records" }}
        />
      </section>
      <section data-record-family-case="sequence-error">
        <RecordSequence
          rows={[]}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "error", message: "Ordered course records are unavailable." }}
          ariaLabel="Unavailable ordered course records"
          emptyState={{ title: "No ordered records" }}
        />
      </section>
      <section data-record-family-case="table">
        <h2>Course roster</h2>
        <RecordTable
          rows={ALIGNMENT_RECORDS}
          rowId={(record) => record.id}
          rowHeader={{
            id: "student",
            header: "Student",
            width: "30%",
            content: (record) => record.title,
          }}
          columns={[
            { id: "week", header: "Week", width: "35%", cell: (record) => record.metadata },
            {
              id: "action",
              header: "Action",
              width: "35%",
              align: "end",
              cell: (record) => <button type="button">Open {record.title}</button>,
            },
          ]}
          state={{ kind: "ready" }}
          ariaLabel="Course roster records"
          emptyState={{ title: "No roster records" }}
        />
      </section>
      <section data-record-family-case="outline">
        <h2>Course outline</h2>
        <RecordOutlineList
          state={{ kind: "ready" }}
          isEmpty={false}
          ariaLabel="Course modules"
          emptyState={{ title: "No modules" }}
        >
          <RecordOutlineItem recordId="module-one">
            <strong>Module one</strong>
            <RecordOutlineList
              state={{ kind: "ready" }}
              isEmpty={false}
              ariaLabel="Module one assessments"
              emptyState={{ title: "No assessments" }}
            >
              <RecordOutlineItem recordId="assessment-one">
                <button type="button">Open assessment one</button>
              </RecordOutlineItem>
            </RecordOutlineList>
          </RecordOutlineItem>
        </RecordOutlineList>
      </section>
      <section data-record-family-case="detail">
        <h2>Attempt review</h2>
        <RecordDetailList
          rows={DETAIL_RECORDS}
          recordId={(record) => record.id}
          renderRecord={(record) => (
            <>
              <h3>{record.title}</h3>
              <p>{record.feedback}</p>
              <button type="button">Review {record.title}</button>
            </>
          )}
          state={{ kind: "ready" }}
          ariaLabel="Attempt review records"
          emptyState={{ title: "No review records" }}
        />
      </section>
    </section>
  );
}

export function mountRecordListHarness(target: HTMLElement): RecordListHarness {
  let setWindowFocusedRecord: (recordId: string | undefined) => void = () => undefined;
  let setWindowScrollTop: (scrollTopPx: number) => void = () => undefined;

  function ControlledWindowCase(): JSX.Element {
    const [scrollTopPx, updateScrollTopPx] = createSignal(50);
    const [focusedRecordId, updateFocusedRecordId] = createSignal<string>();
    const [selectedRecordId, setSelectedRecordId] = createSignal<string>();
    const viewportHeightPx = 80;
    setWindowFocusedRecord = updateFocusedRecordId;
    setWindowScrollTop = updateScrollTopPx;
    const windowedRecords = createMemo(() =>
      recordListWindow({
        records: WINDOW_RECORDS,
        recordId: (record) => record.id,
        estimatedRecordHeightPx: 50,
        measuredRecordHeightsPx: WINDOW_HEIGHTS,
        scrollTopPx: scrollTopPx(),
        viewportHeightPx,
        overscanPx: 0,
        focusedRecordId: focusedRecordId(),
      }),
    );
    const regions = recordRegions(selectedRecordId, setSelectedRecordId);

    function scrollToRecord(recordId: string): void {
      const nextScrollTopPx = recordListWindowScrollTopForRecord(
        {
          records: WINDOW_RECORDS,
          recordId: (record) => record.id,
          estimatedRecordHeightPx: 50,
          measuredRecordHeightsPx: WINDOW_HEIGHTS,
        },
        recordId,
        viewportHeightPx,
        "start",
        scrollTopPx(),
      );
      if (nextScrollTopPx !== undefined) updateScrollTopPx(nextScrollTopPx);
    }

    return (
      <section data-record-list-case="window">
        <h1>Windowed records</h1>
        <RecordList
          rows={WINDOW_RECORDS}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="All matched records"
          emptyState={{ title: "No records" }}
        />
        <button type="button" onClick={() => scrollToRecord("foxtrot")}>
          Scroll to Foxtrot
        </button>
        <p data-record-list-window-scroll-top={scrollTopPx()}>Scroll top: {scrollTopPx()}</p>
        <div
          data-record-list-window-spacer="top"
          style={{ height: `${windowedRecords().topSpacerHeightPx}px` }}
        />
        <RecordList
          rows={windowedRecords().records}
          regions={regions}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="Windowed records"
          emptyState={{ title: "No records" }}
        />
        <div
          data-record-list-window-spacer="bottom"
          style={{ height: `${windowedRecords().bottomSpacerHeightPx}px` }}
        />
        <p data-record-list-window-total-height={windowedRecords().totalHeightPx}>
          Total height: {windowedRecords().totalHeightPx}
        </p>
      </section>
    );
  }

  render(
    () => (
      <div data-record-list-harness>
        <AlignmentCase />
        <PresentationCase id="one-variant" variants="one" />
        <PresentationCase id="two-variants" variants="two" />
        <ReorderCase />
        <ControlledWindowCase />
        <FamilyComponentsCase />
        <PrimitiveStateCase id="ready" state={{ kind: "ready" }} />
        <PrimitiveStateCase id="empty" state={{ kind: "ready" }} />
        <PrimitiveStateCase
          id="loading"
          state={{ kind: "loading", label: "Loading primitive records..." }}
        />
        <PrimitiveStateCase
          id="error"
          state={{ kind: "error", message: "Primitive records are unavailable." }}
        />
      </div>
    ),
    target,
  );
  return { setWindowFocusedRecord, setWindowScrollTop };
}
