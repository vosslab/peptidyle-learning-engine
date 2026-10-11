// Browser-only fixtures for stable shared record behavior.
import { Show, createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";

import "../../src/browser_environment";
import type { RecordCollectionState } from "../../src/components/record_list/record_collection_state";
import { RecordList, type RecordContent } from "../../src/components/record_list/record_list";
import { RecordListImageBrowser } from "../../src/components/record_list/record_list_image_browser";
import { reorderedRecordListRows } from "../../src/components/record_list/record_list_reorder";
import { RecordSequence } from "../../src/components/record_list/record_sequence";

type DemoRecord = {
  readonly id: string;
  readonly title: string;
};
const PRIMITIVE_RECORDS: ReadonlyArray<DemoRecord> = [
  { id: "enzyme", title: "Enzyme kinetics" },
  { id: "genetics", title: "Genetics review" },
  { id: "proteins", title: "Protein structure" },
];
const SELECTION_RECORDS: ReadonlyArray<DemoRecord> = [
  { id: "question-dna", title: "DNA replication evidence" },
  { id: "avatar-amber", title: "Amber arch" },
  { id: "student-score", title: "Week 2 Coursework" },
];
const REORDER_RECORDS: ReadonlyArray<DemoRecord> = [
  { id: "alpha", title: "Alpha" },
  { id: "bravo", title: "Bravo" },
  { id: "charlie", title: "Charlie" },
];
function demoContent(record: DemoRecord): RecordContent {
  return { title: record.title, details: [], actions: [] };
}

function MetadataUpdateCase(): JSX.Element {
  const [records, setRecords] = createSignal([
    { id: "question-dna", title: "DNA replication evidence", status: "Initial metadata" },
  ]);
  const [draft, setDraft] = createSignal("Working interpretation");
  const [selectedIds, setSelectedIds] = createSignal<ReadonlySet<string>>(new Set());
  return (
    <section data-record-list-case="metadata-update">
      <h1>Record metadata update</h1>
      <button
        type="button"
        onClick={() =>
          setRecords((current) =>
            current.map((record) => ({ ...record, status: "Refreshed metadata" })),
          )
        }
      >
        Refresh same records
      </button>
      <RecordList
        rows={records()}
        content={(record) => ({
          title: record.title,
          details: [{ kind: "text", label: "Status", value: record.status }],
          actions: [],
        })}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Metadata update records"
        emptyState={{ title: "No records" }}
        selection={{
          kind: "checkbox",
          selectedIds,
          onChange: (record, selected) => setSelectedIds(new Set(selected ? [record.id] : [])),
        }}
        renderBody={() => (
          <label>
            Unsaved Question note
            <input value={draft()} onInput={(event) => setDraft(event.currentTarget.value)} />
          </label>
        )}
      />
    </section>
  );
}

function SelectionCase(): JSX.Element {
  const [selectedRadioIds, setSelectedRadioIds] = createSignal<ReadonlySet<string>>(
    new Set(["question-dna"]),
  );
  const [selectedCheckboxIds, setSelectedCheckboxIds] = createSignal<ReadonlySet<string>>(
    new Set(["student-score"]),
  );
  return (
    <section data-record-list-case="selection">
      <h1>Native record selection</h1>
      <RecordList
        rows={SELECTION_RECORDS}
        content={demoContent}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Single record selection"
        emptyState={{ title: "No selectable records" }}
        selection={{
          kind: "radio",
          selectedIds: selectedRadioIds,
          disabled: (record) => record.id === "avatar-amber",
          onChange: (record, selected) => {
            if (selected) setSelectedRadioIds(new Set([record.id]));
          },
        }}
      />
      <RecordList
        rows={SELECTION_RECORDS}
        content={demoContent}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Multiple record selection"
        emptyState={{ title: "No selectable records" }}
        selection={{
          kind: "checkbox",
          selectedIds: selectedCheckboxIds,
          disabled: (record) => record.id === "avatar-amber",
          onChange: (record, selected) => {
            setSelectedCheckboxIds((current) => {
              const next = new Set(current);
              if (selected) next.add(record.id);
              else next.delete(record.id);
              return next;
            });
          },
        }}
      />
    </section>
  );
}

function ImageBrowserCase(): JSX.Element {
  const records = [
    {
      id: "amber-arch",
      title: "Amber arch",
      description: "Amber arch with two blue dots.",
      imageUrl: "/assets/avatar_catalog/svg/amber-arch.svg",
    },
    {
      id: "fallback-arch",
      title: "Fallback arch",
      description: "The name and description remain available when its image cannot load.",
      // Invalid image bytes exercise the browser's real image failure without a network error.
      imageUrl: "data:image/png;base64,bm90YW5pbWFnZQ==",
    },
  ];
  const [selectedIds, setSelectedIds] = createSignal<ReadonlySet<string>>(new Set(["amber-arch"]));
  return (
    <section data-record-list-case="image-browser">
      <h1>Image browser</h1>
      <RecordListImageBrowser
        rows={records}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Image browser records"
        emptyState={{ title: "No image records" }}
        content={(record) => ({
          title: record.title,
          description: record.description,
          details: [],
          media: { src: record.imageUrl, alt: "" },
          actions: [],
        })}
        selection={{
          kind: "radio",
          selectedIds,
          onChange: (record, selected) => {
            if (selected) setSelectedIds(new Set([record.id]));
          },
        }}
      />
    </section>
  );
}

function SequenceReorderCase(props: { readonly rejectMove?: boolean }): JSX.Element {
  const [records, setRecords] = createSignal(REORDER_RECORDS);
  const [failureMessage, setFailureMessage] = createSignal<string>();
  const label = props.rejectMove ? "Rejected reordered records" : "Reorderable records";
  return (
    <section data-record-list-case={props.rejectMove ? "rejected-reorder" : "reorder"}>
      <h1>{label}</h1>
      <Show when={failureMessage()}>{(message) => <p role="alert">{message()}</p>}</Show>
      <RecordSequence
        rows={records()}
        content={demoContent}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel={label}
        emptyState={{ title: "No reorderable records" }}
        reorder={{
          onMove: (sourceIndex, destinationIndex): Promise<void> => {
            if (props.rejectMove) {
              setFailureMessage("The saved order was not updated.");
              return Promise.reject(new Error("The order save was rejected."));
            }
            setRecords((current) =>
              reorderedRecordListRows(current, sourceIndex, destinationIndex),
            );
            return Promise.resolve();
          },
          recordLabel: (record) => record.title,
          isDisabled: (record) => record.id === "bravo",
        }}
      />
    </section>
  );
}

function PrimitiveStateCase(props: {
  readonly id: "ready" | "empty" | "loading" | "error";
  readonly state: RecordCollectionState;
}): JSX.Element {
  const caseId = `primitive-${props.id}`;
  return (
    <section data-record-list-case={caseId}>
      <h1>{`Primitive ${props.state.kind}`}</h1>
      <RecordList
        rows={props.id === "ready" ? PRIMITIVE_RECORDS : []}
        content={demoContent}
        recordId={(record) => record.id}
        state={props.state}
        ariaLabel={`${caseId} records`}
        emptyState={{ title: "No primitive records" }}
      />
    </section>
  );
}

export function mountRecordListHarness(target: HTMLElement): void {
  render(
    () => (
      <div data-record-list-harness>
        <MetadataUpdateCase />
        <SelectionCase />
        <ImageBrowserCase />
        <SequenceReorderCase />
        <SequenceReorderCase rejectMove />
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
}
