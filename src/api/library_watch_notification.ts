// Private Instructor Watch inbox contracts.

import type { QuestionId } from "../../generated/api/QuestionId";

export type LibraryWatchTargetKind = "question" | "questionPool";

/** One self-only in-app notification; it contains no watcher or actor identity. */
type LibraryWatchNotificationBase = {
  readonly targetKind: LibraryWatchTargetKind;
  readonly targetPublicId: QuestionId;
  readonly occurredAt: number;
};

export type LibraryWatchNotification =
  | (LibraryWatchNotificationBase & {
      readonly eventKind: "revision";
      readonly revisionNumber: number;
      readonly forkedPublicId: null;
      readonly activityId: null;
    })
  | (LibraryWatchNotificationBase & {
      readonly eventKind: "fork";
      readonly revisionNumber: number;
      readonly forkedPublicId: QuestionId;
      readonly activityId: null;
    })
  | (LibraryWatchNotificationBase & {
      readonly eventKind: "improvementThread";
      /** Creation Revision and exact retained thread. */
      readonly revisionNumber: number;
      readonly forkedPublicId: null;
      readonly activityId: string;
    })
  | (LibraryWatchNotificationBase & {
      readonly eventKind: "impactNotice";
      /** The affected Revision can be absent; activityId is the exact retained notice. */
      readonly revisionNumber: number | null;
      readonly forkedPublicId: null;
      readonly activityId: string;
    });

export type LibraryWatchEventKind = LibraryWatchNotification["eventKind"];

export interface LibraryWatchNotificationClient {
  readonly getLibraryWatchNotifications: (
    limit?: number,
  ) => Promise<ReadonlyArray<LibraryWatchNotification>>;
}
