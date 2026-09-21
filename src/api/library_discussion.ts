// Retained text-only Question Library improvement-thread and impact-notice contracts.

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";

export type LibraryObjectDiscussionKind = "question" | "questionPool";

export interface LibraryImprovementPost {
  readonly postId: string;
  readonly authorDisplayName: string;
  readonly body: string;
  readonly createdAt: number;
  readonly updatedAt: number | null;
  readonly viewerMayEdit: boolean;
}

type LibraryImprovementThreadBase = {
  readonly threadId: string;
  readonly creationRevisionNumber: number;
  readonly createdAt: number;
  readonly viewerMayResolve: boolean;
  readonly posts: ReadonlyArray<LibraryImprovementPost>;
};

export type LibraryImprovementThread =
  | (LibraryImprovementThreadBase & {
      readonly state: "open";
      readonly resolvedAt: null;
    })
  | (LibraryImprovementThreadBase & {
      readonly state: "resolved";
      readonly resolvedAt: number;
    });

export type ImprovementThreadState = LibraryImprovementThread["state"];

type LibraryImpactNoticeBase = {
  readonly impactNoticeId: string;
  readonly affectedRevisionNumber: number | null;
  readonly authorDisplayName: string;
  readonly body: string;
  readonly createdAt: number;
  readonly updatedAt: number;
};

export type LibraryImpactNotice =
  | (LibraryImpactNoticeBase & {
      readonly state: "active";
      readonly cancelledAt: null;
      readonly viewerMayManage: boolean;
    })
  | (LibraryImpactNoticeBase & {
      readonly state: "cancelled";
      readonly cancelledAt: number;
      readonly viewerMayManage: false;
    });

export type ImpactNoticeState = LibraryImpactNotice["state"];

export interface LibraryDiscussionView {
  readonly viewerMayManage: boolean;
  readonly threads: ReadonlyArray<LibraryImprovementThread>;
  readonly impactNotices: ReadonlyArray<LibraryImpactNotice>;
}

/** Same-origin capability for one stable Question or Pool lineage. */
export interface LibraryDiscussionClient {
  readonly getLibraryDiscussion: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
  ) => Promise<LibraryDiscussionView>;
  readonly createImprovementThread: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
    body: string,
  ) => Promise<void>;
  readonly replyToImprovementThread: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
    threadId: string,
    body: string,
  ) => Promise<void>;
  readonly editOwnImprovementPost: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
    postId: string,
    body: string,
  ) => Promise<void>;
  readonly setImprovementThreadResolved: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
    threadId: string,
    resolved: boolean,
  ) => Promise<void>;
  readonly createImpactNotice: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
    affectedRevisionNumber: number | null,
    body: string,
  ) => Promise<void>;
  readonly updateImpactNotice: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
    impactNoticeId: string,
    affectedRevisionNumber: number | null,
    body: string,
  ) => Promise<void>;
  readonly cancelImpactNotice: (
    kind: LibraryObjectDiscussionKind,
    publicId: PublishedQuestionId,
    impactNoticeId: string,
  ) => Promise<void>;
}
