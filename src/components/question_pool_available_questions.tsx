// One Pool-specific Library composition for creation and membership editing.

import { Show, createSignal, onMount, type JSX } from "solid-js";

import type { ContentClassificationClient } from "../api/content_classification";
import type { QuestionPickerEligibility } from "../features/question_picker";
import { SearchBlock } from "./search_block/search_block";
import { SearchControls } from "./search_block/search_controls";
import {
  type LibraryObjectQuestionRow,
  type LibrarySearchRow,
  type QuestionLibraryBrowseRepository,
} from "../api/question_library_projection";
import { EMPTY_QUESTION_LIBRARY_BROWSE_QUERY } from "../features/question_library/library_query";
import { fixedKindLibrarySearchFields } from "../features/question_library/library_search_fields";
import {
  createQuestionLibraryCollection,
  questionLibraryContent,
  questionLibrarySearchDefinition,
} from "../features/question_library/question_library_search_definition";
import type { RecordTransferBinding } from "./data_sheet/data_sheet_interactions";
import {
  questionPoolAvailableQuery,
  questionPoolAvailableRepository,
} from "./question_pool_available_model";

export interface QuestionPoolAvailableQuestionsProps {
  readonly repository: QuestionLibraryBrowseRepository;
  readonly classificationClient: ContentClassificationClient;
  readonly eligibility: QuestionPickerEligibility;
  readonly includedIds: ReadonlySet<string>;
  readonly disabled: boolean;
  readonly onAdd: (row: LibraryObjectQuestionRow) => Promise<boolean>;
  readonly transfer?: RecordTransferBinding<LibrarySearchRow>;
}

/** Reuses the Library query/fields/content while the editor alone owns membership. */
export function QuestionPoolAvailableQuestions(
  props: QuestionPoolAvailableQuestionsProps,
): JSX.Element {
  const [error, setError] = createSignal<string | null>(null);
  const repository = questionPoolAvailableRepository(props.repository, props.eligibility);
  const base = questionLibrarySearchDefinition(
    repository,
    questionPoolAvailableQuery(EMPTY_QUESTION_LIBRARY_BROWSE_QUERY, props.eligibility),
    true,
    "questions",
  );
  const fields = fixedKindLibrarySearchFields.map((field) =>
    ["discipline", "subject", "type", "backend"].includes(field.id)
      ? { ...field, filterKey: undefined }
      : field,
  );
  const library = createQuestionLibraryCollection({
    definition: {
      ...base,
      cleanup: (query) => questionPoolAvailableQuery(base.cleanup(query), props.eligibility),
      appliedFilters: (query) =>
        (base.appliedFilters?.(query) ?? []).filter(
          (filter) =>
            !["kind", "discipline", "classification-subject", "backends", "questionTypes"].includes(
              filter.id,
            ) &&
            !(
              filter.id === "questionLicenses" &&
              base.initialQuery.questionLicenses.every((license) =>
                query.questionLicenses.includes(license),
              )
            ),
        ),
      content: (row) => {
        if (row.kind !== "question") throw new Error("Pool Library results must be Questions.");
        const content = questionLibraryContent(row);
        const command = props.transfer?.command(row) ?? {
          label: props.includedIds.has(row.displayId) ? "Included" : "Add",
          disabled: props.includedIds.has(row.displayId) || props.disabled,
          run: (): void => {
            if (!props.includedIds.has(row.displayId) && !props.disabled) void props.onAdd(row);
          },
        };
        return {
          ...content,
          actions: [
            ...content.actions,
            {
              id: "add",
              kind: "command" as const,
              label: command.label,
              primary: true,
              disabled: command.disabled,
              onClick: (): void => {
                void command.run();
              },
            },
          ],
        };
      },
    },
    repository,
    classificationClient: props.classificationClient,
    fields,
    onError: setError,
  });
  const search = library.state;
  onMount(() => void search.open(search.query()));
  return (
    <>
      <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
      <SearchBlock
        layout="above"
        state={search}
        controls={<SearchControls state={search} textLabel="Search available Library Questions" />}
        collection={{
          ariaLabel: "Available Library Questions",
          ...library.collection,
          transfer: props.transfer,
          emptyState: { title: "No eligible Library Questions match this search." },
        }}
      />
    </>
  );
}
