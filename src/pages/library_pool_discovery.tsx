// Distinct Pool discovery with its own continuation and retained inspection state.

import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { QuestionPoolLibrarySummary } from "../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolRevisionView } from "../../generated/api/QuestionPoolRevisionView";
import type { ContentClassificationClient } from "../api/content_classification";
import type { QuestionPoolLibraryClient } from "../api/question_pool_library";
import type { QuestionPoolLibraryFilter } from "../api/question_pool_library";
import { questionPoolLibraryFilter } from "../api/question_pool_library_filter";
import { ApiRequestError } from "../api/http_client/error";
import {
  EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  libraryClassificationFilter,
  type LibraryClassificationFilter,
} from "../api/library_classification_filter";
import { LibraryClassificationSearch } from "../components/library_classification_search";

export function LibraryPoolDiscovery(props: {
  readonly client: QuestionPoolLibraryClient;
  readonly classificationClient: ContentClassificationClient;
}): JSX.Element {
  const [filter, setFilter] = createSignal<LibraryClassificationFilter>(
    EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  );
  const [items, setItems] = createSignal<ReadonlyArray<QuestionPoolLibrarySummary>>([]);
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal(false);
  const [invalidQuery, setInvalidQuery] = createSignal(false);
  const [text, setText] = createSignal("");
  const [tags, setTags] = createSignal("");
  const [validation, setValidation] = createSignal("");
  const [submitted, setSubmitted] = createSignal<QuestionPoolLibraryFilter>(
    questionPoolLibraryFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER),
  );
  const [inspecting, setInspecting] = createSignal<QuestionPoolLibrarySummary | null>(null);
  const [detail, setDetail] = createSignal<QuestionPoolRevisionView | null>(null);
  const [detailError, setDetailError] = createSignal(false);
  let listGeneration = 0;
  let detailGeneration = 0;
  let retryCursor: string | undefined;
  let returnButton: HTMLButtonElement | undefined;
  let detailHeading: HTMLHeadingElement | undefined;
  let returnScroll = 0;

  async function readPage(after?: string): Promise<void> {
    const generation = ++listGeneration;
    const submittedFilter = submitted();
    retryCursor = after;
    setLoading(true);
    setError(false);
    setInvalidQuery(false);
    if (after === undefined) {
      setItems([]);
      setCursor(null);
    }
    try {
      const page = await props.client.listQuestionPools(after, 50, submittedFilter);
      if (generation !== listGeneration) return;
      setItems((previous) => (after === undefined ? page.items : [...previous, ...page.items]));
      setCursor(page.nextCursor);
    } catch (cause) {
      if (generation !== listGeneration) return;
      setError(true);
      setInvalidQuery(cause instanceof ApiRequestError && cause.status === 400);
    } finally {
      if (generation === listGeneration) setLoading(false);
    }
  }

  function changeFilter(change: Partial<LibraryClassificationFilter>): void {
    setFilter((previous) => libraryClassificationFilter({ ...previous, ...change }));
    // Hierarchy changes apply to the current submitted search, never unsent drafts.
    setSubmitted((previous) => questionPoolLibraryFilter({ ...previous, ...filter() }));
    void readPage();
  }

  function submitSearch(): void {
    try {
      const next = questionPoolLibraryFilter({
        ...filter(),
        text: text(),
        tags: tags()
          .split(",")
          .map((tag) => tag.trim())
          .filter(Boolean),
      });
      setValidation("");
      setSubmitted(next);
      setText(next.text ?? "");
      setTags((next.tags ?? []).join(", "));
      void readPage();
    } catch (cause) {
      setValidation(cause instanceof Error ? cause.message : "Check your Pool search.");
    }
  }

  function clearSearch(): void {
    setFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER);
    setText("");
    setTags("");
    setValidation("");
    setSubmitted(questionPoolLibraryFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER));
    void readPage();
  }

  async function readDetail(pool: QuestionPoolLibrarySummary): Promise<void> {
    const generation = ++detailGeneration;
    setDetail(null);
    setDetailError(false);
    try {
      const value = await props.client.getQuestionPool(pool.questionPoolRevision.questionPoolId);
      if (generation === detailGeneration) setDetail(value);
    } catch {
      if (generation === detailGeneration) setDetailError(true);
    }
  }

  function inspect(pool: QuestionPoolLibrarySummary, button: HTMLButtonElement): void {
    returnButton = button;
    returnScroll = window.scrollY;
    setInspecting(pool);
    queueMicrotask(() => detailHeading?.focus());
    void readDetail(pool);
  }

  function returnToResults(): void {
    ++detailGeneration;
    setInspecting(null);
    setDetail(null);
    setDetailError(false);
    queueMicrotask(() => {
      returnButton?.focus({ preventScroll: true });
      window.scrollTo({ top: returnScroll });
    });
  }

  onMount(() => void readPage());
  onCleanup(() => {
    ++listGeneration;
    ++detailGeneration;
  });

  return (
    <section aria-label="Question Pool discovery">
      <div hidden={inspecting() !== null}>
        <h2>Published Question Pools</h2>
        <p>
          Search each Pool's own title, description, Tags and classification, not its member
          Questions. The Question search box does not filter Pools.
        </p>
        <form
          class="question-library-controls"
          onSubmit={(event) => {
            event.preventDefault();
            submitSearch();
          }}
        >
          <label class="question-library-pool-search-control">
            Search published Pools
            <input
              type="search"
              value={text()}
              onInput={(event) => setText(event.currentTarget.value)}
              aria-describedby="pool-search-help"
              placeholder="Search Pools with ordinary words"
            />
          </label>
          <label>
            Pool Tags (comma-separated)
            <input
              value={tags()}
              onInput={(event) => setTags(event.currentTarget.value)}
              aria-describedby="pool-tags-help"
              placeholder="review, practice"
            />
          </label>
          <button
            type="button"
            onClick={() => {
              setTags("");
              setSubmitted((previous) => questionPoolLibraryFilter({ ...previous, tags: [] }));
              void readPage();
            }}
          >
            Clear Pool Tags
          </button>
          <p id="pool-tags-help" class="question-library-pool-search-help">
            Match any exact Tag, ignoring case. Search Pools applies your edits.
          </p>
          <details id="pool-search-help" class="question-library-pool-search-help">
            <summary>Pool search syntax</summary>
            <p>
              Combine words, use quotes for an exact phrase, or a minus sign to exclude a term.
              Fields: discipline:, subject:, topic:, subtopic:, tags: (for example,
              topic:"chromosomal inheritance"). Type and author fields are not supported for Pools.
            </p>
          </details>
          <LibraryClassificationSearch
            value={filter()}
            client={props.classificationClient}
            onChange={changeFilter}
          />
          <button type="submit" disabled={loading()}>
            Search Pools
          </button>
          <button type="button" onClick={clearSearch}>
            Clear Pool filters
          </button>
        </form>
        <Show when={validation()}>
          <p role="alert">{validation()}</p>
        </Show>
        <p>
          Applied Pool search: {submitted().text || "All words"} | Tags:{" "}
          {(submitted().tags ?? []).join(", ") || "Any"}
        </p>
        <Show when={loading()}>
          <p role="status">Loading published Pools...</p>
        </Show>
        <Show when={error()}>
          <p role="alert">
            {invalidQuery()
              ? "Pool search was not accepted. Check quotes and field syntax; type: and author: are not supported. Edit your search and choose Search Pools."
              : "Could not load Pools. Your applied search is retained. Retry this request."}
          </p>
          <Show when={!invalidQuery()}>
            <button type="button" onClick={() => void readPage(retryCursor)}>
              Retry Pool results
            </button>
          </Show>
        </Show>
        <Show when={!loading() && !error() && items().length === 0}>
          <p>No published Pools match these filters. Change or clear the Pool filters.</p>
        </Show>
        <div class="question-library-results" aria-label="Pool results" aria-busy={loading()}>
          <For each={items()}>
            {(pool) => (
              <article class="question-library-pool-row">
                <h3>{pool.metadata.title}</h3>
                {/* ASVS 1.2.1: metadata is rendered as text, never injected HTML. */}
                <p>{pool.metadata.description}</p>
                <p>
                  Pool ID: {pool.questionPoolRevision.questionPoolId} | Revision:{" "}
                  {pool.questionPoolRevision.revisionNumber} | Members: {pool.memberCount}
                </p>
                <button type="button" onClick={(event) => inspect(pool, event.currentTarget)}>
                  Inspect Pool {pool.metadata.title}
                </button>
              </article>
            )}
          </For>
        </div>
        <Show when={cursor() !== null && !error()}>
          <button
            type="button"
            disabled={loading()}
            onClick={() => void readPage(cursor() ?? undefined)}
          >
            Load more Pools
          </button>
        </Show>
      </div>
      <Show when={inspecting()}>
        {(pool) => (
          <section aria-label="Pool inspection">
            <h2
              ref={(element) => {
                detailHeading = element;
              }}
              tabindex="-1"
            >
              {detail()?.metadata.title ?? pool().metadata.title}
            </h2>
            <button type="button" onClick={returnToResults}>
              Return to Pool results
            </button>
            <Show when={!detail() && !detailError()}>
              <p role="status">Loading Pool detail...</p>
            </Show>
            <Show when={detailError()}>
              <p role="alert">Could not load this Pool. Retry or return to your results.</p>
              <button type="button" onClick={() => void readDetail(pool())}>
                Retry Pool detail
              </button>
            </Show>
            <Show when={detail()}>
              {(value) => (
                <>
                  <p>{value().metadata.description}</p>
                  <p>
                    Pool ID: {value().questionPoolRevision.questionPoolId} | Revision:{" "}
                    {value().questionPoolRevision.revisionNumber} | Members:{" "}
                    {value().members.length}
                  </p>
                  <p>Tags: {value().metadata.tags.join(", ") || "None"}</p>
                  <h3>Exact Question Revisions</h3>
                  <ol>
                    <For each={value().members}>
                      {(member) => (
                        <li>
                          {member.questionRevision.questionId} | Revision:{" "}
                          {member.questionRevision.revisionNumber}
                        </li>
                      )}
                    </For>
                  </ol>
                </>
              )}
            </Show>
          </section>
        )}
      </Show>
    </section>
  );
}
