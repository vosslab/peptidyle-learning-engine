// library_browse_controls.tsx - progressive navigation over the shared Library query.

import { A } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  type Accessor,
  type JSX,
} from "solid-js";

import type { LibraryFacetField } from "../../generated/api/LibraryFacetField";
import { checkboxChange } from "../components/controlled_form_change";
import type {
  RecordFilterBinding,
  RecordFilterChoice,
  RecordFilterChoices,
} from "../components/data_sheet/record_fields";
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
  readonly expandedByDefault: () => boolean;
  readonly onSelectedChoices: (choices: ReadonlyArray<RecordFilterChoice>) => void;
}): JSX.Element {
  const [search, setSearch] = createSignal("");
  const [page, setPage] = createSignal<RecordFilterChoices | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal(false);
  const [retry, setRetry] = createSignal(0);
  let request: AbortController | undefined;
  let requestNumber = 0;
  let disclosure!: HTMLDetailsElement;

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
  const scopeKey = createMemo(() =>
    levels
      .slice(0, levels.findIndex((level) => level.field === props.field) + 1)
      .map((level) => (props.filters.values.get(level.field) ?? []).join(","))
      .join("|"),
  );
  const selectedChoices = createMemo(() =>
    visibleChoices().filter((choice) => selectedKeys().includes(choice.key)),
  );
  createEffect(() => {
    const choices = selectedChoices();
    if (choices.length === selectedKeys().length) props.onSelectedChoices(choices);
  });
  // Reset presentation only when this classification path or the layout changes.
  createEffect(() => {
    scopeKey();
    const open = props.expandedByDefault();
    if (!open && disclosure.contains(document.activeElement)) {
      disclosure.querySelector("summary")?.focus();
    }
    disclosure.open = open;
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
    props.onSelectedChoices(
      visibleChoices().filter((choice) => choice.key === key || selected.includes(choice.key)),
    );
    props.filters.onChange(
      props.field,
      selected.includes(key) ? selected.filter((value) => value !== key) : [key],
    );
  }

  return (
    <section aria-label={props.label}>
      <details
        class="question-library-browse-level"
        ref={(element) => {
          disclosure = element;
        }}
      >
        <summary>
          <strong>{props.label}</strong>
          <Show
            when={selectedKeys().length > 0}
            fallback={<span>Choose {props.label.toLowerCase()}</span>}
          >
            <span>
              {selectedChoices()
                .map((choice) => `${choice.label} (${choice.count})`)
                .join(", ") || `${selectedKeys().length} selected`}
            </span>
          </Show>
        </summary>
        <div class="question-library-browse-level-body">
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
        </div>
      </details>
    </section>
  );
}

export function LibraryBrowseControls(props: LibraryBrowseControlsProps): JSX.Element {
  const [wideScreen, setWideScreen] = createSignal(true);
  const [selectedChoices, setSelectedChoices] = createSignal<
    Partial<Record<ClassificationField, ReadonlyArray<RecordFilterChoice>>>
  >({});
  let classification!: HTMLDetailsElement;
  const expandedByDefault = createMemo(() => wideScreen() || !props.hasCriteria());
  const classificationKey = createMemo(() =>
    [
      ...levels.map(({ field }) => (props.filters.values.get(field) ?? []).join(",")),
      String(props.query().cross_discipline),
    ].join("|"),
  );
  const hasClassification = createMemo(() =>
    levels.some(({ field }) => (props.filters.values.get(field) ?? []).length > 0),
  );
  const selectedPath = createMemo(() =>
    levels
      .flatMap(({ field, label }) => {
        const selected = props.filters.values.get(field) ?? [];
        return selected.map(
          (key) =>
            selectedChoices()[field]?.find((choice) => choice.key === key)?.label ??
            `${label} selected`,
        );
      })
      .join(" / "),
  );
  // Keep a user's disclosure choice until the classification or layout changes.
  createEffect(() => {
    classificationKey();
    const open = wideScreen() || !hasClassification();
    const summary = classification.querySelector("summary");
    if (
      !open &&
      classification.open &&
      document.activeElement !== summary &&
      classification.contains(document.activeElement)
    ) {
      summary?.focus();
    }
    classification.open = open;
  });
  onMount(() => {
    const query = window.matchMedia("(min-width: 64rem)");
    const update = (): void => {
      setWideScreen(query.matches);
    };
    update();
    query.addEventListener("change", update);
    onCleanup(() => query.removeEventListener("change", update));
  });
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
      <details
        class="question-library-browse-classification"
        ref={(element) => {
          classification = element;
        }}
      >
        <summary>
          <strong>Classification</strong>
          <span>{selectedPath() || "Choose discipline"}</span>
          <Show when={props.query().cross_discipline}>
            <span>Across Disciplines</span>
          </Show>
        </summary>
        <div class="question-library-browse-classification-body">
          <div class="question-library-browse-levels">
            <For each={visibleLevels()}>
              {(level) => (
                <BrowseLevel
                  field={level.field}
                  label={level.label}
                  query={props.query}
                  filters={props.filters}
                  disabled={props.disabled}
                  expandedByDefault={expandedByDefault}
                  onSelectedChoices={(choices) =>
                    setSelectedChoices((current) => ({ ...current, [level.field]: choices }))
                  }
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
        </div>
      </details>
    </section>
  );
}
