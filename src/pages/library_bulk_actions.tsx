import { Show, createEffect, createSignal, onCleanup, type Accessor, type JSX } from "solid-js";

import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import { decodeQuestionType } from "../api/decoders/question_response_format";
import { QUESTION_BACKENDS } from "../api/decoders/shared";
import type { ContentClassificationClient } from "../api/content_classification";
import type {
  QuestionBulkMetadataClient,
  QuestionBulkMetadataUpdateResult,
} from "../api/question_bulk_metadata";
import type {
  QuestionPoolSearchMetadataClient,
  QuestionPoolSearchMetadataUpdateResult,
} from "../api/question_pool_search_metadata";
import { questionLibraryBulkSelectionRequest } from "../api/question_library_repository";
import { QuestionBulkMetadataEditor } from "../components/question_bulk_metadata_editor";
import type { QuestionPoolStartingQuestion } from "../components/question_pool_create_model";
import type { RecordAction } from "../components/record_list/record_list";
import { confirmUnsavedChanges } from "../components/unsaved_changes_guard";
import {
  QuestionPoolSearchMetadataEditor,
  type QuestionPoolSearchMetadataTarget,
} from "../components/question_pool_search_metadata_editor";
import type { SearchStateController } from "../features/search/search_state";
import type {
  LibrarySearchRow,
  QuestionLibraryBrowseQuery,
  QuestionLibraryFilterCounts,
} from "./library_page_model";

type SelectedLibraryRow =
  | {
      readonly kind: "question";
      readonly title: string;
      readonly startingQuestion: QuestionPoolStartingQuestion;
    }
  | {
      readonly kind: "pool";
      readonly title: string;
      readonly target: QuestionPoolSearchMetadataTarget;
    };

export type LibraryBulkActionsController = {
  readonly guardScope: symbol;
  readonly busy: Accessor<boolean>;
  readonly selectionNotice: Accessor<string | null>;
  readonly questionTitles: Accessor<ReadonlyMap<string, string>>;
  readonly selectedQuestionCount: Accessor<number>;
  readonly selectedPoolCount: Accessor<number>;
  readonly toolbarActions: Accessor<ReadonlyArray<RecordAction>>;
  readonly clearForQueryChange: () => void;
  readonly updateSelection: (row: LibrarySearchRow, selected: boolean) => Promise<void>;
  readonly selectLoaded: () => Promise<void>;
  readonly clearSelection: () => Promise<void>;
  readonly openQuestionMetadataEditor: () => Promise<void>;
  readonly openPoolMetadataEditor: () => Promise<void>;
  readonly openPoolCreation: () => Promise<void>;
  readonly setEditorBusy: (busy: boolean) => void;
  readonly metadata: Accessor<ReadonlyArray<PublishedQuestionSharedMetadata> | null>;
  readonly poolTargets: Accessor<ReadonlyArray<QuestionPoolSearchMetadataTarget> | null>;
  readonly loading: Accessor<boolean>;
  readonly loadError: Accessor<boolean>;
  readonly updateResults: Accessor<ReadonlyArray<QuestionBulkMetadataUpdateResult> | null>;
  readonly poolUpdateResults: Accessor<ReadonlyArray<QuestionPoolSearchMetadataUpdateResult> | null>;
  readonly cancelEditor: () => void;
  readonly metadataUpdateSucceeded: (
    results: ReadonlyArray<QuestionBulkMetadataUpdateResult>,
  ) => void;
  readonly poolMetadataUpdateSucceeded: (
    results: ReadonlyArray<QuestionPoolSearchMetadataUpdateResult>,
  ) => void;
};

type LibraryBulkActionsControllerProps = {
  readonly state: SearchStateController<
    QuestionLibraryBrowseQuery,
    LibrarySearchRow,
    QuestionLibraryFilterCounts
  >;
  readonly displayedRows: Accessor<ReadonlyArray<LibrarySearchRow>>;
  readonly refresh: () => Promise<void>;
  readonly metadataClient: QuestionBulkMetadataClient;
  readonly onCreatePoolFromQuestions: (
    startingQuestions: ReadonlyArray<QuestionPoolStartingQuestion>,
  ) => void;
  readonly guardScope: symbol;
};

function selectedPoolRow(row: Extract<LibrarySearchRow, { kind: "pool" }>): SelectedLibraryRow {
  return {
    kind: "pool",
    title: row.title,
    target: {
      questionPoolId: row.displayId,
      questionPoolMetadataEditNumber: row.questionPoolMetadataEditNumber,
      title: row.title,
      disciplineName: row.disciplineName,
      subjectUuid: row.subjectUuid,
      topicUuid: row.topicUuid,
      subtopicUuid: row.subtopicUuid,
      tags: row.tags,
    },
  };
}

export function selectedQuestionRow(
  row: Extract<LibrarySearchRow, { kind: "question" }>,
): SelectedLibraryRow {
  if (!QUESTION_BACKENDS.includes(row.backend as QuestionBackend)) {
    throw new Error("The selected Question has an unknown Backend.");
  }
  return {
    kind: "question",
    title: row.questionTitle,
    startingQuestion: {
      publishedQuestionRevisionTuple: row.publishedQuestionRevisionTuple,
      questionTitle: row.questionTitle,
      disciplineName: row.classification.disciplineName,
      subjectName: row.classification.subjectName,
      questionType: decodeQuestionType(row.questionType, "selected Question Type"),
      backend: row.backend as QuestionBackend,
    },
  };
}

/** Owns Library-specific selection and delegates each selected kind to its established editor. */
export function createLibraryBulkActions(
  props: LibraryBulkActionsControllerProps,
): LibraryBulkActionsController {
  const [selectionNotice, setSelectionNotice] = createSignal<string | null>(null);
  const [metadata, setMetadata] =
    createSignal<ReadonlyArray<PublishedQuestionSharedMetadata> | null>(null);
  const [poolTargets, setPoolTargets] =
    createSignal<ReadonlyArray<QuestionPoolSearchMetadataTarget> | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [loadError, setLoadError] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [updateResults, setUpdateResults] =
    createSignal<ReadonlyArray<QuestionBulkMetadataUpdateResult> | null>(null);
  const [poolUpdateResults, setPoolUpdateResults] =
    createSignal<ReadonlyArray<QuestionPoolSearchMetadataUpdateResult> | null>(null);
  const [selectedRows, setSelectedRows] = createSignal<ReadonlyMap<string, SelectedLibraryRow>>(
    new Map(),
  );
  let disposed = false;
  let metadataReadId = 0;
  onCleanup(() => {
    disposed = true;
  });

  async function mayDiscardEditor(): Promise<boolean> {
    return (await confirmUnsavedChanges(props.guardScope)) && !disposed;
  }

  createEffect(() => {
    const rows = props.displayedRows();
    const selected = props.state.selectedIds();
    setSelectedRows((current) => {
      const next = new Map<string, SelectedLibraryRow>();
      for (const [id, row] of current) if (selected.has(id)) next.set(id, row);
      for (const row of rows)
        if (selected.has(row.displayId) && row.kind === "pool")
          next.set(row.displayId, selectedPoolRow(row));
      return next;
    });
  });

  const selectedQuestionIds = (): ReadonlyArray<string> =>
    [...selectedRows()].flatMap(([id, row]) => (row.kind === "question" ? [id] : []));
  const selectedQuestionCount = (): number => selectedQuestionIds().length;
  const selectedPoolCount = (): number =>
    [...selectedRows().values()].filter((row) => row.kind === "pool").length;
  const questionTitles = (): ReadonlyMap<string, string> =>
    new Map(
      [...selectedRows()].flatMap(([id, row]) =>
        row.kind === "question" ? [[id, row.title]] : [],
      ),
    );
  const toolbarActions = (): ReadonlyArray<RecordAction> => [
    ...(selectedQuestionCount() > 0
      ? [
          {
            id: "create-pool-from-questions",
            kind: "command" as const,
            label: `Create Pool from Questions (${selectedQuestionCount()})`,
            primary: true,
            onClick: (): void => void openPoolCreation(),
          },
        ]
      : []),
    ...(selectedQuestionCount() > 0
      ? [
          {
            id: "edit-question-metadata",
            kind: "command" as const,
            label: `Edit Question metadata (${selectedQuestionCount()})`,
            primary: true,
            onClick: (): void => void openQuestionMetadataEditor(),
          },
        ]
      : []),
    ...(selectedPoolCount() > 0
      ? [
          {
            id: "edit-pool-search-fields",
            kind: "command" as const,
            label: `Edit Pool search fields (${selectedPoolCount()})`,
            primary: true,
            onClick: (): void => void openPoolMetadataEditor(),
          },
        ]
      : []),
  ];

  function clearEditorState(): void {
    metadataReadId += 1;
    setMetadata(null);
    setPoolTargets(null);
    setLoading(false);
    setBusy(false);
    setLoadError(false);
    setUpdateResults(null);
    setPoolUpdateResults(null);
  }
  function clearForQueryChange(): void {
    clearEditorState();
  }
  async function updateSelection(row: LibrarySearchRow, selected: boolean): Promise<void> {
    const selectedIds = props.state.selectedIds();
    if (selectedIds.has(row.displayId) === selected) return;
    if (
      selected &&
      !selectedIds.has(row.displayId) &&
      selectedIds.size >= MAX_BULK_QUESTION_METADATA_ITEMS
    ) {
      setSelectionNotice(
        `You can select at most ${MAX_BULK_QUESTION_METADATA_ITEMS} Library items at once.`,
      );
      return;
    }
    if (!(await mayDiscardEditor())) return;
    let nextRow: SelectedLibraryRow | undefined;
    if (selected) {
      try {
        nextRow = row.kind === "question" ? selectedQuestionRow(row) : selectedPoolRow(row);
      } catch {
        setSelectionNotice("The selected Question could not be loaded. Try selecting it again.");
        return;
      }
    }
    props.state.select(row, selected);
    setSelectedRows((current) => {
      const next = new Map(current);
      if (selected && nextRow !== undefined) next.set(row.displayId, nextRow);
      else next.delete(row.displayId);
      return next;
    });
    clearEditorState();
    setSelectionNotice(null);
  }
  async function selectLoaded(): Promise<void> {
    if (props.displayedRows().every((row) => props.state.selectedIds().has(row.displayId))) return;
    if (!(await mayDiscardEditor())) return;
    const rows = props
      .displayedRows()
      .filter((row) => !props.state.selectedIds().has(row.displayId))
      .slice(0, MAX_BULK_QUESTION_METADATA_ITEMS - props.state.selectedIds().size);
    let added: ReadonlyArray<[string, SelectedLibraryRow]>;
    try {
      added = rows.map((row): [string, SelectedLibraryRow] => [
        row.displayId,
        row.kind === "question" ? selectedQuestionRow(row) : selectedPoolRow(row),
      ]);
    } catch {
      setSelectionNotice("Some selected Questions could not be loaded. Try again.");
      return;
    }
    props.state.selectLoaded();
    setSelectedRows((current) => new Map([...current, ...added]));
    clearEditorState();
    setSelectionNotice(
      props.displayedRows().length > MAX_BULK_QUESTION_METADATA_ITEMS
        ? `Selected the first ${MAX_BULK_QUESTION_METADATA_ITEMS} loaded Library items.`
        : `Selected all ${props.displayedRows().length} loaded Library items.`,
    );
  }
  async function clearSelection(): Promise<void> {
    if (props.state.selectedIds().size === 0) return;
    if (!(await mayDiscardEditor())) return;
    props.state.clearSelection();
    setSelectedRows(new Map());
    clearEditorState();
    setSelectionNotice("Selection cleared.");
  }
  async function openQuestionMetadataEditor(): Promise<void> {
    if (!(await mayDiscardEditor())) return;
    clearEditorState();
    const readId = metadataReadId;
    setLoading(true);
    setBusy(true);
    setLoadError(false);
    setUpdateResults(null);
    try {
      const selection = questionLibraryBulkSelectionRequest([...selectedQuestionIds()]);
      const current = await props.metadataClient.getCurrentQuestionSharedMetadata(
        selection.questionIds,
      );
      if (!disposed && readId === metadataReadId) setMetadata(current);
    } catch {
      if (!disposed && readId === metadataReadId) setLoadError(true);
    } finally {
      if (!disposed && readId === metadataReadId) {
        setLoading(false);
        setBusy(false);
      }
    }
  }
  async function openPoolMetadataEditor(): Promise<void> {
    if (!(await mayDiscardEditor())) return;
    clearEditorState();
    setPoolTargets(
      [...selectedRows().values()].flatMap((row) => (row.kind === "pool" ? [row.target] : [])),
    );
  }
  async function openPoolCreation(): Promise<void> {
    if (!(await mayDiscardEditor())) return;
    const startingQuestions = [...selectedRows().values()].flatMap((row) =>
      row.kind === "question" ? [row.startingQuestion] : [],
    );
    if (startingQuestions.length === 0) return;
    props.onCreatePoolFromQuestions(startingQuestions);
  }
  function finishUpdate(kind: SelectedLibraryRow["kind"]): void {
    const completedIds = [...selectedRows()].flatMap(([id, row]) =>
      row.kind === kind ? [id] : [],
    );
    props.state.deselectIds(completedIds);
    setSelectedRows((current) => new Map([...current].filter(([, row]) => row.kind !== kind)));
    setSelectionNotice(null);
    void props.refresh();
  }
  function metadataUpdateSucceeded(results: ReadonlyArray<QuestionBulkMetadataUpdateResult>): void {
    setUpdateResults(results);
    setMetadata(null);
    finishUpdate("question");
  }
  function poolMetadataUpdateSucceeded(
    results: ReadonlyArray<QuestionPoolSearchMetadataUpdateResult>,
  ): void {
    setPoolUpdateResults(results);
    setPoolTargets(null);
    finishUpdate("pool");
  }
  return {
    guardScope: props.guardScope,
    busy,
    selectionNotice,
    questionTitles,
    selectedQuestionCount,
    selectedPoolCount,
    toolbarActions,
    clearForQueryChange,
    updateSelection,
    selectLoaded,
    clearSelection,
    openQuestionMetadataEditor,
    openPoolMetadataEditor,
    openPoolCreation,
    setEditorBusy: setBusy,
    metadata,
    poolTargets,
    loading,
    loadError,
    updateResults,
    poolUpdateResults,
    cancelEditor: clearEditorState,
    metadataUpdateSucceeded,
    poolMetadataUpdateSucceeded,
  };
}

export type LibraryBulkActionsProps = {
  readonly controller: LibraryBulkActionsController;
  readonly metadataClient: QuestionBulkMetadataClient;
  readonly poolMetadataClient: QuestionPoolSearchMetadataClient;
  readonly classificationClient: ContentClassificationClient;
};

/** Renders the two established metadata editors for one mixed, globally capped selection. */
export function LibraryBulkActions(props: LibraryBulkActionsProps): JSX.Element {
  const controller = props.controller;
  return (
    <>
      <Show when={controller.loading()}>
        <p class="loading-state" role="status">
          Reading current metadata for selected Questions...
        </p>
      </Show>
      <Show when={controller.loadError()}>
        <section class="route-error" role="alert">
          <h2>Current metadata could not be loaded</h2>
          <p>Your selection is preserved. Retry the read before editing.</p>
          <button
            class="primary-action"
            type="button"
            onClick={() => void controller.openQuestionMetadataEditor()}
          >
            Retry current metadata
          </button>
        </section>
      </Show>
      <Show when={controller.metadata()}>
        {(metadata) => (
          <QuestionBulkMetadataEditor
            client={props.metadataClient}
            classificationClient={props.classificationClient}
            initialMetadata={metadata()}
            questionTitles={controller.questionTitles()}
            guardScope={props.controller.guardScope}
            onBusyChange={controller.setEditorBusy}
            onCancel={controller.cancelEditor}
            onSuccess={controller.metadataUpdateSucceeded}
          />
        )}
      </Show>
      <Show when={controller.poolTargets()}>
        {(targets) => (
          <QuestionPoolSearchMetadataEditor
            client={props.poolMetadataClient}
            classificationClient={props.classificationClient}
            targets={targets()}
            guardScope={props.controller.guardScope}
            disabled={controller.busy()}
            onCancel={controller.cancelEditor}
            onSuccess={controller.poolMetadataUpdateSucceeded}
          />
        )}
      </Show>
      <Show when={controller.updateResults()}>
        {(results) => (
          <section class="question-library-bulk-success" role="status">
            <h2>Updated shared metadata for {results().length} Questions</h2>
            <p>
              The updated Questions were removed from the selection while the Library refreshes.
            </p>
          </section>
        )}
      </Show>
      <Show when={controller.poolUpdateResults()}>
        {(results) => (
          <section class="question-library-bulk-success" role="status">
            <h2>Updated search fields for {results().length} Question Pools</h2>
            <p>The updated Pools were removed from the selection while the Library refreshes.</p>
          </section>
        )}
      </Show>
    </>
  );
}
