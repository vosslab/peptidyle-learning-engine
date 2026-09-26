// Distinct Pool discovery with its own continuation and retained inspection state.

import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { QuestionPoolLibrarySummary } from "../../generated/api/QuestionPoolLibrarySummary";
import type { QuestionPoolBloomFacets } from "../../generated/api/QuestionPoolBloomFacets";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
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
import { RecordSequence } from "../components/record_list/record_sequence";
import {
  RecordList,
  type RecordContent,
  type RecordFact,
} from "../components/record_list/record_list";
import {
  RecordPageControls,
  type RecordPageSize,
} from "../components/record_list/record_page_controls";
import {
  BloomClassificationEditor,
  BloomClassificationText,
} from "../components/bloom_classification";
import type { QuestionPoolMemberView } from "../../generated/api/QuestionPoolMemberView";

type PoolInspectionTarget = {
  readonly publicId: QuestionPoolLibrarySummary["questionPoolId"];
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

function poolMemberContent(member: QuestionPoolMemberView): RecordContent {
  const revision = member.publishedQuestionRevisionTuple;
  return {
    title: member.question.question_library.summary.metadata.questionTitle,
    details: [
      { kind: "text", label: "Published Question ID", value: revision.publishedQuestionId },
      { kind: "text", label: "Revision", value: String(revision.revisionNumber) },
    ],
    actions: [],
  };
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
  const [pageCursor, setPageCursor] = createSignal<string | null>(null);
  const [previousCursors, setPreviousCursors] = createSignal<ReadonlyArray<string | null>>([]);
  const [nextCursor, setNextCursor] = createSignal<string | null>(null);
  const [pageSize, setPageSize] = createSignal<RecordPageSize>(50);
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
  const [detail, setDetail] = createSignal<QuestionPoolView | null>(null);
  const [detailError, setDetailError] = createSignal(false);
  let listGeneration = 0;
  let detailGeneration = 0;
  let retryCursor: string | null = null;
  let retryPrevious: ReadonlyArray<string | null> = [];
  let returnButton: HTMLButtonElement | undefined;
  let poolSearchInput: HTMLInputElement | undefined;
  let detailHeading: HTMLHeadingElement | undefined;
  let returnScroll = 0;

  async function readPage(
    requestedCursor: string | null,
    requestedPrevious: ReadonlyArray<string | null>,
  ): Promise<void> {
    const generation = ++listGeneration;
    const submittedFilter = submitted();
    const replacesQuery = requestedCursor === null && requestedPrevious.length === 0;
    retryCursor = requestedCursor;
    retryPrevious = requestedPrevious;
    setLoading(true);
    setError(false);
    setInvalidQuery(false);
    if (replacesQuery) {
      setItems([]);
      setPageCursor(null);
      setPreviousCursors([]);
      setNextCursor(null);
      setBloomFacets(null);
    }
    try {
      const page = await props.client.listQuestionPools(
        requestedCursor ?? undefined,
        pageSize(),
        submittedFilter,
      );
      if (generation !== listGeneration) return;
      setItems(page.items);
      setPageCursor(requestedCursor);
      setPreviousCursors(requestedPrevious);
      setNextCursor(page.nextCursor);
      setBloomFacets(page.bloomFacets);
    } catch (cause) {
      if (generation !== listGeneration) return;
      setError(true);
      setInvalidQuery(cause instanceof ApiRequestError && cause.status === 400);
    } finally {
      if (generation === listGeneration) setLoading(false);
    }
  }

  function loadFirstPage(nextPageSize: RecordPageSize = pageSize()): void {
    if (nextPageSize !== pageSize()) setPageSize(nextPageSize);
    void readPage(null, []);
  }

  function loadNextPage(): void {
    const cursor = nextCursor();
    if (cursor === null || loading()) return;
    void readPage(cursor, [...previousCursors(), pageCursor()]);
  }

  function loadPreviousPage(): void {
    const cursors = previousCursors();
    const cursor = cursors.length === 0 ? undefined : cursors[cursors.length - 1];
    if (cursor === undefined || loading()) return;
    void readPage(cursor, cursors.slice(0, -1));
  }

  function poolContent(pool: QuestionPoolLibrarySummary): RecordContent {
    const details: RecordFact[] = [
      {
        kind: "courseClassification",
        value: {
          disciplineUuid: pool.metadata.disciplineUuid,
          subjectUuid: pool.metadata.subjectUuid,
          topicUuid: pool.metadata.topicUuid,
          subtopicUuid: pool.metadata.subtopicUuid,
          tags: pool.metadata.tags,
        },
      },
    ];
    const bloom = pool.bloom;
    if (bloom !== null) {
      details.push({
        kind: "text",
        label: "Bloom",
        value: `${bloom.cognitiveProcess} / ${bloom.knowledgeDimension}`,
      });
    }
    details.push(
      { kind: "text", label: "Pool ID", value: pool.questionPoolId },
      { kind: "text", label: "Edit", value: String(pool.questionPoolEditNumber) },
      { kind: "text", label: "Members", value: String(pool.memberCount) },
    );
    return {
      title: pool.metadata.title,
      description: pool.metadata.description,
      details,
      actions: [
        {
          id: "inspect",
          kind: "command",
          label: `Inspect Pool ${pool.metadata.title}`,
          onClick: (event) => inspect(pool, event.currentTarget),
        },
      ],
    };
  }

  function changeFilter(change: Partial<LibraryClassificationFilter>): void {
    setFilter((previous) => libraryClassificationFilter({ ...previous, ...change }));
    // Hierarchy changes apply to the current submitted search, never unsent drafts.
    setSubmitted((previous) => questionPoolLibraryFilter({ ...previous, ...filter() }));
    loadFirstPage();
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
      loadFirstPage();
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
    loadFirstPage();
  }

  function clearSearch(): void {
    setFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER);
    setText("");
    setTags("");
    setValidation("");
    setSubmitted(questionPoolLibraryFilter(EMPTY_LIBRARY_CLASSIFICATION_FILTER));
    loadFirstPage();
  }

  async function readDetail(target: PoolInspectionTarget): Promise<void> {
    const generation = ++detailGeneration;
    setDetail(null);
    setDetailError(false);
    try {
      const value = await props.client.getQuestionPool(target.publicId);
      if (generation === detailGeneration) setDetail(value);
    } catch {
      if (generation === detailGeneration) setDetailError(true);
    }
  }

  function inspect(pool: QuestionPoolLibrarySummary, button: HTMLButtonElement): void {
    returnButton = button;
    returnScroll = window.scrollY;
    const target = {
      publicId: pool.questionPoolId,
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
    if (items().length === 0) loadFirstPage();
    queueMicrotask(() => {
      const focusTarget = returnButton?.isConnected ? returnButton : poolSearchInput;
      focusTarget?.focus({ preventScroll: true });
      window.scrollTo({ top: returnScroll });
    });
  }

  function updateDetailBloom(bloom: QuestionPoolView["bloom"]): void {
    setDetail((current) => (current === null ? null : { ...current, bloom }));
  }

  onMount(() => {
    const publicId = linkedPoolId();
    if (publicId === null) {
      loadFirstPage();
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
              loadFirstPage();
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
            <button type="button" onClick={() => void readPage(retryCursor, retryPrevious)}>
              Retry Pool results
            </button>
          </Show>
        </Show>
        <Show when={!loading() && !error() && items().length === 0}>
          <p>No published Pools match these filters. Change or clear the Pool filters.</p>
        </Show>
        <div class="question-library-results" aria-busy={loading()}>
          <RecordList
            rows={items()}
            content={poolContent}
            recordId={(pool) => pool.questionPoolId}
            state={{ kind: "ready" }}
            ariaLabel="Pool results"
            emptyState={{ title: "No published Pools match these filters." }}
          />
        </div>
        <RecordPageControls
          ariaLabel="Published Question Pool pages"
          hasPrevious={previousCursors().length > 0}
          hasNext={nextCursor() !== null}
          loading={loading()}
          onPrevious={loadPreviousPage}
          onNext={loadNextPage}
          pageSize={pageSize()}
          onPageSizeChange={loadFirstPage}
        />
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
                  <Show when={value().bloom}>
                    {(bloom) => (
                      <p>
                        <BloomClassificationText bloom={bloom()} />
                      </p>
                    )}
                  </Show>
                  <p>
                    Pool ID: {value().questionPoolId} | Edit: {value().questionPoolEditNumber} |
                    Members: {value().members.length}
                  </p>
                  <p>Tags: {value().metadata.tags.join(", ") || "None"}</p>
                  <Show when={props.mayCorrectBloom && value().bloom}>
                    {(bloom) => (
                      <BloomClassificationEditor
                        targetName="Question Pool"
                        contentMarkerKind="Edit"
                        contentMarkerNumber={value().questionPoolEditNumber}
                        bloom={bloom()}
                        save={(request) =>
                          props.client
                            .correctQuestionPoolBloom(value().questionPoolId, request)
                            .then((receipt) => receipt.bloom)
                        }
                        loadCurrent={() =>
                          props.client.getQuestionPool(value().questionPoolId).then((loaded) => {
                            if (loaded.bloom === null) {
                              throw new Error("Bloom Classification is not assigned.");
                            }
                            return loaded.bloom;
                          })
                        }
                        onCurrent={updateDetailBloom}
                        onConflictCurrent={() => loadFirstPage()}
                        onAccepted={(_bloom, changed) => {
                          if (changed) loadFirstPage();
                        }}
                      />
                    )}
                  </Show>
                  <Show when={props.mayWatchPools}>
                    <QuestionPoolWatchControl poolId={value().questionPoolId} />
                  </Show>
                  <h3>Exact Question Revisions</h3>
                  <RecordSequence
                    rows={value().members}
                    content={poolMemberContent}
                    recordId={(member) =>
                      `${member.publishedQuestionRevisionTuple.publishedQuestionId}:${member.publishedQuestionRevisionTuple.revisionNumber}`
                    }
                    state={{ kind: "ready" }}
                    ariaLabel="Ordered exact Question Revisions"
                    emptyState={{ title: "No Question Revisions are in this Pool." }}
                  />
                  <LibraryDiscussionPanel kind="questionPool" publicId={value().questionPoolId} />
                </>
              )}
            </Show>
          </section>
        )}
      </Show>
    </section>
  );
}
