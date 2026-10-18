import { Show, type JSX } from "solid-js";

import { inputChange, selectChange } from "../components/controlled_form_change";
import type { QuestionLibraryBrowseQuery } from "./library_page_model";

export type LibrarySearchFiltersProps = {
  readonly query: () => QuestionLibraryBrowseQuery;
  readonly disabled: () => boolean;
  readonly onChange: (change: Partial<QuestionLibraryBrowseQuery>) => void;
  readonly fixedOwner?: boolean;
};

/** Arranges the Library-specific criteria beside shared search controls. */
export function LibrarySearchFilters(props: LibrarySearchFiltersProps): JSX.Element {
  return (
    <Show
      when={
        props.query().kind !== "pools" ||
        !props.fixedOwner ||
        props.query().subject_uuids.length > 0
      }
    >
      <details class="library-search-filters">
        <summary>Search filters</summary>
        <div class="library-search-filters-body" role="group" aria-label="Question filters">
          <Show when={props.query().kind !== "pools"}>
            <label>
              Question membership
              <select
                value={props.query().questions}
                disabled={props.disabled()}
                onChange={(event) =>
                  selectChange(event.currentTarget, props.query().questions, (questions) =>
                    props.onChange({
                      questions: questions as QuestionLibraryBrowseQuery["questions"],
                    }),
                  )
                }
              >
                <option value="inNoPool">Questions in no Pool</option>
                <option value="all">All Questions</option>
              </select>
            </label>
          </Show>
          <Show when={!props.fixedOwner}>
            <label>
              Owner account ID
              <input
                value={props.query().ownerAccountId ?? ""}
                disabled={props.disabled()}
                onChange={(event) =>
                  inputChange(event.currentTarget, props.query().ownerAccountId ?? "", (value) =>
                    props.onChange({ ownerAccountId: value || null }),
                  )
                }
              />
            </label>
          </Show>
          <Show when={props.query().subject_uuids.length > 0}>
            <label>
              <input
                type="checkbox"
                checked={props.query().cross_discipline}
                disabled={props.disabled()}
                onChange={(event) =>
                  props.onChange({ cross_discipline: event.currentTarget.checked })
                }
              />
              Include this Subject across Disciplines
            </label>
          </Show>
        </div>
      </details>
    </Show>
  );
}
