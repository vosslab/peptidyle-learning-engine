// Private Instructor Watch inbox contracts.

import type { QuestionId } from "../../generated/api/QuestionId";

export type LibraryWatchTargetKind = "question" | "questionPool";
export type LibraryWatchEventKind = "revision" | "fork" | "improvementThread" | "impactNotice";

/** One self-only in-app notification; it contains no watcher or actor identity. */
export interface LibraryWatchNotification {
  readonly targetKind: LibraryWatchTargetKind;
  readonly targetPublicId: QuestionId;
  readonly eventKind: LibraryWatchEventKind;
  readonly revisionNumber: number | null;
  readonly forkedPublicId: QuestionId | null;
  /** Exact thread, post, or impact notice for activity events only. */
  readonly activityId: string | null;
  readonly occurredAt: number;
}

export interface LibraryWatchNotificationClient {
  readonly getLibraryWatchNotifications: (
    limit?: number,
  ) => Promise<ReadonlyArray<LibraryWatchNotification>>;
}
