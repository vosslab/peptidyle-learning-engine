// Render vocabulary names. A missing name stays a label and never the UUID.
import { createResource, Show, type JSX } from "solid-js";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { ContentClassificationItem } from "../api/content_classification";
import { useApplicationApi } from "../api/application_api";
import "./course_classification.css";

export function CourseClassificationSummary(props: {
  readonly value: CourseClassification;
}): JSX.Element {
  const api = useApplicationApi();
  const [disciplines] = createResource(() => api.client.listDisciplinesIncludingRetired());
  const [subjects] = createResource(
    () => props.value.disciplineUuid,
    (uuid) => api.client.listSubjects(uuid),
  );
  const [topics] = createResource(
    () => props.value.subjectUuid || false,
    (uuid) => api.client.listTopics(uuid),
  );
  const [subtopics] = createResource(
    () => props.value.topicUuid || false,
    (uuid) => api.client.listSubtopics(uuid),
  );
  function vocabularyName(
    items: ReadonlyArray<ContentClassificationItem> | undefined,
    uuid: string,
  ): string {
    const item = items?.find((candidate) => candidate.uuid === uuid);
    // ASVS 1.2.1: names stay escaped text. A missing name is not the UUID.
    if (item === undefined) return "Name unavailable";
    return item.isRetired ? `${item.name} (retired)` : item.name;
  }
  return (
    <p class="course-classification-summary">
      Discipline:{" "}
      {vocabularyName(disciplines.error ? undefined : disciplines(), props.value.disciplineUuid)}
      <Show when={props.value.subjectUuid}>
        {(uuid) => (
          <>; Subject: {vocabularyName(subjects.error ? undefined : subjects(), uuid())}</>
        )}
      </Show>
      <Show when={props.value.topicUuid}>
        {(uuid) => <>; Topic: {vocabularyName(topics.error ? undefined : topics(), uuid())}</>}
      </Show>
      <Show when={props.value.subtopicUuid}>
        {(uuid) => (
          <>; Subtopic: {vocabularyName(subtopics.error ? undefined : subtopics(), uuid())}</>
        )}
      </Show>
      ; Tags: {props.value.tags.length === 0 ? "none" : props.value.tags.join(", ")}
    </p>
  );
}
