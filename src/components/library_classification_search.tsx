// Optional identity filters, shared by Library discovery surfaces, with no vocabulary writes.

import { Show, type JSX } from "solid-js";
import type { ContentClassificationClient } from "../api/content_classification";
import {
  libraryClassificationChange,
  type LibraryClassificationFilter,
} from "../api/library_classification_filter";
import { ContentClassificationSelect } from "./content_classification_select";
import { checkboxChange } from "./controlled_form_change";

export function LibraryClassificationSearch(props: {
  readonly value: LibraryClassificationFilter;
  readonly client: ContentClassificationClient;
  readonly disabled?: boolean;
  readonly onChange: (change: Partial<LibraryClassificationFilter>) => void;
}): JSX.Element {
  return (
    <>
      <ContentClassificationSelect
        label="Discipline"
        value={props.value.discipline_uuids[0] ?? null}
        allowRetired
        disabled={props.disabled}
        load={() => props.client.listDisciplinesIncludingRetired()}
        onChange={(uuid) => props.onChange(libraryClassificationChange("discipline_uuids", uuid))}
      />
      <ContentClassificationSelect
        label="Subject"
        value={props.value.subject_uuids[0] ?? null}
        parentUuid={props.value.discipline_uuids[0] ?? null}
        disabled={props.disabled}
        load={(uuid) => props.client.listSubjects(uuid)}
        onChange={(uuid) => props.onChange(libraryClassificationChange("subject_uuids", uuid))}
      />
      <Show when={props.value.subject_uuids.length > 0}>
        <label>
          <input
            type="checkbox"
            checked={props.value.cross_discipline}
            disabled={props.disabled}
            onChange={(event) =>
              checkboxChange(event.currentTarget, props.value.cross_discipline, (checked) =>
                props.onChange({ cross_discipline: checked }),
              )
            }
          />
          Include this Subject across Disciplines
        </label>
      </Show>
      <ContentClassificationSelect
        label="Topic"
        value={props.value.topic_uuids[0] ?? null}
        parentUuid={props.value.subject_uuids[0] ?? null}
        disabled={props.disabled}
        load={(uuid) => props.client.listTopics(uuid)}
        onChange={(uuid) => props.onChange(libraryClassificationChange("topic_uuids", uuid))}
      />
      <ContentClassificationSelect
        label="Subtopic"
        value={props.value.subtopic_uuids[0] ?? null}
        parentUuid={props.value.topic_uuids[0] ?? null}
        disabled={props.disabled}
        load={(uuid) => props.client.listSubtopics(uuid)}
        onChange={(uuid) => props.onChange(libraryClassificationChange("subtopic_uuids", uuid))}
      />
    </>
  );
}
