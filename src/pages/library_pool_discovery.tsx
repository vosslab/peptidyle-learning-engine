// Distinct Pool discovery with its own continuation and retained inspection state.

import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { QuestionPoolLibrarySummary } from "../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolBloomFacets } from "../../generated/api/QuestionPoolBloomFacets";
import type { QuestionPoolRevisionView } from "../../generated/api/QuestionPoolRevisionView";
import type { ContentClassificationClient } from "../api/content_classification";
import type { QuestionPoolLibraryClient } from "../api/question_pool_library";
import type { LibraryDiscussionClient } from "../api/library_discussion";
import type { BloomClassificationCorrectionClient } from "../api/bloom_classification";
import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";
import type { QuestionPoolLibraryFilter } from "../api/question_pool_library";
import { decodeQuestionId } from "../api/decoders/shared";
import { questionPoolLibraryFilter } from "../api/question_pool_library_filter";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
  isBloomCognitiveProcess,
  isBloomKnowledgeDimension,
} from "../api/decoders/bloom_classification";
import { ApiRequestError } from "../api/http_client/error";
import {
  EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  libraryClassificationFilter,
  type LibraryClassificationFilter,
} from "../api/library_classification_filter";
import { LibraryClassificationSearch } from "../components/library_classification_search";
import { LibraryDiscussionPanel } from "../components/library_discussion_panel";
import { QuestionPoolWatchControl } from "../components/question_pool_watch_control";
import {
  BloomClassificationEditor,
  BloomClassificationText,
} from "../components/bloom_classification";

type PoolInspectionTarget = {
  readonly publicId: QuestionPoolLibrarySummary["questionPoolRevision"]["questionPoolId"];
  readonly reference?: QuestionPoolLibrarySummary["questionPoolRevision"];
  readonly title: string;
};

function linkedPoolId(): PoolInspectionTarget["publicId"] | null {
  const values = new URLSearchParams(window.location.search).getAll("pool");
  if (values.length !== 1) return null;
  try {
    return decodeQuestionId(values[0], "pool");
  } catch {
    return null;
  }
}

function selectedCognitiveProcess(value: string): BloomCognitiveProcess | null {
  if (value === "") return null;
  if (isBloomCognitiveProcess(value)) return value;
  throw new Error("Bloom Cognitive Process selection is invalid");
}

function selectedKnowledgeDimension(value: string): BloomKnowledgeDimension | null {
  if (value === "") return null;
  if (isBloomKnowledgeDimension(value)) return value;
  throw new Error("Bloom Knowledge Dimension selection is invalid");
}

export function LibraryPoolDiscovery(props: {
  readonly client: QuestionPoolLibraryClient &
    LibraryDiscussionClient &
    BloomClassificationCorrectionClient;
  readonly classificationClient: ContentClassificationClient;
  /** Sysadmins inspect Pools read-only and never load private Watch state. */
  readonly mayWatchPools: boolean;
  /** Every active vetted Instructor may correct; Sysadmin inspection remains read-only. */
  readonly mayCorrectBloom: boolean;
}): JSX.Element {
  const [filter, setFilter] = createSignal<LibraryClassificationFilter>(
    EMPTY_LIBRARY_CLASSIFICATION_FILTER,
  );
  const [items, setItems] = createSignal<ReadonlyArray<QuestionPoolLibrarySummary>>([]);
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [bloomFacets, setBloomFacets] = createSignal<QuestionPoolBloomFacets | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal(false);
  const [invalidQuery, setInvalidQuery] = createSignal(false);
  const [text, setText] = createSignal("");
  const [tags, setTags] = createSignal("");
  const [validation, setValidation] = createSignal("");
  const [submitted, setSubmitted] = createSignal<QuestionPoolLibraryFilter>(
    questionPoolLibraryFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER),
  );
  const [inspecting, setInspecting] = createSignal<PoolInspectionTarget | null>(null);
  const [detail, setDetail] = createSignal<QuestionPoolRevisionView | null>(null);
  const [detailError, setDetailError] = createSignal(false);
  let listGeneration = 0;
  let detailGeneration = 0;
  let retryCursor: string | undefined;
  let returnButton: HTMLButtonElement | undefined;
  let poolSearchInput: HTMLInputElement | undefined;
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
      setBloomFacets(null);
    }
    try {
      const page = await props.client.listQuestionPools(after, 50, submittedFilter);
      if (generation !== listGeneration) return;
      setItems((previous) => (after === undefined ? page.items : [...previous, ...page.items]));
      setCursor(page.nextCursor);
      setBloomFacets(page.bloomFacets);
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

  function changeBloomFilter(
    change: Pick<
      QuestionPoolLibraryFilter,
      "bloom_cognitive_process" | "bloom_knowledge_dimension"
    >,
  ): void {
    const next = questionPoolLibraryFilter({ ...submitted(), ...change });
    setSubmitted(next);
    void readPage();
  }

  function clearSearch(): void {
    setFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER);
    setText("");
    setTags("");
    setValidation("");
    setSubmitted(questionPoolLibraryFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER));
    void readPage();
  }

  async function readDetail(target: PoolInspectionTarget): Promise<void> {
    const generation = ++detailGeneration;
    setDetail(null);
    setDetailError(false);
    try {
      const value =
        target.reference === undefined
          ? await props.client.getQuestionPool(target.publicId)
          : await props.client.getQuestionPoolRevision(target.reference);
      if (generation === detailGeneration) setDetail(value);
    } catch {
      if (generation === detailGeneration) setDetailError(true);
    }
  }

  function inspect(pool: QuestionPoolLibrarySummary, button: HTMLButtonElement): void {
    returnButton = button;
    returnScroll = window.scrollY;
    const target = {
      publicId: pool.questionPoolRevision.questionPoolId,
      reference: pool.questionPoolRevision,
      title: pool.metadata.title,
    };
    setInspecting(target);
    queueMicrotask(() => detailHeading?.focus());
    void readDetail(target);
  }

  function returnToResults(): void {
    ++detailGeneration;
    setInspecting(null);
    setDetail(null);
    setDetailError(false);
    if (items().length === 0) void readPage();
    queueMicrotask(() => {
      (returnButton ?? poolSearchInput)?.focus({ preventScroll: true });
      window.scrollTo({ top: returnScroll });
    });
  }

  function updateDetailBloom(bloom: QuestionPoolRevisionView["bloom"]): void {
    setDetail((current) => (current === null ? null : { ...current, bloom }));
  }

  onMount(() => {
    const publicId = linkedPoolId();
    if (publicId === null) {
      void readPage();
      return;
    }
    const target = { publicId, title: "Question Pool" };
    setInspecting(target);
    queueMicrotask(() => detailHeading?.focus());
    void readDetail(target);
  });
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
              ref={(element) => {
                poolSearchInput = element;
              }}
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
          <fieldset class="question-library-bloom-filters">
            <legend>Bloom Classification</legend>
            <label>
              Bloom Cognitive Process
              <select
                value={submitted().bloom_cognitive_process ?? ""}
                onChange={(event) =>
                  changeBloomFilter({
                    bloom_cognitive_process: selectedCognitiveProcess(event.currentTarget.value),
                    bloom_knowledge_dimension: submitted().bloom_knowledge_dimension ?? null,
                  })
                }
                disabled={loading()}
              >
                <option value="">Any</option>
                <For each={BLOOM_COGNITIVE_PROCESSES}>
                  {(value) => <option value={value}>{value}</option>}
                </For>
              </select>
            </label>
            <label>
              Bloom Knowledge Dimension
              <select
                value={submitted().bloom_knowledge_dimension ?? ""}
                onChange={(event) =>
                  changeBloomFilter({
                    bloom_cognitive_process: submitted().bloom_cognitive_process ?? null,
                    bloom_knowledge_dimension: selectedKnowledgeDimension(
                      event.currentTarget.value,
                    ),
                  })
                }
                disabled={loading()}
              >
                <option value="">Any</option>
                <For each={BLOOM_KNOWLEDGE_DIMENSIONS}>
                  {(value) => <option value={value}>{value}</option>}
                </For>
              </select>
            </label>
          </fieldset>
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
          {(submitted().tags ?? []).join(", ") || "Any"} | Bloom Cognitive Process:{" "}
          {submitted().bloom_cognitive_process ?? "Any"} | Bloom Knowledge Dimension:{" "}
          {submitted().bloom_knowledge_dimension ?? "Any"}
        </p>
        <Show when={bloomFacets()}>
          {(facets) => (
            <section class="question-library-bloom-report" aria-label="Pool Bloom report">
              <p>
                Counts describe every Question Pool matching all applied Pool filters, including
                both Bloom selections.
              </p>
              <div>
                <section aria-labelledby="pool-bloom-cognitive-counts">
                  <h3 id="pool-bloom-cognitive-counts">Cognitive Process</h3>
                  <dl>
                    <For each={facets().cognitiveProcesses}>
                      {(facet) => (
                        <div>
                          <dt>{facet.cognitiveProcess}</dt>
                          <dd>{facet.count}</dd>
                        </div>
                      )}
                    </For>
                  </dl>
                </section>
                <section aria-labelledby="pool-bloom-knowledge-counts">
                  <h3 id="pool-bloom-knowledge-counts">Knowledge Dimension</h3>
                  <dl>
                    <For each={facets().knowledgeDimensions}>
                      {(facet) => (
                        <div>
                          <dt>{facet.knowledgeDimension}</dt>
                          <dd>{facet.count}</dd>
                        </div>
                      )}
                    </For>
                  </dl>
                </section>
              </div>
            </section>
          )}
        </Show>
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
                  Discipline: {pool.metadata.disciplineName}
                  <Show when={pool.metadata.disciplineIsRetired}> (retired)</Show>
                </p>
                <p>
                  <BloomClassificationText bloom={pool.bloom} />
                </p>
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
              {detail()?.metadata.title ?? pool().title}
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
                    Discipline: {value().metadata.disciplineName}
                    <Show when={value().metadata.disciplineIsRetired}> (retired)</Show>
                  </p>
                  <p>
                    <BloomClassificationText bloom={value().bloom} />
                  </p>
                  <p>
                    Pool ID: {value().questionPoolRevision.questionPoolId} | Revision:{" "}
                    {value().questionPoolRevision.revisionNumber} | Members:{" "}
                    {value().members.length}
                  </p>
                  <p>Tags: {value().metadata.tags.join(", ") || "None"}</p>
                  <Show when={props.mayCorrectBloom}>
                    <BloomClassificationEditor
                      targetName="Question Pool"
                      revisionNumber={value().questionPoolRevision.revisionNumber}
                      bloom={value().bloom}
                      save={(request) =>
                        props.client
                          .correctQuestionPoolBloom(value().questionPoolRevision, request)
                          .then((receipt) => receipt.bloom)
                      }
                      loadCurrent={() =>
                        props.client
                          .getQuestionPoolRevision(value().questionPoolRevision)
                          .then((loaded) => loaded.bloom)
                      }
                      onCurrent={updateDetailBloom}
                      onConflictCurrent={() => void readPage()}
                      onAccepted={(_bloom, changed) => {
                        if (changed) void readPage();
                      }}
                    />
                  </Show>
                  <Show when={props.mayWatchPools}>
                    <QuestionPoolWatchControl
                      poolId={value().questionPoolRevision.questionPoolId}
                    />
                  </Show>
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
                  <LibraryDiscussionPanel
                    kind="questionPool"
                    publicId={value().questionPoolRevision.questionPoolId}
                  />
                </>
              )}
            </Show>
          </section>
        )}
      </Show>
    </section>
  );
}
