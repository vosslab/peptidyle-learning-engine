// Private in-app Watch inbox for the signed-in Instructor.

import { A } from "@solidjs/router";
import { Show, createResource, type JSX } from "solid-js";

import type { LibraryWatchNotification } from "../api/library_watch_notification";
import { useApplicationApi } from "../api/application_api";
import { browserDisplayTimeZone, createDisplayDateTimeFormatter } from "../format_datetime";
import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";

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

function notificationContent(
  formatTimestamp: (timestamp: number | Date) => string,
): (notification: LibraryWatchNotification) => RecordContent {
  return (notification) => {
    const activity = activityHref(notification);
    return {
      title: eventLabel(notification),
      description: `${targetLabel(notification)}: ${notification.targetPublicId}`,
      details: [
        ...(notification.revisionNumber === null
          ? []
          : [
              {
                kind: "text" as const,
                label: notification.targetKind === "questionPool" ? "Edit number" : "Revision",
                value: String(notification.revisionNumber),
              },
            ]),
        ...(notification.forkedPublicId === null
          ? []
          : [{ kind: "text" as const, label: "Fork ID", value: notification.forkedPublicId }]),
        ...(notification.activityId === null
          ? []
          : [{ kind: "text" as const, label: "Activity ID", value: notification.activityId }]),
        {
          kind: "time" as const,
          label: "Occurred",
          value: formatTimestamp(notification.occurredAt),
          dateTime: new Date(notification.occurredAt).toISOString(),
        },
      ],
      actions:
        activity === null
          ? []
          : [
              {
                id: "open-exact-activity",
                kind: "link" as const,
                label: "Open exact activity",
                href: activity,
                primary: true,
              },
            ],
    };
  };
}

function notificationId(notification: LibraryWatchNotification): string {
  return [
    notification.eventKind,
    notification.targetPublicId,
    notification.occurredAt,
    notification.revisionNumber ?? "",
    notification.forkedPublicId ?? "",
    notification.activityId ?? "",
  ].join("-");
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
        content={notificationContent(formatTimestamp)}
        recordId={notificationId}
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
