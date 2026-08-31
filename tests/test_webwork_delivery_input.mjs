import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  requireWebworkPublishedQuestionFixtureInput,
  webworkPublishedQuestionTitle,
  writeVisibleIssuanceAcknowledgement,
} from "./playwright/e2e/webwork_delivery_input.ts";

test("WebWork Published Question fixture input accepts only the canonical public hand-off", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-webwork-delivery-input-"));
  try {
    const path = join(directory, "published-question-fixture.json");
    writeFileSync(
      path,
      JSON.stringify({
        questionId: "ABC-1234",
        scenarioId: "webwork_delivery",
        schemaVersion: 1,
        title: webworkPublishedQuestionTitle,
      }),
      { encoding: "ascii", mode: 0o600 },
    );
    chmodSync(path, 0o600);
    assert.deepEqual(
      requireWebworkPublishedQuestionFixtureInput({
        PLE_WEBWORK_PUBLISHED_QUESTION_FIXTURE_INPUT_FILE: path,
      }),
      {
        questionId: "ABC-1234",
        scenarioId: "webwork_delivery",
        schemaVersion: 1,
        title: webworkPublishedQuestionTitle,
      },
    );
    writeFileSync(
      path,
      JSON.stringify({
        questionId: "ABC-1234",
        scenarioId: "webwork_delivery",
        schemaVersion: 1,
        source: "private",
        title: webworkPublishedQuestionTitle,
      }),
      { encoding: "ascii" },
    );
    assert.throws(
      () =>
        requireWebworkPublishedQuestionFixtureInput({
          PLE_WEBWORK_PUBLISHED_QUESTION_FIXTURE_INPUT_FILE: path,
        }),
      /invalid/u,
    );
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
});

test("WebWork visible issuance acknowledgement uses the owner canonical field order", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-webwork-delivery-ack-"));
  try {
    const path = join(directory, "issued.json");
    writeVisibleIssuanceAcknowledgement(
      { PLE_WEBWORK_RENDERER_ISSUANCE_ACK_FILE: path },
      {
        questionId: "ABC-1234",
        scenarioId: "webwork_delivery",
        schemaVersion: 1,
        title: webworkPublishedQuestionTitle,
      },
      "bs1-0123456789ab-webwork_delivery",
    );
    assert.equal(
      readFileSync(path, "ascii"),
      '{"event":"visible_question_issued","namespace":"bs1-0123456789ab-webwork_delivery","scenarioId":"webwork_delivery","schemaVersion":1}',
    );
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
});
