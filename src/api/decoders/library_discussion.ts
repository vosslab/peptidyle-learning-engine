// Strict untrusted-response decoder for Library improvement threads and notices.

import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";
import type {
  ImpactNoticeState,
  ImprovementThreadState,
  LibraryDiscussionView,
  LibraryImpactNotice,
  LibraryImprovementPost,
  LibraryImprovementThread,
} from "../library_discussion";

const THREAD_STATES = ["open", "resolved"] as const satisfies ReadonlyArray<ImprovementThreadState>;
const NOTICE_STATES = ["active", "cancelled"] as const satisfies ReadonlyArray<ImpactNoticeState>;

function hasControlCharacter(value: string): boolean {
  return Array.from(value).some((character) => {
    const codePoint = character.codePointAt(0);
    return (
      codePoint !== undefined &&
      ((codePoint >= 0 && codePoint <= 0x1f) || (codePoint >= 0x7f && codePoint <= 0x9f))
    );
  });
}

function hasUnpairedSurrogate(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const codeUnit = value.charCodeAt(index);
    if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      const nextCodeUnit = value.charCodeAt(index + 1);
      if (!(nextCodeUnit >= 0xdc00 && nextCodeUnit <= 0xdfff)) {
        return true;
      }
      index += 1;
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      return true;
    }
  }
  return false;
}

function text(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  // ASVS 2.2.1: Accept only the bounded Unicode scalar-value text contract.
  if (
    decoded !== decoded.trim() ||
    decoded.length === 0 ||
    Array.from(decoded).length > 4_000 ||
    hasControlCharacter(decoded) ||
    hasUnpairedSurrogate(decoded)
  ) {
    throw new DecodeError(
      path,
      "trimmed control-free text no longer than 4000 Unicode scalar values",
    );
  }
  return decoded;
}

function timestamp(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 0) throw new DecodeError(path, "a nonnegative millisecond timestamp");
  return decoded;
}

function post(value: unknown, path: string): LibraryImprovementPost {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "postId",
    "authorDisplayName",
    "body",
    "createdAt",
    "updatedAt",
    "viewerMayEdit",
  ]);
  const createdAt = timestamp(field(record, "createdAt", path), `${path}.createdAt`);
  const updatedAt = decodeNullable(
    field(record, "updatedAt", path),
    `${path}.updatedAt`,
    timestamp,
  );
  if (updatedAt !== null && updatedAt < createdAt) {
    throw new DecodeError(`${path}.updatedAt`, "a timestamp no earlier than createdAt");
  }
  return {
    postId: decodeUuid(field(record, "postId", path), `${path}.postId`),
    authorDisplayName: text(field(record, "authorDisplayName", path), `${path}.authorDisplayName`),
    body: text(field(record, "body", path), `${path}.body`),
    createdAt,
    updatedAt,
    viewerMayEdit: decodeBoolean(field(record, "viewerMayEdit", path), `${path}.viewerMayEdit`),
  };
}

function thread(value: unknown, path: string): LibraryImprovementThread {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "threadId",
    "creationRevisionNumber",
    "state",
    "createdAt",
    "resolvedAt",
    "viewerMayResolve",
    "posts",
  ]);
  const state = decodeStringEnum(field(record, "state", path), `${path}.state`, THREAD_STATES);
  const createdAt = timestamp(field(record, "createdAt", path), `${path}.createdAt`);
  const resolvedAt = decodeNullable(
    field(record, "resolvedAt", path),
    `${path}.resolvedAt`,
    timestamp,
  );
  const posts = decodeArray(field(record, "posts", path), `${path}.posts`, post);
  if (posts.length === 0) {
    throw new DecodeError(`${path}.posts`, "a retained thread with its initial post");
  }
  const initialPost = posts[0];
  if (initialPost === undefined || initialPost.createdAt !== createdAt) {
    throw new DecodeError(`${path}.posts[0].createdAt`, "the thread creation timestamp");
  }
  if (posts.some((item) => item.createdAt < createdAt)) {
    throw new DecodeError(`${path}.posts`, "posts created no earlier than the thread");
  }
  const common = {
    threadId: decodeUuid(field(record, "threadId", path), `${path}.threadId`),
    creationRevisionNumber: decodePositiveInteger(
      field(record, "creationRevisionNumber", path),
      `${path}.creationRevisionNumber`,
    ),
    state,
    createdAt,
    resolvedAt,
    viewerMayResolve: decodeBoolean(
      field(record, "viewerMayResolve", path),
      `${path}.viewerMayResolve`,
    ),
    posts,
  };
  switch (state) {
    case "open":
      if (resolvedAt !== null) {
        throw new DecodeError(`${path}.resolvedAt`, "no resolved timestamp for an open thread");
      }
      return { ...common, state, resolvedAt };
    case "resolved":
      if (resolvedAt === null || resolvedAt < createdAt) {
        throw new DecodeError(
          `${path}.resolvedAt`,
          "a resolved timestamp no earlier than the thread creation timestamp",
        );
      }
      return { ...common, state, resolvedAt };
  }
}

function impactNotice(value: unknown, path: string): LibraryImpactNotice {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "impactNoticeId",
    "affectedRevisionNumber",
    "authorDisplayName",
    "body",
    "state",
    "createdAt",
    "updatedAt",
    "cancelledAt",
    "viewerMayManage",
  ]);
  const state = decodeStringEnum(field(record, "state", path), `${path}.state`, NOTICE_STATES);
  const createdAt = timestamp(field(record, "createdAt", path), `${path}.createdAt`);
  const updatedAt = timestamp(field(record, "updatedAt", path), `${path}.updatedAt`);
  const cancelledAt = decodeNullable(
    field(record, "cancelledAt", path),
    `${path}.cancelledAt`,
    timestamp,
  );
  if (updatedAt < createdAt) {
    throw new DecodeError(`${path}.updatedAt`, "a timestamp no earlier than createdAt");
  }
  const viewerMayManage = decodeBoolean(
    field(record, "viewerMayManage", path),
    `${path}.viewerMayManage`,
  );
  const common = {
    impactNoticeId: decodeUuid(field(record, "impactNoticeId", path), `${path}.impactNoticeId`),
    affectedRevisionNumber: decodeNullable(
      field(record, "affectedRevisionNumber", path),
      `${path}.affectedRevisionNumber`,
      decodePositiveInteger,
    ),
    authorDisplayName: text(field(record, "authorDisplayName", path), `${path}.authorDisplayName`),
    body: text(field(record, "body", path), `${path}.body`),
    createdAt,
    updatedAt,
  };
  switch (state) {
    case "active":
      if (cancelledAt !== null) {
        throw new DecodeError(`${path}.cancelledAt`, "no cancelled timestamp for an active notice");
      }
      return { ...common, state, cancelledAt, viewerMayManage };
    case "cancelled":
      if (
        cancelledAt === null ||
        cancelledAt < createdAt ||
        updatedAt !== cancelledAt ||
        viewerMayManage
      ) {
        throw new DecodeError(
          `${path}.cancelledAt`,
          "a retained cancelled notice with its terminal timestamp and no management capability",
        );
      }
      return { ...common, state, cancelledAt, viewerMayManage };
  }
}

export function decodeLibraryDiscussionView(
  value: unknown,
  path = "response",
): LibraryDiscussionView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["viewerMayManage", "threads", "impactNotices"]);
  return {
    viewerMayManage: decodeBoolean(
      field(record, "viewerMayManage", path),
      `${path}.viewerMayManage`,
    ),
    threads: decodeArray(field(record, "threads", path), `${path}.threads`, thread),
    impactNotices: decodeArray(
      field(record, "impactNotices", path),
      `${path}.impactNotices`,
      impactNotice,
    ),
  };
}
