import { Show, createEffect, createSignal, type Accessor, type JSX } from "solid-js";

import { MAX_BULK_QUESTION_METADATA_ITEMS } from "../../generated/api/MAX_BULK_QUESTION_METADATA_ITEMS";
import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
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
import {
  QuestionPoolSearchMetadataEditor,
  type QuestionPoolSearchMetadataTarget,
} from "../components/question_pool_search_metadata_editor";
import { SearchSelectionBar } from "../features/search/search_selection_bar";
import type { SearchStateController } from "../features/search/search_state";
import type { QuestionSearchSort } from "../../generated/api/QuestionSearchSort";
import type {
  LibrarySearchRow,
  QuestionLibraryBrowseQuery,
  QuestionLibraryFilterCounts,
} from "./library_page_model";

type SelectedLibraryRow =
  | { readonly kind: "question"; readonly title: string }
  | {
      readonly kind: "pool";
      readonly title: string;
      readonly target: QuestionPoolSearchMetadataTarget;
    };

export type LibraryBulkActionsController = {
  readonly busy: Accessor<boolean>;
  readonly selectionNotice: Accessor<string | null>;
  readonly questionTitles: Accessor<ReadonlyMap<string, string>>;
  readonly selectedQuestionCount: Accessor<number>;
  readonly selectedPoolCount: Accessor<number>;
  readonly clearForQueryChange: () => void;
  readonly updateSelection: (row: LibrarySearchRow, selected: boolean) => void;
  readonly selectLoaded: () => void;
  readonly clearSelection: () => void;
  readonly openQuestionMetadataEditor: () => Promise<void>;
  readonly openPoolMetadataEditor: () => void;
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
    QuestionLibraryFilterCounts,
    QuestionSearchSort
  >;
  readonly displayedRows: Accessor<ReadonlyArray<LibrarySearchRow>>;
  readonly refresh: () => Promise<void>;
  readonly metadataClient: QuestionBulkMetadataClient;
};

function selectedRow(row: LibrarySearchRow): SelectedLibraryRow {
  if (row.kind === "question") return { kind: "question", title: row.questionTitle };
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

  createEffect(() => {
    const rows = props.displayedRows();
    const selected = props.state.selectedIds();
    setSelectedRows((current) => {
      const next = new Map<string, SelectedLibraryRow>();
      for (const [id, row] of current) if (selected.has(id)) next.set(id, row);
      for (const row of rows)
        if (selected.has(row.displayId)) next.set(row.displayId, selectedRow(row));
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

  function clearEditorState(): void {
    setMetadata(null);
    setPoolTargets(null);
    setLoadError(false);
    setUpdateResults(null);
    setPoolUpdateResults(null);
  }
  function clearForQueryChange(): void {
    clearEditorState();
  }
  function updateSelection(row: LibrarySearchRow, selected: boolean): void {
    const selectedIds = props.state.selectedIds();
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
    props.state.select(row, selected);
    setSelectedRows((current) => {
      const next = new Map(current);
      if (selected) next.set(row.displayId, selectedRow(row));
      else next.delete(row.displayId);
      return next;
    });
    clearEditorState();
    setSelectionNotice(null);
  }
  function selectLoaded(): void {
    props.state.selectLoaded();
    clearEditorState();
    setSelectionNotice(
      props.displayedRows().length > MAX_BULK_QUESTION_METADATA_ITEMS
        ? `Selected the first ${MAX_BULK_QUESTION_METADATA_ITEMS} loaded Library items.`
        : `Selected all ${props.displayedRows().length} loaded Library items.`,
    );
  }
  function clearSelection(): void {
    props.state.clearSelection();
    setSelectedRows(new Map());
    clearEditorState();
    setSelectionNotice("Selection cleared.");
  }
  async function openQuestionMetadataEditor(): Promise<void> {
    setLoading(true);
    setBusy(true);
    setLoadError(false);
    setUpdateResults(null);
    try {
      const selection = questionLibraryBulkSelectionRequest([...selectedQuestionIds()]);
      setMetadata(await props.metadataClient.getCurrentQuestionBulkMetadata(selection.questionIds));
    } catch {
      setLoadError(true);
    } finally {
      setLoading(false);
      setBusy(false);
    }
  }
  function openPoolMetadataEditor(): void {
    setPoolTargets(
      [...selectedRows().values()].flatMap((row) => (row.kind === "pool" ? [row.target] : [])),
    );
    setMetadata(null);
    setLoadError(false);
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
    busy,
    selectionNotice,
    questionTitles,
    selectedQuestionCount,
    selectedPoolCount,
    clearForQueryChange,
    updateSelection,
    selectLoaded,
    clearSelection,
    openQuestionMetadataEditor,
    openPoolMetadataEditor,
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
  readonly selectedCount: () => number;
  readonly loadedCount: () => number;
};

/** Renders the two established metadata editors for one mixed, globally capped selection. */
export function LibraryBulkActions(props: LibraryBulkActionsProps): JSX.Element {
  const controller = props.controller;
  return (
    <>
      <SearchSelectionBar
        selectedCount={props.selectedCount}
        loadedCount={props.loadedCount}
        disabled={controller.busy}
        onSelectLoaded={controller.selectLoaded}
        onClearSelection={controller.clearSelection}
      >
        <Show when={controller.selectedQuestionCount() > 0}>
          <button
            type="button"
            class="primary-action"
            disabled={controller.busy()}
            onClick={() => void controller.openQuestionMetadataEditor()}
          >
            Edit Question metadata ({controller.selectedQuestionCount()})
          </button>
        </Show>
        <Show when={controller.selectedPoolCount() > 0}>
          <button
            type="button"
            class="primary-action"
            disabled={controller.busy()}
            onClick={controller.openPoolMetadataEditor}
          >
            Edit Pool search fields ({controller.selectedPoolCount()})
          </button>
        </Show>
      </SearchSelectionBar>
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
            disabled={controller.busy()}
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
