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

test("Library Watch inbox keeps exact target and applicable Revision or fork evidence private", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(input.toString(), "https://ple.example"), init));
      return noStoreJson({
        notifications: [
          {
            targetKind: "question",
            targetPublicId: questionId,
            eventKind: "fork",
            revisionNumber: 3,
            forkedPublicId: questionId,
            activityId: null,
            occurredAt: 1_750_000_000_000,
          },
          {
            targetKind: "questionPool",
            targetPublicId: questionId,
            eventKind: "impactNotice",
            revisionNumber: null,
            forkedPublicId: null,
            activityId: "00000000-0000-4000-8000-000000000001",
            occurredAt: 1_750_000_000_001,
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
      eventKind: "fork",
      revisionNumber: 3,
      forkedPublicId: questionId,
      activityId: null,
      occurredAt: 1_750_000_000_000,
    },
    {
      targetKind: "questionPool",
      targetPublicId: questionId,
      eventKind: "impactNotice",
      revisionNumber: null,
      forkedPublicId: null,
      activityId: "00000000-0000-4000-8000-000000000001",
      occurredAt: 1_750_000_000_001,
    },
  ]);
  assert.equal(new URL(requests[0].url).pathname, "/api/library/watch-notifications");
  assert.equal(new URL(requests[0].url).searchParams.get("limit"), "25");
});

test("Library Watch inbox rejects missing fork evidence and recipient facts", () => {
  assert.throws(
    () =>
      decodeLibraryWatchNotifications({
        notifications: [
          {
            targetKind: "questionPool",
            targetPublicId: questionId,
            eventKind: "fork",
            revisionNumber: null,
            forkedPublicId: null,
            activityId: null,
            occurredAt: 1_750_000_000_000,
          },
        ],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeLibraryWatchNotifications({
        notifications: [
          {
            targetKind: "question",
            targetPublicId: questionId,
            eventKind: "revision",
            revisionNumber: 3,
            forkedPublicId: null,
            activityId: null,
            occurredAt: 1_750_000_000_000,
            recipientAccountId: "must-not-be-delivered",
          },
        ],
      }),
    DecodeError,
  );
});
