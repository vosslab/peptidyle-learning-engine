// library_browse_controls.tsx - progressive navigation over the shared Library query.

import { A } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  type Accessor,
  type JSX,
} from "solid-js";

import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import { checkboxChange } from "../components/controlled_form_change";
import type {
  RecordFilterBinding,
  RecordFilterChoice,
  RecordFilterChoices,
} from "../components/record_list/record_fields";
import type { QuestionLibraryBrowseQuery } from "./library_page_model";

type ClassificationField = "disciplineUuids" | "subjectUuids" | "topicUuids" | "subtopicUuids";

const levels: ReadonlyArray<{ readonly field: ClassificationField; readonly label: string }> = [
  { field: "disciplineUuids", label: "Discipline" },
  { field: "subjectUuids", label: "Subject" },
  { field: "topicUuids", label: "Topic" },
  { field: "subtopicUuids", label: "Subtopic" },
];

export interface LibraryBrowseControlsProps {
  readonly query: Accessor<QuestionLibraryBrowseQuery>;
  readonly filters: RecordFilterBinding<LibraryFacetField>;
  readonly hasCriteria: () => boolean;
  readonly searchWithinResultsPath: () => string;
  readonly disabled: () => boolean;
  readonly changeQuery: (change: Partial<QuestionLibraryBrowseQuery>) => void;
  readonly startOver: () => void;
}

function BrowseLevel(props: {
  readonly field: ClassificationField;
  readonly label: string;
  readonly query: Accessor<QuestionLibraryBrowseQuery>;
  readonly filters: RecordFilterBinding<LibraryFacetField>;
  readonly disabled: () => boolean;
}): JSX.Element {
  const [search, setSearch] = createSignal("");
  const [page, setPage] = createSignal<RecordFilterChoices | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal(false);
  const [retry, setRetry] = createSignal(0);
  let request: AbortController | undefined;
  let requestNumber = 0;

  const selectedKeys = (): ReadonlyArray<string> => props.filters.values.get(props.field) ?? [];
  const visibleChoices = createMemo(() => {
    const current = page();
    if (current === null) return [];
    const byKey = new Map<string, RecordFilterChoice>();
    for (const choice of current.selected) byKey.set(choice.key, choice);
    for (const choice of current.choices) {
      if (choice.count > 0 || selectedKeys().includes(choice.key)) byKey.set(choice.key, choice);
    }
    return [...byKey.values()];
  });

  function load(cursor: string | null, append: boolean): void {
    request?.abort();
    const currentRequest = new AbortController();
    request = currentRequest;
    const number = ++requestNumber;
    setLoading(true);
    setError(false);
    void props.filters.choices(props.field, search(), cursor, currentRequest.signal).then(
      (result) => {
        if (currentRequest.signal.aborted || number !== requestNumber) return;
        const previous = page();
        const byKey = new Map<string, RecordFilterChoice>();
        for (const choice of append ? (previous?.choices ?? []) : []) byKey.set(choice.key, choice);
        for (const choice of result.choices) byKey.set(choice.key, choice);
        setPage({ ...result, choices: [...byKey.values()] });
        setLoading(false);
      },
      () => {
        if (currentRequest.signal.aborted || number !== requestNumber) return;
        setError(true);
        setLoading(false);
      },
    );
  }

  createEffect(() => {
    props.query();
    search();
    retry();
    setPage(null);
    load(null, false);
    onCleanup(() => {
      request?.abort();
      requestNumber++;
    });
  });

  function choose(key: string): void {
    const selected = selectedKeys();
    props.filters.onChange(
      props.field,
      selected.includes(key) ? selected.filter((value) => value !== key) : [key],
    );
  }

  return (
    <section class="question-library-browse-level" aria-label={props.label}>
      <label>
        {props.label}
        <input
          type="search"
          value={search()}
          placeholder={`Find ${props.label.toLowerCase()}`}
          aria-label={`Find ${props.label.toLowerCase()}`}
          disabled={props.disabled()}
          onInput={(event) => setSearch(event.currentTarget.value)}
        />
      </label>
      <Show when={loading() && page() === null}>
        <p role="status">Loading {props.label.toLowerCase()} choices...</p>
      </Show>
      <Show when={error()}>
        <p role="alert">Could not load {props.label.toLowerCase()} choices.</p>
        <button
          type="button"
          disabled={props.disabled()}
          onClick={() => setRetry((value) => value + 1)}
        >
          Retry
        </button>
      </Show>
      <div class="question-library-browse-choices">
        <For each={visibleChoices()}>
          {(choice) => (
            <button
              type="button"
              aria-pressed={selectedKeys().includes(choice.key)}
              disabled={props.disabled()}
              onClick={() => choose(choice.key)}
            >
              <span>{choice.label}</span>
              <strong>{choice.count}</strong>
            </button>
          )}
        </For>
      </div>
      <Show when={!loading() && !error() && page() !== null && visibleChoices().length === 0}>
        <p>No matching {props.label.toLowerCase()} choices.</p>
      </Show>
      <Show when={page()?.nextCursor && !error()}>
        <button
          type="button"
          disabled={loading() || props.disabled()}
          onClick={() => load(page()!.nextCursor, true)}
        >
          Load more
        </button>
      </Show>
    </section>
  );
}

export function LibraryBrowseControls(props: LibraryBrowseControlsProps): JSX.Element {
  const visibleLevels = createMemo(() => {
    const query = props.query();
    return levels.filter(({ field }) => {
      if (field === "subjectUuids") return query.discipline_uuids.length > 0;
      if (field === "topicUuids") return query.subject_uuids.length > 0;
      if (field === "subtopicUuids") return query.topic_uuids.length > 0;
      return true;
    });
  });

  return (
    <section class="question-library-browse-controls" aria-label="Browse Question Library">
      <Show when={props.hasCriteria()}>
        <div class="question-library-browse-actions">
          <A href={props.searchWithinResultsPath()}>Search within results</A>
          <button type="button" disabled={props.disabled()} onClick={props.startOver}>
            Start over
          </button>
        </div>
      </Show>
      <div class="question-library-browse-levels">
        <For each={visibleLevels()}>
          {(level) => (
            <BrowseLevel
              field={level.field}
              label={level.label}
              query={props.query}
              filters={props.filters}
              disabled={props.disabled}
            />
          )}
        </For>
      </div>
      <Show when={props.query().subject_uuids.length > 0}>
        <label class="question-library-browse-cross-discipline">
          <input
            type="checkbox"
            checked={props.query().cross_discipline}
            disabled={props.disabled()}
            onChange={(event) =>
              checkboxChange(event.currentTarget, props.query().cross_discipline, (checked) =>
                props.changeQuery({ cross_discipline: checked }),
              )
            }
          />
          Include this Subject across Disciplines
        </label>
      </Show>
    </section>
  );
}
