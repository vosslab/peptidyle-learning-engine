// record_list_harness.tsx - browser-only composition of the shared RecordList APIs.
import { ErrorBoundary, For, Show, createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";
import "../../src/browser_environment";
import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { ContentClassificationItem } from "../../src/api/content_classification";
import { RecordList, type RecordContent } from "../../src/components/record_list/record_list";
import { RecordListImageBrowser } from "../../src/components/record_list/record_list_image_browser";
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
import { PROVIDED_AVATAR_CATALOG } from "../../src/features/profile_avatar/avatar_catalog_generated";
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
type SemanticRecord = {
  readonly id: string;
  readonly title: string;
  readonly description: string;
  readonly status: string;
  readonly editDisabled: boolean;
};
type ImageBrowserRecord = {
  readonly id: string;
  readonly title: string;
  readonly description: string;
  readonly imageUrl: string;
};
type RecordListHarness = {
  readonly refreshSemanticSequence: () => void;
  readonly refreshSemanticRecords: () => void;
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
function alignmentContent(record: AlignmentRecord): RecordContent {
  return {
    title: record.title,
    details: [
      { kind: "text", label: "Metadata", value: record.metadata },
      { kind: "text", label: "Status", value: record.status },
      { kind: "text", label: "Note", value: record.note },
    ],
    actions: [
      {
        id: "open",
        kind: "command",
        label: `Open ${record.title}`,
        onClick: (): void => undefined,
      },
    ],
  };
}
const DETAIL_RECORDS = [
  { id: "attempt-one", title: "Question 1", feedback: "Explain why the DNA sequence changes." },
  { id: "attempt-two", title: "Question 2", feedback: "Compare the two protein structures." },
] as const;
const SEMANTIC_AVATAR = PROVIDED_AVATAR_CATALOG[0];
const CLASSIFICATION_DISCIPLINE_UUID = "discipline-molecular-biology";
const CLASSIFICATION_SUBJECT_UUID = "subject-nucleic-acids";
const CLASSIFICATION_API = {
  client: {
    listDisciplinesIncludingRetired: (): Promise<ReadonlyArray<ContentClassificationItem>> =>
      Promise.resolve([
        {
          uuid: CLASSIFICATION_DISCIPLINE_UUID,
          name: "Molecular Biology",
          isRetired: true,
        },
      ]),
    listSubjects: (): Promise<ReadonlyArray<ContentClassificationItem>> =>
      Promise.resolve([
        { uuid: CLASSIFICATION_SUBJECT_UUID, name: "Nucleic acids", isRetired: false },
      ]),
    listTopics: (): Promise<ReadonlyArray<ContentClassificationItem>> => Promise.resolve([]),
    listSubtopics: (): Promise<ReadonlyArray<ContentClassificationItem>> => Promise.resolve([]),
  },
  queries: {},
} as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
const SEMANTIC_RECORDS: ReadonlyArray<SemanticRecord> = [
  {
    id: "question-dna",
    title: "DNA replication evidence",
    description: "Compare the molecular observations before choosing an explanation.",
    status: "Initial metadata",
    editDisabled: false,
  },
  {
    id: "avatar-amber",
    title: SEMANTIC_AVATAR.name,
    description: SEMANTIC_AVATAR.description,
    status: "Available avatar",
    editDisabled: false,
  },
  {
    id: "student-score",
    title: "Week 2 Coursework",
    description: "Your submitted work is ready for review.",
    status: "Score not released",
    editDisabled: false,
  },
];
const IMAGE_BROWSER_RECORDS: ReadonlyArray<ImageBrowserRecord> = [
  {
    id: "amber-arch",
    title: SEMANTIC_AVATAR.name,
    description: SEMANTIC_AVATAR.description,
    imageUrl: SEMANTIC_AVATAR.assetPath,
  },
  {
    id: "fallback-arch",
    title: "Fallback arch",
    description: "The name and description remain available when its image cannot load.",
    imageUrl: SEMANTIC_AVATAR.assetPath,
  },
];
function SemanticContentCase(props: {
  readonly onRefreshReady: (refresh: () => void) => void;
}): JSX.Element {
  const [records, setRecords] = createSignal(SEMANTIC_RECORDS);
  const [state, setState] = createSignal<RecordCollectionState>({ kind: "ready" });
  const [draft, setDraft] = createSignal("Working interpretation");
  function refreshRecords(): void {
    setRecords((current) =>
      current.map((record) =>
        record.id === "question-dna"
          ? {
              ...record,
              description: "Fresh metadata from the same Question record.",
              status: "Refreshed metadata",
              editDisabled: true,
            }
          : { ...record },
      ),
    );
  }
  props.onRefreshReady(refreshRecords);
  function content(record: SemanticRecord): RecordContent {
    const details: Array<RecordContent["details"][number]> = [
      { kind: "text", label: "Status", value: record.status },
    ];
    if (record.id === "question-dna") {
      details.push(
        { kind: "assessmentType" as const, value: "quiz" },
        {
          kind: "time" as const,
          dateTime: "2026-09-25T14:30:00.000Z",
          value: "September 25, 2026, 9:30 AM",
        },
        { kind: "questionId" as const, questionTitle: record.title, displayId: "7K3M-79QP" },
        {
          kind: "link" as const,
          label: "Course: Molecular Biology",
          href: "/courses/molecular-biology",
        },
        {
          kind: "courseClassification" as const,
          value: {
            disciplineUuid: CLASSIFICATION_DISCIPLINE_UUID,
            subjectUuid: CLASSIFICATION_SUBJECT_UUID,
            topicUuid: null,
            subtopicUuid: null,
            tags: ["replication", "evidence"],
          },
        },
      );
    }
    return {
      title: record.title,
      description: record.description,
      details,
      media:
        record.id === "avatar-amber"
          ? {
              src: SEMANTIC_AVATAR.assetPath,
              alt: `${SEMANTIC_AVATAR.name}: ${SEMANTIC_AVATAR.description}`,
            }
          : undefined,
      actions:
        record.id === "question-dna"
          ? [
              {
                id: "open",
                kind: "link",
                label: "Open",
                href: "/library/7K3M-79QP",
                primary: true,
              },
              {
                id: "template",
                kind: "command",
                label: "Use template",
                pressed: true,
                disabled: record.editDisabled,
                onClick: () => undefined,
              },
              {
                id: "forks",
                kind: "command",
                label: "Known Forks",
                expanded: true,
                controls: "semantic-known-forks",
                onClick: () => undefined,
              },
            ]
          : [{ id: "open", kind: "link", label: "Open", href: "/" }],
    };
  }
  return (
    <section data-record-list-case="semantic-content">
      <h1>Semantic records</h1>
      <button type="button" onClick={refreshRecords}>
        Refresh same records
      </button>
      <button
        type="button"
        onClick={() => setState({ kind: "loading", label: "Refreshing records..." })}
      >
        Show retained loading
      </button>
      <button
        type="button"
        onClick={() =>
          setState({
            kind: "error",
            message: "The latest refresh did not finish.",
            retry: () => setState({ kind: "ready" }),
          })
        }
      >
        Show retained error
      </button>
      <RecordList
        rows={records()}
        content={content}
        recordId={(record) => record.id}
        state={state()}
        ariaLabel="Semantic content records"
        emptyState={{ title: "No semantic records" }}
        renderBody={(row) => (
          <Show when={row().id === "question-dna"}>
            <label>
              Unsaved Question note
              <input value={draft()} onInput={(event) => setDraft(event.currentTarget.value)} />
            </label>
            <p id="semantic-known-forks">Known Forks are expanded for this Question.</p>
          </Show>
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
  const content = (record: SemanticRecord): RecordContent => ({
    title: record.title,
    description: record.description,
    details: [{ kind: "text", label: "Status", value: record.status }],
    actions: [{ id: "open", kind: "link", label: "Open", href: "/" }],
  });

  return (
    <section data-record-list-case="selection">
      <h1>Native record selection</h1>
      <RecordList
        rows={SEMANTIC_RECORDS}
        content={content}
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
        rows={SEMANTIC_RECORDS}
        content={content}
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
  const [selectedIds, setSelectedIds] = createSignal<ReadonlySet<string>>(new Set(["amber-arch"]));

  return (
    <section data-record-list-case="image-browser">
      <h1>Image browser</h1>
      <RecordListImageBrowser
        rows={IMAGE_BROWSER_RECORDS}
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

function SemanticSequenceCase(props: {
  readonly onRefreshReady: (refresh: () => void) => void;
}): JSX.Element {
  const [records, setRecords] = createSignal(SEMANTIC_RECORDS.slice(0, 2));
  const [state, setState] = createSignal<RecordCollectionState>({ kind: "ready" });
  const [draft, setDraft] = createSignal("Sequence draft");
  function refreshRecords(): void {
    setRecords((current) =>
      current.map((record) =>
        record.id === "question-dna"
          ? {
              ...record,
              description: "Fresh ordered metadata from the same Question record.",
              status: "Refreshed ordered metadata",
              editDisabled: true,
            }
          : { ...record },
      ),
    );
  }
  props.onRefreshReady(refreshRecords);
  function content(record: SemanticRecord): RecordContent {
    return {
      title: record.title,
      description: record.description,
      details: [{ kind: "text", label: "Status", value: record.status }],
      actions: [
        {
          id: "inspect",
          kind: "command",
          label: "Inspect",
          disabled: record.editDisabled,
          onClick: (): void => undefined,
        },
      ],
    };
  }
  return (
    <section data-record-list-case="semantic-sequence">
      <h1>Semantic ordered records</h1>
      <button
        type="button"
        onClick={() => setState({ kind: "loading", label: "Refreshing order..." })}
      >
        Show retained sequence loading
      </button>
      <button
        type="button"
        onClick={() =>
          setState({
            kind: "error",
            message: "The ordered refresh did not finish.",
            retry: () => setState({ kind: "ready" }),
          })
        }
      >
        Show retained sequence error
      </button>
      <RecordSequence
        rows={records()}
        content={content}
        recordId={(record) => record.id}
        state={state()}
        ariaLabel="Semantic ordered records"
        emptyState={{ title: "No semantic ordered records" }}
        renderBody={(row) => (
          <Show when={row().id === "question-dna"}>
            <label>
              Unsaved ordered note
              <input value={draft()} onInput={(event) => setDraft(event.currentTarget.value)} />
            </label>
          </Show>
        )}
      />
      <RecordSequence
        rows={[]}
        content={content}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Empty semantic ordered records"
        emptyState={{ title: "No semantic ordered records" }}
      />
    </section>
  );
}
function SequenceReorderCase(): JSX.Element {
  const [localRecords, setLocalRecords] = createSignal(REORDER_RECORDS);
  const [asynchronousRecords, setAsynchronousRecords] = createSignal(REORDER_RECORDS);
  const [failedRecords] = createSignal(REORDER_RECORDS);
  const [unchangedRecords] = createSignal(REORDER_RECORDS);
  const [failureMessage, setFailureMessage] = createSignal<string>();
  function content(record: DemoRecord): RecordContent {
    return {
      title: record.label,
      details: [{ kind: "text", label: "Height", value: `${record.heightPx}px` }],
      actions: [],
    };
  }
  return (
    <section data-record-list-case="sequence-reorder">
      <h1>Controlled sequence movement</h1>
      <RecordSequence
        rows={localRecords()}
        content={content}
        recordId={(record): string => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Locally reordered records"
        emptyState={{ title: "No locally reordered records" }}
        reorder={{
          onMove: (sourceIndex, destinationIndex): void => {
            setLocalRecords((records) =>
              reorderedRecordListRows(records, sourceIndex, destinationIndex),
            );
          },
          recordLabel: (record): string => record.label,
          isDisabled: (record): boolean => record.id === "bravo",
        }}
      />
      <RecordSequence
        rows={asynchronousRecords()}
        content={content}
        recordId={(record): string => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Asynchronously reordered records"
        emptyState={{ title: "No asynchronously reordered records" }}
        reorder={{
          onMove: async (sourceIndex, destinationIndex): Promise<void> => {
            await Promise.resolve();
            setAsynchronousRecords((records) =>
              reorderedRecordListRows(records, sourceIndex, destinationIndex),
            );
          },
          recordLabel: (record): string => record.label,
        }}
      />
      <Show when={failureMessage()}>{(message) => <p role="alert">{message()}</p>}</Show>
      <RecordSequence
        rows={failedRecords()}
        content={content}
        recordId={(record): string => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Failed reordered records"
        emptyState={{ title: "No failed reordered records" }}
        reorder={{
          onMove: (): Promise<void> => {
            setFailureMessage("The saved order was not updated.");
            return Promise.reject(new Error("reorder failed"));
          },
          recordLabel: (record): string => record.label,
        }}
      />
      <RecordSequence
        rows={unchangedRecords()}
        content={content}
        recordId={(record): string => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Unchanged reordered records"
        emptyState={{ title: "No unchanged reordered records" }}
        reorder={{
          onMove: (): Promise<void> => Promise.resolve(),
          recordLabel: (record): string => record.label,
        }}
      />
    </section>
  );
}
type OutlineReorderMode = "synchronous" | "asynchronous" | "rejected" | "unchanged";
function OutlineReorderList(props: {
  readonly ariaLabel: string;
  readonly mode: OutlineReorderMode;
}): JSX.Element {
  const [modules, setModules] = createSignal(REORDER_RECORDS);
  function moveModules(sourceIndex: number, destinationIndex: number): void {
    setModules((records) => reorderedRecordListRows(records, sourceIndex, destinationIndex));
  }
  function onMove(sourceIndex: number, destinationIndex: number): void | Promise<void> {
    if (props.mode === "rejected") return Promise.reject(new Error("Module order was rejected."));
    if (props.mode === "unchanged") return Promise.resolve();
    if (props.mode === "asynchronous") {
      return new Promise<void>((resolve) => {
        queueMicrotask(() => {
          moveModules(sourceIndex, destinationIndex);
          resolve();
        });
      });
    }
    moveModules(sourceIndex, destinationIndex);
  }
  return (
    <RecordOutlineList
      state={{ kind: "ready" }}
      isEmpty={modules().length === 0}
      ariaLabel={props.ariaLabel}
      emptyState={{ title: "No fork Modules" }}
      reorder={{
        recordIds: () => modules().map((module) => module.id),
        onMove,
        recordLabel: (recordId): string =>
          modules().find((module) => module.id === recordId)?.label ?? recordId,
        isDisabled: (recordId): boolean => recordId === "bravo",
      }}
    >
      <For each={modules()}>
        {(module) => (
          <RecordOutlineItem recordId={module.id}>
            <strong>{module.label} Module</strong>
            <RecordOutlineList
              state={{ kind: "ready" }}
              isEmpty={false}
              ariaLabel={`${module.label} Module Assessments`}
              emptyState={{ title: "No Assessments" }}
            >
              <RecordOutlineItem recordId={`assessment-${module.id}`}>
                {module.label} Assessment
              </RecordOutlineItem>
            </RecordOutlineList>
          </RecordOutlineItem>
        )}
      </For>
    </RecordOutlineList>
  );
}
function OutlineReorderCase(): JSX.Element {
  return (
    <section data-record-list-case="outline-reorder">
      <h1>Controlled fork Modules</h1>
      <OutlineReorderList ariaLabel="Reordered fork Modules" mode="synchronous" />
      <OutlineReorderList ariaLabel="Asynchronously reordered fork Modules" mode="asynchronous" />
      <OutlineReorderList ariaLabel="Rejected reordered fork Modules" mode="rejected" />
      <OutlineReorderList ariaLabel="Unchanged reordered fork Modules" mode="unchanged" />
    </section>
  );
}
function DuplicateActionIdCase(): JSX.Element {
  const [duplicates, setDuplicates] = createSignal(false);
  const row = { id: "duplicate-actions", title: "Duplicate action check" };
  const action = {
    id: "open",
    kind: "command" as const,
    label: "Open",
    onClick: (): void => undefined,
  };

  return (
    <section data-record-list-case="duplicate-action-id">
      <h1>Duplicate action ID check</h1>
      <button type="button" onClick={() => setDuplicates(true)}>
        Render duplicate action IDs
      </button>
      <ErrorBoundary
        fallback={(error) => (
          <p role="alert">{error instanceof Error ? error.message : String(error)}</p>
        )}
      >
        <RecordList
          rows={[row]}
          content={() => ({
            title: row.title,
            details: [],
            actions: duplicates() ? [action, action] : [action],
          })}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="Duplicate action check records"
          emptyState={{ title: "No duplicate action check records" }}
        />
      </ErrorBoundary>
    </section>
  );
}

function AlignmentCase(): JSX.Element {
  return (
    <section data-record-list-case="alignment">
      <h1>Record alignment</h1>
      <RecordList
        rows={ALIGNMENT_RECORDS}
        content={alignmentContent}
        recordId={(record) => record.id}
        state={{ kind: "ready" }}
        ariaLabel="Alignment records"
        emptyState={{ title: "No records" }}
      />
    </section>
  );
}

function demoContent(
  selection: () => string | undefined,
  selectRecord: (recordId: string) => void,
): (record: DemoRecord) => RecordContent {
  return (record) => ({
    title: record.label,
    description: selection() === record.id ? "Selected" : "Ordinary",
    details: [],
    actions: [
      {
        id: "select",
        kind: "command",
        label: `Select ${record.label}`,
        pressed: selection() === record.id,
        onClick: (): void => selectRecord(record.id),
      },
    ],
  });
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
  const content = demoContent(selectedRecordId, setSelectedRecordId);

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
          content={content}
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
          content={(record) => ({ title: record.label, details: [], actions: [] })}
          renderBody={(row) => (
            <RecordListReorderControls
              recordId={row().id}
              recordLabel={row().label}
              index={() => records().findIndex((candidate) => candidate.id === row().id)}
              count={() => records().length}
              disabled={false}
            />
          )}
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
        content={demoContent(selectedRecordId, setSelectedRecordId)}
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
  const content = demoContent(selectedRecordId, setSelectedRecordId);

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
          content={content}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="Ordered course records"
          emptyState={{ title: "No ordered records" }}
        />
      </section>
      <section data-record-family-case="sequence-empty">
        <RecordSequence
          rows={[]}
          content={content}
          recordId={(record) => record.id}
          state={{ kind: "ready" }}
          ariaLabel="Empty ordered course records"
          emptyState={{ title: "No ordered records" }}
        />
      </section>
      <section data-record-family-case="sequence-loading">
        <RecordSequence
          rows={[]}
          content={content}
          recordId={(record) => record.id}
          state={{ kind: "loading", label: "Loading ordered course records..." }}
          ariaLabel="Loading ordered course records"
          emptyState={{ title: "No ordered records" }}
        />
      </section>
      <section data-record-family-case="sequence-error">
        <RecordSequence
          rows={[]}
          content={content}
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
  let refreshSemanticSequence: () => void = () => undefined;
  let refreshSemanticRecords: () => void = () => undefined;

  render(
    () => (
      <ApplicationApiProvider applicationApi={CLASSIFICATION_API}>
        <div data-record-list-harness>
          <SemanticContentCase
            onRefreshReady={(refresh) => {
              refreshSemanticRecords = refresh;
            }}
          />
          <SemanticSequenceCase
            onRefreshReady={(refresh) => {
              refreshSemanticSequence = refresh;
            }}
          />
          <SelectionCase />
          <ImageBrowserCase />
          <SequenceReorderCase />
          <OutlineReorderCase />
          <DuplicateActionIdCase />
          <AlignmentCase />
          <PresentationCase id="one-variant" variants="one" />
          <PresentationCase id="two-variants" variants="two" />
          <ReorderCase />
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
      </ApplicationApiProvider>
    ),
    target,
  );
  return {
    refreshSemanticRecords,
    refreshSemanticSequence,
  };
}
