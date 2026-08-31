// Account-owned response surface for pending Course Invitations.

import { For, Show, createSignal, onMount, type JSX } from "solid-js";

import type { CourseInvitationTerminalAction } from "../../generated/api/CourseInvitationTerminalAction";
import type { PendingCourseInvitationView } from "../../generated/api/PendingCourseInvitationView";
import { ApiRequestError } from "../api/http_client/error";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import {
  appendTeachingTeamPage,
  conflictRecoveryCopy,
  invitationStateLabel,
  isPendingInvitation,
  serverExpiryCopy,
} from "./teaching_team_model";
import "./teaching_team_panel.css";

interface PendingInvitationData {
  readonly invitations: ReadonlyArray<PendingCourseInvitationView>;
  readonly nextCursor: string | null;
}

interface PendingResponse {
  readonly invitation: PendingCourseInvitationView;
  readonly action: CourseInvitationTerminalAction;
  readonly trigger: HTMLButtonElement;
}

function responseErrorCopy(error: unknown): string {
  if (error instanceof ApiRequestError && error.status === 412) return conflictRecoveryCopy();
  return "Your invitation response could not be saved. Check your connection and try again.";
}

function responseHeading(action: CourseInvitationTerminalAction): string {
  return action === "accept" ? "Accept this invitation?" : "Decline this invitation?";
}

function responseCopy(action: CourseInvitationTerminalAction): string {
  return action === "accept"
    ? "Accepting grants you direct course membership."
    : "Declining closes this invitation. A course instructor can invite you again later.";
}

export function AccountPendingInvitationsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const [data, setData] = createSignal<PendingInvitationData | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");
  const [pendingResponse, setPendingResponse] = createSignal<PendingResponse | null>(null);
  let heading: HTMLHeadingElement | undefined;

  async function load(): Promise<void> {
    if (session.state().kind !== "authenticated") return;
    setError(null);
    try {
      const page = await runtime.client.listPendingCourseInvitations(undefined, 25);
      setData({ invitations: page.invitations, nextCursor: page.nextCursor });
    } catch {
      setError("Your pending invitations could not load. Check your connection and try again.");
    }
  }

  async function loadMore(): Promise<void> {
    const current = data();
    if (current === null || current.nextCursor === null) return;
    setBusy(true);
    setError(null);
    try {
      const page = await runtime.client.listPendingCourseInvitations(current.nextCursor, 25);
      setData({
        invitations: appendTeachingTeamPage(current.invitations, page.invitations),
        nextCursor: page.nextCursor,
      });
      setAnnouncement(`Loaded ${page.invitations.length} more invitations.`);
    } catch {
      setError("More invitations could not load. Check your connection and try again.");
    } finally {
      setBusy(false);
    }
  }

  function cancelResponse(): void {
    const response = pendingResponse();
    setPendingResponse(null);
    queueMicrotask(() => response?.trigger.focus());
  }

  async function confirmResponse(): Promise<void> {
    const response = pendingResponse();
    if (response === null) return;
    setPendingResponse(null);
    setBusy(true);
    setError(null);
    try {
      await runtime.client.respondToCourseInvitation(
        response.invitation.reference,
        { action: response.action },
        response.invitation.revision,
      );
      setAnnouncement(
        response.action === "accept" ? "Invitation accepted." : "Invitation declined.",
      );
      await load();
      queueMicrotask(() => heading?.focus());
    } catch (caught) {
      setError(responseErrorCopy(caught));
      if (caught instanceof ApiRequestError && caught.status === 412) await load();
    } finally {
      setBusy(false);
    }
  }

  onMount(() => void load());

  return (
    <section
      class="page pending-invitations-page"
      data-route-surface="accountPendingInvitations"
      aria-labelledby="pending-invitations-heading"
    >
      <p class="eyebrow">Account invitations</p>
      <h1 ref={(element) => (heading = element)} id="pending-invitations-heading" tabindex={-1}>
        Pending teaching invitations
      </h1>
      <p class="page-lede">
        Review invitations addressed to this signed-in account. An invitation is not course
        authority until you accept it.
      </p>
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
      <Show when={session.state().kind !== "authenticated"}>
        <section class="inline-error" role="alert">
          <p>Sign in to review invitations addressed to this account.</p>
        </section>
      </Show>
      <Show when={session.state().kind === "authenticated"}>
        <Show when={error()}>
          {(message) => (
            <section class="inline-error" role="alert">
              <p>{message()}</p>
              <button class="quiet-action" type="button" onClick={() => void load()}>
                Retry
              </button>
            </section>
          )}
        </Show>
        <Show
          when={data()}
          fallback={
            <p class="loading-state" role="status">
              Loading pending invitations...
            </p>
          }
        >
          {(current) => (
            <section class="pending-invitation-list" aria-label="Pending teaching invitations">
              <For
                each={current().invitations}
                fallback={
                  <section class="auth-panel empty-state">
                    <h2>No invitations waiting</h2>
                    <p>When a course instructor invites this account, it appears here.</p>
                  </section>
                }
              >
                {(invitation) => (
                  <article class="auth-panel pending-invitation-card">
                    <div>
                      <h2>{invitation.courseLabel}</h2>
                      <p class="teaching-team-meta">{invitationStateLabel(invitation.state)}</p>
                      <p class="teaching-team-meta">{serverExpiryCopy(invitation.expiresAt)}</p>
                    </div>
                    <Show
                      when={isPendingInvitation(invitation.state)}
                      fallback={
                        <p class="teaching-team-meta">This invitation is no longer actionable.</p>
                      }
                    >
                      <div class="action-row">
                        <button
                          class="quiet-action"
                          type="button"
                          disabled={busy()}
                          onClick={(event) =>
                            setPendingResponse({
                              invitation,
                              action: "decline",
                              trigger: event.currentTarget,
                            })
                          }
                        >
                          Decline
                        </button>
                        <button
                          class="primary-action"
                          type="button"
                          disabled={busy()}
                          onClick={(event) =>
                            setPendingResponse({
                              invitation,
                              action: "accept",
                              trigger: event.currentTarget,
                            })
                          }
                        >
                          Accept
                        </button>
                      </div>
                    </Show>
                  </article>
                )}
              </For>
              <Show when={current().nextCursor !== null}>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={busy()}
                  onClick={() => void loadMore()}
                >
                  Load more invitations
                </button>
              </Show>
            </section>
          )}
        </Show>
        <Show when={pendingResponse()}>
          {(response) => (
            <dialog
              class="confirmation-dialog"
              aria-labelledby="pending-invitation-confirm-heading"
              ref={(element) => queueMicrotask(() => element.showModal())}
              onCancel={(event) => {
                event.preventDefault();
                cancelResponse();
              }}
            >
              <h2 id="pending-invitation-confirm-heading">{responseHeading(response().action)}</h2>
              <p>{responseCopy(response().action)}</p>
              <div class="action-row">
                <button class="quiet-action" type="button" onClick={cancelResponse}>
                  Go back
                </button>
                <button
                  class="primary-action"
                  type="button"
                  ref={(element) => queueMicrotask(() => element.focus())}
                  onClick={() => void confirmResponse()}
                >
                  {response().action === "accept" ? "Accept invitation" : "Decline invitation"}
                </button>
              </div>
            </dialog>
          )}
        </Show>
      </Show>
    </section>
  );
}
