import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeLibraryWatchNotifications } from "../src/api/decoders/library_watch_notification.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const questionId = publishedQuestionFixture.publishedQuestion.questionId;

function noStoreJson(value) {
  return new Response(JSON.stringify(value), {
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Library Watch inbox returns every discriminated event shape privately", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(input.toString(), "https://ple.example"), init));
      return noStoreJson({
        notifications: [
          {
            targetKind: "question",
            targetPublicId: questionId,
            eventKind: "revision",
            revisionNumber: 2,
            forkedPublicId: null,
            activityId: null,
            occurredAt: 1_750_000_000_000,
          },
          {
            targetKind: "question",
            targetPublicId: questionId,
            eventKind: "fork",
            revisionNumber: 3,
            forkedPublicId: questionId,
            activityId: null,
            occurredAt: 1_750_000_000_001,
          },
          {
            targetKind: "questionPool",
            targetPublicId: questionId,
            eventKind: "improvementThread",
            revisionNumber: 4,
            forkedPublicId: null,
            activityId: "00000000-0000-4000-8000-000000000001",
            occurredAt: 1_750_000_000_002,
          },
          {
            targetKind: "questionPool",
            targetPublicId: questionId,
            eventKind: "impactNotice",
            revisionNumber: null,
            forkedPublicId: null,
            activityId: "00000000-0000-4000-8000-000000000002",
            occurredAt: 1_750_000_000_003,
          },
        ],
      });
    },
  });

  const notifications = await client.getLibraryWatchNotifications();

  assert.deepEqual(notifications, [
    {
      targetKind: "question",
      targetPublicId: questionId,
      eventKind: "revision",
      revisionNumber: 2,
      forkedPublicId: null,
      activityId: null,
      occurredAt: 1_750_000_000_000,
    },
    {
      targetKind: "question",
      targetPublicId: questionId,
      eventKind: "fork",
      revisionNumber: 3,
      forkedPublicId: questionId,
      activityId: null,
      occurredAt: 1_750_000_000_001,
    },
    {
      targetKind: "questionPool",
      targetPublicId: questionId,
      eventKind: "improvementThread",
      revisionNumber: 4,
      forkedPublicId: null,
      activityId: "00000000-0000-4000-8000-000000000001",
      occurredAt: 1_750_000_000_002,
    },
    {
      targetKind: "questionPool",
      targetPublicId: questionId,
      eventKind: "impactNotice",
      revisionNumber: null,
      forkedPublicId: null,
      activityId: "00000000-0000-4000-8000-000000000002",
      occurredAt: 1_750_000_000_003,
    },
  ]);
  assert.equal(new URL(requests[0].url).pathname, "/api/library/watch-notifications");
  assert.equal(new URL(requests[0].url).searchParams.get("limit"), "25");
});

test("Library Watch inbox rejects cross-kind evidence and recipient facts", () => {
  const invalidNotification = (patch) => {
    const notification = {
      targetKind: "question",
      targetPublicId: questionId,
      eventKind: "revision",
      revisionNumber: 3,
      forkedPublicId: null,
      activityId: null,
      occurredAt: 1_750_000_000_000,
    };
    patch(notification);
    return { notifications: [notification] };
  };

  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.revisionNumber = null;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.forkedPublicId = questionId;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.activityId = "00000000-0000-4000-8000-000000000003";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.eventKind = "fork";
          notification.forkedPublicId = null;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.eventKind = "fork";
          notification.forkedPublicId = questionId;
          notification.activityId = "00000000-0000-4000-8000-000000000003";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.eventKind = "improvementThread";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.eventKind = "impactNotice";
          notification.activityId = "00000000-0000-4000-8000-000000000004";
          notification.forkedPublicId = questionId;
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.eventKind = "impactNotice";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.recipientAccountId = "must-not-be-delivered";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.targetKind = "unrecognized";
        }),
      ),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications(
        invalidNotification((notification) => {
          notification.eventKind = "unrecognized";
        }),
      ),
    DecodeError,
  );
});
