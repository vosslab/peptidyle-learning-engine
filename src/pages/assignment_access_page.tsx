// assignment_access_page.tsx - course-scoped M2/M3/M4 access modifier workspace.

import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { TeachingOperationRevision } from "../../generated/api/TeachingOperationRevision";
import type { TeachingPreviewView } from "../../generated/api/TeachingPreviewView";
import type { ApiClient } from "../api/client";
import { ApiRequestError } from "../api/http_client/error";
import type { AssignmentRouteReference, CourseRouteReference } from "../navigation/public_route";
import { ModifierDialog } from "./assignment_access/modifier_dialog";
import {
  type ModifierMode,
  type ModifierPatchDraft,
  type SelectedStudent,
} from "./assignment_access/model";
import { PolicyPreview } from "./assignment_access/policy_preview";
import "./assignment_access/assignment_access.css";

export interface AssignmentAccessPageProps {
  readonly client: ApiClient;
  readonly courseId: CourseId;
  readonly assignmentId: AssignmentId;
  readonly initialRevision: TeachingOperationRevision;
  readonly courseReference: CourseRouteReference;
  readonly assignmentReference: AssignmentRouteReference;
  /** Fetches the current strong revision after a compare-and-swap conflict. */
  readonly reloadAssignmentRevision: () => Promise<TeachingOperationRevision>;
  /** Route owners may supply the authorized course-members list when their projection supports it. */
  readonly loadPreviewSubjects?: () => Promise<ReadonlyArray<SelectedStudent>>;
}

type PageState = "loading" | "ready" | "error" | "permission" | "offline";
type SubjectsState = "loading" | "ready" | "error";

function failureState(error: unknown): PageState {
  if (error instanceof ApiRequestError && (error.status === 401 || error.status === 403))
    return "permission";
  if (error instanceof TypeError || !navigator.onLine) return "offline";
  return "error";
}

function namedDeleteCopy(name: string): string {
  return `Remove the student accommodation for ${name}?`;
}

/**
 * This page takes a narrow Student Membership preview-subject loader and never turns internal
 * identifiers into student labels.
 */
export function AssignmentAccessPage(props: AssignmentAccessPageProps): JSX.Element {
  const [state, setState] = createSignal<PageState>("loading");
  const [subjects, setSubjects] = createSignal<ReadonlyArray<SelectedStudent>>([]);
  const [subjectsState, setSubjectsState] = createSignal<SubjectsState>("loading");
  const [revision, setRevision] = createSignal(props.initialRevision);
  const [dialogOpen, setDialogOpen] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [previewSubject, setPreviewSubject] = createSignal<SelectedStudent>();
  const [preview, setPreview] = createSignal<TeachingPreviewView>();
  const [previewLoading, setPreviewLoading] = createSignal(false);
  const [previewFailure, setPreviewFailure] = createSignal("");
  const [revisionConflict, setRevisionConflict] = createSignal(false);
  const canPreview = createMemo(() => subjects().length > 0);
  let reloadStatus: HTMLParagraphElement | undefined;
  let dialogTrigger: HTMLButtonElement | undefined;

  function load(): void {
    setState("loading");
    setMessage("");
    try {
      setState("ready");
      void loadSubjects();
    } catch (error: unknown) {
      setState(failureState(error));
    }
  }

  async function loadSubjects(): Promise<void> {
    if (props.loadPreviewSubjects === undefined) {
      setSubjects([]);
      setSubjectsState("ready");
      return;
    }
    setSubjectsState("loading");
    try {
      setSubjects(await props.loadPreviewSubjects());
      setSubjectsState("ready");
    } catch {
      setSubjectsState("error");
    }
  }

  async function requestPreview(subject: SelectedStudent | undefined): Promise<void> {
    setPreviewSubject(subject);
    setPreview(undefined);
    setPreviewFailure("");
    if (subject === undefined) return;
    setPreviewLoading(true);
    try {
      setPreview(
        await props.client.getTeachingPreview(
          props.courseId,
          props.assignmentId,
          subject.reference,
        ),
      );
    } catch (error: unknown) {
      setPreviewFailure(
        failureState(error) === "offline"
          ? "Preview is unavailable while offline. Your draft is retained."
          : "The preview could not be resolved.",
      );
    } finally {
      setPreviewLoading(false);
    }
  }

  async function mutate(
    run: () => Promise<{ readonly revision: TeachingOperationRevision }>,
  ): Promise<boolean> {
    setBusy(true);
    setMessage("");
    try {
      const accepted = await run();
      setRevision(accepted.revision);
      setRevisionConflict(false);
      setDialogOpen(false);
      setMessage("Modifier saved. The assignment revision was updated.");
      await requestPreview(previewSubject());
      return true;
    } catch (error: unknown) {
      let safeFailure = "The modifier could not be saved. Your draft remains open.";
      if (error instanceof ApiRequestError && error.status === 412) {
        setRevisionConflict(true);
        safeFailure =
          "This assignment changed elsewhere. Your draft remains open; review it before saving again.";
      } else if (failureState(error) === "offline") {
        safeFailure = "You appear to be offline. Your draft remains open.";
      } else if (failureState(error) === "permission") {
        setState("permission");
        safeFailure =
          "You are no longer entitled to change assignment access. Your draft remains open.";
      }
      setMessage(safeFailure);
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function reloadLatestAssignmentRevision(): Promise<void> {
    setBusy(true);
    setMessage("");
    try {
      const nextRevision = await props.reloadAssignmentRevision();
      setRevision(nextRevision);
      await requestPreview(previewSubject());
      setRevisionConflict(false);
      setMessage("Latest assignment revision loaded. Your modifier draft is unchanged.");
      queueMicrotask(() => reloadStatus?.focus());
    } catch (error: unknown) {
      const safeFailure =
        failureState(error) === "offline"
          ? "You appear to be offline. Reconnect before reloading the assignment revision."
          : "The latest assignment revision could not load. Your modifier draft is unchanged.";
      setMessage(safeFailure);
      queueMicrotask(() => reloadStatus?.focus());
    } finally {
      setBusy(false);
    }
  }

  async function savePolicy(
    target: string,
    mode: ModifierMode,
    draft: ModifierPatchDraft,
  ): Promise<boolean> {
    const request = (await import("./assignment_access/model")).policyRequest(mode, draft);
    return mutate(() =>
      props.client.putAccommodation(
        props.courseId,
        props.assignmentId,
        target,
        request,
        revision(),
      ),
    );
  }

  async function deleteModifier(target: string, name: string): Promise<boolean> {
    if (!window.confirm(namedDeleteCopy(name))) return false;
    return mutate(() =>
      props.client.deleteAccommodation(props.courseId, props.assignmentId, target, revision()),
    );
  }

  onMount(() => void load());
  return (
    <section
      class="page assignment-access-page"
      data-route-surface="assignmentAccess"
      aria-live="polite"
    >
      <p class="eyebrow">Instructor assignment settings</p>
      <h1>Access and modifiers</h1>
      <A
        class="quiet-link"
        href={`/instructor/courses/${props.courseReference}/assignments/${props.assignmentReference}/policies`}
      >
        Return to assignment policies
      </A>
      <Switch>
        <Match when={state() === "loading"}>
          <p role="status">Loading assignment access settings...</p>
        </Match>
        <Match when={state() === "permission"}>
          <p role="alert">You are not entitled to manage assignment access for this course.</p>
        </Match>
        <Match when={state() === "offline"}>
          <p role="alert">
            Assignment access is unavailable while offline. Reconnect and try again.
          </p>
          <button type="button" onClick={() => void load()}>
            Try again
          </button>
        </Match>
        <Match when={state() === "error"}>
          <p role="alert">Assignment access settings could not load.</p>
          <button type="button" onClick={() => void load()}>
            Try again
          </button>
        </Match>
        <Match when={state() === "ready"}>
          <div class="assignment-access-grid">
            <section
              class="assignment-access-panel"
              aria-labelledby="assignment-access-modifiers-heading"
            >
              <h2 id="assignment-access-modifiers-heading">Modifiers</h2>
              <p>Choose a Student Membership, then define its explicit accommodation.</p>
              <Show when={!canPreview()}>
                <p class="assignment-access-help">
                  {subjectsState() === "loading"
                    ? "Loading authorized Student choices..."
                    : "An authorized course-members loader is needed to select Students by safe display name. This route does not expose raw membership references."}
                </p>
                <Show when={subjectsState() === "error"}>
                  <button type="button" onClick={() => void loadSubjects()}>
                    Retry student choices
                  </button>
                </Show>
              </Show>
              <div class="assignment-access-actions">
                <button
                  type="button"
                  disabled={busy() || !canPreview()}
                  onClick={(event) => {
                    dialogTrigger = event.currentTarget;
                    setDialogOpen(true);
                  }}
                >
                  Add or change student accommodation
                </button>
              </div>
              <Show when={revisionConflict()}>
                <p class="assignment-access-error" role="status">
                  This assignment changed elsewhere. Reload its latest revision before retrying;
                  your modifier draft remains open.
                </p>
                <button
                  type="button"
                  disabled={busy()}
                  onClick={() => void reloadLatestAssignmentRevision()}
                >
                  Reload latest assignment revision
                </button>
              </Show>
              <Show when={message()}>
                <p
                  ref={(element) => (reloadStatus = element)}
                  class={
                    message().startsWith("Modifier saved")
                      ? "assignment-access-success"
                      : "assignment-access-error"
                  }
                  role="status"
                  tabindex="-1"
                >
                  {message()}
                </p>
              </Show>
            </section>
            <section
              class="assignment-access-panel"
              aria-labelledby="assignment-access-subject-heading"
            >
              <h2 id="assignment-access-subject-heading">Preview a Student</h2>
              <Show
                when={canPreview()}
                fallback={
                  <p class="assignment-access-help">
                    {subjectsState() === "error"
                      ? "Student choices could not load. Retry without losing modifier work."
                      : "Preview selection awaits an authorized display-name loader from the route owner."}
                  </p>
                }
              >
                <label class="assignment-access-field">
                  Student
                  <select
                    onChange={(event) =>
                      void requestPreview(
                        subjects().find(
                          (subject) => subject.reference === event.currentTarget.value,
                        ),
                      )
                    }
                  >
                    <option value="">Choose a Student</option>
                    <For each={subjects()}>
                      {(subject) => <option value={subject.reference}>{subject.display}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show when={subjectsState() === "error"}>
                <button type="button" onClick={() => void loadSubjects()}>
                  Retry student choices
                </button>
              </Show>
              <PolicyPreview
                preview={preview()}
                loading={previewLoading()}
                failure={previewFailure()}
              />
            </section>
          </div>
        </Match>
      </Switch>
      <Show when={dialogOpen()}>
        <ModifierDialog
          subjects={subjects()}
          revision={revision()}
          busy={busy()}
          revisionConflict={revisionConflict()}
          onClose={() => {
            setDialogOpen(false);
            queueMicrotask(() => dialogTrigger?.focus());
          }}
          onReloadLatestRevision={() => void reloadLatestAssignmentRevision()}
          onSavePolicy={savePolicy}
          onDelete={deleteModifier}
        />
      </Show>
    </section>
  );
}

export { namedDeleteCopy };
