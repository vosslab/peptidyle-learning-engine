// Exact Bloom filters and whole-result-set counts for Question Library discovery.

import { For, Show, type JSX } from "solid-js";

import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
  isBloomCognitiveProcess,
  isBloomKnowledgeDimension,
} from "../api/decoders/bloom_classification";
import type {
  QuestionLibraryBrowseFacetAggregate,
  QuestionLibraryBrowseQuery,
} from "../pages/library_page_model";

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

export function LibraryBloomDiscovery(props: {
  readonly query: () => QuestionLibraryBrowseQuery;
  readonly aggregates: () => ReadonlyArray<QuestionLibraryBrowseFacetAggregate>;
  readonly reportAvailable: boolean;
  readonly disabled: boolean;
  readonly onChange: (change: Partial<QuestionLibraryBrowseQuery>) => void;
}): JSX.Element {
  const count = (facet: QuestionLibraryBrowseFacetAggregate["facet"], value: string): number =>
    props.aggregates().find((aggregate) => aggregate.facet === facet && aggregate.value === value)
      ?.count ?? 0;
  const hasReport = (): boolean =>
    props.reportAvailable &&
    props
      .aggregates()
      .some(
        (aggregate) =>
          aggregate.facet === "bloomCognitiveProcess" ||
          aggregate.facet === "bloomKnowledgeDimension",
      );

  return (
    <section class="question-library-bloom-discovery" aria-labelledby="bloom-discovery-heading">
      <h2 id="bloom-discovery-heading">Bloom Classification</h2>
      <div class="question-library-controls" role="group" aria-label="Bloom filters">
        <label>
          Cognitive Process
          <select
            value={props.query().bloomCognitiveProcess ?? ""}
            onChange={(event) =>
              props.onChange({
                bloomCognitiveProcess: selectedCognitiveProcess(event.currentTarget.value),
              })
            }
            disabled={props.disabled}
          >
            <option value="">Any</option>
            <For each={BLOOM_COGNITIVE_PROCESSES}>
              {(value) => <option value={value}>{value}</option>}
            </For>
          </select>
        </label>
        <label>
          Knowledge Dimension
          <select
            value={props.query().bloomKnowledgeDimension ?? ""}
            onChange={(event) =>
              props.onChange({
                bloomKnowledgeDimension: selectedKnowledgeDimension(event.currentTarget.value),
              })
            }
            disabled={props.disabled}
          >
            <option value="">Any</option>
            <For each={BLOOM_KNOWLEDGE_DIMENSIONS}>
              {(value) => <option value={value}>{value}</option>}
            </For>
          </select>
        </label>
      </div>
      <Show when={hasReport()}>
        <div class="question-library-bloom-report" aria-label="Bloom Classification report">
          <p>
            Counts describe all Questions matching every current filter, including both Bloom
            selections—not only the Questions loaded below.
          </p>
          <div>
            <section aria-labelledby="bloom-cognitive-counts">
              <h3 id="bloom-cognitive-counts">Cognitive Process</h3>
              <dl>
                <For each={BLOOM_COGNITIVE_PROCESSES}>
                  {(value) => (
                    <div>
                      <dt>{value}</dt>
                      <dd>{count("bloomCognitiveProcess", value)}</dd>
                    </div>
                  )}
                </For>
              </dl>
            </section>
            <section aria-labelledby="bloom-knowledge-counts">
              <h3 id="bloom-knowledge-counts">Knowledge Dimension</h3>
              <dl>
                <For each={BLOOM_KNOWLEDGE_DIMENSIONS}>
                  {(value) => (
                    <div>
                      <dt>{value}</dt>
                      <dd>{count("bloomKnowledgeDimension", value)}</dd>
                    </div>
                  )}
                </For>
              </dl>
            </section>
          </div>
        </div>
      </Show>
    </section>
  );
}
