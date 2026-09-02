import assert from "node:assert/strict";
import test from "node:test";

import { decodeQuestionBackendLocator } from "../src/api/decoders/question_model.ts";

test("Question Backend Locator accepts locations while refusing Question Source references", () => {
  assert.deepEqual(decodeQuestionBackendLocator({ backend: "qti", itemId: "item-17" }, "locator"), {
    backend: "qti",
    itemId: "item-17",
  });
  assert.deepEqual(
    decodeQuestionBackendLocator(
      {
        backend: "imathas",
        deploymentReference: "recorded-imathas",
        itemReference: "item-17",
        profile: "recorded-v1",
      },
      "locator",
    ),
    {
      backend: "imathas",
      deploymentReference: "recorded-imathas",
      itemReference: "item-17",
      profile: "recorded-v1",
    },
  );
  for (const sourceReference of [
    {
      backend: "qti",
      itemId: "item-17",
      packageObject: "00000000-0000-0000-0000-000000000017",
      packageSha256: "a".repeat(64),
    },
    {
      backend: "imathas",
      deploymentReference: "recorded-imathas",
      itemReference: "item-17",
      profile: "recorded-v1",
      snapshot: "00000000-0000-0000-0000-000000000017",
      snapshotSha256: "a".repeat(64),
    },
  ]) {
    assert.throws(() => decodeQuestionBackendLocator(sourceReference, "locator"));
  }
  assert.throws(() =>
    decodeQuestionBackendLocator(
      {
        backend: "imathas",
        deploymentReference: "https://untrusted.example",
        itemReference: "item..17",
        profile: "imathas_remote_grading_v1",
      },
      "locator",
    ),
  );
});
