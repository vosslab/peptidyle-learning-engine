import { For, Show, type JSX } from "solid-js";

import { LibraryClassificationSearch } from "../components/library_classification_search";
import type { ContentClassificationClient } from "../api/content_classification";
import type {
  QuestionLibraryBrowseFacetAggregate,
  QuestionLibraryBrowseQuery,
  QuestionLibraryFacetTruncation,
} from "./library_page_model";
import { backendLabel, questionTypeLabel, RetainedSelectOption } from "./library_page_helpers";

type Facet = QuestionLibraryBrowseFacetAggregate["facet"];

export type LibrarySearchFiltersProps = {
  readonly query: () => QuestionLibraryBrowseQuery;
  readonly facets: (
    facet: Facet,
  ) => () => ReadonlyArray<{ readonly value: string; readonly count: number }>;
  readonly truncation: () => QuestionLibraryFacetTruncation;
  readonly classificationClient: ContentClassificationClient;
  readonly disabled: () => boolean;
  readonly onChange: (change: Partial<QuestionLibraryBrowseQuery>) => void;
};

/** Library-owned exact filters; the shared search only supplies their placement and chips. */
export function LibrarySearchFilters(props: LibrarySearchFiltersProps): JSX.Element {
  const poolsOnly = (): boolean => props.query().kind === "pools";
  const questionOnlyDisabled = (): boolean => props.disabled() || poolsOnly();
  const option = (
    facet: Facet,
    current: string | null,
  ): ReadonlyArray<{ readonly value: string; readonly count: number }> =>
    props
      .facets(facet)()
      .filter((item) => item.value !== current);
  return (
    <div class="library-search-filters" role="group" aria-label="Question filters">
      <label>
        Show
        <select
          value={props.query().kind}
          disabled={props.disabled()}
          onChange={(event) =>
            props.onChange({
              kind: event.currentTarget.value as QuestionLibraryBrowseQuery["kind"],
            })
          }
        >
          <option value="both">Questions and Pools</option>
          <option value="questions">Questions</option>
          <option value="pools">Question Pools</option>
        </select>
      </label>
      <label>
        Question membership
        <select
          value={props.query().membership}
          disabled={props.disabled() || poolsOnly()}
          onChange={(event) =>
            props.onChange({
              membership: event.currentTarget.value as QuestionLibraryBrowseQuery["membership"],
            })
          }
        >
          <option value="noPool">Questions in no Pool</option>
          <option value="all">All Questions</option>
        </select>
      </label>
      <label>
        Owner account ID
        <input
          value={props.query().ownerAccountId ?? ""}
          disabled={props.disabled()}
          onChange={(event) =>
            props.onChange({ ownerAccountId: event.currentTarget.value || null })
          }
        />
      </label>
      <LibraryClassificationSearch
        value={props.query()}
        client={props.classificationClient}
        disabled={props.disabled()}
        onChange={props.onChange}
      />
      <label>
        Question Author
        <select
          value={props.query().authorName ?? ""}
          disabled={questionOnlyDisabled()}
          onChange={(event) => props.onChange({ authorName: event.currentTarget.value || null })}
        >
          <option value="">All Question Authors</option>
          <RetainedSelectOption value={props.query().authorName} label={(value) => value} />
          <For each={option("authorName", props.query().authorName)}>
            {(item) => <option value={item.value}>{`${item.value} (${item.count})`}</option>}
          </For>
        </select>
        <Show when={props.truncation().authorNames}>
          <span class="question-library-filter-truncated">
            More Question Authors match. Narrow the search or use <code>author:</code>.
          </span>
        </Show>
      </label>
      <label>
        Backend
        <select
          value={props.query().backend ?? ""}
          disabled={props.disabled()}
          onChange={(event) => props.onChange({ backend: event.currentTarget.value || null })}
        >
          <option value="">All backends</option>
          <RetainedSelectOption value={props.query().backend} label={backendLabel} />
          <For each={option("backend", props.query().backend)}>
            {(item) => (
              <option value={item.value}>{`${backendLabel(item.value)} (${item.count})`}</option>
            )}
          </For>
        </select>
      </label>
      <label>
        Tag
        <select
          value={props.query().tag ?? ""}
          disabled={props.disabled()}
          onChange={(event) => props.onChange({ tag: event.currentTarget.value || null })}
        >
          <option value="">All tags</option>
          <RetainedSelectOption value={props.query().tag} label={(value) => value} />
          <For each={option("tag", props.query().tag)}>
            {(item) => <option value={item.value}>{`${item.value} (${item.count})`}</option>}
          </For>
        </select>
        <Show when={props.truncation().tags}>
          <span class="question-library-filter-truncated">
            More tags match. Narrow the search or use <code>tags:</code>.
          </span>
        </Show>
      </label>
      <label>
        Subject name (additional filter)
        <select
          value={props.query().subjects[0] ?? ""}
          disabled={props.disabled()}
          onChange={(event) =>
            props.onChange({
              subjects: event.currentTarget.value ? [event.currentTarget.value] : [],
              topics: [],
            })
          }
        >
          <option value="">All subjects</option>
          <RetainedSelectOption value={props.query().subjects[0]} label={(value) => value} />
          <For each={option("subject", props.query().subjects[0] ?? null)}>
            {(item) => <option value={item.value}>{`${item.value} (${item.count})`}</option>}
          </For>
        </select>
      </label>
      <label>
        Topic name (additional filter)
        <select
          value={props.query().topics[0] ?? ""}
          disabled={props.disabled()}
          onChange={(event) =>
            props.onChange({ topics: event.currentTarget.value ? [event.currentTarget.value] : [] })
          }
        >
          <option value="">All topics</option>
          <RetainedSelectOption value={props.query().topics[0]} label={(value) => value} />
          <For each={option("topic", props.query().topics[0] ?? null)}>
            {(item) => <option value={item.value}>{`${item.value} (${item.count})`}</option>}
          </For>
        </select>
      </label>
      <label>
        Question Type
        <select
          value={props.query().questionType ?? ""}
          disabled={props.disabled()}
          onChange={(event) => props.onChange({ questionType: event.currentTarget.value || null })}
        >
          <option value="">All Question Types</option>
          <RetainedSelectOption value={props.query().questionType} label={questionTypeLabel} />
          <For each={option("questionType", props.query().questionType)}>
            {(item) => (
              <option
                value={item.value}
              >{`${questionTypeLabel(item.value)} (${item.count})`}</option>
            )}
          </For>
        </select>
      </label>
      <label>
        Question License
        <select
          value={props.query().questionLicense ?? ""}
          disabled={props.disabled()}
          onChange={(event) =>
            props.onChange({ questionLicense: event.currentTarget.value || null })
          }
        >
          <option value="">All Question Licenses</option>
          <RetainedSelectOption value={props.query().questionLicense} label={(value) => value} />
          <For each={option("questionLicense", props.query().questionLicense)}>
            {(item) => <option value={item.value}>{`${item.value} (${item.count})`}</option>}
          </For>
        </select>
      </label>
      <label>
        Capability
        <select
          value={props.query().capability ?? ""}
          disabled={questionOnlyDisabled()}
          onChange={(event) => props.onChange({ capability: event.currentTarget.value || null })}
        >
          <option value="">All capabilities</option>
          <RetainedSelectOption value={props.query().capability} label={(value) => value} />
          <For each={option("capability", props.query().capability)}>
            {(item) => <option value={item.value}>{`${item.value} (${item.count})`}</option>}
          </For>
        </select>
      </label>
    </div>
  );
}
