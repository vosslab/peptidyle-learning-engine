// Private Instructor Watch inbox contracts.

import type { LibraryObjectId } from "../../generated/api/LibraryObjectId";

export type LibraryWatchTargetKind = "question" | "questionPool";

/** One self-only in-app notification; it contains no watcher or actor identity. */
type LibraryWatchNotificationBase = {
  readonly eventId: string;
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

export type LibraryWatchQuery = {
  readonly q: string;
  readonly targetKinds: ReadonlyArray<LibraryWatchTargetKind>;
  readonly eventKinds: ReadonlyArray<LibraryWatchEventKind>;
  readonly direction: "ascending" | "descending";
};

export type LibraryWatchFacet = { readonly value: string; readonly count: number };

export type LibraryWatchPage = {
  readonly notifications: ReadonlyArray<LibraryWatchNotification>;
  readonly nextCursor: string | null;
  readonly targetKindFacets: ReadonlyArray<LibraryWatchFacet>;
  readonly eventKindFacets: ReadonlyArray<LibraryWatchFacet>;
};

export interface LibraryWatchNotificationClient {
  readonly getLibraryWatchNotifications: (
    query: LibraryWatchQuery,
    cursor?: string | null,
    limit?: number,
  ) => Promise<LibraryWatchPage>;
}
