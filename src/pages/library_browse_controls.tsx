// library_browse_controls.tsx - exact-facet navigation for the Question Library browse surface.

import { A } from "@solidjs/router";
import { For, Show, type Accessor, type JSX } from "solid-js";

import type {
  QuestionLibraryBrowseQuery,
  QuestionLibraryBrowseState,
  QuestionLibraryFacetTruncation,
} from "./library_page_model";

type BrowseFacet = "subject" | "topic" | "tag" | "questionType";

export interface LibraryBrowseControlsProps {
  readonly query: Accessor<QuestionLibraryBrowseQuery>;
  readonly hasExactBrowseFilters: () => boolean;
  readonly searchWithinResultsPath: () => string;
  readonly browsingState: Accessor<QuestionLibraryBrowseState>;
  readonly browseFacets: (
    facet: BrowseFacet,
  ) => () => ReadonlyArray<{ readonly value: string; readonly count: number }>;
  readonly facetTruncation: () => QuestionLibraryFacetTruncation;
  readonly questionTypeLabel: (value: string) => string;
  readonly changeQuery: (change: Partial<QuestionLibraryBrowseQuery>) => void;
  readonly startOver: () => void;
}

export function LibraryBrowseControls(props: LibraryBrowseControlsProps): JSX.Element {
  function browsingGroupsState(): "loading" | "ready" | "empty" | "error" {
    const current = props.browsingState();
    if ("rows" in current && current.rows.length > 0) return "ready";
    return current.kind === "initial" ? "loading" : current.kind;
  }

  return (
    <section class="question-library-browse-controls" aria-label="Browse Question Library">
      <div class="question-library-browse-heading">
        <div>
          <h2>{props.hasExactBrowseFilters() ? "Narrow these results" : "Choose a path"}</h2>
          <p>
            Counts describe all authorized Questions matching the current choices, not only the rows
            loaded below.
          </p>
        </div>
        <Show when={props.hasExactBrowseFilters()}>
          <A class="primary-action" href={props.searchWithinResultsPath()}>
            Search within results
          </A>
        </Show>
      </div>
      <Show when={browsingGroupsState() === "loading"}>
        <p class="loading-state" role="status">
          Loading Question Library groups...
        </p>
      </Show>
      <Show when={props.hasExactBrowseFilters()}>
        <div class="question-library-browse-active" aria-label="Current browse filters">
          <span>Browsing:</span>
          <For each={props.query().subjects}>{(subject) => <strong>{subject}</strong>}</For>
          <For each={props.query().topics}>{(topic) => <strong>{topic}</strong>}</For>
          <Show when={props.query().tag}>{(tag) => <strong>Tag: {tag()}</strong>}</Show>
          <Show when={props.query().questionType}>
            {(questionType) => <strong>{props.questionTypeLabel(questionType())}</strong>}
          </Show>
          <button class="quiet-action" type="button" onClick={props.startOver}>
            Start over
          </button>
        </div>
      </Show>
      <div class="question-library-browse-groups" hidden={browsingGroupsState() !== "ready"}>
        <section aria-labelledby="question-library-subjects-heading">
          <h3 id="question-library-subjects-heading">Subjects</h3>
          <div class="question-library-facet-choices">
            <For
              each={props.browseFacets("subject")()}
              fallback={
                <p class="question-library-facet-empty">
                  No Subject groups are available in this view. Browse with Tags or Question Types,
                  or use <A href={props.searchWithinResultsPath()}>Search Question Library</A>.
                </p>
              }
            >
              {(facet) => (
                <button
                  type="button"
                  aria-pressed={props.query().subjects.includes(facet.value)}
                  onClick={() =>
                    props.changeQuery({
                      subjects: [facet.value],
                      topics: [],
                    })
                  }
                >
                  <span>{facet.value}</span>
                  <strong>{facet.count}</strong>
                </button>
              )}
            </For>
          </div>
          <Show when={props.facetTruncation().subjects}>
            <p class="question-library-facet-truncated">
              More subjects are available. Choose one shown here or use Search Question Library to
              find a narrower match.
            </p>
          </Show>
        </section>
        <Show when={props.query().subjects.length > 0}>
          <section aria-labelledby="question-library-topics-heading">
            <h3 id="question-library-topics-heading">Topics in this subject</h3>
            <div class="question-library-facet-choices">
              <For each={props.browseFacets("topic")()}>
                {(facet) => (
                  <button
                    type="button"
                    aria-pressed={props.query().topics.includes(facet.value)}
                    onClick={() => props.changeQuery({ topics: [facet.value] })}
                  >
                    <span>{facet.value}</span>
                    <strong>{facet.count}</strong>
                  </button>
                )}
              </For>
            </div>
            <Show when={props.facetTruncation().topics}>
              <p class="question-library-facet-truncated">
                More topics match. Search within these results to reach a topic not shown here.
              </p>
            </Show>
          </section>
        </Show>
        <section aria-labelledby="question-library-tags-heading">
          <h3 id="question-library-tags-heading">Tags</h3>
          <div class="question-library-facet-choices">
            <For each={props.browseFacets("tag")()}>
              {(facet) => (
                <button
                  type="button"
                  aria-pressed={props.query().tag === facet.value}
                  onClick={() => props.changeQuery({ tag: facet.value })}
                >
                  <span>{facet.value}</span>
                  <strong>{facet.count}</strong>
                </button>
              )}
            </For>
          </div>
          <Show when={props.facetTruncation().tags}>
            <p class="question-library-facet-truncated">
              More tags match. Search within these results to reach a tag not shown here.
            </p>
          </Show>
        </section>
        <section aria-labelledby="question-library-types-heading">
          <h3 id="question-library-types-heading">Question Types</h3>
          <div class="question-library-facet-choices">
            <For each={props.browseFacets("questionType")()}>
              {(facet) => (
                <button
                  type="button"
                  aria-pressed={props.query().questionType === facet.value}
                  onClick={() => props.changeQuery({ questionType: facet.value })}
                >
                  <span>{props.questionTypeLabel(facet.value)}</span>
                  <strong>{facet.count}</strong>
                </button>
              )}
            </For>
          </div>
        </section>
      </div>
    </section>
  );
}
