import assert from "node:assert/strict";
import test from "node:test";

import { publishedProblemFixture } from "./fixtures/published_problem.ts";
import {
  createQtiProfileImportClient,
  QtiProfileImportConflictError,
} from "../src/features/qti_profile_import/qti_profile_import_client.ts";
import { decodeQtiProfileImportResponse } from "../src/features/qti_profile_import/qti_profile_import_decoder.ts";
import {
  acknowledgeQtiProfileReport,
  canConvertQtiProfileItem,
  EMPTY_QTI_PROFILE_REVIEW,
  qtiConversionBlockReason,
  receiveQtiProfileReport,
  selectQtiProfileItem,
  shouldKeepQtiReplacementLocked,
  shouldRetrySameQtiImport,
} from "../src/features/qti_profile_import/qti_profile_import_model.ts";

const workspace = "00000000-0000-4000-8000-000000000101";
const importId = "00000000-0000-4000-8000-000000000102";
const reportRevision = "a".repeat(64);
const reviewToken = "b".repeat(64);

const notice = {
  code: "policy",
  location: "item",
  detail: "PLE default applied: unlimited attempts.",
};

function readyReport(overrides = {}) {
  return {
    importId,
    state: "ready",
    profileId: "canvas-qti-1.2-static-single-choice/v1",
    profileLabel: "Canvas QTI 1.2 static single choice",
    profileVersion: "v1",
    reportRevision,
    items: [
      {
        sourceIdentifier: "canvas/beta-β",
        title: "Accepted item",
        status: "accepted",
        diagnostics: [],
        defaults: [notice],
        warnings: [],
      },
      {
        sourceIdentifier: "unsupported-item",
        title: null,
        status: "rejected",
        diagnostics: [
          { code: "itemShape", location: "unsupported-item", detail: "Needs a simpler item." },
        ],
        defaults: [],
        warnings: [],
      },
    ],
    pleDefaults: [notice],
    reviewToken,
    ...overrides,
  };
}

function jsonResponse(value, status, headers = {}) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "private, no-store",
      "content-type": "application/json",
      ...headers,
    },
  });
}

test("strict decoder preserves safe ordered reports and rejects answer-bearing additions", () => {
  assert.deepEqual(decodeQtiProfileImportResponse({ importId, state: "queued" }), {
    importId,
    state: "queued",
  });
  const decoded = decodeQtiProfileImportResponse(readyReport());
  assert.equal(decoded.state, "ready");
  assert.deepEqual(decoded.state === "ready" ? decoded.items.map((item) => item.status) : [], [
    "accepted",
    "rejected",
  ]);
  assert.equal(
    decoded.state === "ready" ? decoded.items[0]?.defaults[0]?.detail : null,
    notice.detail,
  );

  assert.throws(() =>
    decodeQtiProfileImportResponse({ ...readyReport(), correctChoice: "canvas/beta-β" }),
  );
  const contaminated = readyReport();
  contaminated.items[0].grading = { correctChoice: "secret" };
  assert.throws(() => decodeQtiProfileImportResponse(contaminated));
  assert.throws(() =>
    decodeQtiProfileImportResponse({ importId, state: "ready", error: "wrong shape" }),
  );
});

test("client sends opaque ZIP bytes and exact acknowledgement on same-origin no-store requests", async () => {
  const calls = [];
  const archive = new Blob([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], {
    type: "application/octet-stream",
  });
  const draft = { ...publishedProblemFixture.draft, workspace };
  const client = createQtiProfileImportClient({
    basePath: "/ple",
    fetch: async (input, init = {}) => {
      calls.push({ input: String(input), init });
      if (init.method === "PUT") {
        return jsonResponse({ importId, state: "queued" }, 202);
      }
      if (init.method === "GET") return jsonResponse(readyReport(), 200);
      return jsonResponse(draft, 200, { etag: '"7"' });
    },
  });

  await client.upload(workspace, importId, archive);
  const report = await client.report(workspace, importId);
  assert.equal(report.state, "ready");
  const converted = await client.convert(
    workspace,
    importId,
    "canvas/beta-β",
    { reportRevision, reviewToken },
    '"6"',
  );
  assert.equal(converted.revision, '"7"');

  assert.equal(calls[0].input, `/ple/api/workspaces/${workspace}/qti-imports/${importId}`);
  assert.equal(calls[0].init.body, archive);
  assert.equal(new Headers(calls[0].init.headers).get("content-type"), "application/zip");
  assert.equal(calls[0].init.credentials, "same-origin");
  assert.equal(calls[0].init.cache, "no-store");
  assert.equal(
    calls[2].input,
    `/ple/api/workspaces/${workspace}/qti-imports/${importId}/items/canvas%2Fbeta-%CE%B2/convert-flat`,
  );
  assert.equal(new Headers(calls[2].init.headers).get("if-match"), '"6"');
  assert.deepEqual(JSON.parse(calls[2].init.body), { reportRevision, reviewToken });
});

test("report changes invalidate acknowledgement and accepted-item selection", () => {
  const first = readyReport();
  let state = receiveQtiProfileReport(EMPTY_QTI_PROFILE_REVIEW, first);
  state = selectQtiProfileItem(state, first.items[0]);
  state = acknowledgeQtiProfileReport(state, true);
  assert.equal(canConvertQtiProfileItem(state), true);

  const exactReplay = receiveQtiProfileReport(state, readyReport());
  assert.equal(canConvertQtiProfileItem(exactReplay), true);

  const changed = receiveQtiProfileReport(state, readyReport({ reviewToken: "c".repeat(64) }));
  assert.equal(changed.acknowledged, false);
  assert.equal(changed.selectedItem, null);
  assert.equal(canConvertQtiProfileItem(changed), false);
});

test("conversion requires the displayed clean draft and upload retries preserve identity only after ambiguity", () => {
  const report = readyReport();
  let review = receiveQtiProfileReport(EMPTY_QTI_PROFILE_REVIEW, report);
  review = selectQtiProfileItem(review, report.items[0]);
  review = acknowledgeQtiProfileReport(review, true);

  assert.equal(qtiConversionBlockReason(review, null), "draftUnavailable");
  assert.equal(qtiConversionBlockReason(review, { revision: '"6"', dirty: true }), "draftDirty");
  assert.equal(qtiConversionBlockReason(review, { revision: '"6"', dirty: false }), null);
  assert.equal(shouldRetrySameQtiImport(importId, null), true);
  assert.equal(
    shouldRetrySameQtiImport(importId, { importId, state: "failed", error: "failed" }),
    false,
  );
  assert.equal(shouldRetrySameQtiImport(null, null), false);
  assert.equal(shouldKeepQtiReplacementLocked(false, false), false);
  assert.equal(shouldKeepQtiReplacementLocked(true, false), true);
  assert.equal(shouldKeepQtiReplacementLocked(true, true), false);
});

test("conflicts expose only status and path, never the response body", async () => {
  const secret = "correctChoice=private-answer";
  const client = createQtiProfileImportClient({
    fetch: async () => jsonResponse({ error: secret }, 409),
  });
  await assert.rejects(client.upload(workspace, importId, new Blob(["PK"])), (error) => {
    assert.equal(error instanceof QtiProfileImportConflictError, true);
    assert.equal(error.message.includes(secret), false);
    return true;
  });
});
