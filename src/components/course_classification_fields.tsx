// Parent-filtered Course classification, with no inferred or preselected Discipline.
import { batch, createEffect, createSignal, type JSX } from "solid-js";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import { useApplicationApi } from "../api/application_api";
import { ContentClassificationSelect } from "./content_classification_select";
import "./course_classification.css";

export type CourseClassificationDraft = Omit<CourseClassification, "disciplineUuid"> & {
  readonly disciplineUuid: string | null;
};

export function emptyCourseClassification(): CourseClassificationDraft {
  return { disciplineUuid: null, subjectUuid: null, topicUuid: null, subtopicUuid: null, tags: [] };
}

export function CourseClassificationFields(props: {
  readonly value: CourseClassificationDraft;
  readonly disabled?: boolean;
  readonly onChange: (value: CourseClassificationDraft) => void;
}): JSX.Element {
  const api = useApplicationApi();
  const [tagsText, setTagsText] = createSignal(props.value.tags.join("\n"));
  function tagsFromText(text: string): string[] {
    return text.split("\n").filter((tag) => tag !== "");
  }
  createEffect(() => {
    // Preserve typed blank lines while reflecting an explicit parent-owned reset.
    if (JSON.stringify(props.value.tags) !== JSON.stringify(tagsFromText(tagsText()))) {
      setTagsText(props.value.tags.join("\n"));
    }
  });
  return (
    <fieldset class="course-classification-fields" disabled={props.disabled}>
      <legend>Course classification</legend>
      <ContentClassificationSelect
        label="Discipline"
        required
        value={props.value.disciplineUuid}
        disabled={props.disabled}
        load={() => api.client.listDisciplines()}
        onChange={(disciplineUuid) =>
          props.onChange({
            ...props.value,
            disciplineUuid,
            subjectUuid: null,
            topicUuid: null,
            subtopicUuid: null,
          })
        }
      />
      <ContentClassificationSelect
        label="Subject"
        value={props.value.subjectUuid}
        parentUuid={props.value.disciplineUuid}
        disabled={props.disabled}
        load={(parent) => api.client.listSubjects(parent)}
        onChange={(subjectUuid) =>
          props.onChange({ ...props.value, subjectUuid, topicUuid: null, subtopicUuid: null })
        }
      />
      <ContentClassificationSelect
        label="Topic"
        value={props.value.topicUuid}
        parentUuid={props.value.subjectUuid}
        disabled={props.disabled}
        load={(parent) => api.client.listTopics(parent)}
        onChange={(topicUuid) => props.onChange({ ...props.value, topicUuid, subtopicUuid: null })}
      />
      <ContentClassificationSelect
        label="Subtopic"
        value={props.value.subtopicUuid}
        parentUuid={props.value.topicUuid}
        disabled={props.disabled}
        load={(parent) => api.client.listSubtopics(parent)}
        onChange={(subtopicUuid) => props.onChange({ ...props.value, subtopicUuid })}
      />
      <label class="course-classification-fields__tags">
        Tags (optional; one per line)
        <textarea
          value={tagsText()}
          onInput={(event) => {
            const text = event.currentTarget.value;
            batch(() => {
              setTagsText(text);
              props.onChange({ ...props.value, tags: tagsFromText(text) });
            });
          }}
        />
      </label>
    </fieldset>
  );
}
