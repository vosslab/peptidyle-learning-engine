// Read-only optional hierarchy filters using the shared vocabulary and selectors.
import { Show, type JSX } from "solid-js";
import type { BlueprintCourseClassificationSearch } from "../api/blueprint_course";
import type {
  ContentClassificationClient,
  ContentClassificationItem,
} from "../api/content_classification";
import { ContentClassificationSelect } from "../components/content_classification_select";

export function emptyBlueprintClassificationSearch(): BlueprintCourseClassificationSearch {
  return {
    disciplineUuid: null,
    subjectUuid: null,
    topicUuid: null,
    subtopicUuid: null,
    crossDiscipline: false,
  };
}

export function BlueprintSearchClassification(props: {
  readonly client: ContentClassificationClient;
  readonly value: BlueprintCourseClassificationSearch;
  readonly onChange: (value: BlueprintCourseClassificationSearch, description: string) => void;
}): JSX.Element {
  // Labels describe the submitted identities, not an independently maintained vocabulary.
  const labels = new Map<string, string>();
  async function choices(
    load: () => Promise<ReadonlyArray<ContentClassificationItem>>,
  ): Promise<ReadonlyArray<ContentClassificationItem>> {
    const items = await load();
    for (const item of items) labels.set(item.uuid, item.name);
    return items;
  }
  function change(value: BlueprintCourseClassificationSearch): void {
    const names = [value.disciplineUuid, value.subjectUuid, value.topicUuid, value.subtopicUuid]
      .filter((uuid): uuid is string => uuid !== null)
      .map((uuid) => labels.get(uuid) ?? uuid);
    const description =
      names.join(" / ") + (value.crossDiscipline ? " (Subject across Disciplines)" : "");
    props.onChange(value, description);
  }
  return (
    <fieldset
      style={{
        display: "flex",
        "flex-wrap": "wrap",
        gap: "0.75rem",
        margin: "0",
        padding: "0.75rem",
        "min-width": "0",
      }}
    >
      <legend>Classification filters</legend>
      <ContentClassificationSelect
        label="Discipline"
        value={props.value.disciplineUuid}
        allowRetired
        load={() => choices(() => props.client.listDisciplinesIncludingRetired())}
        onChange={(disciplineUuid) =>
          change({ ...emptyBlueprintClassificationSearch(), disciplineUuid })
        }
      />
      <Show when={props.value.disciplineUuid}>
        <ContentClassificationSelect
          label="Subject"
          value={props.value.subjectUuid}
          parentUuid={props.value.disciplineUuid}
          load={(parent) => choices(() => props.client.listSubjects(parent))}
          onChange={(subjectUuid) =>
            change({
              ...props.value,
              subjectUuid,
              topicUuid: null,
              subtopicUuid: null,
              crossDiscipline: false,
            })
          }
        />
      </Show>
      <Show when={props.value.subjectUuid}>
        <ContentClassificationSelect
          label="Topic"
          value={props.value.topicUuid}
          parentUuid={props.value.subjectUuid}
          load={(parent) => choices(() => props.client.listTopics(parent))}
          onChange={(topicUuid) => change({ ...props.value, topicUuid, subtopicUuid: null })}
        />
      </Show>
      <Show when={props.value.topicUuid}>
        <ContentClassificationSelect
          label="Subtopic"
          value={props.value.subtopicUuid}
          parentUuid={props.value.topicUuid}
          load={(parent) => choices(() => props.client.listSubtopics(parent))}
          onChange={(subtopicUuid) => change({ ...props.value, subtopicUuid })}
        />
      </Show>
      <Show when={props.value.subjectUuid}>
        <label style={{ "flex-basis": "100%" }}>
          <input
            type="checkbox"
            checked={props.value.crossDiscipline}
            onChange={(event) =>
              change({ ...props.value, crossDiscipline: event.currentTarget.checked })
            }
          />{" "}
          Include this Subject across Disciplines
        </label>
      </Show>
    </fieldset>
  );
}
