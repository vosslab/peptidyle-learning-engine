import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeLibraryWatchNotifications } from "../src/api/decoders/library_watch_notification.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

const questionId = "7K3M-79QP";
const questionPoolId = "3S8B-24DZ";
const forkedPoolId = "2R5X-E7YA";

function noStoreJson(value) {
  return new Response(JSON.stringify(value), {
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Library Watch inbox decodes Revisions, Pool changes, and forks privately", async () => {
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
            questionRevisionNumber: 2,
            questionPoolEditNumber: null,
            forkedPublicId: null,
            occurredAt: 1_750_000_000_000,
          },
          {
            targetKind: "questionPool",
            targetPublicId: questionPoolId,
            eventKind: "membersChanged",
            questionRevisionNumber: null,
            questionPoolEditNumber: 5,
            forkedPublicId: null,
            occurredAt: 1_750_000_000_004,
          },
          {
            targetKind: "question",
            targetPublicId: questionId,
            eventKind: "fork",
            questionRevisionNumber: 3,
            questionPoolEditNumber: null,
            forkedPublicId: questionId,
            occurredAt: 1_750_000_000_001,
          },
          {
            targetKind: "questionPool",
            targetPublicId: questionPoolId,
            eventKind: "fork",
            questionRevisionNumber: null,
            questionPoolEditNumber: 1,
            forkedPublicId: forkedPoolId,
            occurredAt: 1_750_000_000_002,
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
      questionRevisionNumber: 2,
      questionPoolEditNumber: null,
      forkedPublicId: null,
      occurredAt: 1_750_000_000_000,
    },
    {
      targetKind: "questionPool",
      targetPublicId: questionPoolId,
      eventKind: "membersChanged",
      questionRevisionNumber: null,
      questionPoolEditNumber: 5,
      forkedPublicId: null,
      occurredAt: 1_750_000_000_004,
    },
    {
      targetKind: "question",
      targetPublicId: questionId,
      eventKind: "fork",
      questionRevisionNumber: 3,
      questionPoolEditNumber: null,
      forkedPublicId: questionId,
      occurredAt: 1_750_000_000_001,
    },
    {
      targetKind: "questionPool",
      targetPublicId: questionPoolId,
      eventKind: "fork",
      questionRevisionNumber: null,
      questionPoolEditNumber: 1,
      forkedPublicId: forkedPoolId,
      occurredAt: 1_750_000_000_002,
    },
  ]);
  assert.deepEqual(
    notifications.map((notification) => notification.eventKind),
    ["revision", "membersChanged", "fork", "fork"],
  );
  assert.equal(new URL(requests[0].url).pathname, "/api/library/watch-notifications");
  assert.equal(new URL(requests[0].url).searchParams.get("limit"), "25");
});

test("Library Watch inbox rejects incomplete evidence and recipient facts", () => {
  const invalidNotification = (patch) => {
    const notification = {
      targetKind: "question",
      targetPublicId: questionId,
      eventKind: "revision",
      questionRevisionNumber: 3,
      questionPoolEditNumber: null,
      forkedPublicId: null,
      occurredAt: 1_750_000_000_000,
    };
    patch(notification);
    return { notifications: [notification] };
  };

  for (const patch of [
    (notification) => {
      notification.questionRevisionNumber = null;
    },
    (notification) => {
      notification.forkedPublicId = questionId;
    },
    (notification) => {
      notification.eventKind = "unrecognized";
    },
    (notification) => {
      notification.recipientAccountId = "must-not-be-delivered";
    },
    (notification) => {
      notification.targetKind = "unrecognized";
    },
    (notification) => {
      notification.targetKind = "questionPool";
    },
    (notification) => {
      notification.eventKind = "membersChanged";
    },
    (notification) => {
      notification.eventKind = "fork";
      notification.forkedPublicId = null;
    },
    (notification) => {
      notification.questionRevisionNumber = null;
      notification.questionPoolEditNumber = 3;
    },
    (notification) => {
      notification.questionPoolEditNumber = 3;
    },
  ]) {
    assert.throws(() => decodeLibraryWatchNotifications(invalidNotification(patch)), DecodeError);
  }
});
