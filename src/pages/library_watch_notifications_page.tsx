// Private in-app Watch inbox for the signed-in Instructor.

import { A } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { LibraryWatchNotification } from "../api/library_watch_notification";
import { useApplicationApi } from "../api/application_api";
import { browserDisplayTimeZone, createDisplayDateTimeFormatter } from "../format_datetime";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";

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

function notificationRegions(
  formatTimestamp: (timestamp: number | Date) => string,
): ReadonlyArray<RecordRegion<LibraryWatchNotification>> {
  return [
    {
      id: "event",
      role: "identity",
      priority: "required",
      width: "minmax(11rem, 1fr)",
      align: "start",
      content: (notification) => (
        <>
          <span class="eyebrow">{eventLabel(notification)}</span>
          <strong>{targetLabel(notification)}</strong>
          <span>Library ID: {notification.targetPublicId}</span>
        </>
      ),
    },
    {
      id: "details",
      role: "metadata",
      priority: "high",
      width: "minmax(12rem, 1fr)",
      align: "start",
      content: (notification) => (
        <>
          <Show when={notification.revisionNumber !== null}>
            <span>
              {notification.targetKind === "questionPool" ? "Edit Number" : "Revision"}:{" "}
              {notification.revisionNumber}
            </span>
          </Show>
          <Show when={notification.forkedPublicId !== null}>
            <span>Fork ID: {notification.forkedPublicId}</span>
          </Show>
          <Show when={notification.activityId !== null}>
            <span>Activity ID: {notification.activityId}</span>
          </Show>
        </>
      ),
    },
    {
      id: "occurred-at",
      role: "status",
      priority: "medium",
      width: "minmax(11rem, auto)",
      align: "end",
      content: (notification) => (
        <time datetime={new Date(notification.occurredAt).toISOString()}>
          {formatTimestamp(notification.occurredAt)}
        </time>
      ),
    },
    {
      id: "actions",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "end",
      content: (notification) => (
        <Show when={activityHref(notification)}>
          {(href) => (
            <A class="primary-link" href={href()}>
              Open exact activity
            </A>
          )}
        </Show>
      ),
    },
  ];
}

/** Shows only this active Instructor's private, newest-first Watch inbox. */
export function LibraryWatchNotificationsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const [notifications, { refetch }] = createResource(() =>
    runtime.client.getLibraryWatchNotifications(),
  );
  const failed = (): boolean => inboxFailed(notifications.error);
  const formatTimestamp = createDisplayDateTimeFormatter(browserDisplayTimeZone());
  const notificationListState = (): RecordListState => {
    if (notifications.loading) return { kind: "loading", label: "Loading Watch activity..." };
    if (failed()) {
      return {
        kind: "error",
        title: "Watch activity could not load",
        message: "Check your connection and try again.",
        retry: (): void => void refetch(),
      };
    }
    return { kind: "ready" };
  };

  return (
    <PageFrame
      routeSurface="libraryWatchNotifications"
      headingId="library-watch-notifications-heading"
      eyebrow="Question Library"
      title="Watch activity"
      lede="Changes and stewardship activity for the Published Questions and Question Pools you watch. Your watch list and this inbox are private."
    >
      <RecordList
        ariaLabel="Private Watch activity"
        emptyState={{
          title: "No Watch activity yet",
          message: "New Revisions, public forks, and stewardship activity will appear here.",
        }}
        recordId={(notification) =>
          `${notification.eventKind}-${notification.targetPublicId}-${notification.occurredAt}`
        }
        regions={notificationRegions(formatTimestamp)}
        rows={notifications() ?? []}
        state={notificationListState()}
      />
      <Show when={!notifications.loading && !failed() && notifications()?.length === 0}>
        <p>
          <A class="primary-link" href="/library">
            Open Question Library
          </A>
        </p>
      </Show>
    </PageFrame>
  );
}
