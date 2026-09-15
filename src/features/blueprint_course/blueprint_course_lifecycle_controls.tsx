// Visible Blueprint lifecycle controls derived from the server-owned state machine.

import { A } from "@solidjs/router";
import { Show, type JSX } from "solid-js";

import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import { blueprintLifecyclePresentation } from "./blueprint_course_model";

export interface BlueprintCourseLifecycleControlsProps {
  readonly view: BlueprintCourseView;
  readonly editing: boolean;
  readonly metadataSaving: boolean;
  readonly hasUnsavedContent: boolean;
  readonly archiveConfirmation: string;
  readonly onArchiveConfirmationInput: (value: string) => void;
  readonly onToggleEditor: () => void;
  readonly onPublish: () => void;
  readonly onReturnToPrivate: () => void;
  readonly onArchive: () => void;
  readonly onRestore: () => void;
}

/** Presents only the typed lifecycle commands that the server can authorize. */
export function BlueprintCourseLifecycleControls(
  props: BlueprintCourseLifecycleControlsProps,
): JSX.Element {
  const lifecycle = () =>
    blueprintLifecyclePresentation(props.view.availability, props.view.read_access);
  return (
    <>
      <p class="blueprint-course-field-help">{lifecycle().meaning}</p>
      <nav class="blueprint-course-detail-actions" aria-label="Blueprint Course actions">
        <Show when={lifecycle().canAdopt}>
          <A
            class="primary-link"
            href={`/?blueprint=${encodeURIComponent(props.view.reference)}#create-course-instance`}
          >
            Create Course Instance from this Blueprint
          </A>
        </Show>
        <Show when={lifecycle().canEdit}>
          <button type="button" class="quiet-action" onClick={props.onToggleEditor}>
            {props.editing ? "Return to Blueprint overview" : "Open Course Editor"}
          </button>
        </Show>
      </nav>
      <Show
        when={
          lifecycle().canPublish ||
          lifecycle().canArchive ||
          lifecycle().canRestore ||
          lifecycle().canReturnToPrivate
        }
      >
        <aside class="blueprint-course-inspection">
          <Show when={lifecycle().canPublish}>
            <h2>Publish Blueprint Course</h2>
            <p>Publish lets active Instructors browse and adopt the current Revision.</p>
            <button
              type="button"
              disabled={props.metadataSaving || props.hasUnsavedContent}
              onClick={props.onPublish}
            >
              {props.metadataSaving ? "Publishing..." : "Publish Blueprint Course"}
            </button>
          </Show>
          <Show when={lifecycle().canReturnToPrivate}>
            <h2>Return Blueprint Course to Private</h2>
            <p>
              This is available only while the server confirms that no Course Instance has adopted
              this Blueprint Course.
            </p>
            <button
              type="button"
              class="quiet-action"
              disabled={props.metadataSaving}
              onClick={props.onReturnToPrivate}
            >
              {props.metadataSaving ? "Returning..." : "Return to Private"}
            </button>
          </Show>
          <Show when={lifecycle().canArchive}>
            <h2>Archive Blueprint Course</h2>
            <p>
              Archive removes this Blueprint Course from new selection. Saved Revisions remain
              intact.
            </p>
            <label>
              Confirm Blueprint Course long name
              <input
                value={props.archiveConfirmation}
                maxlength="200"
                disabled={props.metadataSaving}
                onInput={(event) => props.onArchiveConfirmationInput(event.currentTarget.value)}
              />
            </label>
            <button type="button" disabled={props.metadataSaving} onClick={props.onArchive}>
              {props.metadataSaving ? "Archiving..." : "Archive Blueprint Course"}
            </button>
          </Show>
          <Show when={lifecycle().canRestore}>
            <h2>Restore Blueprint Course</h2>
            <p>Restore makes this Blueprint Course Public for new Instructor adoption.</p>
            <button type="button" disabled={props.metadataSaving} onClick={props.onRestore}>
              {props.metadataSaving ? "Restoring..." : "Restore Blueprint Course"}
            </button>
          </Show>
        </aside>
      </Show>
    </>
  );
}
