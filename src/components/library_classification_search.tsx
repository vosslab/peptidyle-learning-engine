// Optional identity filters, shared by Library discovery surfaces, with no vocabulary writes.

import { Show, type JSX } from "solid-js";
import type { ContentClassificationClient } from "../api/content_classification";
import { libraryClassificationChange, type LibraryClassificationFilter } from "../api/library_classification_filter";
import { ContentClassificationSelect } from "./content_classification_select";

export function LibraryClassificationSearch(props: {
  readonly value: LibraryClassificationFilter;
  readonly client: ContentClassificationClient;
  readonly disabled?: boolean;
  readonly onChange: (change: Partial<LibraryClassificationFilter>) => void;
}): JSX.Element {
  return <>
    <ContentClassificationSelect label="Discipline" value={props.value.discipline_uuid}
      disabled={props.disabled} load={() => props.client.listDisciplines()}
      onChange={(uuid) => props.onChange(libraryClassificationChange("discipline_uuid", uuid))} />
    <ContentClassificationSelect label="Subject" value={props.value.subject_uuid}
      parentUuid={props.value.discipline_uuid} disabled={props.disabled}
      load={(uuid) => props.client.listSubjects(uuid)}
      onChange={(uuid) => props.onChange(libraryClassificationChange("subject_uuid", uuid))} />
    <Show when={props.value.subject_uuid !== null}>
      <label>
        <input type="checkbox" checked={props.value.cross_discipline} disabled={props.disabled}
          onChange={(event) => props.onChange({ cross_discipline: event.currentTarget.checked })} />
        Include this Subject across Disciplines
      </label>
    </Show>
    <ContentClassificationSelect label="Topic" value={props.value.topic_uuid}
      parentUuid={props.value.subject_uuid} disabled={props.disabled}
      load={(uuid) => props.client.listTopics(uuid)}
      onChange={(uuid) => props.onChange(libraryClassificationChange("topic_uuid", uuid))} />
    <ContentClassificationSelect label="Subtopic" value={props.value.subtopic_uuid}
      parentUuid={props.value.topic_uuid} disabled={props.disabled}
      load={(uuid) => props.client.listSubtopics(uuid)}
      onChange={(uuid) => props.onChange(libraryClassificationChange("subtopic_uuid", uuid))} />
  </>;
}
