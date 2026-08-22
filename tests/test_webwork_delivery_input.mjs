import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  requireWebworkCatalogBaselineInput,
  webworkCatalogTitle,
  writeVisibleIssuanceAcknowledgement,
} from "./playwright/e2e/webwork_delivery_input.ts";

test("WebWork catalog browser input accepts only the canonical public hand-off", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-webwork-delivery-input-"));
  try {
    const path = join(directory, "catalog.json");
    writeFileSync(
      path,
      JSON.stringify({
        questionId: "ABC-1234",
        scenarioId: "webwork_delivery",
        schemaVersion: 1,
        title: webworkCatalogTitle,
      }),
      { encoding: "ascii", mode: 0o600 },
    );
    chmodSync(path, 0o600);
    assert.deepEqual(
      requireWebworkCatalogBaselineInput({ PLE_WEBWORK_CATALOG_BASELINE_INPUT_FILE: path }),
      {
        questionId: "ABC-1234",
        scenarioId: "webwork_delivery",
        schemaVersion: 1,
        title: webworkCatalogTitle,
      },
    );
    writeFileSync(
      path,
      JSON.stringify({
        questionId: "ABC-1234",
        scenarioId: "webwork_delivery",
        schemaVersion: 1,
        source: "private",
        title: webworkCatalogTitle,
      }),
      { encoding: "ascii" },
    );
    assert.throws(
      () => requireWebworkCatalogBaselineInput({ PLE_WEBWORK_CATALOG_BASELINE_INPUT_FILE: path }),
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
        title: webworkCatalogTitle,
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
