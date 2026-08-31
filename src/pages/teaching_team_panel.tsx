// Course-scoped Instructor Course Invitation and direct-membership management.

import { A } from "@solidjs/router";
import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import type { CourseInvitationTargetView } from "../../generated/api/CourseInvitationTargetView";
import type { InstructorCourseInvitationView } from "../../generated/api/InstructorCourseInvitationView";
import type { InstructorMembershipView } from "../../generated/api/InstructorMembershipView";
import type { TeachingOperationRevision } from "../../generated/api/TeachingOperationRevision";
import { ApiRequestError } from "../api/http_client/error";
import { useApplicationApi } from "../api/application_api";
import {
  appendTeachingTeamPage,
  appendTeachingTeamRows,
  conflictRecoveryCopy,
  finalInstructorConflictCopy,
  invitationStateLabel,
  serverExpiryCopy,
} from "./teaching_team_model";
import "./teaching_team_panel.css";

interface TeachingTeamPanelProps {
  readonly courseId: CourseId;
}

interface TeachingTeamData {
  readonly instructors: ReadonlyArray<InstructorMembershipView>;
  readonly invitations: ReadonlyArray<InstructorCourseInvitationView>;
  readonly rosterRevision: TeachingOperationRevision;
  readonly instructorCursor: string | null;
  readonly invitationCursor: string | null;
}

type PendingAction =
  | {
      readonly kind: "revoke";
      readonly row: InstructorCourseInvitationView;
      readonly trigger: HTMLButtonElement;
    }
  | {
      readonly kind: "remove";
      readonly row: InstructorMembershipView;
      readonly trigger: HTMLButtonElement;
    };

function errorCopy(error: unknown): string {
  if (error instanceof ApiRequestError && error.status === 409)
    return finalInstructorConflictCopy();
  if (error instanceof ApiRequestError && error.status === 412) return conflictRecoveryCopy();
  return "That teaching-team change could not be completed. Check your connection and try again.";
}

export function TeachingTeamPanel(props: TeachingTeamPanelProps): JSX.Element {
  const runtime = useApplicationApi();
  const [data, setData] = createSignal<TeachingTeamData | null>(null);
  const [query, setQuery] = createSignal("");
  const [targets, setTargets] = createSignal<ReadonlyArray<CourseInvitationTargetView>>([]);
  const [targetCursor, setTargetCursor] = createSignal<string | null>(null);
  const [selectedTarget, setSelectedTarget] = createSignal<CourseInvitationTargetView | null>(null);
  const [searching, setSearching] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");
  const [pendingAction, setPendingAction] = createSignal<PendingAction | null>(null);
  let heading: HTMLHeadingElement | undefined;

  const queryEligible = createMemo(() => query().trim().length >= 2);

  async function load(): Promise<void> {
    setError(null);
    try {
      const [instructors, invitations] = await Promise.all([
        runtime.client.listCourseInstructors(props.courseId, undefined, 25),
        runtime.client.listInstructorCourseInvitations(props.courseId, undefined, 25),
      ]);
      setData({
        instructors: instructors.instructors,
        invitations: invitations.invitations,
        rosterRevision: instructors.rosterRevision,
        instructorCursor: instructors.nextCursor,
        invitationCursor: invitations.nextCursor,
      });
    } catch {
      setError("The teaching team could not load. Check your connection and try again.");
    }
  }

  async function search(): Promise<void> {
    if (!queryEligible()) {
      setTargets([]);
      setSelectedTarget(null);
      return;
    }
    setSearching(true);
    setError(null);
    try {
      const result = await runtime.client.searchInstructorCourseInvitationTargets(
        props.courseId,
        query().trim(),
        undefined,
        20,
      );
      setTargets(result.targets);
      setTargetCursor(result.nextCursor);
      setSelectedTarget(null);
      setAnnouncement(`${result.targets.length} eligible people found.`);
    } catch {
      setError("Eligible people could not be searched. Check your connection and try again.");
    } finally {
      setSearching(false);
    }
  }

  async function loadMoreTargets(): Promise<void> {
    const cursor = targetCursor();
    if (cursor === null) return;
    setSearching(true);
    setError(null);
    try {
      const result = await runtime.client.searchInstructorCourseInvitationTargets(
        props.courseId,
        query().trim(),
        cursor,
        20,
      );
      const known = new Set(targets().map((target) => target.account.reference));
      setTargets([
        ...targets(),
        ...result.targets.filter((target) => !known.has(target.account.reference)),
      ]);
      setTargetCursor(result.nextCursor);
      setAnnouncement(`${result.targets.length} more eligible people found.`);
    } catch {
      setError("More eligible people could not be searched. Check your connection and try again.");
    } finally {
      setSearching(false);
    }
  }

  async function invite(): Promise<void> {
    const target = selectedTarget();
    if (target === null) return;
    setBusy(true);
    setError(null);
    try {
      await runtime.client.createInstructorCourseInvitation(props.courseId, {
        target: target.account.reference,
      });
      setAnnouncement(`An invitation was created for ${target.account.display}.`);
      await load();
    } catch (caught) {
      setError(errorCopy(caught));
      if (caught instanceof ApiRequestError && caught.status === 412) await load();
    } finally {
      setBusy(false);
    }
  }

  async function loadMore(kind: "instructors" | "invitations"): Promise<void> {
    const current = data();
    if (current === null) return;
    const cursor = kind === "instructors" ? current.instructorCursor : current.invitationCursor;
    if (cursor === null) return;
    setBusy(true);
    setError(null);
    try {
      if (kind === "instructors") {
        const next = await runtime.client.listCourseInstructors(props.courseId, cursor, 25);
        if (next.rosterRevision !== current.rosterRevision) {
          await load();
          setAnnouncement(conflictRecoveryCopy());
          return;
        }
        setData({
          ...current,
          instructors: appendTeachingTeamRows(
            current.instructors,
            next.instructors,
            (instructor) => instructor.membership,
          ),
          instructorCursor: next.nextCursor,
        });
      } else {
        const next = await runtime.client.listInstructorCourseInvitations(
          props.courseId,
          cursor,
          25,
        );
        setData({
          ...current,
          invitations: appendTeachingTeamPage(current.invitations, next.invitations),
          invitationCursor: next.nextCursor,
        });
      }
    } catch {
      setError("More teaching-team records could not load. Check your connection and try again.");
    } finally {
      setBusy(false);
    }
  }

  function cancelAction(): void {
    const action = pendingAction();
    setPendingAction(null);
    queueMicrotask(() => action?.trigger.focus());
  }

  async function confirmAction(): Promise<void> {
    const action = pendingAction();
    const current = data();
    if (action === null || current === null) return;
    setPendingAction(null);
    setBusy(true);
    setError(null);
    try {
      if (action.kind === "revoke") {
        const invitation = action.row;
        await runtime.client.revokeInstructorCourseInvitation(
          props.courseId,
          invitation.reference,
          invitation.revision,
        );
        setAnnouncement("The pending Instructor Course Invitation was canceled.");
      } else {
        const instructor = action.row;
        await runtime.client.removeCourseInstructor(
          props.courseId,
          instructor.membership,
          {},
          current.rosterRevision,
        );
        setAnnouncement(`${instructor.account.display} no longer has course instructor access.`);
      }
      await load();
      queueMicrotask(() => heading?.focus());
    } catch (caught) {
      setError(errorCopy(caught));
      if (caught instanceof ApiRequestError && caught.status === 412) await load();
    } finally {
      setBusy(false);
    }
  }

  onMount(() => void load());

  return (
    <section class="teaching-team-panel" aria-labelledby="teaching-team-heading">
      <h2 ref={(element) => (heading = element)} id="teaching-team-heading" tabindex={-1}>
        Teaching team
      </h2>
      <p class="page-lede">
        Approved eligibility allows an invitation; it does not itself grant course authority.
      </p>
      <A class="quiet-link" href="/account/course-invitations">
        Review invitations addressed to your account
      </A>
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
      <Show when={error()}>
        {(message) => (
          <section class="inline-error" role="alert">
            <p>{message()}</p>
            <button class="quiet-action" type="button" onClick={() => void load()}>
              Reload teaching team
            </button>
          </section>
        )}
      </Show>

      <div class="teaching-team-grid">
        <form
          class="auth-panel auth-form teaching-team-card"
          onSubmit={(event) => {
            event.preventDefault();
            void search();
          }}
        >
          <h3>Invite an Instructor</h3>
          <label for="instructor-course-invitation-search">Find an approved colleague</label>
          <input
            id="instructor-course-invitation-search"
            type="search"
            minlength={2}
            maxlength={100}
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
          />
          <p class="field-help">
            Enter at least two characters. Search shows approved eligible people only.
          </p>
          <button
            class="quiet-action"
            type="submit"
            disabled={!queryEligible() || searching() || busy()}
          >
            {searching() ? "Searching..." : "Search eligible people"}
          </button>
          <Show when={targets().length > 0}>
            <ul
              class="teaching-team-results"
              aria-label="Eligible Instructor Course Invitation search results"
            >
              <For each={targets()}>
                {(target) => (
                  <li class="teaching-team-result">
                    <span>{target.account.display}</span>
                    <button
                      class="quiet-action"
                      type="button"
                      aria-pressed={
                        selectedTarget()?.account.reference === target.account.reference
                      }
                      onClick={() => setSelectedTarget(target)}
                    >
                      {selectedTarget()?.account.reference === target.account.reference
                        ? "Selected"
                        : "Select"}
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <Show when={targetCursor() !== null}>
            <button
              class="quiet-action"
              type="button"
              disabled={searching() || busy()}
              onClick={() => void loadMoreTargets()}
            >
              {searching() ? "Loading eligible people..." : "Load more eligible people"}
            </button>
          </Show>
          <button
            class="primary-action"
            type="button"
            disabled={selectedTarget() === null || busy()}
            onClick={() => void invite()}
          >
            Invite selected colleague
          </button>
        </form>

        <section class="auth-panel teaching-team-card" aria-labelledby="active-instructors-heading">
          <h3 id="active-instructors-heading">Active instructors</h3>
          <Show when={data()} fallback={<p role="status">Loading instructors...</p>}>
            {(current) => (
              <>
                <For
                  each={current().instructors}
                  fallback={<p>No active instructors were returned.</p>}
                >
                  {(instructor) => (
                    <article class="teaching-team-row">
                      <div>
                        <strong>{instructor.account.display}</strong>
                        <p class="teaching-team-meta">Active direct instructor</p>
                      </div>
                      <button
                        class="quiet-action"
                        type="button"
                        disabled={busy()}
                        onClick={(event) =>
                          setPendingAction({
                            kind: "remove",
                            row: instructor,
                            trigger: event.currentTarget,
                          })
                        }
                      >
                        Remove
                      </button>
                    </article>
                  )}
                </For>
                <Show when={current().instructorCursor !== null}>
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={busy()}
                    onClick={() => void loadMore("instructors")}
                  >
                    Load more instructors
                  </button>
                </Show>
              </>
            )}
          </Show>
        </section>

        <section
          class="auth-panel teaching-team-card"
          aria-labelledby="pending-instructors-heading"
        >
          <h3 id="pending-instructors-heading">Pending invitations</h3>
          <Show when={data()} fallback={<p role="status">Loading invitations...</p>}>
            {(current) => (
              <>
                <For each={current().invitations} fallback={<p>No pending invitations.</p>}>
                  {(invitation) => (
                    <article class="teaching-team-row">
                      <div>
                        <strong>{invitation.target.account.display}</strong>
                        <p class="teaching-team-meta">
                          {invitationStateLabel(invitation.state)}.{" "}
                          {serverExpiryCopy(invitation.expiresAt)}
                        </p>
                      </div>
                      <Show when={invitation.state === "pending"}>
                        <button
                          class="quiet-action"
                          type="button"
                          disabled={busy()}
                          onClick={(event) =>
                            setPendingAction({
                              kind: "revoke",
                              row: invitation,
                              trigger: event.currentTarget,
                            })
                          }
                        >
                          Cancel invitation
                        </button>
                      </Show>
                    </article>
                  )}
                </For>
                <Show when={current().invitationCursor !== null}>
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={busy()}
                    onClick={() => void loadMore("invitations")}
                  >
                    Load more invitations
                  </button>
                </Show>
              </>
            )}
          </Show>
        </section>
      </div>

      <Show when={pendingAction()}>
        {(action) => (
          <dialog
            class="confirmation-dialog"
            aria-labelledby="teaching-team-confirm-heading"
            ref={(element) => queueMicrotask(() => element.showModal())}
            onCancel={(event) => {
              event.preventDefault();
              cancelAction();
            }}
          >
            <h2 id="teaching-team-confirm-heading">
              {action().kind === "remove" ? "Remove this instructor?" : "Cancel this invitation?"}
            </h2>
            <p>
              {action().kind === "remove"
                ? "This person immediately loses direct instructor access to this course."
                : "This person can no longer accept this pending invitation."}
            </p>
            <div class="action-row">
              <button class="quiet-action" type="button" onClick={cancelAction}>
                Keep it
              </button>
              <button
                class="primary-action"
                type="button"
                ref={(element) => queueMicrotask(() => element.focus())}
                onClick={() => void confirmAction()}
              >
                {action().kind === "remove" ? "Remove instructor" : "Cancel invitation"}
              </button>
            </div>
          </dialog>
        )}
      </Show>
    </section>
  );
}
