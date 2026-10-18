// Shared Assessment composition; callers retain the complete authored draft and commands.
import { Show, createMemo, createSignal, type Accessor, type JSX } from "solid-js";

import type { ContentClassificationClient } from "../../api/content_classification";
import type { RecordContent } from "../../components/data_sheet/record_content";
import {
  RecordTransferProvider,
  type RecordTransferCommand,
} from "../../components/data_sheet/record_interactions";
import type { RecordField } from "../../components/data_sheet/record_fields";
import type {
  LibrarySearchRow,
  QuestionLibraryBrowseRepository,
} from "../../pages/library_page_model";
import { librarySearchFields } from "../../pages/library_search_fields";
import { libraryRecordQuery } from "../../pages/library_record_filters";
import {
  librarySearchContent,
  questionLibrarySearchDefinition,
} from "../../pages/question_library_search_definition";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import {
  assessmentContentPickerSelection,
  type AssessmentContentPickerSelection,
} from "./assessment_content_picker_model";
import "./assessment_editor.css";

type EntryRecord<Entry> = {
  readonly entry: Entry;
  readonly id: string;
  readonly position: number;
};

export type AssessmentEditorProps<Entry> = {
  readonly entries: ReadonlyArray<Entry>;
  readonly entryId: (entry: Entry) => string;
  readonly entryKind: (entry: Entry) => string;
  readonly content: (entry: Entry, position: number) => RecordContent;
  readonly renderBody?: (entry: Accessor<Entry>) => JSX.Element;
  readonly disabled: boolean;
  readonly onMove: (entryId: string, destinationEntryId: string) => void;
  readonly removeCommand: (entry: Entry) => RecordTransferCommand;
  readonly contentsTools?: JSX.Element;
  readonly libraryTools?: JSX.Element;
  readonly repository: QuestionLibraryBrowseRepository;
  readonly classificationClient: ContentClassificationClient;
  readonly included: (row: LibrarySearchRow) => boolean;
  readonly duplicateAllowed: (row: LibrarySearchRow) => boolean;
  /** Domain duplicate/capacity rules decide availability; matching rows remain ordinary results. */
  readonly addDisabled: (row: LibrarySearchRow) => boolean;
  readonly onAdd: (selection: AssessmentContentPickerSelection) => void;
};

/** Two independent SearchBlocks over one caller-owned Assessment draft. */
export function AssessmentEditor<Entry>(props: AssessmentEditorProps<Entry>): JSX.Element {
  const records = createMemo(() =>
    props.entries.map((entry, index) => ({
      entry,
      id: props.entryId(entry),
      position: index + 1,
    })),
  );
  const [filterError, setFilterError] = createSignal<string | null>(null);

  function content(record: EntryRecord<Entry>): RecordContent {
    const base = props.content(record.entry, record.position);
    const command = props.removeCommand(record.entry);
    return {
      ...base,
      details: [
        { kind: "text", label: "Entry", value: String(record.position), fieldId: "position" },
        ...base.details,
      ],
      actions: [
        ...base.actions,
        {
          id: "remove",
          kind: "command",
          label: command.label,
          disabled: command.disabled,
          onClick: () => void command.run(),
        },
      ],
    };
  }

  const fields: ReadonlyArray<RecordField<EntryRecord<Entry>, never, "kind">> = [
    {
      id: "position",
      label: "Entry",
      value: (record) => record.position,
      defaultColumn: { priority: 1, width: "5rem", visible: true },
    },
    {
      id: "title",
      label: "Title",
      role: "identity",
      value: (record) => props.content(record.entry, record.position).title,
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "kind",
      label: "Library object",
      filterKey: "kind",
      value: (record) => props.entryKind(record.entry),
      defaultColumn: { priority: 2, width: "10rem" },
    },
  ];
  const contents = createLocalCompleteSearch(
    { rows: records },
    {
      fields,
      rowId: (record) => record.id,
      keyword: (record) => {
        const value = props.content(record.entry, record.position);
        const facts = value.details
          .map((fact) => {
            const value =
              "value" in fact ? fact.value : fact.kind === "questionId" ? fact.displayId : "";
            return typeof value === "string" ? value : "";
          })
          .join(" ");
        const kind = props.entryKind(record.entry);
        return `${record.position} ${value.title} ${value.description ?? ""} ${kind} ${facts}`;
      },
      content,
    },
  );
  // Saved-order contents deliberately expose no display-sort binding.
  const contentsQuery = { kind: contents.query.kind, filters: contents.query.filters };
  function addCommand(row: LibrarySearchRow): RecordTransferCommand {
    return {
      label: props.included(row) ? (props.duplicateAllowed(row) ? "Add again" : "Included") : "Add",
      disabled: props.disabled || props.addDisabled(row),
      run: (): void => {
        if (props.disabled || props.addDisabled(row)) return;
        const selection = assessmentContentPickerSelection([row]);
        if (selection !== undefined) props.onAdd(selection);
      },
    };
  }
  const definition = questionLibrarySearchDefinition(props.repository);
  const library = createSearchState({
    ...definition,
    content: (row: LibrarySearchRow): RecordContent => {
      const base = librarySearchContent(row);
      const command = addCommand(row);
      return {
        ...base,
        details: props.included(row)
          ? [...base.details, { kind: "text", label: "Membership", value: "Included" }]
          : base.details,
        actions: [
          ...base.actions,
          {
            id: "add",
            kind: "command",
            label: command.label,
            title: props.included(row) ? "Included in this Assessment" : "Add to this Assessment",
            disabled: command.disabled,
            onClick: () => void command.run(),
          },
        ],
      };
    },
  });
  const libraryQuery = libraryRecordQuery({
    query: library.query,
    apply: library.apply,
    sort: library.sort,
    replacementRevision: library.replacementRevision,
    repository: props.repository,
    classificationClient: props.classificationClient,
    onError: setFilterError,
  });

  return (
    <RecordTransferProvider>
      <div class="assessment-editor-panels">
        <section class="assessment-editor-panel" aria-label="Assessment contents">
          <div class="assessment-editor-contents-heading">
            <h2>Ordered Assessment Entries</h2>
            {props.contentsTools}
          </div>
          <SearchBlock
            layout="above"
            state={contents.state}
            controls={
              <SearchControls state={contents.state} textLabel="Search Assessment contents" />
            }
            collection={{
              ariaLabel: "Ordered Assessment Entries",
              fields,
              query: contentsQuery,
              disabled: () => props.disabled,
              reorder: {
                orderForRecord: (id) => {
                  const current = records();
                  return current.some((record) => record.id === id)
                    ? current.map((record) => record.id)
                    : [];
                },
                onMove: props.onMove,
                recordLabel: (id) => {
                  const record = records().find((record) => record.id === id);
                  return record === undefined
                    ? "Assessment Entry"
                    : `Assessment Entry ${record.position}`;
                },
                isDisabled: () => props.disabled,
              },
              transfer: {
                panelKey: "contents",
                targetPanelKey: "library",
                command: (record) => props.removeCommand(record.entry),
              },
              emptyState: { title: "No Assessment Entries match this search." },
              renderBody:
                props.renderBody === undefined
                  ? undefined
                  : (record): JSX.Element => props.renderBody!(() => record().entry),
            }}
          />
        </section>
        <section class="assessment-editor-panel" aria-label="Available Assessment content">
          <h2>Available published Questions and Pools</h2>
          {props.libraryTools}
          <Show when={filterError()}>{(error) => <p role="alert">{error()}</p>}</Show>
          <SearchBlock
            layout="above"
            state={library}
            controls={
              <SearchControls state={library} textLabel="Search published Assessment content">
                <label>
                  Show
                  <select
                    value={library.query().kind}
                    onChange={(event) => {
                      const kind = event.currentTarget.value;
                      // ASVS 2.2.1: accept only the Library's closed kind values.
                      if (kind === "both" || kind === "questions" || kind === "pools")
                        void library.apply({ ...library.query(), kind });
                    }}
                  >
                    <option value="both">Questions and Question Pools</option>
                    <option value="questions">Questions only</option>
                    <option value="pools">Question Pools only</option>
                  </select>
                </label>
                <label>
                  Question membership
                  <select
                    value={library.query().questions}
                    disabled={library.query().kind === "pools"}
                    onChange={(event) => {
                      const questions = event.currentTarget.value;
                      if (questions === "all" || questions === "inNoPool")
                        void library.apply({ ...library.query(), questions });
                    }}
                  >
                    <option value="inNoPool">Questions in no Pool</option>
                    <option value="all">All Questions</option>
                  </select>
                </label>
              </SearchControls>
            }
            collection={{
              ariaLabel: "Published Assessment content",
              fields: librarySearchFields,
              query: libraryQuery,
              disabled: () => props.disabled,
              transfer: {
                panelKey: "library",
                targetPanelKey: "contents",
                command: addCommand,
              },
              initialEmptyState: {
                message: "Enter a keyword or choose a column filter to find Library objects.",
              },
              emptyState: { title: "No published Assessment content matches these choices." },
            }}
          />
        </section>
      </div>
    </RecordTransferProvider>
  );
}
