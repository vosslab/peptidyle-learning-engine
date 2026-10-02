// Parent-filtered Course classification, with no inferred or preselected Discipline.
import { batch, createEffect, createSignal, Show, type JSX } from "solid-js";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import { useApplicationApi } from "../api/application_api";
import { AuthoringClassificationLevel } from "./authoring_classification_level";
import { ContentClassificationSelect } from "./content_classification_select";
import { DisciplineRequestInputError, submitDisciplineRequest } from "./discipline_request";
import "./course_classification.css";

export { ApplicationApiProvider } from "../api/application_api";

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
        load={() => api.client.listDisciplinesIncludingRetired()}
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
      <DisciplineRequestControl disabled={props.disabled} />
      <AuthoringClassificationLevel
        label="Subject"
        value={props.value.subjectUuid}
        parentUuid={props.value.disciplineUuid}
        disabled={props.disabled}
        load={(parent) => api.client.listSubjects(parent)}
        onChange={(subjectUuid) =>
          props.onChange({ ...props.value, subjectUuid, topicUuid: null, subtopicUuid: null })
        }
        createName={(name, parent) => api.client.createSubject(name, parent)}
        acceptExisting={(uuid, parent) => api.client.acceptSubjectDiscipline(uuid, parent)}
      />
      <AuthoringClassificationLevel
        label="Topic"
        value={props.value.topicUuid}
        parentUuid={props.value.subjectUuid}
        disabled={props.disabled}
        load={(parent) => api.client.listTopics(parent)}
        onChange={(topicUuid) => props.onChange({ ...props.value, topicUuid, subtopicUuid: null })}
        createName={async (name, parent) => {
          const item = await api.client.createTopic(name, parent);
          return { uuid: item.uuid, name: item.name, needsAcceptance: false };
        }}
      />
      <AuthoringClassificationLevel
        label="Subtopic"
        value={props.value.subtopicUuid}
        parentUuid={props.value.topicUuid}
        disabled={props.disabled}
        load={(parent) => api.client.listSubtopics(parent)}
        onChange={(subtopicUuid) => props.onChange({ ...props.value, subtopicUuid })}
        createName={async (name, parent) => {
          const item = await api.client.createSubtopic(name, parent);
          return { uuid: item.uuid, name: item.name, needsAcceptance: false };
        }}
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

/** Asks a Sysadmin for a Discipline the Course selector does not offer. */
export function DisciplineRequestControl(props: { readonly disabled?: boolean }): JSX.Element {
  const api = useApplicationApi();
  const [name, setName] = createSignal("");
  const [pending, setPending] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [status, setStatus] = createSignal("");

  async function request(): Promise<void> {
    if (pending()) return;
    setPending(true);
    setError(null);
    try {
      const message = await submitDisciplineRequest(api.client, name());
      setName("");
      setStatus(message);
    } catch (caught) {
      setStatus("");
      setError(
        caught instanceof DisciplineRequestInputError
          ? caught.message
          : "That Discipline request could not be recorded.",
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <div class="course-classification-fields__request">
      <p>Needed Discipline unavailable? Request it. A Sysadmin creates Disciplines.</p>
      <label for="request-discipline-name">
        Requested Discipline name
        <input
          id="request-discipline-name"
          name="requestedDisciplineName"
          type="text"
          value={name()}
          maxlength={120}
          disabled={props.disabled || pending()}
          onInput={(event) => setName(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key !== "Enter") return;
            event.preventDefault();
            void request();
          }}
        />
      </label>
      <button
        class="quiet-action"
        type="button"
        disabled={props.disabled || pending()}
        onClick={() => void request()}
      >
        {pending() ? "Requesting Discipline..." : "Request a new Discipline"}
      </button>
      <Show when={error()}>
        {(message) => (
          <span class="inline-error" role="alert">
            {message()}
          </span>
        )}
      </Show>
      <Show when={status()}>{(message) => <p role="status">{message()}</p>}</Show>
    </div>
  );
}
