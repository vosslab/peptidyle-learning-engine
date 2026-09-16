// Small metadata-only editor; stale validators require an explicit current-state reload.
import { createSignal, Show, type JSX } from "solid-js";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import { ApiRequestError } from "../api/http_client/error";
import { decodeCourseClassification } from "../api/decoders/course_classification";
import {
  CourseClassificationFields,
  type CourseClassificationDraft,
} from "./course_classification_fields";
import { CourseClassificationSummary } from "./course_classification_summary";
import { UnsavedChangesGuard } from "./unsaved_changes_guard";

export function CourseClassificationEditor(props: {
  readonly value: CourseClassification;
  readonly metadataEtag: string;
  readonly canEdit: boolean;
  readonly save: (value: CourseClassification, metadataEtag: string) => Promise<void>;
  readonly reload: () => Promise<{
    readonly classification: CourseClassification;
    readonly metadataEtag: string;
  }>;
}): JSX.Element {
  const [editing, setEditing] = createSignal(false);
  const [draft, setDraft] = createSignal<CourseClassificationDraft>(props.value);
  const [etag, setEtag] = createSignal(props.metadataEtag);
  const [busy, setBusy] = createSignal(false);
  const [stale, setStale] = createSignal(false);
  const [message, setMessage] = createSignal("");
  let editButton: HTMLButtonElement | undefined;
  let editForm: HTMLFormElement | undefined;
  async function save(): Promise<boolean> {
    if (busy() || stale() || !props.canEdit) return false;
    let classification: CourseClassification;
    try {
      classification = decodeCourseClassification(draft(), "classification");
    } catch {
      setMessage(
        "Choose a Discipline. Check the hierarchy and use unique Tags of 1 through 120 characters without leading or trailing spaces.",
      );
      return false;
    }
    setBusy(true);
    try {
      await props.save(classification, etag());
      setEditing(false);
      setMessage("Course classification saved.");
      queueMicrotask(() => editButton?.focus());
      return true;
    } catch (error: unknown) {
      // ASVS 16.5.1/16.5.3: retain input and show safe actionable failure copy.
      const conflict = error instanceof ApiRequestError && [409, 412, 428].includes(error.status);
      setStale(conflict);
      setMessage(
        conflict
          ? "Course metadata changed elsewhere. Load current classification and review it before retrying. Your selections remain here."
          : "Course classification was not saved. Your selections remain here; check them and try again.",
      );
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function reload(): Promise<void> {
    setBusy(true);
    try {
      const current = await props.reload();
      setEtag(current.metadataEtag);
      setStale(false);
      setMessage(
        "Current classification loaded below. Your selections remain in the fields; review both before saving again.",
      );
    } catch {
      setMessage("Current classification could not load. Your selections remain here. Try again.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <section class="course-classification-editor" aria-label="Course classification">
      <CourseClassificationSummary value={props.value} />
      <Show when={props.canEdit && !editing()}>
        <button
          ref={(element) => (editButton = element)}
          type="button"
          class="quiet-action"
          onClick={() => {
            setDraft({ ...props.value, tags: [...props.value.tags] });
            setEtag(props.metadataEtag);
            setMessage("");
            setStale(false);
            setEditing(true);
            queueMicrotask(() => editForm?.focus());
          }}
        >
          Edit Course classification
        </button>
      </Show>
      <Show when={editing()}>
        <form
          ref={(element) => (editForm = element)}
          tabindex="-1"
          aria-label="Edit Course classification"
          aria-busy={busy()}
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <CourseClassificationFields
            value={draft()}
            disabled={busy() || !props.canEdit}
            onChange={setDraft}
          />
          <button type="submit" disabled={busy() || stale() || !props.canEdit}>
            {busy() ? "Saving..." : "Save Course classification"}
          </button>
          <button
            type="button"
            disabled={busy()}
            onClick={() => {
              setEditing(false);
              setMessage("");
              queueMicrotask(() => editButton?.focus());
            }}
          >
            Cancel classification edits
          </button>
          <Show when={stale()}>
            <button type="button" disabled={busy()} onClick={() => void reload()}>
              Load current classification
            </button>
          </Show>
        </form>
      </Show>
      <Show when={message()}>
        <p role="status" aria-live="polite">
          {message()}
        </p>
      </Show>
      <UnsavedChangesGuard
        dirty={() => editing() && JSON.stringify(draft()) !== JSON.stringify(props.value)}
        save={save}
        copy={{
          heading: "Keep Course classification edits?",
          description: "Your classification selections have not been saved.",
          saveActionLabel: "Save classification",
          savingActionLabel: "Saving classification...",
          saveFailureMessage:
            "Classification was not saved. Keep editing and resolve any stale metadata before leaving.",
        }}
      />
    </section>
  );
}
