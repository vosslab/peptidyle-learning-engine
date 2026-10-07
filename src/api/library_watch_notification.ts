// Private Instructor Watch inbox contracts.

import type { LibraryObjectId } from "../../generated/api/LibraryObjectId";

export type LibraryWatchTargetKind = "question" | "questionPool";

/** One self-only in-app notification; it contains no watcher or actor identity. */
type LibraryWatchNotificationBase = {
  readonly targetPublicId: LibraryObjectId;
  readonly occurredAt: number;
};

export type LibraryWatchNotification =
  | (LibraryWatchNotificationBase & {
      readonly targetKind: "question";
      readonly eventKind: "revision";
      readonly questionRevisionNumber: number;
      readonly questionPoolEditNumber: null;
      readonly forkedPublicId: null;
    })
  | (LibraryWatchNotificationBase & {
      readonly targetKind: "questionPool";
      readonly eventKind: "membersChanged";
      /** The Question Pool Edit Number after the member-list save. */
      readonly questionRevisionNumber: null;
      readonly questionPoolEditNumber: number;
      readonly forkedPublicId: null;
    })
  | (LibraryWatchNotificationBase & {
      readonly targetKind: "question";
      readonly eventKind: "fork";
      /** The source Question Revision Number. */
      readonly questionRevisionNumber: number;
      readonly questionPoolEditNumber: null;
      readonly forkedPublicId: LibraryObjectId;
    })
  | (LibraryWatchNotificationBase & {
      readonly targetKind: "questionPool";
      readonly eventKind: "fork";
      /** The source Question Pool Edit Number. */
      readonly questionRevisionNumber: null;
      readonly questionPoolEditNumber: number;
      readonly forkedPublicId: LibraryObjectId;
    });

export type LibraryWatchEventKind = LibraryWatchNotification["eventKind"];

export interface LibraryWatchNotificationClient {
  readonly getLibraryWatchNotifications: (
    limit?: number,
  ) => Promise<ReadonlyArray<LibraryWatchNotification>>;
}
