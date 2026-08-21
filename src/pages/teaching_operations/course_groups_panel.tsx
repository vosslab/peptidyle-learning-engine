import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseGroupMemberView } from "../../../generated/api/CourseGroupMemberView";
import type { CourseGroupMembershipWarningView } from "../../../generated/api/CourseGroupMembershipWarningView";
import type { CourseGroupPurpose } from "../../../generated/api/CourseGroupPurpose";
import type { CourseGroupSummaryView } from "../../../generated/api/CourseGroupSummaryView";
import type { CourseReference } from "../../../generated/api/CourseReference";
import type { TeachingOperationRevision } from "../../../generated/api/TeachingOperationRevision";
import type { ApiRuntime } from "../../api/runtime";
import { ApiRequestError } from "../../api/http_client";
import {
  COURSE_GROUP_PURPOSES,
  appendGroupPage,
  groupConflictCopy,
  membershipWarningCopy,
  policyCopy,
  purposeLabel,
  referencedGroupCopy,
} from "./course_groups_panel_model";
import "./teaching_operations_panels.css";

type PanelState = "loading" | "ready" | "error";
type GroupDraft = {
  readonly title: string;
  readonly purpose: CourseGroupPurpose;
  readonly members: ReadonlyArray<string>;
};
export interface CourseMemberOption {
  readonly reference: string;
  readonly display: string;
}

export interface CourseGroupsPanelProps {
  readonly courseId: CourseId;
  readonly courseReference: CourseReference;
  readonly runtime: Pick<ApiRuntime, "client">;
  /** Parent supplies only authorized, browser-safe member labels and opaque request values. */
  readonly memberOptions: ReadonlyArray<CourseMemberOption>;
}

function blankDraft(): GroupDraft {
  return { title: "", purpose: "section", members: [] };
}

function memberLabels(members: ReadonlyArray<CourseGroupMemberView>): string {
  return members.map((member) => member.display).join(", ");
}

function requestStatus(error: unknown): number | undefined {
  return error instanceof ApiRequestError ? error.status : undefined;
}

function selectedReferences(event: Event): ReadonlyArray<string> {
  const control = event.currentTarget;
  if (!(control instanceof HTMLSelectElement)) return [];
  return Array.from(control.selectedOptions, (option) => option.value);
}

function courseGroupPurpose(value: string): CourseGroupPurpose {
  switch (value) {
    case "section":
    case "lab":
    case "cohort":
    case "accommodation":
    case "work":
      return value;
    default:
      return "section";
  }
}

function membershipPolicy(value: string): "allow" | "warn" {
  return value === "warn" ? "warn" : "allow";
}

/** Course-local group editing uses visible labels while keeping reference values only in requests. */
export function CourseGroupsPanel(props: CourseGroupsPanelProps): JSX.Element {
  const [state, setState] = createSignal<PanelState>("loading");
  const [groups, setGroups] = createSignal<ReadonlyArray<CourseGroupSummaryView>>([]);
  const [nextCursor, setNextCursor] = createSignal<string | null>(null);
  const [draft, setDraft] = createSignal<GroupDraft>(blankDraft());
  const [editing, setEditing] = createSignal<CourseGroupSummaryView>();
  const [members, setMembers] = createSignal<ReadonlyArray<CourseGroupMemberView>>([]);
  const [message, setMessage] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [policyPurpose, setPolicyPurpose] = createSignal<CourseGroupPurpose>("section");
  const [policy, setPolicy] = createSignal<"allow" | "warn">("warn");
  const [policyRevision, setPolicyRevision] = createSignal<TeachingOperationRevision>();
  const [warning, setWarning] = createSignal<CourseGroupMembershipWarningView>();
  const [pendingDelete, setPendingDelete] = createSignal<CourseGroupSummaryView>();
  const [deleteName, setDeleteName] = createSignal("");
  const [deleteMessage, setDeleteMessage] = createSignal("");
  let statusNode: HTMLParagraphElement | undefined;
  let deleteDialog: HTMLDialogElement | undefined;
  let deleteCancel: HTMLButtonElement | undefined;
  let deleteError: HTMLParagraphElement | undefined;
  let deleteOpener: HTMLButtonElement | undefined;
  let restoreDeleteFocusOnClose = true;

  function focusStatus(): void {
    queueMicrotask(() => statusNode?.focus());
  }
  function currentWarningCopy(): string | undefined {
    const currentWarning = warning();
    if (currentWarning === undefined) return undefined;
    return membershipWarningCopy(currentWarning);
  }
  function resetEditor(): void {
    setEditing(undefined);
    setMembers([]);
    setDraft(blankDraft());
  }
  function restoreDeleteFocus(): void {
    queueMicrotask(() => {
      if (deleteOpener?.isConnected) deleteOpener.focus();
      else statusNode?.focus();
    });
  }
  function clearDeleteDialog(restoreFocus: boolean): void {
    setPendingDelete(undefined);
    setDeleteName("");
    setDeleteMessage("");
    if (restoreFocus) restoreDeleteFocus();
  }
  function closeDeleteDialog(restoreFocus = true): void {
    if (deleteDialog?.open) {
      restoreDeleteFocusOnClose = restoreFocus;
      deleteDialog.close();
    } else clearDeleteDialog(restoreFocus);
  }
  function openDeleteDialog(group: CourseGroupSummaryView, opener: HTMLButtonElement): void {
    deleteOpener = opener;
    setDeleteName("");
    setDeleteMessage("");
    setPendingDelete(group);
    queueMicrotask(() => {
      if (deleteDialog !== undefined && !deleteDialog.open) deleteDialog.showModal();
      deleteCancel?.focus();
    });
  }
  function focusDeleteError(): void {
    queueMicrotask(() => (deleteError ?? deleteCancel)?.focus());
  }
  async function load(): Promise<void> {
    setState("loading");
    try {
      const page = await props.runtime.client.listCourseGroups(props.courseId, undefined, 100);
      setGroups(page.groups);
      setNextCursor(page.nextCursor);
      setState("ready");
      setMessage(page.groups.length === 0 ? "No course groups yet." : "Course groups loaded.");
    } catch (error: unknown) {
      setState("error");
      setMessage(
        requestStatus(error) === 403
          ? "You do not have permission to view course groups."
          : "Course groups could not load. Try again.",
      );
    }
  }
  async function loadMore(): Promise<void> {
    const cursor = nextCursor();
    if (cursor === null) return;
    setBusy(true);
    try {
      const page = await props.runtime.client.listCourseGroups(props.courseId, cursor, 100);
      setGroups((current) => appendGroupPage(current, page.groups));
      setNextCursor(page.nextCursor);
      setMessage("More course groups loaded.");
    } catch {
      setMessage("More course groups could not load. Try Load more groups again.");
      focusStatus();
    } finally {
      setBusy(false);
    }
  }
  async function openGroup(group: CourseGroupSummaryView): Promise<void> {
    setBusy(true);
    try {
      const detail = await props.runtime.client.getCourseGroup(
        props.courseId,
        group.reference,
        undefined,
        100,
      );
      setEditing(detail.group);
      setMembers(detail.members);
      setDraft({
        title: detail.group.title,
        purpose: detail.group.purpose,
        members: detail.members.map((member) => member.reference),
      });
      setMessage(
        `Editing ${detail.group.title}. Members: ${memberLabels(detail.members) || "none"}.`,
      );
      focusStatus();
    } catch (error: unknown) {
      setMessage(
        requestStatus(error) === 403
          ? "You do not have permission to view this group."
          : "This group could not load. Try again.",
      );
      focusStatus();
    } finally {
      setBusy(false);
    }
  }
  async function save(): Promise<void> {
    const title = draft().title.trim();
    if (title.length === 0) {
      setMessage("Enter a group name before saving.");
      focusStatus();
      return;
    }
    const request = { title, purpose: draft().purpose, members: [...draft().members] };
    setBusy(true);
    try {
      const current = editing();
      if (current === undefined) {
        await props.runtime.client.createCourseGroup(props.courseId, request);
        setMessage("Course group created.");
      } else {
        await props.runtime.client.updateCourseGroup(
          props.courseId,
          current.reference,
          request,
          current.revision,
        );
        setMessage("Course group saved.");
      }
      resetEditor();
      await load();
      await loadWarnings();
    } catch (error: unknown) {
      setMessage(
        requestStatus(error) === 412
          ? groupConflictCopy()
          : "The course group could not be saved. Your draft is preserved.",
      );
      focusStatus();
    } finally {
      setBusy(false);
    }
  }
  async function loadPolicy(): Promise<void> {
    setBusy(true);
    try {
      const view = await props.runtime.client.getCourseGroupPurposePolicy(
        props.courseId,
        policyPurpose(),
      );
      setPolicy(view.multipleMembership);
      setPolicyRevision(view.revision);
      setMessage(`${purposeLabel(view.purpose)} membership policy loaded.`);
    } catch {
      setMessage("The membership policy could not load.");
    } finally {
      setBusy(false);
    }
  }
  async function loadWarnings(): Promise<void> {
    try {
      const response = await props.runtime.client.getCourseGroupMembershipWarnings(props.courseId);
      setWarning(response);
    } catch {
      setWarning(undefined);
    }
  }
  function selectPolicyPurpose(value: string): void {
    setPolicyPurpose(courseGroupPurpose(value));
    setPolicyRevision(undefined);
    void loadPolicy();
    void loadWarnings();
  }
  async function savePolicy(): Promise<void> {
    const revision = policyRevision();
    if (revision === undefined) {
      await loadPolicy();
      return;
    }
    setBusy(true);
    try {
      const saved = await props.runtime.client.updateCourseGroupPurposePolicy(
        props.courseId,
        policyPurpose(),
        { multipleMembership: policy() },
        revision,
      );
      setPolicyRevision(saved.revision);
      setMessage("Membership policy saved. Warnings never block a valid group write.");
      await loadWarnings();
    } catch (error: unknown) {
      setMessage(
        requestStatus(error) === 412
          ? "The membership policy changed elsewhere. Reload it before retrying."
          : "The membership policy could not be saved.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function deleteGroup(): Promise<void> {
    const group = pendingDelete();
    if (group === undefined) return;
    setBusy(true);
    try {
      await props.runtime.client.deleteCourseGroup(props.courseId, group.reference, group.revision);
      closeDeleteDialog(false);
      resetEditor();
      setMessage("Course group deleted.");
      await load();
      await loadWarnings();
      focusStatus();
    } catch (error: unknown) {
      const failure =
        requestStatus(error) === 409
          ? referencedGroupCopy()
          : requestStatus(error) === 412
            ? groupConflictCopy()
            : "The course group could not be deleted.";
      setMessage(failure);
      setDeleteMessage(failure);
      focusDeleteError();
    } finally {
      setBusy(false);
    }
  }
  onMount(() => {
    void load();
    void loadPolicy();
    void loadWarnings();
  });

  return (
    <section class="teaching-operations-panel" aria-labelledby="course-groups-heading">
      <h2 id="course-groups-heading">Groups and sections</h2>
      <p class="teaching-operations-context">
        Manage this course's sections, labs, cohorts, and other groups.
      </p>
      <p
        class="teaching-operations-status"
        role="status"
        aria-live="polite"
        tabindex="-1"
        ref={(element) => {
          statusNode = element;
        }}
      >
        {message()}
      </p>
      <Show when={state() === "loading"}>
        <p>Loading course groups...</p>
      </Show>
      <Show when={state() === "error"}>
        <button type="button" onClick={() => void load()}>
          Retry loading groups
        </button>
      </Show>
      <Show when={state() === "ready"}>
        <div class="teaching-operations-columns">
          <div>
            <h3>Course groups</h3>
            <Show
              when={groups().length > 0}
              fallback={<p>No course groups yet. Create one below.</p>}
            >
              <ul class="teaching-operations-list">
                <For each={groups()}>
                  {(group) => (
                    <li>
                      <button type="button" disabled={busy()} onClick={() => void openGroup(group)}>
                        {group.title}
                      </button>
                      <span>
                        {purposeLabel(group.purpose)}; {group.memberCount} members
                      </span>
                      <button
                        type="button"
                        class="teaching-operations-danger-link"
                        disabled={busy()}
                        onClick={(event) => openDeleteDialog(group, event.currentTarget)}
                      >
                        Delete {group.title}
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
            <Show when={nextCursor() !== null}>
              <button type="button" disabled={busy()} onClick={() => void loadMore()}>
                Load more groups
              </button>
            </Show>
          </div>
          <form
            class="teaching-operations-form"
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <h3>{editing() === undefined ? "Create group" : "Edit group"}</h3>
            <label>
              Group name
              <input
                value={draft().title}
                disabled={busy()}
                onInput={(event) => setDraft({ ...draft(), title: event.currentTarget.value })}
              />
            </label>
            <label>
              Purpose
              <select
                value={draft().purpose}
                disabled={busy()}
                onChange={(event) =>
                  setDraft({ ...draft(), purpose: courseGroupPurpose(event.currentTarget.value) })
                }
              >
                <For each={COURSE_GROUP_PURPOSES}>
                  {(purpose) => <option value={purpose}>{purposeLabel(purpose)}</option>}
                </For>
              </select>
            </label>
            <label>
              Members
              <select
                multiple
                value={[...draft().members]}
                disabled={busy()}
                aria-describedby="group-members-help"
                onChange={(event) => setDraft({ ...draft(), members: selectedReferences(event) })}
              >
                <For each={props.memberOptions}>
                  {(member) => <option value={member.reference}>{member.display}</option>}
                </For>
              </select>
            </label>
            <p id="group-members-help">
              Choose from the authorized course roster. Current member labels:{" "}
              {memberLabels(members()) || "none"}.
            </p>
            <div class="teaching-operations-actions">
              <button type="submit" disabled={busy()}>
                {editing() === undefined ? "Create group" : "Save group"}
              </button>
              <button type="button" disabled={busy()} onClick={resetEditor}>
                Clear editor
              </button>
            </div>
          </form>
        </div>
        <fieldset class="teaching-operations-policy">
          <legend>Multiple membership policy</legend>
          <label>
            Group purpose
            <select
              value={policyPurpose()}
              disabled={busy()}
              onChange={(event) => selectPolicyPurpose(event.currentTarget.value)}
            >
              <For each={COURSE_GROUP_PURPOSES}>
                {(purpose) => <option value={purpose}>{purposeLabel(purpose)}</option>}
              </For>
            </select>
          </label>
          <label>
            Policy
            <select
              value={policy()}
              disabled={busy()}
              onChange={(event) => setPolicy(membershipPolicy(event.currentTarget.value))}
            >
              <option value="allow">Allow</option>
              <option value="warn">Warn</option>
            </select>
          </label>
          <p>{policyCopy(policy())} A warning never blocks a valid write.</p>
          <Show
            when={currentWarningCopy()}
            fallback={<p>Membership warning check is not available.</p>}
          >
            {(copy) => <p>{copy()}</p>}
          </Show>
          <div class="teaching-operations-actions">
            <button type="button" disabled={busy()} onClick={() => void loadPolicy()}>
              Reload policy
            </button>
            <button type="button" disabled={busy()} onClick={() => void savePolicy()}>
              Save policy
            </button>
          </div>
        </fieldset>
      </Show>
      <Show when={pendingDelete()} keyed>
        {(group) => (
          <dialog
            class="teaching-operations-confirm"
            aria-labelledby="delete-group-heading"
            ref={(element) => {
              deleteDialog = element;
            }}
            onCancel={(event) => {
              event.preventDefault();
              closeDeleteDialog();
            }}
            onClose={() => {
              clearDeleteDialog(restoreDeleteFocusOnClose);
              restoreDeleteFocusOnClose = true;
            }}
          >
            <h3 id="delete-group-heading">Delete {group.title}?</h3>
            <p>This permanently removes the group. Type its name to confirm.</p>
            <p
              class="teaching-operations-dialog-status"
              role="alert"
              tabindex="-1"
              ref={(element) => {
                deleteError = element;
              }}
            >
              {deleteMessage()}
            </p>
            <label>
              Group name
              <input
                value={deleteName()}
                onInput={(event) => setDeleteName(event.currentTarget.value)}
              />
            </label>
            <div class="teaching-operations-actions">
              <button
                type="button"
                disabled={busy() || deleteName() !== group.title}
                onClick={() => void deleteGroup()}
              >
                Confirm delete
              </button>
              <button
                type="button"
                disabled={busy()}
                ref={(element) => {
                  deleteCancel = element;
                }}
                onClick={() => {
                  closeDeleteDialog();
                }}
              >
                Cancel
              </button>
            </div>
          </dialog>
        )}
      </Show>
    </section>
  );
}
