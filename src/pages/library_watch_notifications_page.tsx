// Private in-app Watch inbox for the signed-in Instructor.

import { A } from "@solidjs/router";
import { For, Show, createResource, type JSX } from "solid-js";

import type { LibraryWatchNotification } from "../api/library_watch_notification";
import { useApplicationApi } from "../api/application_api";

function eventLabel(value: LibraryWatchNotification): string {
  switch (value.eventKind) {
    case "revision":
      return "New Revision";
    case "fork":
      return "New public fork";
    case "improvementThread":
      return "Improvement thread activity";
    case "impactNotice":
      return "Impact notice activity";
  }
}

function targetLabel(value: LibraryWatchNotification): string {
  return value.targetKind === "question" ? "Published Question" : "Question Pool";
}

function timestamp(value: number): string {
  return new Date(value).toLocaleString();
}

function activityHref(value: LibraryWatchNotification): string | null {
  if (value.activityId === null) return null;
  const activity = encodeURIComponent(value.activityId);
  if (value.targetKind === "question") {
    return `/library/${encodeURIComponent(value.targetPublicId)}#library-activity-${activity}`;
  }
  return `/library?pool=${encodeURIComponent(value.targetPublicId)}#library-activity-${activity}`;
}

function inboxFailed(value: unknown): value is Error {
  return value instanceof Error;
}

/** Shows only this active Instructor's private, newest-first Watch inbox. */
export function LibraryWatchNotificationsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const [notifications, { refetch }] = createResource(() =>
    runtime.client.getLibraryWatchNotifications(),
  );
  const failed = (): boolean => inboxFailed(notifications.error);

  return (
    <section
      class="page"
      data-route-surface="libraryWatchNotifications"
      aria-labelledby="library-watch-notifications-heading"
    >
      <p class="eyebrow">Question Library</p>
      <h1 id="library-watch-notifications-heading">Watch activity</h1>
      <p class="page-lede">
        Changes and stewardship activity for the Published Questions and Question Pools you watch.
        Your watch list and this inbox are private.
      </p>
      <Show when={failed()}>
        <section class="inline-error" role="alert">
          <p>Your Watch activity could not load. Check your connection and try again.</p>
          <button class="quiet-action" type="button" onClick={() => void refetch()}>
            Retry
          </button>
        </section>
      </Show>
      <Show
        when={!failed() && !notifications.loading && notifications()}
        fallback={
          <Show when={!failed()}>
            <p class="loading-state" role="status">
              Loading Watch activity...
            </p>
          </Show>
        }
      >
        {(items) => (
          <section aria-label="Private Watch activity">
            <For
              each={items()}
              fallback={
                <section class="auth-panel empty-state">
                  <h2>No Watch activity yet</h2>
                  <p>New Revisions, public forks, and stewardship activity will appear here.</p>
                  <A class="primary-link" href="/library">
                    Open Question Library
                  </A>
                </section>
              }
            >
              {(notification) => (
                <article class="auth-panel">
                  <p class="eyebrow">{eventLabel(notification)}</p>
                  <h2>{targetLabel(notification)}</h2>
                  <p>
                    <strong>Library ID:</strong> {notification.targetPublicId}
                  </p>
                  <Show when={notification.revisionNumber !== null}>
                    <p>
                      <strong>
                        {notification.targetKind === "questionPool" ? "Edit Number" : "Revision"}:
                      </strong>{" "}
                      {notification.revisionNumber}
                    </p>
                  </Show>
                  <Show when={notification.forkedPublicId !== null}>
                    <p>
                      <strong>Fork ID:</strong> {notification.forkedPublicId}
                    </p>
                  </Show>
                  <Show when={activityHref(notification)}>
                    {(href) => (
                      <>
                        <p>
                          <strong>Activity ID:</strong> {notification.activityId}
                        </p>
                        <p>
                          <A class="primary-link" href={href()}>
                            Open exact activity
                          </A>
                        </p>
                      </>
                    )}
                  </Show>
                  <p class="teaching-team-meta">
                    <time datetime={new Date(notification.occurredAt).toISOString()}>
                      {timestamp(notification.occurredAt)}
                    </time>
                  </p>
                </article>
              )}
            </For>
          </section>
        )}
      </Show>
    </section>
  );
}
